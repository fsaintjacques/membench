use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "membench")]
#[command(about = "Privacy-preserving memcache traffic capture and replay")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Capture memcache traffic from network interface
    Record {
        #[arg(short, long)]
        interface: String,
        #[arg(short, long, default_value = "11211")]
        port: u16,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        salt: Option<u64>,
    },
    /// Replay traffic from profile against target server
    Replay {
        #[arg(short, long)]
        input: String,
        #[arg(short, long, default_value = "localhost:11211")]
        target: String,
        #[arg(short, long, default_value = "4")]
        concurrency: usize,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    match cli.command {
        Commands::Record { interface, port, output, salt } => {
            if let Err(e) = run_record(&interface, port, &output, salt) {
                eprintln!("Record error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Replay { input, target, concurrency } => {
            if let Err(e) = run_replay(&input, &target, concurrency).await {
                eprintln!("Replay error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn run_record(interface: &str, port: u16, output: &str, salt: Option<u64>) -> anyhow::Result<()> {
    use std::time::SystemTime;
    use std::collections::HashMap;
    use membench::record::{PacketCapture, StreamReassembler, MemcacheParser, Anonymizer, ProfileWriter};
    use membench::profile::{Event, Response};

    let salt = salt.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });

    println!("Recording from {}:{} to {} (salt: {})", interface, port, output, salt);
    println!("Capturing memcache traffic... Press Ctrl+C to stop.");

    // Initialize components
    let mut capture = PacketCapture::new(interface, port)?;
    let _reassembler = StreamReassembler::new();
    let parser = MemcacheParser::new();
    let anonymizer = Anonymizer::new(salt);
    let mut writer = ProfileWriter::new(output)?;

    // Track connection state
    let mut packet_count = 0u64;
    let mut event_count = 0u64;

    println!("\nCapturing packets... (Press Ctrl+C to stop)\n");

    loop {
        // Capture packet
        match capture.next_packet() {
            Ok(packet_data) => {
                packet_count += 1;

                // Log progress every 1000 packets
                if packet_count % 1000 == 0 {
                    println!("Captured {} packets, {} events", packet_count, event_count);
                }

                // Try to parse as memcache command
                // Note: This is a simplified parser that handles basic cases
                if let Ok(data_str) = std::str::from_utf8(packet_data) {
                    if data_str.contains('\r') && data_str.contains('\n') {
                        // Try parsing as a command
                        if let Ok((cmd, _)) = parser.parse_command(packet_data) {
                            // Create event from parsed command
                            let event = Event {
                                timestamp: SystemTime::now()
                                    .duration_since(SystemTime::UNIX_EPOCH)
                                    .unwrap()
                                    .as_micros() as u64,
                                conn_id: (packet_count % 32) as u32, // Simplified connection ID
                                cmd_type: cmd.cmd_type,
                                key_hash: anonymizer.hash_key(b"captured_key"), // Would extract real key in full impl
                                key_size: cmd.key_range.len() as u32,
                                value_size: cmd.value_size,
                                flags: cmd.flags,
                                response: Response::Found(0), // Would parse response in full impl
                            };

                            writer.write_event(&event)?;
                            event_count += 1;

                            if event_count % 100 == 0 {
                                println!("  Captured {} events", event_count);
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // On interrupt (Ctrl+C), gracefully finish
                if packet_count == 0 {
                    println!("\nNo packets captured. Make sure:");
                    println!("  1. memcached is running on {}:{}", interface, port);
                    println!("  2. There is traffic on that port");
                    println!("  3. You have appropriate permissions (may need sudo)");
                }
                break;
            }
        }
    }

    // Finalize profile
    println!("\nFinalizing profile...");
    writer.finish()?;

    println!("✓ Recording complete");
    println!("  Profile: {}", output);
    println!("  Packets captured: {}", packet_count);
    println!("  Events recorded: {}", event_count);

    Ok(())
}

async fn run_replay(input: &str, target: &str, concurrency: usize) -> anyhow::Result<()> {
    use membench::replay::{ProfileReader, DistributionAnalyzer, TrafficGenerator, ReplayClient};

    let reader = ProfileReader::new(input)?;
    let analysis = DistributionAnalyzer::analyze(reader.events());

    println!("Profile loaded: {} events, hit rate: {:.2}%",
             analysis.total_events, analysis.hit_rate * 100.0);
    println!("Replaying to {} with {} concurrent connections. Press Ctrl+C to stop.",
             target, concurrency);

    let mut sent = 0u64;
    let mut errors = 0u64;

    loop {
        for _ in 0..concurrency {
            let mut gen = TrafficGenerator::new(analysis.clone());
            let event = gen.next_command();

            match ReplayClient::new(target, 65536) {
                Ok(mut client) => {
                    match client.send_command(&event) {
                        Ok(_) => {
                            sent += 1;
                            if sent % 1000 == 0 {
                                println!("Sent {} commands ({} errors)", sent, errors);
                            }
                        }
                        Err(e) => {
                            errors += 1;
                            tracing::warn!("Send error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    errors += 1;
                    tracing::warn!("Connection error: {}", e);
                }
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
