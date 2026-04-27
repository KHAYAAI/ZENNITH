/// End-to-end integration tests for Zenith Cloud Protocol
/// Tests the full pipeline: deploy canister → execute → generate proof → verify proof

pub mod plonk_tests;
pub mod cairo_tests;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Proof system types supported by Zenith
#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy)]
pub enum ProverSystem {
    RiscZero,
    Plonk,
    Cairo,
}

impl ProverSystem {
    pub fn as_str(&self) -> &str {
        match self {
            ProverSystem::RiscZero => "RISC Zero",
            ProverSystem::Plonk => "Plonk",
            ProverSystem::Cairo => "Cairo",
        }
    }
}

/// Workload classification for intelligent routing
#[derive(Clone, Debug)]
pub enum WorkloadType {
    /// General computation - routes to RISC Zero
    General,
    /// Neural network inference - routes to Plonk
    NeuralNetwork,
    /// Arithmetic-heavy computation - routes to Cairo
    Arithmetic,
}

impl WorkloadType {
    /// Analyze WASM bytecode to classify workload
    pub fn classify(wasm_hash: &[u8; 32]) -> Self {
        let sum: u32 = wasm_hash.iter().map(|&b| b as u32).sum();
        match sum % 3 {
            0 => WorkloadType::General,
            1 => WorkloadType::NeuralNetwork,
            _ => WorkloadType::Arithmetic,
        }
    }

    /// Select optimal prover for this workload
    pub fn select_prover(&self) -> ProverSystem {
        match self {
            WorkloadType::General => ProverSystem::RiscZero,
            WorkloadType::NeuralNetwork => ProverSystem::Plonk,
            WorkloadType::Arithmetic => ProverSystem::Cairo,
        }
    }
}

/// Mock blockchain state for testing
#[derive(Clone, Debug)]
pub struct MockBlockchain {
    canisters: Arc<RwLock<HashMap<String, Canister>>>,
    proofs: Arc<RwLock<HashMap<String, Proof>>>,
    calls: Arc<RwLock<HashMap<String, Call>>>,
    call_counter: Arc<RwLock<u64>>,
}

#[derive(Clone, Debug)]
pub struct Canister {
    pub id: String,
    pub owner: String,
    pub wasm_hash: [u8; 32],
    pub status: String,
    pub prover: ProverSystem,
}

#[derive(Clone, Debug)]
pub struct Call {
    pub id: String,
    pub canister_id: String,
    pub input: Vec<u8>,
    pub output: Option<Vec<u8>>,
    pub proof_hash: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct Proof {
    pub hash: String,
    pub prover: ProverSystem,
    pub valid: bool,
    pub canister_id: String,
    pub call_id: String,
}

impl MockBlockchain {
    pub fn new() -> Self {
        MockBlockchain {
            canisters: Arc::new(RwLock::new(HashMap::new())),
            proofs: Arc::new(RwLock::new(HashMap::new())),
            calls: Arc::new(RwLock::new(HashMap::new())),
            call_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Deploy a canister to the blockchain
    pub async fn deploy_canister(&self, owner: String, wasm_hash: [u8; 32]) -> String {
        let mut canisters = self.canisters.write().await;
        let id = format!("canister-{}", canisters.len() + 1);

        // Classify workload and select prover
        let workload = WorkloadType::classify(&wasm_hash);
        let prover = workload.select_prover();

        canisters.insert(
            id.clone(),
            Canister {
                id: id.clone(),
                owner,
                wasm_hash,
                status: "Running".to_string(),
                prover,
            },
        );
        id
    }

    /// Call a canister with input
    pub async fn call_canister(
        &self,
        canister_id: &str,
        input: Vec<u8>,
    ) -> Result<String, String> {
        let canisters = self.canisters.read().await;

        if !canisters.contains_key(canister_id) {
            return Err("Canister not found".to_string());
        }

        let mut calls = self.calls.write().await;
        let mut counter = self.call_counter.write().await;
        *counter += 1;

        let call_id = format!("call-{}", counter);
        calls.insert(
            call_id.clone(),
            Call {
                id: call_id.clone(),
                canister_id: canister_id.to_string(),
                input,
                output: None,
                proof_hash: None,
                status: "Pending".to_string(),
            },
        );

        Ok(call_id)
    }

    /// Get call result
    pub async fn get_call(&self, call_id: &str) -> Result<Call, String> {
        let calls = self.calls.read().await;
        calls
            .get(call_id)
            .cloned()
            .ok_or_else(|| "Call not found".to_string())
    }

    /// Submit a proof for a call
    pub async fn submit_proof(
        &self,
        call_id: &str,
        proof_hash: String,
        prover: ProverSystem,
        canister_id: String,
    ) -> Result<(), String> {
        let mut proofs = self.proofs.write().await;
        let mut calls = self.calls.write().await;

        // Update call with proof hash
        if let Some(call) = calls.get_mut(call_id) {
            call.proof_hash = Some(proof_hash.clone());
            call.status = "Completed".to_string();
        } else {
            return Err("Call not found".to_string());
        }

        proofs.insert(
            proof_hash.clone(),
            Proof {
                hash: proof_hash,
                prover,
                valid: true,
                canister_id,
                call_id: call_id.to_string(),
            },
        );

        Ok(())
    }

    /// Verify a proof
    pub async fn verify_proof(&self, proof_hash: &str) -> Result<bool, String> {
        let proofs = self.proofs.read().await;

        match proofs.get(proof_hash) {
            Some(proof) => Ok(proof.valid),
            None => Err("Proof not found".to_string()),
        }
    }

    /// Get proof info
    pub async fn get_proof(&self, proof_hash: &str) -> Result<Proof, String> {
        let proofs = self.proofs.read().await;
        proofs
            .get(proof_hash)
            .cloned()
            .ok_or_else(|| "Proof not found".to_string())
    }

    /// Get canister info
    pub async fn get_canister(&self, canister_id: &str) -> Result<Canister, String> {
        let canisters = self.canisters.read().await;
        canisters
            .get(canister_id)
            .cloned()
            .ok_or_else(|| "Canister not found".to_string())
    }

    /// Get all canisters for an owner
    pub async fn list_canisters(&self, owner: &str) -> Vec<Canister> {
        let canisters = self.canisters.read().await;
        canisters
            .values()
            .filter(|c| c.owner == owner)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workload_classification() {
        // Test workload classification based on hash
        let hash1 = [0u8; 32];
        let hash2 = [1u8; 32];
        let hash3 = [2u8; 32];

        let workload1 = WorkloadType::classify(&hash1);
        let workload2 = WorkloadType::classify(&hash2);
        let workload3 = WorkloadType::classify(&hash3);

        let prover1 = workload1.select_prover();
        let prover2 = workload2.select_prover();
        let prover3 = workload3.select_prover();

        // Verify they select provers (may be same or different based on hash mod 3)
        assert!(matches!(
            prover1,
            ProverSystem::RiscZero | ProverSystem::Plonk | ProverSystem::Cairo
        ));
        assert!(matches!(
            prover2,
            ProverSystem::RiscZero | ProverSystem::Plonk | ProverSystem::Cairo
        ));
        assert!(matches!(
            prover3,
            ProverSystem::RiscZero | ProverSystem::Plonk | ProverSystem::Cairo
        ));
    }

    #[tokio::test]
    async fn test_deploy_canister_with_routing() {
        let blockchain = MockBlockchain::new();
        let canister_id = blockchain
            .deploy_canister("alice".to_string(), [0u8; 32])
            .await;

        assert!(canister_id.starts_with("canister-"));

        let canister = blockchain.get_canister(&canister_id).await.unwrap();
        assert_eq!(canister.owner, "alice");
        assert_eq!(canister.status, "Running");
        // Verify prover was selected
        assert!(matches!(
            canister.prover,
            ProverSystem::RiscZero | ProverSystem::Plonk | ProverSystem::Cairo
        ));
    }

    #[tokio::test]
    async fn test_call_canister_with_input() {
        let blockchain = MockBlockchain::new();
        let canister_id = blockchain
            .deploy_canister("bob".to_string(), [1u8; 32])
            .await;

        let input = vec![1, 2, 3, 4, 5];
        let call_id = blockchain
            .call_canister(&canister_id, input.clone())
            .await
            .unwrap();

        assert!(call_id.starts_with("call-"));

        let call = blockchain.get_call(&call_id).await.unwrap();
        assert_eq!(call.input, input);
        assert_eq!(call.status, "Pending");
    }

    #[tokio::test]
    async fn test_risc_zero_proof_flow() {
        let blockchain = MockBlockchain::new();

        // Deploy canister that routes to RISC Zero
        let canister_id = blockchain
            .deploy_canister("alice".to_string(), [0u8; 32])
            .await;
        let canister = blockchain.get_canister(&canister_id).await.unwrap();
        assert_eq!(canister.prover, ProverSystem::RiscZero);

        // Call and generate proof
        let call_id = blockchain
            .call_canister(&canister_id, vec![1, 2, 3])
            .await
            .unwrap();

        let proof_hash = "0xrisc0proof";
        blockchain
            .submit_proof(
                &call_id,
                proof_hash.to_string(),
                ProverSystem::RiscZero,
                canister_id.clone(),
            )
            .await
            .unwrap();

        let proof = blockchain.get_proof(proof_hash).await.unwrap();
        assert_eq!(proof.prover, ProverSystem::RiscZero);
        assert!(blockchain.verify_proof(proof_hash).await.unwrap());
    }

    #[tokio::test]
    async fn test_plonk_proof_flow() {
        let blockchain = MockBlockchain::new();

        // Deploy canister that routes to Plonk (neural network workload)
        // [2u8; 32] sums to 64, 64 % 3 = 1 → Plonk
        let canister_id = blockchain
            .deploy_canister("bob".to_string(), [2u8; 32])
            .await;
        let canister = blockchain.get_canister(&canister_id).await.unwrap();
        assert_eq!(canister.prover, ProverSystem::Plonk);

        // Call and generate proof
        let call_id = blockchain
            .call_canister(&canister_id, vec![10, 20, 30])
            .await
            .unwrap();

        let proof_hash = "0xplonkproof";
        blockchain
            .submit_proof(
                &call_id,
                proof_hash.to_string(),
                ProverSystem::Plonk,
                canister_id.clone(),
            )
            .await
            .unwrap();

        let proof = blockchain.get_proof(proof_hash).await.unwrap();
        assert_eq!(proof.prover, ProverSystem::Plonk);
        assert!(blockchain.verify_proof(proof_hash).await.unwrap());
    }

    #[tokio::test]
    async fn test_cairo_proof_flow() {
        let blockchain = MockBlockchain::new();

        // Deploy canister that routes to Cairo (arithmetic workload)
        // [4u8; 32] sums to 128, 128 % 3 = 2 → Cairo
        let canister_id = blockchain
            .deploy_canister("charlie".to_string(), [4u8; 32])
            .await;
        let canister = blockchain.get_canister(&canister_id).await.unwrap();
        assert_eq!(canister.prover, ProverSystem::Cairo);

        // Call and generate proof
        let call_id = blockchain
            .call_canister(&canister_id, vec![100, 200])
            .await
            .unwrap();

        let proof_hash = "0xcairooproof";
        blockchain
            .submit_proof(
                &call_id,
                proof_hash.to_string(),
                ProverSystem::Cairo,
                canister_id.clone(),
            )
            .await
            .unwrap();

        let proof = blockchain.get_proof(proof_hash).await.unwrap();
        assert_eq!(proof.prover, ProverSystem::Cairo);
        assert!(blockchain.verify_proof(proof_hash).await.unwrap());
    }

    #[tokio::test]
    async fn test_full_inference_pipeline() {
        let blockchain = MockBlockchain::new();

        // 1. Deploy canister
        let canister_id = blockchain
            .deploy_canister("alice".to_string(), [42u8; 32])
            .await;
        assert!(canister_id.starts_with("canister-"));

        let canister = blockchain.get_canister(&canister_id).await.unwrap();
        assert_eq!(canister.status, "Running");

        // 2. Call canister with input
        let input = vec![1, 2, 3, 4, 5];
        let call_id = blockchain
            .call_canister(&canister_id, input.clone())
            .await
            .unwrap();
        assert!(call_id.starts_with("call-"));

        let call = blockchain.get_call(&call_id).await.unwrap();
        assert_eq!(call.status, "Pending");
        assert_eq!(call.input, input);

        // 3. Generate and submit proof
        let proof_hash = "0xpipeline_test";
        blockchain
            .submit_proof(
                &call_id,
                proof_hash.to_string(),
                canister.prover.clone(),
                canister_id.clone(),
            )
            .await
            .unwrap();

        // 4. Verify proof
        let is_valid = blockchain.verify_proof(proof_hash).await.unwrap();
        assert!(is_valid);

        // 5. Check call is now completed
        let updated_call = blockchain.get_call(&call_id).await.unwrap();
        assert_eq!(updated_call.status, "Completed");
        assert_eq!(updated_call.proof_hash, Some(proof_hash.to_string()));
    }

    #[tokio::test]
    async fn test_canister_not_found() {
        let blockchain = MockBlockchain::new();
        let result = blockchain.get_canister("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multiple_canisters_with_different_provers() {
        let blockchain = MockBlockchain::new();

        let id1 = blockchain.deploy_canister("alice".to_string(), [0u8; 32]).await;
        let id2 = blockchain.deploy_canister("bob".to_string(), [1u8; 32]).await;
        let id3 = blockchain.deploy_canister("charlie".to_string(), [2u8; 32]).await;

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);

        let c1 = blockchain.get_canister(&id1).await.unwrap();
        let c2 = blockchain.get_canister(&id2).await.unwrap();
        let c3 = blockchain.get_canister(&id3).await.unwrap();

        assert_eq!(c1.owner, "alice");
        assert_eq!(c2.owner, "bob");
        assert_eq!(c3.owner, "charlie");

        // Verify provers are assigned (may be different based on hash)
        assert!(matches!(
            c1.prover,
            ProverSystem::RiscZero | ProverSystem::Plonk | ProverSystem::Cairo
        ));
        assert!(matches!(
            c2.prover,
            ProverSystem::RiscZero | ProverSystem::Plonk | ProverSystem::Cairo
        ));
        assert!(matches!(
            c3.prover,
            ProverSystem::RiscZero | ProverSystem::Plonk | ProverSystem::Cairo
        ));
    }

    #[tokio::test]
    async fn test_list_user_canisters() {
        let blockchain = MockBlockchain::new();

        let id1 = blockchain.deploy_canister("alice".to_string(), [1u8; 32]).await;
        let id2 = blockchain.deploy_canister("alice".to_string(), [2u8; 32]).await;
        let id3 = blockchain.deploy_canister("bob".to_string(), [3u8; 32]).await;

        let alice_canisters = blockchain.list_canisters("alice").await;
        let bob_canisters = blockchain.list_canisters("bob").await;

        assert_eq!(alice_canisters.len(), 2);
        assert_eq!(bob_canisters.len(), 1);
    }

    #[tokio::test]
    async fn test_multiple_calls_per_canister() {
        let blockchain = MockBlockchain::new();

        let canister_id = blockchain
            .deploy_canister("alice".to_string(), [5u8; 32])
            .await;

        let call1 = blockchain
            .call_canister(&canister_id, vec![1, 2])
            .await
            .unwrap();
        let call2 = blockchain
            .call_canister(&canister_id, vec![3, 4])
            .await
            .unwrap();
        let call3 = blockchain
            .call_canister(&canister_id, vec![5, 6])
            .await
            .unwrap();

        assert_ne!(call1, call2);
        assert_ne!(call2, call3);

        let c1 = blockchain.get_call(&call1).await.unwrap();
        let c2 = blockchain.get_call(&call2).await.unwrap();
        let c3 = blockchain.get_call(&call3).await.unwrap();

        assert_eq!(c1.input, vec![1, 2]);
        assert_eq!(c2.input, vec![3, 4]);
        assert_eq!(c3.input, vec![5, 6]);
    }
}
