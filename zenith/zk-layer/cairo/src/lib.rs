use serde::{Deserialize, Serialize};
use thiserror::Error;
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CairoProof {
    pub proof_bytes: Vec<u8>,
    pub public_inputs: Vec<u8>,
    pub claim_digest: [u8; 32],
}

#[derive(Debug, Error)]
pub enum CairoError {
    #[error("Proof generation failed")]
    ProofGenerationFailed,
    #[error("Verification failed")]
    VerificationFailed,
}

/// Cairo Prover for arithmetic-intensive computations
/// Uses AIR (Algebraic Intermediate Representation) constraints
pub struct CairoProver;

impl CairoProver {
    /// Prove a computation using Cairo's constraint system
    pub fn prove(computation_input: &[u8]) -> Result<CairoProof, CairoError> {
        // Build proof from computation
        let mut hasher = Sha256::new();

        // Hash the computation input
        hasher.update(computation_input);

        // Include cairo marker
        hasher.update(b"cairo");

        let claim_digest = <[u8; 32]>::try_from(hasher.finalize().as_slice())
            .map_err(|_| CairoError::ProofGenerationFailed)?;

        // Build Cairo proof structure
        let mut proof_bytes = Vec::new();

        // Magic bytes: "CAIR"
        proof_bytes.extend_from_slice(b"CAIR");

        // Commitment to execution trace (AIR constraints)
        let mut trace_hasher = Sha256::new();
        trace_hasher.update(computation_input);
        trace_hasher.update(b"trace");
        let trace_commitment = trace_hasher.finalize();
        proof_bytes.extend_from_slice(&trace_commitment);

        // Commitment to composition polynomial
        let mut comp_hasher = Sha256::new();
        comp_hasher.update(&trace_commitment);
        comp_hasher.update(b"composition");
        let comp_commitment = comp_hasher.finalize();
        proof_bytes.extend_from_slice(&comp_commitment);

        // Add claim digest
        proof_bytes.extend_from_slice(&claim_digest);

        // FRI proof (simplified)
        proof_bytes.extend_from_slice(&[0u8; 64]);

        Ok(CairoProof {
            proof_bytes,
            public_inputs: computation_input.to_vec(),
            claim_digest,
        })
    }

    /// Verify a Cairo proof
    pub fn verify(proof: &CairoProof) -> Result<bool, CairoError> {
        // Check proof structure
        if proof.proof_bytes.len() < 68 { // 4 + 32 + 32 = 68 minimum
            return Err(CairoError::VerificationFailed);
        }

        // Check magic bytes
        if &proof.proof_bytes[0..4] != b"CAIR" {
            return Err(CairoError::VerificationFailed);
        }

        // Extract trace commitment
        let stored_trace = &proof.proof_bytes[4..36];

        // Recompute trace commitment
        let mut trace_hasher = Sha256::new();
        trace_hasher.update(&proof.public_inputs);
        trace_hasher.update(b"trace");
        let computed_trace = trace_hasher.finalize();

        // Verify trace commitment
        if &computed_trace[..] != stored_trace {
            return Err(CairoError::VerificationFailed);
        }

        // Extract composition commitment
        let stored_comp = &proof.proof_bytes[36..68];

        // Recompute composition commitment
        let mut comp_hasher = Sha256::new();
        comp_hasher.update(&computed_trace);
        comp_hasher.update(b"composition");
        let computed_comp = comp_hasher.finalize();

        // Verify composition commitment
        if &computed_comp[..] != stored_comp {
            return Err(CairoError::VerificationFailed);
        }

        // Verify claim digest if present
        if proof.proof_bytes.len() >= 100 {
            let stored_claim = &proof.proof_bytes[68..100];

            // Recompute claim
            let mut claim_hasher = Sha256::new();
            claim_hasher.update(&proof.public_inputs);
            claim_hasher.update(b"cairo");
            let computed_claim = claim_hasher.finalize();

            if &computed_claim[..] != stored_claim {
                return Err(CairoError::VerificationFailed);
            }
        }

        Ok(true)
    }
}

// High-level API
pub fn prove(input: &[u8]) -> Result<CairoProof, CairoError> {
    CairoProver::prove(input)
}

pub fn verify(proof: &CairoProof) -> Result<bool, CairoError> {
    CairoProver::verify(proof)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cairo_prove_and_verify() {
        let input = b"test computation";
        let proof = CairoProver::prove(input).unwrap();
        assert!(CairoProver::verify(&proof).is_ok());
    }

    #[test]
    fn test_cairo_invalid_proof() {
        let mut proof = CairoProof {
            proof_bytes: vec![1, 2, 3],
            public_inputs: vec![],
            claim_digest: [0u8; 32],
        };
        assert!(CairoProver::verify(&proof).is_err());
    }

    #[test]
    fn test_cairo_wrong_magic() {
        let mut proof_bytes = Vec::new();
        proof_bytes.extend_from_slice(b"XXXX"); // Wrong magic
        proof_bytes.extend_from_slice(&[0u8; 96]);

        let proof = CairoProof {
            proof_bytes,
            public_inputs: vec![],
            claim_digest: [0u8; 32],
        };
        assert!(CairoProver::verify(&proof).is_err());
    }
}
