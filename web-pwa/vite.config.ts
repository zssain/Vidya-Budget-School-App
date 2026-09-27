import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

// The iPhone PWA build (Phase 19). It compiles the SAME React app and design tokens
// as the Tauri build (via the `@` → ../src alias — no forked styles, §18), but with
// platform = 'web', so the app uses the Web backend (IndexedDB + WASM + Drive) and
// the phone layout. Output is static files only, hosted on GitHub/Cloudflare Pages;
// nothing here needs a company server (§13). The service worker + manifest are
// added in Step 6; the WASM module is built separately (`npm run build:wasm`).

const appVersion: string = JSON.parse(
  readFileSync(fileURLToPath(new URL('../package.json', import.meta.url)), 'utf8'),
).version

export default defineConfig({
  root: fileURLToPath(new URL('.', import.meta.url)),
  plugins: [react(), tailwindcss()],
  resolve: {
    // Same alias as the app so the shared src/ components resolve identically.
    alias: { '@': fileURLToPath(new URL('../src', import.meta.url)) },
  },
  define: {
    'import.meta.env.VITE_PLATFORM': JSON.stringify('web'),
    __APP_VERSION__: JSON.stringify(appVersion),
  },
  build: {
    outDir: fileURLToPath(new URL('./dist', import.meta.url)),
    emptyOutDir: true,
    target: 'es2022', // top-level await + modern WASM loading (iOS 16.4+ Safari)
  },
  clearScreen: false,
  server: { port: 5273, strictPort: true },
})
