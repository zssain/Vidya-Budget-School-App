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
  if (isWeb) return webInvoke<T>(cmd, args)
  return tauriInvoke<T>(cmd, args).catch((err: unknown) => {
    // Surface the full failure (error code + detail) to the platform console. On
    // Android this is the ONLY way to read a command's failure reason from a real
    // device — `adb logcat` captures console output, but Rust `tracing` does not
    // reach logcat. Harmless elsewhere; the rejection is re-thrown unchanged.
    let printable: string
    try {
      printable = typeof err === 'string' ? err : JSON.stringify(err)
    } catch {
      printable = String(err)
    }
    console.error(`[vidya cmd failed] ${cmd}: ${printable}`)
    throw err
  })
}
