import type { AppLanguage } from '../types/dns';
import { en } from './en';
import { fa, type Translations } from './fa';

export type { Translations };

const DICTIONARIES: Record<AppLanguage, Translations> = {
  fa,
  en,
};

export function getTranslations(language: AppLanguage): Translations {
  return DICTIONARIES[language] ?? fa;
}

export function formatTemplate(
  template: string,
  vars: Record<string, string>,
): string {
  return Object.entries(vars).reduce(
    (acc, [key, value]) => acc.replaceAll(`{${key}}`, value),
    template,
  );
}
