use tauri::State;

use crate::backup::BackupStatusSummary;
use crate::errors::CommandError;
use crate::models::{ApplyDnsResult, RestoreDnsResult};
use crate::AppState;

#[tauri::command]
pub async fn get_backup_status(
    state: State<'_, AppState>,
    adapter_guid: String,
) -> Result<BackupStatusSummary, CommandError> {
    state
        .engine
        .get_backup_status(&adapter_guid)
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn list_all_backups(
    state: State<'_, AppState>,
) -> Result<Vec<BackupStatusSummary>, CommandError> {
    state
        .engine
        .list_all_backups()
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn apply_dns(
    state: State<'_, AppState>,
    adapter_guid: String,
    primary_dns: String,
    secondary_dns: Option<String>,
) -> Result<ApplyDnsResult, CommandError> {
    state
        .engine
        .apply_dns(&adapter_guid, &primary_dns, secondary_dns.as_deref())
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn restore_dns(
    state: State<'_, AppState>,
    adapter_guid: String,
) -> Result<RestoreDnsResult, CommandError> {
    state
        .engine
        .restore_dns(&adapter_guid)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn discard_corrupt_backup(
    state: State<'_, AppState>,
    adapter_guid: String,
) -> Result<(), CommandError> {
    state
        .engine
        .discard_corrupt_backup(&adapter_guid)
        .map_err(CommandError::from)
}
