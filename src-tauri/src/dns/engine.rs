use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::admin::ElevationProvider;
use crate::backup::{BackupRecord, BackupStatusSummary, BackupStore};
use crate::dns::validation::{validate_adapter_guid, validate_dns_list, validate_dns_pair};
use crate::errors::AppError;
use crate::models::{
    AdapterStatus, ApplyDnsResult, DnsMode, Ipv6DnsSnapshot, NetworkAdapter, RestoreDnsResult,
};
use crate::network::NetworkBackend;

pub struct DnsEngine {
    backend: Arc<dyn NetworkBackend>,
    backup_store: Arc<BackupStore>,
    elevation: Arc<dyn ElevationProvider>,
    adapter_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl DnsEngine {
    pub fn new(
        backend: Arc<dyn NetworkBackend>,
        backup_store: Arc<BackupStore>,
        elevation: Arc<dyn ElevationProvider>,
    ) -> Self {
        Self {
            backend,
            backup_store,
            elevation,
            adapter_locks: Mutex::new(HashMap::new()),
        }
    }

    async fn get_adapter_lock(&self, canonical_guid: &str) -> Arc<Mutex<()>> {
        let mut map = self.adapter_locks.lock().await;
        map.entry(canonical_guid.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    pub async fn list_adapters(&self) -> Result<Vec<NetworkAdapter>, AppError> {
        let backend = self.backend.clone();
        tokio::task::spawn_blocking(move || backend.list_adapters())
            .await
            .map_err(|e| AppError::InternalError {
                detail: format!("Adapter listing task panicked: {e}"),
            })?
    }

    pub async fn get_adapter_details(&self, adapter_guid: &str) -> Result<NetworkAdapter, AppError> {
        let canonical = validate_adapter_guid(adapter_guid)?;
        let backend = self.backend.clone();
        let guid_clone = canonical.clone();
        let found = tokio::task::spawn_blocking(move || backend.get_adapter_by_guid(&guid_clone))
            .await
            .map_err(|e| AppError::InternalError {
                detail: format!("Adapter query task panicked: {e}"),
            })??;

        found.ok_or(AppError::AdapterNotFound { guid: canonical })
    }

    pub fn get_backup_status(&self, adapter_guid: &str) -> Result<BackupStatusSummary, AppError> {
        self.backup_store.get_status(adapter_guid)
    }

    pub fn list_all_backups(&self) -> Result<Vec<BackupStatusSummary>, AppError> {
        self.backup_store.list_backups()
    }

    pub fn discard_corrupt_backup(&self, adapter_guid: &str) -> Result<(), AppError> {
        self.backup_store
            .discard_corrupt_backup(adapter_guid)
            .map(|_| ())
    }

    pub fn is_elevated(&self) -> bool {
        self.elevation.is_elevated()
    }

    pub fn can_request_elevation(&self) -> bool {
        self.elevation.can_request_elevation()
    }

    /// Complete DNS application workflow with strict IPv4-only mutation and IPv6 preservation verification:
    /// 1. Validate IPv4 addresses & adapter GUID.
    /// 2. Acquire per-adapter exclusivity lock.
    /// 3. Verify adapter existence & confident original DNS mode detection.
    /// 4. Verify adapter is enabled and operational.
    /// 5. Check administrator privileges / elevation capability.
    /// 6. Capture read-only IPv6 snapshot BEFORE any IPv4 DNS changes.
    /// 7. Save original IPv4 DNS backup BEFORE applying any DNS changes.
    /// 8. Apply requested IPv4 DNS servers (in-process if elevated; single atomic apply+verify+rollback UAC invocation otherwise).
    /// 9. Re-read IPv4 & IPv6 configuration from Windows and verify BOTH IPv4 target match AND IPv6 preservation.
    /// 10. Return structured verified result (or recover/rollback safely while preserving backup on failure).
    pub async fn apply_dns(
        &self,
        adapter_guid: &str,
        primary_dns: &str,
        secondary_dns: Option<&str>,
    ) -> Result<ApplyDnsResult, AppError> {
        let canonical_guid = validate_adapter_guid(adapter_guid)?;
        let validated_ips = validate_dns_pair(primary_dns, secondary_dns)?;
        let expected_servers: Vec<String> = validated_ips.iter().map(|ip| ip.to_string()).collect();

        let lock = self.get_adapter_lock(&canonical_guid).await;
        let _guard = lock
            .try_lock()
            .map_err(|_| AppError::ConcurrentOperation {
                guid: canonical_guid.clone(),
            })?;

        info!(
            adapter_guid = %canonical_guid,
            servers = ?expected_servers,
            "Starting apply_dns workflow"
        );

        let adapter = self.fetch_adapter_blocking(&canonical_guid).await?;

        if let Some(reason) = &adapter.dns_mode_indeterminate_reason {
            return Err(AppError::DnsModeIndeterminate {
                guid: canonical_guid,
                detail: reason.clone(),
            });
        }

        match adapter.operational_status {
            AdapterStatus::Disabled => {
                return Err(AppError::AdapterDisabled {
                    guid: canonical_guid,
                    name: adapter.name,
                });
            }
            AdapterStatus::Disconnected | AdapterStatus::NotPresent => {
                return Err(AppError::AdapterDisconnected {
                    guid: canonical_guid,
                    name: adapter.name,
                });
            }
            AdapterStatus::Up | AdapterStatus::Unknown => {}
        }

        let already_elevated = self.elevation.is_elevated();
        if !already_elevated && !self.elevation.can_request_elevation() {
            return Err(AppError::AdminAccessDenied {
                detail: "Current process lacks Administrator privileges and elevation is unavailable"
                    .into(),
            });
        }

        // Capture read-only IPv6 DNS state BEFORE making any IPv4 DNS change.
        let before_ipv6 = self.fetch_ipv6_state_blocking(&canonical_guid).await?;

        // Save original backup BEFORE applying any DNS changes.
        let (backup_record, backup_created) = self.backup_store.ensure_original_backup(&adapter)?;

        // Execute IPv4-only DNS change.
        // When non-elevated, run apply + verify + rollback inside a single UAC helper invocation
        // so rollback never triggers a second UAC prompt.
        let apply_res = if already_elevated {
            self.execute_set_dns(&canonical_guid, &validated_ips, true)
                .await
        } else {
            self.execute_set_dns_with_rollback_uac(&canonical_guid, &validated_ips, &backup_record)
                .await
        };

        if let Err(err) = apply_res {
            if matches!(err, AppError::UacCancelled) {
                if backup_created {
                    let _ = self.backup_store.remove_backup(&canonical_guid);
                }
                return Err(err);
            }

            if matches!(
                err,
                AppError::RollbackNotPerformed { .. } | AppError::PowershellTimeout { .. }
            ) {
                match self.fetch_adapter_blocking(&canonical_guid).await {
                    Ok(observed) => {
                        warn!(
                            adapter_guid = %canonical_guid,
                            dns_mode = ?observed.dns_mode,
                            current_servers = ?observed.current_ipv4_dns_servers,
                            indeterminate_reason = ?observed.dns_mode_indeterminate_reason,
                            error = %err,
                            "Observed adapter state after elevated/set DNS timeout or failure; preserving backup"
                        );
                    }
                    Err(read_err) => {
                        warn!(
                            adapter_guid = %canonical_guid,
                            read_error = %read_err,
                            error = %err,
                            "Failed to re-read adapter state after set DNS timeout or failure; preserving backup"
                        );
                    }
                }
            }

            let _ = self.backup_store.mark_apply_failed_preserved(&canonical_guid);
            return Err(err);
        }

        // Re-read fresh IPv4 and IPv6 configuration from Windows and verify.
        let fresh = match retry_read(|| self.fetch_adapter_blocking(&canonical_guid)).await {
            Ok(a) => a,
            Err(AppError::AdapterNotFound { .. }) => {
                let _ = self.backup_store.mark_apply_failed_preserved(&canonical_guid);
                return Err(self
                    .handle_post_apply_failure(
                        &canonical_guid,
                        &backup_record,
                        already_elevated,
                        AppError::NetworkChanged {
                            detail: format!(
                                "Adapter {canonical_guid} disappeared after DNS change"
                            ),
                        },
                    )
                    .await);
            }
            Err(e) => {
                let _ = self.backup_store.mark_apply_failed_preserved(&canonical_guid);
                return Err(self
                    .handle_post_apply_failure(&canonical_guid, &backup_record, already_elevated, e)
                    .await);
            }
        };

        let after_ipv6 = match retry_read(|| self.fetch_ipv6_state_blocking(&canonical_guid)).await
        {
            Ok(s) => s,
            Err(e) => {
                let _ = self.backup_store.mark_apply_failed_preserved(&canonical_guid);
                return Err(self
                    .handle_post_apply_failure(&canonical_guid, &backup_record, already_elevated, e)
                    .await);
            }
        };

        if let Err(ipv6_err) = self.verify_ipv6_preserved(&before_ipv6, &after_ipv6) {
            error!(
                adapter_guid = %canonical_guid,
                error = %ipv6_err,
                "Post-apply IPv6 preservation check failed; attempting recovery and preserving backup"
            );
            let _ = self.backup_store.mark_apply_failed_preserved(&canonical_guid);
            if already_elevated {
                if let Some(orig_v6) = &before_ipv6 {
                    self.attempt_ipv6_recovery(&canonical_guid, orig_v6, true)
                        .await;
                }
            }
            return Err(self
                .handle_post_apply_failure(
                    &canonical_guid,
                    &backup_record,
                    already_elevated,
                    ipv6_err,
                )
                .await);
        }

        if let Some(reason) = &fresh.dns_mode_indeterminate_reason {
            error!(
                adapter_guid = %canonical_guid,
                reason = %reason,
                "Post-apply adapter DNS mode became indeterminate; rolling back safely"
            );
            let _ = self.backup_store.mark_apply_failed_preserved(&canonical_guid);
            return Err(self
                .handle_post_apply_failure(
                    &canonical_guid,
                    &backup_record,
                    already_elevated,
                    AppError::DnsModeIndeterminate {
                        guid: canonical_guid.clone(),
                        detail: reason.clone(),
                    },
                )
                .await);
        }

        let verified = fresh.dns_mode == DnsMode::Static
            && fresh.current_ipv4_dns_servers == expected_servers;

        if !verified {
            error!(
                adapter_guid = %canonical_guid,
                expected = ?expected_servers,
                actual = ?fresh.current_ipv4_dns_servers,
                actual_mode = ?fresh.dns_mode,
                "Post-apply DNS verification failed; attempting safe rollback"
            );
            let _ = self.backup_store.mark_apply_failed_preserved(&canonical_guid);
            return Err(self
                .handle_post_apply_failure(
                    &canonical_guid,
                    &backup_record,
                    already_elevated,
                    AppError::VerificationFailed {
                        detail: format!(
                            "Expected static DNS {:?}, but Windows reported mode {:?} with servers {:?}",
                            expected_servers, fresh.dns_mode, fresh.current_ipv4_dns_servers
                        ),
                    },
                )
                .await);
        }

        info!(
            adapter_guid = %canonical_guid,
            servers = ?expected_servers,
            "Successfully applied and verified IPv4 DNS configuration with IPv6 preserved"
        );

        Ok(ApplyDnsResult {
            adapter_guid: canonical_guid,
            applied_servers: expected_servers,
            verified: true,
            backup_created,
            adapter: fresh,
        })
    }

    async fn handle_post_apply_failure(
        &self,
        canonical_guid: &str,
        backup: &BackupRecord,
        already_elevated: bool,
        primary_err: AppError,
    ) -> AppError {
        if !already_elevated {
            warn!(
                adapter_guid = %canonical_guid,
                error = %primary_err,
                "Post-apply check failed after elevated helper returned; skipping second UAC prompt"
            );
            return AppError::RollbackNotPerformed {
                detail: format!("DNS changed, rollback not performed: {primary_err}"),
            };
        }

        match self
            .attempt_safe_rollback(canonical_guid, backup, true)
            .await
        {
            Ok(()) => primary_err,
            Err(rb_err) => AppError::RollbackNotPerformed {
                detail: format!(
                    "DNS changed, rollback not performed ({rb_err}) after failure: {primary_err}"
                ),
            },
        }
    }

    /// Restores an adapter's original IPv4 DNS configuration from persistent backup while
    /// proving IPv6 DNS remained untouched. Backup is preserved if IPv4 restoration or IPv6
    /// preservation verification fails, and removed only after all checks pass.
    pub async fn restore_dns(&self, adapter_guid: &str) -> Result<RestoreDnsResult, AppError> {
        let canonical_guid = validate_adapter_guid(adapter_guid)?;

        let lock = self.get_adapter_lock(&canonical_guid).await;
        let _guard = lock
            .try_lock()
            .map_err(|_| AppError::ConcurrentOperation {
                guid: canonical_guid.clone(),
            })?;

        info!(adapter_guid = %canonical_guid, "Starting restore_dns workflow");

        // Load and validate integrity of backup before touching the adapter.
        let backup = self.backup_store.load_backup(&canonical_guid)?;

        let adapter = self.fetch_adapter_blocking(&canonical_guid).await?;
        if matches!(
            adapter.operational_status,
            AdapterStatus::Disabled | AdapterStatus::NotPresent
        ) {
            return Err(AppError::AdapterDisabled {
                guid: canonical_guid,
                name: adapter.name,
            });
        }

        let already_elevated = self.elevation.is_elevated();
        if !already_elevated && !self.elevation.can_request_elevation() {
            return Err(AppError::AdminAccessDenied {
                detail: "Current process lacks Administrator privileges and elevation is unavailable"
                    .into(),
            });
        }

        // Capture read-only IPv6 DNS state BEFORE resetting/restoring IPv4 DNS.
        let before_ipv6 = self.fetch_ipv6_state_blocking(&canonical_guid).await?;

        let restore_op = match backup.original_dns_mode {
            DnsMode::Automatic => self.execute_reset_dns(&canonical_guid, already_elevated).await,
            DnsMode::Static => {
                let ips = validate_dns_list(&backup.original_ipv4_dns_servers)?;
                self.execute_set_dns(&canonical_guid, &ips, already_elevated)
                    .await
            }
        };

        if let Err(err) = restore_op {
            warn!(
                adapter_guid = %canonical_guid,
                error = %err,
                "DNS restoration operation failed; preserving backup"
            );
            if matches!(
                err,
                AppError::UacCancelled | AppError::AdminAccessDenied { .. }
            ) {
                return Err(err);
            }
            return Err(AppError::RestoreFailed {
                detail: format!("Failed to restore original IPv4 DNS: {err}"),
            });
        }

        // Verify IPv6 preservation AND IPv4 restoration against fresh Windows adapter state.
        let after_ipv6 = self
            .fetch_ipv6_state_blocking(&canonical_guid)
            .await
            .map_err(|e| AppError::RestoreFailed {
                detail: format!("Failed to verify IPv6 state after DNS restore: {e}"),
            })?;

        if let Err(ipv6_err) = self.verify_ipv6_preserved(&before_ipv6, &after_ipv6) {
            error!(
                adapter_guid = %canonical_guid,
                error = %ipv6_err,
                "Post-restore IPv6 preservation check failed; attempting IPv6 recovery and preserving backup"
            );
            if let Some(orig_v6) = &before_ipv6 {
                self.attempt_ipv6_recovery(&canonical_guid, orig_v6, already_elevated)
                    .await;
            }
            return Err(ipv6_err);
        }

        let fresh = self
            .fetch_adapter_blocking(&canonical_guid)
            .await
            .map_err(|e| AppError::RestoreFailed {
                detail: format!("Failed to verify adapter state after DNS restore: {e}"),
            })?;

        let verified = fresh.dns_mode_indeterminate_reason.is_none()
            && match backup.original_dns_mode {
                DnsMode::Automatic => fresh.dns_mode == DnsMode::Automatic,
                DnsMode::Static => {
                    fresh.dns_mode == DnsMode::Static
                        && fresh.current_ipv4_dns_servers == backup.original_ipv4_dns_servers
                }
            };

        if !verified {
            error!(
                adapter_guid = %canonical_guid,
                expected_mode = ?backup.original_dns_mode,
                expected_servers = ?backup.original_ipv4_dns_servers,
                actual_mode = ?fresh.dns_mode,
                actual_servers = ?fresh.current_ipv4_dns_servers,
                indeterminate_reason = ?fresh.dns_mode_indeterminate_reason,
                "Post-restore verification failed; preserving backup"
            );
            return Err(AppError::RestoreFailed {
                detail: format!(
                    "Expected restored mode {:?} ({:?}), but Windows reported {:?} ({:?})",
                    backup.original_dns_mode,
                    backup.original_ipv4_dns_servers,
                    fresh.dns_mode,
                    fresh.current_ipv4_dns_servers
                ),
            });
        }

        // Only delete backup after verified IPv4 restoration AND IPv6 preservation.
        self.backup_store.remove_backup(&canonical_guid)?;

        info!(
            adapter_guid = %canonical_guid,
            restored_mode = ?backup.original_dns_mode,
            "Successfully restored and verified original IPv4 DNS configuration with IPv6 preserved"
        );

        Ok(RestoreDnsResult {
            adapter_guid: canonical_guid,
            restored_mode: backup.original_dns_mode,
            restored_servers: fresh.current_ipv4_dns_servers.clone(),
            verified: true,
            backup_cleared: true,
            adapter: fresh,
        })
    }

    fn verify_ipv6_preserved(
        &self,
        before: &Option<Ipv6DnsSnapshot>,
        after: &Option<Ipv6DnsSnapshot>,
    ) -> Result<(), AppError> {
        match (before, after) {
            (Some(b), Some(a)) => {
                if !b.matches(a) {
                    return Err(AppError::Ipv6ModificationDetected {
                        detail: format!(
                            "IPv6 DNS changed unexpectedly from mode={:?} servers={:?} to mode={:?} servers={:?}",
                            b.dns_mode, b.server_addresses, a.dns_mode, a.server_addresses
                        ),
                    });
                }
                Ok(())
            }
            (None, None) => Ok(()),
            (b, a) => Err(AppError::Ipv6ModificationDetected {
                detail: format!("IPv6 adapter presence changed unexpectedly: before={b:?}, after={a:?}"),
            }),
        }
    }

    async fn fetch_adapter_blocking(&self, canonical_guid: &str) -> Result<NetworkAdapter, AppError> {
        let backend = self.backend.clone();
        let guid = canonical_guid.to_string();
        let found = tokio::task::spawn_blocking(move || backend.get_adapter_by_guid(&guid))
            .await
            .map_err(|e| AppError::InternalError {
                detail: format!("Adapter fetch task panicked: {e}"),
            })??;

        found.ok_or_else(|| AppError::AdapterNotFound {
            guid: canonical_guid.to_string(),
        })
    }

    async fn fetch_ipv6_state_blocking(
        &self,
        canonical_guid: &str,
    ) -> Result<Option<Ipv6DnsSnapshot>, AppError> {
        let backend = self.backend.clone();
        let guid = canonical_guid.to_string();
        tokio::task::spawn_blocking(move || backend.get_ipv6_dns_state(&guid))
            .await
            .map_err(|e| AppError::InternalError {
                detail: format!("IPv6 state query task panicked: {e}"),
            })?
    }

    async fn execute_set_dns(
        &self,
        canonical_guid: &str,
        ips: &[Ipv4Addr],
        already_elevated: bool,
    ) -> Result<(), AppError> {
        let guid = canonical_guid.to_string();
        let servers = ips.to_vec();
        if already_elevated {
            let backend = self.backend.clone();
            tokio::task::spawn_blocking(move || backend.set_ipv4_dns(&guid, &servers))
                .await
                .map_err(|e| AppError::InternalError {
                    detail: format!("DNS apply task panicked: {e}"),
                })?
        } else {
            let elevation = self.elevation.clone();
            tokio::task::spawn_blocking(move || elevation.run_elevated_set_dns(&guid, &servers))
                .await
                .map_err(|e| AppError::InternalError {
                    detail: format!("Elevated DNS apply task panicked: {e}"),
                })?
        }
    }

    async fn execute_set_dns_with_rollback_uac(
        &self,
        canonical_guid: &str,
        ips: &[Ipv4Addr],
        backup: &BackupRecord,
    ) -> Result<(), AppError> {
        let guid = canonical_guid.to_string();
        let servers = ips.to_vec();
        let rollback_mode = backup.original_dns_mode;
        let rollback_servers = match rollback_mode {
            DnsMode::Automatic => Vec::new(),
            DnsMode::Static => validate_dns_list(&backup.original_ipv4_dns_servers)?,
        };
        let elevation = self.elevation.clone();
        tokio::task::spawn_blocking(move || {
            elevation.run_elevated_set_dns_with_rollback(
                &guid,
                &servers,
                rollback_mode,
                &rollback_servers,
            )
        })
        .await
        .map_err(|e| AppError::InternalError {
            detail: format!("Elevated DNS apply task panicked: {e}"),
        })?
    }

    async fn execute_reset_dns(
        &self,
        canonical_guid: &str,
        already_elevated: bool,
    ) -> Result<(), AppError> {
        let guid = canonical_guid.to_string();
        if already_elevated {
            let backend = self.backend.clone();
            tokio::task::spawn_blocking(move || backend.reset_ipv4_dns_to_automatic(&guid))
                .await
                .map_err(|e| AppError::InternalError {
                    detail: format!("DNS reset task panicked: {e}"),
                })?
        } else {
            let elevation = self.elevation.clone();
            tokio::task::spawn_blocking(move || elevation.run_elevated_reset_dns(&guid))
                .await
                .map_err(|e| AppError::InternalError {
                    detail: format!("Elevated DNS reset task panicked: {e}"),
                })?
        }
    }

    async fn attempt_ipv6_recovery(
        &self,
        canonical_guid: &str,
        original: &Ipv6DnsSnapshot,
        already_elevated: bool,
    ) {
        if !already_elevated {
            warn!(
                adapter_guid = %canonical_guid,
                "Cannot perform automatic IPv6 recovery without elevated process privileges"
            );
            return;
        }
        let backend = self.backend.clone();
        let guid = canonical_guid.to_string();
        let snap = original.clone();
        let res = tokio::task::spawn_blocking(move || backend.restore_ipv6_dns_state(&guid, &snap))
            .await;
        match res {
            Ok(Ok(())) => {
                info!(adapter_guid = %canonical_guid, "Recovered original IPv6 DNS state after unexpected discrepancy");
            }
            Ok(Err(e)) => {
                warn!(adapter_guid = %canonical_guid, error = %e, "Best-effort IPv6 recovery failed");
            }
            Err(e) => {
                warn!(adapter_guid = %canonical_guid, error = %e, "IPv6 recovery task panicked");
            }
        }
    }

    async fn attempt_safe_rollback(
        &self,
        canonical_guid: &str,
        backup: &BackupRecord,
        already_elevated: bool,
    ) -> Result<(), AppError> {
        if !already_elevated {
            return Err(AppError::RollbackNotPerformed {
                detail: "DNS changed, rollback not performed without elevation".into(),
            });
        }

        let res = match backup.original_dns_mode {
            DnsMode::Automatic => self.execute_reset_dns(canonical_guid, true).await,
            DnsMode::Static => {
                let ips = validate_dns_list(&backup.original_ipv4_dns_servers)?;
                self.execute_set_dns(canonical_guid, &ips, true).await
            }
        };

        if let Err(e) = &res {
            warn!(
                adapter_guid = %canonical_guid,
                error = %e,
                "Rollback after verification failure did not complete; backup preserved"
            );
        }
        res
    }
}

async fn retry_read<T, F, Fut>(mut op: F) -> Result<T, AppError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, AppError>>,
{
    const MAX_RETRIES: usize = 3;
    const RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(500);

    let mut attempt = 0;
    loop {
        match op().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                if attempt >= MAX_RETRIES {
                    return Err(err);
                }
                attempt += 1;
                warn!(
                    attempt,
                    max_retries = MAX_RETRIES,
                    error = %err,
                    "Post-apply follow-up read failed transiently; retrying"
                );
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin::elevation::{execute_elevated_command, ElevatedCommand};
    use crate::backup::BackupRecordStatus;
    use crate::models::{AdapterType, DhcpStatus};
    use std::fs;
    use std::sync::Mutex as StdMutex;
    use uuid::Uuid;

    struct MockNetworkState {
        adapters: HashMap<String, NetworkAdapter>,
        ipv6_states: HashMap<String, Ipv6DnsSnapshot>,
        fail_verify_with_stale_dns: bool,
        become_indeterminate_on_set: bool,
        corrupt_after_first_read_count: usize,
        fail_set_dns: bool,
        fail_reset_dns: bool,
        corrupt_ipv6_on_set: bool,
        corrupt_ipv6_on_reset: bool,
        ipv6_recovery_calls: usize,
        dhcp_dns_servers: Vec<String>,
        get_adapter_calls: usize,
        transient_adapter_read_failures_after_call: Option<(usize, usize)>,
    }

    struct MockNetworkBackend {
        state: Arc<StdMutex<MockNetworkState>>,
    }

    impl MockNetworkBackend {
        fn new(adapters: Vec<NetworkAdapter>) -> Self {
            let mut map = HashMap::new();
            let mut ipv6_map = HashMap::new();
            for ad in adapters {
                ipv6_map.insert(
                    ad.guid.clone(),
                    Ipv6DnsSnapshot {
                        server_addresses: vec!["2001:4860:4860::8888".into()],
                        static_name_server_reg: Some("2001:4860:4860::8888".into()),
                        profile_name_server_reg: None,
                        dns_mode: DnsMode::Static,
                    },
                );
                map.insert(ad.guid.clone(), ad);
            }
            Self {
                state: Arc::new(StdMutex::new(MockNetworkState {
                    adapters: map,
                    ipv6_states: ipv6_map,
                    fail_verify_with_stale_dns: false,
                    become_indeterminate_on_set: false,
                    corrupt_after_first_read_count: 0,
                    fail_set_dns: false,
                    fail_reset_dns: false,
                    corrupt_ipv6_on_set: false,
                    corrupt_ipv6_on_reset: false,
                    ipv6_recovery_calls: 0,
                    dhcp_dns_servers: vec!["192.168.1.1".into()],
                    get_adapter_calls: 0,
                    transient_adapter_read_failures_after_call: None,
                })),
            }
        }
    }

    impl NetworkBackend for MockNetworkBackend {
        fn list_adapters(&self) -> Result<Vec<NetworkAdapter>, AppError> {
            let guard = self.state.lock().unwrap();
            Ok(guard.adapters.values().cloned().collect())
        }

        fn get_adapter_by_guid(&self, adapter_guid: &str) -> Result<Option<NetworkAdapter>, AppError> {
            let mut guard = self.state.lock().unwrap();
            guard.get_adapter_calls += 1;
            let call_num = guard.get_adapter_calls;
            if let Some((start_call, remaining)) = &mut guard.transient_adapter_read_failures_after_call
            {
                if call_num >= *start_call && *remaining > 0 {
                    *remaining -= 1;
                    return Err(AppError::PowershellExecutionFailed {
                        detail: "Simulated transient WMI read failure".into(),
                    });
                }
            }
            let threshold = guard.corrupt_after_first_read_count;
            let mut found = guard.adapters.get(adapter_guid).cloned();
            if threshold > 0 && call_num >= threshold {
                if let Some(ad) = &mut found {
                    ad.current_ipv4_dns_servers = vec!["4.2.2.1".into()];
                }
            }
            Ok(found)
        }

        fn set_ipv4_dns(&self, adapter_guid: &str, servers: &[Ipv4Addr]) -> Result<(), AppError> {
            let mut guard = self.state.lock().unwrap();
            if guard.fail_set_dns {
                return Err(AppError::DnsApplyFailed {
                    detail: "Simulated Set-DnsClientServerAddress failure".into(),
                });
            }
            let stale = guard.fail_verify_with_stale_dns;
            let indet = guard.become_indeterminate_on_set;
            let corrupt_v6 = guard.corrupt_ipv6_on_set;
            let adapter = guard
                .adapters
                .get_mut(adapter_guid)
                .ok_or_else(|| AppError::AdapterNotFound {
                    guid: adapter_guid.to_string(),
                })?;

            if !stale {
                adapter.dns_mode = DnsMode::Static;
                adapter.current_ipv4_dns_servers = servers.iter().map(|ip| ip.to_string()).collect();
            }
            if indet {
                adapter.dns_mode_indeterminate_reason =
                    Some("Conflicting post-apply registry state".into());
            } else {
                adapter.dns_mode_indeterminate_reason = None;
            }

            if corrupt_v6 {
                guard.ipv6_states.insert(
                    adapter_guid.to_string(),
                    Ipv6DnsSnapshot {
                        server_addresses: vec![],
                        static_name_server_reg: None,
                        profile_name_server_reg: None,
                        dns_mode: DnsMode::Automatic,
                    },
                );
            }
            Ok(())
        }

        fn reset_ipv4_dns_to_automatic(&self, adapter_guid: &str) -> Result<(), AppError> {
            let mut guard = self.state.lock().unwrap();
            if guard.fail_reset_dns {
                return Err(AppError::DnsApplyFailed {
                    detail: "Simulated netsh reset failure".into(),
                });
            }
            let dhcp_servers = guard.dhcp_dns_servers.clone();
            let corrupt_v6 = guard.corrupt_ipv6_on_reset;
            let adapter = guard
                .adapters
                .get_mut(adapter_guid)
                .ok_or_else(|| AppError::AdapterNotFound {
                    guid: adapter_guid.to_string(),
                })?;
            adapter.dns_mode = DnsMode::Automatic;
            adapter.dns_mode_indeterminate_reason = None;
            adapter.current_ipv4_dns_servers = dhcp_servers;

            if corrupt_v6 {
                guard.ipv6_states.insert(
                    adapter_guid.to_string(),
                    Ipv6DnsSnapshot {
                        server_addresses: vec![],
                        static_name_server_reg: None,
                        profile_name_server_reg: None,
                        dns_mode: DnsMode::Automatic,
                    },
                );
            }
            Ok(())
        }

        fn get_ipv6_dns_state(&self, adapter_guid: &str) -> Result<Option<Ipv6DnsSnapshot>, AppError> {
            let guard = self.state.lock().unwrap();
            Ok(guard.ipv6_states.get(adapter_guid).cloned())
        }

        fn restore_ipv6_dns_state(
            &self,
            adapter_guid: &str,
            snapshot: &Ipv6DnsSnapshot,
        ) -> Result<(), AppError> {
            let mut guard = self.state.lock().unwrap();
            guard.ipv6_recovery_calls += 1;
            guard.ipv6_states.insert(adapter_guid.to_string(), snapshot.clone());
            Ok(())
        }
    }

    struct MockElevation {
        elevated: bool,
        can_elevate: bool,
        cancel_uac: bool,
        simulate_set_timeout: bool,
        backend: Arc<MockNetworkBackend>,
        uac_invocations: StdMutex<usize>,
    }

    impl MockElevation {
        fn new(
            elevated: bool,
            can_elevate: bool,
            cancel_uac: bool,
            backend: Arc<MockNetworkBackend>,
        ) -> Self {
            Self {
                elevated,
                can_elevate,
                cancel_uac,
                simulate_set_timeout: false,
                backend,
                uac_invocations: StdMutex::new(0),
            }
        }

        fn with_set_timeout(backend: Arc<MockNetworkBackend>) -> Self {
            Self {
                elevated: false,
                can_elevate: true,
                cancel_uac: false,
                simulate_set_timeout: true,
                backend,
                uac_invocations: StdMutex::new(0),
            }
        }

        fn invocations(&self) -> usize {
            *self.uac_invocations.lock().unwrap()
        }
    }

    impl ElevationProvider for MockElevation {
        fn is_elevated(&self) -> bool {
            self.elevated
        }

        fn can_request_elevation(&self) -> bool {
            self.can_elevate
        }

        fn run_elevated_set_dns(
            &self,
            adapter_guid: &str,
            servers: &[Ipv4Addr],
        ) -> Result<(), AppError> {
            *self.uac_invocations.lock().unwrap() += 1;
            if self.cancel_uac {
                return Err(AppError::UacCancelled);
            }
            if self.simulate_set_timeout {
                return Err(AppError::RollbackNotPerformed {
                    detail: "Elevated helper timed out after 120s and was terminated; DNS state is unknown".into(),
                });
            }
            self.backend.set_ipv4_dns(adapter_guid, servers)
        }

        fn run_elevated_set_dns_with_rollback(
            &self,
            adapter_guid: &str,
            servers: &[Ipv4Addr],
            rollback_mode: DnsMode,
            rollback_servers: &[Ipv4Addr],
        ) -> Result<(), AppError> {
            *self.uac_invocations.lock().unwrap() += 1;
            if self.cancel_uac {
                return Err(AppError::UacCancelled);
            }
            if self.simulate_set_timeout {
                return Err(AppError::RollbackNotPerformed {
                    detail: "Elevated helper timed out after 120s and was terminated; DNS state is unknown".into(),
                });
            }
            execute_elevated_command(
                self.backend.as_ref(),
                &ElevatedCommand::SetIpv4DnsWithRollback {
                    adapter_guid: adapter_guid.to_string(),
                    servers: servers.to_vec(),
                    rollback_mode,
                    rollback_servers: rollback_servers.to_vec(),
                },
            )
        }

        fn run_elevated_reset_dns(&self, adapter_guid: &str) -> Result<(), AppError> {
            *self.uac_invocations.lock().unwrap() += 1;
            if self.cancel_uac {
                return Err(AppError::UacCancelled);
            }
            self.backend.reset_ipv4_dns_to_automatic(adapter_guid)
        }
    }

    fn mk_adapter(
        guid: &str,
        idx: u32,
        status: AdapterStatus,
        mode: DnsMode,
        dns: Vec<&str>,
    ) -> NetworkAdapter {
        NetworkAdapter {
            guid: guid.into(),
            interface_index: idx,
            name: format!("Ethernet-{idx}"),
            interface_description: "Mock NIC".into(),
            adapter_type: AdapterType::Ethernet,
            operational_status: status,
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

    #[tokio::test]
    async fn automatic_dns_backup_apply_and_restore_workflow_preserves_ipv6() {
        let dir = std::env::temp_dir().join(format!("dns-engine-auto-{}", Uuid::new_v4()));
        let guid = "{11111111-2222-3333-4444-555555555555}";
        let backend = Arc::new(MockNetworkBackend::new(vec![mk_adapter(
            guid,
            10,
            AdapterStatus::Up,
            DnsMode::Automatic,
            vec!["192.168.1.1"],
        )]));
        let store = Arc::new(BackupStore::new(&dir));
        let elevation = Arc::new(MockElevation::new(true, true, false, backend.clone()));

        let engine = DnsEngine::new(backend.clone(), store.clone(), elevation);
        let applied = engine
            .apply_dns(guid, "1.1.1.1", Some("1.0.0.1"))
            .await
            .unwrap();
        assert!(applied.verified);
        assert!(applied.backup_created);
        assert_eq!(applied.applied_servers, vec!["1.1.1.1", "1.0.0.1"]);

        // Applying second provider must NOT overwrite original Automatic backup
        let applied2 = engine
            .apply_dns(guid, "8.8.8.8", Some("8.8.4.4"))
            .await
            .unwrap();
        assert!(!applied2.backup_created);

        let restored = engine.restore_dns(guid).await.unwrap();
        assert!(restored.verified);
        assert!(restored.backup_cleared);
        assert_eq!(restored.restored_mode, DnsMode::Automatic);
        assert_eq!(restored.adapter.current_ipv4_dns_servers, vec!["192.168.1.1"]);
        assert!(!store.get_status(guid).unwrap().has_backup);

        // IPv6 must remain completely intact and emergency recovery must never have been invoked
        let guard = backend.state.lock().unwrap();
        assert_eq!(guard.ipv6_recovery_calls, 0);
        let v6 = guard.ipv6_states.get(guid).unwrap();
        assert_eq!(v6.dns_mode, DnsMode::Static);
        assert_eq!(v6.server_addresses, vec!["2001:4860:4860::8888"]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn static_dns_backup_and_restore_supports_three_plus_servers_and_app_restart() {
        let dir = std::env::temp_dir().join(format!("dns-engine-restart-{}", Uuid::new_v4()));
        let guid = "{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}";
        let original_servers = vec!["9.9.9.9", "149.112.112.112", "8.8.8.8", "1.0.0.1"];
        let backend = Arc::new(MockNetworkBackend::new(vec![mk_adapter(
            guid,
            4,
            AdapterStatus::Up,
            DnsMode::Static,
            original_servers.clone(),
        )]));

        {
            let store = Arc::new(BackupStore::new(&dir));
            let elevation = Arc::new(MockElevation::new(true, true, false, backend.clone()));
            let engine = DnsEngine::new(backend.clone(), store, elevation);
            let res = engine.apply_dns(guid, "1.1.1.1", Some("1.0.0.1")).await.unwrap();
            assert!(res.verified);
        }

        let restarted_store = Arc::new(BackupStore::new(&dir));
        let restarted_elevation = Arc::new(MockElevation::new(true, true, false, backend.clone()));
        let restarted_engine = DnsEngine::new(backend.clone(), restarted_store, restarted_elevation);

        let restored = restarted_engine.restore_dns(guid).await.unwrap();
        assert_eq!(restored.restored_mode, DnsMode::Static);
        assert_eq!(restored.restored_servers, original_servers);

        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn detects_unexpected_ipv6_modification_attempts_recovery_and_preserves_backup() {
        let dir = std::env::temp_dir().join(format!("dns-engine-v6-err-{}", Uuid::new_v4()));
        let guid = "{77777777-8888-9999-AAAA-BBBBBBBBBBBB}";
        let backend = Arc::new(MockNetworkBackend::new(vec![mk_adapter(
            guid,
            15,
            AdapterStatus::Up,
            DnsMode::Automatic,
            vec!["192.168.1.1"],
        )]));

        let store = Arc::new(BackupStore::new(&dir));
        let elevation = Arc::new(MockElevation::new(true, true, false, backend.clone()));
        let engine = DnsEngine::new(backend.clone(), store.clone(), elevation);

        // Case 1: Unexpected IPv6 change during apply_dns
        backend.state.lock().unwrap().corrupt_ipv6_on_set = true;
        let err = engine
            .apply_dns(guid, "1.1.1.1", Some("1.0.0.1"))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Ipv6ModificationDetected { .. }));
        assert!(store.get_status(guid).unwrap().has_backup);
        {
            let guard = backend.state.lock().unwrap();
            assert_eq!(guard.ipv6_recovery_calls, 1);
            assert_eq!(
                guard.ipv6_states.get(guid).unwrap().server_addresses,
                vec!["2001:4860:4860::8888"]
            );
        }

        // Case 2: Unexpected IPv6 change during restore_dns must preserve backup and recover IPv6
        {
            let mut guard = backend.state.lock().unwrap();
            guard.corrupt_ipv6_on_set = false;
            guard.corrupt_ipv6_on_reset = true;
        }
        let err2 = engine.restore_dns(guid).await.unwrap_err();
        assert!(matches!(err2, AppError::Ipv6ModificationDetected { .. }));
        assert!(store.get_status(guid).unwrap().has_backup);
        assert_eq!(backend.state.lock().unwrap().ipv6_recovery_calls, 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn refuses_apply_and_backup_when_original_dns_mode_is_indeterminate() {
        let dir = std::env::temp_dir().join(format!("dns-engine-indet-{}", Uuid::new_v4()));
        let guid = "{66666666-7777-8888-9999-AAAAAAAAAAAA}";
        let mut adapter = mk_adapter(guid, 12, AdapterStatus::Up, DnsMode::Automatic, vec![]);
        adapter.dns_mode_indeterminate_reason =
            Some("Tcpip interface registry key missing".into());

        let backend = Arc::new(MockNetworkBackend::new(vec![adapter]));
        let store = Arc::new(BackupStore::new(&dir));
        let elevation = Arc::new(MockElevation::new(true, true, false, backend.clone()));
        let engine = DnsEngine::new(backend, store.clone(), elevation);

        let err = engine
            .apply_dns(guid, "1.1.1.1", Some("1.0.0.1"))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::DnsModeIndeterminate { .. }));
        assert!(!store.get_status(guid).unwrap().has_backup);

        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn post_apply_indeterminate_rolls_back_and_non_elevated_uses_single_uac_prompt() {
        let dir = std::env::temp_dir().join(format!("dns-engine-indet-rb-{}", Uuid::new_v4()));
        let guid = "{88888888-1111-2222-3333-444444444444}";
        let backend = Arc::new(MockNetworkBackend::new(vec![mk_adapter(
            guid,
            9,
            AdapterStatus::Up,
            DnsMode::Automatic,
            vec!["192.168.1.1"],
        )]));
        backend.state.lock().unwrap().become_indeterminate_on_set = true;

        let store = Arc::new(BackupStore::new(&dir));
        let elevation = Arc::new(MockElevation::new(false, true, false, backend.clone()));
        let engine = DnsEngine::new(backend.clone(), store.clone(), elevation.clone());

        let err = engine
            .apply_dns(guid, "1.1.1.1", Some("1.0.0.1"))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::DnsModeIndeterminate { .. }));
        // Single UAC invocation handled apply + verify + rollback atomically.
        assert_eq!(elevation.invocations(), 1);
        assert_eq!(
            backend.get_adapter_by_guid(guid).unwrap().unwrap().dns_mode,
            DnsMode::Automatic
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn non_elevated_post_helper_discrepancy_returns_rollback_not_performed_without_second_uac() {
        let dir = std::env::temp_dir().join(format!("dns-engine-nouac2-{}", Uuid::new_v4()));
        let guid = "{99999999-1111-2222-3333-444444444444}";
        let backend = Arc::new(MockNetworkBackend::new(vec![mk_adapter(
            guid,
            9,
            AdapterStatus::Up,
            DnsMode::Automatic,
            vec!["192.168.1.1"],
        )]));
        // Call 1: pre-apply read in engine
        // Call 2: inside helper verification (passes)
        // Call 3: outer engine re-read sees corrupted servers
        backend.state.lock().unwrap().corrupt_after_first_read_count = 3;

        let store = Arc::new(BackupStore::new(&dir));
        let elevation = Arc::new(MockElevation::new(false, true, false, backend.clone()));
        let engine = DnsEngine::new(backend, store, elevation.clone());

        let err = engine
            .apply_dns(guid, "1.1.1.1", Some("1.0.0.1"))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::RollbackNotPerformed { .. }));
        assert_eq!(elevation.invocations(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn handles_invalid_guid_invalid_dns_disabled_and_disconnected_adapters() {
        let dir = std::env::temp_dir().join(format!("dns-engine-errs-{}", Uuid::new_v4()));
        let disabled_guid = "{11111111-1111-1111-1111-111111111111}";
        let disconnected_guid = "{22222222-2222-2222-2222-222222222222}";
        let missing_guid = "{33333333-3333-3333-3333-333333333333}";

        let backend = Arc::new(MockNetworkBackend::new(vec![
            mk_adapter(disabled_guid, 1, AdapterStatus::Disabled, DnsMode::Automatic, vec![]),
            mk_adapter(
                disconnected_guid,
                2,
                AdapterStatus::Disconnected,
                DnsMode::Automatic,
                vec![],
            ),
        ]));
        let store = Arc::new(BackupStore::new(&dir));
        let elevation = Arc::new(MockElevation::new(true, true, false, backend.clone()));
        let engine = DnsEngine::new(backend, store, elevation);

        assert!(matches!(
            engine.apply_dns("not-a-guid", "1.1.1.1", Some("1.0.0.1")).await,
            Err(AppError::InvalidAdapterGuid { .. })
        ));
        assert!(matches!(
            engine
                .apply_dns(disabled_guid, "999.1.1.1", Some("1.0.0.1"))
                .await,
            Err(AppError::InvalidDnsAddress { .. })
        ));
        assert!(matches!(
            engine
                .apply_dns(missing_guid, "1.1.1.1", Some("1.0.0.1"))
                .await,
            Err(AppError::AdapterNotFound { .. })
        ));
        assert!(matches!(
            engine
                .apply_dns(disabled_guid, "1.1.1.1", Some("1.0.0.1"))
                .await,
            Err(AppError::AdapterDisabled { .. })
        ));
        assert!(matches!(
            engine
                .apply_dns(disconnected_guid, "1.1.1.1", Some("1.0.0.1"))
                .await,
            Err(AppError::AdapterDisconnected { .. })
        ));

        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn handles_permission_denial_and_uac_cancellation() {
        let dir = std::env::temp_dir().join(format!("dns-engine-uac-{}", Uuid::new_v4()));
        let guid = "{44444444-4444-4444-4444-444444444444}";
        let backend = Arc::new(MockNetworkBackend::new(vec![mk_adapter(
            guid,
            8,
            AdapterStatus::Up,
            DnsMode::Automatic,
            vec!["192.168.1.1"],
        )]));
        let store = Arc::new(BackupStore::new(&dir));

        let denied_engine = DnsEngine::new(
            backend.clone(),
            store.clone(),
            Arc::new(MockElevation::new(false, false, false, backend.clone())),
        );
        assert!(matches!(
            denied_engine
                .apply_dns(guid, "1.1.1.1", Some("1.0.0.1"))
                .await,
            Err(AppError::AdminAccessDenied { .. })
        ));

        let uac_engine = DnsEngine::new(
            backend.clone(),
            store.clone(),
            Arc::new(MockElevation::new(false, true, true, backend.clone())),
        );
        assert!(matches!(
            uac_engine
                .apply_dns(guid, "1.1.1.1", Some("1.0.0.1"))
                .await,
            Err(AppError::UacCancelled)
        ));
        assert!(!store.get_status(guid).unwrap().has_backup);

        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn preserves_backup_on_verification_failure_and_uses_restore_failed_on_restore_error() {
        let dir = std::env::temp_dir().join(format!("dns-engine-verify-{}", Uuid::new_v4()));
        let guid = "{55555555-5555-5555-5555-555555555555}";
        let backend = Arc::new(MockNetworkBackend::new(vec![mk_adapter(
            guid,
            3,
            AdapterStatus::Up,
            DnsMode::Static,
            vec!["8.8.8.8", "8.8.4.4"],
        )]));
        backend.state.lock().unwrap().fail_verify_with_stale_dns = true;

        let store = Arc::new(BackupStore::new(&dir));
        let elevation = Arc::new(MockElevation::new(true, true, false, backend.clone()));
        let engine = DnsEngine::new(backend.clone(), store.clone(), elevation);

        let err = engine
            .apply_dns(guid, "1.1.1.1", Some("1.0.0.1"))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::VerificationFailed { .. }));

        let status = store.get_status(guid).unwrap();
        assert!(status.has_backup);
        assert!(status.is_valid);
        assert_eq!(
            status.backup.unwrap().original_ipv4_dns_servers,
            vec!["8.8.8.8", "8.8.4.4"]
        );

        // Restore failure returns RestoreFailed and preserves backup
        let restore_err = engine.restore_dns(guid).await.unwrap_err();
        assert!(matches!(restore_err, AppError::RestoreFailed { .. }));
        assert!(store.get_status(guid).unwrap().has_backup);

        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn elevated_set_timeout_rereads_adapter_preserves_backup_and_returns_rollback_not_performed(
    ) {
        let dir = std::env::temp_dir().join(format!("dns-engine-uac-timeout-{}", Uuid::new_v4()));
        let guid = "{AAAAAAAA-1111-2222-3333-BBBBBBBBBBBB}";
        let backend = Arc::new(MockNetworkBackend::new(vec![mk_adapter(
            guid,
            7,
            AdapterStatus::Up,
            DnsMode::Automatic,
            vec!["192.168.1.1"],
        )]));

        let store = Arc::new(BackupStore::new(&dir));
        let elevation = Arc::new(MockElevation::with_set_timeout(backend.clone()));
        let engine = DnsEngine::new(backend.clone(), store.clone(), elevation);

        let err = engine
            .apply_dns(guid, "1.1.1.1", Some("1.0.0.1"))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::RollbackNotPerformed { .. }));

        // Pre-apply read (1) + post-timeout state observation re-read (2).
        assert_eq!(backend.state.lock().unwrap().get_adapter_calls, 2);

        let status = store.get_status(guid).unwrap();
        assert!(status.has_backup);
        assert!(status.is_valid);
        assert_eq!(
            status.backup.unwrap().backup_status,
            BackupRecordStatus::ApplyFailedPreserved
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn non_elevated_apply_retries_transient_followup_read_and_succeeds() {
        let dir = std::env::temp_dir().join(format!("dns-engine-retry-read-{}", Uuid::new_v4()));
        let guid = "{BBBBBBBB-1111-2222-3333-CCCCCCCCCCCC}";
        let backend = Arc::new(MockNetworkBackend::new(vec![mk_adapter(
            guid,
            11,
            AdapterStatus::Up,
            DnsMode::Automatic,
            vec!["192.168.1.1"],
        )]));
        // Call 1: engine pre-apply read (succeeds)
        // Call 2: helper verification read (succeeds)
        // Call 3: engine follow-up read #1 fails once, then call 4 succeeds on retry
        backend
            .state
            .lock()
            .unwrap()
            .transient_adapter_read_failures_after_call = Some((3, 1));

        let store = Arc::new(BackupStore::new(&dir));
        let elevation = Arc::new(MockElevation::new(false, true, false, backend.clone()));
        let engine = DnsEngine::new(backend.clone(), store.clone(), elevation.clone());

        let res = engine
            .apply_dns(guid, "1.1.1.1", Some("1.0.0.1"))
            .await
            .unwrap();
        assert!(res.verified);
        assert_eq!(res.applied_servers, vec!["1.1.1.1", "1.0.0.1"]);
        assert_eq!(elevation.invocations(), 1);
        assert_eq!(backend.state.lock().unwrap().get_adapter_calls, 4);

        let status = store.get_status(guid).unwrap();
        assert!(status.has_backup);
        assert_eq!(
            status.backup.unwrap().backup_status,
            BackupRecordStatus::Protected
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
