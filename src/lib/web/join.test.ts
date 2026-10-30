import { describe, expect, it } from 'vitest'
import { parseJoinFragment } from './join'

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
