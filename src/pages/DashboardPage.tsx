import {
  AlertCircle,
  AlertTriangle,
  CheckCircle2,
  Play,
  RotateCcw,
  ShieldCheck,
  WifiOff,
} from 'lucide-react';
import {
  AdapterErrorCard,
  TechnicalDetailsSection,
} from '../components/AdapterErrorCard';
import type { Translations } from '../localization';
import type {
  AdminStatus,
  BackupStatusSummary,
  CommandError,
  ConnectionStatus,
  DnsProvider,
  LastOperationInfo,
  NavPage,
  NetworkAdapter,
} from '../types/dns';

interface DashboardPageProps {
  t: Translations;
  desktopRuntime: boolean;
  connectionStatus: ConnectionStatus;
  selectedAdapter: NetworkAdapter | null;
  selectedProvider: DnsProvider | null;
  activeConfiguredProvider: DnsProvider | null;
  selectedBackup: BackupStatusSummary | null;
  otherAdapterBackupsCount: number;
  adminStatus: AdminStatus | null;
  adapterError: CommandError | null;
  isLoadingAdapters: boolean;
  lastOperation: LastOperationInfo;
  isMutating: boolean;
  onStart: () => void;
  onStopRestore: () => void;
  onDiscardCorruptBackup: () => void;
  onRetryAdapters: () => void;
  onNavigate: (page: NavPage) => void;
}

export function DashboardPage({
  t,
  desktopRuntime,
  connectionStatus,
  selectedAdapter,
  selectedProvider,
  activeConfiguredProvider,
  selectedBackup,
  otherAdapterBackupsCount,
  adminStatus,
  adapterError,
  isLoadingAdapters,
  lastOperation,
  isMutating,
  onStart,
  onStopRestore,
  onDiscardCorruptBackup,
  onRetryAdapters,
  onNavigate,
}: DashboardPageProps) {
  const isDnsModeIndeterminate = Boolean(
    selectedAdapter?.dnsModeIndeterminateReason,
  );

  const canStart =
    desktopRuntime &&
    !isMutating &&
    selectedAdapter !== null &&
    selectedProvider !== null &&
    selectedAdapter.operationalStatus === 'up' &&
    !isDnsModeIndeterminate;

  const canStopRestore =
    desktopRuntime &&
    !isMutating &&
    selectedAdapter !== null &&
    Boolean(selectedBackup?.hasBackup && selectedBackup.isValid) &&
    selectedAdapter.operationalStatus !== 'disabled' &&
    selectedAdapter.operationalStatus !== 'notPresent';

  const statusMeta =
    connectionStatus === 'connected'
      ? {
          label: t.status.connected,
          detail: t.status.connectedDetail,
          icon: CheckCircle2,
          tone: 'border-emerald-200 bg-emerald-50/70 text-emerald-800 dark:border-emerald-500/30 dark:bg-emerald-500/10 dark:text-emerald-300',
          iconColor: 'text-emerald-600 dark:text-emerald-400',
        }
      : connectionStatus === 'error'
        ? {
            label: t.status.error,
            detail: t.status.errorDetail,
            icon: AlertCircle,
            tone: 'border-rose-200 bg-rose-50/70 text-rose-800 dark:border-rose-500/30 dark:bg-rose-500/10 dark:text-rose-300',
            iconColor: 'text-rose-600 dark:text-rose-400',
          }
        : {
            label: t.status.disconnected,
            detail: t.status.disconnectedDetail,
            icon: WifiOff,
            tone: 'border-slate-200 bg-slate-100/70 text-slate-700 dark:border-slate-800 dark:bg-slate-800/60 dark:text-slate-300',
            iconColor: 'text-slate-500 dark:text-slate-400',
          };

  const StatusIcon = statusMeta.icon;

  const displayedProvider =
    connectionStatus === 'connected' && activeConfiguredProvider
      ? activeConfiguredProvider
      : selectedProvider;

  const operationLabel = t.dashboard.operations[lastOperation.operation];
  const outcomeLabel = t.dashboard.outcomes[lastOperation.outcome];

  const startButtonTitle = !desktopRuntime
    ? t.common.browserPreviewTitle
    : !selectedAdapter
      ? t.dashboard.noAdapterSelected
      : isDnsModeIndeterminate
        ? t.dashboard.dnsModeIndeterminateWarning
        : selectedAdapter.operationalStatus !== 'up'
          ? t.adapterStatus[selectedAdapter.operationalStatus]
          : t.dashboard.startTooltip;

  const stopButtonTitle = !desktopRuntime
    ? t.common.browserPreviewTitle
    : !selectedAdapter
      ? t.dashboard.noAdapterSelected
      : !selectedBackup?.hasBackup
        ? t.dashboard.backupUnavailable
        : t.dashboard.stopTooltip;

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold text-slate-900 dark:text-slate-100">
          {t.dashboard.title}
        </h2>
        <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
          {t.dashboard.subtitle}
        </p>
      </div>

      {adapterError && (
        <AdapterErrorCard
          error={adapterError}
          t={t}
          desktopRuntime={desktopRuntime}
          isRetrying={isLoadingAdapters}
          onRetry={onRetryAdapters}
        />
      )}

      {/* Connection Status + Main START / STOP Controls */}
      <section className="rounded-xl border border-slate-200 bg-white p-6 dark:border-slate-800 dark:bg-slate-900">
        <div className="flex flex-col gap-6 lg:flex-row lg:items-center lg:justify-between">
          <div className="space-y-3">
            <div className="text-xs font-medium text-slate-500 dark:text-slate-400">
              {t.dashboard.connectionStatusLabel}
            </div>
            <div className="flex items-center gap-3">
              <div
                className={`inline-flex items-center gap-2 rounded-lg border px-3.5 py-1.5 text-sm font-semibold ${statusMeta.tone}`}
              >
                <StatusIcon className={`h-4 w-4 ${statusMeta.iconColor}`} />
                <span>{statusMeta.label}</span>
              </div>
              <span className="text-xs text-slate-500 dark:text-slate-400">
                {connectionStatus === 'connected'
                  ? t.dashboard.activeViaApp
                  : t.dashboard.notConfiguredViaApp}
              </span>
            </div>
            <p className="text-sm text-slate-600 dark:text-slate-300">
              {statusMeta.detail}
            </p>
            {adminStatus && !adminStatus.isElevated && (
              <p className="text-xs text-slate-500 dark:text-slate-400">
                {t.common.uacNotice}
              </p>
            )}
          </div>

          <div className="flex flex-wrap items-center gap-3 shrink-0">
            <button
              type="button"
              onClick={onStart}
              disabled={!canStart}
              title={startButtonTitle}
              className="inline-flex min-w-[148px] items-center justify-center gap-2 rounded-lg bg-sky-600 px-5 py-3 text-sm font-semibold text-white transition-colors hover:bg-sky-500 disabled:cursor-not-allowed disabled:opacity-45 whitespace-nowrap focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-500 focus-visible:ring-offset-2"
            >
              <Play className="h-4 w-4" />
              <span>
                {isMutating ? t.dashboard.startingButton : t.dashboard.startButton}
              </span>
            </button>

            <button
              type="button"
              onClick={onStopRestore}
              disabled={!canStopRestore}
              title={stopButtonTitle}
              className="inline-flex min-w-[168px] items-center justify-center gap-2 rounded-lg border border-slate-300 bg-white px-5 py-3 text-sm font-semibold text-slate-800 transition-colors hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-45 whitespace-nowrap focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-500 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 dark:hover:bg-slate-700"
            >
              <RotateCcw className="h-4 w-4" />
              <span>
                {isMutating ? t.dashboard.stoppingButton : t.dashboard.stopButton}
              </span>
            </button>
          </div>
        </div>
      </section>

      {isDnsModeIndeterminate && selectedAdapter && (
        <div className="flex items-start gap-2.5 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-xs text-rose-900 dark:border-rose-500/25 dark:bg-rose-500/10 dark:text-rose-200">
          <AlertCircle className="mt-0.5 h-4 w-4 shrink-0 text-rose-600 dark:text-rose-400" />
          <div className="space-y-1">
            <p className="font-medium">{t.dashboard.dnsModeIndeterminateWarning}</p>
            <p className="font-mono-tech text-[11px] opacity-85">
              {selectedAdapter.dnsModeIndeterminateReason}
            </p>
          </div>
        </div>
      )}

      {otherAdapterBackupsCount > 0 && (
        <div className="flex items-center gap-2.5 rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-900 dark:border-amber-500/25 dark:bg-amber-500/10 dark:text-amber-200">
          <AlertTriangle className="h-4 w-4 shrink-0 text-amber-600 dark:text-amber-400" />
          <span>{t.dashboard.otherBackupsWarning}</span>
        </div>
      )}

      {/* DNS Info + Network Info */}
      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <section className="rounded-xl border border-slate-200 bg-white p-6 dark:border-slate-800 dark:bg-slate-900">
          <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100">
            {t.dashboard.dnsInfoTitle}
          </h3>
          <dl className="mt-4 divide-y divide-slate-100 text-sm dark:divide-slate-800">
            <div className="flex items-center justify-between gap-4 py-2.5">
              <dt className="text-slate-500 dark:text-slate-400">
                {t.dashboard.selectedProvider}
              </dt>
              <dd className="font-medium text-slate-900 dark:text-slate-100">
                {displayedProvider ? displayedProvider.name : t.common.none}
              </dd>
            </div>
            <div className="flex items-center justify-between gap-4 py-2.5">
              <dt className="text-slate-500 dark:text-slate-400">
                {t.dashboard.primaryDns}
              </dt>
              <dd className="font-mono-tech text-slate-900 dark:text-slate-100">
                {displayedProvider?.primaryDns ?? '—'}
              </dd>
            </div>
            <div className="flex items-center justify-between gap-4 py-2.5">
              <dt className="text-slate-500 dark:text-slate-400">
                {t.dashboard.secondaryDns}
              </dt>
              <dd className="font-mono-tech text-slate-900 dark:text-slate-100">
                {displayedProvider?.secondaryDns ?? '—'}
              </dd>
            </div>
            <div className="flex items-center justify-between gap-4 py-2.5">
              <dt className="text-slate-500 dark:text-slate-400">
                {t.dashboard.activeAdapter}
              </dt>
              <dd className="font-medium text-slate-900 dark:text-slate-100">
                {selectedAdapter ? selectedAdapter.name : t.common.none}
              </dd>
            </div>
            <div className="flex items-center justify-between gap-4 py-2.5">
              <dt className="text-slate-500 dark:text-slate-400">
                {t.dashboard.currentDnsMode}
              </dt>
              <dd
                className={`font-medium ${
                  isDnsModeIndeterminate
                    ? 'text-rose-600 dark:text-rose-400'
                    : 'text-slate-900 dark:text-slate-100'
                }`}
              >
                {selectedAdapter
                  ? isDnsModeIndeterminate
                    ? t.dnsModes.indeterminate
                    : t.dnsModes[selectedAdapter.dnsMode]
                  : t.common.unknown}
              </dd>
            </div>
          </dl>
        </section>

        <section className="rounded-xl border border-slate-200 bg-white p-6 dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center justify-between gap-2">
            <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100">
              {t.dashboard.networkInfoTitle}
            </h3>
            {!selectedAdapter && (
              <button
                type="button"
                onClick={() => onNavigate('adapters')}
                className="text-xs font-medium text-sky-600 hover:underline dark:text-sky-400"
              >
                {t.dashboard.goToAdapters}
              </button>
            )}
          </div>

          {!selectedAdapter ? (
            <p className="mt-4 text-sm text-slate-500 dark:text-slate-400">
              {t.dashboard.noAdapterSelected}
            </p>
          ) : (
            <dl className="mt-4 divide-y divide-slate-100 text-sm dark:divide-slate-800">
              <div className="flex items-center justify-between gap-4 py-2.5">
                <dt className="text-slate-500 dark:text-slate-400">
                  {t.dashboard.adapterName}
                </dt>
                <dd className="font-medium text-slate-900 dark:text-slate-100">
                  {selectedAdapter.name}
                </dd>
              </div>
              <div className="flex items-center justify-between gap-4 py-2.5">
                <dt className="text-slate-500 dark:text-slate-400">
                  {t.dashboard.adapterType}
                </dt>
                <dd className="text-slate-800 dark:text-slate-200">
                  {t.adapterTypes[selectedAdapter.adapterType]}
                </dd>
              </div>
              <div className="flex items-center justify-between gap-4 py-2.5">
                <dt className="text-slate-500 dark:text-slate-400">
                  {t.dashboard.adapterStatus}
                </dt>
                <dd className="font-medium text-slate-900 dark:text-slate-100">
                  {t.adapterStatus[selectedAdapter.operationalStatus]}
                </dd>
              </div>
              <div className="flex items-center justify-between gap-4 py-2.5">
                <dt className="text-slate-500 dark:text-slate-400">
                  {t.dashboard.ipv4Address}
                </dt>
                <dd className="font-mono-tech text-slate-900 dark:text-slate-100">
                  {selectedAdapter.ipv4Addresses.length > 0
                    ? selectedAdapter.ipv4Addresses.join(', ')
                    : '—'}
                </dd>
              </div>
              <div className="flex items-center justify-between gap-4 py-2.5">
                <dt className="text-slate-500 dark:text-slate-400">
                  {t.dashboard.gateway}
                </dt>
                <dd className="font-mono-tech text-slate-900 dark:text-slate-100">
                  {selectedAdapter.ipv4Gateway ?? '—'}
                </dd>
              </div>
              <div className="flex items-center justify-between gap-4 py-2.5">
                <dt className="text-slate-500 dark:text-slate-400">
                  {t.dashboard.currentDnsServers}
                </dt>
                <dd className="font-mono-tech text-sky-700 dark:text-sky-400">
                  {selectedAdapter.currentIpv4DnsServers.length > 0
                    ? selectedAdapter.currentIpv4DnsServers.join(', ')
                    : t.common.none}
                </dd>
              </div>
              <div className="flex items-center justify-between gap-4 py-2.5">
                <dt className="text-slate-500 dark:text-slate-400">
                  {t.dashboard.dhcpStatus}
                </dt>
                <dd className="text-slate-800 dark:text-slate-200">
                  {t.dhcpStatuses[selectedAdapter.dhcpStatus]}
                </dd>
              </div>
            </dl>
          )}
        </section>
      </div>

      {/* Operation Info + Backup & Recovery */}
      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <section className="rounded-xl border border-slate-200 bg-white p-6 dark:border-slate-800 dark:bg-slate-900">
          <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100">
            {t.dashboard.operationInfoTitle}
          </h3>
          <dl className="mt-4 divide-y divide-slate-100 text-sm dark:divide-slate-800">
            <div className="flex items-center justify-between gap-4 py-2.5">
              <dt className="text-slate-500 dark:text-slate-400">
                {t.dashboard.lastOperation}
              </dt>
              <dd className="font-medium text-slate-900 dark:text-slate-100">
                {operationLabel}
              </dd>
            </div>
            <div className="flex items-center justify-between gap-4 py-2.5">
              <dt className="text-slate-500 dark:text-slate-400">
                {t.dashboard.lastTimestamp}
              </dt>
              <dd className="font-mono-tech text-slate-700 dark:text-slate-300">
                {lastOperation.timestamp ?? '—'}
              </dd>
            </div>
            <div className="flex items-center justify-between gap-4 py-2.5">
              <dt className="text-slate-500 dark:text-slate-400">
                {t.dashboard.lastResult}
              </dt>
              <dd
                className={`font-medium ${
                  lastOperation.outcome === 'error'
                    ? 'text-rose-600 dark:text-rose-400'
                    : lastOperation.outcome === 'success'
                      ? 'text-emerald-600 dark:text-emerald-400'
                      : 'text-slate-700 dark:text-slate-300'
                }`}
              >
                {outcomeLabel}
              </dd>
            </div>
            {lastOperation.message && (
              <div className="py-2.5">
                <dt className="text-slate-500 dark:text-slate-400">
                  {t.dashboard.errorDetail}
                </dt>
                <dd className="mt-1 text-xs leading-relaxed text-slate-700 dark:text-slate-300">
                  {lastOperation.message}
                  {lastOperation.errorCode ? (
                    <span className="ms-2 font-mono-tech text-slate-500 dark:text-slate-400">
                      ({lastOperation.errorCode})
                    </span>
                  ) : null}
                </dd>
              </div>
            )}
          </dl>
          {lastOperation.outcome === 'error' &&
            (lastOperation.technicalDetail ?? lastOperation.message) && (
              <TechnicalDetailsSection
                code={lastOperation.errorCode}
                detail={
                  lastOperation.technicalDetail ?? lastOperation.message ?? ''
                }
                t={t}
                className="mt-3 border-t border-slate-100 pt-3 dark:border-slate-800"
              />
            )}
        </section>

        <section className="rounded-xl border border-slate-200 bg-white p-6 dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center gap-2">
            <ShieldCheck className="h-4 w-4 text-sky-600 dark:text-sky-400" />
            <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100">
              {t.dashboard.backupSectionTitle}
            </h3>
          </div>

          <dl className="mt-4 divide-y divide-slate-100 text-sm dark:divide-slate-800">
            <div className="flex items-center justify-between gap-4 py-2.5">
              <dt className="text-slate-500 dark:text-slate-400">
                {t.dashboard.backupAvailability}
              </dt>
              <dd
                className={`font-medium ${
                  selectedBackup?.hasBackup && !selectedBackup.isValid
                    ? 'text-rose-600 dark:text-rose-400'
                    : selectedBackup?.hasBackup
                      ? 'text-emerald-600 dark:text-emerald-400'
                      : 'text-slate-600 dark:text-slate-400'
                }`}
              >
                {selectedBackup?.hasBackup
                  ? selectedBackup.isValid
                    ? t.dashboard.backupAvailable
                    : t.dashboard.backupCorrupted
                  : t.dashboard.backupUnavailable}
              </dd>
            </div>

            {selectedBackup?.hasBackup && !selectedBackup.isValid && (
              <div className="space-y-3 py-3">
                {selectedBackup.integrityError && (
                  <p className="font-mono-tech text-[11px] leading-relaxed text-rose-700 dark:text-rose-300">
                    {selectedBackup.integrityError}
                  </p>
                )}
                {desktopRuntime && (
                  <button
                    type="button"
                    onClick={onDiscardCorruptBackup}
                    disabled={isMutating}
                    className="inline-flex items-center gap-2 rounded-lg border border-rose-300 bg-rose-50 px-3.5 py-2 text-xs font-semibold text-rose-700 transition-colors hover:bg-rose-100 disabled:cursor-not-allowed disabled:opacity-50 dark:border-rose-500/40 dark:bg-rose-500/15 dark:text-rose-300 dark:hover:bg-rose-500/25"
                  >
                    <AlertTriangle className="h-3.5 w-3.5" />
                    <span>{t.dashboard.discardCorruptBackup}</span>
                  </button>
                )}
              </div>
            )}

            {selectedBackup?.backup && (
              <>
                <div className="flex items-center justify-between gap-4 py-2.5">
                  <dt className="text-slate-500 dark:text-slate-400">
                    {t.dashboard.backupAdapter}
                  </dt>
                  <dd className="font-medium text-slate-900 dark:text-slate-100">
                    {selectedBackup.backup.adapterNameAtBackup}
                  </dd>
                </div>
                <div className="flex items-center justify-between gap-4 py-2.5">
                  <dt className="text-slate-500 dark:text-slate-400">
                    {t.dashboard.backupTimestamp}
                  </dt>
                  <dd className="font-mono-tech text-xs text-slate-700 dark:text-slate-300">
                    {selectedBackup.backup.backupTimestamp}
                  </dd>
                </div>
                <div className="flex items-center justify-between gap-4 py-2.5">
                  <dt className="text-slate-500 dark:text-slate-400">
                    {t.dashboard.backupOriginalMode}
                  </dt>
                  <dd className="text-slate-900 dark:text-slate-100">
                    {t.dnsModes[selectedBackup.backup.originalDnsMode]}
                  </dd>
                </div>
                <div className="flex items-center justify-between gap-4 py-2.5">
                  <dt className="text-slate-500 dark:text-slate-400">
                    {t.dashboard.backupOriginalServers}
                  </dt>
                  <dd className="font-mono-tech text-slate-900 dark:text-slate-100">
                    {selectedBackup.backup.originalIpv4DnsServers.length > 0
                      ? selectedBackup.backup.originalIpv4DnsServers.join(', ')
                      : t.dnsModes.automatic}
                  </dd>
                </div>
              </>
            )}
          </dl>
        </section>
      </div>
    </div>
  );
}
