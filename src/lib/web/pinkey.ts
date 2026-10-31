// The PIN-derived wrapping key for the PWA's secrets (repair K2). The secret KV
// values (device token, session key, audience keys, pending invite) are sealed under
// the non-extractable WebCrypto data key (crypto.ts) AND, inside that, under this
// key — so a copied browser profile (or one opened without the PIN) cannot read them.
//
// Derivation reuses the app's existing primitives only: Argon2id with the SAME
// parameters as the PIN verifier (vidya-wasm `pin_hash_with_salt`, m=19456 t=2 p=1,
// 32-byte output) over a SEPARATE random 16-byte salt (`pin_wrap_salt`, kept in kv),
// and the 32-byte Argon2id output imported as a non-extractable AES-GCM key. The
// stored PIN verifier uses a different salt, so it reveals nothing about this key.
//
// The key lives ONLY in memory while the app is unlocked: lock (or idle auto-lock)
// drops it, and `currentPinKey()` re-checks the session on every use.

import { initWasm, pinHashWithSalt } from './wasm'

/** kv entry holding the wrap salt (16 random bytes; not secret). */
export const PIN_SALT_KV = 'pin_wrap_salt'

let key: CryptoKey | null = null
let guard: () => boolean = () => true

/** lock.ts installs `isUnlocked` here so an idle-expired session drops the key. */
export function setSessionGuard(fn: () => boolean): void {
  guard = fn
}

export function setPinKey(k: CryptoKey): void {
  key = k
}

export function clearPinKey(): void {
  key = null
}

/** The PIN key if the session is still unlocked, else null (and forgotten). */
export function currentPinKey(): CryptoKey | null {
  if (key && !guard()) key = null
  return key
}

function b64Decode(s: string): Uint8Array {
  const b64 = s.replace(/-/g, '+').replace(/_/g, '/')
  return Uint8Array.from(atob(b64 + '='.repeat((4 - (b64.length % 4)) % 4)), (c) => c.charCodeAt(0))
}

/** The raw Argon2id output from a PHC string (`$argon2id$v=19$m=…$<salt>$<hash>`). */
export function phcHash(phc: string): Uint8Array {
  const parts = phc.split('$')
  if (parts.length < 6 || !parts[1].startsWith('argon2')) throw new Error('not an argon2 PHC string')
  const out = b64Decode(parts[parts.length - 1])
  if (out.length !== 32) throw new Error('unexpected argon2 output length')
  return out
}

/** Import 32 derived bytes as a non-extractable AES-GCM key, then wipe them. */
export async function pinKeyFromBytes(bytes: Uint8Array): Promise<CryptoKey> {
  try {
    return await crypto.subtle.importKey('raw', bytes as BufferSource, { name: 'AES-GCM' }, false, ['encrypt', 'decrypt'])
  } finally {
    bytes.fill(0)
  }
}

/** Derive the wrapping key from the PIN + wrap salt (Argon2id via WASM). */
export async function derivePinKey(pin: string, salt: Uint8Array): Promise<CryptoKey> {
  await initWasm()
  return pinKeyFromBytes(phcHash(pinHashWithSalt(pin, salt)))
}

export function newSalt(): Uint8Array {
  return crypto.getRandomValues(new Uint8Array(16))
}
