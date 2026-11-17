import {
  AlertCircle,
  Cable,
  Check,
  Cpu,
  Network,
  RefreshCw,
  Wifi,
  WifiOff,
} from 'lucide-react';
import { AdapterErrorCard } from '../components/AdapterErrorCard';
import type { Translations } from '../localization';
import type { CommandError, NetworkAdapter } from '../types/dns';

interface AdaptersPageProps {
  t: Translations;
  desktopRuntime: boolean;
  adapters: NetworkAdapter[];
  selectedGuid: string | null;
  selectedAdapter: NetworkAdapter | null;
  adapterError: CommandError | null;
  isLoading: boolean;
  onSelectAdapter: (guid: string) => void;
  onRefresh: () => void;
}

function AdapterTypeIcon({ type }: { type: NetworkAdapter['adapterType'] }) {
  if (type === 'wifi') return <Wifi className="h-4 w-4 text-sky-600 dark:text-sky-400" />;
  if (type === 'ethernet') return <Cable className="h-4 w-4 text-emerald-600 dark:text-emerald-400" />;
  if (type === 'virtual') return <Cpu className="h-4 w-4 text-cyan-600 dark:text-cyan-400" />;
  return <Network className="h-4 w-4 text-slate-500 dark:text-slate-400" />;
}

export function AdaptersPage({
  t,
  desktopRuntime,
  adapters,
  selectedGuid,
  selectedAdapter,
  adapterError,
  isLoading,
  onSelectAdapter,
  onRefresh,
}: AdaptersPageProps) {
  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 className="text-xl font-semibold text-slate-900 dark:text-slate-100">
            {t.adapters.title}
          </h2>
          <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
            {t.adapters.subtitle}
          </p>
        </div>
        <button
          type="button"
          onClick={onRefresh}
          disabled={!desktopRuntime || isLoading}
          className="inline-flex items-center gap-2 rounded-lg border border-slate-200 bg-white px-4 py-2 text-xs font-medium text-slate-700 transition-colors hover:bg-slate-50 disabled:opacity-50 whitespace-nowrap dark:border-slate-700 dark:bg-slate-800 dark:text-slate-200 dark:hover:bg-slate-700"
        >
          <RefreshCw className={`h-3.5 w-3.5 ${isLoading ? 'animate-spin' : ''}`} />
          <span>{isLoading ? t.common.refreshing : t.common.refresh}</span>
        </button>
      </div>

      <p className="text-xs text-slate-500 dark:text-slate-400">
        {t.adapters.selectionSafeNote}
      </p>

      {adapterError && (
        <AdapterErrorCard
          error={adapterError}
          t={t}
          desktopRuntime={desktopRuntime}
          isRetrying={isLoading}
          onRetry={onRefresh}
        />
      )}

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-5">
        <div className="space-y-3 lg:col-span-3">
          {adapters.length === 0 ? (
            <div className="rounded-xl border border-dashed border-slate-300 bg-white p-8 text-center text-sm text-slate-500 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-400">
              {t.adapters.emptyList}
            </div>
          ) : (
            adapters.map((adapter) => {
              const selected = adapter.guid === selectedGuid;
              const isDisabled = adapter.operationalStatus === 'disabled';
              const isDisconnected = adapter.operationalStatus === 'disconnected';
              const statusTone =
                adapter.operationalStatus === 'up'
                  ? 'text-emerald-700 dark:text-emerald-400'
                  : isDisabled
                    ? 'text-rose-700 dark:text-rose-400'
                    : 'text-amber-700 dark:text-amber-400';

              return (
                <button
                  key={adapter.guid}
                  type="button"
                  onClick={() => onSelectAdapter(adapter.guid)}
                  className={`w-full rounded-xl border p-5 text-start transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-500 ${
                    selected
                      ? 'border-sky-500 bg-sky-50/70 dark:border-sky-400 dark:bg-sky-500/10'
                      : 'border-slate-200 bg-white hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700'
                  }`}
                >
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="flex items-center gap-3">
                      <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-slate-100 dark:bg-slate-800">
                        <AdapterTypeIcon type={adapter.adapterType} />
                      </div>
                      <div>
                        <div className="flex items-center gap-2">
                          <span className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                            {adapter.name}
                          </span>
                          {selected && (
                            <span className="inline-flex items-center gap-1 text-xs font-semibold text-sky-700 dark:text-sky-300">
                              <Check className="h-3.5 w-3.5" />
                              <span>{t.adapters.activeBadge}</span>
                            </span>
                          )}
                        </div>
                        <div className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
                          {adapter.interfaceDescription}
                        </div>
                      </div>
                    </div>

                    <div className={`inline-flex items-center gap-1.5 text-xs font-medium ${statusTone}`}>
                      {isDisabled ? (
                        <AlertCircle className="h-3.5 w-3.5" />
                      ) : isDisconnected ? (
                        <WifiOff className="h-3.5 w-3.5" />
                      ) : null}
                      <span>{t.adapterStatus[adapter.operationalStatus]}</span>
                    </div>
                  </div>

                  <div className="mt-4 grid grid-cols-1 gap-2 border-t border-slate-100 pt-3 text-xs sm:grid-cols-3 dark:border-slate-800">
                    <div>
                      <span className="text-slate-500 dark:text-slate-400">
                        {t.adapters.ipv4Label}:{' '}
                      </span>
                      <span className="font-mono-tech text-slate-800 dark:text-slate-200">
                        {adapter.ipv4Addresses[0] ?? '—'}
                      </span>
                    </div>
                    <div>
                      <span className="text-slate-500 dark:text-slate-400">
                        {t.adapters.dnsLabel}:{' '}
                      </span>
                      <span className="font-mono-tech text-sky-700 dark:text-sky-400">
                        {adapter.currentIpv4DnsServers.length > 0
                          ? adapter.currentIpv4DnsServers.join(', ')
                          : t.common.none}
                      </span>
                    </div>
                    <div>
                      <span className="text-slate-500 dark:text-slate-400">
                        {t.adapters.dhcpLabel}:{' '}
                      </span>
                      <span className="text-slate-800 dark:text-slate-200">
                        {t.dhcpStatuses[adapter.dhcpStatus]}
                      </span>
                    </div>
                  </div>

                  {isDisabled && (
                    <p className="mt-2.5 text-xs text-rose-700 dark:text-rose-400">
                      {t.adapters.disabledHint}
                    </p>
                  )}
                  {isDisconnected && (
                    <p className="mt-2.5 text-xs text-amber-700 dark:text-amber-400">
                      {t.adapters.disconnectedHint}
                    </p>
                  )}
                  {adapter.dnsModeIndeterminateReason && (
                    <p className="mt-2.5 text-xs text-rose-700 dark:text-rose-400">
                      {t.dnsModes.indeterminate}: {adapter.dnsModeIndeterminateReason}
                    </p>
                  )}
                </button>
              );
            })
          )}
        </div>

        <section className="h-fit rounded-xl border border-slate-200 bg-white p-6 lg:col-span-2 dark:border-slate-800 dark:bg-slate-900">
          <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100">
            {t.adapters.detailsTitle}
          </h3>

          {!selectedAdapter ? (
            <p className="mt-4 text-sm text-slate-500 dark:text-slate-400">
              {t.adapters.emptyList}
            </p>
          ) : (
            <dl className="mt-4 divide-y divide-slate-100 text-xs dark:divide-slate-800">
              <div className="py-2.5">
                <dt className="text-slate-500 dark:text-slate-400">
                  {t.adapters.guidLabel}
                </dt>
                <dd className="mt-1 break-all font-mono-tech text-slate-900 dark:text-slate-100">
                  {selectedAdapter.guid}
                </dd>
              </div>
              <div className="flex items-center justify-between gap-4 py-2.5">
                <dt className="text-slate-500 dark:text-slate-400">
                  {t.adapters.indexLabel}
                </dt>
                <dd className="font-mono-tech text-slate-900 dark:text-slate-100">
                  #{selectedAdapter.interfaceIndex}
                </dd>
              </div>
              <div className="flex items-center justify-between gap-4 py-2.5">
                <dt className="text-slate-500 dark:text-slate-400">
                  {t.adapters.dnsModeLabel}
                </dt>
                <dd
                  className={`font-medium ${
                    selectedAdapter.dnsModeIndeterminateReason
                      ? 'text-rose-600 dark:text-rose-400'
                      : 'text-slate-900 dark:text-slate-100'
                  }`}
                >
                  {selectedAdapter.dnsModeIndeterminateReason
                    ? t.dnsModes.indeterminate
                    : t.dnsModes[selectedAdapter.dnsMode]}
                </dd>
              </div>
              <div className="flex items-center justify-between gap-4 py-2.5">
                <dt className="text-slate-500 dark:text-slate-400">
                  {t.adapters.dhcpLabel}
                </dt>
                <dd className="text-slate-900 dark:text-slate-100">
                  {t.dhcpStatuses[selectedAdapter.dhcpStatus]}
                </dd>
              </div>
              <div className="flex items-center justify-between gap-4 py-2.5">
                <dt className="text-slate-500 dark:text-slate-400">
                  {t.adapters.ipv4Label}
                </dt>
                <dd className="font-mono-tech text-slate-900 dark:text-slate-100">
                  {selectedAdapter.ipv4Addresses.length > 0
                    ? selectedAdapter.ipv4Addresses.join(', ')
                    : '—'}
                </dd>
              </div>
              <div className="flex items-center justify-between gap-4 py-2.5">
                <dt className="text-slate-500 dark:text-slate-400">
                  {t.adapters.gatewayLabel}
                </dt>
                <dd className="font-mono-tech text-slate-900 dark:text-slate-100">
                  {selectedAdapter.ipv4Gateway ?? '—'}
                </dd>
              </div>
              <div className="flex items-center justify-between gap-4 py-2.5">
                <dt className="text-slate-500 dark:text-slate-400">
                  {t.adapters.macLabel}
                </dt>
                <dd className="font-mono-tech text-slate-900 dark:text-slate-100">
                  {selectedAdapter.macAddress ?? '—'}
                </dd>
              </div>
              <div className="flex items-center justify-between gap-4 py-2.5">
                <dt className="text-slate-500 dark:text-slate-400">
                  {t.adapters.speedLabel}
                </dt>
                <dd className="font-mono-tech text-slate-900 dark:text-slate-100">
                  {selectedAdapter.linkSpeed ?? '—'}
                </dd>
              </div>
            </dl>
          )}
        </section>
      </div>
    </div>
  );
}
