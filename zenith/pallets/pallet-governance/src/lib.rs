#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};

// Placeholder types for pallet-governance
// Full pallet implementation will be completed in Prompt 2

#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    pub proposer: String,
    pub title: String,
    pub description: String,
    pub votes_for: u128,
    pub votes_against: u128,
    pub created_at: u64,
    pub deadline: u64,
}

#[derive(Debug, Clone, Copy, Encode, Decode, Serialize, Deserialize)]
pub enum VoteDirection {
    For,
    Against,
    Abstain,
}

impl Default for VoteDirection {
    fn default() -> Self {
        VoteDirection::Abstain
    }
}
