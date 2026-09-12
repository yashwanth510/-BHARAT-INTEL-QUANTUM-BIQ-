import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests',
  timeout: 90000,
  expect: { timeout: 20000 },
  workers: 1,
  use: {
    viewport: {width:1280, height:720},
    baseURL: process.env.E2E_BASE_URL || 'http://127.0.0.1:3000',
    channel: process.env.E2E_BROWSER_CHANNEL || 'chrome',
    headless: true,
    launchOptions: { args: ['--no-sandbox', '--enable-unsafe-swiftshader', '--ignore-certificate-errors'] },
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
});
