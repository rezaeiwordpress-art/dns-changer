use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::models::DnsMode;

pub const BACKUP_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackupRecordStatus {
    Protected,
    ApplyFailedPreserved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRecord {
    pub schema_version: u32,
    pub adapter_guid: String,
    pub interface_index_at_backup: u32,
    pub adapter_name_at_backup: String,
    pub original_dns_mode: DnsMode,
    pub original_ipv4_dns_servers: Vec<String>,
    pub backup_timestamp: String,
    pub backup_status: BackupRecordStatus,
    pub checksum: String,
}

impl BackupRecord {
    pub fn compute_checksum(
        schema_version: u32,
        adapter_guid: &str,
        interface_index_at_backup: u32,
        adapter_name_at_backup: &str,
        original_dns_mode: DnsMode,
        original_ipv4_dns_servers: &[String],
        backup_timestamp: &str,
        backup_status: BackupRecordStatus,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(schema_version.to_le_bytes());
        hasher.update(b"|");
        hasher.update(adapter_guid.as_bytes());
        hasher.update(b"|");
        hasher.update(interface_index_at_backup.to_le_bytes());
        hasher.update(b"|");
        hasher.update(adapter_name_at_backup.as_bytes());
        hasher.update(b"|");
        let mode_str = match original_dns_mode {
            DnsMode::Automatic => "automatic",
            DnsMode::Static => "static",
        };
        hasher.update(mode_str.as_bytes());
        hasher.update(b"|");
        for srv in original_ipv4_dns_servers {
            hasher.update(srv.as_bytes());
            hasher.update(b",");
        }
        hasher.update(b"|");
        hasher.update(backup_timestamp.as_bytes());
        hasher.update(b"|");
        let status_str = match backup_status {
            BackupRecordStatus::Protected => "protected",
            BackupRecordStatus::ApplyFailedPreserved => "apply_failed_preserved",
        };
        hasher.update(status_str.as_bytes());

        hex::encode(hasher.finalize())
    }

    pub fn verify_checksum(&self) -> bool {
        let expected = Self::compute_checksum(
            self.schema_version,
            &self.adapter_guid,
            self.interface_index_at_backup,
            &self.adapter_name_at_backup,
            self.original_dns_mode,
            &self.original_ipv4_dns_servers,
            &self.backup_timestamp,
            self.backup_status,
        );
        self.checksum == expected
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatusSummary {
    pub adapter_guid: String,
    pub has_backup: bool,
    pub is_valid: bool,
    pub backup: Option<BackupRecord>,
    pub integrity_error: Option<String>,
}
