// Platform is decided at BUILD time: vite.config.ts sets VITE_PLATFORM from
// TAURI_ENV_PLATFORM. In a plain browser (`npm run dev`) a DEV-only
// `?platform=android` query overrides it for previewing the phone layout.

export type Platform = 'android' | 'desktop'

function detect(): Platform {
  if (import.meta.env.DEV && typeof location !== 'undefined') {
    const q = new URLSearchParams(location.search).get('platform')
    if (q === 'android' || q === 'desktop') return q
  }
  return import.meta.env.VITE_PLATFORM === 'android' ? 'android' : 'desktop'
}

export const platform: Platform = detect()
export const isPhone: boolean = platform === 'android'
