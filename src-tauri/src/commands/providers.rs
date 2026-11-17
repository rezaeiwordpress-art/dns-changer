use tauri::State;

use crate::dns::build_provider_catalog;
use crate::errors::{AppError, CommandError};
use crate::models::{DnsProvider, DnsProviderCatalog};
use crate::storage::AppConfig;
use crate::AppState;

#[tauri::command]
pub async fn get_dns_providers(
    state: State<'_, AppState>,
) -> Result<DnsProviderCatalog, CommandError> {
    let config = state.config_store.load().map_err(CommandError::from)?;
    Ok(build_provider_catalog(config.custom_providers))
}

#[tauri::command]
pub async fn get_app_config(state: State<'_, AppState>) -> Result<AppConfig, CommandError> {
    state.config_store.load().map_err(CommandError::from)
}

#[tauri::command]
pub async fn save_app_preferences(
    state: State<'_, AppState>,
    language: Option<String>,
    theme: Option<String>,
    selected_adapter_guid: Option<String>,
    selected_provider_id: Option<String>,
    remember_selected_adapter: Option<bool>,
    remember_selected_provider: Option<bool>,
    start_page: Option<String>,
    confirm_before_restore: Option<bool>,
    show_notifications: Option<bool>,
) -> Result<AppConfig, CommandError> {
    state
        .config_store
        .update(|cfg| {
            if let Some(lang) = language {
                if lang == "fa" || lang == "en" {
                    cfg.language = lang;
                } else {
                    return Err(AppError::InvalidInput {
                        detail: format!("Unsupported language '{lang}'"),
                    });
                }
            }
            if let Some(t) = theme {
                if t == "dark" || t == "light" || t == "system" {
                    cfg.theme = t;
                } else {
                    return Err(AppError::InvalidInput {
                        detail: format!("Unsupported theme '{t}'"),
                    });
                }
            }
            if let Some(guid) = selected_adapter_guid {
                cfg.selected_adapter_guid = if guid.trim().is_empty() {
                    None
                } else {
                    Some(guid)
                };
            }
            if let Some(pid) = selected_provider_id {
                cfg.selected_provider_id = if pid.trim().is_empty() {
                    None
                } else {
                    Some(pid)
                };
            }
            if let Some(v) = remember_selected_adapter {
                cfg.remember_selected_adapter = v;
                if !v {
                    cfg.selected_adapter_guid = None;
                }
            }
            if let Some(v) = remember_selected_provider {
                cfg.remember_selected_provider = v;
                if !v {
                    cfg.selected_provider_id = None;
                }
            }
            if let Some(page) = start_page {
                if matches!(
                    page.as_str(),
                    "dashboard" | "providers" | "customDns" | "adapters" | "settings" | "about"
                ) {
                    cfg.start_page = page;
                } else {
                    return Err(AppError::InvalidInput {
                        detail: format!("Unsupported start page '{page}'"),
                    });
                }
            }
            if let Some(v) = confirm_before_restore {
                cfg.confirm_before_restore = v;
            }
            if let Some(v) = show_notifications {
                cfg.show_notifications = v;
            }
            Ok(())
        })
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn add_custom_provider(
    state: State<'_, AppState>,
    name: String,
    primary_dns: String,
    secondary_dns: Option<String>,
    description: Option<String>,
) -> Result<DnsProvider, CommandError> {
    state
        .config_store
        .add_custom_provider(name, primary_dns, secondary_dns, description)
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn update_custom_provider(
    state: State<'_, AppState>,
    provider_id: String,
    name: String,
    primary_dns: String,
    secondary_dns: Option<String>,
    description: Option<String>,
) -> Result<DnsProvider, CommandError> {
    state
        .config_store
        .update_custom_provider(provider_id, name, primary_dns, secondary_dns, description)
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn delete_custom_provider(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<DnsProviderCatalog, CommandError> {
    let remaining = state
        .config_store
        .delete_custom_provider(&provider_id)
        .map_err(CommandError::from)?;
    Ok(build_provider_catalog(remaining))
}
