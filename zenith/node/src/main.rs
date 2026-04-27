use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "zenith-node")]
#[command(about = "Zenith Substrate Node")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(long, global = true)]
    dev: bool,

    #[arg(long, global = true)]
    validator: bool,

    #[arg(long, global = true)]
    prover: bool,

    #[arg(long, global = true)]
    storage: bool,

    #[arg(long, global = true)]
    base_path: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[command(about = "Run in development mode")]
    Dev,

    #[command(about = "Purge the chain database")]
    PurgeChain,

    #[command(about = "Export the state")]
    Export {
        #[arg(help = "Output file")]
        output: PathBuf,
    },
}

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    match args.command {
        Some(Command::Dev) => {
            println!("Running in development mode...");
        }
        Some(Command::PurgeChain) => {
            println!("Purging chain database...");
        }
        Some(Command::Export { output }) => {
            println!("Exporting state to {:?}", output);
        }
        None => {
            println!("Starting Zenith node...");
            if args.dev {
                println!("  Mode: dev");
            }
            if args.validator {
                println!("  Mode: validator");
            }
            if args.prover {
                println!("  Mode: prover");
            }
            if args.storage {
                println!("  Mode: storage");
            }
        }
    }

    println!("Zenith node started");
}
