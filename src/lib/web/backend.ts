// The Web backend for the iPhone PWA (Phase 19, Step 2). It implements EXACTLY the
// phone commands the staff client needs (registered in Step 3+ as the IndexedDB
// store, WASM and Drive sync land). Every other command — the school server, setup,
// restore, backups, licences — returns `NOT_AVAILABLE_ON_WEB` so the UI can hide
// what the PWA never provides (00-context §18).
//
// Import-safe: no browser API (IndexedDB, WebCrypto, fetch) runs at module load, so
// this file imports cleanly under vitest/Node and in the Tauri bundle (where it is
// dead code — `isWeb` is false there).

import type { AppState, AppStateResponse, CmdError, SessionStaff } from '../api'
import { hasJoined } from './join'
import { createPin, hasPin, isUnlocked, lock } from './lock'
import { kvGet } from './store'

/** A Web command handler: `(args) => result`. */
export type WebHandler = (args?: Record<string, unknown>) => Promise<unknown>

/**
 * Map the PWA's local onboarding/session flags to the shared `AppState` the router
 * (`routeForState`) already understands, so the app boots past the status screen
 * (Phase 19 "what remains" #1). Pure (no I/O) → unit-tested directly.
 *
 * A device that has not finished joining — no identity, or an identity without a PIN
 * (the join flow sets the PIN: `/join` → request → poll → set PIN → home) — routes to
 * `no_school`, where the join/onboarding UI (C2) picks it up; it is never an impassable
 * PIN gate. Once joined with a PIN it is `locked` until the PIN is entered, then
 * `unlocked` for the staff member who joined (teachers get the phone screens).
 */
export function webAppState(f: {
  joined: boolean
  hasPin: boolean
  unlocked: boolean
  staff: SessionStaff | undefined
}): AppState {
  if (!f.joined || !f.hasPin) return { kind: 'no_school' }
  if (f.unlocked && f.staff) return { kind: 'unlocked', staff: f.staff }
  return { kind: 'locked' }
}

/**
 * The commands the Web backend serves. Step 3 wires the local-store foundation
 * (PIN create/lock via WASM Argon2id + the encrypted IndexedDB store); the rest of
 * the phone-screen commands (unlock→app_state, attendance, notes, staff check-in,
 * leave, requests, inbox, the accountant collect-fee, sync status…) are registered
 * as the join flow (Step 5) and each screen are ported. A command absent from this
 * map is intentionally unavailable in the PWA and returns NOT_AVAILABLE_ON_WEB.
 */
export const WEB_COMMANDS: Record<string, WebHandler> = {
  // Boot/route state — the first ported command, so the PWA leaves the status screen
  // and lands on join (not yet onboarded), the PIN gate, or the teacher home.
  app_state: async (): Promise<AppStateResponse> => {
    const [joined, pinSet] = await Promise.all([hasJoined(), hasPin()])
    const staff = await kvGet<SessionStaff>('staff')
    return {
      state: webAppState({ joined, hasPin: pinSet, unlocked: isUnlocked(), staff }),
      licence_status: null, // the PWA is a joined client; licensing lives on the school PC
    }
  },
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
