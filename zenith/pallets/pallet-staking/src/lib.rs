#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};

// Placeholder types for pallet-staking
// Full pallet implementation will be completed in Prompt 2

#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct StakeInfo {
    pub validator: String,
    pub amount: u128,
    pub staked_at: u64,
}

#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct ProverStakeInfo {
    pub prover: String,
    pub amount: u128,
    pub staked_at: u64,
    pub proofs_verified: u64,
    pub slashes: u64,
}

impl ProverStakeInfo {
    pub fn new(prover: String, amount: u128, staked_at: u64) -> Self {
        ProverStakeInfo {
            prover,
            amount,
            staked_at,
            proofs_verified: 0,
            slashes: 0,
        }
    }
}
