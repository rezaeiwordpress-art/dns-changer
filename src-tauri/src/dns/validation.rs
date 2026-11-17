use std::net::Ipv4Addr;
use std::str::FromStr;
use uuid::Uuid;

use crate::errors::AppError;

/// Strictly validates a single IPv4 DNS server address using Rust's standard `Ipv4Addr`.
/// Rejects empty strings, whitespace, invalid octets, unspecified (`0.0.0.0`),
/// broadcast (`255.255.255.255`), and multicast (`224.0.0.0/4`) addresses.
pub fn validate_ipv4(input: &str) -> Result<Ipv4Addr, AppError> {
    if input.is_empty() || input != input.trim() {
        return Err(AppError::InvalidDnsAddress {
            address: input.to_string(),
            reason: "address must not be empty or contain surrounding whitespace".into(),
        });
    }

    let addr = Ipv4Addr::from_str(input).map_err(|_| AppError::InvalidDnsAddress {
        address: input.to_string(),
        reason: "must be a valid dotted-quad IPv4 address (e.g. 1.1.1.1)".into(),
    })?;

    if addr.is_unspecified() {
        return Err(AppError::InvalidDnsAddress {
            address: input.to_string(),
            reason: "unspecified address 0.0.0.0 cannot be used as a DNS server".into(),
        });
    }

    if addr.is_broadcast() {
        return Err(AppError::InvalidDnsAddress {
            address: input.to_string(),
            reason: "broadcast address 255.255.255.255 cannot be used as a DNS server".into(),
        });
    }

    if addr.is_multicast() {
        return Err(AppError::InvalidDnsAddress {
            address: input.to_string(),
            reason: "multicast addresses cannot be used as a DNS server".into(),
        });
    }

    Ok(addr)
}

pub const MAX_IPV4_DNS_SERVERS: usize = 8;

/// Validates a non-empty list of IPv4 DNS server addresses (1..=8 servers).
pub fn validate_dns_list<S: AsRef<str>>(servers: &[S]) -> Result<Vec<Ipv4Addr>, AppError> {
    if servers.is_empty() {
        return Err(AppError::InvalidDnsAddress {
            address: String::new(),
            reason: "at least one IPv4 DNS address is required".into(),
        });
    }
    if servers.len() > MAX_IPV4_DNS_SERVERS {
        return Err(AppError::InvalidDnsAddress {
            address: servers
                .iter()
                .map(|s| s.as_ref())
                .collect::<Vec<_>>()
                .join(","),
            reason: format!("at most {MAX_IPV4_DNS_SERVERS} IPv4 DNS addresses are supported"),
        });
    }

    servers
        .iter()
        .map(|s| validate_ipv4(s.as_ref()))
        .collect()
}

/// Validates primary (required) and secondary (optional) IPv4 DNS addresses.
/// Note: If `Some("")` is explicitly supplied for secondary, it is rejected rather than silently ignored.
/// Rejects identical primary and secondary DNS addresses.
pub fn validate_dns_pair(
    primary: &str,
    secondary: Option<&str>,
) -> Result<Vec<Ipv4Addr>, AppError> {
    let primary_ip = validate_ipv4(primary)?;
    let mut servers = vec![primary_ip];

    if let Some(sec) = secondary {
        let secondary_ip = validate_ipv4(sec)?;
        if secondary_ip == primary_ip {
            return Err(AppError::InvalidDnsAddress {
                address: sec.to_string(),
                reason: "secondary DNS address must differ from primary DNS address".into(),
            });
        }
        servers.push(secondary_ip);
    }

    Ok(servers)
}

/// Validates and canonicalizes a Windows interface GUID into uppercase braced format:
/// `{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}`.
pub fn validate_adapter_guid(input: &str) -> Result<String, AppError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidAdapterGuid {
            guid: input.to_string(),
        });
    }

    let stripped = trimmed
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(trimmed);

    // Ensure braces were balanced (reject "{uuid" or "uuid}")
    if stripped.contains('{') || stripped.contains('}') {
        return Err(AppError::InvalidAdapterGuid {
            guid: input.to_string(),
        });
    }

    let parsed = Uuid::parse_str(stripped).map_err(|_| AppError::InvalidAdapterGuid {
        guid: input.to_string(),
    })?;

    Ok(format!("{{{}}}", parsed.hyphenated().to_string().to_uppercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_ipv4_addresses() {
        assert_eq!(validate_ipv4("1.1.1.1").unwrap(), Ipv4Addr::new(1, 1, 1, 1));
        assert_eq!(validate_ipv4("8.8.4.4").unwrap(), Ipv4Addr::new(8, 8, 4, 4));
        assert_eq!(
            validate_ipv4("192.168.1.1").unwrap(),
            Ipv4Addr::new(192, 168, 1, 1)
        );
        assert_eq!(
            validate_ipv4("127.0.0.1").unwrap(),
            Ipv4Addr::new(127, 0, 0, 1)
        );
    }

    #[test]
    fn rejects_invalid_ipv4_addresses() {
        for bad in [
            "",
            "   ",
            " 1.1.1.1",
            "1.1.1.1 ",
            "256.1.1.1",
            "1.1.1.999",
            "1.1.1",
            "1.1.1.1.1",
            "01.1.1.1",
            "abc.def.ghi.jkl",
            "1.1.1.-1",
            "0.0.0.0",
            "255.255.255.255",
            "224.0.0.1",
            "2001:4860:4860::8888",
            "1.1.1.1; whoami",
        ] {
            assert!(
                validate_ipv4(bad).is_err(),
                "expected '{bad}' to be rejected"
            );
        }
    }

    #[test]
    fn validates_dns_pairs_and_rejects_identical_pair() {
        let pair = validate_dns_pair("1.1.1.1", Some("1.0.0.1")).unwrap();
        assert_eq!(
            pair,
            vec![Ipv4Addr::new(1, 1, 1, 1), Ipv4Addr::new(1, 0, 0, 1)]
        );

        let single = validate_dns_pair("8.8.8.8", None).unwrap();
        assert_eq!(single, vec![Ipv4Addr::new(8, 8, 8, 8)]);

        assert!(validate_dns_pair("1.1.1.1", Some("1.1.1.1")).is_err());
        assert!(validate_dns_pair("8.8.8.8", Some("")).is_err());
        assert!(validate_dns_pair("", Some("8.8.4.4")).is_err());
    }

    #[test]
    fn validates_dns_lists_up_to_eight_servers() {
        let empty: [&str; 0] = [];
        assert!(validate_dns_list(&empty).is_err());

        let four = ["1.1.1.1", "1.0.0.1", "8.8.8.8", "8.8.4.4"];
        assert_eq!(validate_dns_list(&four).unwrap().len(), 4);

        let eight = [
            "1.1.1.1", "1.0.0.1", "8.8.8.8", "8.8.4.4", "9.9.9.9", "149.112.112.112",
            "208.67.222.222", "208.67.220.220",
        ];
        assert_eq!(validate_dns_list(&eight).unwrap().len(), 8);

        let nine = [
            "1.1.1.1", "1.0.0.1", "8.8.8.8", "8.8.4.4", "9.9.9.9", "149.112.112.112",
            "208.67.222.222", "208.67.220.220", "94.140.14.14",
        ];
        assert!(validate_dns_list(&nine).is_err());
    }

    #[test]
    fn validates_and_canonicalizes_adapter_guids() {
        let raw = "6f9619ff-8b86-d011-b42d-00cf4fc964ff";
        assert_eq!(
            validate_adapter_guid(raw).unwrap(),
            "{6F9619FF-8B86-D011-B42D-00CF4FC964FF}"
        );
        assert_eq!(
            validate_adapter_guid(&format!("{{{raw}}}")).unwrap(),
            "{6F9619FF-8B86-D011-B42D-00CF4FC964FF}"
        );

        for bad in [
            "",
            "Ethernet",
            "{6f9619ff-8b86-d011-b42d-00cf4fc964ff",
            "6f9619ff-8b86-d011-b42d-00cf4fc964ff}",
            "not-a-guid",
            "{6F9619FF-8B86-D011-B42D-00CF4FC964FF}; Remove-Item",
        ] {
            assert!(
                validate_adapter_guid(bad).is_err(),
                "expected GUID '{bad}' to be rejected"
            );
        }
    }
}
