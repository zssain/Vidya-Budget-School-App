// PIN unlock + auto-lock for the iPhone PWA (Phase 19, Step 3, §9). The PIN is
// hashed/verified with WASM Argon2id (identical parameters to the app); the PHC
// string + failure count + lockout time live in the local kv store. The AES-GCM
// data key (crypto.ts) is independent of the PIN — the PIN gates the UI/session; the
// key protects data at rest. Auto-lock after 5 minutes hidden.

import { initWasm, pinHash, pinIsLocked, pinLockoutSeconds, pinVerify } from './wasm'
import { kvGet, kvPut } from './store'

const KEY_PHC = 'pin_phc'
const KEY_FAIL = 'pin_fail'
const KEY_LOCKED_UNTIL = 'pin_locked_until'
const AUTO_LOCK_MS = 5 * 60 * 1000

let unlocked = false

/** True while the app is unlocked this session. */
export function isUnlocked(): boolean {
  return unlocked
}

/** True once a PIN has been created on this device. */
export async function hasPin(): Promise<boolean> {
  return (await kvGet<string>(KEY_PHC)) != null
}

/** Create the PIN (once, at join/first run). Leaves the app unlocked. */
export async function createPin(pin: string): Promise<void> {
  await initWasm()
  await kvPut(KEY_PHC, pinHash(pin))
  await kvPut(KEY_FAIL, 0)
  await kvPut(KEY_LOCKED_UNTIL, null)
  unlocked = true
}

export interface UnlockResult {
  ok: boolean
  /** Seconds the account is locked for (present when locked out). */
  locked_seconds?: number
  /** Tries left before the next lockout (present on a wrong PIN). */
  remaining?: number
}

/** Verify a PIN and unlock, applying the same lockout curve as the app (§9). */
export async function unlock(pin: string): Promise<UnlockResult> {
  await initWasm()
  const phc = await kvGet<string>(KEY_PHC)
  if (!phc) return { ok: false }
  const now = Date.now()
  const lockedUntil = (await kvGet<number | null>(KEY_LOCKED_UNTIL)) ?? null
  if (pinIsLocked(lockedUntil, now)) {
    return { ok: false, locked_seconds: Math.ceil(((lockedUntil as number) - now) / 1000) }
  }
  if (pinVerify(pin, phc)) {
    await kvPut(KEY_FAIL, 0)
    await kvPut(KEY_LOCKED_UNTIL, null)
    unlocked = true
    return { ok: true }
  }
  const fail = ((await kvGet<number>(KEY_FAIL)) ?? 0) + 1
  await kvPut(KEY_FAIL, fail)
  const wait = pinLockoutSeconds(fail)
  if (wait != null) await kvPut(KEY_LOCKED_UNTIL, now + wait * 1000)
  const remaining = Math.max(0, 5 - fail)
  return { ok: false, locked_seconds: wait ?? undefined, remaining }
}

/** Lock the app (drop the in-memory session flag). */
export function lock(): void {
  unlocked = false
}

/** Install the 5-minute auto-lock: lock when the app returns after being hidden. */
export function installAutoLock(): void {
  if (typeof document === 'undefined') return
  let hiddenAt = 0
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'hidden') {
      hiddenAt = Date.now()
    } else if (hiddenAt && Date.now() - hiddenAt > AUTO_LOCK_MS) {
      lock()
    }
  })
}
