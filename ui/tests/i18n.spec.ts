import { expect, test } from '@playwright/test';
import { get } from 'svelte/store';
import { browserLocale, dictionary, locale, t } from '../src/lib/i18n';

/** Revue 2026-11-05 [5] : parité FR/EN et langue du navigateur. */

test('French and English dictionaries have the same keys, none empty', () => {
  const fr = Object.keys(dictionary.fr).sort();
  const en = Object.keys(dictionary.en).sort();
  expect(fr.filter((key) => !en.includes(key)), 'missing in en').toEqual([]);
  expect(en.filter((key) => !fr.includes(key)), 'missing in fr').toEqual([]);
  for (const locale of ['fr', 'en'] as const) {
    const empty = Object.entries(dictionary[locale]).filter(([, value]) => !value.trim());
    expect(empty, `empty values in ${locale}`).toEqual([]);
  }
});

test('browser language picks the console language', () => {
  expect(browserLocale(['fr-FR', 'en-US'])).toBe('fr');
  expect(browserLocale(['en-GB', 'fr'])).toBe('en');
  expect(browserLocale(['de-DE', 'fr-CH'])).toBe('fr');
  expect(browserLocale(['de-DE'])).toBe('en');
  expect(browserLocale([])).toBe('en');
});

test('plural forms follow Intl.PluralRules', () => {
  locale.set('fr');
  expect(get(t)('overview.window', { n: 1 })).toBe('sur la dernière heure');
  expect(get(t)('overview.window', { n: 24 })).toBe('sur les 24 dernières heures');
  locale.set('en');
  expect(get(t)('overview.window', { n: 1 })).toBe('over the last hour');
  expect(get(t)('overview.window', { n: 24 })).toBe('over the last 24 hours');
});

test.describe('English browser', () => {
  test.use({ locale: 'en-US' });

  test('every screen opens in English without a raw key', async ({ page }) => {
    const keys = Object.keys(dictionary.en).filter((key) => key.includes('.'));
    for (const path of ['/', '/providers', '/routes', '/logs']) {
      await page.goto(path);
      await expect(page.locator('html')).toHaveAttribute('lang', 'en');
      await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
      const text = await page.locator('body').innerText();
      expect(keys.filter((key) => text.includes(key)), `${path}: raw keys`).toEqual([]);
    }
    await expect(page.getByRole('link', { name: 'Providers' })).toBeVisible();
  });
});
