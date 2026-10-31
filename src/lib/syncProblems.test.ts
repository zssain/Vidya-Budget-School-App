import { describe, expect, it } from 'vitest'
import { describeSyncProblem } from './syncProblems'
import { toSyncProblems, type OutboxRow } from './web/store'
import { enDict, hiDict, t } from './i18n'
import syncProblems from './i18n/strings/syncProblems'

const tr = (key: string) => enDict[key] ?? key

describe('describeSyncProblem (B1)', () => {
  it('names the kind of change and the reason in words', () => {
    expect(describeSyncProblem({ id: 'op1', reason: 'SHEET_LOCKED', table: 'marks_sheet' }, tr)).toEqual({
      what: 'Marks',
      why: enDict['error.SHEET_LOCKED'],
    })
  })
  it('falls back to the raw code, "no reason" and a generic kind', () => {
    expect(describeSyncProblem({ id: 'op1', reason: 'SOMETHING_NEW' }, tr)).toEqual({ what: 'A change', why: 'SOMETHING_NEW' })
    expect(describeSyncProblem({ id: 'op1', reason: null, table: 'unknown_table' }, tr)).toEqual({ what: 'A change', why: enDict['syncp.noReason'] })
  })
  it('has every string in en, hi and te', () => {
    const en = Object.keys(syncProblems.en).sort()
    expect(Object.keys(syncProblems.hi).sort()).toEqual(en)
    expect(Object.keys(syncProblems.te ?? {}).sort()).toEqual(en)
    expect(hiDict['syncp.title']).toBeTruthy()
    expect(t('syncp.banner', { n: 2 })).toContain('2')
  })
})

describe('toSyncProblems (PWA list_sync_problems)', () => {
  const row = (op_id: string, hlc: string, extra: Partial<OutboxRow> = {}): OutboxRow => ({ op_id, hlc, audience: 'class:c1', enc: new Uint8Array(), ...extra })
  it('lists only refused ops, oldest first, with the reason and the op table', async () => {
    const rows = [
      row('b', '2', { status: 'rejected', reason: 'FORBIDDEN' }),
      row('pending', '0'),
      row('a', '1', { status: 'conflict', reason: null }),
    ]
    const out = await toSyncProblems(rows, async (r) => ({ table: r.op_id === 'a' ? 'attendance_sheet' : 'marks_sheet' }))
    expect(out).toEqual([
      { id: 'a', reason: null, table: 'attendance_sheet' },
      { id: 'b', reason: 'FORBIDDEN', table: 'marks_sheet' },
    ])
  })
  it('still lists an op it cannot decrypt (without its table)', async () => {
    const out = await toSyncProblems([row('x', '1', { status: 'flagged', reason: 'MALFORMED' })], async () => {
      throw new Error('locked')
    })
    expect(out).toEqual([{ id: 'x', reason: 'MALFORMED' }])
  })
})
