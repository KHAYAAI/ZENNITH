#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Placeholder types for pallet-zk-verifier
// Full pallet implementation with frame-support will be completed in Prompt 4

#[derive(Debug, Clone, Copy, Encode, Decode, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProverSystem {
    RiscZero,
    Plonk,
    Cairo,
}

#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct ZenithProof {
    pub proof_hash: [u8; 32],
    pub computation_hash: [u8; 32],
    pub result_hash: [u8; 32],
    pub prover_system: ProverSystem,
    pub prover_node: String,
    pub timestamp: u64,
    pub canister_id: u64,
    pub call_id: u64,
    pub proof_bytes: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum VerifierError {
    #[error("Invalid proof")]
    InvalidProof,
    #[error("Unauthorized prover")]
    UnauthorizedProver,
    #[error("Verification failed")]
    VerificationFailed,
}

pub trait ProofVerifier {
    fn verify_proof(&self, proof: &ZenithProof) -> Result<bool, VerifierError>;
    fn submit_proof(&mut self, proof: ZenithProof) -> Result<(), VerifierError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_creation() {
        let proof = ZenithProof {
            proof_hash: [0u8; 32],
            computation_hash: [0u8; 32],
            result_hash: [0u8; 32],
            prover_system: ProverSystem::Plonk,
            prover_node: "validator-1".to_string(),
            timestamp: 0,
            canister_id: 1,
            call_id: 1,
            proof_bytes: vec![],
        };
        assert_eq!(proof.prover_system, ProverSystem::Plonk);
    }
}
