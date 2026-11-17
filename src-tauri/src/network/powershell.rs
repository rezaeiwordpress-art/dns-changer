use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::io::Read;
use std::net::Ipv4Addr;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, warn};

use crate::dns::validation::{validate_adapter_guid, MAX_IPV4_DNS_SERVERS};
use crate::errors::AppError;
use crate::models::{DnsMode, Ipv6DnsSnapshot, NetworkAdapter};
use crate::network::parser::{
    parse_adapters_json, parse_ipv6_dns_snapshot_json, parse_single_adapter_json,
};
use crate::network::NetworkBackend;

const POWERSHELL_QUERY_TIMEOUT_SECS: u64 = 45;
const POWERSHELL_MUTATION_TIMEOUT_SECS: u64 = 15;
const STDERR_PREVIEW_MAX_CHARS: usize = 500;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Fixed PowerShell script to list all Windows network adapters (including Disabled and Disconnected)
/// along with IPv4 addresses, gateway, DHCP status, active IPv4 DNS servers, and multiple Windows
/// Tcpip interface registry signals (`NameServer`, `ProfileNameServer`, `DhcpNameServer`, `EnableDHCP`).
/// Uses `Get-NetAdapter` primarily and falls back to `.NET NetworkInterface` if NetAdapter/WMI fails or returns empty.
const GET_ALL_ADAPTERS_SCRIPT: &str = r#"
try {
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    $OutputEncoding = [System.Text.Encoding]::UTF8
} catch { }
$ErrorActionPreference = 'Stop'

function Get-TcpipInterfaceSignals {
    param([string]$Guid)
    $regPath = "HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces\$Guid"
    $regExists = $false
    $regNameServer = $null
    $regProfileNameServer = $null
    $regDhcpNameServer = $null
    $regEnableDhcp = $null
    try {
        $regExists = [bool](Test-Path -LiteralPath $regPath -ErrorAction SilentlyContinue)
        if ($regExists) {
            $regProps = Get-ItemProperty -LiteralPath $regPath -ErrorAction SilentlyContinue
            if ($null -ne $regProps) {
                if ($null -ne $regProps.PSObject.Properties['NameServer']) {
                    $regNameServer = [string]$regProps.NameServer
                }
                if ($null -ne $regProps.PSObject.Properties['ProfileNameServer']) {
                    $regProfileNameServer = [string]$regProps.ProfileNameServer
                }
                if ($null -ne $regProps.PSObject.Properties['DhcpNameServer']) {
                    $regDhcpNameServer = [string]$regProps.DhcpNameServer
                }
                if ($null -ne $regProps.PSObject.Properties['EnableDHCP']) {
                    $parsedDhcp = [uint32]0
                    if ([uint32]::TryParse([string]$regProps.EnableDHCP, [ref]$parsedDhcp)) {
                        $regEnableDhcp = $parsedDhcp
                    }
                }
            }
        }
    } catch { }

    return @{
        Exists = $regExists
        NameServer = $regNameServer
        ProfileNameServer = $regProfileNameServer
        DhcpNameServer = $regDhcpNameServer
        EnableDHCP = $regEnableDhcp
    }
}

$results = @()
$netAdapterFailed = $false
$netAdapterError = $null
$adapters = @()
try {
    $adapters = @(Get-NetAdapter -ErrorAction Stop)
} catch {
    $netAdapterFailed = $true
    $netAdapterError = $_.Exception.Message
}

if (-not $netAdapterFailed -and $adapters.Count -gt 0) {
    $ipAddrs = @()
    $routes = @()
    $ipIfaces = @()
    $dnsAddrs = @()
    try { $ipAddrs = @(Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue) } catch { }
    try { $routes = @(Get-NetRoute -DestinationPrefix '0.0.0.0/0' -AddressFamily IPv4 -ErrorAction SilentlyContinue) } catch { }
    try { $ipIfaces = @(Get-NetIPInterface -AddressFamily IPv4 -ErrorAction SilentlyContinue) } catch { }
    try { $dnsAddrs = @(Get-DnsClientServerAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue) } catch { }

    $addrMap = @{}
    foreach ($a in $ipAddrs) {
        $idx = 0
        if ([int]::TryParse([string]$a.InterfaceIndex, [ref]$idx)) {
            if (-not $addrMap.ContainsKey($idx)) { $addrMap[$idx] = @() }
            if ($a.IPAddress) { $addrMap[$idx] += [string]$a.IPAddress }
        }
    }

    $gwMap = @{}
    foreach ($r in ($routes | Sort-Object RouteMetric)) {
        $idx = 0
        if ([int]::TryParse([string]$r.InterfaceIndex, [ref]$idx)) {
            if (-not $gwMap.ContainsKey($idx) -and $r.NextHop -and $r.NextHop -ne '0.0.0.0') {
                $gwMap[$idx] = [string]$r.NextHop
            }
        }
    }

    $dhcpMap = @{}
    foreach ($i in $ipIfaces) {
        $idx = 0
        if ([int]::TryParse([string]$i.InterfaceIndex, [ref]$idx)) {
            $dhcpMap[$idx] = [string]$i.Dhcp
        }
    }

    $dnsMap = @{}
    foreach ($d in $dnsAddrs) {
        $idx = 0
        if ([int]::TryParse([string]$d.InterfaceIndex, [ref]$idx)) {
            $dnsMap[$idx] = @($d.ServerAddresses | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { [string]$_ })
        }
    }

    foreach ($ad in $adapters) {
        try {
            $guid = [string]$ad.InterfaceGuid
            if ([string]::IsNullOrWhiteSpace($guid)) { continue }
            $idx = 0
            [void][int]::TryParse([string]$ad.ifIndex, [ref]$idx)
            $reg = Get-TcpipInterfaceSignals -Guid $guid

            $results += [PSCustomObject]@{
                guid = $guid
                interfaceIndex = $idx
                name = if ($null -ne $ad.Name) { [string]$ad.Name } else { '' }
                interfaceDescription = if ($null -ne $ad.InterfaceDescription) { [string]$ad.InterfaceDescription } else { '' }
                physicalMediaType = if ($null -ne $ad.PhysicalMediaType) { [string]$ad.PhysicalMediaType } else { $null }
                isVirtual = [bool]$ad.Virtual
                operationalStatus = if ($null -ne $ad.Status) { [string]$ad.Status } else { 'Unknown' }
                ipv4Addresses = if ($addrMap.ContainsKey($idx)) { @($addrMap[$idx]) } else { @() }
                ipv4Gateway = if ($gwMap.ContainsKey($idx)) { $gwMap[$idx] } else { $null }
                currentIpv4DnsServers = if ($dnsMap.ContainsKey($idx)) { @($dnsMap[$idx]) } else { @() }
                staticNameServerReg = $reg.NameServer
                profileNameServerReg = $reg.ProfileNameServer
                dhcpNameServerReg = $reg.DhcpNameServer
                enableDhcpReg = $reg.EnableDHCP
                ipv4RegKeyExists = [bool]$reg.Exists
                dhcpStatus = if ($dhcpMap.ContainsKey($idx)) { $dhcpMap[$idx] } else { 'Unknown' }
                macAddress = if ($null -ne $ad.MacAddress) { [string]$ad.MacAddress } else { $null }
                linkSpeed = if ($null -ne $ad.LinkSpeed) { [string]$ad.LinkSpeed } else { $null }
            }
        } catch {
            if ([string]::IsNullOrWhiteSpace($netAdapterError)) {
                $netAdapterError = $_.Exception.Message
            }
        }
    }
}

$fallbackError = $null
$netIfacesCount = 0
if ($results.Count -eq 0) {
    $netIfaces = @()
    try {
        $netIfaces = @([System.Net.NetworkInformation.NetworkInterface]::GetAllNetworkInterfaces())
        $netIfacesCount = $netIfaces.Count
    } catch {
        $fallbackError = $_.Exception.Message
    }
    foreach ($ni in $netIfaces) {
        try {
            $niType = [string]$ni.NetworkInterfaceType
            if ($niType -eq 'Loopback' -or $niType -eq 'Tunnel') { continue }

            $guid = [string]$ni.Id
            if ([string]::IsNullOrWhiteSpace($guid)) { continue }

            $ipProps = $null
            $ipv4Props = $null
            try {
                $ipProps = $ni.GetIPProperties()
                if ($null -ne $ipProps) {
                    $ipv4Props = $ipProps.GetIPv4Properties()
                }
            } catch {
                $ipv4Props = $null
            }
            if ($null -eq $ipv4Props) { continue }

            $idx = [int]$ipv4Props.Index
            $dhcpStatus = if ($ipv4Props.IsDhcpEnabled) { 'Enabled' } else { 'Disabled' }
            $ipv4Addrs = @(
                $ipProps.UnicastAddresses |
                    Where-Object { $null -ne $_.Address -and $_.Address.AddressFamily -eq [System.Net.Sockets.AddressFamily]::InterNetwork } |
                    ForEach-Object { [string]$_.Address.IPAddressToString }
            )
            $gw = $null
            $gwObj = $ipProps.GatewayAddresses |
                Where-Object { $null -ne $_.Address -and $_.Address.AddressFamily -eq [System.Net.Sockets.AddressFamily]::InterNetwork -and $_.Address.IPAddressToString -ne '0.0.0.0' } |
                Select-Object -First 1
            if ($null -ne $gwObj) {
                $gw = [string]$gwObj.Address.IPAddressToString
            }
            $dnsServers = @(
                $ipProps.DnsAddresses |
                    Where-Object { $null -ne $_ -and $_.AddressFamily -eq [System.Net.Sockets.AddressFamily]::InterNetwork } |
                    ForEach-Object { [string]$_.IPAddressToString }
            )

            $mediaType = switch -Regex ($niType) {
                'Wireless80211' { 'Native 802.11'; break }
                'Ethernet|GigabitEthernet|FastEthernet' { '802.3'; break }
                'Loopback' { 'Loopback'; break }
                default { $niType }
            }
            $isVirtual = ($niType -eq 'Tunnel' -or $niType -eq 'Ppp')
            $opRaw = [string]$ni.OperationalStatus
            $opStatus = switch ($opRaw) {
                'Up' { 'Up'; break }
                'Down' { 'Disconnected'; break }
                'Dormant' { 'Disconnected'; break }
                'LowerLayerDown' { 'Disconnected'; break }
                'NotPresent' { 'NotPresent'; break }
                default { $opRaw }
            }

            $mac = $null
            try {
                $rawMac = $ni.GetPhysicalAddress().ToString()
                if (-not [string]::IsNullOrWhiteSpace($rawMac)) {
                    if ($rawMac.Length -eq 12) {
                        $mac = (($rawMac -split '(.{2})' | Where-Object { $_ -ne '' }) -join '-')
                    } else {
                        $mac = $rawMac
                    }
                }
            } catch { }

            $linkSpeed = $null
            try {
                if ($ni.Speed -gt 0) {
                    if ($ni.Speed -ge 1000000000) {
                        $linkSpeed = "$([math]::Round($ni.Speed / 1000000000, 1)) Gbps"
                    } elseif ($ni.Speed -ge 1000000) {
                        $linkSpeed = "$([math]::Round($ni.Speed / 1000000, 1)) Mbps"
                    } else {
                        $linkSpeed = "$($ni.Speed) bps"
                    }
                }
            } catch { }

            $reg = Get-TcpipInterfaceSignals -Guid $guid
            $results += [PSCustomObject]@{
                guid = $guid
                interfaceIndex = $idx
                name = if ($null -ne $ni.Name) { [string]$ni.Name } else { '' }
                interfaceDescription = if ($null -ne $ni.Description) { [string]$ni.Description } else { '' }
                physicalMediaType = $mediaType
                isVirtual = [bool]$isVirtual
                operationalStatus = $opStatus
                ipv4Addresses = @($ipv4Addrs)
                ipv4Gateway = $gw
                currentIpv4DnsServers = @($dnsServers)
                staticNameServerReg = $reg.NameServer
                profileNameServerReg = $reg.ProfileNameServer
                dhcpNameServerReg = $reg.DhcpNameServer
                enableDhcpReg = $reg.EnableDHCP
                ipv4RegKeyExists = [bool]$reg.Exists
                dhcpStatus = $dhcpStatus
                macAddress = $mac
                linkSpeed = $linkSpeed
            }
        } catch {
            if ([string]::IsNullOrWhiteSpace($fallbackError)) {
                $fallbackError = $_.Exception.Message
            }
        }
    }
}

if ($results.Count -eq 0) {
    $details = @("Get-NetAdapter count=$($adapters.Count)")
    if (-not [string]::IsNullOrWhiteSpace($netAdapterError)) {
        $details += "Get-NetAdapter error: $netAdapterError"
    }
    $details += "NetworkInterface count=$netIfacesCount"
    if (-not [string]::IsNullOrWhiteSpace($fallbackError)) {
        $details += "NetworkInterface error: $fallbackError"
    }
    throw ("No network adapters returned by Windows (" + ($details -join '; ') + ")")
}

ConvertTo-Json -InputObject @($results) -Depth 5 -Compress
"#;

/// Fixed PowerShell script querying a single adapter strictly by `$env:DNS_CHANGER_ADAPTER_GUID`.
const GET_ADAPTER_BY_GUID_SCRIPT: &str = r#"
try {
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    $OutputEncoding = [System.Text.Encoding]::UTF8
} catch { }
$ErrorActionPreference = 'Stop'
$targetGuid = $env:DNS_CHANGER_ADAPTER_GUID
if ([string]::IsNullOrWhiteSpace($targetGuid)) {
    throw "Missing DNS_CHANGER_ADAPTER_GUID"
}

function Get-TcpipInterfaceSignals {
    param([string]$Guid)
    $regPath = "HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces\$Guid"
    $regExists = $false
    $regNameServer = $null
    $regProfileNameServer = $null
    $regDhcpNameServer = $null
    $regEnableDhcp = $null
    try {
        $regExists = [bool](Test-Path -LiteralPath $regPath -ErrorAction SilentlyContinue)
        if ($regExists) {
            $regProps = Get-ItemProperty -LiteralPath $regPath -ErrorAction SilentlyContinue
            if ($null -ne $regProps) {
                if ($null -ne $regProps.PSObject.Properties['NameServer']) {
                    $regNameServer = [string]$regProps.NameServer
                }
                if ($null -ne $regProps.PSObject.Properties['ProfileNameServer']) {
                    $regProfileNameServer = [string]$regProps.ProfileNameServer
                }
                if ($null -ne $regProps.PSObject.Properties['DhcpNameServer']) {
                    $regDhcpNameServer = [string]$regProps.DhcpNameServer
                }
                if ($null -ne $regProps.PSObject.Properties['EnableDHCP']) {
                    $parsedDhcp = [uint32]0
                    if ([uint32]::TryParse([string]$regProps.EnableDHCP, [ref]$parsedDhcp)) {
                        $regEnableDhcp = $parsedDhcp
                    }
                }
            }
        }
    } catch { }

    return @{
        Exists = $regExists
        NameServer = $regNameServer
        ProfileNameServer = $regProfileNameServer
        DhcpNameServer = $regDhcpNameServer
        EnableDHCP = $regEnableDhcp
    }
}

$ad = $null
$netAdapterFailed = $false
try {
    $ad = Get-NetAdapter -ErrorAction Stop | Where-Object { $_.InterfaceGuid -ieq $targetGuid } | Select-Object -First 1
} catch {
    $netAdapterFailed = $true
}

if (-not $netAdapterFailed -and $null -ne $ad) {
    $idx = 0
    [void][int]::TryParse([string]$ad.ifIndex, [ref]$idx)
    $guid = [string]$ad.InterfaceGuid
    $ipConfig = $null
    $ipIface = $null
    $dnsObj = $null
    try { $ipConfig = Get-NetIPConfiguration -InterfaceIndex $idx -ErrorAction SilentlyContinue } catch { }
    try { $ipIface = Get-NetIPInterface -InterfaceIndex $idx -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object -First 1 } catch { }
    try { $dnsObj = Get-DnsClientServerAddress -InterfaceIndex $idx -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object -First 1 } catch { }

    $ipv4Addrs = @()
    if ($null -ne $ipConfig -and $null -ne $ipConfig.IPv4Address) {
        $ipv4Addrs = @($ipConfig.IPv4Address | ForEach-Object { [string]$_.IPAddress })
    } else {
        try {
            $ipv4Addrs = @(Get-NetIPAddress -InterfaceIndex $idx -AddressFamily IPv4 -ErrorAction SilentlyContinue | ForEach-Object { [string]$_.IPAddress })
        } catch { }
    }

    $gw = $null
    if ($null -ne $ipConfig -and $null -ne $ipConfig.IPv4DefaultGateway) {
        $gw = [string]($ipConfig.IPv4DefaultGateway | Select-Object -First 1).NextHop
    }

    $dnsServers = @()
    if ($null -ne $dnsObj -and $null -ne $dnsObj.ServerAddresses) {
        $dnsServers = @($dnsObj.ServerAddresses | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { [string]$_ })
    }

    $reg = Get-TcpipInterfaceSignals -Guid $guid
    $result = [PSCustomObject]@{
        guid = $guid
        interfaceIndex = $idx
        name = if ($null -ne $ad.Name) { [string]$ad.Name } else { '' }
        interfaceDescription = if ($null -ne $ad.InterfaceDescription) { [string]$ad.InterfaceDescription } else { '' }
        physicalMediaType = if ($null -ne $ad.PhysicalMediaType) { [string]$ad.PhysicalMediaType } else { $null }
        isVirtual = [bool]$ad.Virtual
        operationalStatus = if ($null -ne $ad.Status) { [string]$ad.Status } else { 'Unknown' }
        ipv4Addresses = @($ipv4Addrs)
        ipv4Gateway = $gw
        currentIpv4DnsServers = @($dnsServers)
        staticNameServerReg = $reg.NameServer
        profileNameServerReg = $reg.ProfileNameServer
        dhcpNameServerReg = $reg.DhcpNameServer
        enableDhcpReg = $reg.EnableDHCP
        ipv4RegKeyExists = [bool]$reg.Exists
        dhcpStatus = if ($null -ne $ipIface) { [string]$ipIface.Dhcp } else { 'Unknown' }
        macAddress = if ($null -ne $ad.MacAddress) { [string]$ad.MacAddress } else { $null }
        linkSpeed = if ($null -ne $ad.LinkSpeed) { [string]$ad.LinkSpeed } else { $null }
    }
    ConvertTo-Json -InputObject $result -Depth 5 -Compress
    exit 0
}

$ni = [System.Net.NetworkInformation.NetworkInterface]::GetAllNetworkInterfaces() |
    Where-Object { $_.Id -ieq $targetGuid } |
    Select-Object -First 1
if ($null -eq $ni) {
    'null'
    exit 0
}

$niType = [string]$ni.NetworkInterfaceType
if ($niType -eq 'Loopback' -or $niType -eq 'Tunnel') {
    'null'
    exit 0
}

$guid = [string]$ni.Id
$ipProps = $null
$ipv4Props = $null
try {
    $ipProps = $ni.GetIPProperties()
    if ($null -ne $ipProps) {
        $ipv4Props = $ipProps.GetIPv4Properties()
    }
} catch {
    $ipv4Props = $null
}
if ($null -eq $ipv4Props) {
    'null'
    exit 0
}

$idx = [int]$ipv4Props.Index
$dhcpStatus = if ($ipv4Props.IsDhcpEnabled) { 'Enabled' } else { 'Disabled' }
$ipv4Addrs = @(
    $ipProps.UnicastAddresses |
        Where-Object { $null -ne $_.Address -and $_.Address.AddressFamily -eq [System.Net.Sockets.AddressFamily]::InterNetwork } |
        ForEach-Object { [string]$_.Address.IPAddressToString }
)
$gw = $null
$gwObj = $ipProps.GatewayAddresses |
    Where-Object { $null -ne $_.Address -and $_.Address.AddressFamily -eq [System.Net.Sockets.AddressFamily]::InterNetwork -and $_.Address.IPAddressToString -ne '0.0.0.0' } |
    Select-Object -First 1
if ($null -ne $gwObj) {
    $gw = [string]$gwObj.Address.IPAddressToString
}
$dnsServers = @(
    $ipProps.DnsAddresses |
        Where-Object { $null -ne $_ -and $_.AddressFamily -eq [System.Net.Sockets.AddressFamily]::InterNetwork } |
        ForEach-Object { [string]$_.IPAddressToString }
)

$mediaType = switch -Regex ($niType) {
    'Wireless80211' { 'Native 802.11'; break }
    'Ethernet|GigabitEthernet|FastEthernet' { '802.3'; break }
    'Loopback' { 'Loopback'; break }
    default { $niType }
}
$isVirtual = ($niType -eq 'Tunnel' -or $niType -eq 'Ppp')
$opRaw = [string]$ni.OperationalStatus
$opStatus = switch ($opRaw) {
    'Up' { 'Up'; break }
    'Down' { 'Disconnected'; break }
    'Dormant' { 'Disconnected'; break }
    'LowerLayerDown' { 'Disconnected'; break }
    'NotPresent' { 'NotPresent'; break }
    default { $opRaw }
}

$mac = $null
try {
    $rawMac = $ni.GetPhysicalAddress().ToString()
    if (-not [string]::IsNullOrWhiteSpace($rawMac)) {
        if ($rawMac.Length -eq 12) {
            $mac = (($rawMac -split '(.{2})' | Where-Object { $_ -ne '' }) -join '-')
        } else {
            $mac = $rawMac
        }
    }
} catch { }

$linkSpeed = $null
try {
    if ($ni.Speed -gt 0) {
        if ($ni.Speed -ge 1000000000) {
            $linkSpeed = "$([math]::Round($ni.Speed / 1000000000, 1)) Gbps"
        } elseif ($ni.Speed -ge 1000000) {
            $linkSpeed = "$([math]::Round($ni.Speed / 1000000, 1)) Mbps"
        } else {
            $linkSpeed = "$($ni.Speed) bps"
        }
    }
} catch { }

$reg = Get-TcpipInterfaceSignals -Guid $guid
$result = [PSCustomObject]@{
    guid = $guid
    interfaceIndex = $idx
    name = if ($null -ne $ni.Name) { [string]$ni.Name } else { '' }
    interfaceDescription = if ($null -ne $ni.Description) { [string]$ni.Description } else { '' }
    physicalMediaType = $mediaType
    isVirtual = [bool]$isVirtual
    operationalStatus = $opStatus
    ipv4Addresses = @($ipv4Addrs)
    ipv4Gateway = $gw
    currentIpv4DnsServers = @($dnsServers)
    staticNameServerReg = $reg.NameServer
    profileNameServerReg = $reg.ProfileNameServer
    dhcpNameServerReg = $reg.DhcpNameServer
    enableDhcpReg = $reg.EnableDHCP
    ipv4RegKeyExists = [bool]$reg.Exists
    dhcpStatus = $dhcpStatus
    macAddress = $mac
    linkSpeed = $linkSpeed
}

ConvertTo-Json -InputObject $result -Depth 5 -Compress
"#;

/// Fixed PowerShell script applying 1..=8 validated IPv4-only DNS servers to the adapter matching
/// `$env:DNS_CHANGER_ADAPTER_GUID`. Validates IPv4 InterNetwork address family and touches IPv4 only.
const SET_IPV4_DNS_SCRIPT: &str = r#"
try {
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    $OutputEncoding = [System.Text.Encoding]::UTF8
} catch { }
$ErrorActionPreference = 'Stop'
$targetGuid = $env:DNS_CHANGER_ADAPTER_GUID
$serversRaw = $env:DNS_CHANGER_DNS_SERVERS

if ([string]::IsNullOrWhiteSpace($targetGuid) -or [string]::IsNullOrWhiteSpace($serversRaw)) {
    throw "Missing required DNS parameters"
}

$tokens = @($serversRaw -split ',' | ForEach-Object { $_.Trim() } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
if ($tokens.Count -lt 1 -or $tokens.Count -gt 8) {
    throw "Invalid IPv4 DNS server count: expected between 1 and 8"
}

$servers = @()
foreach ($tok in $tokens) {
    $parsedIp = $null
    if (-not [System.Net.IPAddress]::TryParse($tok, [ref]$parsedIp) -or $parsedIp.AddressFamily -ne [System.Net.Sockets.AddressFamily]::InterNetwork) {
        throw "Invalid IPv4 DNS address: $tok"
    }
    $servers += $parsedIp.IPAddressToString
}

$idx = $null
try {
    $ad = Get-NetAdapter -ErrorAction Stop | Where-Object { $_.InterfaceGuid -ieq $targetGuid } | Select-Object -First 1
    if ($null -ne $ad) {
        $idx = [int]$ad.ifIndex
    }
} catch { }

if ($null -eq $idx) {
    $ni = [System.Net.NetworkInformation.NetworkInterface]::GetAllNetworkInterfaces() |
        Where-Object { $_.Id -ieq $targetGuid } |
        Select-Object -First 1
    if ($null -ne $ni) {
        $ipv4Props = $ni.GetIPProperties().GetIPv4Properties()
        if ($null -ne $ipv4Props) {
            $idx = [int]$ipv4Props.Index
        }
    }
}

if ($null -eq $idx) {
    throw "ADAPTER_NOT_FOUND:$targetGuid"
}

try {
    Set-DnsClientServerAddress -InterfaceIndex $idx -ServerAddresses $servers -ErrorAction Stop
} catch {
    $errMsg = $_.Exception.Message
    if ($errMsg -match 'Access is denied|PermissionDenied|UnauthorizedAccess|0x80070005') {
        throw
    }
    $first = $servers[0]
    $out = & netsh.exe interface ipv4 set dnsservers "name=$idx" "source=static" "address=$first" "register=primary" 2>&1
    if ($LASTEXITCODE -ne 0) {
        $msg = ($out | Out-String).Trim()
        if ([string]::IsNullOrWhiteSpace($msg)) {
            $msg = "netsh interface ipv4 set dnsservers failed with exit code $LASTEXITCODE"
        }
        throw $msg
    }
    for ($i = 1; $i -lt $servers.Count; $i++) {
        $extra = $servers[$i]
        $addOut = & netsh.exe interface ipv4 add dnsservers "name=$idx" "address=$extra" ("index=" + ($i + 1)) 2>&1
        if ($LASTEXITCODE -ne 0) {
            $msg = ($addOut | Out-String).Trim()
            if ([string]::IsNullOrWhiteSpace($msg)) {
                $msg = "netsh interface ipv4 add dnsservers failed with exit code $LASTEXITCODE"
            }
            throw $msg
        }
    }
}
"#;

/// Fixed PowerShell script resetting ONLY IPv4 DNS to automatic/DHCP on the adapter matching
/// `$env:DNS_CHANGER_ADAPTER_GUID` using `netsh interface ipv4 set dnsservers`.
/// Never calls `-ResetServerAddresses` and never touches IPv6 DNS or `Tcpip6` registry values.
const RESET_IPV4_DNS_SCRIPT: &str = r#"
try {
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    $OutputEncoding = [System.Text.Encoding]::UTF8
} catch { }
$ErrorActionPreference = 'Stop'
$targetGuid = $env:DNS_CHANGER_ADAPTER_GUID

if ([string]::IsNullOrWhiteSpace($targetGuid)) {
    throw "Missing DNS_CHANGER_ADAPTER_GUID"
}

$idx = $null
try {
    $ad = Get-NetAdapter -ErrorAction Stop | Where-Object { $_.InterfaceGuid -ieq $targetGuid } | Select-Object -First 1
    if ($null -ne $ad) {
        $idx = [int]$ad.ifIndex
    }
} catch { }

if ($null -eq $idx) {
    $ni = [System.Net.NetworkInformation.NetworkInterface]::GetAllNetworkInterfaces() |
        Where-Object { $_.Id -ieq $targetGuid } |
        Select-Object -First 1
    if ($null -ne $ni) {
        $ipv4Props = $ni.GetIPProperties().GetIPv4Properties()
        if ($null -ne $ipv4Props) {
            $idx = [int]$ipv4Props.Index
        }
    }
}

if ($null -eq $idx) {
    throw "ADAPTER_NOT_FOUND:$targetGuid"
}

$output = & netsh.exe interface ipv4 set dnsservers "name=$idx" "source=dhcp" 2>&1
if ($LASTEXITCODE -ne 0) {
    $msg = ($output | Out-String).Trim()
    if ([string]::IsNullOrWhiteSpace($msg)) {
        $msg = "netsh interface ipv4 set dnsservers failed with exit code $LASTEXITCODE"
    }
    throw $msg
}
"#;

/// Read-only PowerShell script capturing the adapter's current IPv6 DNS servers and Tcpip6
/// registry values before/after IPv4 DNS changes to prove IPv6 remained untouched.
const GET_IPV6_DNS_STATE_SCRIPT: &str = r#"
try {
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    $OutputEncoding = [System.Text.Encoding]::UTF8
} catch { }
$ErrorActionPreference = 'Stop'
$targetGuid = $env:DNS_CHANGER_ADAPTER_GUID
if ([string]::IsNullOrWhiteSpace($targetGuid)) {
    throw "Missing DNS_CHANGER_ADAPTER_GUID"
}

$ad = $null
$idx = $null
$guid = $targetGuid
try {
    $ad = Get-NetAdapter -ErrorAction Stop | Where-Object { $_.InterfaceGuid -ieq $targetGuid } | Select-Object -First 1
    if ($null -ne $ad) {
        $idx = [int]$ad.ifIndex
        $guid = [string]$ad.InterfaceGuid
    }
} catch { }

if ($null -eq $idx) {
    $ni = [System.Net.NetworkInformation.NetworkInterface]::GetAllNetworkInterfaces() |
        Where-Object { $_.Id -ieq $targetGuid } |
        Select-Object -First 1
    if ($null -eq $ni) {
        'null'
        exit 0
    }
    $guid = [string]$ni.Id
    try {
        $ipv6Props = $ni.GetIPProperties().GetIPv6Properties()
        if ($null -ne $ipv6Props) {
            $idx = [int]$ipv6Props.Index
        }
    } catch { }
}

$dnsServers = @()
if ($null -ne $idx) {
    try {
        $dnsObj = Get-DnsClientServerAddress -InterfaceIndex $idx -AddressFamily IPv6 -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($null -ne $dnsObj -and $null -ne $dnsObj.ServerAddresses) {
            $dnsServers = @($dnsObj.ServerAddresses | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { [string]$_ })
        }
    } catch { }
}

$ipv6RegPath = "HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip6\Parameters\Interfaces\$guid"
$regNameServer = $null
$regProfileNameServer = $null
try {
    if (Test-Path -LiteralPath $ipv6RegPath -ErrorAction SilentlyContinue) {
        $regProps = Get-ItemProperty -LiteralPath $ipv6RegPath -ErrorAction SilentlyContinue
        if ($null -ne $regProps) {
            if ($null -ne $regProps.PSObject.Properties['NameServer']) {
                $regNameServer = [string]$regProps.NameServer
            }
            if ($null -ne $regProps.PSObject.Properties['ProfileNameServer']) {
                $regProfileNameServer = [string]$regProps.ProfileNameServer
            }
        }
    }
} catch { }

$result = [PSCustomObject]@{
    serverAddresses = @($dnsServers)
    staticNameServerReg = $regNameServer
    profileNameServerReg = $regProfileNameServer
}

ConvertTo-Json -InputObject $result -Depth 4 -Compress
"#;

/// Emergency recovery script invoked ONLY if post-operation verification detects an unexpected
/// IPv6 modification after an IPv4 operation. Never invoked during normal IPv4 operations.
const RESTORE_IPV6_DNS_EMERGENCY_SCRIPT: &str = r#"
try {
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    $OutputEncoding = [System.Text.Encoding]::UTF8
} catch { }
$ErrorActionPreference = 'Stop'
$targetGuid = $env:DNS_CHANGER_ADAPTER_GUID
$mode = $env:DNS_CHANGER_IPV6_MODE
$serversRaw = $env:DNS_CHANGER_IPV6_SERVERS

if ([string]::IsNullOrWhiteSpace($targetGuid)) {
    throw "Missing DNS_CHANGER_ADAPTER_GUID"
}

$ad = Get-NetAdapter | Where-Object { $_.InterfaceGuid -ieq $targetGuid } | Select-Object -First 1
if ($null -eq $ad) {
    throw "ADAPTER_NOT_FOUND:$targetGuid"
}

$idx = [int]$ad.ifIndex
if ($mode -eq 'automatic') {
    $output = & netsh.exe interface ipv6 set dnsservers "name=$idx" "source=dhcp" 2>&1
    if ($LASTEXITCODE -ne 0) {
        $msg = ($output | Out-String).Trim()
        throw "IPv6 recovery failed: $msg"
    }
} else {
    $servers = @($serversRaw -split ',' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    if ($servers.Count -gt 0) {
        Set-DnsClientServerAddress -InterfaceIndex $idx -ServerAddresses $servers -ErrorAction Stop
    }
}
"#;

pub struct PowerShellNetworkBackend;

impl PowerShellNetworkBackend {
    pub fn new() -> Self {
        Self
    }

    #[cfg(windows)]
    fn resolve_powershell_executable() -> std::path::PathBuf {
        if let Some(sys_root) = std::env::var_os("SystemRoot").or_else(|| std::env::var_os("WINDIR"))
        {
            let candidate = std::path::PathBuf::from(sys_root)
                .join("System32")
                .join("WindowsPowerShell")
                .join("v1.0")
                .join("powershell.exe");
            if candidate.is_file() {
                return candidate;
            }
        }
        std::path::PathBuf::from("powershell.exe")
    }

    fn encode_script(script: &str) -> String {
        let utf16_bytes: Vec<u8> = script
            .encode_utf16()
            .flat_map(|unit| unit.to_le_bytes())
            .collect();
        BASE64.encode(utf16_bytes)
    }

    fn truncate_stderr_preview(raw_stderr: &str) -> String {
        raw_stderr
            .trim_start_matches('\u{feff}')
            .trim()
            .chars()
            .take(STDERR_PREVIEW_MAX_CHARS)
            .collect()
    }

    fn map_powershell_timeout(script_name: &str, timeout_secs: u64, raw_stderr: &str) -> AppError {
        let stderr_preview = Self::truncate_stderr_preview(raw_stderr);
        let detail = if stderr_preview.is_empty() {
            format!("{script_name} timed out after {timeout_secs}s")
        } else {
            format!("{script_name} timed out after {timeout_secs}s; stderr: {stderr_preview}")
        };
        AppError::PowershellTimeout {
            timeout_secs,
            detail,
        }
    }

    fn map_powershell_failure(
        script_name: &str,
        exit_code: i32,
        raw_stderr: &str,
        adapter_guid: Option<&str>,
    ) -> AppError {
        let stderr_preview = Self::truncate_stderr_preview(raw_stderr);
        let lower = stderr_preview.to_ascii_lowercase();

        let has_blocked_phrase = [
            "malicious content",
            "blocked by your antivirus",
            "has been blocked",
            "execution of scripts is disabled",
            "running scripts is disabled",
        ]
        .iter()
        .any(|phrase| lower.contains(phrase));

        let is_missing_networking_tool = lower.contains("is not recognized")
            && [
                "get-netadapter",
                "get-netipaddress",
                "get-netroute",
                "get-netipinterface",
                "get-dnsclientserveraddress",
                "set-dnsclientserveraddress",
                "get-netipconfiguration",
                "netsh",
            ]
            .iter()
            .any(|tool| lower.contains(tool));

        if has_blocked_phrase || is_missing_networking_tool {
            return AppError::PowershellBlocked {
                detail: format!("{script_name} blocked (exit {exit_code}): {stderr_preview}"),
            };
        }

        if lower.contains("access is denied")
            || lower.contains("requires elevation")
            || lower.contains("0x80070005")
            || lower.contains("permissiondenied")
            || lower.contains("unauthorizedaccess")
        {
            return AppError::AdminAccessDenied {
                detail: stderr_preview,
            };
        }

        if raw_stderr.contains("ADAPTER_NOT_FOUND") {
            return AppError::AdapterNotFound {
                guid: adapter_guid.unwrap_or_default().to_string(),
            };
        }

        AppError::PowershellExecutionFailed {
            detail: format!("{script_name} failed (exit {exit_code}): {stderr_preview}"),
        }
    }

    fn run_fixed_script(
        &self,
        script_name: &str,
        script: &'static str,
        env_vars: &[(&'static str, &str)],
        timeout_secs: u64,
    ) -> Result<String, AppError> {
        #[cfg(not(windows))]
        {
            let _ = (script_name, script, env_vars, timeout_secs);
            return Err(AppError::UnsupportedPlatform);
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;

            let encoded = Self::encode_script(script);
            let ps_exe = Self::resolve_powershell_executable();
            let mut cmd = Command::new(&ps_exe);
            cmd.args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-EncodedCommand",
                &encoded,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW);

            for (key, val) in env_vars {
                cmd.env(key, val);
            }

            debug!(
                script = script_name,
                timeout_secs,
                "Executing controlled PowerShell networking script"
            );

            let mut child = cmd.spawn().map_err(|e| AppError::PowershellExecutionFailed {
                detail: format!("Failed to spawn powershell.exe for {script_name}: {e}"),
            })?;

            let mut stdout_pipe = child.stdout.take().ok_or_else(|| {
                AppError::PowershellExecutionFailed {
                    detail: "Failed to capture PowerShell stdout".into(),
                }
            })?;
            let mut stderr_pipe = child.stderr.take().ok_or_else(|| {
                AppError::PowershellExecutionFailed {
                    detail: "Failed to capture PowerShell stderr".into(),
                }
            })?;

            let stdout_handle = thread::spawn(move || -> Result<Vec<u8>, std::io::Error> {
                let mut buf = Vec::new();
                stdout_pipe.read_to_end(&mut buf)?;
                Ok(buf)
            });
            let stderr_handle = thread::spawn(move || -> Result<Vec<u8>, std::io::Error> {
                let mut buf = Vec::new();
                stderr_pipe.read_to_end(&mut buf)?;
                Ok(buf)
            });

            let timeout = Duration::from_secs(timeout_secs);
            let started = Instant::now();
            let status = loop {
                match child.try_wait() {
                    Ok(Some(status)) => break status,
                    Ok(None) => {
                        if started.elapsed() >= timeout {
                            let _ = child.kill();
                            let _ = child.wait();
                            let _ = stdout_handle.join();
                            let stderr_text = stderr_handle
                                .join()
                                .ok()
                                .and_then(Result::ok)
                                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                                .unwrap_or_default();
                            let stderr_preview = Self::truncate_stderr_preview(&stderr_text);
                            warn!(
                                script = script_name,
                                timeout_secs,
                                stderr_preview = %stderr_preview,
                                "Killed timed-out PowerShell process"
                            );
                            return Err(Self::map_powershell_timeout(
                                script_name,
                                timeout_secs,
                                &stderr_text,
                            ));
                        }
                        thread::sleep(Duration::from_millis(25));
                    }
                    Err(e) => {
                        let _ = child.kill();
                        return Err(AppError::PowershellExecutionFailed {
                            detail: format!("Failed while waiting for {script_name}: {e}"),
                        });
                    }
                }
            };

            let stdout_bytes = stdout_handle
                .join()
                .map_err(|_| AppError::PowershellExecutionFailed {
                    detail: format!("PowerShell stdout reader thread panicked for {script_name}"),
                })?
                .map_err(|e| AppError::PowershellExecutionFailed {
                    detail: format!("Failed to read PowerShell stdout for {script_name}: {e}"),
                })?;

            let stderr_bytes = stderr_handle
                .join()
                .map_err(|_| AppError::PowershellExecutionFailed {
                    detail: format!("PowerShell stderr reader thread panicked for {script_name}"),
                })?
                .map_err(|e| AppError::PowershellExecutionFailed {
                    detail: format!("Failed to read PowerShell stderr for {script_name}: {e}"),
                })?;

            let stdout = String::from_utf8_lossy(&stdout_bytes)
                .trim_start_matches('\u{feff}')
                .into_owned();
            let stderr = String::from_utf8_lossy(&stderr_bytes)
                .trim_start_matches('\u{feff}')
                .into_owned();

            if !status.success() {
                let code = status.code().unwrap_or(-1);
                let stderr_preview = Self::truncate_stderr_preview(&stderr);
                error!(
                    script = script_name,
                    exit_code = code,
                    stderr = %stderr_preview,
                    "PowerShell script returned non-zero exit status"
                );

                let adapter_guid = env_vars
                    .iter()
                    .find(|(k, _)| *k == "DNS_CHANGER_ADAPTER_GUID")
                    .map(|(_, v)| *v);
                return Err(Self::map_powershell_failure(
                    script_name,
                    code,
                    &stderr,
                    adapter_guid,
                ));
            }

            Ok(stdout)
        }
    }
}

impl Default for PowerShellNetworkBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkBackend for PowerShellNetworkBackend {
    fn list_adapters(&self) -> Result<Vec<NetworkAdapter>, AppError> {
        let stdout = self.run_fixed_script(
            "get_all_adapters",
            GET_ALL_ADAPTERS_SCRIPT,
            &[],
            POWERSHELL_QUERY_TIMEOUT_SECS,
        )?;
        let adapters = parse_adapters_json(&stdout)?;
        debug!(count = adapters.len(), "Enumerated Windows network adapters");
        Ok(adapters)
    }

    fn get_adapter_by_guid(&self, adapter_guid: &str) -> Result<Option<NetworkAdapter>, AppError> {
        let canonical = validate_adapter_guid(adapter_guid)?;
        let stdout = self.run_fixed_script(
            "get_adapter_by_guid",
            GET_ADAPTER_BY_GUID_SCRIPT,
            &[("DNS_CHANGER_ADAPTER_GUID", &canonical)],
            POWERSHELL_QUERY_TIMEOUT_SECS,
        )?;
        parse_single_adapter_json(&stdout)
    }

    fn set_ipv4_dns(&self, adapter_guid: &str, servers: &[Ipv4Addr]) -> Result<(), AppError> {
        let canonical = validate_adapter_guid(adapter_guid)?;
        if servers.is_empty() || servers.len() > MAX_IPV4_DNS_SERVERS {
            return Err(AppError::InvalidDnsAddress {
                address: servers
                    .iter()
                    .map(|ip| ip.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                reason: format!("expected between 1 and {MAX_IPV4_DNS_SERVERS} IPv4 DNS addresses"),
            });
        }

        let csv = servers
            .iter()
            .map(|ip| ip.to_string())
            .collect::<Vec<_>>()
            .join(",");

        self.run_fixed_script(
            "set_ipv4_dns",
            SET_IPV4_DNS_SCRIPT,
            &[
                ("DNS_CHANGER_ADAPTER_GUID", &canonical),
                ("DNS_CHANGER_DNS_SERVERS", &csv),
            ],
            POWERSHELL_MUTATION_TIMEOUT_SECS,
        )?;

        Ok(())
    }

    fn reset_ipv4_dns_to_automatic(&self, adapter_guid: &str) -> Result<(), AppError> {
        let canonical = validate_adapter_guid(adapter_guid)?;
        self.run_fixed_script(
            "reset_ipv4_dns",
            RESET_IPV4_DNS_SCRIPT,
            &[("DNS_CHANGER_ADAPTER_GUID", &canonical)],
            POWERSHELL_MUTATION_TIMEOUT_SECS,
        )?;
        Ok(())
    }

    fn get_ipv6_dns_state(&self, adapter_guid: &str) -> Result<Option<Ipv6DnsSnapshot>, AppError> {
        let canonical = validate_adapter_guid(adapter_guid)?;
        let stdout = self.run_fixed_script(
            "get_ipv6_dns_state",
            GET_IPV6_DNS_STATE_SCRIPT,
            &[("DNS_CHANGER_ADAPTER_GUID", &canonical)],
            POWERSHELL_MUTATION_TIMEOUT_SECS,
        )?;
        parse_ipv6_dns_snapshot_json(&stdout)
    }

    fn restore_ipv6_dns_state(
        &self,
        adapter_guid: &str,
        snapshot: &Ipv6DnsSnapshot,
    ) -> Result<(), AppError> {
        let canonical = validate_adapter_guid(adapter_guid)?;
        let mode_str = match snapshot.dns_mode {
            DnsMode::Automatic => "automatic",
            DnsMode::Static => "static",
        };
        let servers_str = snapshot
            .profile_name_server_reg
            .as_deref()
            .or(snapshot.static_name_server_reg.as_deref())
            .map(ToString::to_string)
            .unwrap_or_else(|| snapshot.server_addresses.join(","));

        self.run_fixed_script(
            "restore_ipv6_dns_emergency",
            RESTORE_IPV6_DNS_EMERGENCY_SCRIPT,
            &[
                ("DNS_CHANGER_ADAPTER_GUID", &canonical),
                ("DNS_CHANGER_IPV6_MODE", mode_str),
                ("DNS_CHANGER_IPV6_SERVERS", &servers_str),
            ],
            POWERSHELL_MUTATION_TIMEOUT_SECS,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_utf16le_base64_for_powershell_encoded_command() {
        let encoded = PowerShellNetworkBackend::encode_script("Write-Output 'ok'");
        assert!(!encoded.is_empty());
        let decoded = BASE64.decode(encoded).unwrap();
        assert_eq!(decoded.len() % 2, 0);
    }

    #[test]
    fn reset_script_is_strictly_ipv4_only_and_never_touches_ipv6_or_reset_server_addresses() {
        assert!(RESET_IPV4_DNS_SCRIPT.contains("netsh.exe interface ipv4 set dnsservers"));
        assert!(!RESET_IPV4_DNS_SCRIPT.contains("-ResetServerAddresses"));
        assert!(!RESET_IPV4_DNS_SCRIPT.contains("Tcpip6"));
        assert!(!RESET_IPV4_DNS_SCRIPT.contains("ipv6"));
    }

    #[test]
    fn set_script_validates_ipv4_inter_network_family_and_multi_server_csv() {
        assert!(SET_IPV4_DNS_SCRIPT.contains("AddressFamily]::InterNetwork"));
        assert!(SET_IPV4_DNS_SCRIPT.contains("DNS_CHANGER_DNS_SERVERS"));
        assert!(SET_IPV4_DNS_SCRIPT.contains("-gt 8"));
        assert!(!SET_IPV4_DNS_SCRIPT.contains("Tcpip6"));
        assert!(!SET_IPV4_DNS_SCRIPT.contains("-ResetServerAddresses"));
    }

    #[test]
    fn all_powershell_scripts_configure_utf8_output_encoding() {
        let expected = "try {\n    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8\n    $OutputEncoding = [System.Text.Encoding]::UTF8\n} catch { }";
        for script in [
            GET_ALL_ADAPTERS_SCRIPT,
            GET_ADAPTER_BY_GUID_SCRIPT,
            SET_IPV4_DNS_SCRIPT,
            RESET_IPV4_DNS_SCRIPT,
            GET_IPV6_DNS_STATE_SCRIPT,
            RESTORE_IPV6_DNS_EMERGENCY_SCRIPT,
        ] {
            assert!(
                script.contains(expected),
                "Expected PowerShell script to wrap UTF-8 output encoding setup in try/catch"
            );
        }
    }

    #[test]
    fn query_and_mutation_timeout_constants_differ_appropriately() {
        assert_eq!(POWERSHELL_QUERY_TIMEOUT_SECS, 45);
        assert_eq!(POWERSHELL_MUTATION_TIMEOUT_SECS, 15);
    }

    #[test]
    fn maps_blocked_antivirus_and_missing_cmdlet_stderr_to_powershell_blocked() {
        for msg in [
            "This script contains malicious content and has been blocked by your antivirus software.",
            "Program 'powershell.exe' failed to run: blocked by your antivirus.",
            "This script has been blocked by your organization's policy.",
            "File C:\\temp\\run.ps1 cannot be loaded because execution of scripts is disabled on this system.",
            "File C:\\temp\\run.ps1 cannot be loaded because running scripts is disabled on this system.",
            "The term 'Get-NetAdapter' is not recognized as the name of a cmdlet.",
            "The term 'Set-DnsClientServerAddress' is not recognized as the name of a cmdlet.",
            "netsh : The term 'netsh' is not recognized as the name of a cmdlet.",
        ] {
            let err = PowerShellNetworkBackend::map_powershell_failure(
                "get_all_adapters",
                1,
                msg,
                None,
            );
            assert!(
                matches!(err, AppError::PowershellBlocked { .. }),
                "Expected PowershellBlocked for stderr: {msg}"
            );
            assert!(err.to_string().contains("get_all_adapters"));
        }
    }

    #[test]
    fn maps_unrelated_is_not_recognized_error_to_execution_failed_with_stderr() {
        let msg = "The term 'Foo-Bar' is not recognized as the name of a cmdlet.";
        let err = PowerShellNetworkBackend::map_powershell_failure(
            "get_all_adapters",
            1,
            msg,
            None,
        );
        match err {
            AppError::PowershellExecutionFailed { detail } => {
                assert!(detail.contains("Foo-Bar"));
                assert!(detail.contains("is not recognized"));
            }
            other => panic!("Expected PowershellExecutionFailed, got {other:?}"),
        }
    }

    #[test]
    fn maps_access_denied_adapter_not_found_and_truncates_stderr_to_500_chars() {
        let long_stderr = "x".repeat(800);
        let fail_err = PowerShellNetworkBackend::map_powershell_failure(
            "get_all_adapters",
            1,
            &long_stderr,
            None,
        );
        match fail_err {
            AppError::PowershellExecutionFailed { detail } => {
                assert!(detail.contains(&"x".repeat(500)));
                assert!(!detail.contains(&"x".repeat(501)));
            }
            other => panic!("Expected PowershellExecutionFailed, got {other:?}"),
        }

        let timeout_err = PowerShellNetworkBackend::map_powershell_timeout(
            "get_adapter_by_guid",
            45,
            &long_stderr,
        );
        match timeout_err {
            AppError::PowershellTimeout {
                timeout_secs,
                detail,
            } => {
                assert_eq!(timeout_secs, 45);
                assert!(detail.contains(&"x".repeat(500)));
                assert!(!detail.contains(&"x".repeat(501)));
            }
            other => panic!("Expected PowershellTimeout, got {other:?}"),
        }

        let denied = PowerShellNetworkBackend::map_powershell_failure(
            "set_ipv4_dns",
            1,
            "Access is denied (0x80070005)",
            None,
        );
        assert!(matches!(denied, AppError::AdminAccessDenied { .. }));

        let missing = PowerShellNetworkBackend::map_powershell_failure(
            "set_ipv4_dns",
            1,
            "ADAPTER_NOT_FOUND:{11111111-2222-3333-4444-555555555555}",
            Some("{11111111-2222-3333-4444-555555555555}"),
        );
        assert!(matches!(missing, AppError::AdapterNotFound { .. }));
    }

    #[test]
    fn adapter_query_scripts_filter_loopback_tunnel_and_non_ipv4_in_dotnet_fallback() {
        for script in [GET_ALL_ADAPTERS_SCRIPT, GET_ADAPTER_BY_GUID_SCRIPT] {
            assert!(script.contains("GetAllNetworkInterfaces()"));
            assert!(script.contains("$niType -eq 'Loopback' -or $niType -eq 'Tunnel'"));
            assert!(script.contains("GetIPv4Properties()"));
            assert!(script.contains("if ($null -eq $ipv4Props)"));
            assert!(script.contains("[uint32]::TryParse"));
        }
    }

    #[test]
    fn dotnet_fallback_still_queries_tcpip_interface_signals() {
        let all_fallback = GET_ALL_ADAPTERS_SCRIPT
            .split("GetAllNetworkInterfaces()")
            .nth(1)
            .expect("Expected .NET fallback section in GET_ALL_ADAPTERS_SCRIPT");
        assert!(all_fallback.contains("Get-TcpipInterfaceSignals -Guid $guid"));

        let single_fallback = GET_ADAPTER_BY_GUID_SCRIPT
            .split("GetAllNetworkInterfaces()")
            .nth(1)
            .expect("Expected .NET fallback section in GET_ADAPTER_BY_GUID_SCRIPT");
        assert!(single_fallback.contains("Get-TcpipInterfaceSignals -Guid $guid"));
    }
}
