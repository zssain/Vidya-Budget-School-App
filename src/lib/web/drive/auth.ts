// Google sign-in for the PWA (Phase 19, Step 4; see docs/phase-notes/phase-19-spike.md).
// The browser-only GIS **token model**: no client secret, no backend. A short-lived
// (1 h) access token for `drive.file` is obtained via `initTokenClient` +
// `requestAccessToken`. Renewal is silent (`prompt: ''`) while the Google session is
// active; when it lapses the app asks the user to re-tap "Reconnect Google Drive"
// (honest status — never a fake "synced"). The client id is the owner's Web OAuth
// client (VITE_GOOGLE_CLIENT_ID_WEB); the GIS script is loaded network-only.

const GIS_SRC = 'https://accounts.google.com/gsi/client'
const DRIVE_SCOPE = 'https://www.googleapis.com/auth/drive.file'

// Minimal GIS surface (avoids an @types dependency, Rule 4).
interface GisTokenResponse {
  access_token?: string
  expires_in?: number
  error?: string
}
interface GisTokenClient {
  requestAccessToken(opts?: { prompt?: '' | 'none' | 'consent' }): void
}
interface GisOauth2 {
  initTokenClient(config: {
    client_id: string
    scope: string
    callback: (r: GisTokenResponse) => void
    error_callback?: (e: { type?: string }) => void
  }): GisTokenClient
}
type GisWindow = Window & { google?: { accounts?: { oauth2?: GisOauth2 } } }

let scriptPromise: Promise<GisOauth2> | null = null

function loadGis(): Promise<GisOauth2> {
  scriptPromise ??= new Promise<GisOauth2>((resolve, reject) => {
    const w = window as GisWindow
    const ready = () => {
      const o = w.google?.accounts?.oauth2
      if (o) resolve(o)
      else reject(new Error('GIS loaded but oauth2 missing'))
    }
    if (w.google?.accounts?.oauth2) return ready()
    const s = document.createElement('script')
    s.src = GIS_SRC
    s.async = true
    s.onload = ready
    s.onerror = () => reject(new Error('failed to load Google Identity Services'))
    document.head.appendChild(s)
  })
  return scriptPromise
}

const clientId = (): string => (import.meta.env.VITE_GOOGLE_CLIENT_ID_WEB ?? '').trim()

interface Token {
  value: string
  expiresAt: number // epoch ms
}
let cached: Token | null = null

/** Request a token. `interactive` allows GIS to show UI (a tap); otherwise silent. */
function requestToken(interactive: boolean): Promise<Token> {
  return new Promise((resolve, reject) => {
    if (!clientId()) {
      reject(new Error('DRIVE_NOT_CONFIGURED'))
      return
    }
    loadGis()
      .then((oauth2) => {
        const client = oauth2.initTokenClient({
          client_id: clientId(),
          scope: DRIVE_SCOPE,
          callback: (r) => {
            if (r.access_token) {
              resolve({ value: r.access_token, expiresAt: Date.now() + (r.expires_in ?? 3600) * 1000 })
            } else {
              reject(new Error(r.error ?? 'DRIVE_SIGNIN_REQUIRED'))
            }
          },
          error_callback: (e) => reject(new Error(e.type ?? 'DRIVE_SIGNIN_REQUIRED')),
        })
        // '' = no UI when a session + prior consent exist; 'consent' forces the tap.
        client.requestAccessToken({ prompt: interactive ? 'consent' : '' })
      })
      .catch(reject)
  })
}

/** A valid access token, renewing silently if possible. Rejects DRIVE_SIGNIN_REQUIRED
 *  when the user must re-tap (the UI shows "Reconnect Google Drive"). */
export async function getAccessToken(): Promise<string> {
  if (cached && Date.now() < cached.expiresAt - 60_000) return cached.value
  cached = await requestToken(false)
  return cached.value
}

/** Explicit user-initiated (re)connect — shows the Google consent/account UI. */
export async function connectDrive(): Promise<void> {
  cached = await requestToken(true)
}

/** Whether a Web OAuth client id is configured for this build. */
export const driveConfigured = (): boolean => clientId().length > 0

/** Forget the cached token (on 401 or sign-out). */
export function clearToken(): void {
  cached = null
}
