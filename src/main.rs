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
    /// Analyze a captured profile file
    Analyze {
        #[arg(short, long)]
        input: String,
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
        Commands::Analyze { input } => {
            if let Err(e) = run_analyze(&input) {
                eprintln!("Analyze error: {}", e);
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

fn run_analyze(input: &str) -> anyhow::Result<()> {
    use membench::replay::{ProfileReader, DistributionAnalyzer};

    let reader = ProfileReader::new(input)?;
    let metadata = reader.metadata();
    let analysis = DistributionAnalyzer::analyze(reader.events());

    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!("║            Profile Analysis Report                  ║");
    println!("╚═══════════════════════════════════════════════════════╝\n");

    // File metadata
    println!("Profile: {}", input);
    println!("Version: {}\n", metadata.version);

    // Event statistics
    println!("─ Event Statistics ─");
    println!("Total events: {}", analysis.total_events);
    println!("Unique connections: {}\n", metadata.unique_connections);

    // Time range
    let time_range = metadata.time_range;
    if time_range.0 > 0 || time_range.1 > 0 {
        let duration_micros = time_range.1.saturating_sub(time_range.0);
        let duration_secs = duration_micros as f64 / 1_000_000.0;
        println!("Time range: {:.2} seconds\n", duration_secs);
    }

    // Command distribution
    println!("─ Command Distribution ─");
    let mut cmd_entries: Vec<_> = analysis.command_distribution.iter().collect();
    cmd_entries.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (cmd, count) in cmd_entries {
        let percentage = (*count as f64 / analysis.total_events as f64) * 100.0;
        println!("{:?}: {} ({:.1}%)", cmd, count, percentage);
    }

    // Response statistics
    println!("\n─ Response Statistics ─");
    println!("Total responses: {}", analysis.total_responses);
    println!("Cache hits: {} ({:.2}%)", analysis.hit_count, analysis.hit_rate * 100.0);
    println!("Cache misses: {} ({:.2}%)", analysis.total_responses - analysis.hit_count, (1.0 - analysis.hit_rate) * 100.0);

    // Response distribution
    if !analysis.response_distribution.is_empty() {
        println!("\nResponse breakdown:");
        let mut resp_entries: Vec<_> = analysis.response_distribution.iter().collect();
        resp_entries.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        for (resp_type, count) in resp_entries {
            let percentage = (*count as f64 / analysis.total_responses as f64) * 100.0;
            println!("  {}: {} ({:.1}%)", resp_type, count, percentage);
        }
    }

    // Key size distribution
    println!("\n─ Key Size Distribution ─");
    if !analysis.key_size_distribution.is_empty() {
        let mut key_sizes: Vec<_> = analysis.key_size_distribution.clone();
        key_sizes.sort_by_key(|(size, _)| *size);

        let total_keys = analysis.total_events;
        let min_size = key_sizes.iter().map(|(s, _)| *s).min().unwrap_or(0);
        let max_size = key_sizes.iter().map(|(s, _)| *s).max().unwrap_or(0);

        let avg_size: f64 = key_sizes.iter()
            .map(|(size, count)| *size as f64 * *count as f64)
            .sum::<f64>() / total_keys.max(1) as f64;

        println!("Min: {} bytes", min_size);
        println!("Max: {} bytes", max_size);
        println!("Avg: {:.1} bytes", avg_size);

        if key_sizes.len() <= 10 {
            println!("\nDistribution:");
            for (size, count) in &key_sizes {
                let percentage = (*count as f64 / total_keys as f64) * 100.0;
                println!("  {} bytes: {} ({:.1}%)", size, count, percentage);
            }
        } else {
            println!("\nTop 10 sizes:");
            let mut top_sizes = key_sizes.clone();
            top_sizes.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
            for (size, count) in top_sizes.iter().take(10) {
                let percentage = (*count as f64 / total_keys as f64) * 100.0;
                println!("  {} bytes: {} ({:.1}%)", size, count, percentage);
            }
        }
    }

    // Value size distribution
    println!("\n─ Value Size Distribution ─");
    if !analysis.value_size_distribution.is_empty() {
        let mut value_sizes: Vec<_> = analysis.value_size_distribution.clone();
        value_sizes.sort_by_key(|(size, _)| *size);

        let total_values = value_sizes.iter().map(|(_, c)| c).sum::<u64>();
        let min_size = value_sizes.iter().map(|(s, _)| *s).min().unwrap_or(0);
        let max_size = value_sizes.iter().map(|(s, _)| *s).max().unwrap_or(0);

        let avg_size: f64 = value_sizes.iter()
            .map(|(size, count)| *size as f64 * *count as f64)
            .sum::<f64>() / total_values.max(1) as f64;

        println!("Min: {} bytes", min_size);
        println!("Max: {} bytes", max_size);
        println!("Avg: {:.1} bytes", avg_size);
        println!("Total with values: {} ({:.1}%)", total_values, (total_values as f64 / analysis.total_events as f64) * 100.0);

        if value_sizes.len() <= 10 {
            println!("\nDistribution:");
            for (size, count) in &value_sizes {
                let percentage = (*count as f64 / total_values as f64) * 100.0;
                println!("  {} bytes: {} ({:.1}%)", size, count, percentage);
            }
        } else {
            println!("\nTop 10 sizes:");
            let mut top_sizes = value_sizes.clone();
            top_sizes.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
            for (size, count) in top_sizes.iter().take(10) {
                let percentage = (*count as f64 / total_values as f64) * 100.0;
                println!("  {} bytes: {} ({:.1}%)", size, count, percentage);
            }
        }
    } else {
        println!("No value data in profile");
    }

    println!("\n");

    Ok(())
}

async fn run_replay(input: &str, target: &str, concurrency: usize) -> anyhow::Result<()> {
    use membench::replay::{ProfileReader, DistributionAnalyzer, TrafficGenerator, ReplayClient};
    use std::time::{Instant, Duration};

    let reader = ProfileReader::new(input)?;
    let analysis = DistributionAnalyzer::analyze(reader.events());

    println!("\n╔════════════════════════════════════════════╗");
    println!("║         Replay Statistics                  ║");
    println!("╚════════════════════════════════════════════╝\n");
    println!("Profile: {}", input);
    println!("Target: {}", target);
    println!("Concurrency: {}", concurrency);
    println!("Total events in profile: {}", analysis.total_events);
    println!("Command distribution:");
    for (cmd, count) in &analysis.command_distribution {
        println!("  {:?}: {}", cmd, count);
    }
    println!("Cache hit rate: {:.2}%\n", analysis.hit_rate * 100.0);
    println!("Starting replay... (Press Ctrl+C to stop)\n");

    let mut sent = 0u64;
    let mut errors = 0u64;
    let start_time = Instant::now();
    let mut last_report = Instant::now();

    loop {
        for _ in 0..concurrency {
            let mut gen = TrafficGenerator::new(analysis.clone());
            let event = gen.next_command();

            match ReplayClient::new(target, 65536) {
                Ok(mut client) => {
                    match client.send_command(&event) {
                        Ok(_) => {
                            sent += 1;
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

        // Report statistics every 5 seconds
        if last_report.elapsed() >= Duration::from_secs(5) {
            let elapsed = start_time.elapsed().as_secs_f64();
            let throughput = sent as f64 / elapsed;
            let error_rate = if sent > 0 {
                (errors as f64 / (sent + errors) as f64) * 100.0
            } else {
                0.0
            };

            println!(
                "[{:6}s] Sent: {:8} | Errors: {:6} | Throughput: {:8.0} ops/sec | Error rate: {:.2}%",
                elapsed as u64, sent, errors, throughput, error_rate
            );
            last_report = Instant::now();
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
