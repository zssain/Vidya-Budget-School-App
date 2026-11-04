import { describe, expect, it, vi } from 'vitest'

// hasJoined runs while LOCKED (app_state at relaunch), when the session key is
// PIN-wrapped and opening it rejects LOCKED. A fake kv models exactly that.
const kv = new Map<string, unknown>()
vi.mock('./store', () => ({
  kvGet: async (k: string) => kv.get(k),
  kvPut: async (k: string, v: unknown) => void kv.set(k, v),
  secretGet: async () => {
    throw { code: 'LOCKED', message_key: 'error.LOCKED', vars: {} }
  },
  secretPut: async () => undefined,
}))

import { hasJoined, parseJoinFragment } from './join'

const enc = (o: unknown) =>
  btoa(JSON.stringify(o)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
const secret = btoa(String.fromCharCode(...new Uint8Array(32).fill(7))).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')

describe('parseJoinFragment (join v2)', () => {
  it('accepts a v2 web invite with a 256-bit secret', () => {
    const p = { v: 2, invite_id: 'inv-1', secret, school_name: 'Saraswati', school_id: 'sch-1' }
    expect(parseJoinFragment(`#d=${enc(p)}`)).toEqual(p)
  })
  it('refuses an old v1 link carrying a plaintext invite code', () => {
    expect(parseJoinFragment(`#d=${enc({ invite_code: 'ABCD1234', school_name: 'S', school_id: 'x' })}`)).toBeNull()
  })
  it('refuses a short / missing secret', () => {
    expect(parseJoinFragment(`#d=${enc({ v: 2, invite_id: 'i', secret: 'c2hvcnQ', school_name: 'S', school_id: 'x' })}`)).toBeNull()
    expect(parseJoinFragment(`#d=${enc({ v: 2, invite_id: 'i', school_name: 'S', school_id: 'x' })}`)).toBeNull()
  })
  it('ignores garbage', () => {
    expect(parseJoinFragment('#d=%%%')).toBeNull()
    expect(parseJoinFragment('')).toBeNull()
  })
})

describe('hasJoined', () => {
  it('is true while locked with a PIN-wrapped session key (never rejects LOCKED)', async () => {
    kv.clear()
    kv.set('device_id', 'dev-1')
    kv.set('session_key', { pinsealed: new Uint8Array([1, 2, 3]) })
    await expect(hasJoined()).resolves.toBe(true)
  })
  it('is false without an identity or a stored session key (incl. a cleared one)', async () => {
    kv.clear()
    await expect(hasJoined()).resolves.toBe(false)
    kv.set('device_id', 'dev-1')
    await expect(hasJoined()).resolves.toBe(false)
    kv.set('session_key', null)
    await expect(hasJoined()).resolves.toBe(false)
  })
})
