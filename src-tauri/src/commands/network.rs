use tauri::State;

use crate::errors::CommandError;
use crate::models::NetworkAdapter;
use crate::AppState;

#[tauri::command]
pub async fn get_network_adapters(
    state: State<'_, AppState>,
) -> Result<Vec<NetworkAdapter>, CommandError> {
    state
        .engine
        .list_adapters()
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_adapter_details(
    state: State<'_, AppState>,
    adapter_guid: String,
) -> Result<NetworkAdapter, CommandError> {
    state
        .engine
        .get_adapter_details(&adapter_guid)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn refresh_network_information(
    state: State<'_, AppState>,
) -> Result<Vec<NetworkAdapter>, CommandError> {
    state
        .engine
        .list_adapters()
        .await
        .map_err(CommandError::from)
}
