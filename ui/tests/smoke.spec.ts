import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';
import { adminToken } from './admin-token';

test('home page has no serious accessibility violation', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  const results = await new AxeBuilder({ page }).analyze();
  const serious = results.violations.filter((violation) =>
    ['serious', 'critical'].includes(violation.impact ?? '')
  );
  expect(serious, JSON.stringify(serious, null, 2)).toEqual([]);
});

test('theme toggle and language switch work', async ({ page }) => {
  await page.goto('/');
  const html = page.locator('html');
  await expect(html).toHaveAttribute('data-theme', 'light');
  await page.getByRole('button', { name: /thème sombre|dark theme/i }).click();
  await expect(html).toHaveAttribute('data-theme', 'dark');
  await page.locator('#langue').selectOption('en');
  // Le lien de navigation est ciblé pour éviter la carte de la page d'accueil.
  const navigation = page.getByRole('navigation', { name: /Navigation principale|Main navigation/i });
  await expect(navigation.getByRole('link', { name: 'Providers' })).toBeVisible();
});

test.describe('admin token prompt', () => {
  // Navigateur sans en-tête d'authentification : la console doit demander le jeton.
  test.use({ extraHTTPHeaders: {} });

  test('a 401 asks for the token, which then unlocks the screen', async ({ page }) => {
    await page.goto('/providers');
    const prompt = page.getByRole('form', { name: 'Jeton d’administration' });
    await expect(prompt).toBeVisible();
    const results = await new AxeBuilder({ page }).analyze();
    const serious = results.violations.filter((violation) =>
      ['serious', 'critical'].includes(violation.impact ?? '')
    );
    expect(serious, JSON.stringify(serious, null, 2)).toEqual([]);
    await prompt.getByLabel('Jeton').fill(adminToken);
    await prompt.getByRole('button', { name: 'Valider' }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Fournisseurs' })).toBeVisible();
    await expect(prompt).toHaveCount(0);
  });
});
