//! Tauri command surface (prompts/P03 Step 5). Commands are thin: they check the
//! session, call vidya-core `can`, run reads/writes, and return DTOs or a
//! `CmdError { code, message_key, vars }`. Business rules live in vidya-core.
//!
//! The pure logic lives in `logic::*` (takes `&mut Connection` + params) so it is
//! unit-testable without a Tauri runtime; the `#[tauri::command]` wrappers lock
//! `RtCtx` and call it.

pub mod logic;

use tauri::State;

use crate::ctx::RtCtx;
use crate::error::CmdResult;
use crate::state::AppStateResponse;

use logic::*;

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
    "list_classes",
    "list_students",
    "search_students",
    "get_student",
    "create_student",
    "get_attendance_sheet",
    "save_attendance_draft",
    "submit_attendance",
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
    state.with_db(|conn| Ok(crate::state::compute(conn, session)?))
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
pub fn list_classes(state: State<RtCtx>) -> CmdResult<Vec<ClassDto>> {
    state.with_db(list_classes_logic)
}

#[tauri::command]
pub fn list_students(state: State<RtCtx>, class_id: Option<String>) -> CmdResult<Vec<StudentDto>> {
    state.with_db(|conn| list_students_logic(conn, class_id.as_deref()))
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
    state.with_db(|conn| create_student_logic(conn, &actor, &input))
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
