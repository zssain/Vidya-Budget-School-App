import { describe, expect, it } from 'vitest'
import {
  buildExams,
  buildRequests,
  buildStudents,
  buildSyncStatus,
  buildTeacherDashboard,
  cancelCheck,
  cancelPayload,
  grantedClasses,
} from './teacher'
import { isAvailableOnWeb } from './backend'

const classes = [
  { id: 'c5a', display: 'V-A', class_teacher_id: 't1', sort_order: 5 },
  { id: 'c7b', display: 'VII-B', class_teacher_id: 't1', sort_order: 6 },
  { id: 'c1a', display: 'I-A', class_teacher_id: 't2', sort_order: 1 },
]
const enrollments = [
  { id: 'e1', student_id: 's1', class_id: 'c5a', roll_no: 2, from_date: '2026-04-01', to_date: null },
  { id: 'e2', student_id: 's2', class_id: 'c5a', roll_no: 1, from_date: '2026-04-01', to_date: null },
  { id: 'e3', student_id: 's3', class_id: 'c5a', roll_no: 3, from_date: '2026-04-01', to_date: '2026-06-30' }, // left
  { id: 'e4', student_id: 's4', class_id: 'c1a', roll_no: 1, from_date: '2026-04-01', to_date: null },
  { id: 'e5', student_id: 's5', class_id: 'c5a', roll_no: 4, from_date: '2026-10-01', to_date: null }, // joins later
]

describe('dashboard_teacher (web mirror of dashboard_teacher_logic)', () => {
  const base = { classes, classSubjects: [
    { id: 'cs-5a-eng', class_id: 'c5a', subject_id: 'eng', teacher_id: 't1' },
    { id: 'cs-1a-eng', class_id: 'c1a', subject_id: 'eng', teacher_id: 't1' },
    { id: 'cs-1a-sci', class_id: 'c1a', subject_id: 'sci', teacher_id: 't2' },
  ], enrollments, requests: [], substitutions: [] }

  it('lists own classes in order with status and the roster enrolled today', () => {
    const d = buildTeacherDashboard({ ...base, staffId: 't1', role: 'teacher', today: '2026-09-23', sheets: [
      { id: 'sh1', class_id: 'c7b', date: '2026-09-23', status: 'draft' },
      { id: 'sh0', class_id: 'c5a', date: '2026-09-22', status: 'submitted' }, // yesterday
    ] })
    expect(d.classes.map((c) => [c.class_id, c.status, c.student_count, c.duty])).toEqual([
      ['c5a', 'not_started', 2, false],
      ['c7b', 'draft', 0, false],
    ])
    expect(d.class_teacher_of).toEqual(['c5a', 'c7b'])
    expect(d.class_subject_ids).toEqual(['cs-1a-eng', 'cs-5a-eng'])
  })

  it('adds a duty class for today from a substitution or an approved duty request', () => {
    const subs = [
      { id: 'sub1', date: '2026-09-23', substitute_teacher_id: 't1', class_id: 'c1a', includes_attendance: 1 },
      { id: 'sub2', date: '2026-09-24', substitute_teacher_id: 't1', class_id: 'c9z', includes_attendance: 1 },
      { id: 'sub3', date: '2026-09-23', substitute_teacher_id: 't1', class_id: 'c9z', includes_attendance: 0 }, // period cover only
    ]
    const d = buildTeacherDashboard({ ...base, substitutions: subs, staffId: 't1', role: 'teacher', today: '2026-09-23', sheets: [
      { id: 'sh2', class_id: 'c1a', date: '2026-09-23T00:00:00', status: 'submitted' },
    ] })
    expect(d.classes.map((c) => [c.class_id, c.duty, c.submitted_today])).toEqual([
      ['c5a', false, false],
      ['c7b', false, false],
      ['c1a', true, true],
    ])
    expect(d.class_teacher_of).not.toContain('c1a')
    // Not on another day.
    const other = buildTeacherDashboard({ ...base, substitutions: subs, staffId: 't1', role: 'teacher', today: '2026-09-25', sheets: [] })
    expect(other.classes.some((c) => c.duty)).toBe(false)
  })

  it('grants from attendance-duty requests only when approved, own and in range', () => {
    const req = (over: object) => ({ id: 'r', type: 'attendance_duty', requested_by: 't1', status: 'approved', after_json: JSON.stringify({ class_id: 'c1a', from_date: '2026-09-20', to_date: '2026-09-25' }), ...over })
    expect([...grantedClasses('t1', '2026-09-23', [], [req({})])]).toEqual(['c1a'])
    expect(grantedClasses('t1', '2026-09-26', [], [req({})]).size).toBe(0)
    expect(grantedClasses('t1', '2026-09-23', [], [req({ status: 'pending' })]).size).toBe(0)
    expect(grantedClasses('t1', '2026-09-23', [], [req({ requested_by: 't2' })]).size).toBe(0)
    expect(grantedClasses('t1', '2026-09-23', [], [req({ type: 'leave' })]).size).toBe(0)
    expect(grantedClasses('t1', '2026-09-23', [], [req({ after_json: '{bad' })]).size).toBe(0)
  })
})

describe('list_exams (web)', () => {
  it('orders exams newest first and joins each subject to its class + subject', () => {
    const out = buildExams({
      exams: [
        { id: 'x1', name: 'Unit test 1', term_id: 'term1', starts_on: '2026-07-01' },
        { id: 'x2', name: 'Half-yearly', term_id: null, starts_on: '2026-10-01' },
      ],
      examSubjects: [
        { id: 'es1', exam_id: 'x1', class_subject_id: 'cs-5a-sci', max_marks: 25 },
        { id: 'es2', exam_id: 'x1', class_subject_id: 'cs-5a-eng', max_marks: 25 },
        { id: 'es3', exam_id: 'x1', class_subject_id: 'cs-unknown', max_marks: 25 },
      ],
      classSubjects: [
        { id: 'cs-5a-sci', class_id: 'c5a', subject_id: 'sci' },
        { id: 'cs-5a-eng', class_id: 'c5a', subject_id: 'eng' },
      ],
      classes,
      subjects: [{ id: 'sci', name: 'Science' }, { id: 'eng', name: 'English' }],
      terms: [{ id: 'term1', name: 'Term 1' }],
      sheets: [{ id: 'm1', exam_subject_id: 'es1', status: 'submitted' }],
    })
    expect(out.map((e) => e.id)).toEqual(['x2', 'x1'])
    expect(out[1].term_name).toBe('Term 1')
    expect(out[1].subjects.map((s) => [s.id, s.subject_name, s.class_display, s.status])).toEqual([
      ['es2', 'English', 'V-A', 'not_started'],
      ['es1', 'Science', 'V-A', 'submitted'],
    ])
    expect(Object.keys(out[1].subjects[0]).sort()).toEqual(['class_display', 'class_id', 'class_subject_id', 'id', 'max_marks', 'status', 'subject_name'])
  })
})

describe('list_students (web)', () => {
  const students = [
    { id: 's1', name: 'Bilal', admission_no: 'A1' },
    { id: 's2', name: 'Asha' },
    { id: 's3', name: 'Chitra' },
    { id: 's4', name: 'Dev', status: 'left' },
  ]
  it('lists active students by name with their current class + roll', () => {
    const out = buildStudents({ students, enrollments, classes })
    expect(out.map((s) => [s.name, s.class_display, s.roll_no])).toEqual([
      ['Asha', 'V-A', 1],
      ['Bilal', 'V-A', 2],
      ['Chitra', null, null],
    ])
    expect(out[1].admission_no).toBe('A1')
  })
  it('one class by roll number', () => {
    expect(buildStudents({ classId: 'c5a', students, enrollments, classes }).map((s) => s.id)).toEqual(['s2', 's1'])
  })
})

describe('list_requests / cancel_request (web)', () => {
  const requests = [
    { id: 'r1', type: 'leave', requested_by: 't1', status: 'pending', reason: 'Wedding', created_at: '2026-09-20T10:00:00Z' },
    { id: 'r2', type: 'attendance_correction', requested_by: 't1', status: 'returned', reason: 'Fix', created_at: '2026-09-22T10:00:00Z', note: 'Which day?' },
    { id: 'r3', type: 'leave', requested_by: 't2', status: 'pending', reason: 'x', created_at: '2026-09-21T10:00:00Z' },
  ]
  const staffList = [{ id: 't1', name: 'Meena Iyer' }, { id: 't2', name: 'Anita Rao' }]
  it("a teacher sees only their own, newest first, as RequestDto", () => {
    const out = buildRequests({ staff: { id: 't1', role: 'teacher', name: 'Meena Iyer' }, requests, staffList })
    expect(out.map((r) => r.id)).toEqual(['r2', 'r1'])
    expect(out[0]).toMatchObject({ kind: 'attendance_correction', status: 'returned', note: 'Which day?', requester_name: 'Meena Iyer', apply_state: 'not_applied' })
    expect(buildRequests({ staff: { id: 't1', role: 'teacher', name: 'M' }, requests, staffList, status: 'pending' }).map((r) => r.id)).toEqual(['r1'])
    expect(buildRequests({ staff: { id: 'p', role: 'principal', name: 'P' }, requests, staffList })).toHaveLength(3)
  })
  it('a local (not yet synced) leave row keeps its kind', () => {
    const out = buildRequests({ staff: { id: 't1', role: 'teacher', name: 'M' }, requests: [{ id: 'r9', kind: 'leave', type: 'leave', requested_by: 't1', status: 'pending', created_at: '2026-09-23' }], staffList: [] })
    expect(out[0].kind).toBe('leave')
    expect(out[0].requester_name).toBe('M')
  })
  it('cancel: own pending/returned only (cancel_request_in)', () => {
    expect(cancelCheck('t1', requests[0])).toBeNull()
    expect(cancelCheck('t1', requests[1])).toBeNull()
    expect(cancelCheck('t1', requests[2])).toMatchObject({ code: 'FORBIDDEN', vars: { reason: 'not_requester' } })
    expect(cancelCheck('t1', { ...requests[0], status: 'approved' })).toMatchObject({ code: 'VALIDATION', vars: { rule: 'not_cancellable' } })
    expect(cancelCheck('t1', undefined)).toMatchObject({ code: 'NOT_FOUND' })
    // Same JSON the school PC deserialises as RequestCmd::Cancel.
    expect(cancelPayload('r1')).toEqual({ cmd: 'cancel', id: 'r1' })
  })
})

describe('sync_status (web)', () => {
  it('reports the outbox and the last completed pass, never a fake time', () => {
    expect(buildSyncStatus(3, null)).toEqual({ pending_count: 3, pending_paise: 0, last_confirmed_at: null })
    expect(buildSyncStatus(0, Date.UTC(2026, 9, 31, 8, 0, 0)).last_confirmed_at).toBe('2026-10-31T08:00:00.000Z')
  })
})

describe('teacher-home commands are served on web', () => {
  it('registers the tile commands', () => {
    for (const c of ['dashboard_teacher', 'list_exams', 'list_students', 'list_requests', 'cancel_request', 'sync_status', 'sync_now', 'list_circulars', 'mark_circular_read', 'get_marks_sheet', 'get_report_remarks', 'lock']) {
      expect(isAvailableOnWeb(c)).toBe(true)
    }
  })
})
