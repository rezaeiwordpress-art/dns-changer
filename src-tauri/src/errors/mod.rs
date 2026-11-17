use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    AdapterNotFound,
    AdapterDisabled,
    AdapterDisconnected,
    InvalidDnsAddress,
    InvalidAdapterGuid,
    AdminAccessDenied,
    UacCancelled,
    ElevationFailed,
    PowershellExecutionFailed,
    PowershellBlocked,
    PowershellTimeout,
    DnsApplyFailed,
    VerificationFailed,
    BackupCorruption,
    BackupNotFound,
    RestoreFailed,
    NetworkChanged,
    ConcurrentOperation,
    UnsupportedPlatform,
    StorageError,
    DnsModeIndeterminate,
    Ipv6ModificationDetected,
    Ipv6RecoveryFailed,
    DuplicateProviderName,
    InvalidInput,
    RollbackNotPerformed,
    InternalError,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Network adapter not found: {guid}")]
    AdapterNotFound { guid: String },

    #[error("Network adapter '{name}' is disabled")]
    AdapterDisabled { guid: String, name: String },

    #[error("Network adapter '{name}' is disconnected")]
    AdapterDisconnected { guid: String, name: String },

    #[error("Invalid IPv4 DNS address '{address}': {reason}")]
    InvalidDnsAddress { address: String, reason: String },

    #[error("Invalid adapter GUID '{guid}'")]
    InvalidAdapterGuid { guid: String },

    #[error("Administrator privileges are required to modify DNS settings")]
    AdminAccessDenied { detail: String },

    #[error("Administrator elevation was cancelled by the user")]
    UacCancelled,

    #[error("Failed to launch elevated helper process: {detail}")]
    ElevationFailed { detail: String },

    #[error("PowerShell execution failed: {detail}")]
    PowershellExecutionFailed { detail: String },

    #[error("PowerShell execution was blocked: {detail}")]
    PowershellBlocked { detail: String },

    #[error("PowerShell operation timed out after {timeout_secs}s: {detail}")]
    PowershellTimeout { timeout_secs: u64, detail: String },

    #[error("Failed to apply DNS configuration: {detail}")]
    DnsApplyFailed { detail: String },

    #[error("DNS verification failed: {detail}")]
    VerificationFailed { detail: String },

    #[error("DNS backup integrity check failed: {detail}")]
    BackupCorruption { detail: String },

    #[error("No original DNS backup found for adapter {guid}")]
    BackupNotFound { guid: String },

    #[error("Failed to restore original DNS configuration: {detail}")]
    RestoreFailed { detail: String },

    #[error("Network adapter state changed during operation: {detail}")]
    NetworkChanged { detail: String },

    #[error("Another DNS operation is already in progress for adapter {guid}")]
    ConcurrentOperation { guid: String },

    #[error("Windows networking operations require Windows 10 or Windows 11")]
    UnsupportedPlatform,

    #[error("Local storage error: {detail}")]
    StorageError { detail: String },

    #[error("Could not confidently determine IPv4 DNS mode for adapter {guid}: {detail}")]
    DnsModeIndeterminate { guid: String, detail: String },

    #[error("Unexpected IPv6 DNS change detected during IPv4 operation: {detail}")]
    Ipv6ModificationDetected { detail: String },

    #[error("Unexpected IPv6 DNS change detected and IPv6 recovery failed: {detail}")]
    Ipv6RecoveryFailed { detail: String },

    #[error("Duplicate DNS provider name '{name}'")]
    DuplicateProviderName { name: String },

    #[error("Invalid input: {detail}")]
    InvalidInput { detail: String },

    #[error("DNS changed, rollback not performed: {detail}")]
    RollbackNotPerformed { detail: String },

    #[error("Internal engine error: {detail}")]
    InternalError { detail: String },
}

impl AppError {
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::AdapterNotFound { .. } => ErrorCode::AdapterNotFound,
            Self::AdapterDisabled { .. } => ErrorCode::AdapterDisabled,
            Self::AdapterDisconnected { .. } => ErrorCode::AdapterDisconnected,
            Self::InvalidDnsAddress { .. } => ErrorCode::InvalidDnsAddress,
            Self::InvalidAdapterGuid { .. } => ErrorCode::InvalidAdapterGuid,
            Self::AdminAccessDenied { .. } => ErrorCode::AdminAccessDenied,
            Self::UacCancelled => ErrorCode::UacCancelled,
            Self::ElevationFailed { .. } => ErrorCode::ElevationFailed,
            Self::PowershellExecutionFailed { .. } => ErrorCode::PowershellExecutionFailed,
            Self::PowershellBlocked { .. } => ErrorCode::PowershellBlocked,
            Self::PowershellTimeout { .. } => ErrorCode::PowershellTimeout,
            Self::DnsApplyFailed { .. } => ErrorCode::DnsApplyFailed,
            Self::VerificationFailed { .. } => ErrorCode::VerificationFailed,
            Self::BackupCorruption { .. } => ErrorCode::BackupCorruption,
            Self::BackupNotFound { .. } => ErrorCode::BackupNotFound,
            Self::RestoreFailed { .. } => ErrorCode::RestoreFailed,
            Self::NetworkChanged { .. } => ErrorCode::NetworkChanged,
            Self::ConcurrentOperation { .. } => ErrorCode::ConcurrentOperation,
            Self::UnsupportedPlatform => ErrorCode::UnsupportedPlatform,
            Self::StorageError { .. } => ErrorCode::StorageError,
            Self::DnsModeIndeterminate { .. } => ErrorCode::DnsModeIndeterminate,
            Self::Ipv6ModificationDetected { .. } => ErrorCode::Ipv6ModificationDetected,
            Self::Ipv6RecoveryFailed { .. } => ErrorCode::Ipv6RecoveryFailed,
            Self::DuplicateProviderName { .. } => ErrorCode::DuplicateProviderName,
            Self::InvalidInput { .. } => ErrorCode::InvalidInput,
            Self::RollbackNotPerformed { .. } => ErrorCode::RollbackNotPerformed,
            Self::InternalError { .. } => ErrorCode::InternalError,
        }
    }

    /// Safe user-facing message that never leaks stack traces or raw shell internals.
    pub fn user_message(&self) -> String {
        match self {
            Self::AdapterNotFound { .. } => {
                "The selected network adapter could not be found. Please refresh adapters and try again.".into()
            }
            Self::AdapterDisabled { name, .. } => {
                format!("Network adapter \"{name}\" is currently disabled. Enable it in Windows before changing DNS.")
            }
            Self::AdapterDisconnected { name, .. } => {
                format!("Network adapter \"{name}\" is disconnected. Connect the adapter before applying DNS settings.")
            }
            Self::InvalidDnsAddress { address, reason } => {
                format!("Invalid IPv4 DNS address \"{address}\": {reason}.")
            }
            Self::InvalidAdapterGuid { .. } => {
                "The provided network adapter identifier is not a valid GUID.".into()
            }
            Self::AdminAccessDenied { .. } => {
                "Administrator access was denied. DNS changes require elevated Windows privileges.".into()
            }
            Self::UacCancelled => {
                "Administrator elevation was cancelled (UAC). No DNS settings were changed.".into()
            }
            Self::ElevationFailed { .. } => {
                "Could not start the elevated DNS helper process. Please try again or run as Administrator.".into()
            }
            Self::PowershellExecutionFailed { .. } => {
                "Windows network configuration command failed. Check application logs for diagnostics.".into()
            }
            Self::PowershellBlocked { .. } => {
                "PowerShell execution was blocked by antivirus software, security policy, or a missing system component. Check your antivirus or Windows Features.".into()
            }
            Self::PowershellTimeout { timeout_secs, .. } => {
                format!("The Windows network operation timed out after {timeout_secs} seconds.")
            }
            Self::DnsApplyFailed { .. } => {
                "Windows could not apply the requested DNS servers. Your original backup was preserved.".into()
            }
            Self::VerificationFailed { .. } => {
                "DNS settings did not match the expected configuration after applying. Original backup was preserved.".into()
            }
            Self::BackupCorruption { .. } => {
                "The stored DNS backup file failed integrity validation and was not overwritten.".into()
            }
            Self::BackupNotFound { .. } => {
                "No original DNS backup exists for the selected adapter.".into()
            }
            Self::RestoreFailed { .. } => {
                "Could not restore the original DNS configuration. Your backup file has been preserved.".into()
            }
            Self::NetworkChanged { .. } => {
                "The network adapter changed or disappeared during the operation. Please refresh and verify.".into()
            }
            Self::ConcurrentOperation { .. } => {
                "A DNS modification is already in progress for this adapter. Wait for it to complete.".into()
            }
            Self::UnsupportedPlatform => {
                "DNS modification is only supported on Windows 10 and Windows 11.".into()
            }
            Self::StorageError { .. } => {
                "Failed to read or write local application storage.".into()
            }
            Self::DnsModeIndeterminate { .. } => {
                "Could not confidently verify the adapter's current IPv4 DNS mode. Check adapter configuration and refresh.".into()
            }
            Self::Ipv6ModificationDetected { .. } => {
                "An unexpected IPv6 DNS change was detected during the IPv4 operation. Original settings were preserved and recovery was attempted.".into()
            }
            Self::Ipv6RecoveryFailed { .. } => {
                "An unexpected IPv6 DNS change occurred; IPv4 settings were rolled back, but automatic IPv6 recovery failed. Please check your adapter's IPv6 DNS settings in Windows.".into()
            }
            Self::DuplicateProviderName { .. } => {
                "A DNS provider with this name already exists.".into()
            }
            Self::InvalidInput { .. } => {
                "One or more input values are invalid. Please review your entries.".into()
            }
            Self::RollbackNotPerformed { .. } => {
                "DNS changed, rollback not performed. Your original backup was preserved so you can retry or restore manually.".into()
            }
            Self::InternalError { .. } => {
                "An unexpected internal error occurred. Check application logs for details.".into()
            }
        }
    }
}

const TECHNICAL_DETAIL_MAX_CHARS: usize = 1000;

fn sanitize_user_profile_paths(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < bytes.len() {
        if i + 9 <= bytes.len()
            && bytes[i].is_ascii_alphabetic()
            && bytes[i + 1] == b':'
            && (bytes[i + 2] == b'\\' || bytes[i + 2] == b'/')
            && bytes[i + 3..i + 8].eq_ignore_ascii_case(b"users")
            && (bytes[i + 8] == b'\\' || bytes[i + 8] == b'/')
        {
            let is_boundary = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            if is_boundary {
                let name_start = i + 9;
                if let Some(rest_str) = input.get(name_start..) {
                    let mut sep_end: Option<usize> = None;
                    for (offset, ch) in rest_str.char_indices() {
                        if ch == '\\' || ch == '/' {
                            if offset > 0 {
                                sep_end = Some(offset);
                            }
                            break;
                        }
                        if ch == '\r' || ch == '\n' || ch == '"' || ch == '\'' || ch == ';' {
                            break;
                        }
                        if offset > 128 {
                            break;
                        }
                    }

                    if let Some(seg_bytes) = sep_end {
                        out.push_str("%USERPROFILE%\\");
                        i = name_start + seg_bytes + 1;
                        continue;
                    }

                    let mut token_end = rest_str.len();
                    for (offset, ch) in rest_str.char_indices() {
                        if ch.is_whitespace()
                            || matches!(ch, '"' | '\'' | ';' | ',' | ')' | ']')
                        {
                            token_end = offset;
                            break;
                        }
                    }
                    if token_end > 0 {
                        out.push_str("%USERPROFILE%");
                        i = name_start + token_end;
                        continue;
                    }
                }
            }
        }

        let ch = input[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }

    out
}

fn sanitize_technical_detail(raw: &str) -> String {
    let no_controls: String = raw
        .chars()
        .filter_map(|c| {
            if c == '\n' || c == '\r' || c == '\t' {
                Some(' ')
            } else if c.is_control() {
                None
            } else {
                Some(c)
            }
        })
        .collect();

    let sanitized = sanitize_user_profile_paths(no_controls.trim());
    sanitized
        .trim()
        .chars()
        .take(TECHNICAL_DETAIL_MAX_CHARS)
        .collect()
}

impl AppError {
    fn includes_technical_detail(&self) -> bool {
        !matches!(self, Self::UacCancelled | Self::UnsupportedPlatform)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub technical_detail: Option<String>,
}

impl From<AppError> for CommandError {
    fn from(err: AppError) -> Self {
        let code = err.code();
        let message = err.user_message();
        let technical_detail = if err.includes_technical_detail() {
            let cleaned = sanitize_technical_detail(&err.to_string());
            if cleaned.is_empty() {
                None
            } else {
                Some(cleaned)
            }
        } else {
            None
        };
        error!(error_code = ?code, technical_detail = %err, "Command failed");
        Self {
            code,
            message,
            technical_detail,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv6_recovery_failed_maps_and_serializes_as_screaming_snake_case() {
        let err = AppError::Ipv6RecoveryFailed {
            detail: "netsh ipv6 failed".into(),
        };
        assert_eq!(err.code(), ErrorCode::Ipv6RecoveryFailed);
        let serialized = serde_json::to_string(&err.code()).unwrap();
        assert_eq!(serialized, "\"IPV6_RECOVERY_FAILED\"");
    }

    #[test]
    fn invalid_input_and_duplicate_provider_name_return_generic_user_messages() {
        let dup = AppError::DuplicateProviderName {
            name: "SecretInternalProviderName".into(),
        };
        let dup_msg = dup.user_message();
        assert!(!dup_msg.contains("SecretInternalProviderName"));
        assert!(dup.to_string().contains("SecretInternalProviderName"));

        let inv = AppError::InvalidInput {
            detail: "Raw English internal validation detail".into(),
        };
        let inv_msg = inv.user_message();
        assert!(!inv_msg.contains("Raw English internal validation detail"));
        assert!(inv.to_string().contains("Raw English internal validation detail"));
    }

    #[test]
    fn powershell_blocked_maps_and_serializes_as_screaming_snake_case() {
        let err = AppError::PowershellBlocked {
            detail: "blocked by your antivirus".into(),
        };
        assert_eq!(err.code(), ErrorCode::PowershellBlocked);
        let serialized = serde_json::to_string(&err.code()).unwrap();
        assert_eq!(serialized, "\"POWERSHELL_BLOCKED\"");
    }

    #[test]
    fn command_error_includes_sanitized_truncated_technical_detail_for_powershell_failure() {
        let long_tail = "z".repeat(1200);
        let err = AppError::PowershellExecutionFailed {
            detail: format!(
                "script failed at C:\\Users\\Alice Smith\\AppData\\Local\\Temp\\test.ps1\u{0007}\nwith detail: {long_tail}"
            ),
        };

        let cmd_err = CommandError::from(err);
        assert_eq!(cmd_err.code, ErrorCode::PowershellExecutionFailed);
        let detail = cmd_err
            .technical_detail
            .as_deref()
            .expect("Expected technical_detail to be populated");

        assert!(detail.contains("%USERPROFILE%\\AppData\\Local\\Temp\\test.ps1"));
        assert!(!detail.contains("Alice Smith"));
        assert!(!detail.chars().any(|c| c.is_control()));
        assert_eq!(detail.chars().count(), 1000);
    }

    #[test]
    fn command_error_omits_technical_detail_for_uac_cancelled() {
        let cmd_err = CommandError::from(AppError::UacCancelled);
        assert_eq!(cmd_err.code, ErrorCode::UacCancelled);
        assert!(cmd_err.technical_detail.is_none());

        let json = serde_json::to_string(&cmd_err).unwrap();
        assert!(!json.contains("technicalDetail"));
    }
}
