//! End-to-end flow tests (prompts/P03 "DONE MEANS"), driven through the REAL
//! command-logic layer against a real encrypted (in-memory) SQLCipher DB.
//!
//! A Tauri-window Playwright e2e cannot run on macOS (tauri-driver supports Linux
//! and Windows only), so these exercise the same command functions the Tauri
//! wrappers call — the honest, portable proof that the flows work.
//!
//! Flows: (1) fresh setup → PIN → unlocked (state machine); (2) record a payment →
//! real receipt, "collected today" updates, audit valid; (3) attendance →
//! correction request → Principal approves → mark changed + audit; (4) payment
//! reversal request → approve → reversal row + audit.

use rusqlite::Connection;
use time::OffsetDateTime;

use vidya_lib::commands::logic::*;
use vidya_lib::state::{self, AppState, SessionStaff, KV_PENDING_LICENCE};
use vidya_lib::write::DeviceMode;
use vidya_lib::{dash, db, kv, seed, security};

const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn fresh() -> Connection {
    let mut c = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut c).unwrap();
    c
}

fn seeded() -> Connection {
    let mut c = fresh();
    let now = OffsetDateTime::parse("2026-09-23T09:00:00Z", &time::format_description::well_known::Rfc3339).unwrap();
    seed::seed_demo_school(&mut c, now).unwrap();
    c
}

fn session(id: &str, role: &str) -> SessionStaff {
    SessionStaff { id: id.into(), name: id.into(), role: role.into() }
}

/// Flow 1 — fresh install → setup wizard → PIN → unlocked.
#[test]
fn flow_setup_to_unlocked() {
    let mut c = fresh();

    // Fresh: no school, no pending licence → no_school.
    assert_eq!(state::compute(&c, None).unwrap().state, AppState::NoSchool);

    // Activation is verified elsewhere (parse_activate tests); simulate the
    // pending licence it stores → activated.
    kv::set(&c, KV_PENDING_LICENCE, &serde_json::json!({
        "licence_id":"lic_x","school_id":"sch_x","plan":"perpetual",
        "issued_at":"2026-09-23T00:00:00Z","signature":"sig","raw_json":"raw"
    })).unwrap();
    assert_eq!(state::compute(&c, None).unwrap().state, AppState::Activated);

    // Wizard steps.
    setup_school_logic(&mut c, &SchoolInput {
        name: "Test School".into(), address: Some("A".into()), board: Some("CBSE".into()),
        udise: None, phone: Some("9820011111".into()),
    }).unwrap();
    // School row now exists → setup_in_progress; the licence moved out of pending.
    assert!(matches!(state::compute(&c, None).unwrap().state, AppState::SetupInProgress { .. }));
    let lic: i64 = c.query_row("SELECT COUNT(*) FROM licence", [], |r| r.get(0)).unwrap();
    assert_eq!(lic, 1, "pending licence moved into the licence table");

    setup_session_logic(&mut c, &SessionInput {
        label: "2026–27".into(), starts_on: "2026-04-01".into(), ends_on: "2027-03-31".into(),
        term1_starts: "2026-04-01".into(), term1_ends: "2026-09-30".into(),
        term2_starts: "2026-10-01".into(), term2_ends: "2027-03-31".into(),
    }).unwrap();
    setup_classes_logic(&mut c, &[ClassInput { name: "V".into(), section: Some("A".into()), display: "V-A".into() }]).unwrap();
    setup_principal_logic(&mut c, "Priya Sharma", "9820011111").unwrap();
    create_pin_logic(&mut c, "4321").unwrap();

    // Setup complete → locked (no session).
    assert_eq!(state::compute(&c, None).unwrap().state, AppState::Locked);

    // Unlock with the right PIN.
    let pid: String = c.query_row("SELECT id FROM staff WHERE role='principal'", [], |r| r.get(0)).unwrap();
    let staff = unlock_logic(&mut c, &pid, "4321").unwrap();
    assert_eq!(staff.role, "principal");
    assert_eq!(state::compute(&c, Some(staff.clone())).unwrap().state, AppState::Unlocked { staff });

    // Wrong PIN is rejected.
    assert_eq!(unlock_logic(&mut c, &pid, "0000").unwrap_err().code, "PIN_WRONG");
}

/// Flow 2 — record a payment: real receipt, "collected today" updates, audit valid.
#[test]
fn flow_record_payment_updates_dashboard_and_audit() {
    let mut c = seeded();
    let before = dash::principal_dashboard(&c, "2026-09-23").unwrap();
    assert_eq!(before.collected_today_paise, 4_850_000);
    assert_eq!(before.receipts_today, 23);

    let acc = session("stf-suresh", "accountant");
    let dto = record_payment_logic(
        &mut c, &acc, Some("dev-a2"), DeviceMode::Server,
        &PaymentInput { student_id: "stu-kavya-singh".into(), amount_paise: 310_000, mode: "upi".into(), reference: Some("426518903214".into()) },
    ).unwrap();
    assert_eq!(dto.receipt_no, "R-A2-0419");
    assert!(dto.confirmed);

    let after = dash::principal_dashboard(&c, "2026-09-23").unwrap();
    assert_eq!(after.collected_today_paise, 4_850_000 + 310_000, "collected today grew by ₹3,100");
    assert_eq!(after.receipts_today, 24);

    // Kavya is now settled; the audit chain still verifies.
    assert_eq!(list_fee_dues_logic(&mut c, "stu-kavya-singh").unwrap().total_due_paise, 0);
    assert_eq!(security::audit::verify_chain(&c).unwrap(), None);
}

/// Flow 3 — attendance correction: teacher requests, Principal approves, the mark
/// changes and the audit chain records it.
#[test]
fn flow_attendance_correction_approved_changes_register() {
    let mut c = seeded();
    // Pick a V-A student currently marked Absent.
    let (mark_id, student_id): (String, String) = c
        .query_row(
            "SELECT m.id, m.student_id FROM attendance_mark m \
             JOIN attendance_sheet s ON s.id=m.sheet_id WHERE s.class_id='cls-5a' AND m.mark='A' LIMIT 1",
            [], |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();

    // Teacher (Meena, class teacher of V-A) raises an attendance_correction A→P.
    let meena = session("stf-meena", "teacher");
    let req = create_request_logic(&mut c, &meena, &RequestInput {
        kind: "attendance_correction".into(),
        target_table: "attendance_mark".into(),
        target_id: mark_id.clone(),
        base_version: 1,
        reason: "Was present".into(),
        before_json: Some(serde_json::json!({ "value": "A" }).to_string()),
        after_json: Some(serde_json::json!({ "value": "P", "summary": "V-A Absent → Present" }).to_string()),
    }).unwrap();
    assert_eq!(req.status, "pending");

    // A duplicate request on the same mark is rejected.
    assert_eq!(
        create_request_logic(&mut c, &meena, &RequestInput {
            kind: "attendance_correction".into(), target_table: "attendance_mark".into(), target_id: mark_id.clone(),
            base_version: 1, reason: "again".into(), before_json: None, after_json: None,
        }).unwrap_err().code,
        "REQUEST_ALREADY_PENDING"
    );

    // Principal approves → the mark flips to Present, request applied, audit valid.
    let priya = session("stf-priya", "principal");
    let decided = decide_request_logic(&mut c, &priya, DeviceMode::Server, &req.id, "approve", None).unwrap();
    assert_eq!(decided.status, "approved");
    assert_eq!(decided.apply_state, "applied");

    let mark: String = c.query_row("SELECT mark FROM attendance_mark WHERE id=?1", [&mark_id], |r| r.get(0)).unwrap();
    assert_eq!(mark, "P", "register changed for {student_id}");
    assert_eq!(security::audit::verify_chain(&c).unwrap(), None);

    // A teacher may NOT approve requests.
    let req2 = create_request_logic(&mut c, &meena, &RequestInput {
        kind: "attendance_correction".into(), target_table: "attendance_mark".into(),
        target_id: "some-other".into(), base_version: 1, reason: "x".into(),
        before_json: None, after_json: Some(serde_json::json!({ "value": "P" }).to_string()),
    }).unwrap();
    assert_eq!(
        decide_request_logic(&mut c, &meena, DeviceMode::Server, &req2.id, "approve", None).unwrap_err().code,
        "FORBIDDEN"
    );
}

/// Flow 4 — payment reversal request → approve → a reversal row is appended.
#[test]
fn flow_payment_reversal_appends_reversal() {
    let mut c = seeded();
    let payment_id: String = c.query_row("SELECT id FROM payment WHERE receipt_no='R-A2-0300'", [], |r| r.get(0)).unwrap();

    let acc = session("stf-suresh", "accountant");
    let req = create_request_logic(&mut c, &acc, &RequestInput {
        kind: "payment_reversal".into(),
        target_table: "payment".into(),
        target_id: payment_id.clone(),
        base_version: 1,
        reason: "entered twice".into(),
        before_json: None,
        after_json: Some(serde_json::json!({ "summary": "reversal" }).to_string()),
    }).unwrap();

    let priya = session("stf-priya", "principal");
    decide_request_logic(&mut c, &priya, DeviceMode::Server, &req.id, "approve", Some("ok")).unwrap();

    let reversals: i64 = c
        .query_row("SELECT COUNT(*) FROM reversal WHERE payment_id=?1", [&payment_id], |r| r.get(0))
        .unwrap();
    assert_eq!(reversals, 1, "an approved reversal appends a reversal row");
    assert_eq!(security::audit::verify_chain(&c).unwrap(), None);
}
