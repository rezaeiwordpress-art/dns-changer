import {
  Check,
  Filter,
  Globe,
  Lock,
  Shield,
  SlidersHorizontal,
  Zap,
} from 'lucide-react';
import type { Translations } from '../localization';
import type { DnsProvider, DnsProviderCatalog, NavPage } from '../types/dns';

interface ProvidersPageProps {
  t: Translations;
  catalog: DnsProviderCatalog;
  selectedProvider: DnsProvider | null;
  onSelectProvider: (id: string) => void;
  onNavigate: (page: NavPage) => void;
}

function ProviderVisualIcon({ provider }: { provider: DnsProvider }) {
  if (!provider.isBuiltIn) {
    return <SlidersHorizontal className="h-4 w-4" />;
  }
  switch (provider.id) {
    case 'cloudflare':
      return <Zap className="h-4 w-4" />;
    case 'quad9':
    case 'adguard':
      return <Shield className="h-4 w-4" />;
    case 'cleanbrowsing-adult':
    case 'cleanbrowsing-family':
      return <Filter className="h-4 w-4" />;
    case 'dns-watch':
      return <Lock className="h-4 w-4" />;
    default:
      return <Globe className="h-4 w-4" />;
  }
}

export function ProvidersPage({
  t,
  catalog,
  selectedProvider,
  onSelectProvider,
  onNavigate,
}: ProvidersPageProps) {
  const getDescription = (p: DnsProvider) =>
    (p.isBuiltIn ? t.providers.providerDescriptions[p.id] : p.description) ??
    p.description ??
    '';

  const getCategory = (p: DnsProvider) => {
    if (!p.category) return p.isBuiltIn ? t.common.builtIn : t.common.custom;
    return t.providers.providerCategories[p.category] ?? p.category;
  };

  const renderCard = (provider: DnsProvider) => {
    const selected = selectedProvider?.id === provider.id;
    return (
      <button
        key={provider.id}
        type="button"
        onClick={() => onSelectProvider(provider.id)}
        className={`flex flex-col justify-between rounded-xl border p-5 text-start transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-500 ${
          selected
            ? 'border-sky-500 bg-sky-50/70 dark:border-sky-400 dark:bg-sky-500/10'
            : 'border-slate-200 bg-white hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700'
        }`}
      >
        <div>
          <div className="flex items-start justify-between gap-3">
            <div className="flex items-center gap-2.5">
              <div
                className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-lg ${
                  selected
                    ? 'bg-sky-600 text-white'
                    : 'bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300'
                }`}
              >
                <ProviderVisualIcon provider={provider} />
              </div>
              <div>
                <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                  {provider.name}
                </div>
                <div className="text-xs text-slate-500 dark:text-slate-400">
                  {provider.isBuiltIn ? t.common.builtIn : t.common.custom}
                  {' · '}
                  {getCategory(provider)}
                </div>
              </div>
            </div>

            {selected && (
              <span className="inline-flex items-center gap-1 text-xs font-semibold text-sky-700 dark:text-sky-300">
                <Check className="h-4 w-4" />
                <span>{t.common.selected}</span>
              </span>
            )}
          </div>

          {getDescription(provider) && (
            <p className="mt-3 text-xs leading-relaxed text-slate-600 dark:text-slate-400">
              {getDescription(provider)}
            </p>
          )}
        </div>

        <div className="mt-4 flex items-center justify-between border-t border-slate-100 pt-3 font-mono-tech text-xs text-slate-700 dark:border-slate-800 dark:text-slate-300">
          <span>{provider.primaryDns}</span>
          {provider.secondaryDns ? (
            <>
              <span className="text-slate-400">/</span>
              <span>{provider.secondaryDns}</span>
            </>
          ) : null}
        </div>
      </button>
    );
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 className="text-xl font-semibold text-slate-900 dark:text-slate-100">
            {t.providers.title}
          </h2>
          <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
            {t.providers.subtitle}
          </p>
        </div>
        <button
          type="button"
          onClick={() => onNavigate('customDns')}
          className="rounded-lg border border-slate-200 bg-white px-4 py-2 text-xs font-medium text-slate-700 transition-colors hover:bg-slate-50 whitespace-nowrap dark:border-slate-700 dark:bg-slate-800 dark:text-slate-200 dark:hover:bg-slate-700"
        >
          {t.providers.manageCustomLink}
        </button>
      </div>

      {selectedProvider && (
        <section className="rounded-xl border border-slate-200 bg-white p-5 dark:border-slate-800 dark:bg-slate-900">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div className="space-y-1">
              <div className="text-xs text-slate-500 dark:text-slate-400">
                {t.providers.selectedBanner}
              </div>
              <div className="flex flex-wrap items-center gap-2 text-sm font-semibold text-slate-900 dark:text-slate-100">
                <span>{selectedProvider.name}</span>
                <span className="text-slate-400">·</span>
                <span className="font-mono-tech text-sky-700 dark:text-sky-400">
                  {selectedProvider.primaryDns}
                  {selectedProvider.secondaryDns
                    ? ` / ${selectedProvider.secondaryDns}`
                    : ''}
                </span>
              </div>
              <p className="text-xs text-slate-500 dark:text-slate-400">
                {t.providers.selectionNotice}
              </p>
            </div>
            <button
              type="button"
              onClick={() => onNavigate('dashboard')}
              className="rounded-lg bg-sky-600 px-4 py-2 text-xs font-semibold text-white transition-colors hover:bg-sky-500 whitespace-nowrap"
            >
              {t.nav.dashboard}
            </button>
          </div>
        </section>
      )}

      <section className="space-y-3">
        <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">
          {t.providers.builtInSection} ({catalog.builtIn.length})
        </h3>
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
          {catalog.builtIn.map(renderCard)}
        </div>
      </section>

      <section className="space-y-3">
        <h3 className="text-sm font-semibold text-slate-800 dark:text-slate-200">
          {t.providers.customSection} ({catalog.custom.length})
        </h3>
        {catalog.custom.length === 0 ? (
          <div className="rounded-xl border border-dashed border-slate-300 bg-white p-6 text-sm text-slate-500 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-400">
            {t.providers.emptyCustom}
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
            {catalog.custom.map(renderCard)}
          </div>
        )}
      </section>
    </div>
  );
}
