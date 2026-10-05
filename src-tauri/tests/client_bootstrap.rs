//! Client runtime (Phase 20, join bootstrap): a device that JOINS a school must end up at
//! its teacher home, not the welcome screen. This proves the whole chain end-to-end WITHOUT
//! a device:
//!   1. the server builds a role-scoped snapshot for the joining teacher (`scope::snapshot`),
//!   2. a fresh client applies it (`engine::apply_snapshot`) — the school/session/classes/…
//!      that setup created with direct writes (never op-logged) land locally, and
//!   3. with a stored client identity, `state::compute` reports `Unlocked{teacher}` so the
//!      app routes to the teacher home.
//! If any link breaks (snapshot omits the school row, apply drops columns, compute ignores
//! the client identity), a joined device silently falls back to welcome — this is the guard.

use time::OffsetDateTime;

use vidya_lib::state::{compute, AppState};
use vidya_lib::sync::apply::load_actor;
use vidya_lib::sync::client::{self, ClientIdentity};
use vidya_lib::sync::engine::apply_snapshot;
use vidya_lib::sync::scope;
use vidya_lib::{db, seed};

const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// A client identity for the joined teacher (as `apply_join_response` would persist).
fn identity_for(staff_id: &str) -> ClientIdentity {
    ClientIdentity {
        device_id: "dev-phone-1".into(),
        device_token: "tok".into(),
        staff_id: staff_id.into(),
        staff_name: "Meena".into(),
        staff_role: "teacher".into(),
        session_key: String::new(),
        school_id: "sch-demo".into(),
        school_name: "Demo".into(),
        receipt_series: "R".into(),
        admission_series: "A".into(),
        audience_keys: vec![],
        server_fingerprint: "fp".into(),
        server_addrs: vec!["192.168.0.188".into()],
        server_port: 47650,
        server_epoch: 1,
        bootstrap_cursor: 0,
        lease_expires_at: String::new(),
        protocol: 1,
    }
}

#[test]
fn joined_client_bootstraps_data_and_reaches_the_teacher_home() {
    // --- server side: a fully set-up school, then a role-scoped snapshot for the teacher.
    let mut server = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut server).unwrap();
    seed::seed_demo_school(&mut server, OffsetDateTime::now_utc()).unwrap();

    let actor = load_actor(&server, "stf-meena").unwrap().expect("teacher actor");
    let changes = scope::snapshot(&server, &actor).unwrap();
    assert!(!changes.is_empty(), "snapshot should carry the school's data");
    assert!(changes.iter().any(|c| c.table == "school"), "snapshot MUST include the school row");
    assert!(changes.iter().any(|c| c.table == "class"), "snapshot MUST include the teacher's classes");

    // --- client side: a FRESH device (migrated, nothing seeded) applies the snapshot.
    let mut client = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut client).unwrap();

    // Before bootstrap + identity, a fresh device is `no_school` (welcome).
    assert!(matches!(compute(&client, None).unwrap().state, AppState::NoSchool));

    let n = apply_snapshot(&client, &changes).unwrap();
    assert!(n > 0, "snapshot rows applied");

    // The school's data is now local.
    let school_rows: i64 = client.query_row("SELECT COUNT(*) FROM school", [], |r| r.get(0)).unwrap();
    assert_eq!(school_rows, 1, "the school row landed locally");
    let class_rows: i64 = client.query_row("SELECT COUNT(*) FROM class", [], |r| r.get(0)).unwrap();
    assert!(class_rows >= 1, "the teacher's classes landed locally");

    // Persist the client identity (as join does) → compute now reports the teacher home.
    client::store(&client, &identity_for("stf-meena")).unwrap();
    match compute(&client, None).unwrap().state {
        AppState::Unlocked { staff } => assert_eq!(staff.id, "stf-meena"),
        other => panic!("joined client should be Unlocked at its home, got {other:?}"),
    }
}

#[test]
fn joined_client_is_signed_in_even_before_bootstrap_not_welcome() {
    // A client identity but no school data yet (snapshot hasn't landed): the device is still
    // signed in as the joined teacher (Unlocked) — NOT NoSchool (welcome) and NOT the dead PIN
    // screen. The home shows empty until the 15s sync loop self-heals the snapshot.
    let mut client = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut client).unwrap();
    client::store(&client, &identity_for("stf-meena")).unwrap();
    match compute(&client, None).unwrap().state {
        AppState::Unlocked { staff } => assert_eq!(staff.id, "stf-meena"),
        other => panic!("joined client should be Unlocked, got {other:?}"),
    }
}

#[test]
fn server_without_client_identity_is_unchanged() {
    // Regression: a device with NO client identity keeps the original (server) behaviour.
    let mut server = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut server).unwrap();
    assert!(matches!(compute(&server, None).unwrap().state, AppState::NoSchool));
}
