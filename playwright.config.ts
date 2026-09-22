import { defineConfig, devices } from '@playwright/test'

// Fidelity + interaction tests (docs/01-MOCK-SPEC.md §9, prompts/P01 Step 12).
// The single dev server serves BOTH the app (hash routes) and the unmodified
// mocks at /design/screens/*.dc.html (via the design-mock middleware in
// vite.config.ts). Screenshots are compared at ≤ 0.1% (maxDiffPixelRatio 0.001).
export default defineConfig({
  testDir: './tests/e2e',
  snapshotDir: './tests/e2e/__screens__',
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: process.env.CI ? 'line' : 'list',
  timeout: 60_000,
  expect: {
    // ≤ 0.1% difference; freeze animations to their settled end-state.
    toHaveScreenshot: { maxDiffPixelRatio: 0.001, animations: 'disabled' },
    toMatchSnapshot: { maxDiffPixelRatio: 0.001 },
  },
  use: {
    baseURL: 'http://localhost:5173',
    ...devices['Desktop Chrome'],
    deviceScaleFactor: 1,
  },
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:5173',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
})
