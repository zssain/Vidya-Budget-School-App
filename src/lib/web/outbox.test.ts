import { describe, expect, it } from 'vitest'
import { REUPLOAD_AFTER_MS, rowsToPush, type OutboxRow } from './store'

const row = (op_id: string, hlc: string, extra: Partial<OutboxRow> = {}): OutboxRow => ({
  op_id,
  hlc,
  audience: 'class:1',
  enc: new Uint8Array(),
  ...extra,
})

describe('rowsToPush (PWA outbox)', () => {
  const now = 1_700_000_000_000
  it('pushes never-uploaded ops in HLC order', () => {
    expect(rowsToPush([row('b', '2'), row('a', '1')], now).map((r) => r.op_id)).toEqual(['a', 'b'])
  })
  it('does not re-upload an op every pass while its ack is pending', () => {
    expect(rowsToPush([row('a', '1', { uploaded_at: now - 60_000 })], now)).toEqual([])
  })
  it('re-uploads if no ack arrived for a long time (bundle may be lost)', () => {
    expect(rowsToPush([row('a', '1', { uploaded_at: now - REUPLOAD_AFTER_MS - 1 })], now)).toHaveLength(1)
  })
  it('never re-pushes an op the school PC refused (kept for the user instead)', () => {
    expect(rowsToPush([row('a', '1', { status: 'rejected', reason: 'FORBIDDEN' })], now)).toEqual([])
  })
})
