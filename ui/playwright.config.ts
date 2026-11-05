import { defineConfig, devices } from '@playwright/test';
import { adminToken } from './tests/admin-token';

/**
 * Par défaut, tests contre une instance réelle (`tests/serve-e2e.sh` :
 * simulateur + `buntu serve`). `BUNTU_UI_URL` permet de viser une instance déjà lancée.
 */
const baseURL = process.env.BUNTU_UI_URL ?? 'http://127.0.0.1:19000';

export default defineConfig({
  testDir: 'tests',
  timeout: 60_000,
  retries: 0,
  reporter: [['list']],
  use: {
    baseURL,
    extraHTTPHeaders: { Authorization: `Bearer ${adminToken}` },
    trace: 'retain-on-failure'
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: process.env.BUNTU_UI_URL
    ? undefined
    : {
        command: 'bash tests/serve-e2e.sh',
        url: 'http://127.0.0.1:19000/health',
        reuseExistingServer: false,
        env: { BUNTU_ADMIN_TOKEN: adminToken },
        timeout: 180_000
      }
});
