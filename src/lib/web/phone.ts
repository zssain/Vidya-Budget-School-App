// Phone-screen commands for the iPhone app (P21). Reads come from the school PC's
// down export (drive/sync.ts readDown → the local encrypted stores); writes are
// queued as the SAME command ops the native app sends, and the school PC replays
// them as this staff member through the same domain logic (sync/apply.rs):
//   attendance → `attendance_sheet`/action {class_id, date, submit, marks}
//   check-in/out → `staff_attendance`/action {cmd, …} (route is decided by the
//                  school PC from how the op arrived — always Drive for this app)
//   leave → `request`/action {cmd:'leave', id, input}
// The pure builders are unit-tested; the shells only read/write the local store.

import type {
  AbsenceListDto,
  AbsentStudentDto,
  AttendanceSheetDto,
  CheckInInput,
  CheckInResult,
  CircularDto,
  LeaveRequestInput,
  MarkInput,
  MessageDto,
  RecordMessageInput,
  RequestDto,
  SessionStaff,
} from '../api'
import { kvGet, listRecords } from './store'
import { newId, writeOp } from './op'

interface ClassRec { id: string; display: string }
interface EnrollmentRec { id: string; student_id: string; class_id: string; roll_no?: number | null; from_date: string; to_date?: string | null }
interface StudentRec { id: string; name: string }
interface SheetRec { id: string; class_id: string; date: string; status: string; local_marks?: MarkInput[] }
interface MarkRec { id: string; sheet_id: string; student_id: string; mark: string }

/** The attendance sheet for a class on a date (pure): the class roster enrolled on
 *  that date, ordered by roll number, with marks from a pending local save if there
 *  is one, else from the school PC's canonical marks. */
export function buildAttendanceSheet(p: {
  classId: string
  date: string
  cls: ClassRec | undefined
  enrollments: EnrollmentRec[]
  students: StudentRec[]
  sheet: SheetRec | undefined
  marks: MarkRec[]
}): AttendanceSheetDto {
  const day = p.date.slice(0, 10)
  const names = new Map(p.students.map((s) => [s.id, s.name]))
  const roster = p.enrollments
    .filter((e) => e.class_id === p.classId && e.from_date.slice(0, 10) <= day && (!e.to_date || e.to_date.slice(0, 10) >= day))
    .sort((a, b) => (a.roll_no ?? 1e9) - (b.roll_no ?? 1e9) || (names.get(a.student_id) ?? '').localeCompare(names.get(b.student_id) ?? ''))
  const markOf = new Map<string, string>()
  if (p.sheet?.local_marks) {
    for (const m of p.sheet.local_marks) markOf.set(m.student_id, m.mark)
  } else if (p.sheet) {
    for (const m of p.marks) if (m.sheet_id === p.sheet.id) markOf.set(m.student_id, m.mark)
  }
  return {
    class_id: p.classId,
    class_display: p.cls?.display ?? '',
    date: p.date,
    status: p.sheet?.status ?? 'draft',
    rows: roster.map((e) => ({
      student_id: e.student_id,
      name: names.get(e.student_id) ?? '',
      roll_no: e.roll_no ?? null,
      mark: markOf.get(e.student_id) ?? null,
    })),
  }
}

/** The attendance op payload the school PC replays (sync/apply.rs apply_attendance_action). */
export function attendancePayload(classId: string, date: string, marks: MarkInput[], submit: boolean): Record<string, unknown> {
  return { class_id: classId, date, submit, marks: marks.map((m) => ({ student_id: m.student_id, mark: m.mark })) }
}

/** Sheet id: the existing one for (class, date), else the deterministic id the school PC uses. */
export function sheetIdFor(existing: SheetRec | undefined, classId: string, date: string): string {
  return existing?.id ?? `sheet-${classId}-${date.slice(0, 10)}`
}

async function me(): Promise<SessionStaff> {
  const staff = await kvGet<SessionStaff>('staff')
  if (!staff) throw { code: 'LOCKED', message_key: 'error.LOCKED', vars: {} }
  return staff
}

export async function getAttendanceSheet(classId: string, date: string): Promise<AttendanceSheetDto> {
  const [classes, enrollments, students, sheets] = await Promise.all([
    listRecords<ClassRec>('class', (c) => c.id === classId),
    listRecords<EnrollmentRec>('enrollment', (e) => e.class_id === classId),
    listRecords<StudentRec>('student'),
    listRecords<SheetRec>('attendance_sheet', (s) => s.class_id === classId && s.date.slice(0, 10) === date.slice(0, 10)),
  ])
  const sheet = sheets[0]
  const marks = sheet ? await listRecords<MarkRec>('attendance_mark', (m) => m.sheet_id === sheet.id) : []
  return buildAttendanceSheet({ classId, date, cls: classes[0], enrollments, students, sheet, marks })
}

export async function writeAttendance(classId: string, date: string, marks: MarkInput[], submit: boolean): Promise<void> {
  const [existing] = await listRecords<SheetRec>('attendance_sheet', (s) => s.class_id === classId && s.date.slice(0, 10) === date.slice(0, 10))
  if (existing?.status === 'submitted' && !existing.local_marks) {
    throw { code: 'SHEET_LOCKED', message_key: 'error.SHEET_LOCKED', vars: {} }
  }
  const id = sheetIdFor(existing, classId, date)
  await writeOp({
    table: 'attendance_sheet',
    recordId: id,
    kind: 'action',
    audience: `class:${classId}`,
    payload: attendancePayload(classId, date, marks, submit),
    localRow: { id, class_id: classId, date, status: submit ? 'submitted' : 'draft', local_marks: marks, sync_state: 'on_device' },
    action: submit ? 'attendance.submit' : 'attendance.save_draft',
  })
}

export async function staffCheckIn(input: CheckInInput): Promise<CheckInResult> {
  const staff = await me()
  // This app only reaches the school through Google Drive, so a check-in can never
  // count as "at school" (§9 default); the school PC records it for the Principal.
  const sent = { ...input, route: 'drive' as const }
  const id = `sat-${staff.id}-${input.date}`
  await writeOp({
    table: 'staff_attendance',
    recordId: id,
    kind: 'action',
    audience: 'admin',
    payload: { cmd: 'check_in', staff_id: staff.id, date: input.date, input: sent },
    localRow: { id, staff_id: staff.id, date: input.date, status: 'away_pending', check_in_min: input.minute, route: 'drive', sync_state: 'on_device' },
    action: 'staff_check_in',
  })
  return { status: 'away_pending', check_in_min: input.minute, clock_warning: false }
}

export async function staffCheckOut(date: string, minute: number): Promise<void> {
  const staff = await me()
  const id = `sat-${staff.id}-${date}`
  const [row] = await listRecords<Record<string, unknown>>('staff_attendance', (r) => r.id === id)
  await writeOp({
    table: 'staff_attendance',
    recordId: id,
    kind: 'action',
    audience: 'admin',
    payload: { cmd: 'check_out', staff_id: staff.id, date, minute },
    localRow: { ...(row ?? { id, staff_id: staff.id, date }), check_out_min: minute, sync_state: 'on_device' },
    action: 'staff_check_out',
  })
}

export async function requestLeave(input: LeaveRequestInput): Promise<RequestDto> {
  const staff = await me()
  const id = newId('req')
  const now = new Date().toISOString()
  const dto: RequestDto = {
    id,
    kind: 'leave',
    target_table: 'staff',
    target_id: staff.id,
    reason: input.reason,
    status: 'pending',
    apply_state: 'not_applied',
    before_json: null,
    after_json: JSON.stringify({ leave_type_id: input.leave_type_id, from: input.from, to: input.to }),
    requested_by: staff.id,
    requester_name: staff.name,
    created_at: now,
    note: null,
  } as RequestDto
  await writeOp({
    table: 'request',
    recordId: id,
    kind: 'action',
    audience: 'admin',
    payload: { cmd: 'leave', id, input },
    localRow: { ...dto, type: 'leave', sync_state: 'on_device' },
    action: 'request_leave',
  })
  return dto
}

// ---- absence alerts + messages ------------------------------------------------

interface GuardianRec { id: string; name: string; mobile?: string | null; email?: string | null; language?: string | null }
interface StudentGuardianRec { id: string; student_id: string; guardian_id: string; is_primary: number | boolean }
interface ConsentRec { id: string; student_id: string; purpose: string; withdrawn_at?: string | null }
interface TemplateRec { id: string; key: string; language: string; body: string }

/** Render a message template: `{key}` → value (vidya_core::messages::render). */
export function renderTemplate(body: string, vars: Record<string, string>): string {
  let out = body
  for (const [k, v] of Object.entries(vars)) out = out.split(`{${k}}`).join(v)
  return out
}

/** Absent students of a SUBMITTED sheet with their primary guardian, consent and a
 *  rendered preview (pure; mirrors commands::logic::list_absent_logic). */
export function buildAbsenceList(p: {
  classId: string
  date: string
  cls: ClassRec | undefined
  sheet: SheetRec | undefined
  marks: MarkRec[]
  students: StudentRec[]
  enrollments: EnrollmentRec[]
  links: StudentGuardianRec[]
  guardians: GuardianRec[]
  consents: ConsentRec[]
  templates: TemplateRec[]
  schoolName: string
}): AbsenceListDto {
  const base = { class_id: p.classId, class_display: p.cls?.display ?? null, date: p.date }
  if (!p.sheet || p.sheet.status !== 'submitted') return { ...base, submitted: false, students: [] }
  const name = new Map(p.students.map((s) => [s.id, s.name]))
  const roll = new Map(p.enrollments.filter((e) => !e.to_date).map((e) => [e.student_id, e.roll_no ?? null]))
  const tmpl = (lang: string) =>
    p.templates.find((t) => t.key === 'absence_alert' && t.language === lang)?.body ??
    p.templates.find((t) => t.key === 'absence_alert' && t.language === 'en')?.body ??
    ''
  const students: AbsentStudentDto[] = p.marks
    .filter((m) => m.sheet_id === p.sheet!.id && m.mark === 'A')
    .map((m) => {
      const link = p.links.find((l) => l.student_id === m.student_id && (l.is_primary === 1 || l.is_primary === true))
      const g = link ? p.guardians.find((x) => x.id === link.guardian_id) : undefined
      const lang = g?.language && ['en', 'hi', 'te'].includes(g.language) ? g.language : 'en'
      const studentName = name.get(m.student_id) ?? ''
      return {
        student_id: m.student_id,
        student_name: studentName,
        roll_no: roll.get(m.student_id) ?? null,
        guardian_id: g?.id ?? null,
        guardian_name: g?.name ?? null,
        guardian_mobile: g?.mobile ?? null,
        guardian_email: g?.email ?? null,
        guardian_language: g?.language ?? null,
        has_messages_consent: p.consents.some((c) => c.student_id === m.student_id && c.purpose === 'messages' && !c.withdrawn_at),
        preview: renderTemplate(tmpl(lang), { student_name: studentName, date: p.date, school_name: p.schoolName }),
      }
    })
    .sort((a, b) => (a.roll_no ?? 1e9) - (b.roll_no ?? 1e9))
  return { ...base, submitted: true, students }
}

export async function listAbsent(classId: string, date: string): Promise<AbsenceListDto> {
  const [classes, sheets, students, enrollments, links, guardians, consents, templates, schools] = await Promise.all([
    listRecords<ClassRec>('class', (c) => c.id === classId),
    listRecords<SheetRec>('attendance_sheet', (s) => s.class_id === classId && s.date.slice(0, 10) === date.slice(0, 10)),
    listRecords<StudentRec>('student'),
    listRecords<EnrollmentRec>('enrollment', (e) => e.class_id === classId),
    listRecords<StudentGuardianRec>('student_guardian'),
    listRecords<GuardianRec>('guardian'),
    listRecords<ConsentRec>('consent'),
    listRecords<TemplateRec>('message_template'),
    listRecords<{ id: string; name: string }>('school'),
  ])
  const sheet = sheets[0]
  const marks = sheet ? await listRecords<MarkRec>('attendance_mark', (m) => m.sheet_id === sheet.id) : []
  return buildAbsenceList({ classId, date, cls: classes[0], sheet, marks, students, enrollments, links, guardians, consents, templates, schoolName: schools[0]?.name ?? '' })
}

/** Record a message (absence alert / tap-to-WhatsApp). The school PC replays it and
 *  alone sends email / automatic WhatsApp; a wa_tap is "tapped" (never "sent"). */
export async function recordMessage(input: RecordMessageInput): Promise<MessageDto> {
  if (input.related_table === 'student' && input.related_id) {
    const consents = await listRecords<ConsentRec>('consent', (c) => c.student_id === input.related_id)
    if (!consents.some((c) => c.purpose === 'messages' && !c.withdrawn_at)) {
      throw { code: 'FORBIDDEN', message_key: 'error.FORBIDDEN', vars: { reason: 'messages_consent' } }
    }
  }
  const id = newId('msg')
  const now = new Date().toISOString()
  const status = input.channel === 'wa_tap' ? 'tapped' : 'queued'
  const dto: MessageDto = {
    id, channel: input.channel, kind: input.kind, language: input.language,
    to_guardian_id: input.to_guardian_id ?? null, to_address: input.to_address ?? null,
    subject: input.subject ?? null, body: input.body ?? null, status, error: null,
    related_table: input.related_table ?? null, related_id: input.related_id ?? null, created_at: now,
  }
  await writeOp({
    table: 'message',
    recordId: id,
    kind: 'action',
    audience: 'admin',
    payload: { cmd: 'record_message', input: { ...input, id } },
    localRow: { ...dto, sync_state: 'on_device' },
    action: 'record_message',
  })
  return dto
}

// ---- circulars (inbox) --------------------------------------------------------

interface CircularRec { id: string; number?: string | null; title: string; body: string; status: string; sent_at?: string | null; created_at: string; created_by?: string | null }
interface CircularReadRec { id: string; circular_id: string; staff_id: string; read_at?: string | null }

export async function listCirculars(): Promise<CircularDto[]> {
  const staff = await me()
  const [circs, reads] = await Promise.all([
    listRecords<CircularRec>('circular', (c) => c.status === 'sent'),
    listRecords<CircularReadRec>('circular_read'),
  ])
  return circs
    .map((c) => ({
      id: c.id, number: c.number ?? null, title: c.title, body: c.body, languages_json: null, audience_json: null,
      channels_json: null, attachments_json: null, status: c.status, created_by: c.created_by ?? null,
      sent_at: c.sent_at ?? null, created_at: c.created_at,
      read_count: reads.filter((r) => r.circular_id === c.id && r.read_at).length,
      staff_count: 0,
      read_by_me: reads.some((r) => r.circular_id === c.id && r.staff_id === staff.id && r.read_at),
    }))
    .sort((a, b) => (b.sent_at ?? b.created_at).localeCompare(a.sent_at ?? a.created_at))
}

export async function markCircularRead(circularId: string): Promise<void> {
  const staff = await me()
  const id = `crd-${circularId}-${staff.id}`
  const now = new Date().toISOString()
  await writeOp({
    table: 'circular_read',
    recordId: id,
    kind: 'action',
    audience: 'admin',
    payload: { cmd: 'mark_circular_read', circular_id: circularId },
    localRow: { id, circular_id: circularId, staff_id: staff.id, read_at: now, sync_state: 'on_device' },
    action: 'mark_circular_read',
  })
}
