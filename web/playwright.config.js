import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './e2e',
  timeout: 120000,
  use: { baseURL: 'http://127.0.0.1:5173', headless: true, actionTimeout: 15000, expectTimeout: 15000, navigationTimeout: 15000 },
  webServer: {
    command: 'npm run dev -- --host 127.0.0.1 --port 5173',
    url: 'http://127.0.0.1:5173',
    reuseExistingServer: !process.env.CI,
    timeout: 60000,
  },
});
