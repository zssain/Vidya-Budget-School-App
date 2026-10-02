import { describe, expect, it } from 'vitest'
import { audienceFor, buildOp, newId } from './op'

// audienceFor mirrors vidya_core::audience::audience_for; buildOp assembles the op the
// sync loop pushes. The payload contract (which columns) is pinned per table by the
// Rust cross-language test (src-tauri/tests/web_op_apply.rs); here we lock the audience
// routing + op shape.
describe('audienceFor', () => {
  it('maps class-scoped tables to class:<id>', () => {
    expect(audienceFor('homework_note', 'cls-5a')).toBe('class:cls-5a')
    expect(audienceFor('attendance_mark', 'cls-5a')).toBe('class:cls-5a')
  })
  it('maps finance + admin domains', () => {
    expect(audienceFor('fee_due')).toBe('finance')
    expect(audienceFor('payment')).toBe('finance')
    expect(audienceFor('staff')).toBe('admin')
    expect(audienceFor('period')).toBe('admin')
  })
  it('throws when a class-scoped table has no class id', () => {
    expect(() => audienceFor('homework_note')).toThrow()
  })
  it('throws for request (requester-scoped, set at the write site) + unknown tables', () => {
    expect(() => audienceFor('request')).toThrow()
    expect(() => audienceFor('nope')).toThrow()
  })
})

describe('buildOp', () => {
  it('assembles the op from the device identity + clock', () => {
    const op = buildOp({
      deviceId: 'dev-web',
      staffId: 'stf-1',
      epoch: 3,
      hlc: 'HLC1',
      table: 'homework_note',
      recordId: 'hw-1',
      kind: 'insert',
      audience: 'class:cls-5a',
      payload: { text: 'hi' },
      baseVersion: null,
    })
    expect(op).toMatchObject({
      device_id: 'dev-web',
      staff_id: 'stf-1',
      server_epoch: 3,
      hlc: 'HLC1',
      table: 'homework_note',
      record_id: 'hw-1',
      kind: 'insert',
      audience: 'class:cls-5a',
      payload: { text: 'hi' },
      base_version: null,
    })
    expect(op.op_id).toMatch(/^op-[0-9a-f]{32}$/)
  })
})

describe('newId', () => {
  it('prefixes and is unique', () => {
    const a = newId('hw')
    const b = newId('hw')
    expect(a).toMatch(/^hw-[0-9a-f]{32}$/)
    expect(a).not.toBe(b)
  })
})
