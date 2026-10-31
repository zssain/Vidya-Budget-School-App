//! Client runtime (Phase 20, join bootstrap): a device that JOINS a school must end up at
//! its teacher home, not the welcome screen. This proves the whole chain end-to-end WITHOUT
//! a device:
//!   1. the server builds a role-scoped snapshot for the joining teacher (`scope::snapshot`),
//!   2. a fresh client applies it (`engine::apply_snapshot`) — the school/session/classes/…
//!      that setup created with direct writes (never op-logged) land locally, and
//!   3. with a stored client identity the device asks for its OWN local PIN (`PinSetup`),
//!      then starts `Locked` on every launch and unlocks only with that PIN.
//!
//! If any link breaks (snapshot omits the school row, apply drops columns, compute ignores
//! the client identity, or a joined device is auto-signed-in) this test fails.

use time::OffsetDateTime;

use vidya_lib::state::{compute, AppState};
use vidya_lib::sync::apply::load_actor;
use vidya_lib::sync::client::{self, ClientIdentity};
use vidya_lib::server::service;
use vidya_lib::sync::engine::apply_snapshot;
use vidya_lib::sync::protocol::{Change, Snapshot, SnapshotManifest};
use vidya_lib::sync::scope;
use vidya_lib::{db, seed};

const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// The school PC's manifest for `changes` (as `/v1/sync/snapshot` would send).
fn manifest_for(server: &rusqlite::Connection, changes: &[Change]) -> SnapshotManifest {
    service::snapshot_of(server, changes.to_vec()).unwrap().manifest
}

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

    let n = apply_snapshot(&mut client, &manifest_for(&server, &changes), &changes).unwrap();
    assert_eq!(n, changes.len(), "EVERY snapshot row applied (all or nothing)");

    // The school's data is now local.
    let school_rows: i64 = client.query_row("SELECT COUNT(*) FROM school", [], |r| r.get(0)).unwrap();
    assert_eq!(school_rows, 1, "the school row landed locally");
    let class_rows: i64 = client.query_row("SELECT COUNT(*) FROM class", [], |r| r.get(0)).unwrap();
    assert!(class_rows >= 1, "the teacher's classes landed locally");

    // Persist the client identity (as join does) → the device must first create its own PIN.
    client::store(&client, &identity_for("stf-meena")).unwrap();
    match compute(&client, None).unwrap().state {
        AppState::PinSetup { staff } => assert_eq!(staff.id, "stf-meena"),
        other => panic!("joined client must ask for a device PIN, got {other:?}"),
    }
    // With a PIN set, a cold start is Locked; the joined teacher's session unlocks it
    // (the bootstrap above completed, so it goes straight home).
    vidya_lib::security::device_pin::create(&client, "4821").unwrap();
    assert!(matches!(compute(&client, None).unwrap().state, AppState::Locked));
    vidya_lib::security::device_pin::verify(&client, "4821", 1_000).unwrap();
    let me = vidya_lib::state::SessionStaff { id: "stf-meena".into(), name: "Meena".into(), role: "teacher".into() };
    match compute(&client, Some(me)).unwrap().state {
        AppState::Unlocked { staff } => assert_eq!(staff.id, "stf-meena"),
        other => panic!("unlocked joined client should reach its home, got {other:?}"),
    }
}

#[test]
fn joined_client_is_never_auto_signed_in() {
    // A client identity but no school data yet: the device is NOT NoSchool (welcome) and is
    // NOT signed in — the join token authenticates the device, never a human session.
    let mut client = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut client).unwrap();
    client::store(&client, &identity_for("stf-meena")).unwrap();
    match compute(&client, None).unwrap().state {
        AppState::PinSetup { staff } => assert_eq!(staff.id, "stf-meena"),
        other => panic!("joined client must not be auto-unlocked, got {other:?}"),
    }
}

#[test]
fn server_without_client_identity_is_unchanged() {
    // Regression: a device with NO client identity keeps the original (server) behaviour.
    let mut server = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut server).unwrap();
    assert!(matches!(compute(&server, None).unwrap().state, AppState::NoSchool));
}

/// Every role's real snapshot applies completely under the strict rule (a role whose
/// filtered rows couldn't insert would block its devices from ever finishing join).
#[test]
fn every_role_snapshot_applies_completely() {
    let mut server = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut server).unwrap();
    seed::seed_demo_school(&mut server, OffsetDateTime::now_utc()).unwrap();
    let staff: Vec<String> = {
        let mut st = server.prepare("SELECT id FROM staff WHERE state='active'").unwrap();
        st.query_map([], |r| r.get(0)).unwrap().collect::<Result<_, _>>().unwrap()
    };
    for id in staff {
        let actor = load_actor(&server, &id).unwrap().unwrap();
        let changes = scope::snapshot(&server, &actor).unwrap();
        let mut client = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut client).unwrap();
        let n = apply_snapshot(&mut client, &manifest_for(&server, &changes), &changes).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(n, changes.len(), "{id}");
        assert!(vidya_lib::sync::engine::bootstrap_complete(&client), "{id}");
    }
}

/// A snapshot with one bad row leaves the device untouched (no partial school that
/// looks ready) and not marked complete, so the sync loop retries.
#[test]
fn a_corrupt_snapshot_is_rolled_back_entirely() {
    let mut server = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut server).unwrap();
    seed::seed_demo_school(&mut server, OffsetDateTime::now_utc()).unwrap();
    let actor = load_actor(&server, "stf-meena").unwrap().unwrap();
    let mut changes = scope::snapshot(&server, &actor).unwrap();
    let mut bad = changes[changes.len() / 2].clone();
    bad.payload = serde_json::json!({ "id": "x", "no_such_column": 1 });
    changes.push(bad);

    let mut client = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut client).unwrap();
    // (Manifest built over the bad rows too, so it is the ROW that fails here.)
    assert!(apply_snapshot(&mut client, &manifest_for(&server, &changes), &changes).is_err());
    let schools: i64 = client.query_row("SELECT COUNT(*) FROM school", [], |r| r.get(0)).unwrap();
    assert_eq!(schools, 0, "nothing from the failed snapshot remains");
    assert!(!vidya_lib::sync::engine::bootstrap_complete(&client));
    // A snapshot without the school row is refused too.
    let no_school: Vec<_> = scope::snapshot(&server, &actor).unwrap().into_iter().filter(|c| c.table != "school").collect();
    assert!(apply_snapshot(&mut client, &manifest_for(&server, &no_school), &no_school).is_err());
}

/// G1 manifest: the device checks counts + digest before committing. A dropped row,
/// an altered row, a swapped-in extra row, a missing manifest or an older-epoch
/// snapshot are each refused with NOTHING applied and `bootstrap_complete` unset; the
/// next (intact) fetch — the sync loop's self-heal — then applies.
#[test]
fn a_snapshot_that_does_not_match_its_manifest_is_refused_then_self_heals() {
    let mut server = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut server).unwrap();
    seed::seed_demo_school(&mut server, OffsetDateTime::now_utc()).unwrap();
    server.execute(
        "INSERT INTO device(id,staff_id,platform,name,token_hash) VALUES ('dev-phone-1','stf-meena','android','Phone','h')",
        [],
    )
    .unwrap();
    let auth = service::DeviceAuth { device_id: "dev-phone-1".into(), staff_id: "stf-meena".into() };
    let snap: Snapshot = service::snapshot_with_manifest(&server, &auth).unwrap();
    let m = &snap.manifest;
    assert_eq!(m.row_count, snap.changes.len());
    assert_eq!(m.tables.values().sum::<usize>(), snap.changes.len());
    assert_eq!(m.tables.get("school"), Some(&1));
    assert!(!m.snapshot_id.is_empty() && m.digest.len() == 64);
    let cursor: i64 = server.query_row("SELECT COALESCE(MAX(server_seq),0) FROM op_log", [], |r| r.get(0)).unwrap();
    assert_eq!((m.cursor, m.server_epoch), (cursor, 1), "bound to the server's cursor + epoch");
    // The wire body round-trips (what the transport parses).
    let wire: Snapshot = serde_json::from_str(&serde_json::to_string(&snap).unwrap()).unwrap();
    assert_eq!(wire, snap);
    // Row order doesn't matter (the digest is over the set).
    let mut reversed = snap.changes.clone();
    reversed.reverse();
    assert!(m.verify(&reversed).is_ok());

    let mut client = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut client).unwrap();
    let untouched = |c: &rusqlite::Connection| {
        let n: i64 = c.query_row("SELECT COUNT(*) FROM school", [], |r| r.get(0)).unwrap();
        n == 0 && !vidya_lib::sync::engine::bootstrap_complete(c)
    };

    // (a) a row dropped in transit
    let mut dropped = snap.changes.clone();
    let i = dropped.iter().position(|c| c.table == "class").unwrap();
    dropped.remove(i);
    let e = apply_snapshot(&mut client, m, &dropped).unwrap_err();
    assert!(e.contains("count"), "{e}");
    assert!(untouched(&client));
    // (b) a row altered (same counts) → digest
    let mut altered = snap.changes.clone();
    let j = altered.iter().position(|c| c.table == "student").unwrap();
    altered[j].payload["name"] = serde_json::json!("Someone Else");
    let e = apply_snapshot(&mut client, m, &altered).unwrap_err();
    assert!(e.contains("digest"), "{e}");
    assert!(untouched(&client));
    // (c) a row swapped for another table's row (total equal, per-table differs)
    let mut swapped = snap.changes.clone();
    swapped[i] = swapped[j].clone();
    assert!(apply_snapshot(&mut client, m, &swapped).unwrap_err().contains("table counts"));
    assert!(untouched(&client));
    // (d) no manifest at all (Default) / tampered header fields
    assert!(apply_snapshot(&mut client, &SnapshotManifest::default(), &snap.changes).is_err());
    let mut wrong_cursor = m.clone();
    wrong_cursor.cursor += 1;
    assert!(apply_snapshot(&mut client, &wrong_cursor, &snap.changes).is_err());
    assert!(untouched(&client));
    // (e) a manifest from an older epoch than this device trusts
    vidya_lib::kv::set(&client, "known_epoch", &5i64).unwrap();
    assert!(apply_snapshot(&mut client, m, &snap.changes).unwrap_err().contains("EPOCH_OLD"));
    assert!(untouched(&client));
    vidya_lib::kv::set(&client, "known_epoch", &1i64).unwrap();

    // Self-heal: the intact snapshot then applies completely and is recorded.
    let n = vidya_lib::sync::client::apply_bootstrap(&mut client, &snap).unwrap();
    assert_eq!(n, snap.changes.len());
    assert!(vidya_lib::sync::engine::bootstrap_complete(&client));
    let held: serde_json::Value = vidya_lib::kv::get(&client, "bootstrap_snapshot").unwrap().unwrap();
    assert_eq!(held["snapshot_id"], serde_json::json!(m.snapshot_id));
    assert_eq!(held["cursor"], serde_json::json!(m.cursor));
}
