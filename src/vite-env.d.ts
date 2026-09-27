/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Set at build time: `android`/`desktop` for the Tauri app (from
   *  TAURI_ENV_PLATFORM, see vite.config.ts), or `web` for the iPhone PWA
   *  (web-pwa/vite.config.ts, Phase 19). */
  readonly VITE_PLATFORM: 'android' | 'desktop' | 'web'
  /** DEV-only Phase 1 flag: `teacher` opens the app on Teacher Home. */
  readonly VITE_DEV_START?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
