/// Persistent payment record storage using sled
use sled::{Db, Error as SledError};
use crate::payment::PaymentRecord;
use std::sync::Arc;

pub struct PaymentDb {
    db: Arc<Db>,
}

impl PaymentDb {
    /// Open or create payment records database
    pub fn new(path: &str) -> Result<Self, SledError> {
        let db = sled::open(path)?;
        Ok(PaymentDb {
            db: Arc::new(db),
        })
    }

    /// Store a payment record
    pub fn insert(&self, payment_id: &str, record: &PaymentRecord) -> Result<(), String> {
        let json = serde_json::to_string(record)
            .map_err(|e| format!("Serialization failed: {}", e))?;
        self.db
            .insert(payment_id.as_bytes(), json.as_bytes())
            .map_err(|e| format!("Database insert failed: {}", e))?;
        Ok(())
    }

    /// Retrieve a payment record
    pub fn get(&self, payment_id: &str) -> Result<Option<PaymentRecord>, String> {
        match self.db
            .get(payment_id.as_bytes())
            .map_err(|e| format!("Database get failed: {}", e))?
        {
            Some(bytes) => {
                let json = String::from_utf8(bytes.to_vec())
                    .map_err(|e| format!("Invalid UTF-8: {}", e))?;
                let record = serde_json::from_str(&json)
                    .map_err(|e| format!("Deserialization failed: {}", e))?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    /// List all payment IDs
    pub fn list_all(&self) -> Result<Vec<String>, String> {
        let mut ids = Vec::new();
        for result in self.db.iter() {
            let (key, _) = result.map_err(|e| format!("Database iteration failed: {}", e))?;
            let id = String::from_utf8(key.to_vec())
                .map_err(|e| format!("Invalid UTF-8: {}", e))?;
            ids.push(id);
        }
        Ok(ids)
    }

    /// Flush database to disk
    pub fn flush(&self) -> Result<(), String> {
        self.db
            .flush()
            .map_err(|e| format!("Database flush failed: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::payment::{PaymentStatus, PaymentToken};
    use tempfile::TempDir;

    #[test]
    fn test_payment_db_insert_and_get() {
        let tmpdir = TempDir::new().unwrap();
        let db = PaymentDb::new(tmpdir.path().to_str().unwrap()).unwrap();

        let record = PaymentRecord {
            id: "payment-123".to_string(),
            user: "0x123".to_string(),
            payment_token: PaymentToken::Usdc,
            payment_amount: "100".to_string(),
            settlement_token: PaymentToken::Zen,
            settlement_amount: "5000".to_string(),
            exchange_rate: 50.0,
            tx_hash: Some("0xabc".to_string()),
            status: PaymentStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
        };

        db.insert("payment-123", &record).unwrap();
        let retrieved = db.get("payment-123").unwrap().unwrap();

        assert_eq!(retrieved.id, "payment-123");
        assert_eq!(retrieved.user, "0x123");
        assert_eq!(retrieved.exchange_rate, 50.0);
    }

    #[test]
    fn test_payment_db_get_nonexistent() {
        let tmpdir = TempDir::new().unwrap();
        let db = PaymentDb::new(tmpdir.path().to_str().unwrap()).unwrap();

        let result = db.get("nonexistent").unwrap();
        assert!(result.is_none());
    }
}
