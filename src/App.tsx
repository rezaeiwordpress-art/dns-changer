import { useCallback, useEffect, useState } from 'react';
import { ConfirmDialog } from './components/ConfirmDialog';
import { NotificationStack } from './components/NotificationStack';
import { Sidebar } from './components/Sidebar';
import { TopBar } from './components/TopBar';
import { useApplicationSettings } from './hooks/useApplicationSettings';
import { useDnsOperations } from './hooks/useDnsOperations';
import { useDnsProviders } from './hooks/useDnsProviders';
import { useLocalization } from './hooks/useLocalization';
import { useNetworkAdapters } from './hooks/useNetworkAdapters';
import { useNotifications } from './hooks/useNotifications';
import { useTheme } from './hooks/useTheme';
import { formatTemplate } from './localization';
import { AboutPage } from './pages/AboutPage';
import { AdaptersPage } from './pages/AdaptersPage';
import { CustomDnsPage } from './pages/CustomDnsPage';
import { DashboardPage } from './pages/DashboardPage';
import { ProvidersPage } from './pages/ProvidersPage';
import { SettingsPage } from './pages/SettingsPage';
import type { NavPage } from './types/dns';

export default function App() {
  const { desktopRuntime, config, isLoaded, updatePreferences } =
    useApplicationSettings();
  const { t } = useLocalization(config.language);
  useTheme(config.theme);
  const { notifications, pushNotification, dismissNotification } =
    useNotifications();

  const [activePage, setActivePage] = useState<NavPage>(config.startPage);
  const [hasAppliedInitialStartPage, setHasAppliedInitialStartPage] =
    useState(false);

  useEffect(() => {
    if (isLoaded && !hasAppliedInitialStartPage) {
      setActivePage(config.startPage);
      setHasAppliedInitialStartPage(true);
    }
  }, [config.startPage, hasAppliedInitialStartPage, isLoaded]);

  const handlePersistAdapterGuid = useCallback(
    async (guid: string) => {
      await updatePreferences({ selectedAdapterGuid: guid });
    },
    [updatePreferences],
  );

  const handlePersistProviderId = useCallback(
    async (providerId: string) => {
      await updatePreferences({ selectedProviderId: providerId });
    },
    [updatePreferences],
  );

  const {
    adapters,
    selectedGuid,
    selectedAdapter,
    adminStatus,
    isLoading: isLoadingAdapters,
    adapterError,
    refreshAdapters,
    selectAdapter,
  } = useNetworkAdapters({
    configLoaded: isLoaded,
    rememberSelectedAdapter: config.rememberSelectedAdapter,
    savedAdapterGuid: config.selectedAdapterGuid,
    onPersistAdapterGuid: handlePersistAdapterGuid,
  });

  const {
    catalog,
    allProviders,
    selectedProvider,
    selectProvider,
    addCustomProvider,
    updateCustomProvider,
    deleteCustomProvider,
  } = useDnsProviders({
    configLoaded: isLoaded,
    rememberSelectedProvider: config.rememberSelectedProvider,
    savedProviderId: config.selectedProviderId,
    onPersistProviderId: handlePersistProviderId,
  });

  const {
    selectedBackup,
    otherAdapterBackupsCount,
    activeConfiguredProvider,
    connectionStatus,
    isMutating,
    lastOperation,
    applySelectedDns,
    restoreOriginalDns,
    discardCorruptBackup,
    manualRefresh,
  } = useDnsOperations({
    selectedAdapter,
    selectedGuid,
    selectedProvider,
    allProviders,
    adapterError,
    showNotifications: config.showNotifications,
    t,
    refreshAdapters,
    pushNotification,
  });

  const [confirmAction, setConfirmAction] = useState<
    'apply' | 'restore' | 'discardBackup' | null
  >(null);

  const handleRequestStart = () => {
    setConfirmAction('apply');
  };

  const handleRequestStopRestore = () => {
    if (config.confirmBeforeRestore) {
      setConfirmAction('restore');
    } else {
      void restoreOriginalDns();
    }
  };

  const handleRequestDiscardCorruptBackup = () => {
    setConfirmAction('discardBackup');
  };

  const handleConfirmAction = () => {
    const action = confirmAction;
    setConfirmAction(null);
    if (action === 'apply') {
      void applySelectedDns();
    } else if (action === 'restore') {
      void restoreOriginalDns();
    } else if (action === 'discardBackup') {
      void discardCorruptBackup();
    }
  };

  return (
    <div className="flex min-h-screen bg-slate-50 text-slate-900 dark:bg-slate-950 dark:text-slate-100">
      <Sidebar
        activePage={activePage}
        onNavigate={setActivePage}
        t={t}
        adminStatus={adminStatus}
        connectionStatus={connectionStatus}
        desktopRuntime={desktopRuntime}
      />

      <div className="flex min-w-0 flex-1 flex-col">
        <TopBar
          activePage={activePage}
          t={t}
          desktopRuntime={desktopRuntime}
          isLoading={isLoadingAdapters}
          isMutating={isMutating}
          onRefresh={() => void manualRefresh()}
        />

        <main className="flex-1 overflow-y-auto p-6">
          <div className="mx-auto max-w-6xl">
            {activePage === 'dashboard' && (
              <DashboardPage
                t={t}
                desktopRuntime={desktopRuntime}
                connectionStatus={connectionStatus}
                selectedAdapter={selectedAdapter}
                selectedProvider={selectedProvider}
                activeConfiguredProvider={activeConfiguredProvider}
                selectedBackup={selectedBackup}
                otherAdapterBackupsCount={otherAdapterBackupsCount}
                adminStatus={adminStatus}
                adapterError={adapterError}
                isLoadingAdapters={isLoadingAdapters}
                lastOperation={lastOperation}
                isMutating={isMutating}
                onStart={handleRequestStart}
                onStopRestore={handleRequestStopRestore}
                onDiscardCorruptBackup={handleRequestDiscardCorruptBackup}
                onRetryAdapters={() => void manualRefresh()}
                onNavigate={setActivePage}
              />
            )}

            {activePage === 'providers' && (
              <ProvidersPage
                t={t}
                catalog={catalog}
                selectedProvider={selectedProvider}
                onSelectProvider={(id) => void selectProvider(id)}
                onNavigate={setActivePage}
              />
            )}

            {activePage === 'customDns' && (
              <CustomDnsPage
                t={t}
                catalog={catalog}
                onAdd={addCustomProvider}
                onUpdate={updateCustomProvider}
                onDelete={deleteCustomProvider}
                onNotifySuccess={(title, message) => {
                  if (config.showNotifications) {
                    pushNotification('success', title, message);
                  }
                }}
                onNotifyError={(title, message) =>
                  pushNotification('error', title, message)
                }
              />
            )}

            {activePage === 'adapters' && (
              <AdaptersPage
                t={t}
                desktopRuntime={desktopRuntime}
                adapters={adapters}
                selectedGuid={selectedGuid}
                selectedAdapter={selectedAdapter}
                adapterError={adapterError}
                isLoading={isLoadingAdapters}
                onSelectAdapter={(guid) => void selectAdapter(guid)}
                onRefresh={() => void manualRefresh()}
              />
            )}

            {activePage === 'settings' && (
              <SettingsPage
                t={t}
                config={config}
                onUpdatePreferences={async (patch) => {
                  await updatePreferences(patch);
                }}
              />
            )}

            {activePage === 'about' && <AboutPage t={t} />}
          </div>
        </main>
      </div>

      <ConfirmDialog
        open={confirmAction !== null}
        title={
          confirmAction === 'apply'
            ? t.dialogs.confirmApplyTitle
            : confirmAction === 'discardBackup'
              ? t.dialogs.confirmDiscardBackupTitle
              : t.dialogs.confirmRestoreTitle
        }
        message={
          confirmAction === 'apply' && selectedAdapter && selectedProvider
            ? selectedProvider.secondaryDns
              ? formatTemplate(t.dialogs.confirmApplyMessage, {
                  adapter: selectedAdapter.name,
                  provider: selectedProvider.name,
                  primary: selectedProvider.primaryDns,
                  secondary: selectedProvider.secondaryDns,
                })
              : formatTemplate(t.dialogs.confirmApplySingleMessage, {
                  adapter: selectedAdapter.name,
                  provider: selectedProvider.name,
                  primary: selectedProvider.primaryDns,
                })
            : confirmAction === 'discardBackup' && selectedAdapter
              ? formatTemplate(t.dialogs.confirmDiscardBackupMessage, {
                  adapter: selectedAdapter.name,
                })
              : selectedAdapter
                ? formatTemplate(t.dialogs.confirmRestoreMessage, {
                    adapter: selectedAdapter.name,
                  })
                : ''
        }
        danger={confirmAction === 'discardBackup'}
        t={t}
        onConfirm={handleConfirmAction}
        onCancel={() => setConfirmAction(null)}
      />

      <NotificationStack
        notifications={notifications}
        t={t}
        onDismiss={dismissNotification}
      />
    </div>
  );
}
