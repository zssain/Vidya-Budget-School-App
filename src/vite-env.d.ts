/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Set at build time: `android`/`desktop` for the Tauri app (from
   *  TAURI_ENV_PLATFORM, see vite.config.ts), or `web` for the iPhone PWA
   *  (web-pwa/vite.config.ts, Phase 19). */
  readonly VITE_PLATFORM: 'android' | 'desktop' | 'web'
  /** DEV-only Phase 1 flag: `teacher` opens the app on Teacher Home. */
  readonly VITE_DEV_START?: string
  /** The PWA's Google "Web application" OAuth client id (Phase 19; owner sets it via
   *  a .env / CI Variable GOOGLE_CLIENT_ID_WEB). Empty until configured. */
  readonly VITE_GOOGLE_CLIENT_ID_WEB?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
