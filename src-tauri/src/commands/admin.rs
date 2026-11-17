use tauri::State;

use crate::admin::RealWindowsElevation;
use crate::errors::CommandError;
use crate::models::AdminStatus;
use crate::AppState;

#[tauri::command]
pub async fn check_admin_privileges(
    state: State<'_, AppState>,
) -> Result<AdminStatus, CommandError> {
    Ok(AdminStatus {
        is_elevated: state.engine.is_elevated(),
        can_request_elevation: state.engine.can_request_elevation(),
        platform: RealWindowsElevation::current_platform().to_string(),
    })
}
