use crate::models::{DnsProvider, DnsProviderCatalog};

pub fn built_in_providers() -> Vec<DnsProvider> {
    vec![
        DnsProvider {
            id: "cloudflare".into(),
            name: "Cloudflare".into(),
            primary_dns: "1.1.1.1".into(),
            secondary_dns: Some("1.0.0.1".into()),
            is_built_in: true,
            category: Some("Speed & Privacy".into()),
            description: Some("Fast, privacy-first DNS resolver by Cloudflare.".into()),
        },
        DnsProvider {
            id: "google".into(),
            name: "Google".into(),
            primary_dns: "8.8.8.8".into(),
            secondary_dns: Some("8.8.4.4".into()),
            is_built_in: true,
            category: Some("General".into()),
            description: Some("Google Public DNS global anycast resolver.".into()),
        },
        DnsProvider {
            id: "quad9".into(),
            name: "Quad9".into(),
            primary_dns: "9.9.9.9".into(),
            secondary_dns: Some("149.112.112.112".into()),
            is_built_in: true,
            category: Some("Security".into()),
            description: Some("Security-focused DNS blocking known malicious domains.".into()),
        },
        DnsProvider {
            id: "opendns".into(),
            name: "OpenDNS".into(),
            primary_dns: "208.67.222.222".into(),
            secondary_dns: Some("208.67.220.220".into()),
            is_built_in: true,
            category: Some("Reliability".into()),
            description: Some("Cisco OpenDNS reliable standard resolver.".into()),
        },
        DnsProvider {
            id: "adguard".into(),
            name: "AdGuard".into(),
            primary_dns: "94.140.14.14".into(),
            secondary_dns: Some("94.140.15.15".into()),
            is_built_in: true,
            category: Some("Ad Blocking".into()),
            description: Some("DNS-level ad and tracker blocking by AdGuard.".into()),
        },
        DnsProvider {
            id: "cleanbrowsing-adult".into(),
            name: "CleanBrowsing Adult".into(),
            primary_dns: "185.228.168.10".into(),
            secondary_dns: Some("185.228.169.11".into()),
            is_built_in: true,
            category: Some("Content Filter".into()),
            description: Some("Blocks adult content and malicious domains.".into()),
        },
        DnsProvider {
            id: "cleanbrowsing-family".into(),
            name: "CleanBrowsing Family".into(),
            primary_dns: "185.228.168.168".into(),
            secondary_dns: Some("185.228.169.168".into()),
            is_built_in: true,
            category: Some("Family Safety".into()),
            description: Some("Family filter blocking adult content, proxies, and malware.".into()),
        },
        DnsProvider {
            id: "dns-watch".into(),
            name: "DNS.Watch".into(),
            primary_dns: "84.200.69.80".into(),
            secondary_dns: Some("84.200.70.40".into()),
            is_built_in: true,
            category: Some("Uncensored".into()),
            description: Some("Zero-logging, uncensored German DNS resolver.".into()),
        },
    ]
}

pub fn build_provider_catalog(custom_providers: Vec<DnsProvider>) -> DnsProviderCatalog {
    DnsProviderCatalog {
        built_in: built_in_providers(),
        custom: custom_providers
            .into_iter()
            .map(|mut p| {
                p.is_built_in = false;
                p
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dns::validation::validate_ipv4;

    #[test]
    fn built_in_providers_match_exact_specification() {
        let providers = built_in_providers();
        assert_eq!(providers.len(), 8);

        let expected = [
            ("Cloudflare", "1.1.1.1", "1.0.0.1"),
            ("Google", "8.8.8.8", "8.8.4.4"),
            ("Quad9", "9.9.9.9", "149.112.112.112"),
            ("OpenDNS", "208.67.222.222", "208.67.220.220"),
            ("AdGuard", "94.140.14.14", "94.140.15.15"),
            ("CleanBrowsing Adult", "185.228.168.10", "185.228.169.11"),
            ("CleanBrowsing Family", "185.228.168.168", "185.228.169.168"),
            ("DNS.Watch", "84.200.69.80", "84.200.70.40"),
        ];

        for (i, (name, primary, secondary)) in expected.iter().enumerate() {
            assert_eq!(providers[i].name, *name);
            assert_eq!(providers[i].primary_dns, *primary);
            assert_eq!(providers[i].secondary_dns.as_deref(), Some(*secondary));
            assert!(providers[i].is_built_in);
            assert!(validate_ipv4(&providers[i].primary_dns).is_ok());
            assert!(validate_ipv4(providers[i].secondary_dns.as_deref().unwrap()).is_ok());
        }
    }

    #[test]
    fn keeps_built_in_separate_from_custom_providers() {
        let custom = vec![DnsProvider {
            id: "custom-home".into(),
            name: "Home Pi-hole".into(),
            primary_dns: "192.168.1.2".into(),
            secondary_dns: Some("1.1.1.1".into()),
            is_built_in: true, // should be forced to false
            category: Some("Custom".into()),
            description: None,
        }];
        let catalog = build_provider_catalog(custom);
        assert_eq!(catalog.built_in.len(), 8);
        assert_eq!(catalog.custom.len(), 1);
        assert!(!catalog.custom[0].is_built_in);
    }
}
