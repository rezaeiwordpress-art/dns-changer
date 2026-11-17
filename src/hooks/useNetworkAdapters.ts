import { useCallback, useEffect, useRef, useState } from 'react';
import {
  isTauriRuntime,
  normalizeCommandError,
  tauriClient,
} from '../services/tauriClient';
import type {
  AdminStatus,
  BackupStatusSummary,
  CommandError,
  NetworkAdapter,
} from '../types/dns';

interface UseNetworkAdaptersOptions {
  configLoaded: boolean;
  rememberSelectedAdapter: boolean;
  savedAdapterGuid: string | null;
  onPersistAdapterGuid: (guid: string) => Promise<void>;
}

export function useNetworkAdapters({
  configLoaded,
  rememberSelectedAdapter,
  savedAdapterGuid,
  onPersistAdapterGuid,
}: UseNetworkAdaptersOptions) {
  const desktopRuntime = isTauriRuntime();
  const [adapters, setAdapters] = useState<NetworkAdapter[]>([]);
  const [selectedGuid, setSelectedGuid] = useState<string | null>(null);
  const [adminStatus, setAdminStatus] = useState<AdminStatus | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(desktopRuntime);
  const [adapterError, setAdapterError] = useState<CommandError | null>(null);

  const rememberAdapterRef = useRef(rememberSelectedAdapter);
  const savedAdapterGuidRef = useRef(savedAdapterGuid);
  rememberAdapterRef.current = rememberSelectedAdapter;
  savedAdapterGuidRef.current = savedAdapterGuid;
  const initialLoadStartedRef = useRef(false);

  const pickInitialAdapter = useCallback(
    (
      list: NetworkAdapter[],
      currentSelected: string | null,
      backups: BackupStatusSummary[],
    ): string | null => {
      if (list.length === 0) return null;
      if (currentSelected && list.some((a) => a.guid === currentSelected)) {
        return currentSelected;
      }
      const saved = savedAdapterGuidRef.current;
      if (
        rememberAdapterRef.current &&
        saved &&
        list.some((a) => a.guid === saved)
      ) {
        return saved;
      }
      const backedUpAdapter = list.find((a) =>
        backups.some((b) => b.adapterGuid === a.guid && b.hasBackup),
      );
      if (backedUpAdapter) {
        return backedUpAdapter.guid;
      }
      return (
        list.find((a) => a.operationalStatus === 'up')?.guid ?? list[0].guid
      );
    },
    [],
  );

  const refreshAdapters = useCallback(
    async (
      knownBackups: BackupStatusSummary[] = [],
    ): Promise<{ adapters: NetworkAdapter[]; error: CommandError | null }> => {
      if (!desktopRuntime) return { adapters: [], error: null };
      setIsLoading(true);
      setAdapterError(null);
      try {
        const [list, admin] = await Promise.all([
          tauriClient.refreshNetworkInformation(),
          tauriClient.checkAdminPrivileges(),
        ]);
        setAdapters(list);
        setAdminStatus(admin);
        setSelectedGuid((prev) => pickInitialAdapter(list, prev, knownBackups));
        return { adapters: list, error: null };
      } catch (err) {
        const normalized = normalizeCommandError(err);
        setAdapterError(normalized);
        return { adapters: [], error: normalized };
      } finally {
        setIsLoading(false);
      }
    },
    [desktopRuntime, pickInitialAdapter],
  );

  useEffect(() => {
    if (!desktopRuntime || !configLoaded || initialLoadStartedRef.current) return;
    initialLoadStartedRef.current = true;
    let active = true;
    let completed = false;
    (async () => {
      setIsLoading(true);
      try {
        const [list, admin, backups] = await Promise.all([
          tauriClient.getNetworkAdapters(),
          tauriClient.checkAdminPrivileges(),
          tauriClient.listAllBackups().catch(() => [] as BackupStatusSummary[]),
        ]);
        if (!active) return;
        completed = true;
        setAdapters(list);
        setAdminStatus(admin);
        setSelectedGuid((prev) => pickInitialAdapter(list, prev, backups));
      } catch (err) {
        if (!active) return;
        completed = true;
        setAdapterError(normalizeCommandError(err));
      } finally {
        if (active) setIsLoading(false);
      }
    })();
    return () => {
      active = false;
      if (!completed) {
        initialLoadStartedRef.current = false;
      }
    };
  }, [configLoaded, desktopRuntime, pickInitialAdapter]);

  const selectAdapter = useCallback(
    async (guid: string) => {
      setSelectedGuid(guid);
      setAdapterError(null);
      if (desktopRuntime && rememberSelectedAdapter) {
        await onPersistAdapterGuid(guid).catch(() => undefined);
      }
    },
    [desktopRuntime, onPersistAdapterGuid, rememberSelectedAdapter],
  );

  const selectedAdapter =
    adapters.find((a) => a.guid === selectedGuid) ?? null;

  return {
    adapters,
    selectedGuid,
    selectedAdapter,
    adminStatus,
    isLoading,
    adapterError,
    refreshAdapters,
    selectAdapter,
  };
}
