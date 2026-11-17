pub mod admin;
pub mod backup;
pub mod commands;
pub mod dns;
pub mod errors;
pub mod logging;
pub mod models;
pub mod network;
pub mod storage;

use std::sync::Arc;
use tauri::Manager;
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;

use crate::admin::RealWindowsElevation;
use crate::backup::BackupStore;
use crate::dns::DnsEngine;
use crate::network::PowerShellNetworkBackend;
use crate::storage::AppConfigStore;

pub struct AppState {
    pub engine: Arc<DnsEngine>,
    pub config_store: Arc<AppConfigStore>,
    pub _log_guard: Option<WorkerGuard>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let local_data_dir = app.path().app_local_data_dir().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Failed to resolve application local data directory: {e}"),
                )
            })?;

            std::fs::create_dir_all(&local_data_dir)?;

            let log_guard = logging::init_logging(&local_data_dir.join("logs"));
            info!(data_dir = %local_data_dir.display(), "Initializing DNS Changer backend");

            let backup_store = Arc::new(BackupStore::new(&local_data_dir));
            let config_store = Arc::new(AppConfigStore::new(&local_data_dir));
            let network_backend = Arc::new(PowerShellNetworkBackend::new());
            let elevation_provider = Arc::new(RealWindowsElevation::new());

            let engine = Arc::new(DnsEngine::new(
                network_backend,
                backup_store,
                elevation_provider,
            ));

            app.manage(AppState {
                engine,
                config_store,
                _log_guard: log_guard,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::network::get_network_adapters,
            commands::network::get_adapter_details,
            commands::network::refresh_network_information,
            commands::providers::get_dns_providers,
            commands::providers::get_app_config,
            commands::providers::save_app_preferences,
            commands::providers::add_custom_provider,
            commands::providers::update_custom_provider,
            commands::providers::delete_custom_provider,
            commands::dns::get_backup_status,
            commands::dns::list_all_backups,
            commands::dns::apply_dns,
            commands::dns::restore_dns,
            commands::dns::discard_corrupt_backup,
            commands::admin::check_admin_privileges,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DNS Changer application");
}
