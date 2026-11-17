use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use crate::backup::models::{
    BackupRecord, BackupRecordStatus, BackupStatusSummary, BACKUP_SCHEMA_VERSION,
};
use crate::dns::validation::{
    validate_adapter_guid, validate_dns_list, validate_ipv4, MAX_IPV4_DNS_SERVERS,
};
use crate::errors::AppError;
use crate::models::{DnsMode, NetworkAdapter};
use crate::storage::config::write_bytes_atomic;

pub struct BackupStore {
    backups_dir: PathBuf,
}

impl BackupStore {
    pub fn new(app_local_data_dir: &Path) -> Self {
        Self {
            backups_dir: app_local_data_dir.join("backups"),
        }
    }

    fn backup_file_path(&self, canonical_guid: &str) -> PathBuf {
        let sanitized = canonical_guid
            .trim_matches(|c| c == '{' || c == '}')
            .to_lowercase();
        self.backups_dir.join(format!("{sanitized}.json"))
    }

    /// Loads and validates an existing backup for `adapter_guid`.
    /// Never matches by interface index; strictly validates canonical adapter GUID and SHA-256 checksum.
    pub fn load_backup(&self, adapter_guid: &str) -> Result<BackupRecord, AppError> {
        let canonical_guid = validate_adapter_guid(adapter_guid)?;
        let path = self.backup_file_path(&canonical_guid);

        if !path.exists() {
            return Err(AppError::BackupNotFound {
                guid: canonical_guid,
            });
        }

        let raw = fs::read_to_string(&path).map_err(|e| AppError::BackupCorruption {
            detail: format!("Failed to read backup file {}: {e}", path.display()),
        })?;

        let record: BackupRecord =
            serde_json::from_str(&raw).map_err(|e| AppError::BackupCorruption {
                detail: format!("Failed to parse backup JSON {}: {e}", path.display()),
            })?;

        self.validate_record(&record, &canonical_guid)?;
        Ok(record)
    }

    fn validate_record(
        &self,
        record: &BackupRecord,
        expected_guid: &str,
    ) -> Result<(), AppError> {
        if record.schema_version != BACKUP_SCHEMA_VERSION {
            return Err(AppError::BackupCorruption {
                detail: format!(
                    "Unsupported backup schema version {} (expected {})",
                    record.schema_version, BACKUP_SCHEMA_VERSION
                ),
            });
        }

        let record_guid = validate_adapter_guid(&record.adapter_guid).map_err(|_| {
            AppError::BackupCorruption {
                detail: "Backup record contains an invalid adapter GUID".into(),
            }
        })?;

        if record_guid != expected_guid {
            return Err(AppError::BackupCorruption {
                detail: format!(
                    "Backup GUID mismatch: file belongs to {record_guid}, requested {expected_guid}"
                ),
            });
        }

        if !record.verify_checksum() {
            return Err(AppError::BackupCorruption {
                detail: format!("Checksum mismatch for backup of adapter {expected_guid}"),
            });
        }

        if record.original_dns_mode == DnsMode::Static {
            if record.original_ipv4_dns_servers.is_empty() {
                return Err(AppError::BackupCorruption {
                    detail: "Static DNS backup has an empty server list".into(),
                });
            }
            if record.original_ipv4_dns_servers.len() > MAX_IPV4_DNS_SERVERS {
                return Err(AppError::BackupCorruption {
                    detail: format!(
                        "Static DNS backup exceeds maximum supported server count ({MAX_IPV4_DNS_SERVERS})"
                    ),
                });
            }
            for addr in &record.original_ipv4_dns_servers {
                validate_ipv4(addr).map_err(|_| AppError::BackupCorruption {
                    detail: format!("Backup contains invalid IPv4 DNS address '{addr}'"),
                })?;
            }
        }

        Ok(())
    }

    /// Returns backup status without throwing on missing or corrupted backups so the UI can inspect health.
    pub fn get_status(&self, adapter_guid: &str) -> Result<BackupStatusSummary, AppError> {
        let canonical_guid = validate_adapter_guid(adapter_guid)?;
        let path = self.backup_file_path(&canonical_guid);
        if !path.exists() {
            return Ok(BackupStatusSummary {
                adapter_guid: canonical_guid,
                has_backup: false,
                is_valid: true,
                backup: None,
                integrity_error: None,
            });
        }

        match self.load_backup(&canonical_guid) {
            Ok(record) => Ok(BackupStatusSummary {
                adapter_guid: canonical_guid,
                has_backup: true,
                is_valid: true,
                backup: Some(record),
                integrity_error: None,
            }),
            Err(AppError::BackupCorruption { detail }) => Ok(BackupStatusSummary {
                adapter_guid: canonical_guid,
                has_backup: true,
                is_valid: false,
                backup: None,
                integrity_error: Some(detail),
            }),
            Err(e) => Err(e),
        }
    }

    /// Lists all backup files currently stored on disk so the UI can reconstruct active DNS state
    /// across all adapters on application startup.
    pub fn list_backups(&self) -> Result<Vec<BackupStatusSummary>, AppError> {
        if !self.backups_dir.exists() {
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(&self.backups_dir).map_err(|e| AppError::StorageError {
            detail: format!(
                "Failed to read backups directory {}: {e}",
                self.backups_dir.display()
            ),
        })?;

        let mut summaries = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem.starts_with('.') {
                    continue;
                }
                if let Ok(canonical) = validate_adapter_guid(stem) {
                    if let Ok(summary) = self.get_status(&canonical) {
                        summaries.push(summary);
                    }
                }
            }
        }
        Ok(summaries)
    }

    /// Saves the adapter's original DNS configuration BEFORE any modification.
    /// Critical safeguard:
    /// - If a valid backup already exists for this adapter GUID, it is NEVER overwritten.
    /// - If an existing backup file is corrupt, returns `BackupCorruption` instead of destroying evidence.
    /// Returns `(record, created_now)`.
    pub fn ensure_original_backup(
        &self,
        adapter: &NetworkAdapter,
    ) -> Result<(BackupRecord, bool), AppError> {
        let canonical_guid = validate_adapter_guid(&adapter.guid)?;
        let path = self.backup_file_path(&canonical_guid);

        if path.exists() {
            let existing = self.load_backup(&canonical_guid)?;
            info!(
                adapter_guid = %canonical_guid,
                "Existing valid original DNS backup preserved; refusing overwrite"
            );
            return Ok((existing, false));
        }

        if let Some(reason) = &adapter.dns_mode_indeterminate_reason {
            return Err(AppError::DnsModeIndeterminate {
                guid: canonical_guid,
                detail: reason.clone(),
            });
        }

        // For Automatic DNS mode, Windows DHCP may report active router DNS servers in
        // `current_ipv4_dns_servers`; we store an empty static list for Automatic mode
        // or the exact static addresses for Static mode (falling back to registry static servers
        // if `current_ipv4_dns_servers` was temporarily empty on a static adapter).
        let original_servers = match adapter.dns_mode {
            DnsMode::Automatic => Vec::new(),
            DnsMode::Static => {
                let candidate_servers = if !adapter.current_ipv4_dns_servers.is_empty() {
                    &adapter.current_ipv4_dns_servers
                } else {
                    &adapter.static_ipv4_dns_reg_servers
                };
                if candidate_servers.is_empty() {
                    return Err(AppError::DnsModeIndeterminate {
                        guid: canonical_guid,
                        detail: "Adapter reports Static DNS mode, but no static IPv4 DNS servers were reported".into(),
                    });
                }
                let validated = validate_dns_list(candidate_servers)?;
                validated.into_iter().map(|ip| ip.to_string()).collect()
            }
        };

        let timestamp = Utc::now().to_rfc3339();
        let status = BackupRecordStatus::Protected;
        let checksum = BackupRecord::compute_checksum(
            BACKUP_SCHEMA_VERSION,
            &canonical_guid,
            adapter.interface_index,
            &adapter.name,
            adapter.dns_mode,
            &original_servers,
            &timestamp,
            status,
        );

        let record = BackupRecord {
            schema_version: BACKUP_SCHEMA_VERSION,
            adapter_guid: canonical_guid.clone(),
            interface_index_at_backup: adapter.interface_index,
            adapter_name_at_backup: adapter.name.clone(),
            original_dns_mode: adapter.dns_mode,
            original_ipv4_dns_servers: original_servers,
            backup_timestamp: timestamp,
            backup_status: status,
            checksum,
        };

        // Never persist a backup record that `validate_record` would later reject.
        self.validate_record(&record, &canonical_guid)?;
        self.write_record(&path, &record)?;
        info!(
            adapter_guid = %canonical_guid,
            dns_mode = ?record.original_dns_mode,
            "Created original DNS backup"
        );
        Ok((record, true))
    }

    pub fn mark_apply_failed_preserved(&self, adapter_guid: &str) -> Result<(), AppError> {
        let canonical_guid = validate_adapter_guid(adapter_guid)?;
        let mut record = self.load_backup(&canonical_guid)?;
        record.backup_status = BackupRecordStatus::ApplyFailedPreserved;
        record.checksum = BackupRecord::compute_checksum(
            record.schema_version,
            &record.adapter_guid,
            record.interface_index_at_backup,
            &record.adapter_name_at_backup,
            record.original_dns_mode,
            &record.original_ipv4_dns_servers,
            &record.backup_timestamp,
            record.backup_status,
        );
        let path = self.backup_file_path(&canonical_guid);
        self.write_record(&path, &record)?;
        warn!(adapter_guid = %canonical_guid, "Marked backup as ApplyFailedPreserved");
        Ok(())
    }

    /// Removes the backup file ONLY after verified restoration (or when cancelling before any
    /// modification was ever attempted for a brand-new backup).
    pub fn remove_backup(&self, adapter_guid: &str) -> Result<(), AppError> {
        let canonical_guid = validate_adapter_guid(adapter_guid)?;
        let path = self.backup_file_path(&canonical_guid);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| AppError::StorageError {
                detail: format!("Failed to remove backup {}: {e}", path.display()),
            })?;
        }
        Ok(())
    }

    /// Quarantines a corrupted backup file by renaming it to `<name>.corrupt-<timestamp>`.
    /// Refuses to discard valid backups or non-existent backups.
    pub fn discard_corrupt_backup(&self, adapter_guid: &str) -> Result<PathBuf, AppError> {
        let canonical_guid = validate_adapter_guid(adapter_guid)?;
        let path = self.backup_file_path(&canonical_guid);

        if !path.exists() {
            return Err(AppError::BackupNotFound {
                guid: canonical_guid,
            });
        }

        match self.load_backup(&canonical_guid) {
            Ok(_) => {
                return Err(AppError::InvalidInput {
                    detail: "Refusing to discard a valid original DNS backup".into(),
                });
            }
            Err(AppError::BackupCorruption { .. }) => {
                let file_name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("backup.json");
                let stamp = Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
                let quarantine_path = path.with_file_name(format!("{file_name}.corrupt-{stamp}"));
                fs::rename(&path, &quarantine_path).map_err(|e| AppError::StorageError {
                    detail: format!(
                        "Failed to quarantine corrupt backup {} to {}: {e}",
                        path.display(),
                        quarantine_path.display()
                    ),
                })?;
                warn!(
                    adapter_guid = %canonical_guid,
                    quarantine_path = %quarantine_path.display(),
                    "Quarantined corrupt DNS backup file"
                );
                Ok(quarantine_path)
            }
            Err(e) => Err(e),
        }
    }

    fn write_record(&self, path: &Path, record: &BackupRecord) -> Result<(), AppError> {
        let json = serde_json::to_vec_pretty(record).map_err(|e| AppError::StorageError {
            detail: format!("Failed to serialize backup record: {e}"),
        })?;
        write_bytes_atomic(path, &json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AdapterStatus, AdapterType, DhcpStatus};
    use uuid::Uuid;

    fn sample_adapter(guid: &str, idx: u32, mode: DnsMode, dns: Vec<&str>) -> NetworkAdapter {
        NetworkAdapter {
            guid: guid.to_string(),
            interface_index: idx,
            name: format!("Adapter-{idx}"),
            interface_description: "Test NIC".into(),
            adapter_type: AdapterType::Ethernet,
            operational_status: AdapterStatus::Up,
            ipv4_addresses: vec!["192.168.1.50".into()],
            ipv4_gateway: Some("192.168.1.1".into()),
            current_ipv4_dns_servers: dns.into_iter().map(String::from).collect(),
            dns_mode: mode,
            dhcp_status: DhcpStatus::Enabled,
            mac_address: Some("00-11-22-33-44-55".into()),
            link_speed: Some("1 Gbps".into()),
            dns_mode_indeterminate_reason: None,
            static_ipv4_dns_reg_servers: Vec::new(),
        }
    }

    #[test]
    fn backup_serialization_and_integrity_roundtrip() {
        let dir = std::env::temp_dir().join(format!("dns-changer-bk-test-{}", Uuid::new_v4()));
        let store = BackupStore::new(&dir);
        let guid = "{11111111-2222-3333-4444-555555555555}";

        let adapter = sample_adapter(guid, 12, DnsMode::Static, vec!["9.9.9.9", "149.112.112.112"]);
        let (created, is_new) = store.ensure_original_backup(&adapter).unwrap();
        assert!(is_new);
        assert!(created.verify_checksum());

        let loaded = store.load_backup(guid).unwrap();
        assert_eq!(loaded, created);
        assert_eq!(loaded.original_dns_mode, DnsMode::Static);
        assert_eq!(
            loaded.original_ipv4_dns_servers,
            vec!["9.9.9.9", "149.112.112.112"]
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn never_overwrites_existing_valid_original_backup() {
        let dir = std::env::temp_dir().join(format!("dns-changer-bk-noow-{}", Uuid::new_v4()));
        let store = BackupStore::new(&dir);
        let guid = "{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}";

        // Original was Automatic
        let original = sample_adapter(guid, 5, DnsMode::Automatic, vec!["192.168.1.1"]);
        let (first, created_first) = store.ensure_original_backup(&original).unwrap();
        assert!(created_first);
        assert_eq!(first.original_dns_mode, DnsMode::Automatic);

        // Subsequent apply sees adapter modified to Cloudflare; MUST NOT overwrite original backup
        let modified = sample_adapter(guid, 5, DnsMode::Static, vec!["1.1.1.1", "1.0.0.1"]);
        let (second, created_second) = store.ensure_original_backup(&modified).unwrap();
        assert!(!created_second);
        assert_eq!(second.original_dns_mode, DnsMode::Automatic);
        assert!(second.original_ipv4_dns_servers.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn isolates_backups_across_multiple_adapters_and_detects_corruption() {
        let dir = std::env::temp_dir().join(format!("dns-changer-bk-iso-{}", Uuid::new_v4()));
        let store = BackupStore::new(&dir);
        let guid_a = "{11111111-1111-1111-1111-111111111111}";
        let guid_b = "{22222222-2222-2222-2222-222222222222}";

        let adapter_a = sample_adapter(guid_a, 10, DnsMode::Automatic, vec![]);
        let adapter_b = sample_adapter(guid_b, 10, DnsMode::Static, vec!["8.8.8.8", "8.8.4.4"]);

        store.ensure_original_backup(&adapter_a).unwrap();
        store.ensure_original_backup(&adapter_b).unwrap();

        assert_eq!(
            store.load_backup(guid_a).unwrap().original_dns_mode,
            DnsMode::Automatic
        );
        assert_eq!(
            store.load_backup(guid_b).unwrap().original_ipv4_dns_servers,
            vec!["8.8.8.8", "8.8.4.4"]
        );

        // Corrupt adapter A's backup file on disk (camelCase JSON)
        let path_a = store.backup_file_path(guid_a);
        let original = fs::read_to_string(&path_a).unwrap();
        let tampered = original.replace("\"automatic\"", "\"static\"");
        assert_ne!(tampered, original);
        fs::write(&path_a, tampered).unwrap();

        let err = store.load_backup(guid_a).unwrap_err();
        assert!(matches!(err, AppError::BackupCorruption { .. }));
        // Adapter B remains unaffected
        assert!(store.load_backup(guid_b).is_ok());

        // Discarding corrupt backup moves it to <name>.corrupt-<timestamp> and refuses valid backups
        let quarantined = store.discard_corrupt_backup(guid_a).unwrap();
        assert!(quarantined.exists());
        assert!(quarantined
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains(".corrupt-"));
        assert!(!store.get_status(guid_a).unwrap().has_backup);

        assert!(matches!(
            store.discard_corrupt_backup(guid_b),
            Err(AppError::InvalidInput { .. })
        ));
        assert!(matches!(
            store.discard_corrupt_backup(guid_a),
            Err(AppError::BackupNotFound { .. })
        ));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn static_backup_falls_back_to_registry_servers_and_never_writes_invalid_record() {
        let dir = std::env::temp_dir().join(format!("dns-changer-bk-regfb-{}", Uuid::new_v4()));
        let store = BackupStore::new(&dir);
        let guid_ok = "{33333333-3333-3333-3333-333333333333}";
        let guid_empty = "{44444444-4444-4444-4444-444444444444}";

        let mut adapter_with_fallback = sample_adapter(guid_ok, 1, DnsMode::Static, vec![]);
        adapter_with_fallback.static_ipv4_dns_reg_servers =
            vec!["1.1.1.1".into(), "1.0.0.1".into(), "8.8.8.8".into()];
        let (rec, created) = store.ensure_original_backup(&adapter_with_fallback).unwrap();
        assert!(created);
        assert_eq!(
            rec.original_ipv4_dns_servers,
            vec!["1.1.1.1", "1.0.0.1", "8.8.8.8"]
        );
        assert!(store.load_backup(guid_ok).is_ok());

        let adapter_no_servers = sample_adapter(guid_empty, 2, DnsMode::Static, vec![]);
        let err = store.ensure_original_backup(&adapter_no_servers).unwrap_err();
        assert!(matches!(err, AppError::DnsModeIndeterminate { .. }));
        assert!(!store.get_status(guid_empty).unwrap().has_backup);

        let _ = fs::remove_dir_all(&dir);
    }
}
