import { expect, test } from '@playwright/test';

/** Revue 2026-11-05 [7.3] : la console reste utilisable sur un téléphone (375 × 812). */
test.use({ viewport: { width: 375, height: 812 } });

test('every screen fits a phone width without horizontal scroll', async ({ page }) => {
  for (const path of ['/', '/providers', '/routes', '/logs']) {
    await page.goto(path);
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth
    );
    expect(overflow, `${path}: horizontal overflow in px`).toBeLessThanOrEqual(0);
    await expect(page.getByRole('navigation').first()).toBeVisible();
  }
});
