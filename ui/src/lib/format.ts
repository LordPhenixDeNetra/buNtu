import type { Locale } from './i18n';

/** Formats dépendant de la langue : aucune unité ni date écrite à la main. */

export function formatDate(seconds: number, locale: Locale): string {
  return new Intl.DateTimeFormat(locale, { dateStyle: 'short', timeStyle: 'medium' }).format(
    new Date(seconds * 1000)
  );
}

export function formatMs(value: number, locale: Locale): string {
  return new Intl.NumberFormat(locale, {
    style: 'unit',
    unit: 'millisecond',
    unitDisplay: 'short',
    maximumFractionDigits: 0
  }).format(value);
}

export function formatPercent(ratio: number, locale: Locale): string {
  return new Intl.NumberFormat(locale, { style: 'percent', maximumFractionDigits: 1 }).format(
    ratio
  );
}

export function formatCount(value: number, locale: Locale): string {
  return new Intl.NumberFormat(locale).format(value);
}
