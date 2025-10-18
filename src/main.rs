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

    let salt = salt.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });

    println!("Recording from {}:{} to {} (salt: {})", interface, port, output, salt);
    println!("Capturing memcache traffic... Press Ctrl+C to stop.");

    // TODO: Implement actual recording logic
    // This would integrate PacketCapture, StreamReassembler, MemcacheParser,
    // Anonymizer, and ProfileWriter

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
