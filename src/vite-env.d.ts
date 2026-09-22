/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Set at build time from TAURI_ENV_PLATFORM (see vite.config.ts). */
  readonly VITE_PLATFORM: 'android' | 'desktop'
  /** DEV-only Phase 1 flag: `teacher` opens the app on Teacher Home. */
  readonly VITE_DEV_START?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
