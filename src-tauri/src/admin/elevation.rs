use std::net::Ipv4Addr;
use std::path::PathBuf;
use tracing::{error, info, warn};
use tracing_appender::non_blocking::WorkerGuard;

use crate::dns::validation::{validate_adapter_guid, validate_dns_list, validate_ipv4};
use crate::errors::AppError;
use crate::logging::init_logging;
use crate::models::DnsMode;
use crate::network::{NetworkBackend, PowerShellNetworkBackend};

pub const APP_LOCAL_DATA_DIR_NAME: &str = "com.dnschanger.desktop";
pub const ELEVATED_HELPER_TIMEOUT_MS: u32 = 120_000;
const ELEVATED_CLI_FLAG: &str = "--elevated-dns-op";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElevatedCommand {
    SetIpv4Dns {
        adapter_guid: String,
        servers: Vec<Ipv4Addr>,
    },
    ResetIpv4Dns {
        adapter_guid: String,
    },
    SetIpv4DnsWithRollback {
        adapter_guid: String,
        servers: Vec<Ipv4Addr>,
        rollback_mode: DnsMode,
        rollback_servers: Vec<Ipv4Addr>,
    },
}

pub trait ElevationProvider: Send + Sync {
    fn is_elevated(&self) -> bool;
    fn can_request_elevation(&self) -> bool;
    fn run_elevated_set_dns(
        &self,
        adapter_guid: &str,
        servers: &[Ipv4Addr],
    ) -> Result<(), AppError>;
    fn run_elevated_set_dns_with_rollback(
        &self,
        adapter_guid: &str,
        servers: &[Ipv4Addr],
        rollback_mode: DnsMode,
        rollback_servers: &[Ipv4Addr],
    ) -> Result<(), AppError>;
    fn run_elevated_reset_dns(&self, adapter_guid: &str) -> Result<(), AppError>;
}

pub struct RealWindowsElevation;

impl RealWindowsElevation {
    pub fn new() -> Self {
        Self
    }

    pub fn current_platform() -> &'static str {
        if cfg!(windows) {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        }
    }

    #[cfg(windows)]
    pub fn is_process_elevated() -> bool {
        use std::mem::size_of;
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
        use windows_sys::Win32::Security::{
            GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
        };
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

        unsafe {
            let mut token: HANDLE = std::ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                return false;
            }
            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut return_len: u32 = 0;
            let ok = GetTokenInformation(
                token,
                TokenElevation,
                &mut elevation as *mut _ as *mut _,
                size_of::<TOKEN_ELEVATION>() as u32,
                &mut return_len,
            );
            CloseHandle(token);
            ok != 0 && elevation.TokenIsElevated != 0
        }
    }

    #[cfg(not(windows))]
    pub fn is_process_elevated() -> bool {
        false
    }

    fn elevated_timeout_error(command: &ElevatedCommand) -> AppError {
        let timeout_secs = u64::from(ELEVATED_HELPER_TIMEOUT_MS / 1000);
        match command {
            ElevatedCommand::SetIpv4DnsWithRollback { .. }
            | ElevatedCommand::SetIpv4Dns { .. } => AppError::RollbackNotPerformed {
                detail: format!(
                    "Elevated helper timed out after {timeout_secs}s and was terminated; DNS state is unknown and automatic rollback may not have run"
                ),
            },
            ElevatedCommand::ResetIpv4Dns { .. } => AppError::PowershellTimeout {
                timeout_secs,
                detail: format!(
                    "Elevated reset helper timed out after {timeout_secs}s and was terminated"
                ),
            },
        }
    }

    fn execute_via_uac(&self, command: &ElevatedCommand) -> Result<(), AppError> {
        #[cfg(not(windows))]
        {
            let _ = command;
            return Err(AppError::UnsupportedPlatform);
        }

        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Foundation::{
                CloseHandle, GetLastError, ERROR_CANCELLED, RPC_E_CHANGED_MODE, S_FALSE, S_OK,
                WAIT_OBJECT_0,
            };
            use windows_sys::Win32::System::Com::{
                CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
            };
            use windows_sys::Win32::System::Threading::{
                GetExitCodeProcess, TerminateProcess, WaitForSingleObject,
            };
            use windows_sys::Win32::UI::Shell::{
                ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
            };
            use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

            struct ComApartmentGuard {
                should_uninitialize: bool,
            }

            impl Drop for ComApartmentGuard {
                fn drop(&mut self) {
                    if self.should_uninitialize {
                        unsafe {
                            CoUninitialize();
                        }
                    }
                }
            }

            let hr = unsafe {
                CoInitializeEx(
                    std::ptr::null(),
                    (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
                )
            };
            let _com_guard = if hr == S_OK || hr == S_FALSE {
                ComApartmentGuard {
                    should_uninitialize: true,
                }
            } else if hr == RPC_E_CHANGED_MODE {
                ComApartmentGuard {
                    should_uninitialize: false,
                }
            } else {
                return Err(AppError::ElevationFailed {
                    detail: format!("CoInitializeEx failed with HRESULT {hr:#x}"),
                });
            };

            let exe_path = std::env::current_exe().map_err(|e| AppError::ElevationFailed {
                detail: format!("Failed to resolve current executable path: {e}"),
            })?;

            let cli_args = build_elevated_cli_args(command)?;
            let params = cli_args.join(" ");

            let verb_w: Vec<u16> = std::ffi::OsStr::new("runas")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let file_w: Vec<u16> = exe_path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let params_w: Vec<u16> = std::ffi::OsStr::new(&params)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let mut sei: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
            sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            sei.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
            sei.lpVerb = verb_w.as_ptr();
            sei.lpFile = file_w.as_ptr();
            sei.lpParameters = params_w.as_ptr();
            sei.nShow = SW_HIDE as i32;

            info!("Requesting Windows UAC elevation for controlled DNS helper");

            let launched = unsafe { ShellExecuteExW(&mut sei) };
            if launched == 0 {
                let last_err = unsafe { GetLastError() };
                if last_err == ERROR_CANCELLED {
                    warn!("User cancelled Windows UAC prompt");
                    return Err(AppError::UacCancelled);
                }
                return Err(AppError::ElevationFailed {
                    detail: format!("ShellExecuteExW failed with Win32 error {last_err}"),
                });
            }

            if sei.hProcess.is_null() {
                return Err(AppError::ElevationFailed {
                    detail: "Elevated helper process handle was null".into(),
                });
            }

            let wait_res = unsafe { WaitForSingleObject(sei.hProcess, ELEVATED_HELPER_TIMEOUT_MS) };
            let mut exit_code: u32 = 1;
            if wait_res == WAIT_OBJECT_0 {
                unsafe {
                    GetExitCodeProcess(sei.hProcess, &mut exit_code);
                    CloseHandle(sei.hProcess);
                }
            } else {
                warn!(
                    timeout_ms = ELEVATED_HELPER_TIMEOUT_MS,
                    "Elevated helper timed out; terminating helper process"
                );
                unsafe {
                    TerminateProcess(sei.hProcess, 1);
                    CloseHandle(sei.hProcess);
                }
                return Err(Self::elevated_timeout_error(command));
            }

            exit_code_to_app_error(exit_code)
        }
    }
}

impl Default for RealWindowsElevation {
    fn default() -> Self {
        Self::new()
    }
}

impl ElevationProvider for RealWindowsElevation {
    fn is_elevated(&self) -> bool {
        Self::is_process_elevated()
    }

    fn can_request_elevation(&self) -> bool {
        cfg!(windows)
    }

    fn run_elevated_set_dns(
        &self,
        adapter_guid: &str,
        servers: &[Ipv4Addr],
    ) -> Result<(), AppError> {
        let canonical = validate_adapter_guid(adapter_guid)?;
        let validated = validate_dns_list(
            &servers
                .iter()
                .map(|ip| ip.to_string())
                .collect::<Vec<_>>(),
        )?;
        let cmd = ElevatedCommand::SetIpv4Dns {
            adapter_guid: canonical,
            servers: validated,
        };
        self.execute_via_uac(&cmd)
    }

    fn run_elevated_set_dns_with_rollback(
        &self,
        adapter_guid: &str,
        servers: &[Ipv4Addr],
        rollback_mode: DnsMode,
        rollback_servers: &[Ipv4Addr],
    ) -> Result<(), AppError> {
        let canonical = validate_adapter_guid(adapter_guid)?;
        let validated_targets = validate_dns_list(
            &servers
                .iter()
                .map(|ip| ip.to_string())
                .collect::<Vec<_>>(),
        )?;
        let validated_rollback = match rollback_mode {
            DnsMode::Automatic => Vec::new(),
            DnsMode::Static => validate_dns_list(
                &rollback_servers
                    .iter()
                    .map(|ip| ip.to_string())
                    .collect::<Vec<_>>(),
            )?,
        };
        let cmd = ElevatedCommand::SetIpv4DnsWithRollback {
            adapter_guid: canonical,
            servers: validated_targets,
            rollback_mode,
            rollback_servers: validated_rollback,
        };
        self.execute_via_uac(&cmd)
    }

    fn run_elevated_reset_dns(&self, adapter_guid: &str) -> Result<(), AppError> {
        let canonical = validate_adapter_guid(adapter_guid)?;
        let cmd = ElevatedCommand::ResetIpv4Dns {
            adapter_guid: canonical,
        };
        self.execute_via_uac(&cmd)
    }
}

pub fn build_elevated_cli_args(command: &ElevatedCommand) -> Result<Vec<String>, AppError> {
    match command {
        ElevatedCommand::SetIpv4Dns {
            adapter_guid,
            servers,
        } => {
            let guid = validate_adapter_guid(adapter_guid)?;
            let checked = validate_dns_list(
                &servers
                    .iter()
                    .map(|ip| ip.to_string())
                    .collect::<Vec<_>>(),
            )?;
            let mut args = vec![ELEVATED_CLI_FLAG.to_string(), "set".to_string(), guid];
            args.extend(checked.into_iter().map(|ip| ip.to_string()));
            Ok(args)
        }
        ElevatedCommand::ResetIpv4Dns { adapter_guid } => {
            let guid = validate_adapter_guid(adapter_guid)?;
            Ok(vec![
                ELEVATED_CLI_FLAG.to_string(),
                "reset".to_string(),
                guid,
            ])
        }
        ElevatedCommand::SetIpv4DnsWithRollback {
            adapter_guid,
            servers,
            rollback_mode,
            rollback_servers,
        } => {
            let guid = validate_adapter_guid(adapter_guid)?;
            let checked_targets = validate_dns_list(
                &servers
                    .iter()
                    .map(|ip| ip.to_string())
                    .collect::<Vec<_>>(),
            )?;
            let target_csv = checked_targets
                .into_iter()
                .map(|ip| ip.to_string())
                .collect::<Vec<_>>()
                .join(",");

            let mut args = vec![
                ELEVATED_CLI_FLAG.to_string(),
                "set-with-rollback".to_string(),
                guid,
                target_csv,
            ];
            match rollback_mode {
                DnsMode::Automatic => args.push("automatic".to_string()),
                DnsMode::Static => {
                    let checked_rb = validate_dns_list(
                        &rollback_servers
                            .iter()
                            .map(|ip| ip.to_string())
                            .collect::<Vec<_>>(),
                    )?;
                    let rb_csv = checked_rb
                        .into_iter()
                        .map(|ip| ip.to_string())
                        .collect::<Vec<_>>()
                        .join(",");
                    args.push("static".to_string());
                    args.push(rb_csv);
                }
            }
            Ok(args)
        }
    }
}

pub fn parse_elevated_cli_args(args: &[String]) -> Result<ElevatedCommand, AppError> {
    if args.get(1).map(|s| s.as_str()) != Some(ELEVATED_CLI_FLAG) {
        return Err(AppError::ElevationFailed {
            detail: "Missing elevated CLI operation flag".into(),
        });
    }

    let op = args.get(2).map(|s| s.as_str()).ok_or_else(|| {
        AppError::ElevationFailed {
            detail: "Missing elevated operation verb".into(),
        }
    })?;

    match op {
        "set" => {
            let raw_guid = args.get(3).ok_or_else(|| AppError::ElevationFailed {
                detail: "Missing adapter GUID for elevated set operation".into(),
            })?;
            let guid = validate_adapter_guid(raw_guid)?;
            let raw_servers = &args[4..];
            let servers = validate_dns_list(raw_servers)?;
            Ok(ElevatedCommand::SetIpv4Dns {
                adapter_guid: guid,
                servers,
            })
        }
        "reset" => {
            if args.len() != 4 {
                return Err(AppError::ElevationFailed {
                    detail: "Invalid argument count for elevated reset operation".into(),
                });
            }
            let guid = validate_adapter_guid(&args[3])?;
            Ok(ElevatedCommand::ResetIpv4Dns { adapter_guid: guid })
        }
        "set-with-rollback" => {
            let raw_guid = args.get(3).ok_or_else(|| AppError::ElevationFailed {
                detail: "Missing adapter GUID for elevated set-with-rollback".into(),
            })?;
            let guid = validate_adapter_guid(raw_guid)?;
            let target_csv = args.get(4).ok_or_else(|| AppError::ElevationFailed {
                detail: "Missing target DNS servers for elevated set-with-rollback".into(),
            })?;
            let target_tokens: Vec<&str> = target_csv.split(',').collect();
            let servers = validate_dns_list(&target_tokens)?;

            let mode_str = args.get(5).map(|s| s.as_str()).ok_or_else(|| {
                AppError::ElevationFailed {
                    detail: "Missing rollback DNS mode for elevated set-with-rollback".into(),
                }
            })?;

            match mode_str {
                "automatic" => {
                    if args.len() != 6 {
                        return Err(AppError::ElevationFailed {
                            detail: "Unexpected extra arguments for automatic rollback mode".into(),
                        });
                    }
                    Ok(ElevatedCommand::SetIpv4DnsWithRollback {
                        adapter_guid: guid,
                        servers,
                        rollback_mode: DnsMode::Automatic,
                        rollback_servers: Vec::new(),
                    })
                }
                "static" => {
                    if args.len() != 7 {
                        return Err(AppError::ElevationFailed {
                            detail: "Static rollback mode requires rollback server list".into(),
                        });
                    }
                    let rb_tokens: Vec<&str> = args[6].split(',').collect();
                    let rollback_servers = validate_dns_list(&rb_tokens)?;
                    Ok(ElevatedCommand::SetIpv4DnsWithRollback {
                        adapter_guid: guid,
                        servers,
                        rollback_mode: DnsMode::Static,
                        rollback_servers,
                    })
                }
                other => Err(AppError::ElevationFailed {
                    detail: format!("Unsupported rollback mode '{other}'"),
                }),
            }
        }
        other => Err(AppError::ElevationFailed {
            detail: format!("Unsupported elevated operation '{other}'"),
        }),
    }
}

pub fn app_error_to_exit_code(err: &AppError) -> i32 {
    match err {
        AppError::AdminAccessDenied { .. } => 10,
        AppError::AdapterNotFound { .. } => 11,
        AppError::InvalidDnsAddress { .. } => 12,
        AppError::InvalidAdapterGuid { .. } => 13,
        AppError::PowershellTimeout { .. } => 14,
        AppError::PowershellExecutionFailed { .. } => 15,
        AppError::DnsApplyFailed { .. } => 16,
        AppError::VerificationFailed { .. } => 17,
        AppError::DnsModeIndeterminate { .. } => 18,
        AppError::Ipv6ModificationDetected { .. } => 19,
        AppError::NetworkChanged { .. } => 20,
        AppError::RollbackNotPerformed { .. } => 21,
        AppError::Ipv6RecoveryFailed { .. } => 23,
        AppError::PowershellBlocked { .. } => 24,
        _ => 22,
    }
}

pub fn exit_code_to_app_error(code: u32) -> Result<(), AppError> {
    match code {
        0 => Ok(()),
        2 => Err(AppError::ElevationFailed {
            detail: "Elevated helper rejected invalid CLI arguments".into(),
        }),
        10 => Err(AppError::AdminAccessDenied {
            detail: "Elevated helper lacked Administrator privileges".into(),
        }),
        11 => Err(AppError::AdapterNotFound {
            guid: String::new(),
        }),
        12 => Err(AppError::InvalidDnsAddress {
            address: String::new(),
            reason: "rejected by elevated helper".into(),
        }),
        13 => Err(AppError::InvalidAdapterGuid {
            guid: String::new(),
        }),
        14 => Err(AppError::PowershellTimeout {
            timeout_secs: 15,
            detail: "PowerShell command timed out inside elevated helper".into(),
        }),
        15 => Err(AppError::PowershellExecutionFailed {
            detail: "PowerShell command failed inside elevated helper".into(),
        }),
        16 => Err(AppError::DnsApplyFailed {
            detail: "Elevated helper failed to apply IPv4 DNS configuration".into(),
        }),
        17 => Err(AppError::VerificationFailed {
            detail: "Post-apply verification failed inside elevated helper; original DNS rolled back".into(),
        }),
        18 => Err(AppError::DnsModeIndeterminate {
            guid: String::new(),
            detail: "Adapter IPv4 DNS mode was indeterminate after apply; original DNS rolled back".into(),
        }),
        19 => Err(AppError::Ipv6ModificationDetected {
            detail: "Unexpected IPv6 DNS modification detected inside elevated helper; recovery attempted".into(),
        }),
        20 => Err(AppError::NetworkChanged {
            detail: "Network adapter disappeared during elevated DNS operation; rollback attempted".into(),
        }),
        21 => Err(AppError::RollbackNotPerformed {
            detail: "DNS changed inside elevated helper, but automatic rollback could not complete".into(),
        }),
        23 => Err(AppError::Ipv6RecoveryFailed {
            detail: "Unexpected IPv6 DNS modification detected inside elevated helper; IPv4 rolled back, but automatic IPv6 recovery failed".into(),
        }),
        24 => Err(AppError::PowershellBlocked {
            detail: "PowerShell execution was blocked inside elevated helper".into(),
        }),
        other => Err(AppError::ElevationFailed {
            detail: format!("Elevated helper exited with unexpected status code {other}"),
        }),
    }
}

fn resolve_helper_log_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(local) = std::env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty()) {
            return PathBuf::from(local)
                .join(APP_LOCAL_DATA_DIR_NAME)
                .join("logs");
        }
    }
    #[cfg(not(windows))]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
            return PathBuf::from(xdg)
                .join(APP_LOCAL_DATA_DIR_NAME)
                .join("logs");
        }
        if let Some(home) = std::env::var_os("HOME").filter(|v| !v.is_empty()) {
            return PathBuf::from(home)
                .join(".local")
                .join("share")
                .join(APP_LOCAL_DATA_DIR_NAME)
                .join("logs");
        }
    }
    std::env::temp_dir()
        .join(APP_LOCAL_DATA_DIR_NAME)
        .join("logs")
}

fn init_helper_logging() -> Option<WorkerGuard> {
    let log_dir = resolve_helper_log_dir();
    init_logging(&log_dir)
}

/// Entrypoint for `--elevated-dns-op <op> ...` helper invocation.
/// Returns `Some(exit_code)` if CLI arguments requested elevated helper execution,
/// or `None` if normal GUI startup should proceed.
pub fn handle_elevated_cli_if_present() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) != Some(ELEVATED_CLI_FLAG) {
        return None;
    }

    let _log_guard = init_helper_logging();
    Some(handle_elevated_cli_args(&args))
}

pub fn handle_elevated_cli_args(args: &[String]) -> i32 {
    let command = match parse_elevated_cli_args(args) {
        Ok(cmd) => cmd,
        Err(err) => {
            error!(error_code = ?err.code(), detail = %err, "Rejected invalid elevated CLI arguments");
            return 2;
        }
    };

    if !RealWindowsElevation::is_process_elevated() {
        let err = AppError::AdminAccessDenied {
            detail: "Helper process is not running with elevated Administrator token".into(),
        };
        error!(error_code = ?err.code(), detail = %err, "Elevated helper invoked without Administrator token");
        return app_error_to_exit_code(&err);
    }

    let backend = PowerShellNetworkBackend::new();
    match execute_elevated_command(&backend, &command) {
        Ok(()) => 0,
        Err(err) => {
            error!(error_code = ?err.code(), detail = %err, "Elevated DNS helper operation failed");
            app_error_to_exit_code(&err)
        }
    }
}

pub fn execute_elevated_command(
    backend: &dyn NetworkBackend,
    command: &ElevatedCommand,
) -> Result<(), AppError> {
    match command {
        ElevatedCommand::SetIpv4Dns {
            adapter_guid,
            servers,
        } => {
            let guid = validate_adapter_guid(adapter_guid)?;
            let validated_ips: Vec<Ipv4Addr> = servers
                .iter()
                .map(|ip| validate_ipv4(&ip.to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            backend.set_ipv4_dns(&guid, &validated_ips)
        }
        ElevatedCommand::ResetIpv4Dns { adapter_guid } => {
            let guid = validate_adapter_guid(adapter_guid)?;
            backend.reset_ipv4_dns_to_automatic(&guid)
        }
        ElevatedCommand::SetIpv4DnsWithRollback {
            adapter_guid,
            servers,
            rollback_mode,
            rollback_servers,
        } => {
            let guid = validate_adapter_guid(adapter_guid)?;
            let validated_targets: Vec<Ipv4Addr> = servers
                .iter()
                .map(|ip| validate_ipv4(&ip.to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            let expected_servers: Vec<String> =
                validated_targets.iter().map(|ip| ip.to_string()).collect();

            let before_ipv6 = backend.get_ipv6_dns_state(&guid)?;
            backend.set_ipv4_dns(&guid, &validated_targets)?;

            let rollback_ipv4 = |backend: &dyn NetworkBackend| -> Result<(), AppError> {
                match rollback_mode {
                    DnsMode::Automatic => backend.reset_ipv4_dns_to_automatic(&guid),
                    DnsMode::Static => {
                        let validated_rb: Vec<Ipv4Addr> = rollback_servers
                            .iter()
                            .map(|ip| validate_ipv4(&ip.to_string()))
                            .collect::<Result<Vec<_>, _>>()?;
                        backend.set_ipv4_dns(&guid, &validated_rb)
                    }
                }
            };

            let after_ipv6 = match backend.get_ipv6_dns_state(&guid) {
                Ok(state) => state,
                Err(e) => {
                    if let Err(rb_err) = rollback_ipv4(backend) {
                        return Err(AppError::RollbackNotPerformed {
                            detail: format!("Post-apply IPv6 read failed ({e}) and rollback failed ({rb_err})"),
                        });
                    }
                    return Err(e);
                }
            };

            let ipv6_preserved = match (&before_ipv6, &after_ipv6) {
                (Some(b), Some(a)) => b.matches(a),
                (None, None) => true,
                _ => false,
            };

            if !ipv6_preserved {
                let ipv6_recovery_res = if let Some(orig_v6) = &before_ipv6 {
                    backend.restore_ipv6_dns_state(&guid, orig_v6)
                } else {
                    Ok(())
                };
                if let Err(ref v6_err) = ipv6_recovery_res {
                    error!(
                        adapter_guid = %guid,
                        error = %v6_err,
                        "Automatic IPv6 recovery failed inside elevated helper"
                    );
                }
                if let Err(rb_err) = rollback_ipv4(backend) {
                    let detail = match ipv6_recovery_res {
                        Ok(()) => {
                            format!("IPv6 changed unexpectedly and IPv4 rollback failed: {rb_err}")
                        }
                        Err(v6_err) => format!(
                            "IPv6 changed unexpectedly, IPv6 recovery failed ({v6_err}), and IPv4 rollback failed ({rb_err})"
                        ),
                    };
                    return Err(AppError::RollbackNotPerformed { detail });
                }
                if let Err(v6_err) = ipv6_recovery_res {
                    return Err(AppError::Ipv6RecoveryFailed {
                        detail: format!(
                            "IPv6 DNS changed unexpectedly during elevated IPv4 apply; IPv4 rolled back, but IPv6 recovery failed: {v6_err}"
                        ),
                    });
                }
                return Err(AppError::Ipv6ModificationDetected {
                    detail: "IPv6 DNS changed unexpectedly during elevated IPv4 apply".into(),
                });
            }

            let fresh = match backend.get_adapter_by_guid(&guid) {
                Ok(Some(adapter)) => adapter,
                Ok(None) => {
                    if let Err(rb_err) = rollback_ipv4(backend) {
                        return Err(AppError::RollbackNotPerformed {
                            detail: format!("Adapter disappeared after apply and rollback failed: {rb_err}"),
                        });
                    }
                    return Err(AppError::NetworkChanged {
                        detail: format!("Adapter {guid} disappeared after elevated DNS change"),
                    });
                }
                Err(e) => {
                    if let Err(rb_err) = rollback_ipv4(backend) {
                        return Err(AppError::RollbackNotPerformed {
                            detail: format!("Post-apply adapter read failed ({e}) and rollback failed ({rb_err})"),
                        });
                    }
                    return Err(e);
                }
            };

            if let Some(reason) = fresh.dns_mode_indeterminate_reason {
                if let Err(rb_err) = rollback_ipv4(backend) {
                    return Err(AppError::RollbackNotPerformed {
                        detail: format!("Adapter DNS mode became indeterminate ({reason}) and rollback failed: {rb_err}"),
                    });
                }
                return Err(AppError::DnsModeIndeterminate {
                    guid,
                    detail: reason,
                });
            }

            if fresh.dns_mode != DnsMode::Static
                || fresh.current_ipv4_dns_servers != expected_servers
            {
                if let Err(rb_err) = rollback_ipv4(backend) {
                    return Err(AppError::RollbackNotPerformed {
                        detail: format!(
                            "Post-apply verification failed (expected {expected_servers:?}, got {:?}) and rollback failed: {rb_err}",
                            fresh.current_ipv4_dns_servers
                        ),
                    });
                }
                return Err(AppError::VerificationFailed {
                    detail: format!(
                        "Expected static DNS {:?}, but Windows reported mode {:?} with servers {:?}",
                        expected_servers, fresh.dns_mode, fresh.current_ipv4_dns_servers
                    ),
                });
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AdapterStatus, AdapterType, DhcpStatus, Ipv6DnsSnapshot, NetworkAdapter};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex as StdMutex};

    #[test]
    fn parses_and_validates_set_and_reset_cli_arguments_including_three_plus_servers() {
        let guid = "11111111-2222-3333-4444-555555555555";
        let set_args = vec![
            "dns-changer.exe".into(),
            ELEVATED_CLI_FLAG.into(),
            "set".into(),
            guid.into(),
            "1.1.1.1".into(),
            "1.0.0.1".into(),
            "8.8.8.8".into(),
            "9.9.9.9".into(),
        ];
        let parsed = parse_elevated_cli_args(&set_args).unwrap();
        assert_eq!(
            parsed,
            ElevatedCommand::SetIpv4Dns {
                adapter_guid: "{11111111-2222-3333-4444-555555555555}".into(),
                servers: vec![
                    Ipv4Addr::new(1, 1, 1, 1),
                    Ipv4Addr::new(1, 0, 0, 1),
                    Ipv4Addr::new(8, 8, 8, 8),
                    Ipv4Addr::new(9, 9, 9, 9),
                ],
            }
        );

        let rebuilt = build_elevated_cli_args(&parsed).unwrap();
        assert_eq!(rebuilt[0], ELEVATED_CLI_FLAG);
        assert_eq!(rebuilt[1], "set");
        assert_eq!(rebuilt.len(), 7);

        let reset_args = vec![
            "dns-changer.exe".into(),
            ELEVATED_CLI_FLAG.into(),
            "reset".into(),
            guid.into(),
        ];
        assert_eq!(
            parse_elevated_cli_args(&reset_args).unwrap(),
            ElevatedCommand::ResetIpv4Dns {
                adapter_guid: "{11111111-2222-3333-4444-555555555555}".into(),
            }
        );
    }

    #[test]
    fn parses_set_with_rollback_cli_arguments() {
        let cmd = ElevatedCommand::SetIpv4DnsWithRollback {
            adapter_guid: "{11111111-2222-3333-4444-555555555555}".into(),
            servers: vec![Ipv4Addr::new(1, 1, 1, 1), Ipv4Addr::new(1, 0, 0, 1)],
            rollback_mode: DnsMode::Static,
            rollback_servers: vec![
                Ipv4Addr::new(8, 8, 8, 8),
                Ipv4Addr::new(8, 8, 4, 4),
                Ipv4Addr::new(9, 9, 9, 9),
            ],
        };

        let mut argv = vec!["dns-changer.exe".to_string()];
        argv.extend(build_elevated_cli_args(&cmd).unwrap());
        assert_eq!(parse_elevated_cli_args(&argv).unwrap(), cmd);

        let auto_cmd = ElevatedCommand::SetIpv4DnsWithRollback {
            adapter_guid: "{11111111-2222-3333-4444-555555555555}".into(),
            servers: vec![Ipv4Addr::new(1, 1, 1, 1)],
            rollback_mode: DnsMode::Automatic,
            rollback_servers: Vec::new(),
        };
        let mut auto_argv = vec!["dns-changer.exe".to_string()];
        auto_argv.extend(build_elevated_cli_args(&auto_cmd).unwrap());
        assert_eq!(parse_elevated_cli_args(&auto_argv).unwrap(), auto_cmd);
    }

    #[test]
    fn rejects_invalid_cli_arguments_guid_ips_and_too_many_servers() {
        // Invalid GUID
        assert!(parse_elevated_cli_args(&[
            "exe".into(),
            ELEVATED_CLI_FLAG.into(),
            "reset".into(),
            "bad-guid".into(),
        ])
        .is_err());

        // No DNS servers
        assert!(parse_elevated_cli_args(&[
            "exe".into(),
            ELEVATED_CLI_FLAG.into(),
            "set".into(),
            "{11111111-2222-3333-4444-555555555555}".into(),
        ])
        .is_err());

        // Injection / invalid IPv4
        assert!(parse_elevated_cli_args(&[
            "exe".into(),
            ELEVATED_CLI_FLAG.into(),
            "set".into(),
            "{11111111-2222-3333-4444-555555555555}".into(),
            "1.1.1.1;whoami".into(),
        ])
        .is_err());

        // 9 servers rejected
        assert!(parse_elevated_cli_args(&[
            "exe".into(),
            ELEVATED_CLI_FLAG.into(),
            "set".into(),
            "{11111111-2222-3333-4444-555555555555}".into(),
            "1.1.1.1".into(),
            "1.0.0.1".into(),
            "8.8.8.8".into(),
            "8.8.4.4".into(),
            "9.9.9.9".into(),
            "149.112.112.112".into(),
            "208.67.222.222".into(),
            "208.67.220.220".into(),
            "94.140.14.14".into(),
        ])
        .is_err());

        // Unknown op
        assert!(parse_elevated_cli_args(&[
            "exe".into(),
            ELEVATED_CLI_FLAG.into(),
            "delete-all".into(),
            "{11111111-2222-3333-4444-555555555555}".into(),
        ])
        .is_err());
    }

    #[test]
    fn helper_cli_fails_closed_when_not_elevated_and_maps_exit_codes() {
        let args = vec![
            "dns-changer.exe".into(),
            ELEVATED_CLI_FLAG.into(),
            "set".into(),
            "{11111111-2222-3333-4444-555555555555}".into(),
            "1.1.1.1".into(),
            "1.0.0.1".into(),
        ];

        let exit = handle_elevated_cli_args(&args);
        assert_eq!(exit, 10);
        assert!(matches!(
            exit_code_to_app_error(exit as u32),
            Err(AppError::AdminAccessDenied { .. })
        ));

        let invalid_exit = handle_elevated_cli_args(&[
            "dns-changer.exe".into(),
            ELEVATED_CLI_FLAG.into(),
            "set".into(),
            "invalid".into(),
        ]);
        assert_eq!(invalid_exit, 2);
        assert!(matches!(
            exit_code_to_app_error(invalid_exit as u32),
            Err(AppError::ElevationFailed { .. })
        ));

        assert!(exit_code_to_app_error(0).is_ok());
        assert!(matches!(
            exit_code_to_app_error(21),
            Err(AppError::RollbackNotPerformed { .. })
        ));
        let v6_rec_code = app_error_to_exit_code(&AppError::Ipv6RecoveryFailed {
            detail: "failed".into(),
        });
        assert_eq!(v6_rec_code, 23);
        assert!(matches!(
            exit_code_to_app_error(23),
            Err(AppError::Ipv6RecoveryFailed { .. })
        ));

        let ps_blocked_code = app_error_to_exit_code(&AppError::PowershellBlocked {
            detail: "blocked".into(),
        });
        assert_eq!(ps_blocked_code, 24);
        assert!(matches!(
            exit_code_to_app_error(24),
            Err(AppError::PowershellBlocked { .. })
        ));
    }

    #[test]
    fn elevated_timeout_error_depends_on_operation_and_uses_120s_constant() {
        assert_eq!(ELEVATED_HELPER_TIMEOUT_MS, 120_000);
        let guid = "{11111111-2222-3333-4444-555555555555}".to_string();

        let set_rb_err = RealWindowsElevation::elevated_timeout_error(
            &ElevatedCommand::SetIpv4DnsWithRollback {
                adapter_guid: guid.clone(),
                servers: vec![Ipv4Addr::new(1, 1, 1, 1)],
                rollback_mode: DnsMode::Automatic,
                rollback_servers: Vec::new(),
            },
        );
        assert!(matches!(set_rb_err, AppError::RollbackNotPerformed { .. }));
        assert!(set_rb_err.to_string().contains("terminated"));

        let set_err = RealWindowsElevation::elevated_timeout_error(&ElevatedCommand::SetIpv4Dns {
            adapter_guid: guid.clone(),
            servers: vec![Ipv4Addr::new(1, 1, 1, 1)],
        });
        assert!(matches!(set_err, AppError::RollbackNotPerformed { .. }));

        let reset_err =
            RealWindowsElevation::elevated_timeout_error(&ElevatedCommand::ResetIpv4Dns {
                adapter_guid: guid,
            });
        assert!(matches!(
            reset_err,
            AppError::PowershellTimeout {
                timeout_secs: 120,
                ..
            }
        ));
    }

    struct HelperMockBackend {
        adapters: Arc<StdMutex<HashMap<String, NetworkAdapter>>>,
        ipv6: Arc<StdMutex<Option<Ipv6DnsSnapshot>>>,
        stale_apply: bool,
        fail_rollback: bool,
        corrupt_ipv6_on_set: bool,
        fail_ipv6_restore: bool,
        rollback_calls: Arc<StdMutex<usize>>,
    }

    impl NetworkBackend for HelperMockBackend {
        fn list_adapters(&self) -> Result<Vec<NetworkAdapter>, AppError> {
            Ok(self.adapters.lock().unwrap().values().cloned().collect())
        }

        fn get_adapter_by_guid(&self, adapter_guid: &str) -> Result<Option<NetworkAdapter>, AppError> {
            Ok(self.adapters.lock().unwrap().get(adapter_guid).cloned())
        }

        fn set_ipv4_dns(&self, adapter_guid: &str, servers: &[Ipv4Addr]) -> Result<(), AppError> {
            let mut map = self.adapters.lock().unwrap();
            let ad = map.get_mut(adapter_guid).unwrap();
            if !self.stale_apply {
                ad.dns_mode = DnsMode::Static;
                ad.current_ipv4_dns_servers = servers.iter().map(|ip| ip.to_string()).collect();
            }
            if self.corrupt_ipv6_on_set {
                *self.ipv6.lock().unwrap() = Some(Ipv6DnsSnapshot {
                    server_addresses: vec![],
                    static_name_server_reg: None,
                    profile_name_server_reg: None,
                    dns_mode: DnsMode::Automatic,
                });
            }
            Ok(())
        }

        fn reset_ipv4_dns_to_automatic(&self, adapter_guid: &str) -> Result<(), AppError> {
            *self.rollback_calls.lock().unwrap() += 1;
            if self.fail_rollback {
                return Err(AppError::DnsApplyFailed {
                    detail: "simulated rollback error".into(),
                });
            }
            let mut map = self.adapters.lock().unwrap();
            let ad = map.get_mut(adapter_guid).unwrap();
            ad.dns_mode = DnsMode::Automatic;
            ad.current_ipv4_dns_servers = vec!["192.168.1.1".into()];
            Ok(())
        }

        fn get_ipv6_dns_state(&self, _guid: &str) -> Result<Option<Ipv6DnsSnapshot>, AppError> {
            Ok(self.ipv6.lock().unwrap().clone())
        }

        fn restore_ipv6_dns_state(
            &self,
            _guid: &str,
            snapshot: &Ipv6DnsSnapshot,
        ) -> Result<(), AppError> {
            if self.fail_ipv6_restore {
                return Err(AppError::PowershellExecutionFailed {
                    detail: "simulated ipv6 restore failure".into(),
                });
            }
            *self.ipv6.lock().unwrap() = Some(snapshot.clone());
            Ok(())
        }
    }

    #[test]
    fn elevated_set_with_rollback_handles_verification_and_rollback_failures() {
        let guid = "{11111111-2222-3333-4444-555555555555}".to_string();
        let adapter = NetworkAdapter {
            guid: guid.clone(),
            interface_index: 5,
            name: "Ethernet".into(),
            interface_description: "Mock".into(),
            adapter_type: AdapterType::Ethernet,
            operational_status: AdapterStatus::Up,
            ipv4_addresses: vec!["192.168.1.2".into()],
            ipv4_gateway: Some("192.168.1.1".into()),
            current_ipv4_dns_servers: vec!["192.168.1.1".into()],
            dns_mode: DnsMode::Automatic,
            dhcp_status: DhcpStatus::Enabled,
            mac_address: None,
            link_speed: None,
            dns_mode_indeterminate_reason: None,
            static_ipv4_dns_reg_servers: Vec::new(),
        };

        let mut map = HashMap::new();
        map.insert(guid.clone(), adapter.clone());
        let rollback_calls = Arc::new(StdMutex::new(0));
        let backend_ok_rb = HelperMockBackend {
            adapters: Arc::new(StdMutex::new(map)),
            ipv6: Arc::new(StdMutex::new(None)),
            stale_apply: true,
            fail_rollback: false,
            corrupt_ipv6_on_set: false,
            fail_ipv6_restore: false,
            rollback_calls: rollback_calls.clone(),
        };

        let cmd = ElevatedCommand::SetIpv4DnsWithRollback {
            adapter_guid: guid.clone(),
            servers: vec![Ipv4Addr::new(1, 1, 1, 1)],
            rollback_mode: DnsMode::Automatic,
            rollback_servers: Vec::new(),
        };
        let err = execute_elevated_command(&backend_ok_rb, &cmd).unwrap_err();
        assert!(matches!(err, AppError::VerificationFailed { .. }));
        assert_eq!(*rollback_calls.lock().unwrap(), 1);

        let mut map2 = HashMap::new();
        map2.insert(guid.clone(), adapter.clone());
        let backend_fail_rb = HelperMockBackend {
            adapters: Arc::new(StdMutex::new(map2)),
            ipv6: Arc::new(StdMutex::new(None)),
            stale_apply: true,
            fail_rollback: true,
            corrupt_ipv6_on_set: false,
            fail_ipv6_restore: false,
            rollback_calls: Arc::new(StdMutex::new(0)),
        };
        let err2 = execute_elevated_command(&backend_fail_rb, &cmd).unwrap_err();
        assert!(matches!(err2, AppError::RollbackNotPerformed { .. }));

        // Case 3: IPv6 changes unexpectedly, IPv4 rollback succeeds, IPv6 restore fails -> Ipv6RecoveryFailed
        let mut map3 = HashMap::new();
        map3.insert(guid.clone(), adapter);
        let rollback_calls_v6 = Arc::new(StdMutex::new(0));
        let backend_fail_v6_restore = HelperMockBackend {
            adapters: Arc::new(StdMutex::new(map3)),
            ipv6: Arc::new(StdMutex::new(Some(Ipv6DnsSnapshot {
                server_addresses: vec!["2001:4860:4860::8888".into()],
                static_name_server_reg: Some("2001:4860:4860::8888".into()),
                profile_name_server_reg: None,
                dns_mode: DnsMode::Static,
            }))),
            stale_apply: false,
            fail_rollback: false,
            corrupt_ipv6_on_set: true,
            fail_ipv6_restore: true,
            rollback_calls: rollback_calls_v6.clone(),
        };
        let err3 = execute_elevated_command(&backend_fail_v6_restore, &cmd).unwrap_err();
        assert!(matches!(err3, AppError::Ipv6RecoveryFailed { .. }));
        assert_eq!(*rollback_calls_v6.lock().unwrap(), 1);
    }
}
