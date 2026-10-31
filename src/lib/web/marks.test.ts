import { describe, expect, it } from 'vitest'
import {
  buildMarksSheet,
  buildReportRemarks,
  buildTemplates,
  marksPayload,
  marksPermission,
  marksSheetIdFor,
  remarkIdFor,
  remarkPayload,
  remarkPermission,
  schoolLanguage,
  validateMarksEntries,
  validateRemark,
} from './marks'
import { isAvailableOnWeb } from './backend'
import { hideUnavailable, navForRole } from '../nav'

const principal = { id: 'p1', role: 'principal' }
const teacher = { id: 't1', role: 'teacher' }
const other = { id: 't2', role: 'teacher' }
const accountant = { id: 'a1', role: 'accountant' }
const cs = { id: 'cs1', class_id: 'c1', subject_id: 'sub1', teacher_id: 't1' }

const enrollments = [
  { id: 'e1', student_id: 's1', class_id: 'c1', roll_no: 2, to_date: null },
  { id: 'e2', student_id: 's2', class_id: 'c1', roll_no: 1, to_date: null },
  { id: 'e3', student_id: 's3', class_id: 'c1', roll_no: 3, to_date: '2026-06-30' }, // left
  { id: 'e4', student_id: 's4', class_id: 'c2', roll_no: 1, to_date: null },
]
const students = [
  { id: 's1', name: 'Asha' },
  { id: 's2', name: 'Bilal' },
  { id: 's3', name: 'Chitra' },
  { id: 's4', name: 'Dev' },
]

describe('marks permission (vidya_core::marks::can_edit)', () => {
  it('lets the Principal and the subject teacher in; refuses others', () => {
    expect(marksPermission(principal, cs)).toBeNull()
    expect(marksPermission(teacher, cs)).toBeNull()
    expect(marksPermission(other, cs)).toMatchObject({ code: 'FORBIDDEN', vars: { reason: 'not_own_class_subject' } })
    expect(marksPermission(accountant, cs)).toMatchObject({ code: 'FORBIDDEN', vars: { reason: 'role' } })
  })
  it('a class teacher who does not teach the subject is refused (same as native)', () => {
    expect(marksPermission({ id: 'ct', role: 'teacher' }, cs)).toMatchObject({ code: 'FORBIDDEN' })
  })
})

describe('marks validation (upsert_marks)', () => {
  const enrolled = new Set(['s1', 's2'])
  it('accepts a valid sheet', () => {
    expect(validateMarksEntries([{ student_id: 's1', marks: 40, absent: false }, { student_id: 's2', marks: null, absent: true }], enrolled, 50)).toBeNull()
  })
  it('refuses duplicates, strangers, absent+marks and out-of-range marks', () => {
    expect(validateMarksEntries([{ student_id: 's1', marks: 1, absent: false }, { student_id: 's1', marks: 2, absent: false }], enrolled, 50)?.vars).toEqual({ field: 'entries', rule: 'duplicate_student' })
    expect(validateMarksEntries([{ student_id: 's3', marks: 1, absent: false }], enrolled, 50)?.vars).toEqual({ field: 'entries', rule: 'student_not_in_class' })
    expect(validateMarksEntries([{ student_id: 's1', marks: 1, absent: true }], enrolled, 50)?.vars).toEqual({ field: 'marks', rule: 'absent_and_marks' })
    expect(validateMarksEntries([{ student_id: 's1', marks: 51, absent: false }], enrolled, 50)?.vars).toEqual({ field: 'marks', rule: 'range' })
    expect(validateMarksEntries([{ student_id: 's1', marks: -1, absent: false }], enrolled, 50)?.vars).toEqual({ field: 'marks', rule: 'range' })
  })
})

describe('marks op (matches the school PC replay, apply_marks_action)', () => {
  it('is ONE action payload with the whole sheet', () => {
    expect(marksPayload('es1', [{ student_id: 's1', marks: 40, absent: false }, { student_id: 's2', marks: null, absent: true }], true)).toEqual({
      exam_subject_id: 'es1',
      submit: true,
      entries: [
        { student_id: 's1', marks: 40, absent: false },
        { student_id: 's2', marks: null, absent: true },
      ],
    })
  })
  it('uses the deterministic msheet-<exam_subject> id for a new sheet', () => {
    expect(marksSheetIdFor(undefined, 'es1')).toBe('msheet-es1')
    expect(marksSheetIdFor({ id: 'existing', exam_subject_id: 'es1', status: 'draft' }, 'es1')).toBe('existing')
  })
})

describe('buildMarksSheet (get_marks_sheet)', () => {
  const base = {
    examSubject: { id: 'es1', exam_id: 'x1', class_subject_id: 'cs1', max_marks: 50 },
    cs,
    subject: { id: 'sub1', name: 'Maths' },
    cls: { id: 'c1', display: 'V-A' },
    enrollments,
    students,
  }
  it('lists the current roster by roll number, not_started without a sheet', () => {
    const dto = buildMarksSheet({ ...base, sheet: undefined, entries: [] })
    expect(dto).toMatchObject({ class_id: 'c1', class_display: 'V-A', subject_name: 'Maths', max_marks: 50, status: 'not_started' })
    expect(dto.rows.map((r) => r.name)).toEqual(['Bilal', 'Asha'])
  })
  it('shows canonical entries, or a pending local save in preference', () => {
    const sheet = { id: 'msheet-es1', exam_subject_id: 'es1', status: 'draft' }
    const entries = [{ id: 'me1', sheet_id: 'msheet-es1', student_id: 's1', marks: 33, absent: 0 }, { id: 'me2', sheet_id: 'msheet-es1', student_id: 's2', marks: null, absent: 1 }]
    const canon = buildMarksSheet({ ...base, sheet, entries })
    expect(canon.rows.find((r) => r.student_id === 's1')).toMatchObject({ marks: 33, absent: false })
    expect(canon.rows.find((r) => r.student_id === 's2')).toMatchObject({ marks: null, absent: true })
    const local = buildMarksSheet({ ...base, entries, sheet: { ...sheet, status: 'submitted', local_entries: [{ student_id: 's1', marks: 45, absent: false }] } })
    expect(local.status).toBe('submitted')
    expect(local.rows.find((r) => r.student_id === 's1')?.marks).toBe(45)
  })
})

describe('report remarks (save_report_remark_logic)', () => {
  const cls = { id: 'c1', display: 'V-A', class_teacher_id: 't1' }
  it('class teacher or Principal only', () => {
    expect(remarkPermission(principal, cls)).toBeNull()
    expect(remarkPermission(teacher, cls)).toBeNull()
    expect(remarkPermission(other, cls)).toMatchObject({ code: 'FORBIDDEN', vars: { reason: 'teacher_not_class_teacher' } })
    expect(remarkPermission(accountant, cls)).toMatchObject({ code: 'FORBIDDEN', vars: { reason: 'accountant_no_academic' } })
  })
  it('validates the remark like vidya_core::report::validate_remark', () => {
    expect(validateRemark('  Good work  ')).toBeNull()
    expect(validateRemark('   ')?.vars).toEqual({ field: 'remark', rule: 'empty' })
    expect(validateRemark('x'.repeat(300))).toBeNull()
    expect(validateRemark('x'.repeat(301))?.vars).toEqual({ field: 'remark', rule: 'too_long' })
    expect(validateRemark('अ'.repeat(300))).toBeNull() // counts characters, not bytes
  })
  it('emits the ClientCmd JSON the school PC deserialises (serde tag = "cmd")', () => {
    expect(remarkPayload({ exam_id: 'x1', student_id: 's1', text: 'Good', template_key: 'k1' })).toEqual({
      cmd: 'save_report_remark',
      input: { exam_id: 'x1', student_id: 's1', text: 'Good', template_key: 'k1' },
    })
    expect((remarkPayload({ exam_id: 'x1', student_id: 's1', text: 'Good' }).input as Record<string, unknown>).template_key).toBeNull()
  })
  it('uses the deterministic rmk-<exam>-<student> id', () => {
    expect(remarkIdFor(undefined, 'x1', 's1')).toBe('rmk-x1-s1')
    expect(remarkIdFor({ id: 'old', exam_id: 'x1', student_id: 's1', text: 'a' }, 'x1', 's1')).toBe('old')
  })
  it('builds the remarks list with templates in the school language', () => {
    const templates = [
      { id: 'a', key: 'b_good', language: 'en', text: 'Good' },
      { id: 'b', key: 'a_fair', language: 'en', text: 'Fair' },
      { id: 'c', key: 'b_good', language: 'hi', text: 'अच्छा' },
    ]
    expect(buildTemplates(templates, 'hi')).toEqual([{ key: 'a_fair', text: 'Fair' }, { key: 'b_good', text: 'अच्छा' }])
    expect(schoolLanguage({ id: 'sc', settings_json: '{"language":"hi"}' })).toBe('hi')
    expect(schoolLanguage({ id: 'sc', settings_json: 'not json' })).toBe('en')
    const dto = buildReportRemarks({
      examId: 'x1', classId: 'c1', cls, enrollments, students, templates, language: 'en',
      remarks: [{ id: 'rmk-x1-s1', exam_id: 'x1', student_id: 's1', text: 'Kind', template_key: null }, { id: 'r2', exam_id: 'x2', student_id: 's2', text: 'other exam' }],
    })
    expect(dto.locked).toBe(false)
    expect(dto.students.map((s) => [s.name, s.remark])).toEqual([['Bilal', null], ['Asha', 'Kind']])
  })
})

describe('web command coverage (P1)', () => {
  it('serves marks entry + remarks, still not collect-fee', () => {
    for (const c of ['get_marks_sheet', 'save_marks_draft', 'submit_marks', 'get_report_remarks', 'save_report_remark']) {
      expect(isAvailableOnWeb(c)).toBe(true)
    }
    expect(isAvailableOnWeb('record_payment')).toBe(false)
  })
  it('hides the accountant Collect-fee nav entry on web', () => {
    const keys = (av: (c: string) => boolean) => hideUnavailable(navForRole('accountant'), av).flatMap((s) => s.items.map((i) => i.key))
    expect(keys(isAvailableOnWeb)).not.toContain('collect')
    expect(keys(() => true)).toContain('collect')
  })
})
