pub mod parser;
pub mod powershell;

use std::net::Ipv4Addr;

use crate::errors::AppError;
use crate::models::{Ipv6DnsSnapshot, NetworkAdapter};

pub use powershell::PowerShellNetworkBackend;

pub trait NetworkBackend: Send + Sync {
    fn list_adapters(&self) -> Result<Vec<NetworkAdapter>, AppError>;
    fn get_adapter_by_guid(&self, adapter_guid: &str) -> Result<Option<NetworkAdapter>, AppError>;
    fn set_ipv4_dns(&self, adapter_guid: &str, servers: &[Ipv4Addr]) -> Result<(), AppError>;
    fn reset_ipv4_dns_to_automatic(&self, adapter_guid: &str) -> Result<(), AppError>;
    fn get_ipv6_dns_state(&self, adapter_guid: &str) -> Result<Option<Ipv6DnsSnapshot>, AppError>;
    fn restore_ipv6_dns_state(
        &self,
        adapter_guid: &str,
        snapshot: &Ipv6DnsSnapshot,
    ) -> Result<(), AppError>;
}
