import { Monitor, Moon, Sun } from 'lucide-react';
import type { Translations } from '../localization';
import type { SavePreferencesPayload } from '../services/tauriClient';
import type { AppConfig, AppLanguage, NavPage, ThemeMode } from '../types/dns';

interface SettingsPageProps {
  t: Translations;
  config: AppConfig;
  onUpdatePreferences: (patch: SavePreferencesPayload) => Promise<void>;
}

const START_PAGES: NavPage[] = [
  'dashboard',
  'providers',
  'customDns',
  'adapters',
  'settings',
  'about',
];

export function SettingsPage({
  t,
  config,
  onUpdatePreferences,
}: SettingsPageProps) {
  const themeOptions: {
    id: ThemeMode;
    label: string;
    icon: typeof Sun;
  }[] = [
    { id: 'light', label: t.settings.themeLight, icon: Sun },
    { id: 'dark', label: t.settings.themeDark, icon: Moon },
    { id: 'system', label: t.settings.themeSystem, icon: Monitor },
  ];

  const languageOptions: { id: AppLanguage; label: string }[] = [
    { id: 'fa', label: t.settings.languageFa },
    { id: 'en', label: t.settings.languageEn },
  ];

  const renderToggle = (
    checked: boolean,
    onChange: (next: boolean) => void,
    label: string,
    description: string,
  ) => (
    <label className="flex cursor-pointer items-start justify-between gap-4 py-3.5">
      <div>
        <div className="text-sm font-medium text-slate-900 dark:text-slate-100">
          {label}
        </div>
        <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
          {description}
        </p>
      </div>
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="mt-1 h-4 w-4 shrink-0 rounded border-slate-300 text-sky-600 focus:ring-sky-500 dark:border-slate-600 dark:bg-slate-800"
      />
    </label>
  );

  return (
    <div className="max-w-3xl space-y-6">
      <div>
        <h2 className="text-xl font-semibold text-slate-900 dark:text-slate-100">
          {t.settings.title}
        </h2>
        <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
          {t.settings.subtitle}
        </p>
      </div>

      {/* Appearance */}
      <section className="rounded-xl border border-slate-200 bg-white p-6 dark:border-slate-800 dark:bg-slate-900">
        <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100">
          {t.settings.appearanceTitle}
        </h3>
        <div className="mt-4 space-y-2">
          <div className="text-xs font-medium text-slate-600 dark:text-slate-400">
            {t.settings.themeLabel}
          </div>
          <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-3">
            {themeOptions.map(({ id, label, icon: Icon }) => {
              const active = config.theme === id;
              return (
                <button
                  key={id}
                  type="button"
                  onClick={() => void onUpdatePreferences({ theme: id })}
                  className={`flex items-center justify-center gap-2 rounded-lg border px-4 py-2.5 text-xs font-medium transition-colors whitespace-nowrap ${
                    active
                      ? 'border-sky-500 bg-sky-50 text-sky-700 dark:border-sky-400 dark:bg-sky-500/15 dark:text-sky-300'
                      : 'border-slate-200 bg-white text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300 dark:hover:bg-slate-700'
                  }`}
                >
                  <Icon className="h-4 w-4 shrink-0" />
                  <span>{label}</span>
                </button>
              );
            })}
          </div>
        </div>
      </section>

      {/* Language */}
      <section className="rounded-xl border border-slate-200 bg-white p-6 dark:border-slate-800 dark:bg-slate-900">
        <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100">
          {t.settings.languageTitle}
        </h3>
        <div className="mt-4 space-y-2">
          <div className="text-xs font-medium text-slate-600 dark:text-slate-400">
            {t.settings.languageLabel}
          </div>
          <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2">
            {languageOptions.map((opt) => {
              const active = config.language === opt.id;
              return (
                <button
                  key={opt.id}
                  type="button"
                  onClick={() => void onUpdatePreferences({ language: opt.id })}
                  className={`rounded-lg border px-4 py-2.5 text-xs font-medium transition-colors whitespace-nowrap ${
                    active
                      ? 'border-sky-500 bg-sky-50 text-sky-700 dark:border-sky-400 dark:bg-sky-500/15 dark:text-sky-300'
                      : 'border-slate-200 bg-white text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300 dark:hover:bg-slate-700'
                  }`}
                >
                  {opt.label}
                </button>
              );
            })}
          </div>
        </div>
      </section>

      {/* Application Behavior */}
      <section className="rounded-xl border border-slate-200 bg-white p-6 dark:border-slate-800 dark:bg-slate-900">
        <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100">
          {t.settings.behaviorTitle}
        </h3>

        <div className="mt-3 divide-y divide-slate-100 dark:divide-slate-800">
          {renderToggle(
            config.rememberSelectedAdapter,
            (val) => void onUpdatePreferences({ rememberSelectedAdapter: val }),
            t.settings.rememberAdapter,
            t.settings.rememberAdapterDesc,
          )}
          {renderToggle(
            config.rememberSelectedProvider,
            (val) => void onUpdatePreferences({ rememberSelectedProvider: val }),
            t.settings.rememberProvider,
            t.settings.rememberProviderDesc,
          )}
          {renderToggle(
            config.confirmBeforeRestore,
            (val) => void onUpdatePreferences({ confirmBeforeRestore: val }),
            t.settings.confirmBeforeRestore,
            t.settings.confirmBeforeRestoreDesc,
          )}
          {renderToggle(
            config.showNotifications,
            (val) => void onUpdatePreferences({ showNotifications: val }),
            t.settings.showNotifications,
            t.settings.showNotificationsDesc,
          )}

          <div className="flex flex-wrap items-center justify-between gap-4 pt-4">
            <label
              htmlFor="start-page-select"
              className="text-sm font-medium text-slate-900 dark:text-slate-100"
            >
              {t.settings.startPageLabel}
            </label>
            <select
              id="start-page-select"
              value={config.startPage}
              onChange={(e) =>
                void onUpdatePreferences({ startPage: e.target.value as NavPage })
              }
              className="rounded-lg border border-slate-300 bg-white px-3 py-2 text-xs font-medium text-slate-900 focus:border-sky-500 focus:outline-none dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
            >
              {START_PAGES.map((page) => (
                <option key={page} value={page}>
                  {t.nav[page]}
                </option>
              ))}
            </select>
          </div>
        </div>
      </section>
    </div>
  );
}
