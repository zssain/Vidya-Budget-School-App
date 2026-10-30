// The command dispatch every `api.ts` wrapper goes through (P19 Step 2). On the
// Tauri app (desktop/android) it calls the Tauri backend; in the iPhone PWA (`web`)
// it routes to the Web backend (IndexedDB + WASM + Drive). Screens import `api.ts`
// only, so they never know which backend served a command — the phone screens run
// unchanged in the PWA. This is the single choke-point the Backend interface needs.

import { invoke as tauriInvoke } from '@tauri-apps/api/core'
import { isWeb } from './platform'
import { webInvoke } from './web/backend'

/** Invoke a Vidya command. Drop-in for `@tauri-apps/api/core`'s `invoke`. */
export function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isWeb) return webInvoke<T>(cmd, args).catch((err: unknown) => {
    notifyIfLocked(err)
    throw err
  })
  return tauriInvoke<T>(cmd, args).catch((err: unknown) => {
    // Log only the command + stable error code (never its vars/detail, which can carry
    // addresses or other context). Full diagnostics are in the app's log file.
    const code = typeof err === 'object' && err !== null ? (err as { code?: string }).code : undefined
    console.warn(`[vidya] ${cmd} failed: ${code ?? 'error'}`)
    notifyIfLocked(err)
    throw err
  })
}

/** Fired when any command reports the session is locked (idle timeout or lock), so
 *  the app re-routes to the PIN screen at once instead of showing stale data. */
export const LOCKED_EVENT = 'vidya:locked'

function notifyIfLocked(err: unknown): void {
  if (typeof window === 'undefined') return
  if ((err as { code?: string } | null)?.code === 'LOCKED') window.dispatchEvent(new Event(LOCKED_EVENT))
}
