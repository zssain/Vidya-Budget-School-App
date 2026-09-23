//! Vidya Tauri application.
//!
//! Phase 3 adds the licence/setup/PIN command surface, the app state machine and
//! the demo seed on top of the Phase-2 encrypted DB + audit chain + write helper.

pub mod commands;
pub mod config;
pub mod ctx;
pub mod dash;
pub mod db;
pub mod error;
pub mod kv;
pub mod licence;
pub mod security;
pub mod server;
pub mod state;
pub mod sync;
pub mod write;

#[cfg(debug_assertions)]
pub mod seed;

use std::sync::Mutex;

use tauri::Manager;

use ctx::RtCtx;
use write::DeviceMode;

/// Build the runtime context: machine id, DB key, encrypted connection.
fn build_ctx(data_dir: std::path::PathBuf) -> RtCtx {
    std::fs::create_dir_all(&data_dir).ok();

    // Machine id: keychain on desktop, file fallback elsewhere.
    #[cfg(not(target_os = "android"))]
    let machine_store: Box<dyn licence::machine::MachineIdStore> =
        Box::new(licence::machine::KeyringMachineStore::new());
    #[cfg(target_os = "android")]
    let machine_store: Box<dyn licence::machine::MachineIdStore> =
        Box::new(licence::machine::FileMachineStore::new(licence::machine_id_fallback_path(&data_dir)));
    let machine_id = licence::machine::get_or_create(machine_store.as_ref()).unwrap_or_default();

    // DB key: keychain on desktop, file fallback elsewhere. If a DB file exists
    // but the key is gone → key_missing (never overwrite the DB).
    let db_path = data_dir.join("vidya.db");
    let db_exists = db_path.exists();
    #[cfg(not(target_os = "android"))]
    let key_store: Box<dyn security::keys::KeyStore> = Box::new(security::keys::KeyringStore::new());
    #[cfg(target_os = "android")]
    let key_store: Box<dyn security::keys::KeyStore> =
        Box::new(security::keys::FileKeyStore::new(data_dir.join("db-key")));

    let mut db_key_hex: Option<String> = None;
    let (db, key_missing) = match security::keys::ensure_key(key_store.as_ref(), db_exists) {
        Ok(key) => {
            let hex = security::keys::to_hex(&key);
            db_key_hex = Some(hex.clone());
            match db::open_encrypted(&db_path, &hex) {
                Ok(mut conn) => match db::run_migrations(&mut conn) {
                    Ok(_) => (Some(conn), false),
                    Err(e) => {
                        tracing::error!("migration failed: {e}");
                        (None, false)
                    }
                },
                Err(e) => {
                    tracing::error!("open db failed: {e}");
                    (None, false)
                }
            }
        }
        Err(security::keys::KeyError::Missing) => (None, true),
        Err(e) => {
            tracing::error!("db key error: {e}");
            (None, false)
        }
    };

    RtCtx {
        db: Mutex::new(db),
        session: Mutex::new(None),
        machine_id,
        device_id: Mutex::new(None),
        device_mode: DeviceMode::Server, // single-PC school (this phase)
        http: reqwest::Client::new(),
        recovery: Mutex::new(None),
        data_dir,
        db_key_hex,
        key_missing: Mutex::new(key_missing),
        server_stop: std::sync::Arc::new(tokio::sync::Notify::new()),
    }
}

/// Re-check the licence with the service when due (§10, P05 Step 5). Unreachable →
/// stay active (perpetual). An explicit `moved`/`revoked` is persisted; if this PC is
/// `moved`, the server + relay tunnel are stopped and the UI goes read-only.
async fn recheck_licence(app: &tauri::AppHandle) {
    let ctx = app.state::<RtCtx>();
    // Read the licence identity + last check time under the DB lock.
    let due: Option<(String, String)> = ctx
        .with_db(|conn| {
            let row = conn
                .query_row(
                    "SELECT licence_id, COALESCE(last_check_at,'') FROM licence LIMIT 1",
                    [],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                )
                .ok();
            Ok(row)
        })
        .unwrap_or(None)
        .and_then(|(id, last)| {
            let last_opt = if last.is_empty() { None } else { Some(last.as_str()) };
            if licence::should_recheck(last_opt, time::OffsetDateTime::now_utc()) {
                Some((id, last))
            } else {
                None
            }
        });
    let Some((licence_id, _)) = due else { return };

    let api = config::get().licence_api.clone();
    if let Some(result) = licence::check(&ctx.http, &api, &licence_id, &ctx.machine_id).await {
        use vidya_core::licence::CheckResult;
        let fenced = matches!(result, CheckResult::Moved | CheckResult::Revoked);
        let _ = ctx.with_db(|conn| {
            licence::persist_check(conn, result, time::OffsetDateTime::now_utc())?;
            Ok(())
        });
        if fenced {
            ctx.server_stop.notify_waiters();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            app.manage(build_ctx(data_dir));

            // Start the school server in the background (Server mode, desktop).
            // Guarded: it returns quietly if there is no school yet.
            #[cfg(not(target_os = "android"))]
            {
                let ctx = app.state::<RtCtx>();
                if ctx.device_mode == DeviceMode::Server {
                    if let Some(key_hex) = ctx.db_key_hex.clone() {
                        let db_path = ctx.db_path();
                        let handle = app.handle().clone();
                        let stop = ctx.server_stop.clone();
                        tauri::async_runtime::spawn(server::start::run_server(handle, db_path, key_hex, stop));
                    }
                }
            }

            // Best-effort licence re-check when due (P05 Step 5). A `moved`/`revoked`
            // answer stops this PC serving; unreachable leaves it active (perpetual).
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    recheck_licence(&handle).await;
                });
            }
            Ok(())
        })
        .invoke_handler(invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// The command set must match `commands::COMMANDS` and `src/lib/commands.json`
// (asserted by a test). seed_demo_school is debug-only.
#[cfg(debug_assertions)]
fn invoke_handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    use commands::*;
    tauri::generate_handler![
        app_state, activate_licence, setup_school, setup_session, setup_classes, setup_principal,
        create_recovery_key, confirm_recovery_key, create_pin, unlock, lock, switch_user,
        list_staff, list_classes, get_school, list_students, list_students_page, search_students, get_student,
        get_student_profile, create_student, check_duplicate_students, transfer_student, mark_student_left,
        export_csv, students_csv_template, import_students_dry_run, import_students_commit,
        fees_overview, list_fee_heads, create_fee_head, preview_fee_head_change, update_fee_head, deactivate_fee_head,
        get_receipt, search_receipts, reverse_payment, day_book, print_page,
        get_attendance_sheet, save_attendance_draft, submit_attendance, attendance_month, correct_attendance,
        list_grade_bands, update_grade_bands, list_class_subjects, list_exams, create_exam, get_marks_sheet, save_marks_draft, submit_marks,
        get_report_card, class_student_ids, list_fee_dues,
        record_payment, list_payments, create_request, cancel_request, list_requests, get_request,
        decide_request, dashboard_principal, dashboard_accountant, dashboard_teacher, set_accent,
        verify_audit_chain,
        list_staff_access, add_staff, suspend_staff, remove_staff, create_invite, revoke_invite,
        set_class_teacher, assign_subject_teacher, effective_access, list_devices, revoke_device,
        server_status, sync_status, sync_now, list_conflicts, resolve_conflict, list_review_flags,
        resolve_review_flag, seed_demo_school
    ]
}

#[cfg(not(debug_assertions))]
fn invoke_handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    use commands::*;
    tauri::generate_handler![
        app_state, activate_licence, setup_school, setup_session, setup_classes, setup_principal,
        create_recovery_key, confirm_recovery_key, create_pin, unlock, lock, switch_user,
        list_staff, list_classes, get_school, list_students, list_students_page, search_students, get_student,
        get_student_profile, create_student, check_duplicate_students, transfer_student, mark_student_left,
        export_csv, students_csv_template, import_students_dry_run, import_students_commit,
        fees_overview, list_fee_heads, create_fee_head, preview_fee_head_change, update_fee_head, deactivate_fee_head,
        get_receipt, search_receipts, reverse_payment, day_book, print_page,
        get_attendance_sheet, save_attendance_draft, submit_attendance, attendance_month, correct_attendance,
        list_grade_bands, update_grade_bands, list_class_subjects, list_exams, create_exam, get_marks_sheet, save_marks_draft, submit_marks,
        get_report_card, class_student_ids, list_fee_dues,
        record_payment, list_payments, create_request, cancel_request, list_requests, get_request,
        decide_request, dashboard_principal, dashboard_accountant, dashboard_teacher, set_accent,
        verify_audit_chain,
        list_staff_access, add_staff, suspend_staff, remove_staff, create_invite, revoke_invite,
        set_class_teacher, assign_subject_teacher, effective_access, list_devices, revoke_device,
        server_status, sync_status, sync_now, list_conflicts, resolve_conflict, list_review_flags,
        resolve_review_flag
    ]
}
