// Typed wrappers over the Tauri command surface (prompts/P03 Step 5).
//
// One exported function per command. `src/lib/api.test.ts` checks this set
// against `src/lib/commands.json` (the shared source of truth), and a Rust test
// checks the registered commands against the same JSON — so the three stay in
// sync or the build fails.
//
// Tauri v2 converts camelCase JS arg keys to the snake_case Rust parameters.

import { invoke } from '@tauri-apps/api/core'

// ---- Error shape (thrown by invoke on Err) -------------------------------
export interface CmdError {
  code: string
  message_key: string
  vars: unknown
}

// ---- Shared DTO types (mirror the Rust serde output) ---------------------
export interface SessionStaff {
  id: string
  name: string
  role: string
}

export interface StaffDto {
  id: string
  name: string
  role: string
}

export type AppState =
  | { kind: 'no_school' }
  | { kind: 'activated' }
  | { kind: 'setup_in_progress'; step: number }
  | { kind: 'locked' }
  | { kind: 'unlocked'; staff: SessionStaff }
  | { kind: 'db_key_missing' }
  | { kind: 'needs_rejoin' }
  | { kind: 'moved' }

export interface AppStateResponse {
  state: AppState
  licence_status: string | null
}

export interface ClassDto {
  id: string
  display: string
  name: string
  section: string | null
}

export interface SchoolDto {
  name: string
  address: string | null
  board: string | null
  phone: string | null
  session_label: string | null
  session_read_only: boolean
}

export interface StudentDto {
  id: string
  name: string
  admission_no: string | null
  provisional_no: string | null
  class_display: string | null
  roll_no: number | null
}

export interface StudentRowDto {
  id: string
  name: string
  admission_no: string | null
  provisional_no: string | null
  class_display: string | null
  section: string | null
  roll_no: number | null
  guardian_name: string | null
  status: string
}

export interface StudentsPageDto {
  rows: StudentRowDto[]
  total: number
}

export interface StudentQuery {
  class_id?: string | null
  section?: string | null
  status?: string | null
  query?: string | null
  limit: number
  offset: number
}

export interface EnrollmentHistoryDto {
  class_display: string | null
  session_label: string | null
  roll_no: number | null
  from_date: string
  to_date: string | null
}

export interface AttendanceSummaryDto {
  present: number
  absent: number
  leave: number
  marked: number
  pct_tenths: number
  term_label: string | null
  from_date: string | null
  to_date: string | null
}

export interface StudentProfileDto {
  id: string
  name: string
  admission_no: string | null
  provisional_no: string | null
  dob: string | null
  gender: string | null
  guardian_name: string | null
  guardian_mobile: string | null
  address: string | null
  transport: boolean
  category: string | null
  rte: boolean
  aadhaar_status: string
  status: string
  left_on: string | null
  left_reason: string | null
  class_id: string | null
  class_display: string | null
  roll_no: number | null
  version: number
  enrollment_history: EnrollmentHistoryDto[]
  attendance: AttendanceSummaryDto
  /** v2 (P13): guardians from the guardian table, primary first. */
  guardians: GuardianDto[]
}

export interface ImportRowError {
  column: string
  reason: string
}
export interface ImportRowDto {
  row: number
  name: string
  class: string
  errors: ImportRowError[]
  duplicate: boolean
}
export interface ImportPreviewDto {
  total: number
  valid: number
  rows: ImportRowDto[]
  error: string | null
}
export interface ImportResultDto {
  imported: number
  skipped: ImportRowDto[]
}

export interface FeeLineDto {
  id: string
  label: string
  amount_paise: number
  balance_paise: number
}

export interface FeeDuesDto {
  student_id: string
  total_due_paise: number
  lines: FeeLineDto[]
}

export interface ReceiptLineDto {
  label: string
  amount_paise: number
}
export interface ReceiptDto {
  id: string
  receipt_no: string
  student_id: string
  student_name: string
  guardian_mobile: string | null
  class_display: string | null
  admission_no: string | null
  provisional_no: string | null
  amount_paise: number
  amount_words_en: string
  amount_words_hi: string
  mode: string
  reference: string | null
  collected_by_name: string | null
  collected_at: string
  confirmed: boolean
  lines: ReceiptLineDto[]
  advance_credit_paise: number
  balance_after_paise: number
  reversed: boolean
}
export interface ReceiptSummaryDto {
  id: string
  receipt_no: string
  student_name: string
  amount_paise: number
  mode: string
  collected_at: string
  confirmed: boolean
  reversed: boolean
}

export interface DayBookEntry {
  time: string
  receipt_no: string
  student_name: string
  class_display: string | null
  mode: string
  reference_last4: string | null
  amount_paise: number
  collected_by: string | null
  confirmed: boolean
  reversed: boolean
}
export interface DayBookDto {
  date: string
  cash_paise: number
  upi_paise: number
  cheque_paise: number
  total_paise: number
  reversals_paise: number
  provisional_paise: number
  entries: DayBookEntry[]
}

export interface FeeOverviewRow {
  class_id: string
  class_display: string
  students_with_dues: number
  outstanding_paise: number
}

export interface FeeHeadDto {
  id: string
  name: string
  name_hi: string | null
  amount_paise: number
  frequency: string
  applies_to: string
  active: boolean
  has_allocations: boolean
}

export interface FeeHeadInput {
  name: string
  name_hi?: string | null
  amount_paise: number
  frequency: string
  applies_to: string
}

export interface FeeHeadChangePreview {
  affected_dues: number
  delta_paise: number
}

export interface PaymentDto {
  id: string
  receipt_no: string
  student_id: string
  amount_paise: number
  mode: string
  confirmed: boolean
}

export interface AttendanceRowDto {
  student_id: string
  name: string
  roll_no: number | null
  mark: string | null
}

export interface AttendanceSheetDto {
  class_id: string
  class_display: string
  date: string
  status: string
  rows: AttendanceRowDto[]
}

export interface MonthStudent {
  id: string
  name: string
  roll_no: number | null
  marks: (string | null)[]
  present: number
  absent: number
  leave: number
  pct_tenths: number
}
export interface AttendanceMonthDto {
  class_id: string
  class_display: string
  month: string
  days: string[]
  students: MonthStudent[]
  day_present: number[]
  day_marked: number[]
}

export interface GradeBandDto {
  min_pct: number
  max_pct: number
  grade: string
  grade_point: number | null
}
export interface ClassSubjectDto {
  id: string
  class_display: string | null
  subject_name: string
}
export interface ExamSubjectDto {
  id: string
  class_subject_id: string
  class_id: string
  class_display: string | null
  subject_name: string
  max_marks: number
  status: string
}
export interface ExamDto {
  id: string
  name: string
  term_name: string | null
  starts_on: string | null
  ends_on: string | null
  subjects: ExamSubjectDto[]
}
export interface ExamSubjectInput {
  class_subject_id: string
  max_marks: number
}
export interface NewExamInput {
  name: string
  term_id?: string | null
  starts_on?: string | null
  ends_on?: string | null
  subjects: ExamSubjectInput[]
}
export interface MarksRowDto {
  student_id: string
  name: string
  roll_no: number | null
  marks: number | null
  absent: boolean
}
export interface MarksSheetDto {
  exam_subject_id: string
  class_id: string
  class_display: string | null
  subject_name: string
  max_marks: number
  status: string
  rows: MarksRowDto[]
}
export interface MarkEntryInput {
  student_id: string
  marks: number | null
  absent: boolean
}
export interface ReportSubjectDto {
  subject_name: string
  max_marks: number
  obtained: number | null
  absent: boolean
  pct_tenths: number
  grade: string
  incomplete: boolean
}
export interface ReportCardDto {
  student_id: string
  student_name: string
  class_display: string | null
  roll_no: number | null
  admission_no: string | null
  provisional_no: string | null
  exam_name: string
  subjects: ReportSubjectDto[]
  total_obtained: number
  total_max: number
  pct_tenths: number
  grade: string | null
  incomplete: boolean
  attendance: AttendanceSummaryDto
}

export interface AuditRowDto {
  seq: number
  at: string
  staff_name: string | null
  action: string
  table: string | null
  record_id: string | null
  reason: string | null
}
export interface AuditPageDto {
  rows: AuditRowDto[]
  total: number
  chain_ok: boolean
  first_bad_seq: number | null
}
export interface AuditQuery {
  staff_id?: string | null
  table?: string | null
  action?: string | null
  date?: string | null
  limit: number
  offset: number
}
export interface MonthCountDto {
  month: string
  count: number
}
export interface MoneyDayDto {
  day: string
  total_paise: number
}
export interface FeeCollectionDto {
  from: string
  to: string
  cash_paise: number
  upi_paise: number
  cheque_paise: number
  total_paise: number
  by_day: MoneyDayDto[]
}
export interface GradeCountDto {
  grade: string
  count: number
}
export interface ExamResultRowDto {
  class_display: string | null
  subject_name: string
  max_marks: number
  graded: number
  average_pct_tenths: number
  distribution: GradeCountDto[]
}

export interface RequestDto {
  id: string
  kind: string
  target_table: string
  target_id: string
  reason: string
  status: string
  apply_state: string
  before_json: string | null
  after_json: string | null
  requested_by: string
  requester_name: string
  created_at: string
  note: string | null
}

export interface ApprovalRow {
  id: string
  kind: string
  what: string
  who: string
  created_at: string
}

export interface ClassPct {
  name: string
  pct: number | null
}

export interface FeeDay {
  day: string
  value_paise: number
}

/** A class whose attendance sheet is still a draft today (Home "Needs attention"). */
export interface PendingClass {
  class: string
  teacher: string | null
}

/** One switchable module (Settings → Languages & modules). `key` is the module_setting key. */
export interface ModuleRow {
  key: string
  enabled: boolean
}

export interface PrincipalDashboard {
  attendance_pct_tenths: number
  attendance_marked: number
  attendance_total: number
  attendance_pending: PendingClass[]
  collected_today_paise: number
  receipts_today: number
  waiting_paise: number
  outstanding_paise: number
  students_with_dues: number
  active_students: number
  admissions_this_week: number
  approvals: ApprovalRow[]
  approvals_total: number
  classes: ClassPct[]
  fee_days: FeeDay[]
  fee_total_paise: number
  term_label: string | null
  open_conflicts: number
  last_backup: LastBackupInfo | null
}

export interface LastBackupInfo {
  at: string
  status: string
  destination: string | null
}

export interface AccountantDashboard {
  collected_today_paise: number
  receipts_today: number
  outstanding_paise: number
  students_with_dues: number
}

export interface TeacherDashboard {
  classes: { class_id: string; display: string; submitted_today: boolean }[]
}

export interface RecoveryKeyDto {
  key: string
}

export interface AuditChainDto {
  ok: boolean
  first_bad_seq: number | null
}

export interface BackupRunDto {
  at: string
  status: string
  destination: string | null
  chain_head: string | null
}

export interface BackupStatusDto {
  enabled: boolean
  runs: BackupRunDto[]
}

// ---- Input types ---------------------------------------------------------
export interface SchoolInput {
  name: string
  address?: string | null
  board?: string | null
  udise?: string | null
  phone?: string | null
}
export interface SessionInput {
  label: string
  starts_on: string
  ends_on: string
  term1_starts: string
  term1_ends: string
  term2_starts: string
  term2_ends: string
}
export interface ClassInput {
  name: string
  section?: string | null
  display: string
}
export interface NewStudentInput {
  name: string
  class_id: string
  roll_no?: number | null
  guardian_name?: string | null
  guardian_mobile?: string | null
  dob?: string | null
  gender?: string | null
  address?: string | null
  transport?: boolean | null
  rte?: boolean | null
  category?: string | null
  aadhaar_status?: string | null
}
export interface MarkInput {
  student_id: string
  mark: string
}
export interface PaymentInput {
  student_id: string
  amount_paise: number
  mode: string
  reference?: string | null
}
export interface RequestInput {
  kind: string
  target_table: string
  target_id: string
  base_version: number
  reason: string
  before_json?: string | null
  after_json?: string | null
}

// ---- Commands (one wrapper per registered command) -----------------------
export const app_state = () => invoke<AppStateResponse>('app_state')
// v2 (Phase 12): licences are offline files. `licenceKey` is the pasted key or the
// text of a loaded `.vlic`; verified offline against this computer's machine code
// and the build-config public key(s). No online activation, no LICENCE_API.
export const activate_licence = (licenceKey: string) =>
  invoke<AppStateResponse>('activate_licence', { licenceKey })
/** This computer's machine code (`XXXX-XXXX-XXXX-C`) for Welcome → Set up. */
export const machine_code = () => invoke<string>('machine_code')
export const setup_school = (input: SchoolInput) => invoke<void>('setup_school', { input })
export const setup_session = (input: SessionInput) => invoke<void>('setup_session', { input })
export const setup_classes = (sections: ClassInput[]) => invoke<void>('setup_classes', { sections })
export const setup_principal = (name: string, mobile: string) =>
  invoke<void>('setup_principal', { name, mobile })
export const create_recovery_key = () => invoke<RecoveryKeyDto>('create_recovery_key')
export const confirm_recovery_key = (group3: string, group5: string) =>
  invoke<void>('confirm_recovery_key', { group3, group5 })
export const create_pin = (pin: string) => invoke<void>('create_pin', { pin })
export const unlock = (staffId: string, pin: string) =>
  invoke<AppStateResponse>('unlock', { staffId, pin })
export const lock = () => invoke<void>('lock')
export const switch_user = () => invoke<void>('switch_user')
export const list_staff = () => invoke<StaffDto[]>('list_staff')
export const list_classes = () => invoke<ClassDto[]>('list_classes')
export const get_school = () => invoke<SchoolDto>('get_school')
export const list_students = (classId?: string) =>
  invoke<StudentDto[]>('list_students', { classId: classId ?? null })
export const list_students_page = (query: StudentQuery) =>
  invoke<StudentsPageDto>('list_students_page', { query })
export const search_students = (query: string) => invoke<StudentDto[]>('search_students', { query })
export const get_student = (id: string) => invoke<StudentDto>('get_student', { id })
export const get_student_profile = (id: string) => invoke<StudentProfileDto>('get_student_profile', { id })
export const create_student = (input: NewStudentInput) => invoke<StudentDto>('create_student', { input })
export const check_duplicate_students = (name: string, dob?: string, guardianMobile?: string) =>
  invoke<StudentRowDto[]>('check_duplicate_students', { name, dob: dob ?? null, guardianMobile: guardianMobile ?? null })
export const transfer_student = (studentId: string, classId: string, rollNo?: number) =>
  invoke<StudentDto>('transfer_student', { studentId, classId, rollNo: rollNo ?? null })
export const mark_student_left = (studentId: string, leftOn: string, reason: string) =>
  invoke<StudentProfileDto>('mark_student_left', { studentId, leftOn, reason })
export const export_csv = (kind: string, path: string, arg?: string) =>
  invoke<number>('export_csv', { kind, path, arg: arg ?? null })
export const students_csv_template = (path: string) => invoke<void>('students_csv_template', { path })
export const import_students_dry_run = (path: string) => invoke<ImportPreviewDto>('import_students_dry_run', { path })
export const import_students_commit = (path: string) => invoke<ImportResultDto>('import_students_commit', { path })
export const get_attendance_sheet = (classId: string, date: string) =>
  invoke<AttendanceSheetDto>('get_attendance_sheet', { classId, date })
export const save_attendance_draft = (classId: string, date: string, marks: MarkInput[]) =>
  invoke<void>('save_attendance_draft', { classId, date, marks })
export const submit_attendance = (classId: string, date: string, marks: MarkInput[]) =>
  invoke<void>('submit_attendance', { classId, date, marks })
export const attendance_month = (classId: string, month: string) =>
  invoke<AttendanceMonthDto>('attendance_month', { classId, month })
export const correct_attendance = (classId: string, date: string, studentId: string, mark: string, reason: string) =>
  invoke<void>('correct_attendance', { classId, date, studentId, mark, reason })
export const list_grade_bands = () => invoke<GradeBandDto[]>('list_grade_bands')
export const update_grade_bands = (bands: GradeBandDto[]) => invoke<GradeBandDto[]>('update_grade_bands', { bands })
export const list_class_subjects = () => invoke<ClassSubjectDto[]>('list_class_subjects')
export const list_exams = () => invoke<ExamDto[]>('list_exams')
export const create_exam = (input: NewExamInput) => invoke<ExamDto>('create_exam', { input })
export const get_marks_sheet = (examSubjectId: string) => invoke<MarksSheetDto>('get_marks_sheet', { examSubjectId })
export const save_marks_draft = (examSubjectId: string, entries: MarkEntryInput[]) =>
  invoke<void>('save_marks_draft', { examSubjectId, entries })
export const submit_marks = (examSubjectId: string, entries: MarkEntryInput[]) =>
  invoke<void>('submit_marks', { examSubjectId, entries })
export const get_report_card = (studentId: string, examId: string) =>
  invoke<ReportCardDto>('get_report_card', { studentId, examId })
export const class_student_ids = (classId: string) => invoke<string[]>('class_student_ids', { classId })
export const list_audit = (query: AuditQuery) => invoke<AuditPageDto>('list_audit', { query })
export const admissions_by_month = () => invoke<MonthCountDto[]>('admissions_by_month')
export const fee_collection_report = (from: string, to: string) =>
  invoke<FeeCollectionDto>('fee_collection_report', { from, to })
export const exam_results = (examId: string) => invoke<ExamResultRowDto[]>('exam_results', { examId })
export const list_fee_dues = (studentId: string) =>
  invoke<FeeDuesDto>('list_fee_dues', { studentId })
export const fees_overview = () => invoke<FeeOverviewRow[]>('fees_overview')
export const list_fee_heads = () => invoke<FeeHeadDto[]>('list_fee_heads')
export const create_fee_head = (input: FeeHeadInput) => invoke<FeeHeadDto>('create_fee_head', { input })
export const preview_fee_head_change = (id: string, newAmount: number) =>
  invoke<FeeHeadChangePreview>('preview_fee_head_change', { id, newAmount })
export const update_fee_head = (id: string, input: FeeHeadInput) =>
  invoke<FeeHeadDto>('update_fee_head', { id, input })
export const deactivate_fee_head = (id: string) => invoke<void>('deactivate_fee_head', { id })
export const get_receipt = (id: string) => invoke<ReceiptDto>('get_receipt', { id })
export const search_receipts = (query: string) => invoke<ReceiptSummaryDto[]>('search_receipts', { query })
export const reverse_payment = (paymentId: string, reason: string) =>
  invoke<RequestDto>('reverse_payment', { paymentId, reason })
export const print_page = () => invoke<void>('print_page')
export const day_book = (date: string) => invoke<DayBookDto>('day_book', { date })
export const record_payment = (input: PaymentInput) => invoke<PaymentDto>('record_payment', { input })
export const list_payments = (studentId?: string) =>
  invoke<PaymentDto[]>('list_payments', { studentId: studentId ?? null })
export const create_request = (input: RequestInput) => invoke<RequestDto>('create_request', { input })
export const cancel_request = (id: string) => invoke<void>('cancel_request', { id })
export const list_requests = (status?: string, kind?: string) =>
  invoke<RequestDto[]>('list_requests', { status: status ?? null, kind: kind ?? null })
export const get_request = (id: string) => invoke<RequestDto>('get_request', { id })
export const decide_request = (id: string, decision: string, note?: string) =>
  invoke<RequestDto>('decide_request', { id, decision, note: note ?? null })
export const dashboard_principal = () => invoke<PrincipalDashboard>('dashboard_principal')
export const dashboard_accountant = () => invoke<AccountantDashboard>('dashboard_accountant')
export const dashboard_teacher = () => invoke<TeacherDashboard>('dashboard_teacher')
export const set_accent = (hex: string) => invoke<void>('set_accent', { hex })
export const list_modules = () => invoke<ModuleRow[]>('list_modules')
export const set_module = (key: string, enabled: boolean) => invoke<void>('set_module', { key, enabled })

// ---- Phase 13: school calendar (weekly offs + holidays/exams/events) --------
export interface CalendarEventDto {
  id: string
  session_id: string | null
  starts_on: string
  ends_on: string
  kind: string
  title: string
  title_hi: string | null
  title_te: string | null
  is_non_working: boolean
  circular_id: string | null
}
export interface CalendarDto {
  /** Working flag per weekday, index 0 = Monday … 6 = Sunday. */
  week: boolean[]
  events: CalendarEventDto[]
}
export interface CalendarEventInput {
  starts_on: string
  ends_on: string
  kind: string
  title: string
  title_hi?: string | null
  title_te?: string | null
  is_non_working: boolean
}
export const get_calendar = () => invoke<CalendarDto>('get_calendar')
export const set_weekly_offs = (working: boolean[]) => invoke<void>('set_weekly_offs', { working })
export const add_calendar_event = (input: CalendarEventInput) =>
  invoke<CalendarEventDto>('add_calendar_event', { input })
export const update_calendar_event = (id: string, input: CalendarEventInput) =>
  invoke<CalendarEventDto>('update_calendar_event', { id, input })
export const delete_calendar_event = (id: string) => invoke<void>('delete_calendar_event', { id })

// ---- Phase 13: guardians (separate table; up to 2 per student, one primary) --
export interface GuardianDto {
  id: string
  name: string
  relation: string | null
  mobile: string | null
  email: string | null
  language: string
  whatsapp_ok: boolean
  is_primary: boolean
}
export interface GuardianEditInput {
  name: string
  relation?: string | null
  mobile?: string | null
  email?: string | null
  language?: string
  whatsapp_ok?: boolean
}
export const add_guardian = (studentId: string, input: GuardianEditInput) =>
  invoke<GuardianDto[]>('add_guardian', { studentId, input })
export const update_guardian = (studentId: string, guardianId: string, input: GuardianEditInput) =>
  invoke<GuardianDto[]>('update_guardian', { studentId, guardianId, input })
export const set_primary_guardian = (studentId: string, guardianId: string) =>
  invoke<GuardianDto[]>('set_primary_guardian', { studentId, guardianId })
export const remove_guardian = (studentId: string, guardianId: string) =>
  invoke<GuardianDto[]>('remove_guardian', { studentId, guardianId })

export const verify_audit_chain = () => invoke<AuditChainDto>('verify_audit_chain')
export const backup_status = () => invoke<BackupStatusDto>('backup_status')
export const backup_now = (recoveryKey?: string) =>
  invoke<BackupStatusDto>('backup_now', { recoveryKey: recoveryKey ?? null })

// ---- Phase 4: staff & access, invitations, devices, sync, conflicts --------
export interface StaffFullDto {
  id: string
  name: string
  role: string
  mobile: string | null
  google_email: string | null
  state: string
  class_teacher_of: string[]
  class_subjects: string[]
}
export interface InviteDto {
  staff_id: string
  code: string
  link: string
  qr_svg: string
  short_fingerprint: string
  expires_at: string
}
export interface AddStaffInput {
  name: string
  role: string
  mobile: string
  google_email?: string | null
}
export interface DeviceDto {
  id: string
  owner_name: string
  platform: string
  series: string | null
  last_seen_at: string | null
  revoked: boolean
  needs_rejoin: boolean
}
export interface ServerStatusDto {
  school_name: string
  server_epoch: number
  port: number
  fingerprint: string
  lan_addrs: string[]
  device_count: number
}
export interface SyncStatusDto {
  pending_count: number
  pending_paise: number
  last_confirmed_at: string | null
}
export interface ConflictDto {
  id: string
  table: string
  record_id: string
  field: string
  value_a: string | null
  value_b: string | null
  staff_b: string | null
  hlc_b: string | null
}
export interface ReviewFlagDto {
  id: string
  kind: string
  ref_table: string
  ref_id: string
  details_json: string | null
}

export const list_staff_access = () => invoke<StaffFullDto[]>('list_staff_access')
export const add_staff = (input: AddStaffInput) => invoke<InviteDto>('add_staff', { input })
export const suspend_staff = (id: string) => invoke<void>('suspend_staff', { id })
export const remove_staff = (id: string) => invoke<void>('remove_staff', { id })
export const create_invite = (staffId: string) => invoke<InviteDto>('create_invite', { staffId })
export const revoke_invite = (staffId: string) => invoke<void>('revoke_invite', { staffId })
export const set_class_teacher = (classId: string, staffId?: string) =>
  invoke<void>('set_class_teacher', { classId, staffId: staffId ?? null })
export const assign_subject_teacher = (classSubjectId: string, staffId?: string) =>
  invoke<void>('assign_subject_teacher', { classSubjectId, staffId: staffId ?? null })
export const effective_access = (staffId: string) => invoke<string[]>('effective_access', { staffId })
export const list_devices = () => invoke<DeviceDto[]>('list_devices')
export const revoke_device = (id: string) => invoke<void>('revoke_device', { id })
export const server_status = () => invoke<ServerStatusDto>('server_status')
export const sync_status = () => invoke<SyncStatusDto>('sync_status')
export const sync_now = () => invoke<SyncStatusDto>('sync_now')
export const list_conflicts = () => invoke<ConflictDto[]>('list_conflicts')
export const resolve_conflict = (id: string, choice: string, value?: string) =>
  invoke<void>('resolve_conflict', { id, choice, value: value ?? null })
export const list_review_flags = () => invoke<ReviewFlagDto[]>('list_review_flags')
export const resolve_review_flag = (id: string) => invoke<void>('resolve_review_flag', { id })
export const seed_demo_school = () => invoke<void>('seed_demo_school')
