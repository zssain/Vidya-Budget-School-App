// Loads vidya-wasm once and exposes ergonomic, typed wrappers over it (Phase 19,
// Step 3). The PWA calls THESE for every rule/HLC/seal/PIN decision, so it behaves
// identically to the Android/desktop app (the wasm is compiled from the same
// vidya-core). Randomness (seal nonce, PIN salt) is drawn from the browser's
// WebCrypto here and passed into the pure wasm functions — vidya-core stays
// deterministic (§18).

import init, * as w from '@wasm'

let ready: Promise<void> | null = null

/** Load + instantiate the WASM module once (idempotent). Await before any wasm use. */
export function initWasm(): Promise<void> {
  // The generated ESM fetches its co-located `.wasm` (a Vite asset, precached by
  // the service worker in Step 6). `init()` with no argument uses that default.
  ready ??= init().then(() => undefined)
  return ready
}

function randomBytes(n: number): Uint8Array {
  const b = new Uint8Array(n)
  crypto.getRandomValues(b)
  return b
}

// ---- rules + HLC ----
export const validateNewMark = (mark: string): string | null => w.validate_new_mark(mark) ?? null
export const percentPresent = (p: number, a: number, l: number): number => w.percent_present(p, a, l)
export const hlcNext = (prev: string | null, wallMs: number, deviceId: string): string =>
  w.hlc_next(prev ?? undefined, wallMs, deviceId)
export const hlcParse = (s: string): { wall_ms: number; counter: number; device_id: string } =>
  JSON.parse(w.hlc_parse(s))
export const hlcSkew = (localMs: number, remoteMs: number): boolean => w.hlc_skew(localMs, remoteMs)

// ---- sealing (the browser supplies the 12-byte nonce) ----
export const bundleAad = (audience: string, keyVersion: number): Uint8Array => w.bundle_aad(audience, keyVersion)
export const relayAad = (method: string, path: string, deviceId: string, epoch: number): Uint8Array =>
  w.relay_aad(method, path, deviceId, epoch)
export function seal(key: Uint8Array, aad: Uint8Array, plaintext: Uint8Array): Uint8Array {
  return w.seal_with_nonce(key, aad, randomBytes(12), plaintext)
}
export const sealOpen = (key: Uint8Array, aad: Uint8Array, sealed: Uint8Array): Uint8Array =>
  w.seal_open(key, aad, sealed)
/** AAD for the school PC's per-device down export (open with the s2c key). */
export const deviceDownAad = (deviceId: string): Uint8Array => w.device_down_aad(deviceId)
/** AAD for a device-sealed Drive bundle (format v2; sealed with the c2s key). */
export const deviceBundleAad = (deviceId: string): Uint8Array => w.device_bundle_aad(deviceId)
export function deriveDirectionKeys(sessionKeyB64: string): { c2s: Uint8Array; s2c: Uint8Array } {
  const both = w.derive_direction_keys(sessionKeyB64)
  return { c2s: both.slice(0, 32), s2c: both.slice(32, 64) }
}
// Join-without-LAN v2 (§18): proof of the invite secret + the per-request
// response key/AAD. The secret never leaves the device.
export const joinV2Proof = (secret: Uint8Array, inviteId: string, requestId: string, deviceName: string, platform: string): Uint8Array =>
  w.join_v2_proof(secret, inviteId, requestId, deviceName, platform)
export const joinV2ResponseKey = (secret: Uint8Array, requestId: string): Uint8Array => w.join_v2_response_key(secret, requestId)
export const joinV2Aad = (inviteId: string, requestId: string, schoolId: string): Uint8Array =>
  w.join_v2_aad(inviteId, requestId, schoolId)

// ---- PIN (the browser supplies the 16-byte salt) ----
export const pinHash = (pin: string): string => w.pin_hash_with_salt(pin, randomBytes(16))
/** The same Argon2id (same params) with a caller-held 16-byte salt — used to DERIVE
 *  the PIN wrapping key (K2) from a separate salt, never to store a verifier. */
export const pinHashWithSalt = (pin: string, salt: Uint8Array): string => w.pin_hash_with_salt(pin, salt)
export const pinVerify = (pin: string, phc: string): boolean => w.pin_verify(pin, phc)
export const pinLockoutSeconds = (failCount: number): number | null => w.pin_lockout_seconds(failCount) ?? null
export const pinRemaining = (failCount: number): number => w.pin_remaining(failCount)
export const pinIsLocked = (lockedUntilMs: number | null, nowMs: number): boolean =>
  w.pin_is_locked(lockedUntilMs ?? undefined, nowMs)
