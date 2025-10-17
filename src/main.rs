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
        Commands::Record { .. } => {
            println!("Record mode not yet implemented");
        }
        Commands::Replay { .. } => {
            println!("Replay mode not yet implemented");
        }
    }
}
