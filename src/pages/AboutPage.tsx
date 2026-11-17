import { BookOpen, Shield, Terminal } from 'lucide-react';
import type { Translations } from '../localization';

interface AboutPageProps {
  t: Translations;
}

const TECHNOLOGIES = [
  'Tauri 2',
  'Rust',
  'React 19',
  'TypeScript',
  'Tailwind CSS',
  'Lucide Icons',
  'Windows 10 / 11',
];

export function AboutPage({ t }: AboutPageProps) {
  return (
    <div className="max-w-3xl space-y-6">
      <div>
        <h2 className="text-xl font-semibold text-slate-900 dark:text-slate-100">
          {t.about.title}
        </h2>
        <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
          {t.about.subtitle}
        </p>
      </div>

      <section className="rounded-xl border border-slate-200 bg-white p-6 dark:border-slate-800 dark:bg-slate-900">
        <div className="flex items-start gap-4">
          <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-sky-600 text-white">
            <Shield className="h-6 w-6" />
          </div>
          <div className="space-y-2">
            <div className="flex flex-wrap items-baseline gap-2.5">
              <h3 className="text-base font-semibold text-slate-900 dark:text-slate-100">
                DNS Changer
              </h3>
              <span className="font-mono-tech text-xs text-sky-700 dark:text-sky-400">
                v1.0.0
              </span>
            </div>
            <p className="text-sm leading-relaxed text-slate-600 dark:text-slate-300">
              {t.about.descriptionBody}
            </p>
          </div>
        </div>

        <dl className="mt-6 divide-y divide-slate-100 border-t border-slate-100 text-sm dark:divide-slate-800 dark:border-slate-800">
          <div className="flex items-center justify-between gap-4 py-3">
            <dt className="text-slate-500 dark:text-slate-400">
              {t.about.appNameLabel}
            </dt>
            <dd className="font-medium text-slate-900 dark:text-slate-100">
              DNS Changer
            </dd>
          </div>
          <div className="flex items-center justify-between gap-4 py-3">
            <dt className="text-slate-500 dark:text-slate-400">
              {t.about.versionLabel}
            </dt>
            <dd className="font-mono-tech text-slate-900 dark:text-slate-100">
              1.0.0
            </dd>
          </div>
          <div className="py-3">
            <dt className="text-slate-500 dark:text-slate-400">
              {t.about.technologyTitle}
            </dt>
            <dd className="mt-1.5 text-xs leading-relaxed text-slate-700 dark:text-slate-300">
              {TECHNOLOGIES.join(' · ')}
            </dd>
          </div>
        </dl>
      </section>

      <section className="rounded-xl border border-slate-200 bg-white p-6 space-y-4 dark:border-slate-800 dark:bg-slate-900">
        <div className="flex items-center gap-2 text-sm font-semibold text-slate-900 dark:text-slate-100">
          <Terminal className="h-4 w-4 text-sky-600 dark:text-sky-400" />
          <span>{t.about.securityTitle}</span>
        </div>
        <p className="text-sm leading-relaxed text-slate-600 dark:text-slate-300">
          {t.about.securityBody}
        </p>

        <div className="border-t border-slate-100 pt-4 dark:border-slate-800">
          <div className="text-xs font-semibold text-slate-800 dark:text-slate-200">
            {t.about.developerTitle}
          </div>
          <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">
            {t.about.developerPlaceholder}
          </p>
        </div>

        <div className="border-t border-slate-100 pt-4 dark:border-slate-800">
          <div className="flex items-center gap-2 text-xs font-semibold text-slate-800 dark:text-slate-200">
            <BookOpen className="h-3.5 w-3.5 text-sky-600 dark:text-sky-400" />
            <span>{t.about.docsTitle}</span>
          </div>
          <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">
            {t.about.docsNote}
          </p>
        </div>

        <div className="border-t border-slate-100 pt-4 text-xs text-slate-500 dark:border-slate-800 dark:text-slate-400">
          <span className="font-semibold text-slate-700 dark:text-slate-300">
            {t.about.copyrightTitle}:{' '}
          </span>
          {t.about.copyrightBody}
        </div>
      </section>
    </div>
  );
}
