/// <reference types="node" />

import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests',
  timeout: 60_000,
  fullyParallel: false,
  workers: process.env.CI ? 2 : 4,
  use: {
    baseURL: 'http://127.0.0.1:5173',
    channel: 'chrome',
    locale: 'zh-CN',
    trace: 'retain-on-failure',
    launchOptions: process.env.CI ? { args: ['--disable-audio-input'] } : undefined,
  },
  webServer: {
    command: 'pnpm dev',
    url: 'http://127.0.0.1:5173/scores',
    reuseExistingServer: !process.env.CI,
  },
  projects: [
    { name: 'browser', outputDir: 'test-results/browser', testMatch: '**/*.browser.spec.ts' },
    {
      name: 'native',
      outputDir: 'test-results/native',
      testMatch: '**/native.spec.ts',
      timeout: 180_000,
    },
  ],
});
