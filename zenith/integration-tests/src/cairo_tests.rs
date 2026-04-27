/// Integration tests for Cairo prover
/// Tests arithmetic-heavy computation proof generation and verification

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct ArithmeticComputation {
    pub program_id: String,
    pub inputs: Vec<u64>,
    pub output: u64,
    pub proof_bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct CairoProverSimulator {
    cache: Arc<RwLock<HashMap<String, ArithmeticComputation>>>,
}

impl CairoProverSimulator {
    pub fn new() -> Self {
        CairoProverSimulator {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Simulate Cairo proof generation for arithmetic computation
    pub async fn prove_computation(
        &self,
        program_id: String,
        inputs: Vec<u64>,
    ) -> Result<ArithmeticComputation, String> {
        // Simulate arithmetic computation
        let output: u64 = inputs.iter().fold(1u64, |acc, &x| acc.wrapping_mul(x));

        // Generate deterministic proof based on program and inputs
        let mut proof = Vec::new();
        proof.extend_from_slice(b"CAIR");

        // Hash program
        let mut prog_hasher = sha2::Sha256::new();
        use sha2::Digest;
        prog_hasher.update(program_id.as_bytes());
        let prog_hash = prog_hasher.finalize();
        proof.extend_from_slice(&prog_hash[..]);

        // Hash inputs
        let mut input_hasher = sha2::Sha256::new();
        for val in &inputs {
            input_hasher.update(val.to_le_bytes());
        }
        let input_hash = input_hasher.finalize();
        proof.extend_from_slice(&input_hash[..]);

        // Hash output
        let mut output_hasher = sha2::Sha256::new();
        output_hasher.update(output.to_le_bytes());
        let output_hash = output_hasher.finalize();
        proof.extend_from_slice(&output_hash[..]);

        let computation = ArithmeticComputation {
            program_id: program_id.clone(),
            inputs,
            output,
            proof_bytes: proof,
        };

        // Cache the computation
        let mut cache = self.cache.write().await;
        cache.insert(program_id, computation.clone());

        Ok(computation)
    }

    /// Verify a Cairo proof
    pub async fn verify_proof(&self, computation: &ArithmeticComputation) -> Result<bool, String> {
        // Check proof structure
        if computation.proof_bytes.len() < 100 {
            return Err("Proof too short".to_string());
        }

        // Verify magic bytes
        if &computation.proof_bytes[0..4] != b"CAIR" {
            return Err("Invalid proof magic".to_string());
        }

        Ok(true)
    }

    /// Batch multiple computations
    pub async fn prove_batch(
        &self,
        program_id: String,
        batch: Vec<Vec<u64>>,
    ) -> Result<Vec<ArithmeticComputation>, String> {
        let mut results = Vec::new();
        for inputs in batch {
            let computation = self
                .prove_computation(program_id.clone(), inputs)
                .await?;
            results.push(computation);
        }
        Ok(results)
    }

    /// Execute and prove a loop-based computation
    pub async fn prove_loop_computation(
        &self,
        program_id: String,
        initial: u64,
        iterations: u64,
    ) -> Result<ArithmeticComputation, String> {
        let mut result = initial;
        let inputs = vec![initial, iterations];

        // Simulate loop: result *= 2 for each iteration
        for _ in 0..iterations {
            result = result.wrapping_mul(2);
        }

        let mut proof = Vec::new();
        proof.extend_from_slice(b"CAIR");

        let mut hasher = sha2::Sha256::new();
        use sha2::Digest;
        hasher.update(program_id.as_bytes());
        hasher.update(initial.to_le_bytes());
        hasher.update(iterations.to_le_bytes());
        hasher.update(result.to_le_bytes());

        let hash = hasher.finalize();
        proof.extend_from_slice(&hash[..]);
        proof.extend_from_slice(&[0u8; 64]); // FRI proof placeholder

        Ok(ArithmeticComputation {
            program_id: program_id.clone(),
            inputs,
            output: result,
            proof_bytes: proof,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cairo_single_computation() {
        let prover = CairoProverSimulator::new();
        let program_id = "prog-mul".to_string();
        let inputs = vec![2, 3, 5];

        let computation = prover
            .prove_computation(program_id, inputs.clone())
            .await
            .unwrap();

        assert_eq!(computation.inputs, inputs);
        assert_eq!(computation.output, 30); // 1 * 2 * 3 * 5
        assert!(computation.proof_bytes.len() > 0);
        assert_eq!(&computation.proof_bytes[0..4], b"CAIR");
    }

    #[tokio::test]
    async fn test_cairo_verify_proof() {
        let prover = CairoProverSimulator::new();
        let program_id = "prog-sum".to_string();
        let inputs = vec![10, 20, 30];

        let computation = prover
            .prove_computation(program_id, inputs)
            .await
            .unwrap();

        let is_valid = prover.verify_proof(&computation).await.unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_cairo_batch_computation() {
        let prover = CairoProverSimulator::new();
        let program_id = "prog-batch".to_string();

        let batch = vec![
            vec![1, 2, 3],
            vec![2, 3, 4],
            vec![3, 4, 5],
        ];

        let results = prover
            .prove_batch(program_id, batch.clone())
            .await
            .unwrap();

        assert_eq!(results.len(), batch.len());
        assert_eq!(results[0].output, 6); // 1 * 2 * 3
        assert_eq!(results[1].output, 24); // 2 * 3 * 4
        assert_eq!(results[2].output, 60); // 3 * 4 * 5
    }

    #[tokio::test]
    async fn test_cairo_invalid_proof() {
        let prover = CairoProverSimulator::new();

        let computation = ArithmeticComputation {
            program_id: "invalid".to_string(),
            inputs: vec![],
            output: 0,
            proof_bytes: vec![1, 2, 3],
        };

        let result = prover.verify_proof(&computation).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cairo_loop_computation() {
        let prover = CairoProverSimulator::new();
        let program_id = "prog-loop".to_string();

        // Compute 5 * 2^3 = 40
        let computation = prover
            .prove_loop_computation(program_id, 5, 3)
            .await
            .unwrap();

        assert_eq!(computation.output, 40);
    }

    #[tokio::test]
    async fn test_cairo_proof_determinism() {
        let prover = CairoProverSimulator::new();
        let program_id = "prog-det".to_string();
        let inputs = vec![7, 11];

        let comp1 = prover
            .prove_computation(program_id.clone(), inputs.clone())
            .await
            .unwrap();

        let prover2 = CairoProverSimulator::new();
        let comp2 = prover2
            .prove_computation(program_id, inputs)
            .await
            .unwrap();

        // Proofs should be identical for same input
        assert_eq!(comp1.proof_bytes, comp2.proof_bytes);
        assert_eq!(comp1.output, comp2.output);
    }

    #[tokio::test]
    async fn test_cairo_different_programs() {
        let prover = CairoProverSimulator::new();
        let inputs = vec![2, 3];

        let comp1 = prover
            .prove_computation("prog-a".to_string(), inputs.clone())
            .await
            .unwrap();

        let comp2 = prover
            .prove_computation("prog-b".to_string(), inputs)
            .await
            .unwrap();

        // Different programs should have different proofs
        assert_ne!(comp1.proof_bytes, comp2.proof_bytes);
    }

    #[tokio::test]
    async fn test_cairo_large_computation() {
        let prover = CairoProverSimulator::new();
        let program_id = "prog-large".to_string();

        // Test with larger numbers
        let inputs: Vec<u64> = (1..=10).collect();
        let computation = prover
            .prove_computation(program_id, inputs)
            .await
            .unwrap();

        // Result should be 10! = 3628800
        assert_eq!(computation.output, 3628800);
    }

    #[tokio::test]
    async fn test_cairo_overflow_safety() {
        let prover = CairoProverSimulator::new();
        let program_id = "prog-overflow".to_string();

        // Test with values that would overflow
        let inputs = vec![u64::MAX, 2];
        let computation = prover
            .prove_computation(program_id, inputs)
            .await
            .unwrap();

        // Should handle overflow gracefully (wrapping)
        assert_eq!(computation.output, u64::MAX.wrapping_mul(2));
    }
}
