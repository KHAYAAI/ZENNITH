use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Execution result from running a canister
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub output: Vec<u8>,
    pub gas_used: u64,
    pub logs: Vec<String>,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Out of gas")]
    OutOfGas,
    #[error("Invalid module")]
    InvalidModule,
    #[error("Memory access violation")]
    MemoryViolation,
}

/// Simplified Wasm executor (Wasmtime integration)
pub struct CanisterExecutor {
    // In production, would contain wasmtime::Engine
    module_cache: std::collections::HashMap<String, Vec<u8>>,
    gas_limit: u64,
}

impl CanisterExecutor {
    pub fn new() -> Result<Self, RuntimeError> {
        Ok(CanisterExecutor {
            module_cache: std::collections::HashMap::new(),
            gas_limit: 10_000_000,
        })
    }

    /// Execute a canister call
    pub fn execute(
        &mut self,
        wasm_hash: &[u8; 32],
        wasm_bytes: &[u8],
        method: &str,
        input: &[u8],
        gas_limit: u64,
    ) -> Result<ExecutionResult, RuntimeError> {
        // Validate Wasm
        if wasm_bytes.is_empty() || !wasm_bytes.starts_with(b"\0asm") {
            return Err(RuntimeError::InvalidModule);
        }

        if gas_limit == 0 {
            return Err(RuntimeError::OutOfGas);
        }

        let hash_str = format!("{:x?}", wasm_hash);

        // Check cache
        if !self.module_cache.contains_key(&hash_str) {
            self.module_cache.insert(hash_str.clone(), wasm_bytes.to_vec());
        }

        // Simulate execution
        let gas_used = (input.len() as u64) * 100;

        if gas_used > gas_limit {
            return Err(RuntimeError::OutOfGas);
        }

        // Generate output (echo input for demo)
        let mut output = format!("Executed method: {} on canister ", method).into_bytes();
        output.extend_from_slice(wasm_hash);

        Ok(ExecutionResult {
            output,
            gas_used,
            logs: vec![format!("Method: {}", method), format!("Gas: {}", gas_used)],
        })
    }

    /// Estimate gas for a call
    pub fn estimate_gas(&self, wasm_bytes: &[u8], method: &str, input: &[u8]) -> u64 {
        let base_cost = 1000u64;
        let code_cost = (wasm_bytes.len() as u64) / 10;
        let method_cost = (method.len() as u64) * 10;
        let input_cost = (input.len() as u64) * 5;

        base_cost + code_cost + method_cost + input_cost
    }
}

impl Default for CanisterExecutor {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let executor = CanisterExecutor::new();
        assert!(executor.is_ok());
    }

    #[test]
    fn test_execute_wasm() {
        let mut executor = CanisterExecutor::new().unwrap();
        let wasm = b"\0asm\x01\x00\x00\x00".to_vec();
        let result = executor.execute(&[0u8; 32], &wasm, "test", b"input", 100_000);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert!(res.gas_used > 0);
    }

    #[test]
    fn test_gas_limit_exceeded() {
        let mut executor = CanisterExecutor::new().unwrap();
        let wasm = b"\0asm\x01\x00\x00\x00".to_vec();
        let result = executor.execute(&[0u8; 32], &wasm, "test", b"input", 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_estimate_gas() {
        let executor = CanisterExecutor::new().unwrap();
        let wasm = b"\0asm\x01\x00\x00\x00".to_vec();
        let estimated = executor.estimate_gas(&wasm, "method", b"input");
        assert!(estimated > 0);
    }
}
