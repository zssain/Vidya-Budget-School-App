import { describe, expect, it } from 'vitest'
import { buildTeacherTimetable, type TeacherTimetableInput } from './timetable'

// The teacher timetable, mirroring my_timetable_logic + SLOT_SELECT: this teacher's
// current (effective_to IS NULL) slots, INNER-joined to class/class_subject/subject,
// ordered by weekday then period; plus the current session's periods ordered by no.
function base(): TeacherTimetableInput {
  return {
    staffId: 'stf-1',
    staffName: 'Meena',
    sessionId: 'ses-1',
    slots: [
      { id: 's2', class_id: 'c5', weekday: 1, period_no: 3, class_subject_id: 'cs-sci', teacher_id: 'stf-1' },
      { id: 's1', class_id: 'c5', weekday: 1, period_no: 1, class_subject_id: 'cs-math', teacher_id: 'stf-1' },
      { id: 's3', class_id: 'c6', weekday: 2, period_no: 1, class_subject_id: 'cs-math', teacher_id: 'stf-1' },
      // another teacher's slot → excluded
      { id: 's4', class_id: 'c5', weekday: 3, period_no: 1, class_subject_id: 'cs-math', teacher_id: 'stf-2' },
      // ended slot → excluded
      { id: 's5', class_id: 'c5', weekday: 4, period_no: 1, class_subject_id: 'cs-math', teacher_id: 'stf-1', effective_to: '2026-09-01' },
      // dangling class_subject (not synced) → INNER JOIN drops it
      { id: 's6', class_id: 'c5', weekday: 5, period_no: 1, class_subject_id: 'cs-gone', teacher_id: 'stf-1' },
    ],
    classes: [
      { id: 'c5', display: '5A' },
      { id: 'c6', display: '6B' },
    ],
    classSubjects: [
      { id: 'cs-math', subject_id: 'sub-m' },
      { id: 'cs-sci', subject_id: 'sub-s' },
    ],
    subjects: [
      { id: 'sub-m', name: 'Maths' },
      { id: 'sub-s', name: 'Science' },
    ],
    periods: [
      { id: 'p2', no: 2, starts_at: '09:45', ends_at: '10:30', session_id: 'ses-1' },
      { id: 'p1', no: 1, starts_at: '09:00', ends_at: '09:45', session_id: 'ses-1' },
      { id: 'px', no: 1, starts_at: '08:00', ends_at: '08:45', session_id: 'ses-old' }, // other session
    ],
  }
}

describe('buildTeacherTimetable', () => {
  it('returns this teacher’s current slots, joined and ordered', () => {
    const d = buildTeacherTimetable(base())
    expect(d.slots.map((s) => s.id)).toEqual(['s1', 's2', 's3']) // mine, not ended, joinable; weekday/period order
    expect(d.slots[0]).toMatchObject({ class_display: '5A', subject_name: 'Maths', teacher_name: 'Meena', weekday: 1, period_no: 1 })
    expect(d.slots[1].subject_name).toBe('Science')
  })

  it('returns the current session’s periods ordered by number', () => {
    const d = buildTeacherTimetable(base())
    expect(d.periods.map((p) => p.id)).toEqual(['p1', 'p2']) // ses-old excluded, ordered by no
  })

  it('has no periods when no session is current', () => {
    expect(buildTeacherTimetable({ ...base(), sessionId: null }).periods).toEqual([])
  })
})
