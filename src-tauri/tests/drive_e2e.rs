//! Google Drive exchange harness — device-sealed bundles (format v2, P21).
//!
//! Every device seals its uploads with its OWN session-derived c2s key (AAD bound to
//! its device id). The school PC opens a bundle with the session key of the live
//! device that owns the folder, so opening it PROVES authorship; the op is then
//! applied as that device's staff (`apply_client_op`). Proven here over FakeDrive:
//!   1. a device's bundle is imported and confirmed;
//!   2. a peer can't open another device's bundle (not with its own key, not with an
//!      audience key);
//!   3. tampered, foreign-key, moved-to-another-folder, legacy audience-sealed and
//!      revoked-device bundles are all quarantined, never applied;
//!   4. staff can never delete `backups/` and write only to their own ops folder.

use std::collections::BTreeMap;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rusqlite::{params, Connection};
use time::OffsetDateTime;
use vidya_lib::sync::drive::bundle;
use vidya_lib::sync::drive::exchange;
use vidya_lib::sync::drive::fake::{Access, FakeDrive, PRINCIPAL};
use vidya_lib::sync::drive::{DriveApi, DriveError};
use vidya_lib::sync::protocol::Op;
use vidya_lib::sync::seal;
use vidya_lib::{db, seed};

const DBKEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn now() -> OffsetDateTime {
    OffsetDateTime::parse("2026-09-23T12:00:00Z", &time::format_description::well_known::Rfc3339).unwrap()
}

fn server() -> Connection {
    let mut c = db::open_in_memory(DBKEY).unwrap();
    db::run_migrations(&mut c).unwrap();
    seed::seed_demo_school(&mut c, now()).unwrap();
    c
}

fn phone() -> Connection {
    let mut c = db::open_in_memory(DBKEY).unwrap();
    db::run_migrations(&mut c).unwrap();
    c
}

/// Register a device on the server (as a join would) with a session key; returns it.
fn register(s: &Connection, device: &str, staff: &str, key_byte: u8) -> String {
    let sk = STANDARD.encode([key_byte; 32]);
    s.execute(
        "INSERT INTO device(id,staff_id,platform,name,token_hash,session_key) VALUES (?1,?2,'android','Phone','h',?3)",
        params![device, staff, sk],
    )
    .unwrap();
    sk
}

fn queue(dev: &Connection, o: &Op) {
    dev.execute(
        "INSERT INTO outbox(op_id,hlc,device_id,staff_id,audience,\"table\",record_id,kind,payload,base_version,server_epoch) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![o.op_id, o.hlc, o.device_id, o.staff_id, o.audience, o.table, o.record_id, o.kind, o.payload.to_string(), o.base_version, o.server_epoch],
    ).unwrap();
}

/// Meena (class teacher of cls-5a) marks a student Present on an OPEN sheet.
fn meena_mark_op(s: &Connection, op_id: &str, device: &str) -> (Op, String) {
    let mark_id: String = s
        .query_row(
            "SELECT m.id FROM attendance_mark m JOIN attendance_sheet sh ON sh.id=m.sheet_id WHERE sh.class_id='cls-5a' AND m.mark='A' LIMIT 1",
            [], |r| r.get(0),
        )
        .unwrap();
    s.execute("UPDATE attendance_sheet SET status='draft' WHERE id=(SELECT sheet_id FROM attendance_mark WHERE id=?1)", [&mark_id]).unwrap();
    let op = Op {
        op_id: op_id.into(),
        hlc: format!("0000000000001000{op_id}"),
        device_id: device.into(),
        staff_id: "stf-meena".into(),
        audience: "class:cls-5a".into(),
        table: "attendance_mark".into(),
        record_id: mark_id.clone(),
        kind: "update".into(),
        payload: serde_json::json!({ "mark": "P" }),
        base_version: None,
        server_epoch: 1,
    };
    (op, mark_id)
}

#[test]
fn a_device_sealed_bundle_is_imported_and_confirmed() {
    let mut s = server();
    let sk = register(&s, "dev-phone", "stf-meena", 0x31);
    let (op, mark_id) = meena_mark_op(&s, "op-d1", "dev-phone");
    let mut ph = phone();
    queue(&ph, &op);

    let drive = FakeDrive::new();
    let l = drive.provision_school(&[("stf-meena", "dev-phone")]);
    let pushed = exchange::push_outbox(&mut ph, &drive.as_actor("stf-meena"), &l.ops["dev-phone"], "dev-phone", &sk).unwrap();
    assert_eq!((pushed.bundles, pushed.ops), (1, 1));
    let files = drive.as_actor(PRINCIPAL).list(&l.ops["dev-phone"]).unwrap();
    assert_eq!(files[0].properties.get(bundle::PROP_FORMAT).map(String::as_str), Some(bundle::FORMAT_DEVICE_V2));
    // Re-pushing before the server has picked it up is retry-safe (same name → skipped).
    let again = exchange::push_outbox(&mut ph, &drive.as_actor("stf-meena"), &l.ops["dev-phone"], "dev-phone", &sk).unwrap();
    assert_eq!(again.bundles, 0);

    let out = exchange::import_all(&mut s, &drive.as_actor(PRINCIPAL), &l.exchange).unwrap();
    assert_eq!((out.confirmed, out.quarantined), (1, 0));
    let mark: String = s.query_row("SELECT mark FROM attendance_mark WHERE id=?1", [&mark_id], |r| r.get(0)).unwrap();
    assert_eq!(mark, "P");
    assert_eq!(out.acks["dev-phone"].results[0].op_id, "op-d1");
}

#[test]
fn a_peer_cannot_open_another_devices_bundle() {
    let s = server();
    let sk_a = register(&s, "dev-a", "stf-meena", 0x41);
    let sk_b = register(&s, "dev-b", "stf-meena", 0x42);
    let (op, _) = meena_mark_op(&s, "op-p1", "dev-a");
    let body = bundle::seal_device_bundle(&seal::derive_keys(&sk_a).unwrap().c2s, "dev-a", std::slice::from_ref(&op)).unwrap();
    // The author's own key opens it…
    assert!(bundle::open_device_bundle(&seal::derive_keys(&sk_a).unwrap().c2s, "dev-a", &body).is_ok());
    // …a peer's key does not, in either direction, nor does any audience key.
    let kb = seal::derive_keys(&sk_b).unwrap();
    assert!(bundle::open_device_bundle(&kb.c2s, "dev-a", &body).is_err());
    assert!(bundle::open_device_bundle(&kb.s2c, "dev-a", &body).is_err());
    assert!(bundle::open_bundle(&[0x22; 32], "class:cls-5a", 1, &body).is_err());
    // A device bundle can only carry the device's own ops.
    let mut foreign = op.clone();
    foreign.device_id = "dev-b".into();
    assert!(bundle::seal_device_bundle(&seal::derive_keys(&sk_a).unwrap().c2s, "dev-a", &[foreign]).is_err());
}

#[test]
fn forged_moved_legacy_and_revoked_bundles_are_quarantined() {
    let mut s = server();
    let sk = register(&s, "dev-phone", "stf-meena", 0x51);
    let sk_other = register(&s, "dev-other", "stf-meena", 0x52);
    let _ = register(&s, "dev-revoked", "stf-meena", 0x53);
    s.execute("UPDATE device SET revoked_at='2026-09-01T00:00:00Z' WHERE id='dev-revoked'", []).unwrap();
    let (op, mark_id) = meena_mark_op(&s, "op-q1", "dev-phone");

    let drive = FakeDrive::new();
    let l = drive.provision_school(&[("stf-meena", "dev-phone"), ("stf-meena", "dev-other"), ("stf-meena", "dev-revoked")]);
    // Everyone signs in as the one school account in reality, so an attacker can write
    // into any folder — model that with the owner account.
    let acct = drive.as_actor(PRINCIPAL);
    let props = bundle::device_bundle_properties();

    // (a) sealed with a key that isn't dev-phone's, dropped into dev-phone's folder.
    let wrong = bundle::seal_device_bundle(&[0x99; 32], "dev-phone", std::slice::from_ref(&op)).unwrap();
    acct.create(&l.ops["dev-phone"], "a-dev-v2.vop", &wrong, &props).unwrap();
    // (b) dev-other's genuine bundle (claiming dev-phone's ops is impossible) moved
    //     into dev-phone's folder: the AAD binds dev-other, so it won't open there.
    let mut other_op = op.clone();
    other_op.device_id = "dev-other".into();
    other_op.op_id = "op-q2".into();
    let moved = bundle::seal_device_bundle(&seal::derive_keys(&sk_other).unwrap().c2s, "dev-other", &[other_op]).unwrap();
    acct.create(&l.ops["dev-phone"], "b-dev-v2.vop", &moved, &props).unwrap();
    // (c) a tampered genuine bundle.
    let mut tampered = bundle::seal_device_bundle(&seal::derive_keys(&sk).unwrap().c2s, "dev-phone", std::slice::from_ref(&op)).unwrap();
    let last = tampered.len() - 1;
    tampered[last] ^= 0x01;
    acct.create(&l.ops["dev-phone"], "c-dev-v2.vop", &tampered, &props).unwrap();
    // (d) a legacy audience-sealed bundle (authorship unprovable).
    let legacy = bundle::seal_bundle(&[0x22; 32], "class:cls-5a", 1, std::slice::from_ref(&op)).unwrap();
    acct.create(&l.ops["dev-phone"], "d-class-v1.vop", &legacy, &bundle::bundle_properties("class:cls-5a", 1)).unwrap();
    // (e) a revoked device's otherwise-valid bundle.
    let mut rev_op = op.clone();
    rev_op.device_id = "dev-revoked".into();
    rev_op.op_id = "op-q3".into();
    let rev = bundle::seal_device_bundle(&seal::derive_keys(&STANDARD.encode([0x53u8; 32])).unwrap().c2s, "dev-revoked", &[rev_op]).unwrap();
    acct.create(&l.ops["dev-revoked"], "e-dev-v2.vop", &rev, &props).unwrap();

    let out = exchange::import_all(&mut s, &acct, &l.exchange).unwrap();
    assert_eq!((out.confirmed, out.quarantined), (0, 5), "every forged/moved/legacy/revoked bundle quarantined");
    let mark: String = s.query_row("SELECT mark FROM attendance_mark WHERE id=?1", [&mark_id], |r| r.get(0)).unwrap();
    assert_eq!(mark, "A", "nothing applied");
}

#[test]
fn staff_cannot_delete_backups_through_the_drive_api() {
    // DONE MEANS: "Staff cannot delete backups/ via API or Drive UI."
    let drive = FakeDrive::new();
    let l = drive.provision_school(&[("teacherA", "devA")]);
    let backup = drive
        .as_actor(PRINCIPAL)
        .create(&l.backups, "2026-09-24.vidbak", b"BACKUP", &BTreeMap::new())
        .unwrap();

    let a = drive.as_actor("teacherA");
    assert_eq!(a.delete(&backup.id), Err(DriveError::PermissionDenied));
    assert_eq!(a.list(&l.backups), Err(DriveError::PermissionDenied));
    assert_eq!(
        a.create(&l.backups, "evil.vidbak", b"x", &BTreeMap::new()),
        Err(DriveError::PermissionDenied)
    );
    // The Principal (owner) still can manage backups.
    assert!(drive.as_actor(PRINCIPAL).delete(&backup.id).is_ok());
}

#[test]
fn actor_writes_are_confined_to_their_own_ops_folder() {
    let drive = FakeDrive::new();
    let l = drive.provision_school(&[("teacherA", "devA"), ("teacherB", "devB")]);
    let a = drive.as_actor("teacherA");
    // A can write to its own folder...
    assert!(a.create(&l.ops["devA"], "mine.vop", b"1", &BTreeMap::new()).is_ok());
    // ...but never into teacherB's folder, even though it can read it.
    assert!(a.list(&l.ops["devB"]).is_ok());
    assert_eq!(
        a.create(&l.ops["devB"], "intrude.vop", b"1", &BTreeMap::new()),
        Err(DriveError::PermissionDenied)
    );
}



// Access is part of the public sharing API used by Step 2 provisioning; touch it
// so a compile-time break in that enum surfaces here too.
#[allow(dead_code)]
fn _access_is_ordered() {
    assert!(Access::Owner > Access::Writer && Access::Writer > Access::Reader);
}

/// The school PC's per-device down export: the device's complete role-scoped data,
/// sealed to that device only; rewritten only when the school data moved.
#[test]
fn down_export_reaches_only_its_device_and_tracks_changes() {
    let s = server();
    let sk = register(&s, "dev-ipad", "stf-meena", 0x71);
    let sk_other = register(&s, "dev-other", "stf-suresh", 0x72);
    let drive = FakeDrive::new();
    let l = drive.provision_school(&[]);
    let acct = drive.as_actor(PRINCIPAL);

    let n = exchange::write_down_exports(&s, &acct, &l.exchange).unwrap();
    assert_eq!(n, 2, "one export per live device");
    let mine = exchange::read_down_export(&acct, &l.exchange, "dev-ipad", &sk).unwrap().expect("export");
    assert!(mine.changes.iter().any(|c| c.table == "school"));
    assert!(mine.changes.iter().any(|c| c.table == "class"), "the teacher's classes");
    assert!(mine.changes.iter().all(|c| c.table != "payment"), "a teacher's export carries no fee data");
    // Another device's key can't open it (wrong key or relabelled as the other device).
    assert!(exchange::read_down_export(&acct, &l.exchange, "dev-ipad", &sk_other).is_err());

    // Unchanged data → nothing rewritten.
    assert_eq!(exchange::write_down_exports(&s, &acct, &l.exchange).unwrap(), 0);
    // A server-side change → fresh exports with a newer cursor.
    s.execute(
        "INSERT INTO op_log(op_id,hlc,device_id,staff_id,\"table\",record_id,kind,payload,applied_at) VALUES ('op-x','h','d','stf-priya','class','cls-5a','update','{}','t')",
        [],
    )
    .unwrap();
    assert_eq!(exchange::write_down_exports(&s, &acct, &l.exchange).unwrap(), 2);
    let again = exchange::read_down_export(&acct, &l.exchange, "dev-ipad", &sk).unwrap().unwrap();
    assert!(again.cursor > mine.cursor);
    // Revoked → no more exports for that device, and its stale export is taken down.
    assert!(exchange::read_down_export(&acct, &l.exchange, "dev-other", &sk_other).unwrap().is_some());
    let mut s = s;
    vidya_lib::server::service::revoke_device(&mut s, "dev-other", now()).unwrap();
    s.execute(
        "INSERT INTO op_log(op_id,hlc,device_id,staff_id,\"table\",record_id,kind,payload,applied_at) VALUES ('op-y','h','d','stf-priya','class','cls-5a','update','{}','t')",
        [],
    )
    .unwrap();
    assert_eq!(exchange::write_down_exports(&s, &acct, &l.exchange).unwrap(), 1);
    assert!(exchange::read_down_export(&acct, &l.exchange, "dev-other", &sk_other).unwrap().is_none());
    assert!(exchange::read_down_export(&acct, &l.exchange, "dev-ipad", &sk).unwrap().is_some());
}
