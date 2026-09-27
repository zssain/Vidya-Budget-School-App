import { defineConfig, devices } from '@playwright/test'

// iPhone-PWA tests (Phase 19, Step 9) — run in **WebKit**, the same engine as iPhone
// Safari, against the PRODUCTION build (web-pwa/dist) served statically with its real
// Content-Security-Policy + service worker. Separate from playwright.config.ts (the
// Chrome fidelity suite against the Tauri dev server) so the two don't share a server
// or a browser. Build first: `npm run build:web` (the test:e2e:pwa script does this).
const PORT = 5280

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
  projects: [{ name: 'webkit', use: { ...devices['Desktop Safari'] } }],
  webServer: {
    command: `node scripts/serve-static.mjs --port ${PORT}`,
    url: `http://localhost:${PORT}/`,
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
})
