#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};

// Placeholder types for Zenith Runtime
// Full Substrate runtime implementation will be completed in Prompt 2

#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct GenesisConfig {
    pub validators: Vec<String>,
    pub initial_balances: Vec<(String, u128)>,
    pub sudo_key: Option<String>,
}

#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub enum PalletConfig {
    Canister,
    ZkVerifier,
    Governance,
    Staking,
    Token,
}

pub struct RuntimeConfig {
    pub genesis: GenesisConfig,
    pub pallets: Vec<PalletConfig>,
}

impl RuntimeConfig {
    pub fn new() -> Self {
        RuntimeConfig {
            genesis: GenesisConfig {
                validators: vec!["Alice".to_string(), "Bob".to_string()],
                initial_balances: vec![
                    ("Alice".to_string(), 1_000_000_000_000_000_000),
                    ("Bob".to_string(), 1_000_000_000_000_000_000),
                ],
                sudo_key: Some("Alice".to_string()),
            },
            pallets: vec![
                PalletConfig::Canister,
                PalletConfig::ZkVerifier,
                PalletConfig::Governance,
                PalletConfig::Staking,
                PalletConfig::Token,
            ],
        }
    }

    pub fn development() -> Self {
        Self::new()
    }

    pub fn testnet() -> Self {
        let mut config = Self::new();
        // Testnet-specific configuration
        config
    }

    pub fn mainnet() -> Self {
        let mut config = Self::new();
        // Mainnet-specific configuration
        config.genesis.sudo_key = None; // No sudo on mainnet
        config
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_development_config() {
        let config = RuntimeConfig::development();
        assert_eq!(config.genesis.validators.len(), 2);
        assert!(config.genesis.sudo_key.is_some());
    }

    #[test]
    fn test_mainnet_config() {
        let config = RuntimeConfig::mainnet();
        assert!(config.genesis.sudo_key.is_none());
    }
}
