import { AlertCircle, Check, ChevronDown, Copy, RefreshCw } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import type { Translations } from '../localization';
import type { CommandError } from '../types/dns';

interface TechnicalDetailsSectionProps {
  code?: string | null;
  detail: string;
  t: Translations;
  className?: string;
}

export function TechnicalDetailsSection({
  code,
  detail,
  t,
  className = '',
}: TechnicalDetailsSectionProps) {
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [copied, setCopied] = useState(false);
  const resetTimerRef = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (resetTimerRef.current !== null) {
        window.clearTimeout(resetTimerRef.current);
      }
    };
  }, []);

  const handleCopy = async () => {
    const payload = code ? `${code}\n${detail}` : detail;
    try {
      if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(payload);
        setCopied(true);
        if (resetTimerRef.current !== null) {
          window.clearTimeout(resetTimerRef.current);
        }
        resetTimerRef.current = window.setTimeout(() => {
          setCopied(false);
          resetTimerRef.current = null;
        }, 2000);
      }
    } catch {
      // Clipboard API may be unavailable in restricted contexts; text remains selectable.
    }
  };

  return (
    <div className={className}>
      <button
        type="button"
        onClick={() => setDetailsOpen((prev) => !prev)}
        aria-expanded={detailsOpen}
        className="inline-flex items-center gap-1.5 text-xs font-medium text-rose-800 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-rose-500 dark:text-rose-300"
      >
        <ChevronDown
          className={`h-3.5 w-3.5 transition-transform ${
            detailsOpen ? 'rotate-180' : ''
          }`}
        />
        <span>{t.common.technicalDetails}</span>
      </button>

      {detailsOpen && (
        <div className="mt-2 rounded-lg border border-rose-200 bg-white/90 p-3 dark:border-rose-500/20 dark:bg-slate-950/70">
          <div className="mb-2 flex items-center justify-between gap-2">
            <span className="font-mono-tech text-[11px] font-medium text-rose-700 dark:text-rose-300">
              {code ?? ''}
            </span>
            <button
              type="button"
              onClick={handleCopy}
              className="inline-flex items-center gap-1 rounded border border-rose-200 bg-rose-50 px-2 py-1 text-[11px] font-medium text-rose-800 transition-colors hover:bg-rose-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-rose-500 dark:border-rose-500/30 dark:bg-rose-500/15 dark:text-rose-200 dark:hover:bg-rose-500/25"
            >
              {copied ? (
                <Check className="h-3 w-3 text-emerald-600 dark:text-emerald-400" />
              ) : (
                <Copy className="h-3 w-3" />
              )}
              <span>{copied ? t.common.copied : t.common.copy}</span>
            </button>
          </div>
          <pre
            dir="ltr"
            className="select-text whitespace-pre-wrap break-words text-left font-mono-tech text-xs leading-relaxed text-rose-900 dark:text-rose-200"
          >
            {detail}
          </pre>
        </div>
      )}
    </div>
  );
}

interface AdapterErrorCardProps {
  error: CommandError;
  t: Translations;
  desktopRuntime: boolean;
  isRetrying: boolean;
  onRetry: () => void;
}

export function AdapterErrorCard({
  error,
  t,
  desktopRuntime,
  isRetrying,
  onRetry,
}: AdapterErrorCardProps) {
  const localizedMessage = t.errors[error.code] ?? error.message;
  const detailText = error.technicalDetail ?? error.message;

  return (
    <section
      role="alert"
      className="rounded-xl border border-rose-200 bg-rose-50/80 p-5 text-rose-950 dark:border-rose-500/30 dark:bg-rose-500/10 dark:text-rose-100"
    >
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex min-w-0 flex-1 items-start gap-3">
          <AlertCircle className="mt-0.5 h-5 w-5 shrink-0 text-rose-600 dark:text-rose-400" />
          <div className="min-w-0 flex-1 space-y-2">
            <div className="flex flex-wrap items-center gap-2.5">
              <h3 className="text-sm font-semibold text-rose-900 dark:text-rose-100">
                {t.common.adapterLoadErrorTitle}
              </h3>
              <code className="rounded bg-rose-100 px-2 py-0.5 font-mono-tech text-xs font-medium text-rose-800 dark:bg-rose-500/20 dark:text-rose-200">
                {error.code}
              </code>
            </div>
            <p className="text-sm leading-relaxed text-rose-900 dark:text-rose-200">
              {localizedMessage}
            </p>
          </div>
        </div>

        <button
          type="button"
          onClick={onRetry}
          disabled={!desktopRuntime || isRetrying}
          className="inline-flex shrink-0 items-center gap-2 rounded-lg border border-rose-300 bg-white px-3.5 py-2 text-xs font-semibold text-rose-800 transition-colors hover:bg-rose-100 disabled:cursor-not-allowed disabled:opacity-50 whitespace-nowrap focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-rose-500 dark:border-rose-500/40 dark:bg-slate-900 dark:text-rose-200 dark:hover:bg-rose-500/20"
        >
          <RefreshCw className={`h-3.5 w-3.5 ${isRetrying ? 'animate-spin' : ''}`} />
          <span>{isRetrying ? t.common.refreshing : t.common.retry}</span>
        </button>
      </div>

      <TechnicalDetailsSection
        code={error.code}
        detail={detailText}
        t={t}
        className="mt-3 border-t border-rose-200/70 pt-3 dark:border-rose-500/20"
      />
    </section>
  );
}
