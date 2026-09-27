import { defineConfig, type Plugin } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { readFileSync, writeFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { fileURLToPath } from 'node:url'

// The iPhone PWA build (Phase 19). It compiles the SAME React app and design tokens
// as the Tauri build (via the `@` → ../src alias — no forked styles, §18), but with
// platform = 'web', so the app uses the Web backend (IndexedDB + WASM + Drive) and
// the phone layout. Output is static files only, hosted on GitHub/Cloudflare Pages;
// nothing here needs a company server (§13). The WASM module is built separately
// (`npm run build:wasm`).

const appVersion: string = JSON.parse(
  readFileSync(fileURLToPath(new URL('../package.json', import.meta.url)), 'utf8'),
).version

// The browser OAuth client ID is a build-time, NON-secret value (like the desktop
// and Android client IDs in build-config/release.json). CI passes it as the repo
// Variable GOOGLE_CLIENT_ID_WEB; a local shell may use the VITE_-prefixed name.
// When neither is set we leave it to Vite's normal .env loading — an empty value
// makes the app show a "sign-in not set up yet" state (no crash). See DEPLOY-PWA.md.
const webClientId = process.env.VITE_GOOGLE_CLIENT_ID_WEB ?? process.env.GOOGLE_CLIENT_ID_WEB

// Step 6: generate dist/sw.js from sw-template.js, injecting the app shell + every
// emitted asset (hashed JS/CSS/WASM/fonts) as the precache list and a content-hash
// version. Hand-written service worker; no Workbox.
function serviceWorker(): Plugin {
  return {
    name: 'vidya-sw',
    apply: 'build',
    writeBundle(_options, bundle) {
      const assets = Object.keys(bundle).map((f) => '/' + f)
      const precache = [...new Set(['/', '/index.html', '/manifest.webmanifest', ...assets])]
      const version = createHash('sha256').update(precache.join('|')).digest('hex').slice(0, 12)
      const template = readFileSync(fileURLToPath(new URL('./sw-template.js', import.meta.url)), 'utf8')
      const sw = template.replace('__VERSION__', version).replace('__PRECACHE__', JSON.stringify(precache))
      writeFileSync(fileURLToPath(new URL('./dist/sw.js', import.meta.url)), sw)
    },
  }
}

// SPA fallback for static hosts. The router is hash-based, but the invitation
// deep-link is a real path — `/join#d=…` — so on its FIRST load (in Safari, before
// the service worker exists) the host must return index.html for that path. GitHub
// Pages serves 404.html for any unknown path, so we emit a copy of index.html as
// 404.html. Cloudflare Pages uses a `_redirects` rule instead (see DEPLOY-PWA.md).
function spaFallback(): Plugin {
  return {
    name: 'vidya-spa-fallback',
    apply: 'build',
    writeBundle() {
      const dist = (p: string) => fileURLToPath(new URL('./dist/' + p, import.meta.url))
      writeFileSync(dist('404.html'), readFileSync(dist('index.html'), 'utf8'))
    },
  }
}

export default defineConfig({
  root: fileURLToPath(new URL('.', import.meta.url)),
  plugins: [react(), tailwindcss(), serviceWorker(), spaFallback()],
  resolve: {
    alias: {
      // Same alias as the app so the shared src/ components resolve identically.
      '@': fileURLToPath(new URL('../src', import.meta.url)),
      // The generated vidya-wasm ES module (built by `npm run build:wasm`).
      '@wasm': fileURLToPath(new URL('./wasm/vidya_wasm.js', import.meta.url)),
    },
  },
  define: {
    'import.meta.env.VITE_PLATFORM': JSON.stringify('web'),
    __APP_VERSION__: JSON.stringify(appVersion),
    ...(webClientId
      ? { 'import.meta.env.VITE_GOOGLE_CLIENT_ID_WEB': JSON.stringify(webClientId) }
      : {}),
  },
  build: {
    outDir: fileURLToPath(new URL('./dist', import.meta.url)),
    emptyOutDir: true,
    target: 'es2022', // top-level await + modern WASM loading (iOS 16.4+ Safari)
  },
  clearScreen: false,
  server: { port: 5273, strictPort: true },
})
