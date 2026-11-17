import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  isTauriRuntime,
  normalizeCommandError,
  tauriClient,
} from '../services/tauriClient';
import type { DnsProvider, DnsProviderCatalog } from '../types/dns';
import { BUILT_IN_DNS_PROVIDERS_REFERENCE } from '../utils/validation';

interface UseDnsProvidersOptions {
  configLoaded: boolean;
  rememberSelectedProvider: boolean;
  savedProviderId: string | null;
  onPersistProviderId: (providerId: string) => Promise<void>;
}

const BROWSER_CUSTOM_PROVIDERS_KEY = 'dns-changer.browser-custom-providers.v1';

function loadBrowserCustomProviders(): DnsProvider[] {
  try {
    const raw = localStorage.getItem(BROWSER_CUSTOM_PROVIDERS_KEY);
    return raw ? (JSON.parse(raw) as DnsProvider[]) : [];
  } catch {
    return [];
  }
}

export function useDnsProviders({
  configLoaded,
  rememberSelectedProvider,
  savedProviderId,
  onPersistProviderId,
}: UseDnsProvidersOptions) {
  const desktopRuntime = isTauriRuntime();
  const [catalog, setCatalog] = useState<DnsProviderCatalog>(() => ({
    builtIn: BUILT_IN_DNS_PROVIDERS_REFERENCE,
    custom: desktopRuntime ? [] : loadBrowserCustomProviders(),
  }));
  const [selectedProviderId, setSelectedProviderId] =
    useState<string>('cloudflare');
  const [isLoading, setIsLoading] = useState<boolean>(desktopRuntime);

  const rememberProviderRef = useRef(rememberSelectedProvider);
  const savedProviderIdRef = useRef(savedProviderId);
  rememberProviderRef.current = rememberSelectedProvider;
  savedProviderIdRef.current = savedProviderId;
  const initialSyncDoneRef = useRef(false);

  const reloadProviders = useCallback(async () => {
    if (!desktopRuntime) return;
    setIsLoading(true);
    try {
      const loaded = await tauriClient.getDnsProviders();
      setCatalog(loaded);
    } catch {
      // keep existing built-in catalog if backend read fails temporarily
    } finally {
      setIsLoading(false);
    }
  }, [desktopRuntime]);

  useEffect(() => {
    if (!configLoaded || initialSyncDoneRef.current) return;
    initialSyncDoneRef.current = true;
    if (rememberProviderRef.current && savedProviderIdRef.current) {
      setSelectedProviderId(savedProviderIdRef.current);
    }
    if (desktopRuntime) {
      void reloadProviders();
    }
  }, [configLoaded, desktopRuntime, reloadProviders]);

  const allProviders = useMemo(
    () => [...catalog.builtIn, ...catalog.custom],
    [catalog.builtIn, catalog.custom],
  );

  const selectedProvider = useMemo(
    () =>
      allProviders.find((p) => p.id === selectedProviderId) ??
      catalog.builtIn[0] ??
      null,
    [allProviders, catalog.builtIn, selectedProviderId],
  );

  const selectProvider = useCallback(
    async (providerId: string) => {
      setSelectedProviderId(providerId);
      if (rememberSelectedProvider) {
        await onPersistProviderId(providerId).catch(() => undefined);
      }
    },
    [onPersistProviderId, rememberSelectedProvider],
  );

  const addCustomProvider = useCallback(
    async (
      name: string,
      primaryDns: string,
      secondaryDns: string | null,
      description: string | null,
    ): Promise<DnsProvider> => {
      const cleanSecondary = secondaryDns?.trim() || null;
      if (!desktopRuntime) {
        const created: DnsProvider = {
          id: `custom-${Date.now()}`,
          name: name.trim(),
          primaryDns: primaryDns.trim(),
          secondaryDns: cleanSecondary,
          isBuiltIn: false,
          category: 'Custom',
          description: description?.trim() || null,
        };
        setCatalog((prev) => {
          const nextCustom = [...prev.custom, created];
          try {
            localStorage.setItem(
              BROWSER_CUSTOM_PROVIDERS_KEY,
              JSON.stringify(nextCustom),
            );
          } catch {
            // ignore
          }
          return { ...prev, custom: nextCustom };
        });
        return created;
      }

      try {
        const created = await tauriClient.addCustomProvider(
          name,
          primaryDns,
          cleanSecondary,
          description,
        );
        await reloadProviders();
        return created;
      } catch (err) {
        throw normalizeCommandError(err);
      }
    },
    [desktopRuntime, reloadProviders],
  );

  const updateCustomProvider = useCallback(
    async (
      providerId: string,
      name: string,
      primaryDns: string,
      secondaryDns: string | null,
      description: string | null,
    ): Promise<DnsProvider> => {
      const cleanSecondary = secondaryDns?.trim() || null;
      if (!desktopRuntime) {
        let updatedItem: DnsProvider | null = null;
        setCatalog((prev) => {
          const nextCustom = prev.custom.map((item) => {
            if (item.id !== providerId) return item;
            updatedItem = {
              ...item,
              name: name.trim(),
              primaryDns: primaryDns.trim(),
              secondaryDns: cleanSecondary,
              description: description?.trim() || null,
            };
            return updatedItem;
          });
          try {
            localStorage.setItem(
              BROWSER_CUSTOM_PROVIDERS_KEY,
              JSON.stringify(nextCustom),
            );
          } catch {
            // ignore
          }
          return { ...prev, custom: nextCustom };
        });
        if (!updatedItem) {
          throw { code: 'STORAGE_ERROR', message: 'Custom provider not found' };
        }
        return updatedItem;
      }

      try {
        const updated = await tauriClient.updateCustomProvider(
          providerId,
          name,
          primaryDns,
          cleanSecondary,
          description,
        );
        await reloadProviders();
        return updated;
      } catch (err) {
        throw normalizeCommandError(err);
      }
    },
    [desktopRuntime, reloadProviders],
  );

  const deleteCustomProvider = useCallback(
    async (providerId: string): Promise<void> => {
      if (!desktopRuntime) {
        setCatalog((prev) => {
          const nextCustom = prev.custom.filter((p) => p.id !== providerId);
          try {
            localStorage.setItem(
              BROWSER_CUSTOM_PROVIDERS_KEY,
              JSON.stringify(nextCustom),
            );
          } catch {
            // ignore
          }
          return { ...prev, custom: nextCustom };
        });
        if (selectedProviderId === providerId) {
          setSelectedProviderId('cloudflare');
        }
        return;
      }

      try {
        const updatedCatalog = await tauriClient.deleteCustomProvider(providerId);
        setCatalog(updatedCatalog);
        if (selectedProviderId === providerId) {
          setSelectedProviderId('cloudflare');
        }
      } catch (err) {
        throw normalizeCommandError(err);
      }
    },
    [desktopRuntime, selectedProviderId],
  );

  return {
    catalog,
    allProviders,
    selectedProviderId,
    selectedProvider,
    isLoading,
    reloadProviders,
    selectProvider,
    addCustomProvider,
    updateCustomProvider,
    deleteCustomProvider,
  };
}
