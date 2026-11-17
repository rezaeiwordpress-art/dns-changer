import { useCallback, useEffect, useState } from 'react';
import {
  isTauriRuntime,
  tauriClient,
  type SavePreferencesPayload,
} from '../services/tauriClient';
import type { AppConfig } from '../types/dns';

const BROWSER_PREFS_KEY = 'dns-changer.browser-prefs.v1';

const DEFAULT_CONFIG: AppConfig = {
  schemaVersion: 1,
  language: 'fa',
  theme: 'system',
  selectedAdapterGuid: null,
  selectedProviderId: 'cloudflare',
  rememberSelectedAdapter: true,
  rememberSelectedProvider: true,
  startPage: 'dashboard',
  confirmBeforeRestore: true,
  showNotifications: true,
  customProviders: [],
};

function loadBrowserFallbackConfig(): AppConfig {
  try {
    const raw = localStorage.getItem(BROWSER_PREFS_KEY);
    if (!raw) return DEFAULT_CONFIG;
    return { ...DEFAULT_CONFIG, ...JSON.parse(raw) };
  } catch {
    return DEFAULT_CONFIG;
  }
}

export function useApplicationSettings() {
  const desktopRuntime = isTauriRuntime();
  const [config, setConfig] = useState<AppConfig>(() =>
    desktopRuntime ? DEFAULT_CONFIG : loadBrowserFallbackConfig(),
  );
  const [isLoaded, setIsLoaded] = useState(!desktopRuntime);

  useEffect(() => {
    if (!desktopRuntime) return;
    let active = true;
    tauriClient
      .getAppConfig()
      .then((loaded) => {
        if (!active) return;
        setConfig(loaded);
      })
      .catch(() => undefined)
      .finally(() => {
        if (active) setIsLoaded(true);
      });
    return () => {
      active = false;
    };
  }, [desktopRuntime]);

  const updatePreferences = useCallback(
    async (patch: SavePreferencesPayload): Promise<AppConfig> => {
      if (!desktopRuntime) {
        let nextConfig = DEFAULT_CONFIG;
        setConfig((prev) => {
          nextConfig = {
            ...prev,
            ...(patch.language !== undefined ? { language: patch.language } : {}),
            ...(patch.theme !== undefined ? { theme: patch.theme } : {}),
            ...(patch.selectedAdapterGuid !== undefined
              ? {
                  selectedAdapterGuid: patch.selectedAdapterGuid || null,
                }
              : {}),
            ...(patch.selectedProviderId !== undefined
              ? {
                  selectedProviderId: patch.selectedProviderId || null,
                }
              : {}),
            ...(patch.rememberSelectedAdapter !== undefined
              ? { rememberSelectedAdapter: patch.rememberSelectedAdapter }
              : {}),
            ...(patch.rememberSelectedProvider !== undefined
              ? { rememberSelectedProvider: patch.rememberSelectedProvider }
              : {}),
            ...(patch.startPage !== undefined ? { startPage: patch.startPage } : {}),
            ...(patch.confirmBeforeRestore !== undefined
              ? { confirmBeforeRestore: patch.confirmBeforeRestore }
              : {}),
            ...(patch.showNotifications !== undefined
              ? { showNotifications: patch.showNotifications }
              : {}),
          };
          try {
            localStorage.setItem(BROWSER_PREFS_KEY, JSON.stringify(nextConfig));
          } catch {
            // ignore storage quota errors in browser preview
          }
          return nextConfig;
        });
        return nextConfig;
      }

      const saved = await tauriClient.saveAppPreferences(patch);
      setConfig(saved);
      return saved;
    },
    [desktopRuntime],
  );

  return {
    desktopRuntime,
    config,
    isLoaded,
    updatePreferences,
    setConfig,
  };
}
