/// Substrate RPC client for Zenith Gateway
/// Handles JSON-RPC 2.0 calls to Substrate blockchain node

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone)]
pub struct SubstrateRpcClient {
    rpc_url: String,
    request_id: std::sync::Arc<AtomicU64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcResponse<T> {
    jsonrpc: String,
    result: Option<T>,
    error: Option<RpcError>,
    id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcError {
    code: i32,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployResponse {
    pub canister_id: String,
    pub block_hash: String,
    pub block_number: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallResponse {
    pub call_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofSubmitResponse {
    pub proof_hash: String,
    pub block_hash: String,
}

impl SubstrateRpcClient {
    pub fn new(rpc_url: String) -> Self {
        SubstrateRpcClient {
            rpc_url,
            request_id: std::sync::Arc::new(AtomicU64::new(1)),
        }
    }

    fn next_request_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Deploy a canister via pallet_canister::deploy_canister extrinsic
    pub async fn deploy_canister(
        &self,
        wasm_base64: &str,
        init_args_base64: &str,
    ) -> Result<DeployResponse, String> {
        // In production, would make actual RPC call to blockchain
        // For now, return mock response that simulates blockchain state

        // Generate deterministic IDs based on input
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        (wasm_base64, init_args_base64).hash(&mut hasher);
        let hash = hasher.finish();

        let canister_id = format!("canister-{:x}", hash % 10000);
        let block_hash = format!("0x{:064x}", hash);
        let block_number = (hash % 1000) as u32;

        Ok(DeployResponse {
            canister_id,
            block_hash,
            block_number,
        })
    }

    /// Call a canister via pallet_canister::call_canister extrinsic
    pub async fn call_canister(
        &self,
        canister_id: &str,
        method: &str,
        input_base64: &str,
    ) -> Result<CallResponse, String> {
        // Validate canister exists
        if !canister_id.starts_with("canister-") {
            return Err("Invalid canister ID".to_string());
        }

        // Generate call ID
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        (canister_id, method, input_base64).hash(&mut hasher);
        let hash = hasher.finish();

        let call_id = format!("call-{:x}", hash);

        Ok(CallResponse {
            call_id,
            status: "Queued".to_string(),
        })
    }

    /// Submit a proof via pallet_zk_verifier::submit_proof extrinsic
    pub async fn submit_proof(
        &self,
        proof_bytes_base64: &str,
        prover_system: &str,
    ) -> Result<ProofSubmitResponse, String> {
        // Generate proof hash (would be calculated on blockchain)
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        (proof_bytes_base64, prover_system).hash(&mut hasher);
        let hash = hasher.finish();

        let proof_hash = format!("0x{:064x}", hash);
        let block_hash = format!("0x{:064x}", hash.wrapping_mul(31));

        Ok(ProofSubmitResponse {
            proof_hash,
            block_hash,
        })
    }

    /// Get the current block number
    pub async fn get_block_number(&self) -> Result<u32, String> {
        // In production, would call system_number RPC method
        // For now, return a mock value
        Ok(42)
    }

    /// Check if node is healthy
    pub async fn health_check(&self) -> Result<bool, String> {
        // In production, would call system_health RPC method
        // For now, return success
        Ok(true)
    }

    /// Get canister info from blockchain
    pub async fn get_canister(&self, canister_id: &str) -> Result<Value, String> {
        if !canister_id.starts_with("canister-") {
            return Err("Canister not found".to_string());
        }

        // Return mock canister info
        Ok(json!({
            "id": canister_id,
            "owner": "5GrwvaEF5zXb26Fz9rcQkQSL6e3h4fMhT9HjPQM6YYRq", // Alice
            "status": "Running",
            "cycles": 1000000000u64,
            "created_at_block": 12345u32,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_deploy_canister() {
        let client = SubstrateRpcClient::new("http://localhost:9944".to_string());
        let response = client
            .deploy_canister("AQIDBA==", "")
            .await
            .unwrap();

        assert!(response.canister_id.starts_with("canister-"));
        assert!(response.block_hash.starts_with("0x"));
    }

    #[tokio::test]
    async fn test_call_canister() {
        let client = SubstrateRpcClient::new("http://localhost:9944".to_string());
        let response = client
            .call_canister("canister-123", "infer", "AQIDBA==")
            .await
            .unwrap();

        assert!(response.call_id.starts_with("call-"));
        assert_eq!(response.status, "Queued");
    }

    #[tokio::test]
    async fn test_submit_proof() {
        let client = SubstrateRpcClient::new("http://localhost:9944".to_string());
        let response = client
            .submit_proof("AQIDBA==", "RiscZero")
            .await
            .unwrap();

        assert!(response.proof_hash.starts_with("0x"));
        assert!(response.block_hash.starts_with("0x"));
    }

    #[tokio::test]
    async fn test_get_block_number() {
        let client = SubstrateRpcClient::new("http://localhost:9944".to_string());
        let block = client.get_block_number().await.unwrap();
        assert!(block > 0);
    }

    #[tokio::test]
    async fn test_health_check() {
        let client = SubstrateRpcClient::new("http://localhost:9944".to_string());
        let is_healthy = client.health_check().await.unwrap();
        assert!(is_healthy);
    }
}
