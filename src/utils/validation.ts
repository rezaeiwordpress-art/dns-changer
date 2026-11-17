import type { DnsProvider } from '../types/dns';

const IPV4_REGEX = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/;

export type Ipv4ValidationErrorKey =
  | 'emptyOrWhitespace'
  | 'invalidFormat'
  | 'leadingZeros'
  | 'octetOutOfRange'
  | 'unspecifiedAddress'
  | 'broadcastAddress'
  | 'multicastAddress';

export function normalizeDnsDigits(value: string): string {
  return value.replace(/[\u06F0-\u06F9\u0660-\u0669]/g, (ch) => {
    const code = ch.charCodeAt(0);
    if (code >= 0x06f0 && code <= 0x06f9) {
      return String(code - 0x06f0);
    }
    return String(code - 0x0660);
  });
}

export function validateIpv4InputKey(
  value: string,
): Ipv4ValidationErrorKey | null {
  if (!value || value !== value.trim()) {
    return 'emptyOrWhitespace';
  }

  const normalized = normalizeDnsDigits(value);
  const match = IPV4_REGEX.exec(normalized);
  if (!match) {
    return 'invalidFormat';
  }

  const octets = match.slice(1, 5);
  for (const octetStr of octets) {
    if (octetStr.length > 1 && octetStr.startsWith('0')) {
      return 'leadingZeros';
    }
    const num = Number(octetStr);
    if (!Number.isInteger(num) || num < 0 || num > 255) {
      return 'octetOutOfRange';
    }
  }

  if (normalized === '0.0.0.0') {
    return 'unspecifiedAddress';
  }
  if (normalized === '255.255.255.255') {
    return 'broadcastAddress';
  }
  const firstOctet = Number(octets[0]);
  if (firstOctet >= 224 && firstOctet <= 239) {
    return 'multicastAddress';
  }

  return null;
}

export function hasIdenticalDnsPair(
  primary: string,
  secondary: string,
): boolean {
  const s = normalizeDnsDigits(secondary.trim());
  if (!s) return false;
  const p = normalizeDnsDigits(primary.trim());
  return p === s;
}

/**
 * Reference catalog matching Rust `dns::providers::built_in_providers()`.
 */
export const BUILT_IN_DNS_PROVIDERS_REFERENCE: DnsProvider[] = [
  {
    id: 'cloudflare',
    name: 'Cloudflare',
    primaryDns: '1.1.1.1',
    secondaryDns: '1.0.0.1',
    isBuiltIn: true,
    category: 'Speed & Privacy',
    description: 'Fast, privacy-first DNS resolver by Cloudflare.',
  },
  {
    id: 'google',
    name: 'Google',
    primaryDns: '8.8.8.8',
    secondaryDns: '8.8.4.4',
    isBuiltIn: true,
    category: 'General',
    description: 'Google Public DNS global anycast resolver.',
  },
  {
    id: 'quad9',
    name: 'Quad9',
    primaryDns: '9.9.9.9',
    secondaryDns: '149.112.112.112',
    isBuiltIn: true,
    category: 'Security',
    description: 'Security-focused DNS blocking known malicious domains.',
  },
  {
    id: 'opendns',
    name: 'OpenDNS',
    primaryDns: '208.67.222.222',
    secondaryDns: '208.67.220.220',
    isBuiltIn: true,
    category: 'Reliability',
    description: 'Cisco OpenDNS reliable standard resolver.',
  },
  {
    id: 'adguard',
    name: 'AdGuard',
    primaryDns: '94.140.14.14',
    secondaryDns: '94.140.15.15',
    isBuiltIn: true,
    category: 'Ad Blocking',
    description: 'DNS-level ad and tracker blocking by AdGuard.',
  },
  {
    id: 'cleanbrowsing-adult',
    name: 'CleanBrowsing Adult',
    primaryDns: '185.228.168.10',
    secondaryDns: '185.228.169.11',
    isBuiltIn: true,
    category: 'Content Filter',
    description: 'Blocks adult content and malicious domains.',
  },
  {
    id: 'cleanbrowsing-family',
    name: 'CleanBrowsing Family',
    primaryDns: '185.228.168.168',
    secondaryDns: '185.228.169.168',
    isBuiltIn: true,
    category: 'Family Safety',
    description: 'Family filter blocking adult content, proxies, and malware.',
  },
  {
    id: 'dns-watch',
    name: 'DNS.Watch',
    primaryDns: '84.200.69.80',
    secondaryDns: '84.200.70.40',
    isBuiltIn: true,
    category: 'Uncensored',
    description: 'Zero-logging, uncensored German DNS resolver.',
  },
];
