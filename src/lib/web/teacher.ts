// Teacher-home commands for the iPhone app (repair T1). The live Teacher Home and its
// tiles (Marks, Report cards, Students, My requests, Sync) read the same commands as
// the Android app; here they are served from the records the school PC's down
// export put on this device (scoped like the native client, sync/scope.rs). Each
// builder mirrors its native counterpart:
//   dashboard_teacher → commands::logic::dashboard_teacher_logic (own classes, then
//                       today's substitution / approved attendance-duty grants —
//                       the attendance_write rule)
//   list_exams        → list_exams_logic + exam_subjects (newest first)
//   list_students     → list_students_logic, teacher-scoped (the device only holds
//                       the teacher's classes)
//   list_requests     → list_requests_logic filtered to the caller (non-Principal)
//   cancel_request    → RequestCmd::Cancel, replayed by the school PC as the author
//   sync_status       → the outbox + the last successful Drive pass
// The pure builders are unit-tested (teacher.test.ts); the shells only read the store.

import type {
  CmdError,
  ExamDto,
  RequestDto,
  SessionStaff,
  StudentDto,
  SyncStatusDto,
  TeacherClassDto,
  TeacherDashboard,
} from '../api'
import { kvGet, listRecords, pendingOps } from './store'
import { writeOp } from './op'
import { LAST_SYNC_OK_KEY } from './drive/sync'

export interface ClassRec { id: string; display: string; class_teacher_id?: string | null; sort_order?: number | null }
export interface ClassSubjectRec { id: string; class_id: string; subject_id: string; teacher_id?: string | null }
export interface SubjectRec { id: string; name: string }
export interface EnrollmentRec { id: string; student_id: string; class_id: string; roll_no?: number | null; from_date?: string | null; to_date?: string | null }
export interface StudentRec { id: string; name: string; admission_no?: string | null; provisional_no?: string | null; status?: string | null }
export interface SheetRec { id: string; class_id: string; date: string; status: string }
export interface SubstitutionRec { id: string; date: string; substitute_teacher_id: string; class_id: string; includes_attendance?: number | boolean | null }
export interface RequestRec {
  id: string
  type?: string
  kind?: string
  target_table?: string
  target_id?: string
  reason?: string
  status?: string
  apply_state?: string
  before_json?: string | null
  after_json?: string | null
  requested_by?: string
  requester_name?: string
  created_at?: string
  note?: string | null
}
export interface ExamRec { id: string; name: string; term_id?: string | null; starts_on?: string | null; ends_on?: string | null }
export interface ExamSubjectRec { id: string; exam_id: string; class_subject_id: string; max_marks: number }
export interface TermRec { id: string; name: string }
export interface MarksSheetRec { id: string; exam_subject_id: string; status: string }
export interface StaffRec { id: string; name: string }

const truthy = (v: unknown): boolean => v === true || v === 1 || v === '1'
const day10 = (d: string | null | undefined): string => (d ?? '').slice(0, 10)

/** Enrolled in the class on `day` (from_date ≤ day ≤ to_date, open-ended allowed). */
function enrolledOn(e: EnrollmentRec, classId: string, day: string): boolean {
  if (e.class_id !== classId) return false
  if (e.from_date && day10(e.from_date) > day) return false
  return !e.to_date || day10(e.to_date) >= day
}

/** vidya_core::permissions::granted_attendance_classes over the grants
 *  sync::apply::attendance_grants_for loads: today's substitutions with attendance,
 *  plus approved attendance-duty requests whose range covers today. */
export function grantedClasses(staffId: string, day: string, subs: SubstitutionRec[], requests: RequestRec[]): Set<string> {
  const out = new Set<string>()
  for (const s of subs) {
    if (s.substitute_teacher_id === staffId && day10(s.date) === day && truthy(s.includes_attendance)) out.add(s.class_id)
  }
  for (const r of requests) {
    if ((r.type ?? r.kind) !== 'attendance_duty' || r.requested_by !== staffId || r.status !== 'approved' || !r.after_json) continue
    try {
      const v = JSON.parse(r.after_json) as { class_id?: unknown; from_date?: unknown; to_date?: unknown }
      if (typeof v.class_id === 'string' && typeof v.from_date === 'string' && typeof v.to_date === 'string' && v.from_date <= day && day <= v.to_date) {
        out.add(v.class_id)
      }
    } catch {
      /* malformed after_json: no grant (same as native) */
    }
  }
  return out
}

/** dashboard_teacher_logic (pure). */
export function buildTeacherDashboard(p: {
  staffId: string
  role: string
  today: string
  classes: ClassRec[]
  classSubjects: ClassSubjectRec[]
  substitutions: SubstitutionRec[]
  requests: RequestRec[]
  enrollments: EnrollmentRec[]
  sheets: SheetRec[]
}): TeacherDashboard {
  const day = day10(p.today)
  const byId = new Map(p.classes.map((c) => [c.id, c]))
  const own = p.classes
    .filter((c) => c.class_teacher_id === p.staffId)
    .sort((a, b) => (a.sort_order ?? 0) - (b.sort_order ?? 0) || a.display.localeCompare(b.display))
  const wanted: { id: string; duty: boolean }[] = own.map((c) => ({ id: c.id, duty: false }))
  if (p.role === 'teacher') {
    for (const cid of [...grantedClasses(p.staffId, day, p.substitutions, p.requests)].sort()) {
      if (!wanted.some((w) => w.id === cid)) wanted.push({ id: cid, duty: true })
    }
  }
  const classes: TeacherClassDto[] = []
  for (const w of wanted) {
    const cls = byId.get(w.id)
    if (!cls) continue
    const sheet = p.sheets.find((s) => s.class_id === w.id && day10(s.date) === day)
    const status = sheet?.status ?? 'not_started'
    classes.push({
      class_id: w.id,
      display: cls.display,
      submitted_today: status === 'submitted',
      status,
      student_count: p.enrollments.filter((e) => enrolledOn(e, w.id, day)).length,
      duty: w.duty,
    })
  }
  return {
    classes,
    class_teacher_of: own.map((c) => c.id),
    class_subject_ids: p.classSubjects.filter((cs) => cs.teacher_id === p.staffId).map((cs) => cs.id).sort(),
  }
}

/** list_exams_logic + exam_subjects (pure): exams newest first, each with the exam
 *  subjects this device holds (the teacher's classes), ordered by class then subject. */
export function buildExams(p: {
  exams: ExamRec[]
  examSubjects: ExamSubjectRec[]
  classSubjects: ClassSubjectRec[]
  classes: ClassRec[]
  subjects: SubjectRec[]
  terms: TermRec[]
  sheets: MarksSheetRec[]
}): ExamDto[] {
  const cs = new Map(p.classSubjects.map((x) => [x.id, x]))
  const cls = new Map(p.classes.map((x) => [x.id, x]))
  const sub = new Map(p.subjects.map((x) => [x.id, x]))
  const term = new Map(p.terms.map((x) => [x.id, x]))
  const exams = [...p.exams].sort((a, b) => {
    const sa = a.starts_on ?? ''
    const sb = b.starts_on ?? ''
    if (sa !== sb) return sa < sb ? 1 : -1 // DESC (NULLs last, as SQLite orders NULL lowest)
    return a.name.localeCompare(b.name)
  })
  return exams.map((e) => {
    const subjects = p.examSubjects
      .filter((es) => es.exam_id === e.id)
      .flatMap((es) => {
        const c = cs.get(es.class_subject_id)
        const k = c ? cls.get(c.class_id) : undefined
        const s = c ? sub.get(c.subject_id) : undefined
        if (!c || !k || !s) return [] // inner joins in the native query
        return [{
          id: es.id,
          class_subject_id: es.class_subject_id,
          class_id: c.class_id,
          class_display: k.display,
          subject_name: s.name,
          max_marks: es.max_marks,
          status: p.sheets.find((m) => m.exam_subject_id === es.id)?.status ?? 'not_started',
          _order: k.sort_order ?? 0,
        }]
      })
      .sort((a, b) => a._order - b._order || a.subject_name.localeCompare(b.subject_name))
      .map(({ _order: _o, ...rest }) => rest)
    return { id: e.id, name: e.name, term_name: e.term_id ? term.get(e.term_id)?.name ?? null : null, starts_on: e.starts_on ?? null, ends_on: e.ends_on ?? null, subjects }
  })
}

/** list_students_logic (pure): active students with their current class + roll, by
 *  name; or one class's students by roll number when `classId` is given. */
export function buildStudents(p: { classId?: string | null; students: StudentRec[]; enrollments: EnrollmentRec[]; classes: ClassRec[] }): StudentDto[] {
  const cls = new Map(p.classes.map((c) => [c.id, c]))
  const current = new Map<string, EnrollmentRec>()
  for (const e of p.enrollments) if (!e.to_date) current.set(e.student_id, e)
  const rows = p.students
    .filter((s) => !s.status || s.status === 'active')
    .map((s) => ({ s, e: current.get(s.id) }))
    .filter(({ e }) => (p.classId ? e?.class_id === p.classId : true))
    .map(({ s, e }) => ({
      id: s.id,
      name: s.name,
      admission_no: s.admission_no ?? null,
      provisional_no: s.provisional_no ?? null,
      class_display: e ? cls.get(e.class_id)?.display ?? null : null,
      roll_no: e?.roll_no ?? null,
    }))
  if (p.classId) return rows.sort((a, b) => (a.roll_no ?? 1e9) - (b.roll_no ?? 1e9))
  return rows.sort((a, b) => a.name.localeCompare(b.name))
}

/** list_requests (pure): the caller's own requests (a Principal sees all), newest
 *  first, optionally filtered by status / kind, mapped to RequestDto. */
export function buildRequests(p: { staff: Pick<SessionStaff, 'id' | 'role' | 'name'>; requests: RequestRec[]; staffList: StaffRec[]; status?: string | null; kind?: string | null }): RequestDto[] {
  const names = new Map(p.staffList.map((s) => [s.id, s.name]))
  return p.requests
    .filter((r) => p.staff.role === 'principal' || r.requested_by === p.staff.id)
    .filter((r) => !p.status || r.status === p.status)
    .filter((r) => !p.kind || (r.type ?? r.kind) === p.kind)
    .sort((a, b) => ((a.created_at ?? '') < (b.created_at ?? '') ? 1 : (a.created_at ?? '') > (b.created_at ?? '') ? -1 : 0))
    .map((r) => ({
      id: r.id,
      kind: r.type ?? r.kind ?? '',
      target_table: r.target_table ?? '',
      target_id: r.target_id ?? '',
      reason: r.reason ?? '',
      status: r.status ?? 'pending',
      apply_state: r.apply_state ?? 'not_applied',
      before_json: r.before_json ?? null,
      after_json: r.after_json ?? null,
      requested_by: r.requested_by ?? '',
      requester_name: names.get(r.requested_by ?? '') ?? r.requester_name ?? (r.requested_by === p.staff.id ? p.staff.name : ''),
      created_at: r.created_at ?? '',
      note: r.note ?? null,
    }))
}

/** cancel_request_in's checks (pure): own request, still pending or returned. */
export function cancelCheck(staffId: string, r: RequestRec | undefined): CmdError | null {
  if (!r) return { code: 'NOT_FOUND', message_key: 'error.NOT_FOUND', vars: {} }
  if (r.requested_by !== staffId) return { code: 'FORBIDDEN', message_key: 'error.FORBIDDEN', vars: { reason: 'not_requester' } }
  if (r.status !== 'pending' && r.status !== 'returned') return { code: 'VALIDATION', message_key: 'error.VALIDATION', vars: { field: 'request', rule: 'not_cancellable' } }
  return null
}

/** The RequestCmd::Cancel payload (serde `#[serde(tag = "cmd")]`). */
export function cancelPayload(id: string): Record<string, unknown> {
  return { cmd: 'cancel', id }
}

/** sync_status (pure): pending = ops still in the outbox (not refused); the "last
 *  confirmed" time is the last Drive pass that completed (recorded by drive/sync.ts). */
export function buildSyncStatus(pendingCount: number, lastOkMs: number | null | undefined): SyncStatusDto {
  return { pending_count: pendingCount, pending_paise: 0, last_confirmed_at: lastOkMs ? new Date(lastOkMs).toISOString() : null }
}

// ---- shells (local store) -----------------------------------------------------

async function me(): Promise<SessionStaff> {
  const staff = await kvGet<SessionStaff>('staff')
  if (!staff) throw { code: 'LOCKED', message_key: 'error.LOCKED', vars: {} } satisfies CmdError
  return staff
}

export async function dashboardTeacher(today: string): Promise<TeacherDashboard> {
  const staff = await me()
  const [classes, classSubjects, substitutions, requests, enrollments, sheets] = await Promise.all([
    listRecords<ClassRec>('class'),
    listRecords<ClassSubjectRec>('class_subject'),
    listRecords<SubstitutionRec>('substitution'),
    listRecords<RequestRec>('request'),
    listRecords<EnrollmentRec>('enrollment'),
    listRecords<SheetRec>('attendance_sheet'),
  ])
  return buildTeacherDashboard({ staffId: staff.id, role: staff.role, today, classes, classSubjects, substitutions, requests, enrollments, sheets })
}

export async function listExams(): Promise<ExamDto[]> {
  await me()
  const [exams, examSubjects, classSubjects, classes, subjects, terms, sheets] = await Promise.all([
    listRecords<ExamRec>('exam'),
    listRecords<ExamSubjectRec>('exam_subject'),
    listRecords<ClassSubjectRec>('class_subject'),
    listRecords<ClassRec>('class'),
    listRecords<SubjectRec>('subject'),
    listRecords<TermRec>('term'),
    listRecords<MarksSheetRec>('marks_sheet'),
  ])
  return buildExams({ exams, examSubjects, classSubjects, classes, subjects, terms, sheets })
}

export async function listStudents(classId?: string | null): Promise<StudentDto[]> {
  await me()
  const [students, enrollments, classes] = await Promise.all([
    listRecords<StudentRec>('student'),
    listRecords<EnrollmentRec>('enrollment'),
    listRecords<ClassRec>('class'),
  ])
  return buildStudents({ classId, students, enrollments, classes })
}

export async function listRequests(status?: string | null, kind?: string | null): Promise<RequestDto[]> {
  const staff = await me()
  const [requests, staffList] = await Promise.all([listRecords<RequestRec>('request'), listRecords<StaffRec>('staff')])
  return buildRequests({ staff, requests, staffList, status, kind })
}

export async function cancelRequest(id: string): Promise<void> {
  const staff = await me()
  const [row] = await listRecords<RequestRec>('request', (r) => r.id === id)
  const denied = cancelCheck(staff.id, row)
  if (denied) throw denied
  await writeOp({
    table: 'request',
    recordId: id,
    kind: 'action',
    audience: 'admin',
    payload: cancelPayload(id),
    localRow: { ...row, status: 'cancelled', sync_state: 'on_device' },
    action: 'cancel_request',
  })
}

export async function syncStatus(): Promise<SyncStatusDto> {
  const [ops, lastOk] = await Promise.all([pendingOps(), kvGet<number>(LAST_SYNC_OK_KEY)])
  return buildSyncStatus(ops.length, lastOk ?? null)
}
