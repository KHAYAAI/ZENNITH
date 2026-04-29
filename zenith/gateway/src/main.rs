use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware,
    response::Json,
    routing::{get, post},
    Router,
};
use clap::Parser;
use prometheus::Registry;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod audit;
mod auth;
mod circuit_breaker;
mod db;
mod metrics;
mod prover_router;
mod rate_limit;
mod rpc_client;
mod payment;
mod payment_api;

use audit::AuditLogger;
use auth::Claims;
use circuit_breaker::CircuitBreaker;
use db::{Database, DeployRequest, ProverRecord};
use metrics::Metrics;
use prover_router::ProverRouter;
use rate_limit::RateLimitMiddleware;
use rpc_client::SubstrateRpcClient;
use payment::PaymentProcessor;

#[derive(Parser, Debug)]
#[command(name = "Zenith Gateway")]
#[command(about = "API Gateway for Zenith Cloud")]
struct Args {
    #[arg(short, long, default_value = "0.0.0.0:8000")]
    listen: String,

    #[arg(short, long)]
    node_rpc: Option<String>,

    #[arg(short, long, default_value = "./zenith-gateway-data")]
    data_dir: String,
}

#[derive(Clone)]
struct AppState {
    db: Arc<Database>,
    rpc_url: String,
    ready: Arc<tokio::sync::Mutex<bool>>,
    rpc_client: Arc<SubstrateRpcClient>,
    metrics: Arc<Metrics>,
    rate_limit: Arc<RateLimitMiddleware>,
    circuit_breaker: Arc<CircuitBreaker>,
    audit_logger: Arc<AuditLogger>,
    payment_processor: Arc<PaymentProcessor>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CallRequest {
    method: String,
    input_base64: String,
    gas_limit: u64,
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

// Handlers

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0,
    })
}

async fn metrics_endpoint() -> Result<String, (StatusCode, String)> {
    metrics::gather_metrics().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn list_canisters(State(state): State<AppState>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state.metrics.requests_total.inc();
    let canisters = state
        .db
        .list_canisters()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let list: Vec<serde_json::Value> = canisters
        .iter()
        .map(|c| {
            json!({
                "id": c.id,
                "status": c.status,
                "cycles": c.cycles,
                "created_at": c.created_at,
            })
        })
        .collect();
    state.metrics.active_canisters.set(list.len() as i64);
    Ok(Json(json!(list)))
}

async fn get_canister(
    State(state): State<AppState>,
    Path(canister_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state.metrics.requests_total.inc();
    match state
        .db
        .get_canister(&canister_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
    {
        Some(c) => Ok(Json(json!({
            "id": c.id,
            "status": c.status,
            "cycles": c.cycles,
            "created_at": c.created_at,
            "calls": [],
        }))),
        None => {
            state.metrics.errors_total.inc();
            Err((
                StatusCode::NOT_FOUND,
                json!({ "error": "canister not found", "id": canister_id }).to_string(),
            ))
        }
    }
}

async fn deploy_canister(
    State(state): State<AppState>,
    _claims: Claims,
    Json(payload): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    state.metrics.requests_total.inc();
    let wasm = payload["wasm_base64"]
        .as_str()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing wasm_base64".to_string()))?
        .to_string();
    let init_args = payload["init_args_base64"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let cycles = payload["cycles"].as_u64().unwrap_or(1_000_000);

    let prover = ProverRouter::select_prover(&wasm);
    let routing_info = ProverRouter::score_workload(&wasm);

    match state
        .rpc_client
        .deploy_canister(&wasm, &init_args)
        .await
    {
        Ok(response) => {
            let canister = DeployRequest {
                id: response.canister_id.clone(),
                wasm_base64: wasm.clone(),
                init_args_base64: init_args,
                cycles,
                created_at: chrono_wrapper::now_rfc3339(),
                status: "deployed".to_string(),
            };

            state
                .db
                .insert_canister(&canister)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

            state.metrics.canisters_deployed.inc();

            info!(
                "Deployed canister: {} with prover {} in block {}",
                response.canister_id, prover.as_str(), response.block_number
            );

            Ok((
                StatusCode::CREATED,
                Json(json!({
                    "canister_id": response.canister_id,
                    "tx_hash": response.block_hash,
                    "block_number": response.block_number,
                    "status": "deployed",
                    "prover_system": prover.as_str(),
                    "routing": routing_info,
                })),
            ))
        }
        Err(e) => {
            state.metrics.errors_total.inc();
            info!("Canister deployment failed: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": e }).to_string()))
        }
    }
}

async fn call_canister(
    State(state): State<AppState>,
    _claims: Claims,
    Path((canister_id, method)): Path<(String, String)>,
    Json(payload): Json<CallRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    state.metrics.requests_total.inc();

    match state
        .rpc_client
        .call_canister(&canister_id, &method, &payload.input_base64)
        .await
    {
        Ok(response) => {
            let input_size = payload.input_base64.len();
            let prover = ProverRouter::select_prover(&payload.input_base64);
            let estimated_latency = ProverRouter::estimate_proof_time(&prover, input_size);

            info!(
                "Call {} queued for canister {} with {} prover",
                response.call_id, canister_id, prover.as_str()
            );

            Ok((
                StatusCode::ACCEPTED,
                Json(json!({
                    "call_id": response.call_id,
                    "canister_id": canister_id,
                    "status": response.status,
                    "estimated_latency_ms": estimated_latency,
                    "prover_system": prover.as_str(),
                })),
            ))
        }
        Err(e) => {
            state.metrics.errors_total.inc();
            info!("Call failed: {}", e);
            Err((StatusCode::BAD_REQUEST, json!({ "error": e }).to_string()))
        }
    }
}

async fn stop_canister(
    State(state): State<AppState>,
    _claims: Claims,
    Path(canister_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state.metrics.requests_total.inc();

    match state
        .db
        .get_canister(&canister_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
    {
        Some(mut c) => {
            c.status = "stopped".to_string();
            state
                .db
                .update_canister(&c)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
            info!("Stopped canister: {}", canister_id);
            Ok(Json(json!({ "id": canister_id, "status": "stopped" })))
        }
        None => {
            state.metrics.errors_total.inc();
            Err((
                StatusCode::NOT_FOUND,
                json!({ "error": "canister not found" }).to_string(),
            ))
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

async fn submit_proof(
    State(state): State<AppState>,
    _claims: Claims,
    Json(payload): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    state.metrics.requests_total.inc();

    let proof_bytes = payload["proof_base64"]
        .as_str()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "proof_base64 required".to_string()))?;
    let prover_system = payload["prover_system"].as_str().unwrap_or("unknown");
    let canister_id = payload["canister_id"].as_str().unwrap_or("");

    if proof_bytes.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "empty proof".to_string()));
    }

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    proof_bytes.hash(&mut h);
    let proof_hash = format!("{:016x}", h.finish());

    let proof_data = json!({
        "proof_hash": proof_hash,
        "canister_id": canister_id,
        "prover_system": prover_system,
        "status": "submitted",
        "timestamp": chrono_wrapper::now_rfc3339(),
    });

    state
        .db
        .store_proof(&proof_hash, &proof_data)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    state.metrics.proofs_submitted.inc();
    info!("Received proof {} for canister {}", proof_hash, canister_id);

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "proof_hash": proof_hash,
            "canister_id": canister_id,
            "prover_system": prover_system,
            "status": "submitted",
        })),
    ))
}

async fn verify_proof(
    State(state): State<AppState>,
    Path(proof_hash): Path<String>,
) -> Result<Json<ProofResponse>, (StatusCode, String)> {
    state.metrics.requests_total.inc();

    match state
        .db
        .get_proof(&proof_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
    {
        Some(proof) => {
            state.metrics.proofs_verified.inc();
            Ok(Json(ProofResponse {
                proof_hash,
                valid: true,
                prover_system: proof["prover_system"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
                timestamp: 0,
            }))
        }
        None => {
            state.metrics.errors_total.inc();
            Err((StatusCode::NOT_FOUND, "proof not found".to_string()))
        }
    }
}

async fn get_proof(
    State(state): State<AppState>,
    Path(proof_hash): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match state
        .db
        .get_proof(&proof_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
    {
        Some(proof) => Ok(Json(proof)),
        None => Err((StatusCode::NOT_FOUND, "proof not found".to_string())),
    }
}

async fn list_provers(State(state): State<AppState>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state.metrics.requests_total.inc();
    let provers = state
        .db
        .list_provers()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
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
    Ok(Json(json!(list)))
}

async fn register_prover(
    State(state): State<AppState>,
    _claims: Claims,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state.metrics.requests_total.inc();

    let node_address = payload["node_address"]
        .as_str()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "node_address required".to_string()))?
        .to_string();
    let gpu_count = payload["gpu_count"].as_u64().unwrap_or(0);
    let stake = payload["stake_amount"].as_u64().unwrap_or(0);
    let prover_id = uuid::Uuid::new_v4().to_string();

    let record = ProverRecord {
        prover_id: prover_id.clone(),
        node_address: node_address.clone(),
        gpu_count,
        stake_amount: stake,
        status: "registered".to_string(),
        registered_at: chrono_wrapper::now_rfc3339(),
    };

    state
        .db
        .insert_prover(&record)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    state.metrics.provers_registered.inc();

    info!(
        "Registered prover {} at {} with {} GPUs, {} ZEN stake",
        prover_id, node_address, gpu_count, stake
    );

    Ok(Json(json!({
        "prover_id": prover_id,
        "status": "registered",
        "node_address": node_address,
        "gpu_count": gpu_count
    })))
}

async fn check_node_ready(rpc_url: &str) -> bool {
    let host_port = rpc_url
        .strip_prefix("http://")
        .or_else(|| rpc_url.strip_prefix("https://"))
        .unwrap_or(rpc_url);

    match tokio::net::TcpStream::connect(host_port).await {
        Ok(_) => {
            info!("✓ Node at {} is reachable", host_port);
            true
        }
        Err(e) => {
            info!("Node check: {} (will operate in demo mode)", e);
            false
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();
    let rpc_url = args
        .node_rpc
        .clone()
        .unwrap_or_else(|| "http://127.0.0.1:9944".to_string());
    let rpc_client = Arc::new(SubstrateRpcClient::new(rpc_url.clone()));

    // Initialize database
    let db = Arc::new(Database::new(&args.data_dir)?);
    info!("✓ Database initialized at {}", args.data_dir);

    // Initialize metrics
    let registry = Registry::new();
    let metrics = Arc::new(Metrics::new(&registry)?);
    info!("✓ Metrics initialized");

    // Initialize circuit breaker for RPC calls (3 failures, 30s timeout)
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 30));
    info!("✓ Circuit breaker initialized");

    // Initialize audit logger
    let audit_log_path = format!("{}/audit.log", args.data_dir);
    let audit_logger = Arc::new(AuditLogger::new(&audit_log_path)?);
    info!("✓ Audit logger initialized at {}", audit_log_path);

    // Initialize payment processor
    let payment_processor = Arc::new(PaymentProcessor::new());
    info!("✓ Payment processor initialized");

    let state = AppState {
        db,
        rpc_url: rpc_url.clone(),
        ready: Arc::new(tokio::sync::Mutex::new(false)),
        rpc_client,
        metrics,
        rate_limit: Arc::new(RateLimitMiddleware::new()),
        circuit_breaker,
        audit_logger,
        payment_processor,
    };

    // Check node readiness
    let ready_check = check_node_ready(&rpc_url).await;
    if ready_check {
        *state.ready.lock().await = true;
        info!("✓ Substrate node is ready at {}", rpc_url);
    } else {
        info!("⚠ Substrate node not responding at {}, proceeding anyway", rpc_url);
    }

    // Build router with auth middleware
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/metrics", get(metrics_endpoint))
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
        // Multi-token payment endpoints (no auth required for estimate, auth for processing)
        .route(
            "/v1/payment/estimate",
            post(payment_api::estimate_price),
        )
        .route(
            "/v1/payment/process",
            post(payment_api::process_payment),
        )
        .route(
            "/v1/payment/:payment_id",
            get(payment_api::get_payment_status),
        )
        .layer(middleware::from_fn(auth::auth_middleware))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(&args.listen).await?;

    info!("🚀 Zenith Gateway listening on {}", args.listen);
    info!("📡 Substrate RPC endpoint: {}", rpc_url);
    info!("💾 Data directory: {}", args.data_dir);

    axum::serve(listener, app).await?;

    Ok(())
}

// Helper modules
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

mod chrono_wrapper {
    pub fn now_rfc3339() -> String {
        ::chrono::Utc::now().to_rfc3339()
    }
}
