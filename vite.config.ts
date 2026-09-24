import { defineConfig } from 'vitest/config'
import type { Plugin } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

// Tauri sets these during `tauri dev/build` and `tauri android dev/build`.
const host = process.env.TAURI_DEV_HOST
const platform = process.env.TAURI_ENV_PLATFORM === 'android' ? 'android' : 'desktop'
// App version, single-sourced from package.json (shown in Settings → About).
const appVersion: string = JSON.parse(
  readFileSync(fileURLToPath(new URL('./package.json', import.meta.url)), 'utf8'),
).version

// DEV ONLY: serve the mock renderer at /design/screens/support.js so the
// unmodified design/screens/*.dc.html files render in the browser for the
// fidelity tests. design/ is never part of the production bundle.
function designMock(): Plugin {
  const supportUrl = new URL('./design/runtime/support.js', import.meta.url)
  return {
    name: 'vidya-design-mock',
    apply: 'serve',
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (req.url && req.url.split('?')[0].endsWith('/design/screens/support.js')) {
          try {
            const js = readFileSync(supportUrl, 'utf8')
            res.setHeader('Content-Type', 'text/javascript')
            res.end(js)
            return
          } catch {
            /* not written yet — fall through */
          }
        }
        next()
      })
    },
  }
}

export default defineConfig({
  plugins: [react(), tailwindcss(), designMock()],
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  // Platform is decided at BUILD time (see src/lib/platform.ts).
  define: {
    'import.meta.env.VITE_PLATFORM': JSON.stringify(platform),
    __APP_VERSION__: JSON.stringify(appVersion),
  },
  clearScreen: false,
  // Vitest runs unit tests only; Playwright owns tests/e2e/*.spec.ts.
  test: {
    include: ['src/**/*.test.ts'],
    environment: 'node',
  },
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 5183 } : undefined,
    watch: { ignored: ['**/src-tauri/**'] },
  },
})
