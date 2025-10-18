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

fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    match cli.command {
        Commands::Record { interface, port, output, salt } => {
            if let Err(e) = run_record(&interface, port, &output, salt) {
                eprintln!("Record error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Replay { .. } => {
            println!("Replay mode not yet implemented");
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
