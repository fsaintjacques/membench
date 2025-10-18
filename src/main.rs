use clap::{Parser, Subcommand};
use membench::record::run_record;
use membench::analyze::run_analyze;
use membench::replay::run_replay;

#[derive(Parser)]
#[command(name = "membench")]
#[command(about = "Privacy-preserving memcache traffic capture and replay")]
struct Cli {
    /// Enable verbose output (-v for info, -vv for debug, -vvv for trace)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

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
    let cli = Cli::parse();

    // Initialize logging based on verbosity level
    let log_level = match cli.verbose {
        0 => tracing::Level::WARN,
        1 => tracing::Level::INFO,
        2 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(cli.verbose >= 2)  // Show module targets in debug+ mode
        .with_level(true)                // Always show log level
        .init();

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
