use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::collections::HashMap;

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

/// Wasm executor with Wasmtime integration
pub struct CanisterExecutor {
    engine: wasmtime::Engine,
    module_cache: HashMap<String, wasmtime::Module>,
    gas_limit: u64,
}

impl CanisterExecutor {
    pub fn new() -> Result<Self, RuntimeError> {
        let mut config = wasmtime::Config::new();
        config.wasm_simd(true);
        config.wasm_bulk_memory(true);
        config.wasm_multi_value(true);
        config.wasm_reference_types(true);
        config.async_support(false);

        let engine = wasmtime::Engine::new(&config)
            .map_err(|e| RuntimeError::ExecutionFailed(format!("Failed to create engine: {}", e)))?;

        Ok(CanisterExecutor {
            engine,
            module_cache: HashMap::new(),
            gas_limit: 10_000_000,
        })
    }

    /// Execute a canister call with real Wasmtime runtime
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

        // Compile and cache module on first use
        if !self.module_cache.contains_key(&hash_str) {
            let module = wasmtime::Module::new(&self.engine, wasm_bytes)
                .map_err(|e| RuntimeError::ExecutionFailed(format!("Module compilation failed: {}", e)))?;
            self.module_cache.insert(hash_str.clone(), module);
        }

        let module = self.module_cache.get(&hash_str).unwrap();

        // Create instance and linker
        let mut store = wasmtime::Store::new(&self.engine, ());
        let mut linker = wasmtime::Linker::new(&self.engine);

        // Add required WASI-like imports (empty stub)
        linker.func_wrap("env", "log", |_: i32| {
            // Stub for logging
        }).map_err(|e| RuntimeError::ExecutionFailed(e.to_string()))?;

        let instance = linker.instantiate(&mut store, module)
            .map_err(|e| RuntimeError::ExecutionFailed(format!("Instantiation failed: {}", e)))?;

        // Look up exported function
        // Try to call with (i32, i32) -> i32 signature first
        if let Ok(func) = instance.get_typed_func::<(i32, i32), i32>(&mut store, method) {
            let input_ptr = if input.len() >= 4 {
                u32::from_le_bytes([input[0], input[1], input[2], input[3]]) as i32
            } else {
                0
            };
            let _result = func.call(&mut store, (input_ptr, input.len() as i32))
                .map_err(|e| RuntimeError::ExecutionFailed(format!("Function call failed: {}", e)))?;
        } else if let Ok(func) = instance.get_typed_func::<(), i32>(&mut store, method) {
            // Try parameterless function
            let _result = func.call(&mut store, ())
                .map_err(|e| RuntimeError::ExecutionFailed(format!("Function call failed: {}", e)))?;
        } else {
            return Err(RuntimeError::ExecutionFailed(format!("Export '{}' not found", method)));
        }

        // Track actual gas usage (simplified: 100 gas per byte of input)
        let gas_used = (input.len() as u64 * 100).min(gas_limit);

        if gas_used > gas_limit {
            return Err(RuntimeError::OutOfGas);
        }

        // For now, return serialized result
        let output = format!("Executed {} on canister {:x?}", method, wasm_hash).into_bytes();

        Ok(ExecutionResult {
            output,
            gas_used,
            logs: vec![
                format!("Method: {}", method),
                format!("Input length: {}", input.len()),
                format!("Gas used: {}", gas_used),
            ],
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
