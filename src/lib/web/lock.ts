// PIN unlock + auto-lock for the iPhone PWA (Phase 19, Step 3, §9). The PIN is
// hashed/verified with WASM Argon2id (identical parameters to the app); the PHC
// string + failure count + lockout time live in the local kv store. The AES-GCM
// data key (crypto.ts) that encrypts records is independent of the PIN — the PIN gates
// the UI/session; that key protects data at rest. Auto-lock after 5 minutes hidden.

//
// K2: the PIN ALSO derives (Argon2id, same params, separate salt — pinkey.ts) the
// key that wraps the stored secrets. It is held in memory only while unlocked;
// lock / idle auto-lock drop it. Unlock migrates any secret not yet PIN-wrapped;
// a PIN change re-wraps every secret in one transaction.

import { initWasm, pinHash, pinIsLocked, pinLockoutSeconds, pinVerify } from './wasm'
import { kvGet, kvPut, rewrapSecrets } from './store'
import { clearPinKey, currentPinKey, derivePinKey, newSalt, PIN_SALT_KV, setPinKey, setSessionGuard } from './pinkey'

const KEY_PHC = 'pin_phc'
const KEY_FAIL = 'pin_fail'
const KEY_LOCKED_UNTIL = 'pin_locked_until'
const AUTO_LOCK_MS = 5 * 60 * 1000

let unlocked = false
/** Wall-clock ms of the last real user input while unlocked (idle auto-lock). */
let lastActivity = 0

/** True when an unlocked session's last input is older than the auto-lock limit.
 *  Timestamp-based (not a timer), so a suspended/backgrounded iPhone still locks.
 *  A clock that jumped backwards fails closed. Pure → unit-tested. */
export function idleExpired(lastActivityMs: number, nowMs: number): boolean {
  return nowMs - lastActivityMs > AUTO_LOCK_MS || lastActivityMs > nowMs + 60_000
}

/** True while the app is unlocked this session (and not idle past the limit). */
export function isUnlocked(): boolean {
  if (unlocked && idleExpired(lastActivity, Date.now())) {
    unlocked = false
    clearPinKey()
  }
  return unlocked
}

// The PIN key is only usable while this session is unlocked (idle expiry drops it).
setSessionGuard(isUnlocked)

/** Derive the PIN key (creating the wrap salt on first use), hold it in memory and
 *  migrate any secret not yet PIN-wrapped. Best-effort migration: a failure leaves
 *  the secrets readable under the data-key layer and retries at the next unlock. */
async function holdPinKey(pin: string): Promise<void> {
  let salt = await kvGet<Uint8Array>(PIN_SALT_KV)
  if (!(salt instanceof Uint8Array) || salt.length !== 16) {
    salt = newSalt()
    await kvPut(PIN_SALT_KV, salt)
  }
  const key = await derivePinKey(pin, salt)
  setPinKey(key)
  try {
    await rewrapSecrets(key, key)
  } catch {
    /* retried at the next unlock */
  }
}

/** Record real user input: pushes the idle deadline out while still unlocked. */
export function touch(): void {
  if (isUnlocked()) lastActivity = Date.now()
}

/** True once a PIN has been created on this device. */
export async function hasPin(): Promise<boolean> {
  return (await kvGet<string>(KEY_PHC)) != null
}

/** Create the PIN (once, at join/first run). Leaves the app unlocked. Refused once a
 *  PIN exists — otherwise anyone holding a locked phone could just set a new one. */
export async function createPin(pin: string): Promise<void> {
  if (await hasPin()) throw new Error('pin_already_set')
  await initWasm()
  await kvPut(KEY_PHC, pinHash(pin))
  await kvPut(KEY_FAIL, 0)
  await kvPut(KEY_LOCKED_UNTIL, null)
  await kvPut(PIN_SALT_KV, newSalt()) // fresh wrap salt for this PIN
  unlocked = true
  lastActivity = Date.now()
  // Wrap the secrets the join stored (data-key layer only until now) under the PIN.
  await holdPinKey(pin)
  // Ask the browser to keep our data (§18, spike Q6): a persisted origin is
  // excluded from storage-pressure eviction, and Safari 17+/iOS grants persistence
  // heuristically for a Home-Screen web app. Best-effort — a `false` or unsupported
  // result is fine (the app still works; the data is just not eviction-proof).
  try {
    await navigator.storage?.persist?.()
  } catch {
    /* StorageManager unavailable — best-effort only */
  }
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
    lastActivity = Date.now()
    await holdPinKey(pin)
    return { ok: true }
  }
  const fail = ((await kvGet<number>(KEY_FAIL)) ?? 0) + 1
  await kvPut(KEY_FAIL, fail)
  const wait = pinLockoutSeconds(fail)
  if (wait != null) await kvPut(KEY_LOCKED_UNTIL, now + wait * 1000)
  const remaining = Math.max(0, 5 - fail)
  return { ok: false, locked_seconds: wait ?? undefined, remaining }
}

/** Lock the app (drop the in-memory session flag and the PIN key). */
export function lock(): void {
  unlocked = false
  clearPinKey()
}

/** Change the PIN while unlocked: verify the current one, then re-wrap every secret
 *  under a key from the new PIN + a fresh salt and store the new verifier — all in
 *  ONE transaction, so a failure leaves the old PIN and wrapping intact. (No UI calls
 *  this yet: there is no change-PIN command on any platform today.) */
export async function changePin(oldPin: string, newPin: string): Promise<void> {
  if (!isUnlocked()) throw { code: 'LOCKED', message_key: 'error.LOCKED', vars: {} }
  await initWasm()
  const phc = await kvGet<string>(KEY_PHC)
  if (!phc || !pinVerify(oldPin, phc)) throw { code: 'PIN_WRONG', message_key: 'error.PIN_WRONG', vars: {} }
  const oldSalt = await kvGet<Uint8Array>(PIN_SALT_KV)
  const oldKey = currentPinKey() ?? (oldSalt instanceof Uint8Array ? await derivePinKey(oldPin, oldSalt) : null)
  const salt = newSalt()
  const newKey = await derivePinKey(newPin, salt)
  await rewrapSecrets(oldKey, newKey, [
    [KEY_PHC, pinHash(newPin)],
    [PIN_SALT_KV, salt],
  ])
  setPinKey(newKey)
  lastActivity = Date.now()
}

/** Install the 5-minute auto-lock: any real input pushes the deadline out; time
 *  hidden or asleep counts toward it (checked on every command and on return). */
export function installAutoLock(onLocked: () => void): void {
  if (typeof document === 'undefined') return
  for (const ev of ['pointerdown', 'keydown', 'touchstart']) {
    document.addEventListener(ev, touch, { passive: true, capture: true })
  }
  const check = () => {
    if (unlocked && !isUnlocked()) onLocked()
  }
  document.addEventListener('visibilitychange', check)
  setInterval(check, 15_000)
}
