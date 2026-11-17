import { useEffect, useMemo } from 'react';
import { getTranslations } from '../localization';
import type { AppLanguage } from '../types/dns';

export function useLocalization(language: AppLanguage) {
  const dir = language === 'fa' ? 'rtl' : 'ltr';
  const t = useMemo(() => getTranslations(language), [language]);

  useEffect(() => {
    const root = document.documentElement;
    root.lang = language;
    root.dir = dir;
  }, [language, dir]);

  return {
    language,
    dir,
    isRtl: dir === 'rtl',
    t,
  };
}
