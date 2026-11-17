import { AlertTriangle, RefreshCw } from 'lucide-react';
import type { Translations } from '../localization';
import type { NavPage } from '../types/dns';

interface TopBarProps {
  activePage: NavPage;
  t: Translations;
  desktopRuntime: boolean;
  isLoading: boolean;
  isMutating: boolean;
  onRefresh: () => void;
}

export function TopBar({
  activePage,
  t,
  desktopRuntime,
  isLoading,
  isMutating,
  onRefresh,
}: TopBarProps) {
  return (
    <div className="shrink-0">
      <header className="flex items-center justify-between gap-4 border-b border-slate-200 bg-white px-6 py-4 dark:border-slate-800 dark:bg-slate-900">
        <h1 className="truncate text-lg font-semibold text-slate-900 dark:text-slate-100">
          {t.nav[activePage]}
        </h1>

        <button
          type="button"
          onClick={onRefresh}
          disabled={!desktopRuntime || isLoading || isMutating}
          className="inline-flex shrink-0 items-center gap-2 rounded-lg border border-slate-200 bg-white px-3.5 py-2 text-xs font-medium text-slate-700 transition-colors hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50 whitespace-nowrap focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-500 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-200 dark:hover:bg-slate-700"
        >
          <RefreshCw
            className={`h-3.5 w-3.5 ${isLoading ? 'animate-spin' : ''}`}
          />
          <span>{isLoading ? t.common.refreshing : t.common.refresh}</span>
        </button>
      </header>

      {!desktopRuntime && (
        <div className="flex items-start gap-3 border-b border-amber-200 bg-amber-50 px-6 py-3 text-xs text-amber-900 dark:border-amber-500/20 dark:bg-amber-500/10 dark:text-amber-200">
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-amber-600 dark:text-amber-400" />
          <div className="space-y-0.5">
            <div className="font-semibold">{t.common.browserPreviewTitle}</div>
            <p className="leading-relaxed opacity-90">
              {t.common.browserPreviewBody}
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
