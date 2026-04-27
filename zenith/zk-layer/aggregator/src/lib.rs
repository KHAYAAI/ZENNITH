use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofType {
    RiscZero,
    Plonk,
    Cairo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedProof {
    pub proof_hash: [u8; 32],
    pub proof_types: Vec<ProofType>,
    pub aggregated_bytes: Vec<u8>,
}

pub fn aggregate(_proofs: Vec<Vec<u8>>) -> AggregatedProof {
    // Placeholder implementation
    AggregatedProof {
        proof_hash: [0u8; 32],
        proof_types: vec![],
        aggregated_bytes: vec![],
    }
}
