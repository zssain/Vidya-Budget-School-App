//! Tauri command surface (prompts/P03 Step 5). Commands are thin: they check the
//! session, call vidya-core `can`, run reads/writes, and return DTOs or a
//! `CmdError { code, message_key, vars }`. Business rules live in vidya-core.
//!
//! The pure logic lives in `logic::*` (takes `&mut Connection` + params) so it is
//! unit-testable without a Tauri runtime; the `#[tauri::command]` wrappers lock
//! `RtCtx` and call it.

pub mod logic;
pub mod p04;

use tauri::State;

use crate::ctx::RtCtx;
use crate::error::CmdResult;
use crate::state::AppStateResponse;

use logic::*;
use p04::*;

fn now_utc() -> time::OffsetDateTime {
    time::OffsetDateTime::now_utc()
}

/// Every registered command name. A test asserts this matches
/// `src/lib/commands.json`, which `src/lib/api.ts` is checked against.
pub const COMMANDS: &[&str] = &[
    "app_state",
    "activate_licence",
    "setup_school",
    "setup_session",
    "setup_classes",
    "setup_principal",
    "create_recovery_key",
    "confirm_recovery_key",
    "create_pin",
    "unlock",
    "lock",
    "switch_user",
    "list_staff",
    "list_classes",
    "get_school",
    "list_students",
    "list_students_page",
    "search_students",
    "get_student",
    "get_student_profile",
    "create_student",
    "check_duplicate_students",
    "transfer_student",
    "mark_student_left",
    "export_csv",
    "students_csv_template",
    "import_students_dry_run",
    "import_students_commit",
    "fees_overview",
    "list_fee_heads",
    "create_fee_head",
    "preview_fee_head_change",
    "update_fee_head",
    "deactivate_fee_head",
    "get_receipt",
    "search_receipts",
    "reverse_payment",
    "day_book",
    "print_page",
    "get_attendance_sheet",
    "save_attendance_draft",
    "submit_attendance",
    "attendance_month",
    "correct_attendance",
    "list_grade_bands",
    "update_grade_bands",
    "list_class_subjects",
    "list_exams",
    "create_exam",
    "get_marks_sheet",
    "save_marks_draft",
    "submit_marks",
    "get_report_card",
    "class_student_ids",
    "list_fee_dues",
    "record_payment",
    "list_payments",
    "create_request",
    "cancel_request",
    "list_requests",
    "get_request",
    "decide_request",
    "dashboard_principal",
    "dashboard_accountant",
    "dashboard_teacher",
    "set_accent",
    "verify_audit_chain",
    // Phase 4 — staff & access, invitations, devices, sync, conflicts.
    "list_staff_access",
    "add_staff",
    "suspend_staff",
    "remove_staff",
    "create_invite",
    "revoke_invite",
    "set_class_teacher",
    "assign_subject_teacher",
    "effective_access",
    "list_devices",
    "revoke_device",
    "server_status",
    "sync_status",
    "sync_now",
    "list_conflicts",
    "resolve_conflict",
    "list_review_flags",
    "resolve_review_flag",
    #[cfg(debug_assertions)]
    "seed_demo_school",
];

// ---------------------------------------------------------------- state ------

#[tauri::command]
pub fn app_state(state: State<RtCtx>) -> CmdResult<AppStateResponse> {
    if *state.key_missing.lock().map_err(|_| crate::error::CmdError::internal("lock"))? {
        return Ok(crate::state::db_key_missing());
    }
    let session = state.session.lock().map_err(|_| crate::error::CmdError::internal("lock"))?.clone();
    let resp = state.with_db(|conn| Ok(crate::state::compute(conn, session)?))?;
    // Fencing (P05 Step 5): once this PC is `moved`, it must stop being the school
    // server. Firing the stop signal halts the LAN listener + the relay tunnel; the
    // UI shows the read-only "no longer the school server" state. Idempotent.
    if matches!(resp.state, crate::state::AppState::Moved) {
        state.server_stop.notify_waiters();
    }
    Ok(resp)
}

// -------------------------------------------------------------- licence ------

#[tauri::command]
pub async fn activate_licence(state: State<'_, RtCtx>, code: String, school_name: String) -> CmdResult<AppStateResponse> {
    activate_licence_impl(&state, &code, &school_name).await
}

// ---------------------------------------------------------------- setup ------

#[tauri::command]
pub fn setup_school(state: State<RtCtx>, input: SchoolInput) -> CmdResult<()> {
    state.with_db(|conn| setup_school_logic(conn, &input))
}

#[tauri::command]
pub fn setup_session(state: State<RtCtx>, input: SessionInput) -> CmdResult<()> {
    state.with_db(|conn| setup_session_logic(conn, &input))
}

#[tauri::command]
pub fn setup_classes(state: State<RtCtx>, sections: Vec<ClassInput>) -> CmdResult<()> {
    state.with_db(|conn| setup_classes_logic(conn, &sections))
}

#[tauri::command]
pub fn setup_principal(state: State<RtCtx>, name: String, mobile: String) -> CmdResult<()> {
    state.with_db(|conn| setup_principal_logic(conn, &name, &mobile))
}

#[tauri::command]
pub fn create_recovery_key(state: State<RtCtx>) -> CmdResult<RecoveryKeyDto> {
    create_recovery_key_logic(&state)
}

#[tauri::command]
pub fn confirm_recovery_key(state: State<RtCtx>, group3: String, group5: String) -> CmdResult<()> {
    confirm_recovery_key_logic(&state, &group3, &group5)
}

#[tauri::command]
pub fn create_pin(state: State<RtCtx>, pin: String) -> CmdResult<()> {
    state.with_db(|conn| create_pin_logic(conn, &pin))
}

// -------------------------------------------------------------- session ------

#[tauri::command]
pub fn unlock(state: State<RtCtx>, staff_id: String, pin: String) -> CmdResult<AppStateResponse> {
    unlock_impl(&state, &staff_id, &pin)
}

#[tauri::command]
pub fn lock(state: State<RtCtx>) -> CmdResult<()> {
    *state.session.lock().map_err(|_| crate::error::CmdError::internal("lock"))? = None;
    Ok(())
}

#[tauri::command]
pub fn switch_user(state: State<RtCtx>) -> CmdResult<()> {
    // Clearing the session drops all in-memory screen state on the UI side too.
    *state.session.lock().map_err(|_| crate::error::CmdError::internal("lock"))? = None;
    Ok(())
}

// ------------------------------------------------------------- students ------

#[tauri::command]
pub fn list_staff(state: State<RtCtx>) -> CmdResult<Vec<StaffDto>> {
    state.with_db(list_staff_logic)
}

#[tauri::command]
pub fn list_classes(state: State<RtCtx>) -> CmdResult<Vec<ClassDto>> {
    state.with_db(list_classes_logic)
}

#[tauri::command]
pub fn get_school(state: State<RtCtx>) -> CmdResult<SchoolDto> {
    state.with_db(get_school_logic)
}

#[tauri::command]
pub fn list_students(state: State<RtCtx>, class_id: Option<String>) -> CmdResult<Vec<StudentDto>> {
    state.with_db(|conn| list_students_logic(conn, class_id.as_deref()))
}

#[tauri::command]
pub fn list_students_page(state: State<RtCtx>, query: StudentQuery) -> CmdResult<StudentsPageDto> {
    state.with_db(|conn| list_students_page_logic(conn, &query))
}

#[tauri::command]
pub fn search_students(state: State<RtCtx>, query: String) -> CmdResult<Vec<StudentDto>> {
    state.with_db(|conn| search_students_logic(conn, &query))
}

#[tauri::command]
pub fn get_student(state: State<RtCtx>, id: String) -> CmdResult<StudentDto> {
    state.with_db(|conn| get_student_logic(conn, &id))
}

#[tauri::command]
pub fn create_student(state: State<RtCtx>, input: NewStudentInput) -> CmdResult<StudentDto> {
    let actor = state.require_session()?;
    let device_id = state.device_id.lock().map_err(|_| crate::error::CmdError::internal("lock"))?.clone();
    let mode = state.device_mode;
    state.with_db(|conn| create_student_logic(conn, &actor, device_id.as_deref(), mode, &today(), &input))
}

#[tauri::command]
pub fn get_student_profile(state: State<RtCtx>, id: String) -> CmdResult<StudentProfileDto> {
    state.with_db(|conn| get_student_profile_logic(conn, &today(), &id))
}

#[tauri::command]
pub fn check_duplicate_students(state: State<RtCtx>, name: String, dob: Option<String>, guardian_mobile: Option<String>) -> CmdResult<Vec<StudentRowDto>> {
    state.with_db(|conn| check_duplicate_students_logic(conn, &name, dob.as_deref(), guardian_mobile.as_deref()))
}

#[tauri::command]
pub fn transfer_student(state: State<RtCtx>, student_id: String, class_id: String, roll_no: Option<i64>) -> CmdResult<StudentDto> {
    let actor = state.require_session()?;
    let device_id = state.device_id.lock().map_err(|_| crate::error::CmdError::internal("lock"))?.clone();
    let mode = state.device_mode;
    state.with_db(|conn| transfer_student_logic(conn, &actor, device_id.as_deref(), mode, &today(), &student_id, &class_id, roll_no))
}

#[tauri::command]
pub fn mark_student_left(state: State<RtCtx>, student_id: String, left_on: String, reason: String) -> CmdResult<StudentProfileDto> {
    let actor = state.require_session()?;
    let device_id = state.device_id.lock().map_err(|_| crate::error::CmdError::internal("lock"))?.clone();
    let mode = state.device_mode;
    state.with_db(|conn| mark_student_left_logic(conn, &actor, device_id.as_deref(), mode, &student_id, &left_on, &reason))
}

// -------------------------------------------------------------------- CSV -----

#[tauri::command]
pub fn export_csv(state: State<RtCtx>, kind: String, path: String, arg: Option<String>) -> CmdResult<i64> {
    let actor = state.require_session()?;
    state.with_db(|conn| export_csv_logic(conn, &actor, &kind, &path, arg.as_deref()))
}

#[tauri::command]
pub fn students_csv_template(path: String) -> CmdResult<()> {
    std::fs::write(&path, students_csv_template_logic().into_bytes())
        .map_err(|_| crate::error::CmdError::internal("csv_write"))?;
    Ok(())
}

#[tauri::command]
pub fn import_students_dry_run(state: State<RtCtx>, path: String) -> CmdResult<ImportPreviewDto> {
    let actor = state.require_session()?;
    state.with_db(|conn| import_students_dry_run_logic(conn, &actor, &path))
}

#[tauri::command]
pub fn import_students_commit(state: State<RtCtx>, path: String) -> CmdResult<ImportResultDto> {
    let actor = state.require_session()?;
    let mode = state.device_mode;
    state.with_db(|conn| import_students_commit_logic(conn, &actor, mode, &today(), &path))
}

// ----------------------------------------------------------- fee structure ----

#[tauri::command]
pub fn fees_overview(state: State<RtCtx>) -> CmdResult<Vec<FeeOverviewRow>> {
    let actor = state.require_session()?;
    state.with_db(|conn| fees_overview_logic(conn, &actor))
}

#[tauri::command]
pub fn list_fee_heads(state: State<RtCtx>) -> CmdResult<Vec<FeeHeadDto>> {
    let actor = state.require_session()?;
    state.with_db(|conn| list_fee_heads_logic(conn, &actor))
}

#[tauri::command]
pub fn create_fee_head(state: State<RtCtx>, input: FeeHeadInput) -> CmdResult<FeeHeadDto> {
    let actor = state.require_session()?;
    state.with_db(|conn| create_fee_head_logic(conn, &actor, &input))
}

#[tauri::command]
pub fn preview_fee_head_change(state: State<RtCtx>, id: String, new_amount: i64) -> CmdResult<FeeHeadChangePreview> {
    let actor = state.require_session()?;
    state.with_db(|conn| preview_fee_head_change_logic(conn, &actor, &id, new_amount))
}

#[tauri::command]
pub fn update_fee_head(state: State<RtCtx>, id: String, input: FeeHeadInput) -> CmdResult<FeeHeadDto> {
    let actor = state.require_session()?;
    state.with_db(|conn| update_fee_head_logic(conn, &actor, &id, &input))
}

#[tauri::command]
pub fn deactivate_fee_head(state: State<RtCtx>, id: String) -> CmdResult<()> {
    let actor = state.require_session()?;
    state.with_db(|conn| deactivate_fee_head_logic(conn, &actor, &id))
}

// -------------------------------------------------------------- receipts ------

#[tauri::command]
pub fn get_receipt(state: State<RtCtx>, id: String) -> CmdResult<ReceiptDto> {
    let actor = state.require_session()?;
    state.with_db(|conn| get_receipt_logic(conn, &actor, &id))
}

#[tauri::command]
pub fn search_receipts(state: State<RtCtx>, query: String) -> CmdResult<Vec<ReceiptSummaryDto>> {
    let actor = state.require_session()?;
    state.with_db(|conn| search_receipts_logic(conn, &actor, &query))
}

#[tauri::command]
pub fn reverse_payment(state: State<RtCtx>, payment_id: String, reason: String) -> CmdResult<RequestDto> {
    let actor = state.require_session()?;
    let mode = state.device_mode;
    state.with_db(|conn| reverse_payment_logic(conn, &actor, mode, &payment_id, &reason))
}

#[tauri::command]
pub fn day_book(state: State<RtCtx>, date: String) -> CmdResult<DayBookDto> {
    let actor = state.require_session()?;
    state.with_db(|conn| day_book_logic(conn, &actor, &date))
}

/// Open the OS print dialog for the current window (Tauri 2 WebviewWindow::print,
/// verified present in tauri 2.11). "Printed" means only that the dialog opened
/// (§7); the caller falls back to window.print() if this errors.
#[tauri::command]
pub fn print_page(window: tauri::WebviewWindow) -> CmdResult<()> {
    window.print().map_err(|_| crate::error::CmdError::internal("print_unavailable"))?;
    Ok(())
}

// ----------------------------------------------------------- attendance ------

#[tauri::command]
pub fn get_attendance_sheet(state: State<RtCtx>, class_id: String, date: String) -> CmdResult<AttendanceSheetDto> {
    state.with_db(|conn| get_attendance_sheet_logic(conn, &class_id, &date))
}

#[tauri::command]
pub fn save_attendance_draft(state: State<RtCtx>, class_id: String, date: String, marks: Vec<MarkInput>) -> CmdResult<()> {
    let actor = state.require_session()?;
    state.with_db(|conn| save_attendance_draft_logic(conn, &actor, &class_id, &date, &marks))
}

#[tauri::command]
pub fn submit_attendance(state: State<RtCtx>, class_id: String, date: String, marks: Vec<MarkInput>) -> CmdResult<()> {
    let actor = state.require_session()?;
    state.with_db(|conn| submit_attendance_logic(conn, &actor, &class_id, &date, &marks))
}

#[tauri::command]
pub fn attendance_month(state: State<RtCtx>, class_id: String, month: String) -> CmdResult<AttendanceMonthDto> {
    let actor = state.require_session()?;
    state.with_db(|conn| attendance_month_logic(conn, &actor, &class_id, &month))
}

#[tauri::command]
pub fn correct_attendance(state: State<RtCtx>, class_id: String, date: String, student_id: String, mark: String, reason: String) -> CmdResult<()> {
    let actor = state.require_session()?;
    let mode = state.device_mode;
    state.with_db(|conn| correct_attendance_mark_logic(conn, &actor, mode, &class_id, &date, &student_id, &mark, &reason))
}

// ------------------------------------------------------------- academics ------

#[tauri::command]
pub fn list_grade_bands(state: State<RtCtx>) -> CmdResult<Vec<GradeBandDto>> {
    state.with_db(list_grade_bands_logic)
}

#[tauri::command]
pub fn update_grade_bands(state: State<RtCtx>, bands: Vec<GradeBandDto>) -> CmdResult<Vec<GradeBandDto>> {
    let actor = state.require_session()?;
    state.with_db(|conn| update_grade_bands_logic(conn, &actor, &bands))
}

#[tauri::command]
pub fn list_class_subjects(state: State<RtCtx>) -> CmdResult<Vec<ClassSubjectDto>> {
    state.with_db(list_class_subjects_logic)
}

#[tauri::command]
pub fn list_exams(state: State<RtCtx>) -> CmdResult<Vec<ExamDto>> {
    state.with_db(list_exams_logic)
}

#[tauri::command]
pub fn create_exam(state: State<RtCtx>, input: NewExamInput) -> CmdResult<ExamDto> {
    let actor = state.require_session()?;
    state.with_db(|conn| create_exam_logic(conn, &actor, &input))
}

#[tauri::command]
pub fn get_marks_sheet(state: State<RtCtx>, exam_subject_id: String) -> CmdResult<MarksSheetDto> {
    let actor = state.require_session()?;
    state.with_db(|conn| get_marks_sheet_logic(conn, &actor, &exam_subject_id))
}

#[tauri::command]
pub fn save_marks_draft(state: State<RtCtx>, exam_subject_id: String, entries: Vec<MarkEntryInput>) -> CmdResult<()> {
    let actor = state.require_session()?;
    let mode = state.device_mode;
    state.with_db(|conn| save_marks_draft_logic(conn, &actor, mode, &exam_subject_id, &entries))
}

#[tauri::command]
pub fn submit_marks(state: State<RtCtx>, exam_subject_id: String, entries: Vec<MarkEntryInput>) -> CmdResult<()> {
    let actor = state.require_session()?;
    let mode = state.device_mode;
    state.with_db(|conn| submit_marks_logic(conn, &actor, mode, &exam_subject_id, &entries))
}

#[tauri::command]
pub fn get_report_card(state: State<RtCtx>, student_id: String, exam_id: String) -> CmdResult<ReportCardDto> {
    let actor = state.require_session()?;
    state.with_db(|conn| get_report_card_logic(conn, &actor, &today(), &student_id, &exam_id))
}

#[tauri::command]
pub fn class_student_ids(state: State<RtCtx>, class_id: String) -> CmdResult<Vec<String>> {
    state.with_db(|conn| class_student_ids_logic(conn, &class_id))
}

// ----------------------------------------------------------------- fees ------

#[tauri::command]
pub fn list_fee_dues(state: State<RtCtx>, student_id: String) -> CmdResult<FeeDuesDto> {
    state.with_db(|conn| list_fee_dues_logic(conn, &student_id))
}

#[tauri::command]
pub fn record_payment(state: State<RtCtx>, input: PaymentInput) -> CmdResult<PaymentDto> {
    let actor = state.require_session()?;
    let device_id = state.device_id.lock().map_err(|_| crate::error::CmdError::internal("lock"))?.clone();
    let mode = state.device_mode;
    state.with_db(|conn| record_payment_logic(conn, &actor, device_id.as_deref(), mode, &input))
}

#[tauri::command]
pub fn list_payments(state: State<RtCtx>, student_id: Option<String>) -> CmdResult<Vec<PaymentDto>> {
    state.with_db(|conn| list_payments_logic(conn, student_id.as_deref()))
}

// -------------------------------------------------------------- requests -----

#[tauri::command]
pub fn create_request(state: State<RtCtx>, input: RequestInput) -> CmdResult<RequestDto> {
    let actor = state.require_session()?;
    state.with_db(|conn| create_request_logic(conn, &actor, &input))
}

#[tauri::command]
pub fn cancel_request(state: State<RtCtx>, id: String) -> CmdResult<()> {
    let actor = state.require_session()?;
    state.with_db(|conn| cancel_request_logic(conn, &actor, &id))
}

#[tauri::command]
pub fn list_requests(state: State<RtCtx>, status: Option<String>, kind: Option<String>) -> CmdResult<Vec<RequestDto>> {
    state.with_db(|conn| list_requests_logic(conn, status.as_deref(), kind.as_deref()))
}

#[tauri::command]
pub fn get_request(state: State<RtCtx>, id: String) -> CmdResult<RequestDto> {
    state.with_db(|conn| get_request_logic(conn, &id))
}

#[tauri::command]
pub fn decide_request(state: State<RtCtx>, id: String, decision: String, note: Option<String>) -> CmdResult<RequestDto> {
    let actor = state.require_session()?;
    let mode = state.device_mode;
    state.with_db(|conn| decide_request_logic(conn, &actor, mode, &id, &decision, note.as_deref()))
}

// ----------------------------------------------------------- dashboards ------

#[tauri::command]
pub fn dashboard_principal(state: State<RtCtx>) -> CmdResult<crate::dash::PrincipalDashboard> {
    state.with_db(|conn| Ok(crate::dash::principal_dashboard(conn, &today())?))
}

#[tauri::command]
pub fn dashboard_accountant(state: State<RtCtx>) -> CmdResult<AccountantDashboard> {
    state.with_db(|conn| dashboard_accountant_logic(conn, &today()))
}

#[tauri::command]
pub fn dashboard_teacher(state: State<RtCtx>) -> CmdResult<TeacherDashboard> {
    let actor = state.require_session()?;
    state.with_db(|conn| dashboard_teacher_logic(conn, &actor, &today()))
}

// ---------------------------------------------------------------- misc -------

#[tauri::command]
pub fn set_accent(state: State<RtCtx>, hex: String) -> CmdResult<()> {
    state.with_db(|conn| set_accent_logic(conn, &hex))
}

#[tauri::command]
pub fn verify_audit_chain(state: State<RtCtx>) -> CmdResult<AuditChainDto> {
    state.with_db(|conn| {
        let first_bad = crate::security::audit::verify_chain(conn)?;
        Ok(AuditChainDto { ok: first_bad.is_none(), first_bad_seq: first_bad })
    })
}

#[cfg(debug_assertions)]
#[tauri::command]
pub fn seed_demo_school(state: State<RtCtx>) -> CmdResult<()> {
    state.with_db(|conn| {
        crate::seed::seed_demo_school(conn, time::OffsetDateTime::now_utc())?;
        Ok(())
    })
}

// -------------------------------------------------- P04: staff & access ------

#[tauri::command]
pub fn list_staff_access(state: State<RtCtx>) -> CmdResult<Vec<StaffFullDto>> {
    state.with_db(list_staff_full_logic)
}

#[tauri::command]
pub fn add_staff(state: State<RtCtx>, input: AddStaffInput) -> CmdResult<InviteDto> {
    let actor = state.require_session()?;
    state.with_db(|conn| add_staff_logic(conn, &actor, &input, now_utc()))
}

#[tauri::command]
pub fn suspend_staff(state: State<RtCtx>, id: String) -> CmdResult<()> {
    let actor = state.require_session()?;
    state.with_db(|conn| suspend_staff_logic(conn, &actor, &id))
}

#[tauri::command]
pub fn remove_staff(state: State<RtCtx>, id: String) -> CmdResult<()> {
    let actor = state.require_session()?;
    state.with_db(|conn| remove_staff_logic(conn, &actor, &id))
}

#[tauri::command]
pub fn create_invite(state: State<RtCtx>, staff_id: String) -> CmdResult<InviteDto> {
    let actor = state.require_session()?;
    state.with_db(|conn| create_invite_logic(conn, &actor, &staff_id, now_utc()))
}

#[tauri::command]
pub fn revoke_invite(state: State<RtCtx>, staff_id: String) -> CmdResult<()> {
    let actor = state.require_session()?;
    state.with_db(|conn| revoke_invite_logic(conn, &actor, &staff_id))
}

#[tauri::command]
pub fn set_class_teacher(state: State<RtCtx>, class_id: String, staff_id: Option<String>) -> CmdResult<()> {
    let actor = state.require_session()?;
    state.with_db(|conn| set_class_teacher_logic(conn, &actor, &class_id, staff_id.as_deref()))
}

#[tauri::command]
pub fn assign_subject_teacher(state: State<RtCtx>, class_subject_id: String, staff_id: Option<String>) -> CmdResult<()> {
    let actor = state.require_session()?;
    state.with_db(|conn| assign_subject_teacher_logic(conn, &actor, &class_subject_id, staff_id.as_deref()))
}

#[tauri::command]
pub fn effective_access(state: State<RtCtx>, staff_id: String) -> CmdResult<Vec<String>> {
    state.with_db(|conn| effective_access_logic(conn, &staff_id))
}

// ----------------------------------------------------- P04: sync & devices ----

#[tauri::command]
pub fn list_devices(state: State<RtCtx>) -> CmdResult<Vec<DeviceDto>> {
    state.with_db(list_devices_logic)
}

#[tauri::command]
pub fn revoke_device(state: State<RtCtx>, id: String) -> CmdResult<()> {
    let actor = state.require_session()?;
    state.with_db(|conn| revoke_device_logic(conn, &actor, &id, now_utc()))
}

#[tauri::command]
pub fn server_status(state: State<RtCtx>) -> CmdResult<ServerStatusDto> {
    state.with_db(server_status_logic)
}

#[tauri::command]
pub fn sync_status(state: State<RtCtx>) -> CmdResult<SyncStatusDto> {
    state.with_db(sync_status_logic)
}

#[tauri::command]
pub fn sync_now(state: State<RtCtx>) -> CmdResult<SyncStatusDto> {
    // Single-PC server: nothing to push (it IS the server) — report current state,
    // honestly (no fake "synced"). On client devices this triggers the engine.
    state.with_db(sync_status_logic)
}

// ------------------------------------------------------ P04: conflict review ---

#[tauri::command]
pub fn list_conflicts(state: State<RtCtx>) -> CmdResult<Vec<ConflictDto>> {
    state.with_db(list_conflicts_logic)
}

#[tauri::command]
pub fn resolve_conflict(state: State<RtCtx>, id: String, choice: String, value: Option<String>) -> CmdResult<()> {
    let actor = state.require_session()?;
    state.with_db(|conn| resolve_conflict_logic(conn, &actor, &id, &choice, value.as_deref()))
}

#[tauri::command]
pub fn list_review_flags(state: State<RtCtx>) -> CmdResult<Vec<ReviewFlagDto>> {
    state.with_db(list_review_flags_logic)
}

#[tauri::command]
pub fn resolve_review_flag(state: State<RtCtx>, id: String) -> CmdResult<()> {
    let actor = state.require_session()?;
    state.with_db(|conn| resolve_review_flag_logic(conn, &actor, &id))
}

/// Today's date (YYYY-MM-DD) in UTC.
pub fn today() -> String {
    time::OffsetDateTime::now_utc()
        .date()
        .format(time::macros::format_description!("[year]-[month]-[day]"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    struct CommandsJson {
        commands: Vec<String>,
        #[serde(rename = "debugOnly")]
        debug_only: Vec<String>,
    }

    #[test]
    fn registered_commands_match_shared_json() {
        // The shared list src/lib/commands.json is the single source of truth for
        // the whole command surface; api.ts is checked against it in vitest.
        let raw = include_str!("../../../src/lib/commands.json");
        let json: CommandsJson = serde_json::from_str(raw).expect("commands.json parses");

        let mut expected: Vec<String> = json.commands.clone();
        // This test runs in debug, where the debug-only commands are registered.
        expected.extend(json.debug_only.clone());
        expected.sort();

        let mut actual: Vec<String> = COMMANDS.iter().map(|s| s.to_string()).collect();
        actual.sort();

        assert_eq!(actual, expected, "registered COMMANDS must match src/lib/commands.json");
    }
}
