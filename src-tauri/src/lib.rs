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
pub mod state;
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

    let (db, key_missing) = match security::keys::ensure_key(key_store.as_ref(), db_exists) {
        Ok(key) => {
            let hex = security::keys::to_hex(&key);
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
        key_missing: Mutex::new(key_missing),
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
        list_staff, list_classes, list_students, search_students, get_student, create_student,
        get_attendance_sheet, save_attendance_draft, submit_attendance, list_fee_dues,
        record_payment, list_payments, create_request, cancel_request, list_requests, get_request,
        decide_request, dashboard_principal, dashboard_accountant, dashboard_teacher, set_accent,
        verify_audit_chain, seed_demo_school
    ]
}

#[cfg(not(debug_assertions))]
fn invoke_handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    use commands::*;
    tauri::generate_handler![
        app_state, activate_licence, setup_school, setup_session, setup_classes, setup_principal,
        create_recovery_key, confirm_recovery_key, create_pin, unlock, lock, switch_user,
        list_staff, list_classes, list_students, search_students, get_student, create_student,
        get_attendance_sheet, save_attendance_draft, submit_attendance, list_fee_dues,
        record_payment, list_payments, create_request, cancel_request, list_requests, get_request,
        decide_request, dashboard_principal, dashboard_accountant, dashboard_teacher, set_accent,
        verify_audit_chain
    ]
}
