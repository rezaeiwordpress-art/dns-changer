import {
  AlertTriangle,
  CheckCircle2,
  Info,
  X,
  XCircle,
} from 'lucide-react';
import type { Translations } from '../localization';
import type { ToastNotification } from '../types/dns';

interface NotificationStackProps {
  notifications: ToastNotification[];
  t: Translations;
  onDismiss: (id: string) => void;
}

export function NotificationStack({
  notifications,
  t,
  onDismiss,
}: NotificationStackProps) {
  if (notifications.length === 0) return null;

  return (
    <div
      className="fixed bottom-5 end-5 z-40 flex w-full max-w-sm flex-col gap-2.5"
      aria-live="polite"
    >
      {notifications.map((item) => {
        const icon =
          item.variant === 'success' ? (
            <CheckCircle2 className="h-4 w-4 shrink-0 text-emerald-600 dark:text-emerald-400" />
          ) : item.variant === 'warning' ? (
            <AlertTriangle className="h-4 w-4 shrink-0 text-amber-600 dark:text-amber-400" />
          ) : item.variant === 'error' ? (
            <XCircle className="h-4 w-4 shrink-0 text-rose-600 dark:text-rose-400" />
          ) : (
            <Info className="h-4 w-4 shrink-0 text-sky-600 dark:text-sky-400" />
          );

        const borderTone =
          item.variant === 'success'
            ? 'border-emerald-200 dark:border-emerald-500/30'
            : item.variant === 'warning'
              ? 'border-amber-200 dark:border-amber-500/30'
              : item.variant === 'error'
                ? 'border-rose-200 dark:border-rose-500/30'
                : 'border-sky-200 dark:border-sky-500/30';

        return (
          <div
            key={item.id}
            className={`flex items-start gap-3 rounded-xl border bg-white p-3.5 shadow-lg dark:bg-slate-900 ${borderTone}`}
          >
            <div className="mt-0.5">{icon}</div>
            <div className="min-w-0 flex-1">
              <div className="text-xs font-semibold text-slate-900 dark:text-slate-100">
                {item.title}
              </div>
              <p className="mt-0.5 text-xs leading-relaxed text-slate-600 dark:text-slate-300">
                {item.message}
              </p>
              {item.errorCode && (
                <div className="mt-1 font-mono-tech text-[11px] text-slate-500 dark:text-slate-400">
                  {item.errorCode}
                </div>
              )}
            </div>
            <button
              type="button"
              onClick={() => onDismiss(item.id)}
              aria-label={t.common.close}
              className="rounded p-1 text-slate-400 hover:bg-slate-100 hover:text-slate-600 dark:hover:bg-slate-800 dark:hover:text-slate-200"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        );
      })}
    </div>
  );
}
