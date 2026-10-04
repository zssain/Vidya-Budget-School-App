// Android Google Drive sign-in wiring (Phase 20). Desktop uses the loopback flow
// (drive_connect); the PWA uses GIS. Android uses Google's installed-app custom-scheme
// redirect: open the consent page in a Custom Tab, then the redirect returns to the app
// via tauri-plugin-deep-link and drive_exchange finishes it. Device-verified (the
// deep-link routing only runs on a real Android build).

import * as api from './api'
import { isWeb } from './platform'

// Google's reversed-client-id redirect scheme (must match the AndroidManifest + the
// Android OAuth client). Any URL on this scheme is an OAuth callback.
const OAUTH_SCHEME = 'com.googleusercontent.apps.'

/** Start the Android Drive sign-in: fetch the auth URL and open it in a Custom Tab. */
export async function connectDriveMobile(): Promise<void> {
  const url = await api.drive_connect_start()
  const { openUrl } = await import('@tauri-apps/plugin-opener')
  await openUrl(url)
}

let registered = false
/** Register the OAuth deep-link callback once (Tauri app only — never the PWA). When
 *  Google redirects back to the app's custom scheme, hand the URL to drive_exchange. */
export async function initDriveDeepLink(): Promise<void> {
  if (registered || isWeb) return
  registered = true
  try {
    const { onOpenUrl } = await import('@tauri-apps/plugin-deep-link')
    await onOpenUrl((urls) => {
      for (const u of urls) {
        if (u.startsWith(OAUTH_SCHEME)) void api.drive_exchange(u).catch(() => undefined)
      }
    })
  } catch {
    /* deep-link plugin unavailable (non-Tauri context) — ignore */
  }
}
