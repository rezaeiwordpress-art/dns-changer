import {
  Globe,
  Info,
  LayoutDashboard,
  Network,
  Settings,
  Shield,
  SlidersHorizontal,
} from 'lucide-react';
import type { Translations } from '../localization';
import type { AdminStatus, ConnectionStatus, NavPage } from '../types/dns';

interface SidebarProps {
  activePage: NavPage;
  onNavigate: (page: NavPage) => void;
  t: Translations;
  adminStatus: AdminStatus | null;
  connectionStatus: ConnectionStatus;
  desktopRuntime: boolean;
}

const NAV_ITEMS: { id: NavPage; icon: typeof LayoutDashboard }[] = [
  { id: 'dashboard', icon: LayoutDashboard },
  { id: 'providers', icon: Globe },
  { id: 'customDns', icon: SlidersHorizontal },
  { id: 'adapters', icon: Network },
  { id: 'settings', icon: Settings },
  { id: 'about', icon: Info },
];

export function Sidebar({
  activePage,
  onNavigate,
  t,
  adminStatus,
  connectionStatus,
  desktopRuntime,
}: SidebarProps) {
  const statusLabel =
    connectionStatus === 'connected'
      ? t.status.connected
      : connectionStatus === 'error'
        ? t.status.error
        : t.status.disconnected;

  const statusTone =
    connectionStatus === 'connected'
      ? 'text-emerald-600 dark:text-emerald-400'
      : connectionStatus === 'error'
        ? 'text-rose-600 dark:text-rose-400'
        : 'text-slate-500 dark:text-slate-400';

  return (
    <aside className="flex w-60 shrink-0 flex-col border-e border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-900">
      <div className="flex items-center gap-3 border-b border-slate-200 px-5 py-4 dark:border-slate-800">
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-sky-600 text-white">
          <Shield className="h-5 w-5" />
        </div>
        <div className="min-w-0">
          <div className="truncate text-sm font-semibold text-slate-900 dark:text-slate-100">
            {t.appName}
          </div>
          <div className={`truncate text-xs font-medium ${statusTone}`}>
            {statusLabel}
          </div>
        </div>
      </div>

      <nav className="flex-1 space-y-1 p-3" aria-label={t.appName}>
        {NAV_ITEMS.map(({ id, icon: Icon }) => {
          const active = activePage === id;
          return (
            <button
              key={id}
              type="button"
              onClick={() => onNavigate(id)}
              className={`flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors whitespace-nowrap focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-500 ${
                active
                  ? 'bg-sky-50 text-sky-700 dark:bg-sky-500/15 dark:text-sky-300'
                  : 'text-slate-600 hover:bg-slate-100 hover:text-slate-900 dark:text-slate-400 dark:hover:bg-slate-800/70 dark:hover:text-slate-100'
              }`}
            >
              <Icon className="h-4 w-4 shrink-0" />
              <span className="truncate">{t.nav[id]}</span>
            </button>
          );
        })}
      </nav>

      <div className="border-t border-slate-200 p-4 text-xs text-slate-500 dark:border-slate-800 dark:text-slate-400">
        {desktopRuntime ? (
          <div className="space-y-1">
            <div className="font-medium text-slate-700 dark:text-slate-300">
              {adminStatus?.isElevated
                ? t.common.elevatedAdmin
                : t.common.standardUser}
            </div>
            <div className="text-[11px] leading-relaxed text-slate-500 dark:text-slate-400">
              {t.common.desktopConnected}
            </div>
          </div>
        ) : (
          <div className="text-[11px] leading-relaxed">
            {t.common.browserPreviewTitle}
          </div>
        )}
      </div>
    </aside>
  );
}
