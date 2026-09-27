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
  return isWeb ? webInvoke<T>(cmd, args) : tauriInvoke<T>(cmd, args)
}
