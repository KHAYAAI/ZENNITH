#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};

// Placeholder types for pallet-token ($ZEN)
// Full pallet implementation will be completed in Prompt 2

pub const TOKEN_DECIMALS: u8 = 12;
pub const TOKEN_SYMBOL: &str = "ZEN";

#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u128,
}

impl TokenInfo {
    pub fn zen() -> Self {
        TokenInfo {
            name: "Zenith".to_string(),
            symbol: TOKEN_SYMBOL.to_string(),
            decimals: TOKEN_DECIMALS,
            total_supply: 100_000_000 * 10_u128.pow(TOKEN_DECIMALS as u32),
        }
    }
}

#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct Transfer {
    pub from: String,
    pub to: String,
    pub amount: u128,
    pub timestamp: u64,
}
