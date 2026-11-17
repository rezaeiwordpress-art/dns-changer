import { invoke } from '@tauri-apps/api/core';
import type {
  AdminStatus,
  AppConfig,
  AppLanguage,
  ApplyDnsResult,
  BackupStatusSummary,
  CommandError,
  DnsProvider,
  DnsProviderCatalog,
  NavPage,
  NetworkAdapter,
  RestoreDnsResult,
  ThemeMode,
} from '../types/dns';

export function isTauriRuntime(): boolean {
  if (typeof window === 'undefined') {
    return false;
  }
  const win = window as unknown as Record<string, unknown>;
  return Boolean(win.__TAURI_INTERNALS__ || win.__TAURI__);
}

export function normalizeCommandError(err: unknown): CommandError {
  if (
    typeof err === 'object' &&
    err !== null &&
    'code' in err &&
    'message' in err
  ) {
    const typed = err as {
      code: CommandError['code'];
      message: string;
      technicalDetail?: unknown;
    };
    const technicalDetail =
      typeof typed.technicalDetail === 'string' &&
      typed.technicalDetail.trim().length > 0
        ? typed.technicalDetail
        : undefined;
    return technicalDetail !== undefined
      ? {
          code: typed.code,
          message: typed.message,
          technicalDetail,
        }
      : { code: typed.code, message: typed.message };
  }

  if (typeof err === 'string') {
    const trimmed = err.trim();
    return trimmed.length > 0
      ? { code: 'INTERNAL_ERROR', message: err, technicalDetail: err }
      : {
          code: 'INTERNAL_ERROR',
          message: 'An unknown error occurred while invoking backend command.',
        };
  }

  if (err instanceof Error) {
    const message =
      err.message.trim().length > 0
        ? err.message
        : 'An unexpected error occurred.';
    const detail =
      err.stack && err.stack.trim().length > 0 ? err.stack : message;
    return {
      code: 'INTERNAL_ERROR',
      message,
      technicalDetail: detail,
    };
  }

  let serialized: string | undefined;
  if (err !== null && err !== undefined) {
    try {
      const candidate =
        typeof err === 'object' ? JSON.stringify(err) : String(err);
      if (candidate && candidate.trim().length > 0) {
        serialized = candidate;
      }
    } catch {
      serialized = undefined;
    }
  }

  return serialized !== undefined
    ? {
        code: 'INTERNAL_ERROR',
        message: 'An unknown error occurred while invoking backend command.',
        technicalDetail: serialized,
      }
    : {
        code: 'INTERNAL_ERROR',
        message: 'An unknown error occurred while invoking backend command.',
      };
}

async function callTauri<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauriRuntime()) {
    throw {
      code: 'UNSUPPORTED_PLATFORM',
      message:
        'Tauri desktop runtime is not active. Launch via `npm run tauri dev` on Windows 10/11.',
    } satisfies CommandError;
  }

  try {
    return await invoke<T>(command, args);
  } catch (err) {
    throw normalizeCommandError(err);
  }
}

export interface SavePreferencesPayload {
  language?: AppLanguage;
  theme?: ThemeMode;
  selectedAdapterGuid?: string;
  selectedProviderId?: string;
  rememberSelectedAdapter?: boolean;
  rememberSelectedProvider?: boolean;
  startPage?: NavPage;
  confirmBeforeRestore?: boolean;
  showNotifications?: boolean;
}

export const tauriClient = {
  getNetworkAdapters: () => callTauri<NetworkAdapter[]>('get_network_adapters'),

  getAdapterDetails: (adapterGuid: string) =>
    callTauri<NetworkAdapter>('get_adapter_details', { adapterGuid }),

  refreshNetworkInformation: () =>
    callTauri<NetworkAdapter[]>('refresh_network_information'),

  getDnsProviders: () => callTauri<DnsProviderCatalog>('get_dns_providers'),

  getBackupStatus: (adapterGuid: string) =>
    callTauri<BackupStatusSummary>('get_backup_status', { adapterGuid }),

  listAllBackups: () => callTauri<BackupStatusSummary[]>('list_all_backups'),

  checkAdminPrivileges: () => callTauri<AdminStatus>('check_admin_privileges'),

  applyDns: (
    adapterGuid: string,
    primaryDns: string,
    secondaryDns: string | null,
  ) =>
    callTauri<ApplyDnsResult>('apply_dns', {
      adapterGuid,
      primaryDns,
      secondaryDns,
    }),

  restoreDns: (adapterGuid: string) =>
    callTauri<RestoreDnsResult>('restore_dns', { adapterGuid }),

  discardCorruptBackup: (adapterGuid: string) =>
    callTauri<void>('discard_corrupt_backup', { adapterGuid }),

  getAppConfig: () => callTauri<AppConfig>('get_app_config'),

  saveAppPreferences: (prefs: SavePreferencesPayload) =>
    callTauri<AppConfig>('save_app_preferences', { ...prefs }),

  addCustomProvider: (
    name: string,
    primaryDns: string,
    secondaryDns: string | null,
    description: string | null,
  ) =>
    callTauri<DnsProvider>('add_custom_provider', {
      name,
      primaryDns,
      secondaryDns,
      description,
    }),

  updateCustomProvider: (
    providerId: string,
    name: string,
    primaryDns: string,
    secondaryDns: string | null,
    description: string | null,
  ) =>
    callTauri<DnsProvider>('update_custom_provider', {
      providerId,
      name,
      primaryDns,
      secondaryDns,
      description,
    }),

  deleteCustomProvider: (providerId: string) =>
    callTauri<DnsProviderCatalog>('delete_custom_provider', { providerId }),
};
