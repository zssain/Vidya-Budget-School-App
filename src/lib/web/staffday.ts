// The teacher-home aggregation (Phase 20, C3) — a faithful TS port of the Rust
// my_staff_day_logic (src-tauri/src/commands/logic.rs), computed over the records the
// Drive sync populates in the local store. Kept PURE (records in → DTO out) so it is
// unit-tested directly; the backend handler is a thin shell that reads the store.
//
// Record shapes below mirror the DB columns my_staff_day reads, which equal the op
// payload the app/server push — the shared op format is the whole premise of sync.

import type { LeaveBalance, StaffDayDto, StaffDayRecent, StaffDayToday } from '../api'

export interface StaffAttendanceRec {
  id: string
  staff_id: string
  date: string
  status: string
  check_in_min: number | null
  check_out_min: number | null
  route: string | null
  clock_warning: number | boolean | null
  note: string | null
}
export interface LeaveTypeRec {
  id: string
  name: string
  name_hi: string | null
  name_te: string | null
  yearly_quota: number | null
  paid: number | boolean
  active: number | boolean
  sort_order?: number | null
}
export interface LeaveRecordRec {
  id: string
  staff_id: string
  leave_type_id: string
  from_date: string
  days: number
}
export interface SchoolRec {
  settings_json?: string | null
}
export interface SessionRec {
  is_current?: number | boolean
  starts_on: string
  ends_on: string
}

const DEFAULT_START_MIN = 540 // 09:00 — vidya_core::hr::DEFAULT_START_MIN
const PRESENT = new Set(['present', 'late', 'half_day']) // my_staff_day "present" bucket

/** 0/1/"1"/true → true (DB stores bools as 0/1; op payloads may carry either). */
export function truthy(v: unknown): boolean {
  return v === true || v === 1 || v === '1'
}

const pad2 = (n: number): string => (n < 10 ? `0${n}` : String(n))

/** minutes-from-midnight → "HH:MM" — vidya_core::hr::fmt_hhmm. */
function fmtHhmm(min: number): string {
  return `${pad2(Math.floor(min / 60))}:${pad2(min % 60)}`
}

/** "HH:MM" → minutes, or null — vidya_core::hr::parse_hhmm. */
function parseHhmm(s: string): number | null {
  const m = /^(\d{1,2}):(\d{2})$/.exec(s.trim())
  if (!m) return null
  const h = Number(m[1])
  const mi = Number(m[2])
  return h <= 23 && mi <= 59 ? h * 60 + mi : null
}

/** vidya_core::hr::leave_balance — unlimited (no quota) → null, else quota − approved. */
function leaveBalance(quota: number | null, approved: number): number | null {
  return quota == null ? null : quota - approved
}

function startMinOf(school: SchoolRec | undefined): number {
  try {
    const v = school?.settings_json ? (JSON.parse(school.settings_json) as { hr?: { start_time?: unknown } }) : {}
    const st = v?.hr?.start_time
    const m = typeof st === 'string' ? parseHhmm(st) : null
    return m ?? DEFAULT_START_MIN
  } catch {
    return DEFAULT_START_MIN
  }
}

export interface StaffDayInput {
  staffId: string
  today: string // YYYY-MM-DD
  attendance: StaffAttendanceRec[]
  school: SchoolRec | undefined
  session: SessionRec | null
  leaveTypes: LeaveTypeRec[]
  leaveRecords: LeaveRecordRec[]
}

export function buildStaffDay(input: StaffDayInput): StaffDayDto {
  const mine = input.attendance.filter((a) => a.staff_id === input.staffId)
  const month = input.today.slice(0, 7)

  const todayRow = mine.find((a) => a.date === input.today)
  const today: StaffDayToday | null = todayRow
    ? {
        status: todayRow.status,
        check_in_min: todayRow.check_in_min ?? null,
        check_out_min: todayRow.check_out_min ?? null,
        route: todayRow.route ?? null,
        clock_warning: truthy(todayRow.clock_warning),
      }
    : null

  let month_present = 0
  let month_leave = 0
  let month_late = 0
  for (const a of mine) {
    if (!a.date.startsWith(month)) continue
    if (PRESENT.has(a.status)) month_present += 1
    if (a.status === 'leave') month_leave += 1
    if (a.status === 'late') month_late += 1
  }

  const recent: StaffDayRecent[] = [...mine]
    .sort((x, y) => (x.date < y.date ? 1 : x.date > y.date ? -1 : 0)) // date DESC
    .slice(0, 8)
    .map((a) => ({
      date: a.date,
      status: a.status,
      check_in_min: a.check_in_min ?? null,
      check_out_min: a.check_out_min ?? null,
      note: a.note ?? null,
    }))

  // Approved leave of each type in the current session (null bounds → count all).
  const bounds = input.session ? { s: input.session.starts_on, e: input.session.ends_on } : null
  const leave_balances: LeaveBalance[] = input.leaveTypes
    .filter((lt) => truthy(lt.active))
    .sort((a, b) => (a.sort_order ?? 0) - (b.sort_order ?? 0) || (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
    .map((lt) => {
      let approved = 0
      for (const r of input.leaveRecords) {
        if (r.staff_id !== input.staffId || r.leave_type_id !== lt.id) continue
        if (bounds && (r.from_date < bounds.s || r.from_date > bounds.e)) continue
        approved += r.days ?? 0
      }
      const quota = lt.yearly_quota ?? null
      return {
        leave_type_id: lt.id,
        name: lt.name,
        name_hi: lt.name_hi ?? null,
        name_te: lt.name_te ?? null,
        yearly_quota: quota,
        approved,
        balance: leaveBalance(quota, approved),
        paid: truthy(lt.paid),
      }
    })

  return {
    date: input.today,
    start_time: fmtHhmm(startMinOf(input.school)),
    today,
    month_present,
    month_leave,
    month_late,
    recent,
    leave_balances,
  }
}
