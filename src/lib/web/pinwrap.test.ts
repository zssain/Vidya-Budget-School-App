// K2: PIN-derived wrapping of the PWA's stored secrets. The envelope functions are
// pure given the keys, so they are tested here with Node's WebCrypto; the Argon2id
// derivation itself is the vidya-wasm function already covered by the Rust tests
// (identical params) and the WebKit PWA harness.
import { describe, expect, it } from 'vitest'
import { isPinSealed, openSecretWith, rewrapValues, sealSecretWith, SECRET_KEYS } from './store'
import { clearPinKey, currentPinKey, phcHash, pinKeyFromBytes, setPinKey, setSessionGuard } from './pinkey'
import { encryptBytesWith } from './crypto'

const aes = () => crypto.subtle.generateKey({ name: 'AES-GCM', length: 256 }, false, ['encrypt', 'decrypt'])

describe('secret envelope (two layers)', () => {
  it('round-trips under data key + PIN key and is PIN-sealed', async () => {
    const [dk, pk] = await Promise.all([aes(), aes()])
    const s = await sealSecretWith('session_key', 'c2VjcmV0', dk, pk)
    expect(isPinSealed(s)).toBe(true)
    expect(await openSecretWith('session_key', s, dk, pk)).toBe('c2VjcmV0')
  })
  it('cannot be read without the PIN key (a copied, locked profile)', async () => {
    const [dk, pk] = await Promise.all([aes(), aes()])
    const s = await sealSecretWith('audience_keys', { 'class:c1': 'k' }, dk, pk)
    await expect(openSecretWith('audience_keys', s, dk, null)).rejects.toMatchObject({ code: 'LOCKED' })
  })
  it('fails closed with the wrong PIN key or under another kv name (AAD)', async () => {
    const [dk, pk, wrong] = await Promise.all([aes(), aes(), aes()])
    const s = await sealSecretWith('device_token', 'tok', dk, pk)
    await expect(openSecretWith('device_token', s, dk, wrong)).rejects.toBeTruthy()
    await expect(openSecretWith('session_key', s, dk, pk)).rejects.toBeTruthy()
  })
  it('still reads the older data-key-only and plain formats', async () => {
    const dk = await aes()
    const legacy = { sealed: await encryptBytesWith(dk, new TextEncoder().encode(JSON.stringify('old'))) }
    expect(await openSecretWith('device_token', legacy, dk, null)).toBe('old')
    expect(await openSecretWith('device_token', 'plain', dk, null)).toBe('plain')
    expect(await openSecretWith('device_token', null, dk, null)).toBeUndefined()
  })
})

describe('rewrapValues (migration at unlock, PIN change)', () => {
  it('migrates data-key-only and plain secrets under the PIN key, skips the ones already wrapped', async () => {
    const [dk, pk] = await Promise.all([aes(), aes()])
    const legacy = await sealSecretWith('device_token', 'tok', dk, null)
    const wrapped = await sealSecretWith('session_key', 'sk', dk, pk)
    const out = await rewrapValues([['device_token', legacy], ['session_key', wrapped], ['audience_keys', { a: 1 }], ['pending_join_fragment', null]], dk, pk, pk)
    expect(out.map(([n]) => n)).toEqual(['device_token', 'audience_keys'])
    for (const [n, v] of out) {
      expect(isPinSealed(v)).toBe(true)
      expect(await openSecretWith(n, v, dk, pk)).toEqual(n === 'device_token' ? 'tok' : { a: 1 })
    }
  })
  it('re-wraps every secret on a PIN change; the old PIN key no longer opens them', async () => {
    const [dk, oldPk, newPk] = await Promise.all([aes(), aes(), aes()])
    const entries: [string, unknown][] = [
      ['device_token', await sealSecretWith('device_token', 'tok', dk, oldPk)],
      ['session_key', await sealSecretWith('session_key', 'sk', dk, oldPk)],
    ]
    const out = await rewrapValues(entries, dk, oldPk, newPk)
    expect(out).toHaveLength(2)
    expect(await openSecretWith('session_key', out[1][1], dk, newPk)).toBe('sk')
    await expect(openSecretWith('session_key', out[1][1], dk, oldPk)).rejects.toBeTruthy()
  })
  it('refuses (throws, nothing dropped) when the old key cannot open a wrapped secret', async () => {
    const [dk, pk, other] = await Promise.all([aes(), aes(), aes()])
    const entries: [string, unknown][] = [['session_key', await sealSecretWith('session_key', 'sk', dk, pk)]]
    await expect(rewrapValues(entries, dk, other, pk)).rejects.toBeTruthy()
    await expect(rewrapValues(entries, dk, null, pk)).rejects.toMatchObject({ code: 'LOCKED' })
  })
  it('covers every secret kv entry', () => {
    expect([...SECRET_KEYS].sort()).toEqual(['audience_keys', 'device_token', 'pending_join_fragment', 'session_key'])
  })
})

describe('PIN key derivation + lifetime', () => {
  it('takes the 32-byte Argon2id output from the PHC string', () => {
    const hash = btoa(String.fromCharCode(...new Uint8Array(32).map((_, i) => i))).replace(/=+$/, '')
    const phc = `$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHRzYWx0c2FsdA$${hash}`
    expect([...phcHash(phc)]).toEqual([...Array(32).keys()])
    expect(() => phcHash('$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$AAAA')).toThrow()
    expect(() => phcHash('not-a-phc')).toThrow()
  })
  it('imports a non-extractable AES-GCM key and wipes the derived bytes', async () => {
    const bytes = new Uint8Array(32).fill(7)
    const key = await pinKeyFromBytes(bytes)
    expect(key.extractable).toBe(false)
    expect(key.algorithm.name).toBe('AES-GCM')
    expect([...bytes].every((b) => b === 0)).toBe(true)
  })
  it('is held only while the session is unlocked; lock clears it', async () => {
    let unlocked = true
    setSessionGuard(() => unlocked)
    setPinKey(await aes())
    expect(currentPinKey()).not.toBeNull()
    unlocked = false // idle auto-lock
    expect(currentPinKey()).toBeNull()
    unlocked = true
    expect(currentPinKey()).toBeNull() // forgotten, not just hidden
    setPinKey(await aes())
    clearPinKey()
    expect(currentPinKey()).toBeNull()
  })
})
