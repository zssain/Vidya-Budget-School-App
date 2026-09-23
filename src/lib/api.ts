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
export const list_classes = () => invoke<ClassDto[]>('list_classes')
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
export const seed_demo_school = () => invoke<void>('seed_demo_school')
