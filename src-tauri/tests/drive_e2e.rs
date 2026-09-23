//! Google Drive fallback harness (prompts/P06 Step 9 — offline-provable slice).
//!
//! Composes the REAL P06 modules — `sync::drive::{fake, bundle, keys}` and
//! `vidya_core::audience` — with no Google and no Step-0 spike, to prove the
//! security-critical exchange properties:
//!   1. server off → a device seals its ops into a `.vop` and uploads to its own
//!      Drive folder (temp name → verify-by-readback → rename);
//!   2. a same-audience device downloads + decrypts it (provisional); a different
//!      class / the accountant CANNOT (no key) — the DONE-MEANS isolation core;
//!   3. a tampered bundle a holder *should* read fails AEAD → quarantine;
//!   4. staff can never delete `backups/` through the Drive API.
//!
//! NOT covered here (needs the real DriveApi client + OAuth from Step 1, gated on
//! the spike, and the Step 4–6 push/pull/import engine): the full "server on →
//! import → all phones Confirmed" loop, LAN+Drive op dedup (idempotent by op_id,
//! already proven in `sync_e2e`), and revoked-author flagging on import
//! (`apply.rs` raises `review_flag revoked_author`). See docs/phase-notes/phase-6.md.

use std::collections::BTreeMap;

use rusqlite::{params, Connection};
use vidya_core::audience::{audience_for, Audience};
use vidya_lib::db;
use vidya_lib::sync::drive::bundle::{self, PROP_AUDIENCE, PROP_KEYVER};
use vidya_lib::sync::drive::exchange::{self, HeldKeys};
use vidya_lib::sync::drive::fake::{Access, FakeDrive, PRINCIPAL};
use vidya_lib::sync::drive::{keys, pull};
use vidya_lib::sync::drive::{DriveApi, DriveError};
use vidya_lib::sync::protocol::Op;

const DBKEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// An attendance op for a class (audience decided by vidya-core, as the write
/// path will).
fn attendance_op(n: usize, class_id: &str) -> Op {
    let aud = match audience_for("attendance_mark", Some(class_id)).unwrap() {
        Audience::Class(_) => format!("class:{class_id}"),
        other => other.as_str(),
    };
    Op {
        op_id: format!("op-{class_id}-{n}"),
        hlc: format!("00000000000000000{n:03}devA"),
        device_id: "devA".into(),
        staff_id: "teacherA".into(),
        audience: aud,
        table: "attendance_mark".into(),
        record_id: format!("mark-{n}"),
        kind: "update".into(),
        payload: serde_json::json!({ "student": format!("s{n}"), "mark": "P" }),
        base_version: Some(1),
        server_epoch: 1,
    }
}

/// sha256 hex of a byte slice — matches what the fake Drive returns as `checksum`.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// The server's audience keys (persisted, versioned) for one audience.
fn server_key(conn: &rusqlite::Connection, audience: &str) -> ([u8; 32], i64) {
    let k = keys::get_or_create(conn, audience).unwrap();
    (keys::decode_key(&k.key_b64).unwrap(), k.version)
}

/// Push one device's same-audience ops as a bundle to its Drive folder, exactly
/// as Step 4 does: seal → create under a temp name → verify the readback
/// checksum → rename to the final `.vop` name. Returns the final file id.
fn push_bundle(
    drive: &FakeDrive,
    actor: &str,
    ops_folder: &str,
    key: &[u8; 32],
    audience: &str,
    version: i64,
    ops: &[Op],
) -> String {
    let client = drive.as_actor(actor);
    let body = bundle::seal_bundle(key, audience, version, ops).unwrap();
    let props = bundle::bundle_properties(audience, version);
    // temp name so a half-written file is never seen
    let tmp = client
        .create(ops_folder, ".tmp-upload", &body, &props)
        .unwrap();
    // verify-by-readback: the metadata checksum must match what we uploaded
    let meta = client.metadata(&tmp.id).unwrap();
    assert_eq!(meta.checksum, sha256_hex(&body), "readback checksum mismatch");
    assert_eq!(meta.size, body.len());
    // only now commit to the final name
    let hlc = &ops[0].hlc;
    let final_name = bundle::bundle_filename(hlc, audience, version);
    client.rename(&tmp.id, &final_name).unwrap();
    tmp.id
}

#[test]
fn same_class_device_sees_provisional_other_class_and_accountant_cannot() {
    // Server issues the audience keys (persisted, versioned).
    let mut conn = db::open_in_memory(DBKEY).unwrap();
    db::run_migrations(&mut conn).unwrap();
    let (va_key, va_ver) = server_key(&conn, "class:VI-A");
    let (_vb_key, _vb_ver) = server_key(&conn, "class:VII-B");
    let (_fin_key, _fin_ver) = server_key(&conn, "finance");

    // Devices and their §11 Drive folders.
    let drive = FakeDrive::new();
    let l = drive.provision_school(&[
        ("teacherA", "devA"), // class VI-A
        ("teacherB", "devB"), // class VI-A (same class as A)
        ("teacherC", "devC"), // class VII-B (other class)
        ("accountant", "devAcc"),
    ]);

    // Keys each device HOLDS (delivered for its audiences only).
    // A & B hold class:VI-A; C holds class:VII-B; accountant holds finance.
    // (No device is handed a key for an audience it isn't in.)

    // --- server off: teacherA (devA) records attendance and pushes it ---------
    let ops = vec![attendance_op(1, "VI-A"), attendance_op(2, "VI-A")];
    assert_eq!(ops[0].audience, "class:VI-A"); // audience decided by vidya-core
    let file_id = push_bundle(&drive, "teacherA", &l.ops["devA"], &va_key, "class:VI-A", va_ver, &ops);

    // --- teacherB (same class:VI-A key) sees it as provisional ----------------
    let b = drive.as_actor("teacherB");
    let listed = b.list(&l.ops["devA"]).unwrap();
    let bundle_file = listed.iter().find(|f| f.name.ends_with(".vop")).expect("B sees the bundle");
    assert_eq!(bundle_file.id, file_id);
    // B reads the audience+version from properties WITHOUT downloading
    assert_eq!(bundle_file.properties.get(PROP_AUDIENCE).unwrap(), "class:VI-A");
    assert_eq!(bundle_file.properties.get(PROP_KEYVER).unwrap(), &va_ver.to_string());
    // B holds class:VI-A v1 → can decrypt → provisional ops
    let body = b.download(&bundle_file.id).unwrap();
    let got = bundle::open_bundle(&va_key, "class:VI-A", va_ver, &body).unwrap();
    assert_eq!(got, ops, "same-class device applies the ops provisionally");

    // --- teacherC (other class) can SEE the file but cannot decrypt it --------
    let c = drive.as_actor("teacherC");
    let c_body = c.download(&bundle_file.id).unwrap(); // reader on exchange → can fetch bytes
    // C does not hold the class:VI-A key at all; its own VII-B key cannot open it.
    assert!(
        bundle::open_bundle(&_vb_key, "class:VI-A", va_ver, &c_body).is_err(),
        "other-class device cannot read VI-A attendance"
    );

    // --- accountant (finance only) also cannot decrypt class bundles ----------
    let body_acc = drive.as_actor("accountant").download(&bundle_file.id).unwrap();
    assert!(bundle::open_bundle(&_fin_key, "class:VI-A", va_ver, &body_acc).is_err());
}

#[test]
fn tampered_bundle_a_holder_should_read_is_quarantined() {
    let mut conn = db::open_in_memory(DBKEY).unwrap();
    db::run_migrations(&mut conn).unwrap();
    let (va_key, va_ver) = server_key(&conn, "class:VI-A");

    let drive = FakeDrive::new();
    let l = drive.provision_school(&[("teacherA", "devA"), ("teacherB", "devB")]);

    // teacherA uploads a bundle, then a byte is corrupted in transit/at rest.
    let ops = vec![attendance_op(1, "VI-A")];
    let mut body = bundle::seal_bundle(&va_key, "class:VI-A", va_ver, &ops).unwrap();
    let last = body.len() - 1;
    body[last] ^= 0x01; // tamper
    let props = bundle::bundle_properties("class:VI-A", va_ver);
    let a = drive.as_actor("teacherA");
    let f = a.create(&l.ops["devA"], "x.vop", &body, &props).unwrap();

    // teacherB HOLDS the class:VI-A key and the properties say VI-A v1, so it
    // must be able to open it — a failure here is tampering → quarantine.
    let b = drive.as_actor("teacherB");
    let meta = b.metadata(&f.id).unwrap();
    assert_eq!(meta.properties.get(PROP_AUDIENCE).unwrap(), "class:VI-A"); // addressed to me
    let fetched = b.download(&f.id).unwrap();
    assert!(
        bundle::open_bundle(&va_key, "class:VI-A", va_ver, &fetched).is_err(),
        "tampered bundle fails AEAD → quarantine, not applied"
    );
}

#[test]
fn rotation_lets_the_server_still_read_old_bundles() {
    // Assignment change rotates the class key; a bundle written under v1 must
    // still be openable by the server (which keeps all versions) — §8.8.
    let mut conn = db::open_in_memory(DBKEY).unwrap();
    db::run_migrations(&mut conn).unwrap();
    let (v1_key, _) = server_key(&conn, "class:VI-A");
    let ops = vec![attendance_op(9, "VI-A")];
    let body = bundle::seal_bundle(&v1_key, "class:VI-A", 1, &ops).unwrap();

    // rotate (new teacher assigned) → v2 is now current, v1 retained.
    let v2 = keys::rotate(&conn, "class:VI-A").unwrap();
    assert_eq!(v2.version, 2);
    // server opens the old v1 bundle using the retained v1 key
    let v1_again = keys::key_bytes(&conn, "class:VI-A", 1).unwrap().unwrap();
    let got = bundle::open_bundle(&v1_again, "class:VI-A", 1, &body).unwrap();
    assert_eq!(got, ops);
    // and the current key is a different, newer version
    assert_ne!(keys::key_bytes(&conn, "class:VI-A", 2).unwrap().unwrap(), v1_key);
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

// ============================ Step 5 — device provisional pull ================

const FIN_KEY: [u8; 32] = [0x11; 32];
const CLASS_KEY: [u8; 32] = [0x22; 32];

fn phone() -> Connection {
    let mut c = db::open_in_memory(DBKEY).unwrap();
    db::run_migrations(&mut c).unwrap();
    c
}

fn queue(dev: &Connection, o: &Op) {
    dev.execute(
        "INSERT INTO outbox(op_id,hlc,device_id,staff_id,audience,\"table\",record_id,kind,payload,base_version,server_epoch) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![o.op_id, o.hlc, o.device_id, o.staff_id, o.audience, o.table, o.record_id, o.kind, o.payload.to_string(), o.base_version, o.server_epoch],
    ).unwrap();
}

fn student_op(id: &str, device: &str, name: &str) -> Op {
    Op {
        op_id: format!("op-{id}"),
        hlc: format!("00000000000000000001{id}"),
        device_id: device.into(),
        staff_id: "accA".into(),
        audience: "finance".into(),
        table: "student".into(),
        record_id: id.into(),
        kind: "insert".into(),
        payload: serde_json::json!({ "name": name, "status": "active" }),
        base_version: None,
        server_epoch: 1,
    }
}

fn held(audience: &str, key: [u8; 32]) -> HeldKeys {
    let mut h = HeldKeys::new();
    h.insert((audience.to_string(), 1), key);
    h
}

/// DONE-MEANS phone side: phone A (finance) shares a student to Drive → phone B
/// (finance) sees it PROVISIONALLY; phone C (a class device) cannot.
#[test]
fn phone_b_sees_provisional_phone_c_cannot() {
    let drive = FakeDrive::new();
    let l = drive.provision_school(&[("accA", "devA"), ("accB", "devB"), ("teacherC", "devC")]);

    // Phone A (accountant) records a student while the server is off, shares it.
    let mut a = phone();
    queue(&a, &student_op("stu-p5", "devA", "Provisional Priya"));
    exchange::push_outbox(&mut a, &drive.as_actor("accA"), &l.ops["devA"], &held("finance", FIN_KEY)).unwrap();

    // Phone B (also finance) pulls → provisional row, marked shared_drive.
    let mut b = phone();
    let out = pull::pull_provisional(&mut b, &drive.as_actor("accB"), &l.exchange, "devB", &held("finance", FIN_KEY)).unwrap();
    assert_eq!((out.applied, out.bundles_seen, out.quarantined), (1, 1, 0));
    let (name, state): (String, String) = b
        .query_row("SELECT name, sync_state FROM student WHERE id='stu-p5'", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!(name, "Provisional Priya");
    assert_eq!(state, "shared_drive", "marked 'Shared through school Drive · waiting for school'");
    assert_eq!(pull::PROVISIONAL_STATUS, "Shared through school Drive · waiting for school");

    // A second pull is a no-op (cursor per folder).
    let again = pull::pull_provisional(&mut b, &drive.as_actor("accB"), &l.exchange, "devB", &held("finance", FIN_KEY)).unwrap();
    assert_eq!((again.applied, again.bundles_seen), (0, 0));

    // Phone C holds only a class key → the finance bundle is "not for me".
    let mut c = phone();
    let cout = pull::pull_provisional(&mut c, &drive.as_actor("teacherC"), &l.exchange, "devC", &held("class:cls-1", CLASS_KEY)).unwrap();
    assert_eq!((cout.applied, cout.skipped_not_for_me), (0, 1));
    let n: i64 = c.query_row("SELECT COUNT(*) FROM student WHERE id='stu-p5'", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 0, "other-audience device never sees the row");
}

/// Step 5: a provisional pull never overwrites the device's own unsent change.
#[test]
fn pull_never_overwrites_own_unsent() {
    let drive = FakeDrive::new();
    let l = drive.provision_school(&[("accA", "devA"), ("accB", "devB")]);
    let mut a = phone();
    queue(&a, &student_op("stu-shared", "devA", "From A"));
    exchange::push_outbox(&mut a, &drive.as_actor("accA"), &l.ops["devA"], &held("finance", FIN_KEY)).unwrap();

    // Phone B has its OWN unsent edit to the same record (an outbox op).
    let mut b = phone();
    queue(&b, &student_op("stu-shared", "devB", "My local edit"));
    let out = pull::pull_provisional(&mut b, &drive.as_actor("accB"), &l.exchange, "devB", &held("finance", FIN_KEY)).unwrap();
    assert_eq!((out.applied, out.kept_local), (0, 1), "own unsent kept, remote flagged not applied");
    let n: i64 = b.query_row("SELECT COUNT(*) FROM student WHERE id='stu-shared'", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 0, "the remote value did not overwrite the local unsent change");
}

/// Step 5: a bundle the device holds the key for but that fails AEAD → quarantined.
#[test]
fn pull_quarantines_a_tampered_bundle() {
    let drive = FakeDrive::new();
    let l = drive.provision_school(&[("accA", "devA"), ("accB", "devB")]);
    // Put a tampered finance bundle in A's folder (correct properties).
    let o = student_op("stu-bad", "devA", "Tampered");
    let mut body = bundle::seal_bundle(&FIN_KEY, "finance", 1, std::slice::from_ref(&o)).unwrap();
    let last = body.len() - 1;
    body[last] ^= 0x01;
    drive
        .as_actor("accA")
        .create(&l.ops["devA"], "bad.vop", &body, &bundle::bundle_properties("finance", 1))
        .unwrap();

    let mut b = phone();
    let out = pull::pull_provisional(&mut b, &drive.as_actor("accB"), &l.exchange, "devB", &held("finance", FIN_KEY)).unwrap();
    assert_eq!((out.quarantined, out.applied), (1, 0));
    let q: i64 = b.query_row("SELECT COUNT(*) FROM drive_state WHERE key LIKE 'quarantine:%'", [], |r| r.get(0)).unwrap();
    assert_eq!(q, 1, "the tampered bundle is recorded for the Principal");
}
