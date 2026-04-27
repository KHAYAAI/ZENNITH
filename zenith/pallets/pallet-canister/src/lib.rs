#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Placeholder types for pallet-canister
// Full pallet implementation with frame-support will be completed in Prompt 2

pub type CanisterId = u64;
pub type CallId = u64;

#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct Canister {
    pub id: CanisterId,
    pub owner: String,
    pub wasm_hash: [u8; 32],
    pub state_hash: [u8; 32],
    pub cycles: u128,
    pub status: CanisterStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Copy, Encode, Decode, Serialize, Deserialize, PartialEq, Eq)]
pub enum CanisterStatus {
    Running,
    Stopped,
    Upgrading,
    Deleted,
}

#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct CallRequest {
    pub canister_id: CanisterId,
    pub method: Vec<u8>,
    pub input: Vec<u8>,
    pub gas_limit: u64,
}

#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct CallResult {
    pub output: Vec<u8>,
    pub proof_hash: [u8; 32],
    pub gas_used: u64,
    pub cycles_remaining: u128,
}

#[derive(Debug, Error)]
pub enum CanisterError {
    #[error("Canister not found")]
    CanisterNotFound,
    #[error("Invalid Wasm")]
    InvalidWasm,
    #[error("Insufficient cycles")]
    InsufficientCycles,
}

pub trait CanisterStore {
    fn deploy_canister(&mut self, canister: Canister) -> Result<CanisterId, CanisterError>;
    fn get_canister(&self, id: CanisterId) -> Result<Canister, CanisterError>;
    fn update_canister(&mut self, canister: Canister) -> Result<(), CanisterError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canister_creation() {
        let canister = Canister {
            id: 1,
            owner: "Alice".to_string(),
            wasm_hash: [0u8; 32],
            state_hash: [0u8; 32],
            cycles: 1_000_000,
            status: CanisterStatus::Running,
            created_at: 0,
            updated_at: 0,
        };
        assert_eq!(canister.status, CanisterStatus::Running);
    }
}
