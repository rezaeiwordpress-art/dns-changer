use serde::{Deserialize, Deserializer};
use std::net::Ipv6Addr;
use tracing::warn;

use crate::dns::validation::validate_adapter_guid;
use crate::errors::AppError;
use crate::models::{
    AdapterStatus, AdapterType, DhcpStatus, DnsMode, Ipv6DnsSnapshot, NetworkAdapter,
};

fn deserialize_null_default_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

fn deserialize_null_default_u32<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<u32>::deserialize(deserializer)?.unwrap_or_default())
}

fn extract_ps_string_list(value: serde_json::Value) -> Result<Vec<String>, String> {
    match value {
        serde_json::Value::Null => Ok(Vec::new()),
        serde_json::Value::String(s) => {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                Ok(Vec::new())
            } else {
                Ok(vec![trimmed])
            }
        }
        serde_json::Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    serde_json::Value::Null => {}
                    serde_json::Value::String(s) => {
                        let trimmed = s.trim().to_string();
                        if !trimmed.is_empty() {
                            out.push(trimmed);
                        }
                    }
                    other => {
                        return Err(format!(
                            "expected string element inside PowerShell string list, got {other}"
                        ));
                    }
                }
            }
            Ok(out)
        }
        serde_json::Value::Object(mut map) => {
            // Windows PowerShell 5.1 ConvertTo-Json serializes PSObject-wrapped arrays
            // as `{ "value": [...], "Count": N }` or `{ "Value": ... }`.
            if let Some(inner) = map.remove("value").or_else(|| map.remove("Value")) {
                extract_ps_string_list(inner)
            } else if map.is_empty() {
                Ok(Vec::new())
            } else {
                Err(format!(
                    "expected PowerShell string list or object with 'value' key, got keys {:?}",
                    map.keys().collect::<Vec<_>>()
                ))
            }
        }
        other => Err(format!("expected sequence, string, or null, got {other}")),
    }
}

fn deserialize_ps_string_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    extract_ps_string_list(value).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawAdapterPayload {
    #[serde(default, deserialize_with = "deserialize_null_default_string")]
    pub guid: String,
    #[serde(default, deserialize_with = "deserialize_null_default_u32")]
    pub interface_index: u32,
    #[serde(default, deserialize_with = "deserialize_null_default_string")]
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_null_default_string")]
    pub interface_description: String,
    pub physical_media_type: Option<String>,
    pub is_virtual: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_null_default_string")]
    pub operational_status: String,
    #[serde(default, deserialize_with = "deserialize_ps_string_list")]
    pub ipv4_addresses: Vec<String>,
    pub ipv4_gateway: Option<String>,
    #[serde(default, deserialize_with = "deserialize_ps_string_list")]
    pub current_ipv4_dns_servers: Vec<String>,
    pub static_name_server_reg: Option<String>,
    pub profile_name_server_reg: Option<String>,
    pub dhcp_name_server_reg: Option<String>,
    pub enable_dhcp_reg: Option<u32>,
    pub ipv4_reg_key_exists: Option<bool>,
    pub dhcp_status: Option<String>,
    pub mac_address: Option<String>,
    pub link_speed: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawIpv6DnsPayload {
    #[serde(default, deserialize_with = "deserialize_ps_string_list")]
    pub server_addresses: Vec<String>,
    pub static_name_server_reg: Option<String>,
    pub profile_name_server_reg: Option<String>,
}

fn split_dns_tokens(raw: Option<&str>) -> Vec<String> {
    match raw {
        Some(text) => text
            .split(|c: char| c == ',' || c.is_whitespace())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .collect(),
        None => Vec::new(),
    }
}

fn normalize_ipv6_list(items: &[String]) -> Vec<String> {
    items
        .iter()
        .map(|raw| {
            let trimmed = raw.trim();
            if let Ok(ip) = trimmed.parse::<Ipv6Addr>() {
                ip.to_string()
            } else {
                trimmed.to_ascii_lowercase()
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn normalize_ipv6_reg(raw: Option<&str>) -> Option<String> {
    let tokens = split_dns_tokens(raw);
    if tokens.is_empty() {
        None
    } else {
        Some(normalize_ipv6_list(&tokens).join(","))
    }
}

pub fn parse_adapter_type(
    physical_media: Option<&str>,
    is_virtual: bool,
    name: &str,
    description: &str,
) -> AdapterType {
    let media = physical_media.unwrap_or("").to_ascii_lowercase();
    let desc_lower = description.to_ascii_lowercase();
    let name_lower = name.to_ascii_lowercase();

    if media.contains("loopback")
        || desc_lower.contains("loopback")
        || name_lower.contains("loopback")
    {
        return AdapterType::Loopback;
    }

    if media.contains("802.11")
        || media.contains("wireless")
        || media.contains("native802_11")
        || name_lower.contains("wi-fi")
        || name_lower.contains("wifi")
        || desc_lower.contains("wi-fi")
        || desc_lower.contains("wireless")
    {
        return AdapterType::Wifi;
    }

    if is_virtual
        || desc_lower.contains("virtual")
        || desc_lower.contains("hyper-v")
        || desc_lower.contains("vmware")
        || desc_lower.contains("wireguard")
        || desc_lower.contains("openvpn")
        || desc_lower.contains("tap-windows")
        || desc_lower.contains("wintun")
    {
        return AdapterType::Virtual;
    }

    if media.contains("802.3")
        || desc_lower.contains("ethernet")
        || desc_lower.contains("gigabit")
    {
        return AdapterType::Ethernet;
    }

    AdapterType::Other
}

pub fn parse_adapter_status(raw: &str) -> AdapterStatus {
    match raw.trim().to_ascii_lowercase().as_str() {
        "up" => AdapterStatus::Up,
        "disconnected" | "media disconnected" => AdapterStatus::Disconnected,
        "disabled" => AdapterStatus::Disabled,
        "not present" | "notpresent" => AdapterStatus::NotPresent,
        _ => AdapterStatus::Unknown,
    }
}

pub fn parse_dhcp_status(raw: Option<&str>) -> DhcpStatus {
    match raw.unwrap_or("").trim().to_ascii_lowercase().as_str() {
        "enabled" | "true" => DhcpStatus::Enabled,
        "disabled" | "false" => DhcpStatus::Disabled,
        _ => DhcpStatus::Unknown,
    }
}

/// Cross-checks multiple Windows IPv4 network configuration signals (`NameServer`,
/// `ProfileNameServer`, `DhcpNameServer`, `EnableDHCP`, interface key existence,
/// `Get-NetIPInterface.Dhcp`, and `Get-DnsClientServerAddress`) instead of relying on
/// an empty server list, a single registry value, or DHCP IP assignment alone.
pub fn detect_ipv4_dns_mode(raw: &RawAdapterPayload) -> Result<DnsMode, String> {
    let static_ns = split_dns_tokens(raw.static_name_server_reg.as_deref());
    let profile_ns = split_dns_tokens(raw.profile_name_server_reg.as_deref());
    let dhcp_ns = split_dns_tokens(raw.dhcp_name_server_reg.as_deref());
    let active_dns: Vec<String> = raw
        .current_ipv4_dns_servers
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let iface_dhcp = parse_dhcp_status(raw.dhcp_status.as_deref());

    // Effective static registry entries (`ProfileNameServer` takes precedence over `NameServer` in Windows).
    let effective_static = if !profile_ns.is_empty() {
        Some(&profile_ns)
    } else if !static_ns.is_empty() {
        Some(&static_ns)
    } else {
        None
    };

    if let Some(static_servers) = effective_static {
        // Cross-check against active DNS servers if present: if active DNS reports pure DHCP servers
        // that completely disagree with static registry values, treat the state as conflicting.
        if !active_dns.is_empty()
            && active_dns != *static_servers
            && !dhcp_ns.is_empty()
            && active_dns == dhcp_ns
        {
            return Err(format!(
                "Conflicting IPv4 DNS signals: static registry has {static_servers:?} while active DNS matches DhcpNameServer {dhcp_ns:?}"
            ));
        }
        return Ok(DnsMode::Static);
    }

    // Both `NameServer` and `ProfileNameServer` are empty.
    // We MUST NOT assume Automatic solely from missing/null registry read or empty DNS list.
    if raw.ipv4_reg_key_exists == Some(false) {
        return Err("Tcpip interface registry key does not exist or is unreadable".into());
    }

    if raw.ipv4_reg_key_exists.is_none() {
        return Err("Missing Tcpip interface registry verification signal".into());
    }

    // Check for contradiction between `Get-NetIPInterface.Dhcp` and registry `EnableDHCP`.
    match (iface_dhcp, raw.enable_dhcp_reg) {
        (DhcpStatus::Enabled, Some(0)) => {
            return Err("Conflicting DHCP signals: NetIPInterface reports Enabled while registry EnableDHCP is 0".into());
        }
        (DhcpStatus::Disabled, Some(1)) => {
            return Err("Conflicting DHCP signals: NetIPInterface reports Disabled while registry EnableDHCP is 1".into());
        }
        (DhcpStatus::Unknown, None) => {
            return Err("Insufficient DHCP interface signals (NetIPInterface Unknown and EnableDHCP missing)".into());
        }
        _ => {}
    }

    let dhcp_enabled = matches!(iface_dhcp, DhcpStatus::Enabled) || raw.enable_dhcp_reg == Some(1);

    if dhcp_enabled {
        // If active DNS servers are reported, confirm they come from DHCP (`DhcpNameServer`),
        // not an unrecognized source with empty `DhcpNameServer`.
        if !active_dns.is_empty() && dhcp_ns.is_empty() {
            return Err(format!(
                "Active IPv4 DNS servers {active_dns:?} reported with empty NameServer, ProfileNameServer, and DhcpNameServer"
            ));
        }
        if !active_dns.is_empty() && !dhcp_ns.is_empty() && active_dns != dhcp_ns {
            return Err(format!(
                "Active IPv4 DNS servers {active_dns:?} do not match DhcpNameServer {dhcp_ns:?}"
            ));
        }
        return Ok(DnsMode::Automatic);
    }

    // DHCP for IP assignment is disabled, and both NameServer & ProfileNameServer are empty.
    if active_dns.is_empty() {
        // Static IP without configured static DNS servers still represents Automatic/unset DNS mode in Windows,
        // verified by empty NameServer, empty ProfileNameServer, empty active DNS, and existing registry key.
        Ok(DnsMode::Automatic)
    } else {
        Err(format!(
            "DHCP is disabled and NameServer/ProfileNameServer are empty, yet active IPv4 DNS reports {active_dns:?}"
        ))
    }
}

pub fn normalize_raw_adapter(raw: RawAdapterPayload) -> Result<NetworkAdapter, AppError> {
    let guid = validate_adapter_guid(&raw.guid)?;
    let adapter_type = parse_adapter_type(
        raw.physical_media_type.as_deref(),
        raw.is_virtual.unwrap_or(false),
        &raw.name,
        &raw.interface_description,
    );
    let operational_status = parse_adapter_status(&raw.operational_status);
    let dhcp_status = parse_dhcp_status(raw.dhcp_status.as_deref());

    let profile_ns = split_dns_tokens(raw.profile_name_server_reg.as_deref());
    let static_ns = split_dns_tokens(raw.static_name_server_reg.as_deref());
    let static_ipv4_dns_reg_servers = if !profile_ns.is_empty() {
        profile_ns
    } else {
        static_ns
    };

    let (dns_mode, dns_mode_indeterminate_reason) = match detect_ipv4_dns_mode(&raw) {
        Ok(mode) => (mode, None),
        Err(reason) => (DnsMode::Automatic, Some(reason)),
    };

    Ok(NetworkAdapter {
        guid,
        interface_index: raw.interface_index,
        name: raw.name,
        interface_description: raw.interface_description,
        adapter_type,
        operational_status,
        ipv4_addresses: raw.ipv4_addresses,
        ipv4_gateway: raw.ipv4_gateway.filter(|s| !s.trim().is_empty()),
        current_ipv4_dns_servers: raw.current_ipv4_dns_servers,
        dns_mode,
        dhcp_status,
        mac_address: raw.mac_address.filter(|s| !s.trim().is_empty()),
        link_speed: raw.link_speed.filter(|s| !s.trim().is_empty()),
        dns_mode_indeterminate_reason,
        static_ipv4_dns_reg_servers,
    })
}

pub fn parse_adapters_json(stdout: &str) -> Result<Vec<NetworkAdapter>, AppError> {
    let trimmed = stdout.trim().trim_start_matches('\u{feff}').trim();
    if trimmed.is_empty() {
        return Err(AppError::PowershellExecutionFailed {
            detail: "PowerShell adapter listing returned empty output".into(),
        });
    }
    if trimmed == "null" {
        return Ok(Vec::new());
    }

    let parsed_value: serde_json::Value =
        serde_json::from_str(trimmed).map_err(|e| AppError::PowershellExecutionFailed {
            detail: format!("Failed to parse adapter JSON from Windows: {e}"),
        })?;

    let items: Vec<serde_json::Value> = match parsed_value {
        serde_json::Value::Null => return Ok(Vec::new()),
        serde_json::Value::Array(arr) => arr,
        serde_json::Value::Object(_) => vec![parsed_value],
        other => {
            return Err(AppError::PowershellExecutionFailed {
                detail: format!(
                    "Unexpected adapter JSON root type from Windows: expected array or object, got {other}"
                ),
            });
        }
    };

    let item_count = items.len();
    let mut adapters = Vec::with_capacity(item_count);
    let mut first_skip_reason: Option<String> = None;
    for (idx, item) in items.into_iter().enumerate() {
        let raw = match serde_json::from_value::<RawAdapterPayload>(item) {
            Ok(raw) => raw,
            Err(err) => {
                let reason = err.to_string();
                warn!(
                    index = idx,
                    error = %reason,
                    "Skipping malformed network adapter entry from Windows enumeration"
                );
                if first_skip_reason.is_none() {
                    first_skip_reason = Some(reason);
                }
                continue;
            }
        };

        match normalize_raw_adapter(raw) {
            Ok(adapter) => adapters.push(adapter),
            Err(err) => {
                let reason = err.to_string();
                warn!(
                    index = idx,
                    error = %reason,
                    "Skipping network adapter entry with invalid metadata from Windows enumeration"
                );
                if first_skip_reason.is_none() {
                    first_skip_reason = Some(reason);
                }
            }
        }
    }

    if item_count > 0 && adapters.is_empty() {
        return Err(AppError::PowershellExecutionFailed {
            detail: format!(
                "Windows returned {item_count} adapter entries, but none could be validated: {}",
                first_skip_reason.unwrap_or_else(|| "unknown validation error".into())
            ),
        });
    }

    Ok(adapters)
}

pub fn parse_single_adapter_json(stdout: &str) -> Result<Option<NetworkAdapter>, AppError> {
    let trimmed = stdout.trim().trim_start_matches('\u{feff}').trim();
    if trimmed.is_empty() {
        return Err(AppError::PowershellExecutionFailed {
            detail: "PowerShell single adapter query returned empty output".into(),
        });
    }
    if trimmed == "null" {
        return Ok(None);
    }

    let raw: RawAdapterPayload =
        serde_json::from_str(trimmed).map_err(|e| AppError::PowershellExecutionFailed {
            detail: format!("Failed to parse single adapter JSON from Windows: {e}"),
        })?;

    let adapter = normalize_raw_adapter(raw)?;
    Ok(Some(adapter))
}

pub fn parse_ipv6_dns_snapshot_json(stdout: &str) -> Result<Option<Ipv6DnsSnapshot>, AppError> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() || trimmed == "null" {
        return Ok(None);
    }

    let raw: RawIpv6DnsPayload =
        serde_json::from_str(trimmed).map_err(|e| AppError::PowershellExecutionFailed {
            detail: format!("Failed to parse IPv6 DNS state JSON from Windows: {e}"),
        })?;

    let static_reg = normalize_ipv6_reg(raw.static_name_server_reg.as_deref());
    let profile_reg = normalize_ipv6_reg(raw.profile_name_server_reg.as_deref());
    let server_addresses = normalize_ipv6_list(&raw.server_addresses);
    let dns_mode = if static_reg.is_some() || profile_reg.is_some() {
        DnsMode::Static
    } else {
        DnsMode::Automatic
    };

    Ok(Some(Ipv6DnsSnapshot {
        server_addresses,
        static_name_server_reg: static_reg,
        profile_name_server_reg: profile_reg,
        dns_mode,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinguishes_automatic_dns_with_dhcp_servers_from_static_dns() {
        let json = r#"[{
            "guid": "{11111111-2222-3333-4444-555555555555}",
            "interfaceIndex": 14,
            "name": "Wi-Fi",
            "interfaceDescription": "Intel(R) Wi-Fi 6 AX201",
            "physicalMediaType": "Native 802.11",
            "isVirtual": false,
            "operationalStatus": "Up",
            "ipv4Addresses": ["192.168.1.42"],
            "ipv4Gateway": "192.168.1.1",
            "currentIpv4DnsServers": ["192.168.1.1"],
            "staticNameServerReg": "",
            "profileNameServerReg": null,
            "dhcpNameServerReg": "192.168.1.1",
            "enableDhcpReg": 1,
            "ipv4RegKeyExists": true,
            "dhcpStatus": "Enabled",
            "macAddress": "AA-BB-CC-DD-EE-FF",
            "linkSpeed": "866.7 Mbps"
        }]"#;

        let adapters = parse_adapters_json(json).unwrap();
        assert_eq!(adapters.len(), 1);
        assert_eq!(adapters[0].adapter_type, AdapterType::Wifi);
        assert_eq!(adapters[0].operational_status, AdapterStatus::Up);
        assert_eq!(adapters[0].dns_mode, DnsMode::Automatic);
        assert_eq!(adapters[0].dhcp_status, DhcpStatus::Enabled);
        assert_eq!(adapters[0].current_ipv4_dns_servers, vec!["192.168.1.1"]);
        assert!(adapters[0].dns_mode_indeterminate_reason.is_none());
    }

    #[test]
    fn detects_static_dns_from_name_server_or_profile_name_server() {
        let json = r#"{
            "guid": "22222222-2222-3333-4444-555555555555",
            "interfaceIndex": 7,
            "name": "Ethernet",
            "interfaceDescription": "Realtek PCIe GbE Family Controller",
            "physicalMediaType": "802.3",
            "isVirtual": false,
            "operationalStatus": "Up",
            "ipv4Addresses": ["192.168.1.10"],
            "ipv4Gateway": "192.168.1.1",
            "currentIpv4DnsServers": ["1.1.1.1", "1.0.0.1"],
            "staticNameServerReg": "",
            "profileNameServerReg": "1.1.1.1,1.0.0.1",
            "dhcpNameServerReg": "192.168.1.1",
            "enableDhcpReg": 1,
            "ipv4RegKeyExists": true,
            "dhcpStatus": "Enabled",
            "macAddress": "00-11-22-33-44-55",
            "linkSpeed": "1 Gbps"
        }"#;

        let adapter = parse_single_adapter_json(json).unwrap().unwrap();
        assert_eq!(adapter.dns_mode, DnsMode::Static);
        assert!(adapter.dns_mode_indeterminate_reason.is_none());
    }

    #[test]
    fn marks_indeterminate_or_conflicting_dns_mode_signals_without_failing_parse() {
        // Missing registry key + unknown DHCP + empty DNS must NOT blindly assume Automatic
        let missing_signals = r#"{
            "guid": "{33333333-2222-3333-4444-555555555555}",
            "interfaceIndex": 9,
            "name": "Ethernet 2",
            "interfaceDescription": "Unverified NIC",
            "physicalMediaType": "802.3",
            "isVirtual": false,
            "operationalStatus": "Up",
            "ipv4Addresses": [],
            "ipv4Gateway": null,
            "currentIpv4DnsServers": [],
            "staticNameServerReg": null,
            "profileNameServerReg": null,
            "dhcpNameServerReg": null,
            "enableDhcpReg": null,
            "ipv4RegKeyExists": false,
            "dhcpStatus": "Unknown",
            "macAddress": null,
            "linkSpeed": null
        }"#;

        let adapter1 = parse_single_adapter_json(missing_signals).unwrap().unwrap();
        assert!(adapter1.dns_mode_indeterminate_reason.is_some());

        // Active DNS present when NameServer, ProfileNameServer, and DhcpNameServer are all empty
        let inconsistent_active = r#"{
            "guid": "{44444444-2222-3333-4444-555555555555}",
            "interfaceIndex": 11,
            "name": "Wi-Fi 2",
            "interfaceDescription": "Intel Wi-Fi",
            "physicalMediaType": "Native 802.11",
            "isVirtual": false,
            "operationalStatus": "Up",
            "ipv4Addresses": ["192.168.1.50"],
            "ipv4Gateway": "192.168.1.1",
            "currentIpv4DnsServers": ["8.8.8.8"],
            "staticNameServerReg": "",
            "profileNameServerReg": "",
            "dhcpNameServerReg": "",
            "enableDhcpReg": 1,
            "ipv4RegKeyExists": true,
            "dhcpStatus": "Enabled",
            "macAddress": null,
            "linkSpeed": null
        }"#;

        let adapter2 = parse_single_adapter_json(inconsistent_active).unwrap().unwrap();
        assert!(adapter2.dns_mode_indeterminate_reason.is_some());
    }

    #[test]
    fn rejects_empty_stdout_from_adapter_queries() {
        assert!(matches!(
            parse_adapters_json(""),
            Err(AppError::PowershellExecutionFailed { .. })
        ));
        assert!(matches!(
            parse_adapters_json("   \r\n "),
            Err(AppError::PowershellExecutionFailed { .. })
        ));
        assert!(matches!(
            parse_single_adapter_json(""),
            Err(AppError::PowershellExecutionFailed { .. })
        ));
        assert!(parse_adapters_json("\u{feff}[]").unwrap().is_empty());
    }

    #[test]
    fn parses_and_normalizes_ipv6_dns_snapshot() {
        let json = r#"{
            "serverAddresses": ["2001:4860:4860:0000:0000:0000:0000:8888", "2001:4860:4860::8844"],
            "staticNameServerReg": "2001:4860:4860:0:0:0:0:8888 2001:4860:4860::8844",
            "profileNameServerReg": ""
        }"#;

        let snap = parse_ipv6_dns_snapshot_json(json).unwrap().unwrap();
        assert_eq!(snap.dns_mode, DnsMode::Static);
        assert_eq!(
            snap.server_addresses,
            vec!["2001:4860:4860::8888", "2001:4860:4860::8844"]
        );
        assert_eq!(
            snap.static_name_server_reg.as_deref(),
            Some("2001:4860:4860::8888,2001:4860:4860::8844")
        );
        assert_eq!(snap.profile_name_server_reg, None);
    }

    #[test]
    fn skips_invalid_guid_or_malformed_sibling_adapter_without_dropping_valid_adapters() {
        let json = r#"[
            {
                "guid": "",
                "interfaceIndex": 1,
                "name": "Broken Virtual Miniport",
                "interfaceDescription": null,
                "operationalStatus": "Unknown"
            },
            {
                "guid": "{55555555-2222-3333-4444-555555555555}",
                "interfaceIndex": 12,
                "name": "Wi-Fi",
                "interfaceDescription": null,
                "physicalMediaType": "Native 802.11",
                "isVirtual": false,
                "operationalStatus": "Up",
                "ipv4Addresses": ["192.168.1.25"],
                "ipv4Gateway": "192.168.1.1",
                "currentIpv4DnsServers": ["192.168.1.1"],
                "staticNameServerReg": "",
                "profileNameServerReg": null,
                "dhcpNameServerReg": "192.168.1.1",
                "enableDhcpReg": 1,
                "ipv4RegKeyExists": true,
                "dhcpStatus": "Enabled",
                "macAddress": "AA-BB-CC-DD-EE-01",
                "linkSpeed": "866.7 Mbps"
            }
        ]"#;

        let adapters = parse_adapters_json(json).unwrap();
        assert_eq!(adapters.len(), 1);
        assert_eq!(adapters[0].name, "Wi-Fi");
        assert_eq!(adapters[0].guid, "{55555555-2222-3333-4444-555555555555}");
    }

    #[test]
    fn accepts_single_object_json_when_powershell_unwraps_single_adapter() {
        let json = r#"{
            "guid": "{66666666-2222-3333-4444-555555555555}",
            "interfaceIndex": 4,
            "name": "Ethernet",
            "interfaceDescription": "Intel Ethernet",
            "physicalMediaType": "802.3",
            "isVirtual": false,
            "operationalStatus": "Up",
            "ipv4Addresses": ["10.0.0.5"],
            "ipv4Gateway": "10.0.0.1",
            "currentIpv4DnsServers": ["10.0.0.1"],
            "staticNameServerReg": "",
            "profileNameServerReg": null,
            "dhcpNameServerReg": "10.0.0.1",
            "enableDhcpReg": 1,
            "ipv4RegKeyExists": true,
            "dhcpStatus": "Enabled",
            "macAddress": "00-11-22-33-44-66",
            "linkSpeed": "1 Gbps"
        }"#;

        let adapters = parse_adapters_json(json).unwrap();
        assert_eq!(adapters.len(), 1);
        assert_eq!(adapters[0].name, "Ethernet");
    }

    #[test]
    fn errors_when_windows_returns_entries_that_all_fail_validation() {
        let json = r#"[
            {
                "guid": "",
                "interfaceIndex": 1,
                "name": "Invalid",
                "interfaceDescription": null,
                "operationalStatus": "Unknown"
            }
        ]"#;

        assert!(matches!(
            parse_adapters_json(json),
            Err(AppError::PowershellExecutionFailed { .. })
        ));
    }

    #[test]
    fn parses_powershell_5_scalar_string_and_wrapped_map_arrays() {
        let json = r#"[
            {
                "guid": "{77777777-2222-3333-4444-555555555555}",
                "interfaceIndex": 5,
                "name": "Ethernet",
                "interfaceDescription": "Realtek PCIe",
                "physicalMediaType": "802.3",
                "isVirtual": false,
                "operationalStatus": "Up",
                "ipv4Addresses": {"value": ["192.168.1.10", "169.254.24.238"], "Count": 2},
                "ipv4Gateway": "192.168.1.1",
                "currentIpv4DnsServers": "192.168.1.1",
                "staticNameServerReg": "",
                "profileNameServerReg": null,
                "dhcpNameServerReg": "192.168.1.1",
                "enableDhcpReg": 1,
                "ipv4RegKeyExists": true,
                "dhcpStatus": "Enabled",
                "macAddress": "00-11-22-33-44-77",
                "linkSpeed": "1 Gbps"
            },
            {
                "guid": "{88888888-2222-3333-4444-555555555555}",
                "interfaceIndex": 6,
                "name": "Ethernet 2",
                "interfaceDescription": "Virtual Adapter",
                "physicalMediaType": "802.3",
                "isVirtual": true,
                "operationalStatus": "Disconnected",
                "ipv4Addresses": "169.254.24.238",
                "ipv4Gateway": null,
                "currentIpv4DnsServers": {"value": "192.168.1.1", "Count": 1},
                "staticNameServerReg": "",
                "profileNameServerReg": null,
                "dhcpNameServerReg": "192.168.1.1",
                "enableDhcpReg": 1,
                "ipv4RegKeyExists": true,
                "dhcpStatus": "Enabled",
                "macAddress": null,
                "linkSpeed": null
            }
        ]"#;

        let adapters = parse_adapters_json(json).unwrap();
        assert_eq!(adapters.len(), 2);
        assert_eq!(
            adapters[0].ipv4_addresses,
            vec!["192.168.1.10", "169.254.24.238"]
        );
        assert_eq!(adapters[0].current_ipv4_dns_servers, vec!["192.168.1.1"]);
        assert_eq!(adapters[1].ipv4_addresses, vec!["169.254.24.238"]);
        assert_eq!(adapters[1].current_ipv4_dns_servers, vec!["192.168.1.1"]);

        let ipv6_scalar = r#"{
            "serverAddresses": "2001:4860:4860::8888",
            "staticNameServerReg": "",
            "profileNameServerReg": null
        }"#;
        let snap = parse_ipv6_dns_snapshot_json(ipv6_scalar).unwrap().unwrap();
        assert_eq!(snap.server_addresses, vec!["2001:4860:4860::8888"]);
    }
}
