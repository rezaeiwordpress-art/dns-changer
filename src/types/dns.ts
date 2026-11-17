export type AdapterType = 'wifi' | 'ethernet' | 'virtual' | 'loopback' | 'other';

export type AdapterStatus =
  | 'up'
  | 'disconnected'
  | 'disabled'
  | 'notPresent'
  | 'unknown';

export type DnsMode = 'automatic' | 'static';

export type DhcpStatus = 'enabled' | 'disabled' | 'unknown';

export type NavPage =
  | 'dashboard'
  | 'providers'
  | 'customDns'
  | 'adapters'
  | 'settings'
  | 'about';

export type AppLanguage = 'fa' | 'en';

export type ThemeMode = 'dark' | 'light' | 'system';

export type ConnectionStatus = 'connected' | 'disconnected' | 'error';

export interface NetworkAdapter {
  guid: string;
  interfaceIndex: number;
  name: string;
  interfaceDescription: string;
  adapterType: AdapterType;
  operationalStatus: AdapterStatus;
  ipv4Addresses: string[];
  ipv4Gateway: string | null;
  currentIpv4DnsServers: string[];
  dnsMode: DnsMode;
  dhcpStatus: DhcpStatus;
  macAddress: string | null;
  linkSpeed: string | null;
  dnsModeIndeterminateReason: string | null;
}

export interface DnsProvider {
  id: string;
  name: string;
  primaryDns: string;
  secondaryDns: string | null;
  isBuiltIn: boolean;
  category: string | null;
  description: string | null;
}

export interface DnsProviderCatalog {
  builtIn: DnsProvider[];
  custom: DnsProvider[];
}

export type BackupRecordStatus = 'protected' | 'applyFailedPreserved';

export interface BackupRecord {
  schemaVersion: number;
  adapterGuid: string;
  interfaceIndexAtBackup: number;
  adapterNameAtBackup: string;
  originalDnsMode: DnsMode;
  originalIpv4DnsServers: string[];
  backupTimestamp: string;
  backupStatus: BackupRecordStatus;
  checksum: string;
}

export interface BackupStatusSummary {
  adapterGuid: string;
  hasBackup: boolean;
  isValid: boolean;
  backup: BackupRecord | null;
  integrityError: string | null;
}

export interface AdminStatus {
  isElevated: boolean;
  canRequestElevation: boolean;
  platform: string;
}

export interface ApplyDnsResult {
  adapterGuid: string;
  appliedServers: string[];
  verified: boolean;
  backupCreated: boolean;
  adapter: NetworkAdapter;
}

export interface RestoreDnsResult {
  adapterGuid: string;
  restoredMode: DnsMode;
  restoredServers: string[];
  verified: boolean;
  backupCleared: boolean;
  adapter: NetworkAdapter;
}

export interface AppConfig {
  schemaVersion: number;
  language: AppLanguage;
  theme: ThemeMode;
  selectedAdapterGuid: string | null;
  selectedProviderId: string | null;
  rememberSelectedAdapter: boolean;
  rememberSelectedProvider: boolean;
  startPage: NavPage;
  confirmBeforeRestore: boolean;
  showNotifications: boolean;
  customProviders: DnsProvider[];
}

export type ErrorCode =
  | 'ADAPTER_NOT_FOUND'
  | 'ADAPTER_DISABLED'
  | 'ADAPTER_DISCONNECTED'
  | 'INVALID_DNS_ADDRESS'
  | 'INVALID_ADAPTER_GUID'
  | 'ADMIN_ACCESS_DENIED'
  | 'UAC_CANCELLED'
  | 'ELEVATION_FAILED'
  | 'POWERSHELL_EXECUTION_FAILED'
  | 'POWERSHELL_BLOCKED'
  | 'POWERSHELL_TIMEOUT'
  | 'DNS_APPLY_FAILED'
  | 'DNS_MODE_INDETERMINATE'
  | 'IPV6_MODIFICATION_DETECTED'
  | 'IPV6_RECOVERY_FAILED'
  | 'VERIFICATION_FAILED'
  | 'ROLLBACK_NOT_PERFORMED'
  | 'BACKUP_CORRUPTION'
  | 'BACKUP_NOT_FOUND'
  | 'RESTORE_FAILED'
  | 'NETWORK_CHANGED'
  | 'CONCURRENT_OPERATION'
  | 'DUPLICATE_PROVIDER_NAME'
  | 'INVALID_INPUT'
  | 'UNSUPPORTED_PLATFORM'
  | 'STORAGE_ERROR'
  | 'INTERNAL_ERROR';

export interface CommandError {
  code: ErrorCode;
  message: string;
  technicalDetail?: string | null;
}

export interface LastOperationInfo {
  operation: 'apply' | 'restore' | 'refresh' | 'none';
  timestamp: string | null;
  outcome: 'success' | 'error' | 'cancelled' | 'idle';
  message: string | null;
  errorCode: ErrorCode | null;
  technicalDetail: string | null;
}

export interface ToastNotification {
  id: string;
  variant: 'success' | 'error' | 'warning' | 'info';
  title: string;
  message: string;
  errorCode?: ErrorCode;
}
