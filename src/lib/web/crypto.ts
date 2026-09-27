// Encryption at rest for the iPhone PWA (Phase 19, Step 3, §18/§9). Records are
// encrypted with an AES-GCM key created by WebCrypto as **non-extractable** and
// stored in IndexedDB: the key handle is usable for encrypt/decrypt but its bytes
// can never be read back out (or exfiltrated), even by the app's own JS. The PIN
// gates the app (lock.ts); this key protects the data if the device is inspected.

import { KV, openDb, reqDone } from './db'

const DB_KEY_ID = 'db_key'
const IV_LEN = 12

interface KvKeyRow {
  key: string
  value: CryptoKey
}

async function loadOrCreateKey(): Promise<CryptoKey> {
  const db = await openDb()
  const existing = await reqDone<KvKeyRow | undefined>(
    db.transaction(KV, 'readonly').objectStore(KV).get(DB_KEY_ID),
  )
  if (existing?.value) return existing.value
  // extractable = false → a CryptoKey handle usable for AES-GCM but not exportable.
  const key = await crypto.subtle.generateKey({ name: 'AES-GCM', length: 256 }, false, ['encrypt', 'decrypt'])
  await new Promise<void>((resolve, reject) => {
    const t = db.transaction(KV, 'readwrite')
    t.objectStore(KV).put({ key: DB_KEY_ID, value: key })
    t.oncomplete = () => resolve()
    t.onerror = () => reject(t.error)
  })
  return key
}

let keyPromise: Promise<CryptoKey> | null = null
/** The non-extractable AES-GCM key, created on first use and cached for the session. */
export function dbKey(): Promise<CryptoKey> {
  if (!keyPromise) keyPromise = loadOrCreateKey()
  return keyPromise
}

/** Encrypt a JSON-serialisable value → `iv(12) ‖ AES-GCM ciphertext`. */
export async function encryptJson(value: unknown): Promise<Uint8Array> {
  const key = await dbKey()
  const iv = crypto.getRandomValues(new Uint8Array(IV_LEN))
  const pt = new TextEncoder().encode(JSON.stringify(value))
  const ct = new Uint8Array(await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, pt))
  const out = new Uint8Array(IV_LEN + ct.length)
  out.set(iv, 0)
  out.set(ct, IV_LEN)
  return out
}

/** Decrypt an `iv(12) ‖ ciphertext` blob back to its value. Throws on tamper. */
export async function decryptJson<T>(enc: Uint8Array): Promise<T> {
  const key = await dbKey()
  const iv = enc.slice(0, IV_LEN)
  const ct = enc.slice(IV_LEN)
  const pt = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, key, ct)
  return JSON.parse(new TextDecoder().decode(pt)) as T
}
