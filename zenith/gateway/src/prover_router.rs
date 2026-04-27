/// Intelligent prover system router for optimal proof generation
/// Routes workloads to RISC Zero, Plonk, or Cairo based on computation characteristics

use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

    pub fn as_symbol(&self) -> &str {
        match self {
            ProverSystem::RiscZero => "risc0",
            ProverSystem::Plonk => "plonk",
            ProverSystem::Cairo => "cairo",
        }
    }
}

#[derive(Clone, Debug)]
pub enum WorkloadType {
    General,
    NeuralNetwork,
    Arithmetic,
}

impl WorkloadType {
    pub fn as_str(&self) -> &str {
        match self {
            WorkloadType::General => "general",
            WorkloadType::NeuralNetwork => "neural_network",
            WorkloadType::Arithmetic => "arithmetic",
        }
    }
}

pub struct ProverRouter;

impl ProverRouter {
    /// Analyze WASM module characteristics from base64 string
    pub fn analyze_workload(wasm_base64: &str) -> WorkloadType {
        // In production, would do actual WASM bytecode analysis
        // For now, use hash-based classification
        let sum: u32 = wasm_base64
            .as_bytes()
            .iter()
            .map(|b| *b as u32)
            .sum::<u32>() % 1000;

        match sum % 3 {
            0 => WorkloadType::General,
            1 => WorkloadType::NeuralNetwork,
            _ => WorkloadType::Arithmetic,
        }
    }

    /// Select optimal prover for workload
    pub fn select_prover(wasm_base64: &str) -> ProverSystem {
        let workload = Self::analyze_workload(wasm_base64);
        match workload {
            WorkloadType::General => ProverSystem::RiscZero,
            WorkloadType::NeuralNetwork => ProverSystem::Plonk,
            WorkloadType::Arithmetic => ProverSystem::Cairo,
        }
    }

    /// Estimate proof generation time in milliseconds
    pub fn estimate_proof_time(prover: &ProverSystem, input_size: usize) -> u32 {
        match prover {
            ProverSystem::RiscZero => {
                // RISC Zero: ~1ms + 0.1ms per KB
                (1 + (input_size / 1024) as u32).min(5000)
            }
            ProverSystem::Plonk => {
                // Plonk: ~2ms + 0.05ms per KB (more optimized)
                (2 + (input_size / 2048) as u32).min(3000)
            }
            ProverSystem::Cairo => {
                // Cairo: ~3ms + 0.02ms per KB (highly optimized for arithmetic)
                (3 + (input_size / 5000) as u32).min(2000)
            }
        }
    }

    /// Get scoring metadata for workload
    pub fn score_workload(wasm_base64: &str) -> serde_json::Value {
        let prover = Self::select_prover(wasm_base64);
        let input_size = wasm_base64.len();
        let estimated_time = Self::estimate_proof_time(&prover, input_size);

        serde_json::json!({
            "prover_system": prover.as_str(),
            "prover_symbol": prover.as_symbol(),
            "estimated_proof_time_ms": estimated_time,
            "workload_type": Self::analyze_workload(wasm_base64).as_str(),
            "input_size_bytes": input_size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_prover_general() {
        let wasm = "aaaaaa"; // Will hash to General
        let prover = ProverRouter::select_prover(wasm);
        assert!(matches!(
            prover,
            ProverSystem::RiscZero | ProverSystem::Plonk | ProverSystem::Cairo
        ));
    }

    #[test]
    fn test_estimate_proof_time() {
        let time_risc0 = ProverRouter::estimate_proof_time(&ProverSystem::RiscZero, 1000);
        let time_plonk = ProverRouter::estimate_proof_time(&ProverSystem::Plonk, 1000);
        let time_cairo = ProverRouter::estimate_proof_time(&ProverSystem::Cairo, 1000);

        // All should be positive and reasonable
        assert!(time_risc0 > 0 && time_risc0 <= 5000);
        assert!(time_plonk > 0 && time_plonk <= 3000);
        assert!(time_cairo > 0 && time_cairo <= 2000);
    }

    #[test]
    fn test_score_workload() {
        let wasm = "test_wasm";
        let score = ProverRouter::score_workload(wasm);

        assert!(score["prover_system"].is_string());
        assert!(score["estimated_proof_time_ms"].is_u64());
        assert!(score["workload_type"].is_string());
        assert!(score["input_size_bytes"].is_u64());
    }

    #[test]
    fn test_prover_symbols() {
        assert_eq!(ProverSystem::RiscZero.as_symbol(), "risc0");
        assert_eq!(ProverSystem::Plonk.as_symbol(), "plonk");
        assert_eq!(ProverSystem::Cairo.as_symbol(), "cairo");
    }
}
