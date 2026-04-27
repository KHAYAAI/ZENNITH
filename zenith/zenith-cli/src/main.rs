use clap::{Parser, Subcommand};
use colored::Colorize;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "zenith")]
#[command(about = "Zenith Cloud Protocol CLI")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Args {
    #[command(subcommand)]
    command: Command,

    #[arg(long, global = true)]
    network: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[command(about = "Initialize a new canister project")]
    Init {
        name: String,

        #[arg(long, default_value = "0.1.0")]
        version: String,
    },

    #[command(about = "Build a canister")]
    Build,

    #[command(about = "Deploy a canister")]
    Deploy {
        #[arg(long)]
        network: Option<String>,
    },

    #[command(about = "Call a canister method")]
    Call {
        #[arg(help = "Canister ID")]
        canister_id: String,

        #[arg(help = "Method name")]
        method: String,

        #[arg(long)]
        arg: Option<String>,

        #[arg(long)]
        wait: bool,
    },

    #[command(about = "Verify a proof")]
    ProofVerify {
        #[arg(help = "Proof hash")]
        proof_hash: String,
    },

    #[command(about = "Show proof history")]
    ProofHistory {
        #[arg(help = "Canister ID")]
        canister_id: String,
    },

    #[command(about = "Show network/canister status")]
    Status {
        #[arg(help = "Optional canister ID")]
        canister_id: Option<String>,
    },

    #[command(about = "Top up a canister")]
    TopUp {
        #[arg(help = "Canister ID")]
        canister_id: String,

        #[arg(long)]
        amount: String,
    },

    #[command(about = "Stream canister logs")]
    Logs {
        #[arg(help = "Canister ID")]
        canister_id: String,

        #[arg(long)]
        follow: bool,
    },
}

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    match args.command {
        Command::Init { name, version } => {
            println!(
                "{}",
                format!("Initializing canister project: {}", name).green()
            );
            println!("  Version: {}", version);
        }
        Command::Build => {
            println!("{}", "Building canister...".green());
        }
        Command::Deploy { network } => {
            let net = network.as_deref().unwrap_or("testnet");
            println!("{}", format!("Deploying to {}...", net).green());
        }
        Command::Call {
            canister_id,
            method,
            arg,
            wait,
        } => {
            println!(
                "{}",
                format!("Calling {}::{}", canister_id, method).green()
            );
            if let Some(a) = arg {
                println!("  Argument: {}", a);
            }
            if wait {
                println!("  Waiting for result...");
            }
        }
        Command::ProofVerify { proof_hash } => {
            println!("{}", format!("Verifying proof: {}", proof_hash).blue());
        }
        Command::ProofHistory { canister_id } => {
            println!("{}", format!("Proof history for: {}", canister_id).blue());
        }
        Command::Status { canister_id } => {
            println!("{}", "Network Status".cyan().bold());
            if let Some(cid) = canister_id {
                println!("  Canister: {}", cid);
            }
        }
        Command::TopUp { canister_id, amount } => {
            println!(
                "{}",
                format!("Top-up {} with {} ZEN", canister_id, amount).yellow()
            );
        }
        Command::Logs { canister_id, follow } => {
            println!("{}", format!("Logs for: {}", canister_id).magenta());
            if follow {
                println!("  Following... (Ctrl+C to stop)");
            }
        }
    }
}
