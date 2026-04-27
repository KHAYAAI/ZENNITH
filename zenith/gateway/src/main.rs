use axum::{
    routing::get,
    Json, Router,
};
use clap::Parser;
use serde::Serialize;
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "Zenith Gateway")]
#[command(about = "Zenith Cloud API Gateway")]
struct Args {
    #[arg(short, long, default_value = "0.0.0.0:8000")]
    listen: SocketAddr,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/metrics", get(metrics));

    let listener = tokio::net::TcpListener::bind(args.listen)
        .await
        .expect("Failed to bind address");

    info!("Zenith Gateway listening on {}", args.listen);

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}

async fn metrics() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "uptime_seconds": 0,
        "requests_total": 0,
        "errors_total": 0,
    }))
}
