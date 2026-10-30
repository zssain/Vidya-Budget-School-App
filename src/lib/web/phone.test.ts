import { describe, expect, it } from 'vitest'
import { attendancePayload, buildAttendanceSheet, sheetIdFor } from './phone'

const enrollments = [
  { id: 'e1', student_id: 's1', class_id: 'c1', roll_no: 2, from_date: '2026-04-01', to_date: null },
  { id: 'e2', student_id: 's2', class_id: 'c1', roll_no: 1, from_date: '2026-04-01', to_date: null },
  { id: 'e3', student_id: 's3', class_id: 'c1', roll_no: 3, from_date: '2026-04-01', to_date: '2026-06-30' }, // left
  { id: 'e4', student_id: 's4', class_id: 'c2', roll_no: 1, from_date: '2026-04-01', to_date: null }, // other class
]
const students = [
  { id: 's1', name: 'Asha' },
  { id: 's2', name: 'Bilal' },
  { id: 's3', name: 'Chitra' },
  { id: 's4', name: 'Dev' },
]

describe('buildAttendanceSheet (iPhone)', () => {
  it('lists the class roster enrolled on the date, by roll number', () => {
    const dto = buildAttendanceSheet({ classId: 'c1', date: '2026-10-15', cls: { id: 'c1', display: 'V-A' }, enrollments, students, sheet: undefined, marks: [] })
    expect(dto.class_display).toBe('V-A')
    expect(dto.rows.map((r) => r.name)).toEqual(['Bilal', 'Asha'])
    expect(dto.status).toBe('draft')
  })
  it('shows canonical marks, or a pending local save in preference', () => {
    const sheet = { id: 'sh1', class_id: 'c1', date: '2026-10-15', status: 'submitted' }
    const marks = [{ id: 'm1', sheet_id: 'sh1', student_id: 's1', mark: 'A' }]
    const canonical = buildAttendanceSheet({ classId: 'c1', date: '2026-10-15', cls: undefined, enrollments, students, sheet, marks })
    expect(canonical.rows.find((r) => r.student_id === 's1')?.mark).toBe('A')
    const local = buildAttendanceSheet({
      classId: 'c1', date: '2026-10-15', cls: undefined, enrollments, students, marks,
      sheet: { ...sheet, status: 'draft', local_marks: [{ student_id: 's1', mark: 'P' }] },
    })
    expect(local.rows.find((r) => r.student_id === 's1')?.mark).toBe('P')
  })
})

describe('attendance op (matches the school PC replay)', () => {
  it('carries the complete sheet in one action payload', () => {
    expect(attendancePayload('c1', '2026-10-15', [{ student_id: 's1', mark: 'P' }], true)).toEqual({
      class_id: 'c1', date: '2026-10-15', submit: true, marks: [{ student_id: 's1', mark: 'P' }],
    })
  })
  it('uses the deterministic sheet id for a new sheet', () => {
    expect(sheetIdFor(undefined, 'c1', '2026-10-15T09:00:00Z')).toBe('sheet-c1-2026-10-15')
    expect(sheetIdFor({ id: 'existing', class_id: 'c1', date: '2026-10-15', status: 'draft' }, 'c1', '2026-10-15')).toBe('existing')
  })
})

import { buildAbsenceList, renderTemplate } from './phone'

describe('absence alerts (iPhone)', () => {
  const base = {
    classId: 'c1', date: '2026-10-15', cls: { id: 'c1', display: 'V-A' },
    students, enrollments,
    links: [{ id: 'l1', student_id: 's1', guardian_id: 'g1', is_primary: 1 }],
    guardians: [{ id: 'g1', name: 'Parent', mobile: '9820011111', language: 'hi' }],
    consents: [{ id: 'k1', student_id: 's1', purpose: 'messages', withdrawn_at: null }],
    templates: [
      { id: 't1', key: 'absence_alert', language: 'en', body: '{student_name} was absent on {date}. — {school_name}' },
      { id: 't2', key: 'absence_alert', language: 'hi', body: '{student_name} {date} को अनुपस्थित — {school_name}' },
    ],
    schoolName: 'Saraswati',
  }
  const marks = [
    { id: 'm1', sheet_id: 'sh1', student_id: 's1', mark: 'A' },
    { id: 'm2', sheet_id: 'sh1', student_id: 's2', mark: 'A' },
    { id: 'm3', sheet_id: 'sh1', student_id: 's3', mark: 'P' },
  ]
  it('is empty until the sheet is submitted', () => {
    const r = buildAbsenceList({ ...base, sheet: { id: 'sh1', class_id: 'c1', date: '2026-10-15', status: 'draft' }, marks })
    expect(r.submitted).toBe(false)
    expect(r.students).toEqual([])
  })
  it('lists absentees with guardian, consent and a preview in the guardian language', () => {
    const r = buildAbsenceList({ ...base, sheet: { id: 'sh1', class_id: 'c1', date: '2026-10-15', status: 'submitted' }, marks })
    expect(r.students.map((s) => s.student_id)).toEqual(['s2', 's1']) // roll order
    const asha = r.students.find((s) => s.student_id === 's1')!
    expect(asha.has_messages_consent).toBe(true)
    expect(asha.preview).toBe('Asha 2026-10-15 को अनुपस्थित — Saraswati')
    const bilal = r.students.find((s) => s.student_id === 's2')!
    expect(bilal.has_messages_consent).toBe(false)
    expect(bilal.guardian_id).toBeNull()
  })
  it('renders every {placeholder}', () => {
    expect(renderTemplate('{a}-{a}-{b}', { a: '1', b: '2' })).toBe('1-1-2')
  })
})
