use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use zenith_sdk::{CanisterId, ZenithClient};

#[derive(Parser, Debug)]
#[command(name = "zenith")]
#[command(about = "Zenith Cloud Protocol CLI")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Args {
    #[command(subcommand)]
    command: Command,

    /// Gateway URL (overrides ZENITH_GATEWAY env var)
    #[arg(long, global = true, env = "ZENITH_GATEWAY", default_value = "http://localhost:8000")]
    gateway: String,
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

    #[command(about = "Deploy a WASM canister to the network")]
    Deploy {
        /// Path to the compiled .wasm file
        #[arg(long)]
        wasm: PathBuf,

        /// Initial cycles to allocate (default 1_000_000)
        #[arg(long, default_value = "1000000")]
        cycles: u64,
    },

    #[command(about = "Call a canister method")]
    Call {
        #[arg(help = "Canister ID")]
        canister_id: String,

        #[arg(help = "Method name")]
        method: String,

        /// Argument as a hex string (e.g. --arg deadbeef)
        #[arg(long)]
        arg: Option<String>,

        #[arg(long)]
        wait: bool,
    },

    #[command(about = "Verify a proof by hash")]
    ProofVerify {
        #[arg(help = "Proof hash")]
        proof_hash: String,
    },

    #[command(about = "Show proof/call history for a canister")]
    ProofHistory {
        #[arg(help = "Canister ID")]
        canister_id: String,
    },

    #[command(about = "Show network and canister status")]
    Status {
        #[arg(help = "Optional canister ID")]
        canister_id: Option<String>,
    },

    #[command(about = "Top up a canister with cycles")]
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

    #[command(about = "List all deployed canisters")]
    List,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();
    let client = ZenithClient::new(&args.gateway);

    match run(args.command, client).await {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            std::process::exit(1);
        }
    }
}

async fn run(command: Command, client: ZenithClient) -> Result<(), String> {
    match command {
        Command::Init { name, version } => {
            println!("{}", format!("Initializing canister project: {}", name).green());
            println!("  Version: {}", version);
            println!("  Run 'cargo build --target wasm32-unknown-unknown' to compile.");
        }

        Command::Build => {
            println!("{}", "Building canister...".green());
            println!("  Run: cargo build --target wasm32-unknown-unknown --release");
        }

        Command::Deploy { wasm, cycles } => {
            let wasm_bytes = std::fs::read(&wasm)
                .map_err(|e| format!("Failed to read wasm file '{}': {}", wasm.display(), e))?;

            println!("{}", format!("Deploying {} ({} bytes)...", wasm.display(), wasm_bytes.len()).green());

            let result = client
                .deploy_canister_with_args(wasm_bytes, vec![], cycles)
                .await
                .map_err(|e| e.to_string())?;

            println!("{}", "Deployed successfully!".green().bold());
            println!("  Canister ID:   {}", result.canister_id.0.cyan());
            println!("  Tx Hash:       {}", result.tx_hash);
            println!("  Block:         {}", result.block_number);
            println!("  Prover:        {}", result.prover_system);
            println!("  Status:        {}", result.status);
        }

        Command::Call { canister_id, method, arg, wait } => {
            let input = match arg {
                Some(hex_str) => {
                    let clean = hex_str.trim_start_matches("0x");
                    (0..clean.len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&clean[i..i + 2], 16))
                        .collect::<Result<Vec<u8>, _>>()
                        .map_err(|e| format!("Invalid hex argument: {}", e))?
                }
                None => vec![],
            };

            println!("{}", format!("Calling {}::{}", canister_id, method).green());

            let result = client
                .call_canister_with_gas(&CanisterId(canister_id.clone()), &method, input, 100_000)
                .await
                .map_err(|e| e.to_string())?;

            println!("  Call ID:       {}", result.call_id.cyan());
            println!("  Status:        {}", result.status);
            println!("  Latency est.:  {}ms", result.estimated_latency_ms);
            if let Some(ps) = &result.prover_system {
                println!("  Prover:        {}", ps);
            }
            if let Some(ph) = &result.proof_hash {
                println!("  Proof Hash:    {}", ph);
            }

            if wait {
                println!("{}", "  Waiting for proof...".yellow());
                // Poll proof verification when proof hash is available
                if let Some(hash) = &result.proof_hash {
                    let verify = client.verify_proof(hash).await.map_err(|e| e.to_string())?;
                    if verify.valid {
                        println!("{}", "  Proof verified!".green().bold());
                    } else {
                        println!("{}", "  Proof invalid!".red().bold());
                    }
                }
            }
        }

        Command::ProofVerify { proof_hash } => {
            println!("{}", format!("Verifying proof {}...", proof_hash).blue());

            let result = client.verify_proof(&proof_hash).await.map_err(|e| e.to_string())?;

            if result.valid {
                println!("{}", "  Valid!".green().bold());
            } else {
                println!("{}", "  Invalid!".red().bold());
            }
            println!("  Prover System: {}", result.prover_system);
            println!("  Proof Hash:    {}", result.proof_hash);
        }

        Command::ProofHistory { canister_id } => {
            println!("{}", format!("Proof history for canister {}:", canister_id).blue());

            let info = client
                .get_canister(&CanisterId(canister_id))
                .await
                .map_err(|e| e.to_string())?;

            if let Some(calls) = info["calls"].as_array() {
                if calls.is_empty() {
                    println!("  No calls recorded.");
                }
                for (i, call) in calls.iter().enumerate() {
                    println!(
                        "  [{}] call_id={} status={} proof={}",
                        i,
                        call["call_id"].as_str().unwrap_or("?"),
                        call["status"].as_str().unwrap_or("?"),
                        call["proof_hash"].as_str().unwrap_or("none"),
                    );
                }
            } else {
                println!("  Canister info: {}", serde_json::to_string_pretty(&info).unwrap_or_default());
            }
        }

        Command::Status { canister_id } => {
            println!("{}", "Network Status".cyan().bold());

            let health = client.health().await.map_err(|e| e.to_string())?;
            println!("  Gateway status:  {}", health.status.green());
            println!("  Version:         {}", health.version);
            println!("  Uptime:          {}s", health.uptime_seconds);

            if let Some(cid) = canister_id {
                println!();
                println!("{}", format!("Canister: {}", cid).cyan().bold());
                let info = client
                    .get_canister(&CanisterId(cid))
                    .await
                    .map_err(|e| e.to_string())?;
                println!("  Status:  {}", info["status"].as_str().unwrap_or("unknown"));
                println!("  Created: {}", info["created_at"].as_str().unwrap_or("unknown"));
            }
        }

        Command::TopUp { canister_id, amount } => {
            println!("{}", format!("Top-up {} with {} ZEN", canister_id, amount).yellow());
            println!("  (Top-up is handled via on-chain extrinsic; use substrate-cli or wallet app)");
        }

        Command::Logs { canister_id, follow } => {
            println!("{}", format!("Logs for canister {}:", canister_id).magenta());
            let info = client
                .get_canister(&CanisterId(canister_id.clone()))
                .await
                .map_err(|e| e.to_string())?;

            println!("  Status: {}", info["status"].as_str().unwrap_or("unknown"));
            if follow {
                println!("  {}", "(Live log streaming requires a running gateway with WebSocket support)".yellow());
            }
        }

        Command::List => {
            println!("{}", "Deployed Canisters".cyan().bold());

            let canisters = client.list_canisters().await.map_err(|e| e.to_string())?;

            if canisters.is_empty() {
                println!("  No canisters deployed.");
            } else {
                for c in &canisters {
                    println!(
                        "  {} | status={} | created={}",
                        c["id"].as_str().unwrap_or("?").cyan(),
                        c["status"].as_str().unwrap_or("?"),
                        c["created_at"].as_str().unwrap_or("?"),
                    );
                }
                println!("  Total: {}", canisters.len());
            }
        }
    }

    Ok(())
}
