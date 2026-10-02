import { describe, expect, it } from 'vitest'
import { buildStaffDay, type StaffDayInput } from './staffday'

// The teacher-home aggregation, mirroring the Rust my_staff_day_logic. Records are the
// shapes sync populates in the store; the test fixes a staff member + "today" and checks
// today's row, the month buckets, the recent (DESC, ≤8) list, and leave balances.
function base(): StaffDayInput {
  return {
    staffId: 'stf-1',
    today: '2026-10-03',
    school: { settings_json: JSON.stringify({ hr: { start_time: '08:30' } }) },
    session: { is_current: 1, starts_on: '2026-06-01', ends_on: '2027-03-31' },
    attendance: [
      { id: 'a1', staff_id: 'stf-1', date: '2026-10-03', status: 'late', check_in_min: 545, check_out_min: null, route: 'lan', clock_warning: 1, note: null },
      { id: 'a2', staff_id: 'stf-1', date: '2026-10-02', status: 'present', check_in_min: 500, check_out_min: 1020, route: 'drive', clock_warning: 0, note: 'ok' },
      { id: 'a3', staff_id: 'stf-1', date: '2026-10-01', status: 'leave', check_in_min: null, check_out_min: null, route: null, clock_warning: 0, note: null },
      { id: 'a4', staff_id: 'stf-1', date: '2026-09-30', status: 'present', check_in_min: 500, check_out_min: 1020, route: null, clock_warning: 0, note: null },
      // another staff's row must be ignored
      { id: 'a5', staff_id: 'stf-2', date: '2026-10-03', status: 'present', check_in_min: 500, check_out_min: null, route: null, clock_warning: 0, note: null },
    ],
    leaveTypes: [
      { id: 'lt-cl', name: 'Casual', name_hi: null, name_te: null, yearly_quota: 12, paid: 1, active: 1, sort_order: 1 },
      { id: 'lt-un', name: 'Unpaid', name_hi: null, name_te: null, yearly_quota: null, paid: 0, active: 1, sort_order: 2 },
      { id: 'lt-old', name: 'Retired', name_hi: null, name_te: null, yearly_quota: 5, paid: 1, active: 0, sort_order: 3 },
    ],
    leaveRecords: [
      { id: 'lr1', staff_id: 'stf-1', leave_type_id: 'lt-cl', from_date: '2026-09-10', days: 2 },
      { id: 'lr2', staff_id: 'stf-1', leave_type_id: 'lt-cl', from_date: '2026-05-01', days: 3 }, // before session → excluded
      { id: 'lr3', staff_id: 'stf-2', leave_type_id: 'lt-cl', from_date: '2026-09-10', days: 9 }, // other staff → excluded
    ],
  }
}

describe('buildStaffDay', () => {
  it('reports today, the month buckets and the recent list for this staff only', () => {
    const d = buildStaffDay(base())
    expect(d.date).toBe('2026-10-03')
    expect(d.start_time).toBe('08:30')
    expect(d.today).toEqual({ status: 'late', check_in_min: 545, check_out_min: null, route: 'lan', clock_warning: true })
    // October: a1(late), a2(present), a3(leave). present bucket = present|late|half_day.
    expect(d.month_present).toBe(2)
    expect(d.month_leave).toBe(1)
    expect(d.month_late).toBe(1)
    // recent: this staff, date DESC, ≤8 — the Sept row included, the other staff excluded.
    expect(d.recent.map((r) => r.date)).toEqual(['2026-10-03', '2026-10-02', '2026-10-01', '2026-09-30'])
  })

  it('computes leave balances over the session, skipping inactive types', () => {
    const d = buildStaffDay(base())
    expect(d.leave_balances.map((b) => b.leave_type_id)).toEqual(['lt-cl', 'lt-un']) // inactive 'lt-old' dropped
    const casual = d.leave_balances[0]
    expect(casual.approved).toBe(2) // only the in-session, same-staff record
    expect(casual.balance).toBe(10) // 12 − 2
    const unpaid = d.leave_balances[1]
    expect(unpaid.yearly_quota).toBeNull()
    expect(unpaid.balance).toBeNull() // unlimited
  })

  it('has no today row and defaults the start time when nothing has synced', () => {
    const d = buildStaffDay({ ...base(), attendance: [], school: undefined, leaveTypes: [], leaveRecords: [] })
    expect(d.today).toBeNull()
    expect(d.month_present).toBe(0)
    expect(d.recent).toEqual([])
    expect(d.leave_balances).toEqual([])
    expect(d.start_time).toBe('09:00') // DEFAULT_START_MIN
  })
})
