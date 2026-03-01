import en from './en.json';
import es from './es.json';

const translations = { en, es } as const;

export type Locale = keyof typeof translations;

export function t(locale: Locale) {
  return translations[locale];
}

export function getLocaleFromUrl(url: URL): Locale {
  const [, segment] = url.pathname.split('/');
  if (segment === 'es') return 'es';
  return 'en';
}

export function getLocalizedPath(path: string, locale: Locale): string {
  if (locale === 'en') return path;
  return `/${locale}${path}`;
}
