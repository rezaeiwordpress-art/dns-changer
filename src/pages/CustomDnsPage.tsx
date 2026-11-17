import { useState, type FormEvent } from 'react';
import { Pencil, Plus, Save, Trash2, X } from 'lucide-react';
import { ConfirmDialog } from '../components/ConfirmDialog';
import { formatTemplate, type Translations } from '../localization';
import type { CommandError, DnsProvider, DnsProviderCatalog } from '../types/dns';
import {
  hasIdenticalDnsPair,
  normalizeDnsDigits,
  validateIpv4InputKey,
} from '../utils/validation';

interface CustomDnsPageProps {
  t: Translations;
  catalog: DnsProviderCatalog;
  onAdd: (
    name: string,
    primaryDns: string,
    secondaryDns: string | null,
    description: string | null,
  ) => Promise<DnsProvider>;
  onUpdate: (
    id: string,
    name: string,
    primaryDns: string,
    secondaryDns: string | null,
    description: string | null,
  ) => Promise<DnsProvider>;
  onDelete: (id: string) => Promise<void>;
  onNotifySuccess: (title: string, message: string) => void;
  onNotifyError: (title: string, message: string) => void;
}

export function CustomDnsPage({
  t,
  catalog,
  onAdd,
  onUpdate,
  onDelete,
  onNotifySuccess,
  onNotifyError,
}: CustomDnsPageProps) {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [primaryDns, setPrimaryDns] = useState('');
  const [secondaryDns, setSecondaryDns] = useState('');
  const [description, setDescription] = useState('');
  const [formError, setFormError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [pendingDelete, setPendingDelete] = useState<DnsProvider | null>(null);

  const resetForm = () => {
    setEditingId(null);
    setName('');
    setPrimaryDns('');
    setSecondaryDns('');
    setDescription('');
    setFormError(null);
  };

  const startEdit = (provider: DnsProvider) => {
    if (provider.isBuiltIn) return;
    setEditingId(provider.id);
    setName(provider.name);
    setPrimaryDns(provider.primaryDns);
    setSecondaryDns(provider.secondaryDns ?? '');
    setDescription(provider.description ?? '');
    setFormError(null);
  };

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    const trimmedName = name.trim();
    if (!trimmedName) {
      setFormError(t.customDns.validation.nameRequired);
      return;
    }

    const normalized = trimmedName.toLowerCase();
    const duplicateBuiltIn = catalog.builtIn.some(
      (p) => p.name.toLowerCase() === normalized,
    );
    const duplicateCustom = catalog.custom.some(
      (p) => p.id !== editingId && p.name.trim().toLowerCase() === normalized,
    );
    if (duplicateBuiltIn || duplicateCustom) {
      setFormError(t.customDns.validation.duplicateName);
      return;
    }

    const pKey = validateIpv4InputKey(primaryDns);
    if (pKey) {
      setFormError(
        `${t.customDns.validation.primaryInvalid}${t.customDns.validation.ipv4Errors[pKey]}`,
      );
      return;
    }

    const hasSecondary = secondaryDns.trim().length > 0;
    if (hasSecondary) {
      const sKey = validateIpv4InputKey(secondaryDns);
      if (sKey) {
        setFormError(
          `${t.customDns.validation.secondaryInvalid}${t.customDns.validation.ipv4Errors[sKey]}`,
        );
        return;
      }
      if (hasIdenticalDnsPair(primaryDns, secondaryDns)) {
        setFormError(t.customDns.validation.secondarySameAsPrimary);
        return;
      }
    }

    setFormError(null);
    setIsSubmitting(true);

    try {
      const cleanDesc = description.trim() || null;
      const cleanPrimary = normalizeDnsDigits(primaryDns.trim());
      const cleanSecondary = hasSecondary
        ? normalizeDnsDigits(secondaryDns.trim())
        : null;
      if (editingId) {
        const updated = await onUpdate(
          editingId,
          trimmedName,
          cleanPrimary,
          cleanSecondary,
          cleanDesc,
        );
        onNotifySuccess(
          t.notifications.customUpdatedTitle,
          formatTemplate(t.notifications.customUpdatedBody, {
            name: updated.name,
          }),
        );
      } else {
        const created = await onAdd(
          trimmedName,
          cleanPrimary,
          cleanSecondary,
          cleanDesc,
        );
        onNotifySuccess(
          t.notifications.customSavedTitle,
          formatTemplate(t.notifications.customSavedBody, {
            name: created.name,
          }),
        );
      }
      resetForm();
    } catch (err) {
      const ce = err as CommandError;
      const msg =
        ce?.code === 'DUPLICATE_PROVIDER_NAME' ||
        ce?.message?.includes('DUPLICATE_PROVIDER_NAME')
          ? t.customDns.validation.duplicateName
          : t.errors[ce.code] ?? ce.message ?? t.errors.STORAGE_ERROR;
      setFormError(msg);
      onNotifyError(t.notifications.operationErrorTitle, msg);
    } finally {
      setIsSubmitting(false);
    }
  };

  const confirmDelete = async () => {
    if (!pendingDelete) return;
    const target = pendingDelete;
    setPendingDelete(null);
    try {
      await onDelete(target.id);
      if (editingId === target.id) {
        resetForm();
      }
      onNotifySuccess(
        t.notifications.customDeletedTitle,
        t.notifications.customDeletedBody,
      );
    } catch (err) {
      const ce = err as CommandError;
      onNotifyError(
        t.notifications.operationErrorTitle,
        t.errors[ce.code] ?? ce.message ?? t.errors.STORAGE_ERROR,
      );
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold text-slate-900 dark:text-slate-100">
          {t.customDns.title}
        </h2>
        <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
          {t.customDns.subtitle}
        </p>
      </div>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-5">
        <section className="rounded-xl border border-slate-200 bg-white p-6 lg:col-span-2 dark:border-slate-800 dark:bg-slate-900">
          <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100">
            {editingId ? t.customDns.formEditTitle : t.customDns.formAddTitle}
          </h3>

          <form onSubmit={handleSubmit} className="mt-4 space-y-4">
            <div>
              <label className="mb-1.5 block text-xs font-medium text-slate-700 dark:text-slate-300">
                {t.customDns.nameLabel}
              </label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder={t.customDns.namePlaceholder}
                className="w-full rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm text-slate-900 placeholder:text-slate-400 focus:border-sky-500 focus:outline-none dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
              />
            </div>

            <div>
              <label className="mb-1.5 block text-xs font-medium text-slate-700 dark:text-slate-300">
                {t.customDns.primaryLabel}
              </label>
              <input
                type="text"
                dir="ltr"
                value={primaryDns}
                onChange={(e) => setPrimaryDns(e.target.value)}
                placeholder={t.customDns.primaryPlaceholder}
                className="font-mono-tech w-full rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm text-slate-900 placeholder:text-slate-400 focus:border-sky-500 focus:outline-none dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
              />
            </div>

            <div>
              <label className="mb-1.5 block text-xs font-medium text-slate-700 dark:text-slate-300">
                {t.customDns.secondaryLabel}
              </label>
              <input
                type="text"
                dir="ltr"
                value={secondaryDns}
                onChange={(e) => setSecondaryDns(e.target.value)}
                placeholder={t.customDns.secondaryPlaceholder}
                className="font-mono-tech w-full rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm text-slate-900 placeholder:text-slate-400 focus:border-sky-500 focus:outline-none dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
              />
            </div>

            <div>
              <label className="mb-1.5 block text-xs font-medium text-slate-700 dark:text-slate-300">
                {t.customDns.descriptionLabel}
              </label>
              <textarea
                rows={2}
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder={t.customDns.descriptionPlaceholder}
                className="w-full rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm text-slate-900 placeholder:text-slate-400 focus:border-sky-500 focus:outline-none dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
              />
            </div>

            {formError && (
              <div className="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-xs text-rose-700 dark:border-rose-500/30 dark:bg-rose-500/10 dark:text-rose-300">
                {formError}
              </div>
            )}

            <div className="flex flex-wrap items-center gap-2 pt-1">
              <button
                type="submit"
                disabled={isSubmitting}
                className="inline-flex items-center gap-1.5 rounded-lg bg-sky-600 px-4 py-2 text-xs font-semibold text-white transition-colors hover:bg-sky-500 disabled:opacity-50 whitespace-nowrap"
              >
                {editingId ? (
                  <Save className="h-3.5 w-3.5" />
                ) : (
                  <Plus className="h-3.5 w-3.5" />
                )}
                <span>
                  {editingId
                    ? t.customDns.saveChanges
                    : t.customDns.createProvider}
                </span>
              </button>

              {editingId && (
                <button
                  type="button"
                  onClick={resetForm}
                  className="inline-flex items-center gap-1.5 rounded-lg border border-slate-300 bg-white px-3.5 py-2 text-xs font-medium text-slate-700 hover:bg-slate-50 whitespace-nowrap dark:border-slate-700 dark:bg-slate-800 dark:text-slate-200"
                >
                  <X className="h-3.5 w-3.5" />
                  <span>{t.customDns.cancelEdit}</span>
                </button>
              )}
            </div>
          </form>
        </section>

        <section className="rounded-xl border border-slate-200 bg-white p-6 lg:col-span-3 dark:border-slate-800 dark:bg-slate-900">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100">
              {t.customDns.listTitle} ({catalog.custom.length})
            </h3>
            <span className="text-xs text-slate-500 dark:text-slate-400">
              {t.customDns.builtInProtectedNote}
            </span>
          </div>

          {catalog.custom.length === 0 ? (
            <div className="mt-4 rounded-lg border border-dashed border-slate-300 p-8 text-center text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
              {t.customDns.emptyList}
            </div>
          ) : (
            <div className="mt-4 divide-y divide-slate-100 dark:divide-slate-800">
              {catalog.custom.map((provider) => (
                <div
                  key={provider.id}
                  className="flex flex-wrap items-center justify-between gap-4 py-3.5"
                >
                  <div className="min-w-0 flex-1">
                    <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                      {provider.name}
                    </div>
                    <div className="mt-0.5 font-mono-tech text-xs text-sky-700 dark:text-sky-400">
                      {provider.primaryDns}
                      {provider.secondaryDns
                        ? ` / ${provider.secondaryDns}`
                        : ''}
                    </div>
                    {provider.description && (
                      <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                        {provider.description}
                      </p>
                    )}
                  </div>

                  <div className="flex items-center gap-2 shrink-0">
                    <button
                      type="button"
                      onClick={() => startEdit(provider)}
                      className="inline-flex items-center gap-1 rounded-lg border border-slate-200 px-3 py-1.5 text-xs font-medium text-slate-700 transition-colors hover:bg-slate-50 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
                    >
                      <Pencil className="h-3.5 w-3.5" />
                      <span>{t.common.edit}</span>
                    </button>
                    <button
                      type="button"
                      onClick={() => setPendingDelete(provider)}
                      className="inline-flex items-center gap-1 rounded-lg border border-rose-200 px-3 py-1.5 text-xs font-medium text-rose-700 transition-colors hover:bg-rose-50 dark:border-rose-500/30 dark:text-rose-300 dark:hover:bg-rose-500/10"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                      <span>{t.common.delete}</span>
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </section>
      </div>

      <ConfirmDialog
        open={pendingDelete !== null}
        title={t.customDns.deleteConfirmTitle}
        message={
          pendingDelete
            ? formatTemplate(t.customDns.deleteConfirmMessage, {
                name: pendingDelete.name,
              })
            : ''
        }
        confirmLabel={t.common.delete}
        danger
        t={t}
        onConfirm={() => void confirmDelete()}
        onCancel={() => setPendingDelete(null)}
      />
    </div>
  );
}
