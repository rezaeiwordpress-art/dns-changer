use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::warn;
use uuid::Uuid;

use crate::dns::providers::built_in_providers;
use crate::dns::validation::validate_dns_pair;
use crate::errors::AppError;
use crate::models::DnsProvider;

pub const CONFIG_SCHEMA_VERSION: u32 = 1;

fn default_true() -> bool {
    true
}

fn default_language() -> String {
    "fa".to_string()
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_start_page() -> String {
    "dashboard".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub schema_version: u32,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    pub selected_adapter_guid: Option<String>,
    pub selected_provider_id: Option<String>,
    #[serde(default = "default_true")]
    pub remember_selected_adapter: bool,
    #[serde(default = "default_true")]
    pub remember_selected_provider: bool,
    #[serde(default = "default_start_page")]
    pub start_page: String,
    #[serde(default = "default_true")]
    pub confirm_before_restore: bool,
    #[serde(default = "default_true")]
    pub show_notifications: bool,
    #[serde(default)]
    pub custom_providers: Vec<DnsProvider>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            language: default_language(),
            theme: default_theme(),
            selected_adapter_guid: None,
            selected_provider_id: Some("cloudflare".to_string()),
            remember_selected_adapter: true,
            remember_selected_provider: true,
            start_page: default_start_page(),
            confirm_before_restore: true,
            show_notifications: true,
            custom_providers: Vec::new(),
        }
    }
}

fn path_is_symlink_or_reparse(path: &Path) -> Result<bool, AppError> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                return Ok(true);
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => {
            return Err(AppError::StorageError {
                detail: format!("Failed to inspect metadata for {}: {e}", path.display()),
            });
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            GetFileAttributesW, FILE_ATTRIBUTE_REPARSE_POINT, INVALID_FILE_ATTRIBUTES,
        };
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let attrs = unsafe { GetFileAttributesW(wide.as_ptr()) };
        if attrs == INVALID_FILE_ATTRIBUTES {
            return Err(AppError::StorageError {
                detail: format!("Failed to inspect Windows attributes for {}", path.display()),
            });
        }
        if (attrs & FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Atomically writes bytes to `target_path` by writing to a unique sibling temp file,
/// flushing to disk with `sync_all()`, and renaming into place.
/// Rejects symlinks and Windows reparse points on both `parent` and `target_path`.
pub fn write_bytes_atomic(target_path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = target_path.parent().ok_or_else(|| AppError::StorageError {
        detail: format!("Target path has no parent directory: {}", target_path.display()),
    })?;

    if path_is_symlink_or_reparse(parent)? {
        return Err(AppError::StorageError {
            detail: format!(
                "Refusing atomic write into symlink or reparse directory {}",
                parent.display()
            ),
        });
    }

    fs::create_dir_all(parent).map_err(|e| AppError::StorageError {
        detail: format!("Failed to create directory {}: {e}", parent.display()),
    })?;

    if path_is_symlink_or_reparse(parent)? || path_is_symlink_or_reparse(target_path)? {
        return Err(AppError::StorageError {
            detail: format!(
                "Refusing atomic write to symlink or reparse target {}",
                target_path.display()
            ),
        });
    }

    let file_name = target_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("data");
    let tmp_name = format!(".{file_name}.tmp.{}", Uuid::new_v4());
    let tmp_path = parent.join(tmp_name);

    {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)
            .map_err(|e| AppError::StorageError {
                detail: format!("Failed to create temporary file {}: {e}", tmp_path.display()),
            })?;
        file.write_all(bytes).map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            AppError::StorageError {
                detail: format!("Failed to write temporary file {}: {e}", tmp_path.display()),
            }
        })?;
        file.sync_all().map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            AppError::StorageError {
                detail: format!("Failed to sync temporary file {}: {e}", tmp_path.display()),
            }
        })?;
    }

    fs::rename(&tmp_path, target_path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        AppError::StorageError {
            detail: format!(
                "Failed to atomically move {} to {}: {e}",
                tmp_path.display(),
                target_path.display()
            ),
        }
    })?;

    Ok(())
}

pub struct AppConfigStore {
    config_path: PathBuf,
    io_lock: Mutex<()>,
}

impl AppConfigStore {
    pub fn new(app_data_dir: &Path) -> Self {
        Self {
            config_path: app_data_dir.join("config.json"),
            io_lock: Mutex::new(()),
        }
    }

    fn load_unlocked(&self) -> Result<AppConfig, AppError> {
        if !self.config_path.exists() {
            let default_cfg = AppConfig::default();
            self.save_unlocked(&default_cfg)?;
            return Ok(default_cfg);
        }

        let raw = fs::read_to_string(&self.config_path).map_err(|e| AppError::StorageError {
            detail: format!("Failed to read {}: {e}", self.config_path.display()),
        })?;

        match serde_json::from_str::<AppConfig>(&raw) {
            Ok(cfg) => Ok(cfg),
            Err(parse_err) => {
                let stamp = Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
                let backup_name = format!("config.json.bak-{stamp}");
                let backup_path = self.config_path.with_file_name(backup_name);
                warn!(
                    path = %self.config_path.display(),
                    backup_path = %backup_path.display(),
                    error = %parse_err,
                    "Failed to parse config.json; backing up corrupt file and recreating defaults"
                );
                fs::rename(&self.config_path, &backup_path).map_err(|rename_err| {
                    AppError::StorageError {
                        detail: format!(
                            "Failed to quarantine corrupt config {} to {}: {rename_err}",
                            self.config_path.display(),
                            backup_path.display()
                        ),
                    }
                })?;

                let default_cfg = AppConfig::default();
                self.save_unlocked(&default_cfg)?;
                Ok(default_cfg)
            }
        }
    }

    fn save_unlocked(&self, config: &AppConfig) -> Result<(), AppError> {
        let json = serde_json::to_vec_pretty(config).map_err(|e| AppError::StorageError {
            detail: format!("Failed to serialize configuration: {e}"),
        })?;
        write_bytes_atomic(&self.config_path, &json)
    }

    pub fn load(&self) -> Result<AppConfig, AppError> {
        let _guard = self.io_lock.lock().map_err(|_| AppError::StorageError {
            detail: "Configuration store lock poisoned".into(),
        })?;
        self.load_unlocked()
    }

    pub fn save(&self, config: &AppConfig) -> Result<(), AppError> {
        let _guard = self.io_lock.lock().map_err(|_| AppError::StorageError {
            detail: "Configuration store lock poisoned".into(),
        })?;
        self.save_unlocked(config)
    }

    pub fn update<F>(&self, updater: F) -> Result<AppConfig, AppError>
    where
        F: FnOnce(&mut AppConfig) -> Result<(), AppError>,
    {
        let _guard = self.io_lock.lock().map_err(|_| AppError::StorageError {
            detail: "Configuration store lock poisoned".into(),
        })?;
        let mut cfg = self.load_unlocked()?;
        updater(&mut cfg)?;
        self.save_unlocked(&cfg)?;
        Ok(cfg)
    }

    fn ensure_unique_provider_name(
        cfg: &AppConfig,
        candidate_name: &str,
        exclude_id: Option<&str>,
    ) -> Result<(), AppError> {
        let trimmed = candidate_name.trim();
        let normalized = trimmed.to_ascii_lowercase();
        let conflict_built_in = built_in_providers()
            .into_iter()
            .any(|p| p.name.to_ascii_lowercase() == normalized);
        if conflict_built_in {
            return Err(AppError::DuplicateProviderName {
                name: trimmed.to_string(),
            });
        }

        let conflict_custom = cfg.custom_providers.iter().any(|p| {
            exclude_id != Some(p.id.as_str()) && p.name.trim().to_ascii_lowercase() == normalized
        });
        if conflict_custom {
            return Err(AppError::DuplicateProviderName {
                name: trimmed.to_string(),
            });
        }
        Ok(())
    }

    pub fn add_custom_provider(
        &self,
        name: String,
        primary_dns: String,
        secondary_dns: Option<String>,
        description: Option<String>,
    ) -> Result<DnsProvider, AppError> {
        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            return Err(AppError::InvalidInput {
                detail: "Custom provider name cannot be empty".into(),
            });
        }

        let secondary_opt = secondary_dns
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let validated = validate_dns_pair(primary_dns.trim(), secondary_opt)?;
        let primary = validated[0].to_string();
        let secondary = validated.get(1).map(|ip| ip.to_string());

        let clean_desc = description
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());

        let provider = DnsProvider {
            id: format!("custom-{}", Uuid::new_v4()),
            name: trimmed_name.to_string(),
            primary_dns: primary,
            secondary_dns: secondary,
            is_built_in: false,
            category: Some("Custom".to_string()),
            description: clean_desc,
        };

        self.update(|cfg| {
            Self::ensure_unique_provider_name(cfg, trimmed_name, None)?;
            cfg.custom_providers.push(provider.clone());
            Ok(())
        })?;

        Ok(provider)
    }

    pub fn update_custom_provider(
        &self,
        provider_id: String,
        name: String,
        primary_dns: String,
        secondary_dns: Option<String>,
        description: Option<String>,
    ) -> Result<DnsProvider, AppError> {
        if built_in_providers().iter().any(|p| p.id == provider_id) {
            return Err(AppError::InvalidInput {
                detail: "Built-in providers cannot be modified".into(),
            });
        }

        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            return Err(AppError::InvalidInput {
                detail: "Custom provider name cannot be empty".into(),
            });
        }

        let secondary_opt = secondary_dns
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let validated = validate_dns_pair(primary_dns.trim(), secondary_opt)?;
        let primary = validated[0].to_string();
        let secondary = validated.get(1).map(|ip| ip.to_string());

        let clean_desc = description
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());

        let mut updated_provider: Option<DnsProvider> = None;
        self.update(|cfg| {
            Self::ensure_unique_provider_name(cfg, trimmed_name, Some(&provider_id))?;
            let target = cfg
                .custom_providers
                .iter_mut()
                .find(|p| p.id == provider_id)
                .ok_or_else(|| AppError::InvalidInput {
                    detail: format!("Custom provider '{provider_id}' not found"),
                })?;

            target.name = trimmed_name.to_string();
            target.primary_dns = primary.clone();
            target.secondary_dns = secondary.clone();
            target.description = clean_desc.clone();
            updated_provider = Some(target.clone());
            Ok(())
        })?;

        updated_provider.ok_or_else(|| AppError::InternalError {
            detail: "Updated provider missing after save".into(),
        })
    }

    pub fn delete_custom_provider(&self, provider_id: &str) -> Result<Vec<DnsProvider>, AppError> {
        if built_in_providers().iter().any(|p| p.id == provider_id) {
            return Err(AppError::InvalidInput {
                detail: "Built-in providers cannot be deleted".into(),
            });
        }

        let cfg = self.update(|cfg| {
            let original_len = cfg.custom_providers.len();
            cfg.custom_providers.retain(|p| p.id != provider_id);
            if cfg.custom_providers.len() == original_len {
                return Err(AppError::InvalidInput {
                    detail: format!("Custom provider '{provider_id}' not found"),
                });
            }
            if cfg.selected_provider_id.as_deref() == Some(provider_id) {
                cfg.selected_provider_id = Some("cloudflare".to_string());
            }
            Ok(())
        })?;

        Ok(cfg.custom_providers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn saves_and_reloads_app_config_atomically() {
        let dir = std::env::temp_dir().join(format!("dns-changer-cfg-test-{}", Uuid::new_v4()));
        let store = AppConfigStore::new(&dir);

        let initial = store.load().unwrap();
        assert_eq!(initial.language, "fa");
        assert_eq!(initial.theme, "system");
        assert!(initial.remember_selected_adapter);
        assert!(initial.confirm_before_restore);

        let custom = store
            .add_custom_provider(
                "Office DNS".into(),
                "10.0.0.53".into(),
                Some("10.0.0.54".into()),
                Some("Internal office resolver".into()),
            )
            .unwrap();
        assert!(!custom.is_built_in);
        assert_eq!(custom.secondary_dns.as_deref(), Some("10.0.0.54"));
        assert_eq!(custom.description.as_deref(), Some("Internal office resolver"));

        // Optional secondary DNS (None or empty) is allowed
        let single_dns = store
            .add_custom_provider(
                "Single Resolver".into(),
                "10.0.0.70".into(),
                None,
                None,
            )
            .unwrap();
        assert_eq!(single_dns.secondary_dns, None);

        // Identical primary and secondary rejected
        assert!(matches!(
            store.add_custom_provider(
                "Same Pair".into(),
                "10.0.0.80".into(),
                Some("10.0.0.80".into()),
                None
            ),
            Err(AppError::InvalidDnsAddress { .. })
        ));

        // Duplicate name rejected (case-insensitive) with DuplicateProviderName
        assert!(matches!(
            store.add_custom_provider(
                "office dns".into(),
                "10.0.0.55".into(),
                Some("10.0.0.56".into()),
                None
            ),
            Err(AppError::DuplicateProviderName { .. })
        ));

        // Built-in name rejected with DuplicateProviderName
        assert!(matches!(
            store.add_custom_provider(
                "Cloudflare".into(),
                "1.1.1.1".into(),
                Some("1.0.0.1".into()),
                None
            ),
            Err(AppError::DuplicateProviderName { .. })
        ));

        // Update works (including clearing secondary DNS)
        let updated = store
            .update_custom_provider(
                custom.id.clone(),
                "Office DNS v2".into(),
                "10.0.0.60".into(),
                Some("   ".into()),
                None,
            )
            .unwrap();
        assert_eq!(updated.name, "Office DNS v2");
        assert_eq!(updated.primary_dns, "10.0.0.60");
        assert_eq!(updated.secondary_dns, None);

        // Built-in delete/edit protected
        assert!(matches!(
            store.delete_custom_provider("cloudflare"),
            Err(AppError::InvalidInput { .. })
        ));

        let reloaded = store.load().unwrap();
        assert_eq!(reloaded.custom_providers.len(), 2);
        assert_eq!(reloaded.custom_providers[0].primary_dns, "10.0.0.60");

        store.delete_custom_provider(&custom.id).unwrap();
        store.delete_custom_provider(&single_dns.id).unwrap();
        assert!(store.load().unwrap().custom_providers.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn backs_up_corrupt_config_and_recreates_defaults() {
        let dir = std::env::temp_dir().join(format!("dns-changer-cfg-corrupt-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("config.json");
        fs::write(&config_path, b"{ not valid json").unwrap();

        let store = AppConfigStore::new(&dir);
        let recovered = store.load().unwrap();
        assert_eq!(recovered.language, "fa");
        assert!(recovered.custom_providers.is_empty());

        let entries: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(entries.iter().any(|name| name.starts_with("config.json.bak-")));
        assert!(entries.iter().any(|name| name == "config.json"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn serializes_concurrent_updates_via_mutex() {
        let dir = std::env::temp_dir().join(format!("dns-changer-cfg-conc-{}", Uuid::new_v4()));
        let store = Arc::new(AppConfigStore::new(&dir));
        let _ = store.load().unwrap();

        let mut handles = Vec::new();
        for i in 0..8 {
            let store_clone = Arc::clone(&store);
            handles.push(std::thread::spawn(move || {
                store_clone
                    .add_custom_provider(
                        format!("Concurrent-{i}"),
                        format!("10.0.1.{}", i + 10),
                        None,
                        None,
                    )
                    .unwrap();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let final_cfg = store.load().unwrap();
        assert_eq!(final_cfg.custom_providers.len(), 8);

        let _ = fs::remove_dir_all(&dir);
    }
}
