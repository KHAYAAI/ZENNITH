use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProverSystem {
    Plonk,    // Best for neural networks & polynomial computations
    Cairo,    // Best for arithmetic & decision trees
    RiscZero, // Universal fallback
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAccessPattern {
    pub sequential_reads: u64,
    pub random_reads: u64,
    pub total_memory_ops: u64,
}

impl MemoryAccessPattern {
    pub fn randomness_ratio(&self) -> f64 {
        if self.total_memory_ops == 0 {
            0.0
        } else {
            self.random_reads as f64 / self.total_memory_ops as f64
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadProfile {
    // Instruction densities
    pub neural_network_density: f64,      // Matmul + activation ops
    pub arithmetic_purity: f64,           // Pure arithmetic (add, mul, div)
    pub control_flow_complexity: f64,     // Branch depth / total instructions
    pub memory_access_pattern: MemoryAccessPattern,

    // Metrics
    pub estimated_cycles: u64,
    pub code_size_bytes: usize,
    pub max_call_depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingScores {
    pub plonk: f64,
    pub cairo: f64,
    pub risc_zero: f64,
}

#[derive(Debug, Error)]
pub enum AnalysisError {
    #[error("Invalid WebAssembly")]
    InvalidWasm,
    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),
}

/// Analyzes Wasm bytecode to extract workload characteristics
pub struct WasmAnalyzer;

impl WasmAnalyzer {
    pub fn analyze(wasm_bytes: &[u8]) -> Result<WorkloadProfile, AnalysisError> {
        if wasm_bytes.is_empty() || wasm_bytes.len() > 10_000_000 {
            return Err(AnalysisError::InvalidWasm);
        }

        // Verify Wasm magic bytes
        if wasm_bytes.len() < 4 || &wasm_bytes[0..4] != b"\0asm" {
            return Err(AnalysisError::InvalidWasm);
        }

        // Simple instruction counting (detailed parser would use wasmparser crate)
        let mut matmul_ops = 0u64;
        let mut arithmetic_ops = 0u64;
        let mut branches = 0u64;
        let mut memory_ops = 0u64;
        let mut call_depth = 0u32;

        // Scan for common patterns (simplified)
        for i in 0..wasm_bytes.len().saturating_sub(1) {
            match wasm_bytes[i] {
                0x20 | 0x21 | 0x22 => memory_ops += 1,     // local.get/set
                0x28 | 0x29 => memory_ops += 2,             // i32/i64.load
                0x36 | 0x37 => memory_ops += 2,             // i32/i64.store
                0x6a | 0x7c => arithmetic_ops += 1,         // i32.add, f32.mul
                0x78 | 0xa0 => matmul_ops += 2,             // potential vector ops
                0x05 | 0x0b => branches += 1,               // block/end
                0x0c => call_depth = call_depth.saturating_add(1),
                _ => {}
            }
        }

        let total_ops = arithmetic_ops + matmul_ops + memory_ops + branches;
        let total_ops = if total_ops == 0 { 1 } else { total_ops };

        Ok(WorkloadProfile {
            neural_network_density: (matmul_ops as f64) / total_ops as f64,
            arithmetic_purity: (arithmetic_ops as f64) / total_ops as f64,
            control_flow_complexity: (branches as f64) / total_ops as f64,
            memory_access_pattern: MemoryAccessPattern {
                sequential_reads: memory_ops / 2,
                random_reads: memory_ops / 4,
                total_memory_ops: memory_ops,
            },
            estimated_cycles: (total_ops as u64) * 1000,
            code_size_bytes: wasm_bytes.len(),
            max_call_depth: call_depth,
        })
    }
}

/// Scores workloads against each proving system
pub struct SystemScorer;

impl SystemScorer {
    /// Score for Plonk/Halo2 - excels at neural networks and polynomial constraints
    fn score_plonk(profile: &WorkloadProfile) -> f64 {
        let nn_bonus = profile.neural_network_density * 2.0; // 2x weight for NN ops
        let complexity_penalty = profile.control_flow_complexity * 1.5;
        let call_penalty = (profile.max_call_depth as f64) * 0.1;

        (nn_bonus - complexity_penalty - call_penalty).max(0.0).min(1.0)
    }

    /// Score for Cairo - excels at pure arithmetic
    fn score_cairo(profile: &WorkloadProfile) -> f64 {
        let arithmetic_bonus = profile.arithmetic_purity * 1.8;
        let control_bonus = if profile.control_flow_complexity < 0.3 { 0.4 } else { 0.0 };
        let nn_penalty = profile.neural_network_density * 0.8;
        let memory_penalty = profile.memory_access_pattern.randomness_ratio() * 0.5;

        (arithmetic_bonus + control_bonus - nn_penalty - memory_penalty)
            .max(0.0)
            .min(1.0)
    }

    /// Score for RISC Zero - universal but slower
    fn score_risc_zero(_profile: &WorkloadProfile) -> f64 {
        1.0 // Always available as fallback
    }

    pub fn score(profile: &WorkloadProfile) -> RoutingScores {
        RoutingScores {
            plonk: Self::score_plonk(profile),
            cairo: Self::score_cairo(profile),
            risc_zero: Self::score_risc_zero(profile),
        }
    }
}

/// Dispatcher selects the best proving system based on scores
pub struct Dispatcher;

impl Dispatcher {
    const PLONK_THRESHOLD: f64 = 0.70;
    const CAIRO_THRESHOLD: f64 = 0.60;

    pub fn select(scores: &RoutingScores) -> ProverSystem {
        if scores.plonk >= Self::PLONK_THRESHOLD {
            ProverSystem::Plonk
        } else if scores.cairo >= Self::CAIRO_THRESHOLD {
            ProverSystem::Cairo
        } else {
            ProverSystem::RiscZero
        }
    }

    pub fn route(wasm_bytes: &[u8]) -> Result<ProverSystem, AnalysisError> {
        let profile = WasmAnalyzer::analyze(wasm_bytes)?;
        let scores = SystemScorer::score(&profile);
        Ok(Self::select(&scores))
    }
}

/// Batch manager groups incoming requests for efficient proving
pub struct BatchManager {
    plonk_queue: Vec<InferenceRequest>,
    cairo_queue: Vec<InferenceRequest>,
    risc_zero_queue: Vec<InferenceRequest>,
    batch_size: usize,
    batch_timeout_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub id: String,
    pub canister_id: u64,
    pub wasm: Vec<u8>,
    pub input: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Batch {
    pub system: ProverSystem,
    pub requests: Vec<InferenceRequest>,
}

impl BatchManager {
    pub fn new(batch_size: usize, batch_timeout_ms: u64) -> Self {
        BatchManager {
            plonk_queue: Vec::new(),
            cairo_queue: Vec::new(),
            risc_zero_queue: Vec::new(),
            batch_size,
            batch_timeout_ms,
        }
    }

    pub fn add_request(&mut self, req: InferenceRequest, system: ProverSystem) {
        match system {
            ProverSystem::Plonk => self.plonk_queue.push(req),
            ProverSystem::Cairo => self.cairo_queue.push(req),
            ProverSystem::RiscZero => self.risc_zero_queue.push(req),
        }
    }

    pub fn get_ready_batches(&mut self) -> Vec<Batch> {
        let mut batches = Vec::new();

        // Flush full batches
        while self.plonk_queue.len() >= self.batch_size {
            let batch = Batch {
                system: ProverSystem::Plonk,
                requests: self.plonk_queue.drain(0..self.batch_size).collect(),
            };
            batches.push(batch);
        }

        while self.cairo_queue.len() >= self.batch_size {
            let batch = Batch {
                system: ProverSystem::Cairo,
                requests: self.cairo_queue.drain(0..self.batch_size).collect(),
            };
            batches.push(batch);
        }

        while self.risc_zero_queue.len() >= self.batch_size {
            let batch = Batch {
                system: ProverSystem::RiscZero,
                requests: self.risc_zero_queue.drain(0..self.batch_size).collect(),
            };
            batches.push(batch);
        }

        batches
    }

    pub fn queue_depth(&self) -> HashMap<ProverSystem, usize> {
        let mut depths = HashMap::new();
        depths.insert(ProverSystem::Plonk, self.plonk_queue.len());
        depths.insert(ProverSystem::Cairo, self.cairo_queue.len());
        depths.insert(ProverSystem::RiscZero, self.risc_zero_queue.len());
        depths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_validation() {
        let valid_wasm = b"\0asm\x01\x00\x00\x00".to_vec();
        let result = WasmAnalyzer::analyze(&valid_wasm);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_wasm() {
        let invalid = b"notawasm".to_vec();
        let result = WasmAnalyzer::analyze(&invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_routing_neural_network() {
        let profile = WorkloadProfile {
            neural_network_density: 0.8,
            arithmetic_purity: 0.1,
            control_flow_complexity: 0.1,
            memory_access_pattern: MemoryAccessPattern {
                sequential_reads: 100,
                random_reads: 10,
                total_memory_ops: 110,
            },
            estimated_cycles: 10000,
            code_size_bytes: 5000,
            max_call_depth: 2,
        };

        let scores = SystemScorer::score(&profile);
        let selected = Dispatcher::select(&scores);

        // High NN density should select Plonk
        assert_eq!(selected, ProverSystem::Plonk);
        assert!(scores.plonk > 0.7);
    }

    #[test]
    fn test_routing_arithmetic() {
        let profile = WorkloadProfile {
            neural_network_density: 0.1,
            arithmetic_purity: 0.8,
            control_flow_complexity: 0.1,
            memory_access_pattern: MemoryAccessPattern {
                sequential_reads: 100,
                random_reads: 10,
                total_memory_ops: 110,
            },
            estimated_cycles: 5000,
            code_size_bytes: 3000,
            max_call_depth: 1,
        };

        let scores = SystemScorer::score(&profile);
        let selected = Dispatcher::select(&scores);

        // High arithmetic purity should select Cairo
        assert_eq!(selected, ProverSystem::Cairo);
        assert!(scores.cairo > 0.6);
    }

    #[test]
    fn test_batch_manager() {
        let mut manager = BatchManager::new(10, 1000);

        for i in 0..25 {
            let req = InferenceRequest {
                id: format!("req-{}", i),
                canister_id: 1,
                wasm: vec![0u8; 100],
                input: vec![],
            };
            manager.add_request(req, ProverSystem::Plonk);
        }

        let batches = manager.get_ready_batches();
        // Should have 2 batches of 10 each
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].requests.len(), 10);
        assert_eq!(batches[1].requests.len(), 10);

        // 5 requests should remain
        assert_eq!(manager.plonk_queue.len(), 5);
    }
}
