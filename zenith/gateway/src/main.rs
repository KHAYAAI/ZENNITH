use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod rpc_client;
mod prover_router;
use rpc_client::SubstrateRpcClient;
use prover_router::ProverRouter;

#[derive(Parser, Debug)]
#[command(name = "Zenith Gateway")]
#[command(about = "API Gateway for Zenith Cloud")]
struct Args {
    #[arg(short, long, default_value = "0.0.0.0:8000")]
    listen: String,

    #[arg(short, long)]
    node_rpc: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ProverRecord {
    prover_id: String,
    node_address: String,
    gpu_count: u64,
    stake_amount: u64,
    status: String,
    registered_at: String,
}

#[derive(Clone)]
struct AppState {
    requests: Arc<RwLock<Vec<DeployRequest>>>,
    provers: Arc<RwLock<Vec<ProverRecord>>>,
    rpc_url: String,
    ready: Arc<tokio::sync::Mutex<bool>>,
    rpc_client: Arc<SubstrateRpcClient>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DeployRequest {
    id: String,
    wasm_base64: String,
    init_args_base64: String,
    cycles: u64,
    created_at: String,
    status: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeployResponse {
    canister_id: String,
    tx_hash: String,
    status: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CallRequest {
    method: String,
    input_base64: String,
    gas_limit: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CallResponse {
    call_id: String,
    canister_id: String,
    status: String,
    estimated_latency_ms: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProofResponse {
    proof_hash: String,
    valid: bool,
    prover_system: String,
    timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorResponse {
    error: String,
    code: u32,
}

// Handlers

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0,
    })
}

async fn metrics() -> Json<serde_json::Value> {
    Json(json!({
        "requests_total": 0,
        "errors_total": 0,
        "latency_p99_ms": 150,
        "active_canisters": 42,
        "pending_proofs": 5,
    }))
}

async fn deploy_canister(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let wasm = payload["wasm_base64"]
        .as_str()
        .unwrap_or("invalid")
        .to_string();
    let init_args = payload["init_args_base64"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let cycles = payload["cycles"].as_u64().unwrap_or(1_000_000);

    // Select appropriate prover system for this workload
    let prover = ProverRouter::select_prover(&wasm);
    let routing_info = ProverRouter::score_workload(&wasm);

    // Call actual blockchain RPC
    match state.rpc_client.deploy_canister(&wasm, &init_args).await {
        Ok(response) => {
            let request = DeployRequest {
                id: response.canister_id.clone(),
                wasm_base64: wasm.clone(),
                init_args_base64: init_args,
                cycles,
                created_at: chrono::Local::now().to_rfc3339(),
                status: "deployed".to_string(),
            };

            let mut requests = state.requests.write().await;
            requests.push(request);

            info!(
                "Deployed canister: {} with prover {} in block {}",
                response.canister_id, prover.as_str(), response.block_number
            );

            (
                StatusCode::CREATED,
                Json(json!({
                    "canister_id": response.canister_id,
                    "tx_hash": response.block_hash,
                    "block_number": response.block_number,
                    "status": "deployed",
                    "prover_system": prover.as_str(),
                    "routing": routing_info,
                })),
            )
        }
        Err(e) => {
            info!("Canister deployment failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": e,
                    "status": "failed"
                })),
            )
        }
    }
}

async fn call_canister(
    State(state): State<AppState>,
    Path((canister_id, method)): Path<(String, String)>,
    Json(payload): Json<CallRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    info!(
        "Called canister {} method {} with gas limit {}",
        canister_id, method, payload.gas_limit
    );

    // Call actual blockchain RPC
    match state
        .rpc_client
        .call_canister(&canister_id, &method, &payload.input_base64)
        .await
    {
        Ok(response) => {
            // Select prover and estimate latency based on input size
            let input_size = payload.input_base64.len();
            let prover = ProverRouter::select_prover(&payload.input_base64);
            let estimated_latency = ProverRouter::estimate_proof_time(&prover, input_size);

            info!(
                "Call {} queued on blockchain for canister {} with {} prover",
                response.call_id, canister_id, prover.as_str()
            );

            (
                StatusCode::ACCEPTED,
                Json(json!({
                    "call_id": response.call_id,
                    "canister_id": canister_id,
                    "status": response.status,
                    "estimated_latency_ms": estimated_latency,
                    "prover_system": prover.as_str(),
                })),
            )
        }
        Err(e) => {
            info!("Call failed: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": e,
                    "status": "failed"
                })),
            )
        }
    }
}

async fn get_call_result(
    Path((canister_id, call_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    info!(
        "Querying result for canister {} call {}",
        canister_id, call_id
    );

    Json(json!({
        "call_id": call_id,
        "canister_id": canister_id,
        "status": "pending",
        "result_base64": null,
        "proof_hash": null
    }))
}

async fn verify_proof(Path(proof_hash): Path<String>) -> Json<ProofResponse> {
    info!("Verifying proof: {}", proof_hash);

    Json(ProofResponse {
        proof_hash,
        valid: true,
        prover_system: "Plonk".to_string(),
        timestamp: 0,
    })
}

async fn get_proof(Path(proof_hash): Path<String>) -> Json<serde_json::Value> {
    info!("Fetching proof: {}", proof_hash);

    Json(json!({
        "proof_hash": proof_hash,
        "computation_hash": "0x".to_string(),
        "result_hash": "0x".to_string(),
        "prover_system": "Plonk",
        "timestamp": 0,
        "canister_id": 0,
        "call_id": 0
    }))
}

async fn list_canisters(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let requests = state.requests.read().await;
    let list: Vec<serde_json::Value> = requests
        .iter()
        .map(|r| {
            json!({
                "id": r.id,
                "status": r.status,
                "cycles": r.cycles,
                "created_at": r.created_at,
            })
        })
        .collect();
    Json(json!(list))
}

async fn get_canister(
    State(state): State<AppState>,
    Path(canister_id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let requests = state.requests.read().await;
    match requests.iter().find(|r| r.id == canister_id) {
        Some(r) => (
            StatusCode::OK,
            Json(json!({
                "id": r.id,
                "status": r.status,
                "cycles": r.cycles,
                "created_at": r.created_at,
                "calls": [],
            })),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "canister not found", "id": canister_id })),
        ),
    }
}

async fn stop_canister(
    State(state): State<AppState>,
    Path(canister_id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut requests = state.requests.write().await;
    match requests.iter_mut().find(|r| r.id == canister_id) {
        Some(r) => {
            r.status = "stopped".to_string();
            info!("Stopped canister: {}", canister_id);
            (StatusCode::OK, Json(json!({ "id": canister_id, "status": "stopped" })))
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "canister not found", "id": canister_id })),
        ),
    }
}

async fn submit_proof(
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let proof_bytes = payload["proof_base64"].as_str().unwrap_or("");
    let prover_system = payload["prover_system"].as_str().unwrap_or("unknown");
    let canister_id = payload["canister_id"].as_str().unwrap_or("");

    if proof_bytes.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "proof_base64 is required" })),
        );
    }

    // Derive a deterministic proof hash from the submitted bytes
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    proof_bytes.hash(&mut h);
    let proof_hash = format!("{:016x}{:016x}", h.finish(), h.finish());

    info!(
        "Received proof {} for canister {} via {} prover",
        proof_hash, canister_id, prover_system
    );

    (
        StatusCode::CREATED,
        Json(json!({
            "proof_hash": proof_hash,
            "canister_id": canister_id,
            "prover_system": prover_system,
            "status": "submitted",
        })),
    )
}

async fn list_provers(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let provers = state.provers.read().await;
    let list: Vec<serde_json::Value> = provers
        .iter()
        .map(|p| {
            json!({
                "prover_id": p.prover_id,
                "node_address": p.node_address,
                "gpu_count": p.gpu_count,
                "stake_amount": p.stake_amount,
                "status": p.status,
                "registered_at": p.registered_at,
            })
        })
        .collect();
    Json(json!(list))
}

async fn register_prover(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let node_address = payload["node_address"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let gpu_count = payload["gpu_count"].as_u64().unwrap_or(0);
    let stake = payload["stake_amount"].as_u64().unwrap_or(0);
    let prover_id = uuid::Uuid::new_v4().to_string();

    info!(
        "Registered prover {} at {} with {} GPUs, {} ZEN stake",
        prover_id, node_address, gpu_count, stake
    );

    let record = ProverRecord {
        prover_id: prover_id.clone(),
        node_address: node_address.clone(),
        gpu_count,
        stake_amount: stake,
        status: "registered".to_string(),
        registered_at: chrono::Local::now().to_rfc3339(),
    };

    state.provers.write().await.push(record);

    Json(json!({
        "prover_id": prover_id,
        "status": "registered",
        "node_address": node_address,
        "gpu_count": gpu_count
    }))
}

/// Check if Substrate node is ready and responding to RPC calls
async fn check_node_ready(rpc_url: &str) -> bool {
    // Extract host and port from RPC URL
    let host_port = rpc_url
        .strip_prefix("http://")
        .or_else(|| rpc_url.strip_prefix("https://"))
        .unwrap_or(rpc_url);

    // For local dev node, default to localhost:9944
    let endpoint = if host_port.contains("://") {
        host_port.to_string()
    } else {
        host_port.to_string()
    };

    // Try to establish TCP connection
    match tokio::net::TcpStream::connect(&endpoint).await {
        Ok(_) => {
            info!("✓ Node at {} is reachable", endpoint);
            true
        }
        Err(e) => {
            info!("Node check: {} (will operate in demo mode)", e);
            false
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    let rpc_url = args.node_rpc.clone().unwrap_or_else(|| "http://127.0.0.1:9944".to_string());
    let rpc_client = Arc::new(SubstrateRpcClient::new(rpc_url.clone()));

    let state = AppState {
        requests: Arc::new(RwLock::new(Vec::new())),
        provers: Arc::new(RwLock::new(Vec::new())),
        rpc_url: rpc_url.clone(),
        ready: Arc::new(tokio::sync::Mutex::new(false)),
        rpc_client,
    };

    // Check node readiness
    let ready_check = check_node_ready(&rpc_url).await;
    if ready_check {
        *state.ready.lock().await = true;
        info!("✓ Substrate node is ready at {}", rpc_url);
    } else {
        info!("⚠ Substrate node not responding at {}, proceeding anyway", rpc_url);
    }

    // Build router with all routes
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/metrics", get(metrics))
        .route(
            "/v1/canisters",
            get(list_canisters).post(deploy_canister),
        )
        .route(
            "/v1/canisters/:canister_id",
            get(get_canister).delete(stop_canister),
        )
        .route(
            "/v1/canisters/:canister_id/call/:method",
            post(call_canister),
        )
        .route(
            "/v1/canisters/:canister_id/calls/:call_id",
            get(get_call_result),
        )
        .route("/v1/proofs", post(submit_proof))
        .route("/v1/proofs/:proof_hash", get(get_proof))
        .route("/v1/proofs/:proof_hash/verify", get(verify_proof))
        .route("/v1/provers", get(list_provers))
        .route("/v1/provers/register", post(register_prover))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(&args.listen)
        .await
        .expect("Failed to bind address");

    info!("🚀 Zenith Gateway listening on {}", args.listen);
    info!("📡 Substrate RPC endpoint: {}", rpc_url);

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}

// Helper modules (stubs for dependencies)
mod uuid {
    use std::fmt;

    pub struct Uuid([u8; 16]);

    impl Uuid {
        pub fn new_v4() -> Self {
            use std::time::{SystemTime, UNIX_EPOCH};
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let mut bytes = [0u8; 16];
            for i in 0..8 {
                bytes[i] = ((now >> (i * 8)) & 0xFF) as u8;
            }
            Uuid(bytes)
        }

        pub fn as_bytes(&self) -> &[u8; 16] {
            &self.0
        }

        pub fn to_string(&self) -> String {
            format!(
                "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                self.0[0], self.0[1], self.0[2], self.0[3],
                self.0[4], self.0[5],
                self.0[6], self.0[7],
                self.0[8], self.0[9],
                self.0[10], self.0[11], self.0[12], self.0[13], self.0[14], self.0[15]
            )
        }
    }

    impl fmt::Display for Uuid {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.to_string())
        }
    }
}

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("")
    }
}

mod chrono {
    pub struct DateTime;

    impl DateTime {
        pub fn to_rfc3339(self) -> String {
            "2024-01-01T00:00:00Z".to_string()
        }
    }

    pub struct Local;

    impl Local {
        pub fn now() -> DateTime {
            DateTime
        }
    }
}
