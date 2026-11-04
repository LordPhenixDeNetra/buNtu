import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

/**
 * Tests E2E contre une instance réelle : `tests/serve-e2e.sh` démarre le simulateur
 * (port 19001) et `buntu serve` (port 19000, base SQLite temporaire). La configuration de
 * départ contient déjà le fournisseur `e2e` et la route `chat-e2e` ; les tests créent donc
 * `e2e-ui` et `chat-ui` pour éviter toute ambiguïté.
 */
test.describe.configure({ mode: 'serial' });

test('create a provider, a route, then find the request in the journal', async ({ page, request }) => {
  // T1.23 — fournisseur pointant vers le simulateur.
  await page.goto('/providers');
  const createProvider = page.getByRole('group', { name: 'Ajouter un fournisseur' });
  await createProvider.getByLabel('Identifiant').fill('e2e-ui');
  await createProvider.getByLabel('Type').selectOption('mock');
  await createProvider.getByLabel('URL de base').fill('http://127.0.0.1:19001');
  await createProvider.getByLabel('Clé API (saisie unique)').fill('sk-e2e-secret-0001');
  await createProvider.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(page.getByRole('heading', { name: 'e2e-ui', exact: true })).toBeVisible();
  // La clé n'est jamais réaffichée en clair ; seul l'aperçu est visible.
  await expect(page.getByText('sk-e2e-secret-0001')).toHaveCount(0);
  await expect(page.getByText('sk-…0001').first()).toBeVisible();

  // T1.23 — route avec chaîne de repli et réordonnancement.
  await page.goto('/routes');
  const createRoute = page.getByRole('group', { name: 'Créer une route' });
  await createRoute.getByLabel('Alias').fill('chat-ui');
  await createRoute
    .getByLabel(/Chaîne de repli/)
    .fill('e2e/modele-a\ne2e/modele-b');
  await createRoute.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(page.getByRole('heading', { name: 'chat-ui', exact: true })).toBeVisible();

  // T1.24 — une requête réelle passe par la passerelle puis apparaît dans le journal.
  const completion = await request.post('/v1/chat/completions', {
    data: { model: 'chat-ui', messages: [{ role: 'user', content: 'bonjour' }] }
  });
  expect(completion.status()).toBe(200);

  await page.goto('/logs?provider=e2e');
  await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  // Le journal est écrit par lots (≤ 1 s) : on recharge jusqu'à voir la requête.
  let found = false;
  for (let attempt = 0; attempt < 20 && !found; attempt += 1) {
    await page.goto('/logs?provider=e2e');
    found = (await page.getByRole('cell', { name: 'e2e', exact: true }).count()) > 0;
    if (!found) {
      await page.waitForTimeout(250);
    }
  }
  expect(found, 'la requête doit apparaître dans le journal').toBe(true);
  await page.getByRole('button', { name: 'Détail' }).first().click();
  const detailPanel = page.getByRole('region', { name: 'Détail' });
  await expect(detailPanel).toBeVisible();
  await expect(detailPanel).toContainText('Tentatives');
  await expect(detailPanel).toContainText('Latence');

  // Accessibilité : aucune violation grave sur les écrans clés.
  for (const path of ['/providers', '/routes', '/logs?provider=e2e']) {
    await page.goto(path);
    const results = await new AxeBuilder({ page }).analyze();
    const serious = results.violations.filter((violation) =>
      ['serious', 'critical'].includes(violation.impact ?? '')
    );
    expect(serious, `${path} : ${JSON.stringify(serious, null, 2)}`).toEqual([]);
  }
});
