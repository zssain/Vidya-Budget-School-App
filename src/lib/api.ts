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

export interface PrincipalDashboard {
  attendance_pct_tenths: number
  attendance_marked: number
  attendance_total: number
  attendance_pending: string[]
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
  guardian_name?: string | null
  guardian_mobile?: string | null
  dob?: string | null
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
export const activate_licence = (code: string, schoolName: string) =>
  invoke<AppStateResponse>('activate_licence', { code, schoolName })
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
export const search_students = (query: string) => invoke<StudentDto[]>('search_students', { query })
export const get_student = (id: string) => invoke<StudentDto>('get_student', { id })
export const create_student = (input: NewStudentInput) => invoke<StudentDto>('create_student', { input })
export const get_attendance_sheet = (classId: string, date: string) =>
  invoke<AttendanceSheetDto>('get_attendance_sheet', { classId, date })
export const save_attendance_draft = (classId: string, date: string, marks: MarkInput[]) =>
  invoke<void>('save_attendance_draft', { classId, date, marks })
export const submit_attendance = (classId: string, date: string, marks: MarkInput[]) =>
  invoke<void>('submit_attendance', { classId, date, marks })
export const list_fee_dues = (studentId: string) =>
  invoke<FeeDuesDto>('list_fee_dues', { studentId })
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
export const verify_audit_chain = () => invoke<AuditChainDto>('verify_audit_chain')

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
