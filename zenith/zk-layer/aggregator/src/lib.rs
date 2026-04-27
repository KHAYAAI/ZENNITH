use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProofType {
    RiscZero,
    Plonk,
    Cairo,
    Unknown,
}

impl ProofType {
    fn detect(proof_bytes: &[u8]) -> Self {
        if proof_bytes.starts_with(b"PLNK") {
            ProofType::Plonk
        } else if proof_bytes.starts_with(b"CAIR") {
            ProofType::Cairo
        } else if proof_bytes.len() >= 96 {
            ProofType::RiscZero
        } else {
            ProofType::Unknown
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedProof {
    pub proof_hash: [u8; 32],
    pub proof_types: Vec<ProofType>,
    pub aggregated_bytes: Vec<u8>,
    pub proof_count: usize,
}

/// Aggregate multiple proofs from different systems into a single commitment.
///
/// The aggregate structure is:
///   [count: 4 bytes LE]
///   [merkle_root: 32 bytes]
///   for each proof:
///     [proof_len: 4 bytes LE][proof_bytes...]
///
/// The Merkle root is SHA-256("zenith-aggregate" || first_64_bytes_of_each_proof...).
pub fn aggregate(proofs: Vec<Vec<u8>>) -> AggregatedProof {
    if proofs.is_empty() {
        return AggregatedProof {
            proof_hash: [0u8; 32],
            proof_types: vec![],
            aggregated_bytes: vec![],
            proof_count: 0,
        };
    }

    // Detect proof types
    let proof_types: Vec<ProofType> = proofs.iter().map(|p| ProofType::detect(p)).collect();

    // Build Merkle root: hash all proof commitment prefixes
    let mut root_hasher = Sha256::new();
    root_hasher.update(b"zenith-aggregate");
    root_hasher.update((proofs.len() as u32).to_le_bytes());
    for proof in &proofs {
        // Hash each proof's first 64 bytes (the core commitment portion)
        let commit_len = proof.len().min(64);
        root_hasher.update(&proof[..commit_len]);
    }
    let root: [u8; 32] = root_hasher.finalize().into();

    // Build aggregated bytes: count + root + length-prefixed proofs
    let mut aggregated = Vec::new();
    aggregated.extend_from_slice(&(proofs.len() as u32).to_le_bytes());
    aggregated.extend_from_slice(&root);
    for proof in &proofs {
        aggregated.extend_from_slice(&(proof.len() as u32).to_le_bytes());
        aggregated.extend_from_slice(proof);
    }

    AggregatedProof {
        proof_hash: root,
        proof_types,
        aggregated_bytes: aggregated,
        proof_count: proofs.len(),
    }
}

/// Verify that an aggregated proof is internally consistent by re-deriving the root.
pub fn verify_aggregated(agg: &AggregatedProof) -> bool {
    if agg.aggregated_bytes.len() < 36 {
        return false;
    }

    // Parse count from header
    let count = u32::from_le_bytes(agg.aggregated_bytes[0..4].try_into().unwrap_or([0; 4])) as usize;
    if count == 0 {
        return agg.proof_hash == [0u8; 32];
    }

    // Re-extract individual proofs from the aggregated bytes
    let mut offset = 36; // skip count(4) + root(32)
    let mut proofs = Vec::new();
    for _ in 0..count {
        if offset + 4 > agg.aggregated_bytes.len() {
            return false;
        }
        let len = u32::from_le_bytes(
            agg.aggregated_bytes[offset..offset + 4].try_into().unwrap_or([0; 4])
        ) as usize;
        offset += 4;
        if offset + len > agg.aggregated_bytes.len() {
            return false;
        }
        proofs.push(agg.aggregated_bytes[offset..offset + len].to_vec());
        offset += len;
    }

    // Re-derive root
    let mut root_hasher = Sha256::new();
    root_hasher.update(b"zenith-aggregate");
    root_hasher.update((count as u32).to_le_bytes());
    for proof in &proofs {
        let commit_len = proof.len().min(64);
        root_hasher.update(&proof[..commit_len]);
    }
    let recomputed: [u8; 32] = root_hasher.finalize().into();

    recomputed == agg.proof_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_plonk_proof() -> Vec<u8> {
        let mut p = Vec::new();
        p.extend_from_slice(b"PLNK");
        p.extend_from_slice(&[1u8; 96]); // non-zero commitments
        p
    }

    fn make_cairo_proof() -> Vec<u8> {
        let mut p = Vec::new();
        p.extend_from_slice(b"CAIR");
        p.extend_from_slice(&[2u8; 96]);
        p
    }

    fn make_risc0_proof() -> Vec<u8> {
        vec![3u8; 96]
    }

    #[test]
    fn test_aggregate_empty() {
        let agg = aggregate(vec![]);
        assert_eq!(agg.proof_count, 0);
        assert_eq!(agg.proof_hash, [0u8; 32]);
        assert!(agg.aggregated_bytes.is_empty());
    }

    #[test]
    fn test_aggregate_single_plonk() {
        let proof = make_plonk_proof();
        let agg = aggregate(vec![proof]);
        assert_eq!(agg.proof_count, 1);
        assert_ne!(agg.proof_hash, [0u8; 32]);
        assert_eq!(agg.proof_types[0], ProofType::Plonk);
    }

    #[test]
    fn test_aggregate_mixed_provers() {
        let proofs = vec![make_plonk_proof(), make_cairo_proof(), make_risc0_proof()];
        let agg = aggregate(proofs);
        assert_eq!(agg.proof_count, 3);
        assert_eq!(agg.proof_types[0], ProofType::Plonk);
        assert_eq!(agg.proof_types[1], ProofType::Cairo);
        assert_eq!(agg.proof_types[2], ProofType::RiscZero);
        assert_ne!(agg.proof_hash, [0u8; 32]);
    }

    #[test]
    fn test_verify_aggregated_valid() {
        let proofs = vec![make_plonk_proof(), make_cairo_proof()];
        let agg = aggregate(proofs);
        assert!(verify_aggregated(&agg));
    }

    #[test]
    fn test_verify_aggregated_tampered() {
        let proofs = vec![make_plonk_proof()];
        let mut agg = aggregate(proofs);
        // Tamper with the hash
        agg.proof_hash[0] ^= 0xFF;
        assert!(!verify_aggregated(&agg));
    }

    #[test]
    fn test_aggregate_determinism() {
        let proofs1 = vec![make_plonk_proof(), make_cairo_proof()];
        let proofs2 = vec![make_plonk_proof(), make_cairo_proof()];
        let agg1 = aggregate(proofs1);
        let agg2 = aggregate(proofs2);
        assert_eq!(agg1.proof_hash, agg2.proof_hash);
    }

    #[test]
    fn test_proof_type_detection() {
        assert_eq!(ProofType::detect(b"PLNK\x01\x02"), ProofType::Plonk);
        assert_eq!(ProofType::detect(b"CAIR\x01\x02"), ProofType::Cairo);
        assert_eq!(ProofType::detect(&[0u8; 96]), ProofType::RiscZero);
        assert_eq!(ProofType::detect(b"short"), ProofType::Unknown);
    }
}
