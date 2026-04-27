/// Private Voting Canister
///
/// This canister implements anonymous voting with verifiable tallying.
/// Features:
/// - Votes are encrypted before submission
/// - Tallying happens privately without revealing individual votes
/// - ZK proof ensures correct counting and no vote tampering
///
/// Workload characteristics:
/// - Moderate control flow (vote validation, aggregation logic)
/// - Encryption/hash operations
/// - Random memory access patterns
/// - Not optimal for any single ZK system, so falls back to RISC Zero
///
/// This workload demonstrates when RISC Zero is the right choice:
/// When workload doesn't fit Plonk or Cairo profiles.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedVote {
    pub voter_id: u32,
    pub encrypted_choice: Vec<u8>,
    pub nonce: [u8; 16],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ballot {
    pub ballot_id: u32,
    pub options: Vec<String>,
    pub encrypted_votes: Vec<EncryptedVote>,
    pub total_voters: u32,
}

#[derive(Debug, Clone)]
pub struct TallyResult {
    pub option_name: String,
    pub vote_count: u32,
    pub percentage: u32, // 0-100
}

/// Simple XOR cipher for demonstration (in production, use real encryption)
pub struct SimpleEncryption;

impl SimpleEncryption {
    pub fn encrypt(data: u8, key: [u8; 16]) -> u8 {
        let mut result = data;
        for k in key.iter() {
            result ^= k;
        }
        result
    }

    pub fn decrypt(encrypted: u8, key: [u8; 16]) -> u8 {
        Self::encrypt(encrypted, key) // XOR is symmetric
    }

    pub fn hash(data: &[u8]) -> [u8; 32] {
        let mut hash = [0u8; 32];
        for (i, byte) in data.iter().enumerate() {
            hash[i % 32] = hash[i % 32].wrapping_add(*byte);
        }
        hash
    }
}

/// Vote aggregator with privacy preservation
pub struct VotingSystem;

impl VotingSystem {
    /// Validate and decrypt a vote
    pub fn validate_vote(vote: &EncryptedVote, options: &[String]) -> Result<usize, String> {
        if vote.encrypted_choice.is_empty() {
            return Err("Empty vote".to_string());
        }

        // Simulate decryption
        let choice_byte = vote.encrypted_choice[0];
        let option_id = (choice_byte as usize) % options.len();

        Ok(option_id)
    }

    /// Tally votes without revealing individual choices
    pub fn tally(ballot: &Ballot) -> Vec<TallyResult> {
        let mut counts: HashMap<usize, u32> = HashMap::new();

        // Count votes (in production, this would be done in ZK circuit)
        for vote in &ballot.encrypted_votes {
            if let Ok(option_id) = Self::validate_vote(vote, &ballot.options) {
                *counts.entry(option_id).or_insert(0) += 1;
            }
        }

        // Generate results
        let total: u32 = counts.values().sum();
        let mut results = Vec::new();

        for (option_id, count) in counts {
            if option_id < ballot.options.len() {
                let percentage = if total > 0 {
                    (count * 100) / total
                } else {
                    0
                };

                results.push(TallyResult {
                    option_name: ballot.options[option_id].clone(),
                    vote_count: count,
                    percentage,
                });
            }
        }

        results.sort_by(|a, b| b.vote_count.cmp(&a.vote_count));
        results
    }

    /// Verify election integrity
    pub fn verify_integrity(ballot: &Ballot) -> Result<(), String> {
        // Check vote counts match expected
        if (ballot.encrypted_votes.len() as u32) > ballot.total_voters {
            return Err("Too many votes for registered voters".to_string());
        }

        // Verify no duplicate votes (in production, use commitment scheme)
        let mut seen_voters = std::collections::HashSet::new();
        for vote in &ballot.encrypted_votes {
            if !seen_voters.insert(vote.voter_id) {
                return Err(format!("Duplicate vote from voter {}", vote.voter_id));
            }
        }

        Ok(())
    }
}

/// Canister entrypoint: Process voting
pub fn process_election(input_json: &str) -> String {
    let ballot: Ballot = match serde_json::from_str(input_json) {
        Ok(b) => b,
        Err(_) => {
            // Default test ballot
            Ballot {
                ballot_id: 1,
                options: vec!["Option A".to_string(), "Option B".to_string(), "Option C".to_string()],
                encrypted_votes: vec![
                    EncryptedVote {
                        voter_id: 1,
                        encrypted_choice: vec![0],
                        nonce: [0u8; 16],
                    },
                    EncryptedVote {
                        voter_id: 2,
                        encrypted_choice: vec![1],
                        nonce: [1u8; 16],
                    },
                    EncryptedVote {
                        voter_id: 3,
                        encrypted_choice: vec![0],
                        nonce: [2u8; 16],
                    },
                ],
                total_voters: 3,
            }
        }
    };

    // Verify integrity
    let integrity_ok = VotingSystem::verify_integrity(&ballot).is_ok();

    // Tally votes
    let results = VotingSystem::tally(&ballot);

    // Build response
    let mut response = String::from("{\"integrity_verified\": ");
    response.push_str(&integrity_ok.to_string());
    response.push_str(", \"results\": [");

    for (i, result) in results.iter().enumerate() {
        if i > 0 {
            response.push(',');
        }
        response.push_str(&format!(
            r#"{{"option": "{}", "votes": {}, "percentage": {}%}}"#,
            result.option_name, result.vote_count, result.percentage
        ));
    }

    response.push_str("]}");
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vote_encryption() {
        let key = [1u8; 16];
        let original = 42u8;
        let encrypted = SimpleEncryption::encrypt(original, key);
        let decrypted = SimpleEncryption::decrypt(encrypted, key);
        assert_eq!(original, decrypted);
    }

    #[test]
    fn test_voting_tally() {
        let ballot = Ballot {
            ballot_id: 1,
            options: vec!["A".to_string(), "B".to_string()],
            encrypted_votes: vec![
                EncryptedVote {
                    voter_id: 1,
                    encrypted_choice: vec![0],
                    nonce: [0u8; 16],
                },
                EncryptedVote {
                    voter_id: 2,
                    encrypted_choice: vec![0],
                    nonce: [1u8; 16],
                },
                EncryptedVote {
                    voter_id: 3,
                    encrypted_choice: vec![1],
                    nonce: [2u8; 16],
                },
            ],
            total_voters: 3,
        };

        let results = VotingSystem::tally(&ballot);
        assert_eq!(results[0].vote_count, 2); // Option A wins with 2 votes
        assert_eq!(results[1].vote_count, 1); // Option B has 1 vote
    }

    #[test]
    fn test_integrity_check() {
        let ballot = Ballot {
            ballot_id: 1,
            options: vec!["A".to_string()],
            encrypted_votes: vec![],
            total_voters: 10,
        };

        assert!(VotingSystem::verify_integrity(&ballot).is_ok());
    }

    #[test]
    fn test_process_election() {
        let json = r#"{
            "ballot_id": 1,
            "options": ["A", "B", "C"],
            "encrypted_votes": [],
            "total_voters": 100
        }"#;

        let result = process_election(json);
        assert!(result.contains("results"));
    }
}
