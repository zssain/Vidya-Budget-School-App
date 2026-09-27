// Platform is decided at BUILD time. For the Tauri app, vite.config.ts sets
// VITE_PLATFORM from TAURI_ENV_PLATFORM (`android`/`desktop`). For the iPhone PWA
// (Phase 19), web-pwa/vite.config.ts sets VITE_PLATFORM=`web`. In a plain browser
// (`npm run dev`) a DEV-only `?platform=` query overrides it for previewing.

export type Platform = 'android' | 'desktop' | 'web'

function detect(): Platform {
  if (import.meta.env.DEV && typeof location !== 'undefined') {
    const q = new URLSearchParams(location.search).get('platform')
    if (q === 'android' || q === 'desktop' || q === 'web') return q
  }
  const p = import.meta.env.VITE_PLATFORM
  if (p === 'android') return 'android'
  if (p === 'web') return 'web'
  return 'desktop'
}

export const platform: Platform = detect()

/** The PWA (`web`) is a staff **phone** client, so it uses the phone layout too. */
export const isPhone: boolean = platform === 'android' || platform === 'web'

/** True only in the browser PWA build (Drive-only sync, no Tauri, staff client). */
export const isWeb: boolean = platform === 'web'
