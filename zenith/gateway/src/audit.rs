use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditLog {
    pub timestamp: String,
    pub user: String,
    pub action: String,
    pub resource: String,
    pub status: String,
    pub details: Option<String>,
    pub ip_address: String,
}

pub struct AuditLogger {
    file_path: String,
    file: Arc<Mutex<std::fs::File>>,
}

impl AuditLogger {
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file_path = path.as_ref().to_string_lossy().to_string();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        Ok(AuditLogger {
            file_path,
            file: Arc::new(Mutex::new(file)),
        })
    }

    pub async fn log(&self, log: AuditLog) {
        match serde_json::to_string(&log) {
            Ok(json_str) => {
                let mut file = self.file.lock().await;
                if let Err(e) = writeln!(file, "{}", json_str) {
                    error!("Failed to write audit log: {}", e);
                } else if let Err(e) = file.flush() {
                    error!("Failed to flush audit log: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to serialize audit log: {}", e);
            }
        }
    }
}

impl AuditLog {
    pub fn new(
        user: String,
        action: String,
        resource: String,
        status: String,
        ip_address: String,
    ) -> Self {
        AuditLog {
            timestamp: Utc::now().to_rfc3339(),
            user,
            action,
            resource,
            status,
            details: None,
            ip_address,
        }
    }

    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_audit_logging() {
        let file = NamedTempFile::new().unwrap();
        let logger = AuditLogger::new(file.path()).unwrap();

        let log = AuditLog::new(
            "user-123".to_string(),
            "deploy_canister".to_string(),
            "canister-abc".to_string(),
            "success".to_string(),
            "127.0.0.1".to_string(),
        );

        logger.log(log).await;

        // Verify file was written
        let contents = std::fs::read_to_string(file.path()).unwrap();
        assert!(contents.contains("deploy_canister"));
        assert!(contents.contains("user-123"));
    }
}
