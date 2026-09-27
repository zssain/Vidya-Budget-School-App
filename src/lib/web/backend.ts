// The Web backend for the iPhone PWA (Phase 19, Step 2). It implements EXACTLY the
// phone commands the staff client needs (registered in Step 3+ as the IndexedDB
// store, WASM and Drive sync land). Every other command — the school server, setup,
// restore, backups, licences — returns `NOT_AVAILABLE_ON_WEB` so the UI can hide
// what the PWA never provides (00-context §18).
//
// Import-safe: no browser API (IndexedDB, WebCrypto, fetch) runs at module load, so
// this file imports cleanly under vitest/Node and in the Tauri bundle (where it is
// dead code — `isWeb` is false there).

import type { CmdError } from '../api'
import { createPin, lock } from './lock'

/** A Web command handler: `(args) => result`. */
export type WebHandler = (args?: Record<string, unknown>) => Promise<unknown>

/**
 * The commands the Web backend serves. Step 3 wires the local-store foundation
 * (PIN create/lock via WASM Argon2id + the encrypted IndexedDB store); the rest of
 * the phone-screen commands (unlock→app_state, attendance, notes, staff check-in,
 * leave, requests, inbox, the accountant collect-fee, sync status…) are registered
 * as the join flow (Step 5) and each screen are ported. A command absent from this
 * map is intentionally unavailable in the PWA and returns NOT_AVAILABLE_ON_WEB.
 */
export const WEB_COMMANDS: Record<string, WebHandler> = {
  create_pin: async (args) => {
    await createPin(String(args?.pin ?? ''))
  },
  lock: async () => {
    lock()
  },
}

/** True if the PWA implements `cmd`. The UI hides any feature whose command isn't. */
export function isAvailableOnWeb(cmd: string): boolean {
  return Object.prototype.hasOwnProperty.call(WEB_COMMANDS, cmd)
}

/** Dispatch a command to its Web handler, or reject with NOT_AVAILABLE_ON_WEB. */
export async function webInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const handler = WEB_COMMANDS[cmd]
  if (!handler) {
    const err: CmdError = {
      code: 'NOT_AVAILABLE_ON_WEB',
      message_key: 'error.NOT_AVAILABLE_ON_WEB',
      vars: { cmd: cmd ?? '' },
    }
    // api.ts wrappers reject with the CmdError shape (as Tauri's invoke does on Err).
    return Promise.reject(err)
  }
  return (await handler(args)) as T
}
