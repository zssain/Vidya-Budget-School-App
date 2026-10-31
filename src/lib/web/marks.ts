// Marks entry + report-card remarks for the iPhone app (P21, repair P1). Reads come
// from the school PC's down export (the local encrypted stores); writes are queued as
// the SAME ops the native client sends, and the school PC replays them as this staff
// member through the same domain logic (src-tauri/src/sync/apply.rs):
//   marks   → `marks_sheet`/action  {exam_subject_id, submit, entries[]}
//             record id = the sheet's id, else `msheet-<exam_subject_id>`
//             (commands::logic::upsert_marks → apply_marks_action)
//   remarks → `report_remark`/action ClientCmd {cmd:'save_report_remark', input}
//             record id = the remark's id, else `rmk-<exam_id>-<student_id>`
//             (commands::logic::save_report_remark_logic → apply_client_cmd)
// The local permission/validation checks mirror the native command exactly (the
// school PC re-checks everything on replay; a refusal comes back in the ack and is
// shown to the user via list_sync_problems).
//
// The pure builders/checks are unit-tested (marks.test.ts); the shells only read/
// write the local store.

import type {
  CmdError,
  MarkEntryInput,
  MarksSheetDto,
  ReportRemarkInput,
  ReportRemarksDto,
  ReportTemplateDto,
  SessionStaff,
} from '../api'
import { kvGet, listRecords } from './store'
import { writeOp } from './op'

export interface ExamSubjectRec { id: string; exam_id: string; class_subject_id: string; max_marks: number }
export interface ClassSubjectRec { id: string; class_id: string; subject_id: string; teacher_id?: string | null }
export interface SubjectRec { id: string; name: string }
export interface ClassRec { id: string; display?: string | null; class_teacher_id?: string | null }
export interface EnrollmentRec { id: string; student_id: string; class_id: string; roll_no?: number | null; to_date?: string | null }
export interface StudentRec { id: string; name: string }
export interface MarksSheetRec { id: string; exam_subject_id: string; status: string; local_entries?: MarkEntryInput[] }
export interface MarkEntryRec { id: string; sheet_id: string; student_id: string; marks?: number | null; absent?: number | boolean | null }
export interface RemarkRec { id: string; exam_id: string; student_id: string; text: string; template_key?: string | null }
export interface TemplateRec { id: string; key: string; language: string; text: string }
export interface SchoolRec { id: string; settings_json?: string | null }

/** vidya_core::report::MAX_REMARK_CHARS */
export const MAX_REMARK_CHARS = 300

const forbidden = (reason: string): CmdError => ({ code: 'FORBIDDEN', message_key: 'error.FORBIDDEN', vars: { reason } })
const validation = (field: string, rule: string): CmdError => ({ code: 'VALIDATION', message_key: 'error.VALIDATION', vars: { field, rule } })
const notFound = (): CmdError => ({ code: 'NOT_FOUND', message_key: 'error.NOT_FOUND', vars: {} })
const truthy = (v: unknown): boolean => v === true || v === 1 || v === '1'

// ---- permission + validation (pure, mirror vidya-core) --------------------------

/** vidya_core::marks::can_edit — the Principal always; a teacher only the subject
 *  they teach in that class (class_subject.teacher_id); never an accountant. */
export function marksPermission(staff: Pick<SessionStaff, 'id' | 'role'>, cs: ClassSubjectRec): CmdError | null {
  if (staff.role === 'principal') return null
  if (staff.role === 'teacher') return cs.teacher_id === staff.id ? null : forbidden('not_own_class_subject')
  return forbidden('role')
}

/** permissions::can(EnterReportRemark) — the Principal always; a teacher only as the
 *  class teacher of the student's class; never an accountant. */
export function remarkPermission(staff: Pick<SessionStaff, 'id' | 'role'>, cls: ClassRec | undefined): CmdError | null {
  if (staff.role === 'principal') return null
  if (staff.role === 'teacher') return cls?.class_teacher_id === staff.id ? null : forbidden('teacher_not_class_teacher')
  return forbidden('accountant_no_academic')
}

/** The whole-sheet validation upsert_marks runs before any write: each student once,
 *  currently enrolled in the class, and every entry 0..=max (absent ⇒ no marks). */
export function validateMarksEntries(entries: MarkEntryInput[], enrolled: Set<string>, maxMarks: number): CmdError | null {
  const seen = new Set<string>()
  for (const e of entries) {
    if (seen.has(e.student_id)) return validation('entries', 'duplicate_student')
    seen.add(e.student_id)
    if (!enrolled.has(e.student_id)) return validation('entries', 'student_not_in_class')
    if (e.absent && e.marks != null) return validation('marks', 'absent_and_marks')
    if (e.marks != null && (!Number.isInteger(e.marks) || e.marks < 0 || e.marks > maxMarks)) return validation('marks', 'range')
  }
  return null
}

/** vidya_core::report::validate_remark (trimmed, non-empty, ≤ 300 characters). */
export function validateRemark(text: string): CmdError | null {
  const t = text.trim()
  if (t.length === 0) return validation('remark', 'empty')
  if ([...t].length > MAX_REMARK_CHARS) return validation('remark', 'too_long')
  return null
}

// ---- op builders (pure) -------------------------------------------------------

/** The marks_sheet action payload (upsert_marks `json!({exam_subject_id, submit, entries})`). */
export function marksPayload(examSubjectId: string, entries: MarkEntryInput[], submit: boolean): Record<string, unknown> {
  return {
    exam_subject_id: examSubjectId,
    submit,
    entries: entries.map((e) => ({ student_id: e.student_id, marks: e.marks ?? null, absent: !!e.absent })),
  }
}

/** Sheet id: the existing one for the exam subject, else the deterministic id. */
export function marksSheetIdFor(existing: MarksSheetRec | undefined, examSubjectId: string): string {
  return existing?.id ?? `msheet-${examSubjectId}`
}

/** The report_remark ClientCmd payload (serde `#[serde(tag = "cmd")]`). */
export function remarkPayload(input: ReportRemarkInput): Record<string, unknown> {
  return {
    cmd: 'save_report_remark',
    input: { exam_id: input.exam_id, student_id: input.student_id, text: input.text, template_key: input.template_key ?? null },
  }
}

/** Remark id: the existing one for (exam, student), else the deterministic id. */
export function remarkIdFor(existing: RemarkRec | undefined, examId: string, studentId: string): string {
  return existing?.id ?? `rmk-${examId}-${studentId}`
}

// ---- read models (pure) -------------------------------------------------------

function currentRoster(enrollments: EnrollmentRec[], classId: string, names: Map<string, string>): EnrollmentRec[] {
  return enrollments
    .filter((e) => e.class_id === classId && !e.to_date)
    .sort((a, b) => (a.roll_no ?? 1e9) - (b.roll_no ?? 1e9) || (names.get(a.student_id) ?? '').localeCompare(names.get(b.student_id) ?? ''))
}

/** get_marks_sheet_logic: the class roster with each student's entry — a pending
 *  local save in preference, else the school PC's canonical entries. */
export function buildMarksSheet(p: {
  examSubject: ExamSubjectRec
  cs: ClassSubjectRec
  subject: SubjectRec | undefined
  cls: ClassRec | undefined
  enrollments: EnrollmentRec[]
  students: StudentRec[]
  sheet: MarksSheetRec | undefined
  entries: MarkEntryRec[]
}): MarksSheetDto {
  const names = new Map(p.students.map((s) => [s.id, s.name]))
  const entryOf = new Map<string, { marks: number | null; absent: boolean }>()
  if (p.sheet?.local_entries) {
    for (const e of p.sheet.local_entries) entryOf.set(e.student_id, { marks: e.marks ?? null, absent: !!e.absent })
  } else if (p.sheet) {
    for (const e of p.entries) if (e.sheet_id === p.sheet.id) entryOf.set(e.student_id, { marks: e.marks ?? null, absent: truthy(e.absent) })
  }
  return {
    exam_subject_id: p.examSubject.id,
    class_id: p.cs.class_id,
    class_display: p.cls?.display ?? null,
    subject_name: p.subject?.name ?? '',
    max_marks: p.examSubject.max_marks,
    status: p.sheet?.status ?? 'not_started',
    rows: currentRoster(p.enrollments, p.cs.class_id, names).map((e) => ({
      student_id: e.student_id,
      name: names.get(e.student_id) ?? '',
      roll_no: e.roll_no ?? null,
      marks: entryOf.get(e.student_id)?.marks ?? null,
      absent: entryOf.get(e.student_id)?.absent ?? false,
    })),
  }
}

/** The school's report-card language (settings_json.language), else English. */
export function schoolLanguage(school: SchoolRec | undefined): string {
  try {
    const lang = school?.settings_json ? (JSON.parse(school.settings_json) as { language?: unknown }).language : undefined
    return typeof lang === 'string' && lang ? lang : 'en'
  } catch {
    return 'en'
  }
}

/** load_report_templates: every English key, in `language` when translated, by key. */
export function buildTemplates(templates: TemplateRec[], language: string): ReportTemplateDto[] {
  return templates
    .filter((t) => t.language === 'en')
    .map((t) => ({ key: t.key, text: templates.find((x) => x.key === t.key && x.language === language)?.text ?? t.text }))
    .sort((a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : 0))
}

/** get_report_remarks_logic. `locked` is false: report_lock is a school-PC table that
 *  is not exported to devices (the native client sees the same) — the school PC
 *  refuses a teacher's remark on a finalised exam on replay (SHEET_LOCKED). */
export function buildReportRemarks(p: {
  examId: string
  classId: string
  cls: ClassRec | undefined
  enrollments: EnrollmentRec[]
  students: StudentRec[]
  remarks: RemarkRec[]
  templates: TemplateRec[]
  language: string
}): ReportRemarksDto {
  const names = new Map(p.students.map((s) => [s.id, s.name]))
  return {
    exam_id: p.examId,
    class_id: p.classId,
    class_display: p.cls?.display ?? null,
    locked: false,
    students: currentRoster(p.enrollments, p.classId, names).map((e) => {
      const r = p.remarks.find((x) => x.exam_id === p.examId && x.student_id === e.student_id)
      return { student_id: e.student_id, name: names.get(e.student_id) ?? '', roll_no: e.roll_no ?? null, remark: r?.text ?? null, template_key: r?.template_key ?? null }
    }),
    templates: buildTemplates(p.templates, p.language),
  }
}

// ---- shells (local store) -----------------------------------------------------

async function me(): Promise<SessionStaff> {
  const staff = await kvGet<SessionStaff>('staff')
  if (!staff) throw { code: 'LOCKED', message_key: 'error.LOCKED', vars: {} } satisfies CmdError
  return staff
}

async function examSubjectMeta(examSubjectId: string): Promise<{ es: ExamSubjectRec; cs: ClassSubjectRec }> {
  const [es] = await listRecords<ExamSubjectRec>('exam_subject', (r) => r.id === examSubjectId)
  if (!es) throw notFound()
  const [cs] = await listRecords<ClassSubjectRec>('class_subject', (r) => r.id === es.class_subject_id)
  if (!cs) throw notFound()
  return { es, cs }
}

export async function getMarksSheet(examSubjectId: string): Promise<MarksSheetDto> {
  const staff = await me()
  const { es, cs } = await examSubjectMeta(examSubjectId)
  const denied = marksPermission(staff, cs)
  if (denied) throw denied
  const [subjects, classes, enrollments, students, sheets] = await Promise.all([
    listRecords<SubjectRec>('subject', (s) => s.id === cs.subject_id),
    listRecords<ClassRec>('class', (c) => c.id === cs.class_id),
    listRecords<EnrollmentRec>('enrollment', (e) => e.class_id === cs.class_id),
    listRecords<StudentRec>('student'),
    listRecords<MarksSheetRec>('marks_sheet', (s) => s.exam_subject_id === examSubjectId),
  ])
  const sheet = sheets[0]
  const entries = sheet ? await listRecords<MarkEntryRec>('mark_entry', (m) => m.sheet_id === sheet.id) : []
  return buildMarksSheet({ examSubject: es, cs, subject: subjects[0], cls: classes[0], enrollments, students, sheet, entries })
}

/** save_marks_draft / submit_marks: one atomic marks_sheet action op with the sheet. */
export async function writeMarks(examSubjectId: string, entries: MarkEntryInput[], submit: boolean): Promise<void> {
  const staff = await me()
  const { es, cs } = await examSubjectMeta(examSubjectId)
  const denied = marksPermission(staff, cs)
  if (denied) throw denied
  const [existing] = await listRecords<MarksSheetRec>('marks_sheet', (s) => s.exam_subject_id === examSubjectId)
  // A submitted sheet is locked (per subject) — corrections go through a request.
  if (existing?.status === 'submitted') throw { code: 'LOCKED', message_key: 'error.LOCKED', vars: {} } satisfies CmdError
  const enrolled = new Set(
    (await listRecords<EnrollmentRec>('enrollment', (e) => e.class_id === cs.class_id && !e.to_date)).map((e) => e.student_id),
  )
  const invalid = validateMarksEntries(entries, enrolled, es.max_marks)
  if (invalid) throw invalid
  const id = marksSheetIdFor(existing, examSubjectId)
  const payload = marksPayload(examSubjectId, entries, submit)
  await writeOp({
    table: 'marks_sheet',
    recordId: id,
    kind: 'action',
    audience: `class:${cs.class_id}`,
    payload,
    localRow: { id, exam_subject_id: examSubjectId, status: submit ? 'submitted' : 'draft', local_entries: payload.entries, sync_state: 'on_device' },
    action: submit ? 'submit_marks' : 'save_marks_draft',
  })
}

export async function getReportRemarks(examId: string, classId: string): Promise<ReportRemarksDto> {
  const staff = await me()
  const [cls] = await listRecords<ClassRec>('class', (c) => c.id === classId)
  const denied = remarkPermission(staff, cls)
  if (denied) throw denied
  const [enrollments, students, remarks, templates, schools] = await Promise.all([
    listRecords<EnrollmentRec>('enrollment', (e) => e.class_id === classId),
    listRecords<StudentRec>('student'),
    listRecords<RemarkRec>('report_remark', (r) => r.exam_id === examId),
    listRecords<TemplateRec>('report_template'),
    listRecords<SchoolRec>('school'),
  ])
  return buildReportRemarks({ examId, classId, cls, enrollments, students, remarks, templates, language: schoolLanguage(schools[0]) })
}

/** save_report_remark: one report_remark ClientCmd op (the school PC replays it). */
export async function saveReportRemark(input: ReportRemarkInput): Promise<void> {
  const staff = await me()
  // The student's current class decides ownership.
  const [enr] = await listRecords<EnrollmentRec>('enrollment', (e) => e.student_id === input.student_id && !e.to_date)
  if (!enr) throw notFound()
  const [cls] = await listRecords<ClassRec>('class', (c) => c.id === enr.class_id)
  const denied = remarkPermission(staff, cls ?? { id: enr.class_id })
  if (denied) throw denied
  const invalid = validateRemark(input.text)
  if (invalid) throw invalid
  const [existing] = await listRecords<RemarkRec>('report_remark', (r) => r.exam_id === input.exam_id && r.student_id === input.student_id)
  const id = remarkIdFor(existing, input.exam_id, input.student_id)
  await writeOp({
    table: 'report_remark',
    recordId: id,
    kind: 'action',
    audience: `class:${enr.class_id}`,
    payload: remarkPayload(input),
    localRow: {
      ...(existing ?? {}),
      id,
      exam_id: input.exam_id,
      student_id: input.student_id,
      text: input.text.trim(),
      template_key: input.template_key ?? null,
      author: staff.id,
      sync_state: 'on_device',
    },
    action: 'save_report_remark',
  })
}
