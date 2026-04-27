use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanisterId(pub String);

impl std::fmt::Display for CanisterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallResult {
    pub call_id: String,
    pub canister_id: String,
    pub status: String,
    pub output: Vec<u8>,
    pub proof_hash: Option<String>,
    pub prover_system: Option<String>,
    pub estimated_latency_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployResult {
    pub canister_id: CanisterId,
    pub tx_hash: String,
    pub block_number: u64,
    pub status: String,
    pub prover_system: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofVerifyResult {
    pub proof_hash: String,
    pub valid: bool,
    pub prover_system: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Gateway error: {0}")]
    GatewayError(String),
    #[error("Invalid canister ID: {0}")]
    InvalidCanister(String),
    #[error("Proof not found: {0}")]
    ProofNotFound(String),
}

/// Zenith SDK client for interacting with the gateway API
pub struct ZenithClient {
    base_url: String,
    http: reqwest::Client,
}

impl ZenithClient {
    pub fn new(base_url: &str) -> Self {
        ZenithClient {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Deploy a WASM canister and return its ID with routing info
    pub async fn deploy_canister(&self, wasm: Vec<u8>) -> Result<DeployResult, SdkError> {
        self.deploy_canister_with_args(wasm, vec![], 1_000_000).await
    }

    pub async fn deploy_canister_with_args(
        &self,
        wasm: Vec<u8>,
        init_args: Vec<u8>,
        cycles: u64,
    ) -> Result<DeployResult, SdkError> {
        let wasm_b64 = BASE64.encode(&wasm);
        let init_b64 = BASE64.encode(&init_args);

        let resp = self
            .http
            .post(format!("{}/v1/canisters", self.base_url))
            .json(&serde_json::json!({
                "wasm_base64": wasm_b64,
                "init_args_base64": init_b64,
                "cycles": cycles,
            }))
            .send()
            .await
            .map_err(|e| SdkError::NetworkError(e.to_string()))?;

        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| SdkError::ParseError(e.to_string()))?;

        if !status.is_success() {
            return Err(SdkError::GatewayError(
                body["error"].as_str().unwrap_or("unknown error").to_string(),
            ));
        }

        Ok(DeployResult {
            canister_id: CanisterId(
                body["canister_id"]
                    .as_str()
                    .ok_or_else(|| SdkError::ParseError("missing canister_id".into()))?
                    .to_string(),
            ),
            tx_hash: body["tx_hash"].as_str().unwrap_or("").to_string(),
            block_number: body["block_number"].as_u64().unwrap_or(0),
            status: body["status"].as_str().unwrap_or("unknown").to_string(),
            prover_system: body["prover_system"].as_str().unwrap_or("unknown").to_string(),
        })
    }

    /// Call a canister method with input bytes
    pub async fn call_canister(
        &self,
        canister_id: &CanisterId,
        method: &str,
        input: Vec<u8>,
    ) -> Result<CallResult, SdkError> {
        self.call_canister_with_gas(canister_id, method, input, 100_000).await
    }

    pub async fn call_canister_with_gas(
        &self,
        canister_id: &CanisterId,
        method: &str,
        input: Vec<u8>,
        gas_limit: u64,
    ) -> Result<CallResult, SdkError> {
        let input_b64 = BASE64.encode(&input);

        let resp = self
            .http
            .post(format!(
                "{}/v1/canisters/{}/call/{}",
                self.base_url, canister_id.0, method
            ))
            .json(&serde_json::json!({
                "method": method,
                "input_base64": input_b64,
                "gas_limit": gas_limit,
            }))
            .send()
            .await
            .map_err(|e| SdkError::NetworkError(e.to_string()))?;

        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| SdkError::ParseError(e.to_string()))?;

        if !status.is_success() {
            return Err(SdkError::GatewayError(
                body["error"].as_str().unwrap_or("call failed").to_string(),
            ));
        }

        Ok(CallResult {
            call_id: body["call_id"].as_str().unwrap_or("").to_string(),
            canister_id: canister_id.0.clone(),
            status: body["status"].as_str().unwrap_or("unknown").to_string(),
            output: vec![],
            proof_hash: None,
            prover_system: body["prover_system"].as_str().map(|s| s.to_string()),
            estimated_latency_ms: body["estimated_latency_ms"].as_u64().unwrap_or(0) as u32,
        })
    }

    /// Verify a proof by hash
    pub async fn verify_proof(&self, proof_hash: &str) -> Result<ProofVerifyResult, SdkError> {
        let resp = self
            .http
            .get(format!("{}/v1/proofs/{}/verify", self.base_url, proof_hash))
            .send()
            .await
            .map_err(|e| SdkError::NetworkError(e.to_string()))?;

        let body: Value = resp.json().await.map_err(|e| SdkError::ParseError(e.to_string()))?;

        Ok(ProofVerifyResult {
            proof_hash: body["proof_hash"].as_str().unwrap_or(proof_hash).to_string(),
            valid: body["valid"].as_bool().unwrap_or(false),
            prover_system: body["prover_system"].as_str().unwrap_or("unknown").to_string(),
        })
    }

    /// Get gateway health status
    pub async fn health(&self) -> Result<HealthStatus, SdkError> {
        let resp = self
            .http
            .get(format!("{}/health", self.base_url))
            .send()
            .await
            .map_err(|e| SdkError::NetworkError(e.to_string()))?;

        let body: Value = resp.json().await.map_err(|e| SdkError::ParseError(e.to_string()))?;

        Ok(HealthStatus {
            status: body["status"].as_str().unwrap_or("unknown").to_string(),
            version: body["version"].as_str().unwrap_or("unknown").to_string(),
            uptime_seconds: body["uptime_seconds"].as_u64().unwrap_or(0),
        })
    }

    /// Get a specific canister's info
    pub async fn get_canister(&self, canister_id: &CanisterId) -> Result<Value, SdkError> {
        let resp = self
            .http
            .get(format!("{}/v1/canisters/{}", self.base_url, canister_id.0))
            .send()
            .await
            .map_err(|e| SdkError::NetworkError(e.to_string()))?;

        if resp.status().as_u16() == 404 {
            return Err(SdkError::InvalidCanister(canister_id.0.clone()));
        }

        resp.json().await.map_err(|e| SdkError::ParseError(e.to_string()))
    }

    /// List all deployed canisters
    pub async fn list_canisters(&self) -> Result<Vec<Value>, SdkError> {
        let resp = self
            .http
            .get(format!("{}/v1/canisters", self.base_url))
            .send()
            .await
            .map_err(|e| SdkError::NetworkError(e.to_string()))?;

        let body: Value = resp.json().await.map_err(|e| SdkError::ParseError(e.to_string()))?;

        Ok(body.as_array().cloned().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = ZenithClient::new("http://localhost:8000");
        assert_eq!(client.base_url, "http://localhost:8000");
    }

    #[test]
    fn test_client_strips_trailing_slash() {
        let client = ZenithClient::new("http://localhost:8000/");
        assert_eq!(client.base_url, "http://localhost:8000");
    }

    #[test]
    fn test_canister_id_display() {
        let id = CanisterId("canister-abc123".to_string());
        assert_eq!(format!("{}", id), "canister-abc123");
    }

    #[test]
    fn test_base64_encoding() {
        let data = b"hello world";
        let encoded = BASE64.encode(data);
        let decoded = BASE64.decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }
}
