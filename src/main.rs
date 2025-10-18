use clap::{Parser, Subcommand};
use membench::record::run_record;
use membench::analyze::run_analyze;
use membench::replay::{run_replay, ProtocolMode};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
        /// Network interface to capture from
        interface: String,
        /// Output profile file path
        output: String,
        #[arg(short, long, default_value = "11211")]
        port: u16,
        #[arg(short, long)]
        salt: Option<u64>,
    },
    /// Analyze a captured profile file
    Analyze {
        /// Profile file to analyze
        file: String,
    },
    /// Replay traffic from profile against target server
    Replay {
        /// Profile file to replay
        file: String,
        #[arg(short, long, default_value = "localhost:11211")]
        target: String,
        #[arg(short, long, default_value = "1")]
        concurrency: usize,
        /// Loop mode: once, infinite, or times:N
        #[arg(short, long, default_value = "once")]
        loop_mode: String,
        /// Protocol mode: ascii (old) or meta (new)
        #[arg(long, default_value = "meta")]
        protocol_mode: String,
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
        Commands::Record { interface, output, port, salt } => {
            if let Err(e) = run_record(&interface, port, &output, salt) {
                eprintln!("Record error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Analyze { file } => {
            if let Err(e) = run_analyze(&file) {
                eprintln!("Analyze error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Replay { file, target, concurrency: _, loop_mode, protocol_mode } => {
            // Parse protocol mode at CLI boundary
            let protocol_mode = match ProtocolMode::from_str(&protocol_mode) {
                Ok(mode) => mode,
                Err(e) => {
                    eprintln!("Replay error: {}", e);
                    std::process::exit(1);
                }
            };

            let should_exit = Arc::new(AtomicBool::new(false));
            let should_exit_clone = Arc::clone(&should_exit);

            let _ctrlc_handle = ctrlc::set_handler(move || {
                eprintln!("\nShutdown signal received, completing current iteration...");
                should_exit_clone.store(true, Ordering::Release);
            }).map_err(|e| {
                eprintln!("Failed to set signal handler: {}", e);
            });

            if let Err(e) = run_replay(&file, &target, &loop_mode, protocol_mode, should_exit).await {
                eprintln!("Replay error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
