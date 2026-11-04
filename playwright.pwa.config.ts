import { defineConfig, devices } from '@playwright/test'

// iPhone-PWA tests (Phase 19, Step 9) — run in **WebKit**, the same engine as iPhone
// Safari, against the PRODUCTION build (web-pwa/dist) served statically with its real
// Content-Security-Policy + service worker. Separate from playwright.config.ts (the
// Chrome fidelity suite against the Tauri dev server) so the two don't share a server
// or a browser. Build first: `npm run build:web` (the test:e2e:pwa script does this).
//
// A second project, `webkit-seeded`, drives a JOINED app: it runs against the
// test-only `--mode e2e` build (web-pwa/dist-e2e, `npm run build:web:e2e`), which is
// the same app plus the seeding harness web-pwa/e2e-seed.ts. That harness is never
// in web-pwa/dist (pwa.spec.ts asserts it). Google is fully stubbed in those tests.
const PORT = 5280
const SEEDED_PORT = 5281

export default defineConfig({
  testDir: './tests/e2e/pwa',
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: process.env.CI ? 'line' : 'list',
  timeout: 60_000,
  use: {
    baseURL: `http://localhost:${PORT}`,
    ...devices['Desktop Safari'], // WebKit
  },
  projects: [
    { name: 'webkit', testMatch: 'pwa.spec.ts', use: { ...devices['Desktop Safari'] } },
    {
      name: 'webkit-seeded',
      testMatch: 'sync-problems.spec.ts',
      use: { ...devices['Desktop Safari'], baseURL: `http://localhost:${SEEDED_PORT}` },
    },
  ],
  webServer: [
    {
      command: `node scripts/serve-static.mjs --port ${PORT}`,
      url: `http://localhost:${PORT}/`,
      reuseExistingServer: !process.env.CI,
      timeout: 60_000,
    },
    {
      command: `node scripts/serve-static.mjs --port ${SEEDED_PORT} --dist dist-e2e`,
      url: `http://localhost:${SEEDED_PORT}/`,
      reuseExistingServer: !process.env.CI,
      timeout: 60_000,
    },
  ],
})
