use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdapterType {
    Wifi,
    Ethernet,
    Virtual,
    Loopback,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdapterStatus {
    Up,
    Disconnected,
    Disabled,
    NotPresent,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DnsMode {
    Automatic,
    Static,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DhcpStatus {
    Enabled,
    Disabled,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkAdapter {
    pub guid: String,
    pub interface_index: u32,
    pub name: String,
    pub interface_description: String,
    pub adapter_type: AdapterType,
    pub operational_status: AdapterStatus,
    pub ipv4_addresses: Vec<String>,
    pub ipv4_gateway: Option<String>,
    pub current_ipv4_dns_servers: Vec<String>,
    pub dns_mode: DnsMode,
    pub dhcp_status: DhcpStatus,
    pub mac_address: Option<String>,
    pub link_speed: Option<String>,
    #[serde(default)]
    pub dns_mode_indeterminate_reason: Option<String>,
    #[serde(skip)]
    pub static_ipv4_dns_reg_servers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ipv6DnsSnapshot {
    pub server_addresses: Vec<String>,
    pub static_name_server_reg: Option<String>,
    pub profile_name_server_reg: Option<String>,
    pub dns_mode: DnsMode,
}

impl Ipv6DnsSnapshot {
    pub fn matches(&self, other: &Self) -> bool {
        self.dns_mode == other.dns_mode
            && self.server_addresses == other.server_addresses
            && self.static_name_server_reg == other.static_name_server_reg
            && self.profile_name_server_reg == other.profile_name_server_reg
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsProvider {
    pub id: String,
    pub name: String,
    pub primary_dns: String,
    #[serde(default)]
    pub secondary_dns: Option<String>,
    pub is_built_in: bool,
    pub category: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsProviderCatalog {
    pub built_in: Vec<DnsProvider>,
    pub custom: Vec<DnsProvider>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminStatus {
    pub is_elevated: bool,
    pub can_request_elevation: bool,
    pub platform: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyDnsResult {
    pub adapter_guid: String,
    pub applied_servers: Vec<String>,
    pub verified: bool,
    pub backup_created: bool,
    pub adapter: NetworkAdapter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreDnsResult {
    pub adapter_guid: String,
    pub restored_mode: DnsMode,
    pub restored_servers: Vec<String>,
    pub verified: bool,
    pub backup_cleared: bool,
    pub adapter: NetworkAdapter,
}
