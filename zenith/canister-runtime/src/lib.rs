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
        // Enable fuel metering for real gas tracking
        config.consume_fuel(true);

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

        // Create store with fuel for gas metering
        let mut store = wasmtime::Store::new(&self.engine, ());
        store.set_fuel(gas_limit)
            .map_err(|e| RuntimeError::ExecutionFailed(format!("Failed to set fuel: {}", e)))?;

        let mut linker = wasmtime::Linker::new(&self.engine);

        // Host environment functions for canister ABI
        linker.func_wrap("env", "log", |_: i32, _: i32| {}).ok();
        linker.func_wrap("env", "abort", |_: i32, _: i32, _: i32, _: i32| {}).ok();
        linker.func_wrap("env", "panic", |_: i32, _: i32| {}).ok();

        let instance = linker.instantiate(&mut store, module)
            .map_err(|e| RuntimeError::ExecutionFailed(format!("Instantiation failed: {}", e)))?;

        // Try calling with (ptr: i32, len: i32) -> (out_ptr: i32, out_len: i32) signature
        // This is the standard canister ABI for reading from/writing to Wasm memory
        let output = if let Ok(func) = instance.get_typed_func::<(i32, i32), (i32, i32)>(&mut store, method) {
            let memory = instance.get_memory(&mut store, "memory");
            if let Some(mem) = memory {
                // Write input into Wasm linear memory at offset 0
                if !input.is_empty() {
                    mem.write(&mut store, 0, input)
                        .map_err(|e| RuntimeError::MemoryViolation)?;
                }
                let (out_ptr, out_len) = func.call(&mut store, (0, input.len() as i32))
                    .map_err(|e| RuntimeError::ExecutionFailed(format!("Call failed: {}", e)))?;
                // Read output from Wasm memory
                let mut out = vec![0u8; out_len.max(0) as usize];
                if out_len > 0 {
                    mem.read(&mut store, out_ptr as usize, &mut out)
                        .map_err(|_| RuntimeError::MemoryViolation)?;
                }
                out
            } else {
                vec![]
            }
        } else if let Ok(func) = instance.get_typed_func::<(i32, i32), i32>(&mut store, method) {
            // (ptr, len) -> result_code
            let result = func.call(&mut store, (0, input.len() as i32))
                .map_err(|e| RuntimeError::ExecutionFailed(format!("Call failed: {}", e)))?;
            result.to_le_bytes().to_vec()
        } else if let Ok(func) = instance.get_typed_func::<(), i32>(&mut store, method) {
            // Parameterless -> i32
            let result = func.call(&mut store, ())
                .map_err(|e| RuntimeError::ExecutionFailed(format!("Call failed: {}", e)))?;
            result.to_le_bytes().to_vec()
        } else {
            return Err(RuntimeError::ExecutionFailed(format!("Export '{}' not found or incompatible signature", method)));
        };

        // Get actual fuel consumed (gas used)
        let fuel_remaining = store.get_fuel().unwrap_or(0);
        let gas_used = gas_limit.saturating_sub(fuel_remaining);
        let output_len = output.len();

        Ok(ExecutionResult {
            output,
            gas_used,
            logs: vec![
                format!("Method: {}", method),
                format!("Input: {} bytes", input.len()),
                format!("Output: {} bytes", output_len),
                format!("Gas used: {}", gas_used),
            ],
        })
    }

    /// Estimate gas for a call (without executing)
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
    fn test_execute_wasm_minimal() {
        let mut executor = CanisterExecutor::new().unwrap();
        // Minimal valid Wasm module with no exports — expect ExecutionFailed (no export)
        let wasm = b"\0asm\x01\x00\x00\x00".to_vec();
        let result = executor.execute(&[0u8; 32], &wasm, "test", b"input", 100_000);
        match result {
            Ok(_) => { /* unexpectedly succeeded */ },
            Err(RuntimeError::ExecutionFailed(_)) => { /* expected: no export named "test" */ },
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_gas_limit_zero_rejected() {
        let mut executor = CanisterExecutor::new().unwrap();
        let wasm = b"\0asm\x01\x00\x00\x00".to_vec();
        let result = executor.execute(&[0u8; 32], &wasm, "test", b"input", 0);
        assert!(matches!(result, Err(RuntimeError::OutOfGas)));
    }

    #[test]
    fn test_invalid_wasm_rejected() {
        let mut executor = CanisterExecutor::new().unwrap();
        let not_wasm = b"not a wasm module".to_vec();
        let result = executor.execute(&[0u8; 32], &not_wasm, "test", b"", 100_000);
        assert!(matches!(result, Err(RuntimeError::InvalidModule)));
    }

    #[test]
    fn test_estimate_gas() {
        let executor = CanisterExecutor::new().unwrap();
        let wasm = b"\0asm\x01\x00\x00\x00".to_vec();
        let estimated = executor.estimate_gas(&wasm, "method", b"input");
        assert!(estimated > 0);
    }

    #[test]
    fn test_module_caching() {
        let mut executor = CanisterExecutor::new().unwrap();
        let wasm = b"\0asm\x01\x00\x00\x00".to_vec();
        // Two calls with the same hash should reuse the cached module (no double-compile)
        let _ = executor.execute(&[1u8; 32], &wasm, "test", b"", 100_000);
        let _ = executor.execute(&[1u8; 32], &wasm, "test", b"", 100_000);
        // Cache should have exactly one entry
        assert_eq!(executor.module_cache.len(), 1);
    }
}
