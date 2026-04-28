use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployRequest {
    pub id: String,
    pub wasm_base64: String,
    pub init_args_base64: String,
    pub cycles: u64,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverRecord {
    pub prover_id: String,
    pub node_address: String,
    pub gpu_count: u64,
    pub stake_amount: u64,
    pub status: String,
    pub registered_at: String,
}

pub struct Database {
    db: sled::Db,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, sled::Error> {
        let db = sled::open(path)?;
        Ok(Database { db })
    }

    // Canister operations
    pub fn insert_canister(&self, canister: &DeployRequest) -> Result<(), String> {
        let key = format!("canister:{}", canister.id);
        let value = serde_json::to_vec(canister).map_err(|e| e.to_string())?;
        self.db
            .insert(key, value)
            .map_err(|e| e.to_string())?;
        debug!("Stored canister: {}", canister.id);
        Ok(())
    }

    pub fn get_canister(&self, id: &str) -> Result<Option<DeployRequest>, String> {
        let key = format!("canister:{}", id);
        match self.db.get(&key).map_err(|e| e.to_string())? {
            Some(v) => {
                let canister = serde_json::from_slice(&v).map_err(|e| e.to_string())?;
                Ok(Some(canister))
            }
            None => Ok(None),
        }
    }

    pub fn list_canisters(&self) -> Result<Vec<DeployRequest>, String> {
        let mut canisters = Vec::new();
        for item in self.db.iter() {
            let (_key, value) = item.map_err(|e| e.to_string())?;
            if let Ok(canister) = serde_json::from_slice::<DeployRequest>(&value) {
                canisters.push(canister);
            }
        }
        Ok(canisters)
    }

    pub fn update_canister(&self, canister: &DeployRequest) -> Result<(), String> {
        let key = format!("canister:{}", canister.id);
        let value = serde_json::to_vec(canister).map_err(|e| e.to_string())?;
        self.db
            .insert(key, value)
            .map_err(|e| e.to_string())?;
        debug!("Updated canister: {}", canister.id);
        Ok(())
    }

    pub fn delete_canister(&self, id: &str) -> Result<(), String> {
        let key = format!("canister:{}", id);
        self.db.remove(key).map_err(|e| e.to_string())?;
        debug!("Deleted canister: {}", id);
        Ok(())
    }

    // Prover operations
    pub fn insert_prover(&self, prover: &ProverRecord) -> Result<(), String> {
        let key = format!("prover:{}", prover.prover_id);
        let value = serde_json::to_vec(prover).map_err(|e| e.to_string())?;
        self.db
            .insert(key, value)
            .map_err(|e| e.to_string())?;
        debug!("Stored prover: {}", prover.prover_id);
        Ok(())
    }

    pub fn get_prover(&self, id: &str) -> Result<Option<ProverRecord>, String> {
        let key = format!("prover:{}", id);
        match self.db.get(&key).map_err(|e| e.to_string())? {
            Some(v) => {
                let prover = serde_json::from_slice(&v).map_err(|e| e.to_string())?;
                Ok(Some(prover))
            }
            None => Ok(None),
        }
    }

    pub fn list_provers(&self) -> Result<Vec<ProverRecord>, String> {
        let mut provers = Vec::new();
        for item in self.db.iter() {
            let (_key, value) = item.map_err(|e| e.to_string())?;
            if let Ok(prover) = serde_json::from_slice::<ProverRecord>(&value) {
                provers.push(prover);
            }
        }
        Ok(provers)
    }

    // Proof operations (simple KV storage)
    pub fn store_proof(&self, proof_hash: &str, proof_data: &serde_json::Value) -> Result<(), String> {
        let key = format!("proof:{}", proof_hash);
        let value = serde_json::to_vec(proof_data).map_err(|e| e.to_string())?;
        self.db
            .insert(key, value)
            .map_err(|e| e.to_string())?;
        debug!("Stored proof: {}", proof_hash);
        Ok(())
    }

    pub fn get_proof(&self, proof_hash: &str) -> Result<Option<serde_json::Value>, String> {
        let key = format!("proof:{}", proof_hash);
        match self.db.get(&key).map_err(|e| e.to_string())? {
            Some(v) => {
                let proof = serde_json::from_slice(&v).map_err(|e| e.to_string())?;
                Ok(Some(proof))
            }
            None => Ok(None),
        }
    }

    pub fn flush(&self) -> Result<(), String> {
        self.db.flush().map_err(|e| e.to_string())?;
        debug!("Database flushed");
        Ok(())
    }
}
