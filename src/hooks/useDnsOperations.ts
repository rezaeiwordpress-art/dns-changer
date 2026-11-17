import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { formatTemplate, type Translations } from '../localization';
import {
  isTauriRuntime,
  normalizeCommandError,
  tauriClient,
} from '../services/tauriClient';
import type {
  BackupStatusSummary,
  CommandError,
  ConnectionStatus,
  DnsProvider,
  LastOperationInfo,
  NetworkAdapter,
  ToastNotification,
} from '../types/dns';

interface UseDnsOperationsOptions {
  selectedAdapter: NetworkAdapter | null;
  selectedGuid: string | null;
  selectedProvider: DnsProvider | null;
  allProviders: DnsProvider[];
  adapterError: CommandError | null;
  showNotifications: boolean;
  t: Translations;
  refreshAdapters: (
    knownBackups?: BackupStatusSummary[],
  ) => Promise<{ adapters: NetworkAdapter[]; error: CommandError | null }>;
  pushNotification: (
    variant: ToastNotification['variant'],
    title: string,
    message: string,
    errorCode?: CommandError['code'],
  ) => void;
}

export function useDnsOperations({
  selectedAdapter,
  selectedGuid,
  selectedProvider,
  allProviders,
  adapterError,
  showNotifications,
  t,
  refreshAdapters,
  pushNotification,
}: UseDnsOperationsOptions) {
  const desktopRuntime = isTauriRuntime();
  const [selectedBackup, setSelectedBackup] =
    useState<BackupStatusSummary | null>(null);
  const [allBackups, setAllBackups] = useState<BackupStatusSummary[]>([]);
  const [isMutating, setIsMutating] = useState(false);
  const mutationGuardRef = useRef(false);

  const prevSelectedGuidRef = useRef<string | null>(null);

  const [lastOperation, setLastOperation] = useState<LastOperationInfo>({
    operation: 'none',
    timestamp: null,
    outcome: 'idle',
    message: null,
    errorCode: null,
    technicalDetail: null,
  });

  const reloadBackupState = useCallback(
    async (guid: string | null): Promise<BackupStatusSummary[]> => {
      if (!desktopRuntime) {
        setSelectedBackup(null);
        setAllBackups([]);
        return [];
      }
      try {
        const [all, current] = await Promise.all([
          tauriClient.listAllBackups(),
          guid ? tauriClient.getBackupStatus(guid) : Promise.resolve(null),
        ]);
        setAllBackups(all);
        setSelectedBackup(current);
        return all;
      } catch {
        return [];
      }
    },
    [desktopRuntime],
  );

  useEffect(() => {
    if (
      prevSelectedGuidRef.current !== null &&
      prevSelectedGuidRef.current !== selectedGuid
    ) {
      setLastOperation({
        operation: 'none',
        timestamp: null,
        outcome: 'idle',
        message: null,
        errorCode: null,
        technicalDetail: null,
      });
    }
    prevSelectedGuidRef.current = selectedGuid;
    void reloadBackupState(selectedGuid);
  }, [reloadBackupState, selectedGuid]);

  const applySelectedDns = useCallback(async () => {
    if (mutationGuardRef.current || !selectedAdapter || !selectedProvider) {
      return;
    }

    mutationGuardRef.current = true;
    setIsMutating(true);

    try {
      const result = await tauriClient.applyDns(
        selectedAdapter.guid,
        selectedProvider.primaryDns,
        selectedProvider.secondaryDns,
      );

      if (!result.verified) {
        throw {
          code: 'VERIFICATION_FAILED',
          message: t.errors.VERIFICATION_FAILED,
        } satisfies CommandError;
      }

      const now = new Date().toLocaleTimeString();
      const summary = formatTemplate(t.notifications.applySuccessBody, {
        servers: result.appliedServers.join(', '),
        adapter: result.adapter.name,
      });

      setLastOperation({
        operation: 'apply',
        timestamp: now,
        outcome: 'success',
        message: summary,
        errorCode: null,
        technicalDetail: null,
      });

      const updatedBackups = await reloadBackupState(selectedAdapter.guid);
      await refreshAdapters(updatedBackups);

      if (showNotifications) {
        pushNotification('success', t.notifications.applySuccessTitle, summary);
      }
    } catch (err) {
      const normalized = normalizeCommandError(err);
      const localizedMsg = t.errors[normalized.code] ?? normalized.message;
      const isCancelled = normalized.code === 'UAC_CANCELLED';

      setLastOperation({
        operation: 'apply',
        timestamp: new Date().toLocaleTimeString(),
        outcome: isCancelled ? 'cancelled' : 'error',
        message: localizedMsg,
        errorCode: normalized.code,
        technicalDetail: isCancelled
          ? null
          : (normalized.technicalDetail ?? normalized.message ?? null),
      });

      await reloadBackupState(selectedAdapter.guid);
      await refreshAdapters();

      pushNotification(
        isCancelled ? 'warning' : 'error',
        isCancelled
          ? t.notifications.uacCancelledTitle
          : t.notifications.operationErrorTitle,
        localizedMsg,
        normalized.code,
      );
    } finally {
      mutationGuardRef.current = false;
      setIsMutating(false);
    }
  }, [
    pushNotification,
    refreshAdapters,
    reloadBackupState,
    selectedAdapter,
    selectedProvider,
    showNotifications,
    t,
  ]);

  const restoreOriginalDns = useCallback(async () => {
    if (mutationGuardRef.current || !selectedAdapter) {
      return;
    }

    mutationGuardRef.current = true;
    setIsMutating(true);

    try {
      const result = await tauriClient.restoreDns(selectedAdapter.guid);
      if (!result.verified) {
        throw {
          code: 'VERIFICATION_FAILED',
          message: t.errors.VERIFICATION_FAILED,
        } satisfies CommandError;
      }

      const now = new Date().toLocaleTimeString();
      const summary = formatTemplate(t.notifications.restoreSuccessBody, {
        adapter: result.adapter.name,
      });

      setLastOperation({
        operation: 'restore',
        timestamp: now,
        outcome: 'success',
        message: summary,
        errorCode: null,
        technicalDetail: null,
      });

      const updatedBackups = await reloadBackupState(selectedAdapter.guid);
      await refreshAdapters(updatedBackups);

      if (showNotifications) {
        pushNotification(
          'success',
          t.notifications.restoreSuccessTitle,
          summary,
        );
      }
    } catch (err) {
      const normalized = normalizeCommandError(err);
      const localizedMsg = t.errors[normalized.code] ?? normalized.message;
      const isCancelled = normalized.code === 'UAC_CANCELLED';

      setLastOperation({
        operation: 'restore',
        timestamp: new Date().toLocaleTimeString(),
        outcome: isCancelled ? 'cancelled' : 'error',
        message: localizedMsg,
        errorCode: normalized.code,
        technicalDetail: isCancelled
          ? null
          : (normalized.technicalDetail ?? normalized.message ?? null),
      });

      await reloadBackupState(selectedAdapter.guid);
      await refreshAdapters();

      pushNotification(
        isCancelled ? 'warning' : 'error',
        isCancelled
          ? t.notifications.uacCancelledTitle
          : t.notifications.operationErrorTitle,
        localizedMsg,
        normalized.code,
      );
    } finally {
      mutationGuardRef.current = false;
      setIsMutating(false);
    }
  }, [
    pushNotification,
    refreshAdapters,
    reloadBackupState,
    selectedAdapter,
    showNotifications,
    t,
  ]);

  const discardCorruptBackup = useCallback(async () => {
    if (mutationGuardRef.current || !selectedAdapter) {
      return;
    }

    mutationGuardRef.current = true;
    setIsMutating(true);

    try {
      await tauriClient.discardCorruptBackup(selectedAdapter.guid);
      const updatedBackups = await reloadBackupState(selectedAdapter.guid);
      await refreshAdapters(updatedBackups);

      setLastOperation({
        operation: 'none',
        timestamp: null,
        outcome: 'idle',
        message: null,
        errorCode: null,
        technicalDetail: null,
      });

      if (showNotifications) {
        pushNotification(
          'success',
          t.notifications.corruptBackupDiscardedTitle,
          formatTemplate(t.notifications.corruptBackupDiscardedBody, {
            adapter: selectedAdapter.name,
          }),
        );
      }
    } catch (err) {
      const normalized = normalizeCommandError(err);
      const localizedMsg = t.errors[normalized.code] ?? normalized.message;
      pushNotification(
        'error',
        t.notifications.operationErrorTitle,
        localizedMsg,
        normalized.code,
      );
    } finally {
      mutationGuardRef.current = false;
      setIsMutating(false);
    }
  }, [
    pushNotification,
    refreshAdapters,
    reloadBackupState,
    selectedAdapter,
    showNotifications,
    t,
  ]);

  const manualRefresh = useCallback(async () => {
    const backups = await reloadBackupState(selectedGuid);
    const { error } = await refreshAdapters(backups);
    if (error) {
      const localizedMsg = t.errors[error.code] ?? error.message;
      setLastOperation({
        operation: 'refresh',
        timestamp: new Date().toLocaleTimeString(),
        outcome: 'error',
        message: localizedMsg,
        errorCode: error.code,
        technicalDetail: error.technicalDetail ?? error.message ?? null,
      });
      return;
    }
    setLastOperation({
      operation: 'refresh',
      timestamp: new Date().toLocaleTimeString(),
      outcome: 'success',
      message: null,
      errorCode: null,
      technicalDetail: null,
    });
  }, [refreshAdapters, reloadBackupState, selectedGuid, t]);

  const activeConfiguredProvider = useMemo(() => {
    if (
      !selectedAdapter ||
      !selectedBackup?.hasBackup ||
      selectedAdapter.dnsMode !== 'static'
    ) {
      return null;
    }
    const servers = selectedAdapter.currentIpv4DnsServers;
    if (servers.length === 0) return null;
    return (
      allProviders.find((prov) => {
        if (prov.secondaryDns === null) {
          return servers.length === 1 && prov.primaryDns === servers[0];
        }
        return (
          servers.length === 2 &&
          prov.primaryDns === servers[0] &&
          prov.secondaryDns === servers[1]
        );
      }) ?? null
    );
  }, [allProviders, selectedAdapter, selectedBackup]);

  const connectionStatus: ConnectionStatus = useMemo(() => {
    if (
      adapterError ||
      lastOperation.outcome === 'error' ||
      (selectedBackup?.hasBackup && !selectedBackup.isValid) ||
      selectedAdapter?.operationalStatus === 'disabled' ||
      Boolean(selectedAdapter?.dnsModeIndeterminateReason)
    ) {
      return 'error';
    }

    if (
      selectedAdapter &&
      selectedAdapter.operationalStatus === 'up' &&
      !selectedAdapter.dnsModeIndeterminateReason &&
      selectedBackup?.hasBackup &&
      selectedBackup.isValid &&
      selectedAdapter.dnsMode === 'static' &&
      selectedAdapter.currentIpv4DnsServers.length > 0
    ) {
      return 'connected';
    }

    return 'disconnected';
  }, [
    adapterError,
    lastOperation.outcome,
    selectedAdapter,
    selectedBackup,
  ]);

  const otherAdapterBackupsCount = useMemo(
    () =>
      allBackups.filter(
        (b) => b.hasBackup && b.adapterGuid !== selectedGuid,
      ).length,
    [allBackups, selectedGuid],
  );

  return {
    selectedBackup,
    allBackups,
    otherAdapterBackupsCount,
    activeConfiguredProvider,
    connectionStatus,
    isMutating,
    lastOperation,
    applySelectedDns,
    restoreOriginalDns,
    discardCorruptBackup,
    manualRefresh,
  };
}
