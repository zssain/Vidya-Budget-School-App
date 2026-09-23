//! Sync integration harness (prompts/P04 Step 12). In-process server + clients
//! over the real service layer (server::service + sync::apply/scope), with the
//! "network" modelled by controlling push order/duplication/delay. Proves the
//! DONE-MEANS: attendance confirm, conflict, revoke, teacher scope, plus §8.6
//! payments, idempotency, and 5,000 queued ops after an outage.
//!
//! A real TLS transport is verified separately (server::cert pinned-verifier test).

use rusqlite::{params, Connection};
use time::OffsetDateTime;

use vidya_lib::server::service;
use vidya_lib::sync::drive::exchange::{self, HeldKeys};
use vidya_lib::sync::drive::fake::{FakeDrive, PRINCIPAL};
use vidya_lib::sync::drive::keys;
use vidya_lib::sync::drive::DriveApi;
use vidya_lib::sync::protocol::{Op, OpStatus};
use vidya_lib::sync::scope;
use vidya_lib::{db, seed};

const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn now() -> OffsetDateTime {
    OffsetDateTime::parse("2026-09-23T12:00:00Z", &time::format_description::well_known::Rfc3339).unwrap()
}

/// The school server: a seeded, migrated encrypted DB.
fn server() -> Connection {
    let mut c = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut c).unwrap();
    seed::seed_demo_school(&mut c, now()).unwrap();
    c
}

#[allow(clippy::too_many_arguments)]
fn op(op_id: &str, staff: &str, device: &str, table: &str, record: &str, kind: &str, payload: serde_json::Value, base: Option<i64>) -> Op {
    Op {
        op_id: op_id.into(),
        hlc: format!("00000000000000000001{op_id}"),
        device_id: device.into(),
        staff_id: staff.into(),
        audience: "admin".into(),
        table: table.into(),
        record_id: record.into(),
        kind: kind.into(),
        payload,
        base_version: base,
        server_epoch: 1,
    }
}

fn push1(conn: &mut Connection, o: &Op) -> OpStatus {
    service::push(conn, std::slice::from_ref(o), now()).unwrap().results[0].status
}

// DONE-MEANS #1 — a queued attendance op is confirmed when it reaches the server.
#[test]
fn attendance_op_confirmed_when_it_reaches_server() {
    let mut s = server();
    // A V-A mark currently 'A' (Meena is class teacher → TakeAttendance allowed).
    let mark_id: String = s
        .query_row(
            "SELECT m.id FROM attendance_mark m JOIN attendance_sheet sh ON sh.id=m.sheet_id WHERE sh.class_id='cls-5a' AND m.mark='A' LIMIT 1",
            [], |r| r.get(0),
        )
        .unwrap();
    let o = op("op-att-1", "stf-meena", "dev-phone", "attendance_mark", &mark_id, "update", serde_json::json!({ "mark": "P" }), None);
    assert_eq!(push1(&mut s, &o), OpStatus::Confirmed);
    let mark: String = s.query_row("SELECT mark FROM attendance_mark WHERE id=?1", [&mark_id], |r| r.get(0)).unwrap();
    assert_eq!(mark, "P", "confirmed op changed the register");
    assert_eq!(vidya_lib::security::audit::verify_chain(&s).unwrap(), None);
}

// Idempotency (§8.4) — the same op sent twice applies once.
#[test]
fn duplicate_op_applies_once() {
    let mut s = server();
    let sid = "stu-new-1";
    let o = op("op-dup", "stf-priya", "dev-a1", "student", sid, "insert", serde_json::json!({ "name": "Dup Test", "status": "active" }), None);
    let r1 = service::push(&mut s, std::slice::from_ref(&o), now()).unwrap().results[0].clone();
    let r2 = service::push(&mut s, std::slice::from_ref(&o), now()).unwrap().results[0].clone();
    assert_eq!(r1.status, OpStatus::Confirmed);
    assert_eq!(r1.server_seq, r2.server_seq, "same op_id → same recorded result");
    let n: i64 = s.query_row("SELECT COUNT(*) FROM student WHERE id=?1", [sid], |r| r.get(0)).unwrap();
    assert_eq!(n, 1, "inserted exactly once");
}

// DONE-MEANS #2 — concurrent edits of the same field from the same base version →
// first confirmed, second is a conflict (kept current, conflict row raised).
#[test]
fn concurrent_address_edits_make_one_conflict() {
    let mut s = server();
    let sid = "stu-kavya-singh";
    let base: i64 = s.query_row("SELECT version FROM student WHERE id=?1", [sid], |r| r.get(0)).unwrap();
    let a = op("op-addr-a", "stf-priya", "dev-A", "student", sid, "update", serde_json::json!({ "address": "12 Temple Road" }), Some(base));
    let b = op("op-addr-b", "stf-priya", "dev-B", "student", sid, "update", serde_json::json!({ "address": "99 Market Lane" }), Some(base));
    assert_eq!(push1(&mut s, &a), OpStatus::Confirmed);
    assert_eq!(push1(&mut s, &b), OpStatus::Conflict, "second edit from the stale base conflicts");
    // Current value kept = A's; one open conflict row exists.
    let addr: String = s.query_row("SELECT address FROM student WHERE id=?1", [sid], |r| r.get(0)).unwrap();
    assert_eq!(addr, "12 Temple Road");
    let conflicts: i64 = s.query_row("SELECT COUNT(*) FROM conflict WHERE record_id=?1 AND field='address' AND status='open'", [sid], |r| r.get(0)).unwrap();
    assert_eq!(conflicts, 1);
}

// §8.6 — two payments together exceeding dues: BOTH confirmed, the excess flagged.
#[test]
fn two_payments_exceeding_dues_both_land_excess_flagged() {
    let mut s = server();
    let sid = "stu-kavya-singh"; // ₹3,100 due
    let pay = |id: &str, rc: &str| op(id, "stf-suresh", "dev-a2", "payment", id, "insert",
        serde_json::json!({ "receipt_no": rc, "student_id": sid, "amount_paise": 300_000, "mode": "cash", "collected_by": "stf-suresh", "collected_at": "2026-09-23T12:00:00Z" }), None);
    let r1 = push1(&mut s, &pay("op-pay-1", "R-A2-0500"));
    let r2 = push1(&mut s, &pay("op-pay-2", "R-A2-0501"));
    assert_eq!(r1, OpStatus::Confirmed);
    assert_eq!(r2, OpStatus::Flagged, "excess payment is flagged, never rejected");
    let payments: i64 = s.query_row("SELECT COUNT(*) FROM payment WHERE id IN ('op-pay-1','op-pay-2')", [], |r| r.get(0)).unwrap();
    assert_eq!(payments, 2, "both payments kept (money never lost)");
    let flags: i64 = s.query_row("SELECT COUNT(*) FROM review_flag WHERE kind='excess_payment' AND status='open'", [], |r| r.get(0)).unwrap();
    assert_eq!(flags, 1);
}

// DONE-MEANS #3 — a revoked device's token stops working.
#[test]
fn revoked_device_token_is_rejected() {
    let mut s = server();
    // Provision a device via invite → join, then revoke it.
    let code = service::create_invite(&mut s, "stf-meena", now()).unwrap();
    let join = service::join(&mut s, &vidya_lib::sync::protocol::JoinReq {
        invite_code: code, device_name: "Meena Phone".into(), platform: "android".into(), google_email: None,
    }, now()).unwrap();
    // Token works before revoke.
    assert!(service::authenticate(&s, &join.device_token).unwrap().is_some());
    service::revoke_device(&mut s, &join.device_id, now()).unwrap();
    // After revoke → DEVICE_REVOKED_OR_UNKNOWN (auth returns None).
    assert!(service::authenticate(&s, &join.device_token).unwrap().is_none());
    // A garbage token is also unknown.
    assert!(service::authenticate(&s, "deadbeef").unwrap().is_none());
}

// DONE-MEANS #4 — a teacher device's scoped snapshot has no fee rows / foreign students,
// and losing the class removes them from pull scope.
#[test]
fn teacher_scope_excludes_fees_and_follows_assignment() {
    let s = server();
    let actor = vidya_lib::sync::apply::load_actor(&s, "stf-meena").unwrap().unwrap(); // class teacher of V-A + VII-B
    let snap = scope::snapshot(&s, &actor).unwrap();
    assert!(snap.iter().all(|c| !scope::FEE_TABLES.contains(&c.table.as_str())), "no fee rows on a teacher device");
    // Meena is class teacher of V-A (34) and VII-B (50); she sees exactly those rosters.
    let expected: i64 = s
        .query_row("SELECT COUNT(DISTINCT student_id) FROM enrollment WHERE class_id IN ('cls-5a','cls-7b') AND to_date IS NULL", [], |r| r.get(0))
        .unwrap();
    assert_eq!(snap.iter().filter(|c| c.table == "student").count() as i64, expected, "only her class rosters");

    // Losing the class (class_teacher_id cleared) → the student is no longer visible.
    let sid: String = s.query_row("SELECT student_id FROM enrollment WHERE class_id='cls-5a' AND to_date IS NULL LIMIT 1", [], |r| r.get(0)).unwrap();
    assert!(scope::visible_row(&s, &actor, "student", &sid).unwrap().is_some());
    let empty = vidya_core::permissions::Actor { class_teacher_of: vec![], class_subjects: vec![], ..actor.clone() };
    assert!(scope::visible_row(&s, &empty, "student", &sid).unwrap().is_none(), "lost assignment removes the row from scope");
}

// A join provisions a fresh series and a snapshot; pull advances by cursor.
#[test]
fn join_then_snapshot_then_incremental_pull() {
    let mut s = server();
    let code = service::create_invite(&mut s, "stf-suresh", now()).unwrap();
    let join = service::join(&mut s, &vidya_lib::sync::protocol::JoinReq {
        invite_code: code, device_name: "Accountant PC".into(), platform: "windows".into(), google_email: None,
    }, now()).unwrap();
    assert!(join.receipt_series.starts_with('A'));
    let auth = service::authenticate(&s, &join.device_token).unwrap().unwrap();

    // Snapshot has fee data (accountant) but no attendance marks.
    let snap = service::snapshot(&s, &auth).unwrap();
    assert!(snap.iter().any(|c| c.table == "payment"));
    assert!(snap.iter().all(|c| c.table != "attendance_mark"));

    // A new op after the join cursor shows up in pull.
    let o = op("op-after-join", "stf-suresh", &join.device_id, "payment", "op-after-join", "insert",
        serde_json::json!({ "receipt_no": "R-A9-0001", "student_id": "stu-kavya-singh", "amount_paise": 10_000, "mode": "cash", "collected_by": "stf-suresh", "collected_at": "2026-09-23T12:00:00Z" }), None);
    service::push(&mut s, std::slice::from_ref(&o), now()).unwrap();
    let pull = service::pull(&s, &auth, join.bootstrap_cursor, 500, now()).unwrap();
    assert!(pull.changes.iter().any(|c| c.record_id == "op-after-join"), "incremental pull returns the new payment");
    assert!(pull.next_cursor > join.bootstrap_cursor);
}

// 5,000 queued ops after a long outage all apply; server stays consistent.
#[test]
fn five_thousand_queued_ops_apply() {
    let mut s = server();
    let before: i64 = s.query_row("SELECT COUNT(*) FROM student", [], |r| r.get(0)).unwrap();
    let mut batch: Vec<Op> = Vec::new();
    for i in 0..5000 {
        batch.push(op(
            &format!("op-bulk-{i}"), "stf-priya", "dev-a1", "student", &format!("stu-bulk-{i}"), "insert",
            serde_json::json!({ "name": format!("Bulk {i}"), "status": "active" }), None,
        ));
    }
    // Pushed in ≤200 batches (protocol cap), as the client would after an outage.
    for chunk in batch.chunks(200) {
        let resp = service::push(&mut s, chunk, now()).unwrap();
        assert!(resp.results.iter().all(|r| r.status == OpStatus::Confirmed));
    }
    let after: i64 = s.query_row("SELECT COUNT(*) FROM student", [], |r| r.get(0)).unwrap();
    assert_eq!(after - before, 5000);
    assert_eq!(vidya_lib::security::audit::verify_chain(&s).unwrap(), None, "audit chain stays valid at scale");
}

// ======================================================================
// P06 — Google Drive fallback exchange (Step 9), over the fake DriveApi.
// Server off → phone seals ops into a `.vop` and uploads to its own Drive
// folder → server on → imports in HLC order via the SAME apply_op path →
// Confirmed. Uses the real `sync::drive::{exchange, keys, bundle, fake}`.
// ======================================================================

/// A fresh, migrated "phone" DB with an outbox (no seed needed on the device).
fn phone() -> Connection {
    let mut c = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut c).unwrap();
    c
}

/// Queue one op into a device's outbox (what `with_write` does in Client mode).
fn queue(dev: &Connection, o: &Op) {
    dev.execute(
        "INSERT INTO outbox(op_id,hlc,device_id,staff_id,audience,\"table\",record_id,kind,payload,base_version,server_epoch) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![o.op_id, o.hlc, o.device_id, o.staff_id, o.audience, o.table, o.record_id, o.kind, o.payload.to_string(), o.base_version, o.server_epoch],
    ).unwrap();
}

/// Deliver the server's current key for `audience` to a device (as join would).
fn deliver(server: &Connection, audience: &str) -> HeldKeys {
    let k = keys::get_or_create(server, audience).unwrap();
    let mut h = HeldKeys::new();
    h.insert((audience.to_string(), k.version), keys::decode_key(&k.key_b64).unwrap());
    h
}

fn attendance_op(id: &str, mark_id: &str) -> Op {
    let mut o = op(id, "stf-meena", "dev-phone", "attendance_mark", mark_id, "update", serde_json::json!({ "mark": "P" }), None);
    o.audience = "class:cls-5a".into(); // decided by vidya_core::audience::audience_for
    o
}

/// DONE-MEANS: server PC off → phone records attendance → shared to Drive →
/// server PC on → imports → Confirmed by school server. The mark changes.
#[test]
fn drive_server_off_phone_pushes_then_server_imports_confirmed() {
    let mut s = server();
    let mark_id: String = s
        .query_row(
            "SELECT m.id FROM attendance_mark m JOIN attendance_sheet sh ON sh.id=m.sheet_id WHERE sh.class_id='cls-5a' AND m.mark='A' LIMIT 1",
            [], |r| r.get(0),
        )
        .unwrap();

    // Phone (server unreachable) queues the mark A→P and holds the class key.
    let mut ph = phone();
    let o = attendance_op("op-att-drive", &mark_id);
    queue(&ph, &o);
    let held = deliver(&s, "class:cls-5a");

    // Drive: provision the school; the phone pushes its outbox.
    let drive = FakeDrive::new();
    let l = drive.provision_school(&[("stf-meena", "dev-phone")]);
    let pushed = exchange::push_outbox(&mut ph, &drive.as_actor("stf-meena"), &l.ops["dev-phone"], &held).unwrap();
    assert_eq!((pushed.bundles, pushed.ops), (1, 1), "one bundle, one op shared to Drive");

    // Server comes back and imports everything from exchange/.
    let out = exchange::import_all(&mut s, &drive.as_actor(PRINCIPAL), &l.exchange).unwrap();
    assert_eq!(out.confirmed, 1, "the attendance op is confirmed on import");
    assert_eq!(out.archived, 1, "processed bundle archived to _done/");
    let mark: String = s.query_row("SELECT mark FROM attendance_mark WHERE id=?1", [&mark_id], |r| r.get(0)).unwrap();
    assert_eq!(mark, "P", "the register changed on the server");
    assert_eq!(vidya_lib::security::audit::verify_chain(&s).unwrap(), None);

    // The ack tells the phone its op is confirmed (Step 7).
    let ack = out.acks.get("dev-phone").expect("ack for the phone");
    assert_eq!(ack.results[0].status, OpStatus::Confirmed);

    // Re-import is a no-op (bundle archived + idempotent by op_id).
    let again = exchange::import_all(&mut s, &drive.as_actor(PRINCIPAL), &l.exchange).unwrap();
    assert_eq!((again.confirmed, again.bundles), (0, 0));
}

/// Step 9: a tampered bundle a holder should read fails AEAD → quarantined, not applied.
#[test]
fn drive_tampered_bundle_is_quarantined_on_import() {
    let mut s = server();
    let mark_id: String = s
        .query_row("SELECT m.id FROM attendance_mark m JOIN attendance_sheet sh ON sh.id=m.sheet_id WHERE sh.class_id='cls-5a' AND m.mark='A' LIMIT 1", [], |r| r.get(0))
        .unwrap();
    let key = keys::get_or_create(&s, "class:cls-5a").unwrap();
    let kb = keys::decode_key(&key.key_b64).unwrap();
    let o = attendance_op("op-tamper", &mark_id);
    let mut body = vidya_lib::sync::drive::bundle::seal_bundle(&kb, "class:cls-5a", key.version, std::slice::from_ref(&o)).unwrap();
    let last = body.len() - 1;
    body[last] ^= 0x01; // tamper in transit

    let drive = FakeDrive::new();
    let l = drive.provision_school(&[("stf-meena", "dev-phone")]);
    let props = vidya_lib::sync::drive::bundle::bundle_properties("class:cls-5a", key.version);
    drive.as_actor("stf-meena").create(&l.ops["dev-phone"], "bad.vop", &body, &props).unwrap();

    let out = exchange::import_all(&mut s, &drive.as_actor(PRINCIPAL), &l.exchange).unwrap();
    assert_eq!((out.quarantined, out.confirmed, out.archived), (1, 0, 0), "tampered bundle not applied");
    let mark: String = s.query_row("SELECT mark FROM attendance_mark WHERE id=?1", [&mark_id], |r| r.get(0)).unwrap();
    assert_eq!(mark, "A", "the register is unchanged");
}

/// Step 9: a revoked/suspended author's later bundle is flagged, not applied.
#[test]
fn drive_revoked_author_bundle_is_flagged_not_applied() {
    let mut s = server();
    let mark_id: String = s
        .query_row("SELECT m.id FROM attendance_mark m JOIN attendance_sheet sh ON sh.id=m.sheet_id WHERE sh.class_id='cls-5a' AND m.mark='A' LIMIT 1", [], |r| r.get(0))
        .unwrap();
    s.execute("UPDATE staff SET state='suspended' WHERE id='stf-meena'", []).unwrap();

    let mut ph = phone();
    let o = attendance_op("op-revoked", &mark_id);
    queue(&ph, &o);
    let held = deliver(&s, "class:cls-5a");
    let drive = FakeDrive::new();
    let l = drive.provision_school(&[("stf-meena", "dev-phone")]);
    exchange::push_outbox(&mut ph, &drive.as_actor("stf-meena"), &l.ops["dev-phone"], &held).unwrap();

    let out = exchange::import_all(&mut s, &drive.as_actor(PRINCIPAL), &l.exchange).unwrap();
    assert_eq!((out.flagged, out.confirmed), (1, 0), "suspended author → flagged");
    let flags: i64 = s.query_row("SELECT COUNT(*) FROM review_flag WHERE kind='revoked_author' AND status='open'", [], |r| r.get(0)).unwrap();
    assert_eq!(flags, 1);
    let mark: String = s.query_row("SELECT mark FROM attendance_mark WHERE id=?1", [&mark_id], |r| r.get(0)).unwrap();
    assert_eq!(mark, "A", "not applied");
}

/// Step 9: the same op via LAN and via Drive is applied exactly once (§8.4).
#[test]
fn drive_same_op_via_lan_and_drive_applies_once() {
    let mut s = server();
    // The op the accountant records (audience = finance).
    let sid = "stu-lan-drive";
    let mut o = op("op-lan-drive", "stf-suresh", "dev-a2", "student", sid, "insert", serde_json::json!({ "name": "Once Only", "status": "active" }), None);
    o.audience = "finance".into();

    // (1) it reaches the server over LAN first → confirmed.
    assert_eq!(push1(&mut s, &o), OpStatus::Confirmed);

    // (2) the same op also went out over Drive; the server imports it → no double apply.
    let drive = FakeDrive::new();
    let l = drive.provision_school(&[("stf-suresh", "dev-a2")]);
    let held = deliver(&s, "finance");
    let mut ph = phone();
    queue(&ph, &o);
    exchange::push_outbox(&mut ph, &drive.as_actor("stf-suresh"), &l.ops["dev-a2"], &held).unwrap();
    let out = exchange::import_all(&mut s, &drive.as_actor(PRINCIPAL), &l.exchange).unwrap();

    // apply_op is idempotent by op_id: the import "confirms" from the remembered
    // result but the student exists exactly once.
    assert_eq!(out.confirmed, 1);
    let n: i64 = s.query_row("SELECT COUNT(*) FROM student WHERE id=?1", [sid], |r| r.get(0)).unwrap();
    assert_eq!(n, 1, "applied once despite arriving via both routes");
    let oplog: i64 = s.query_row("SELECT COUNT(*) FROM op_log WHERE op_id='op-lan-drive'", [], |r| r.get(0)).unwrap();
    assert_eq!(oplog, 1, "one op_log entry");
}

/// Step 6/7: the server's sealed ack round-trips to the device.
#[test]
fn drive_ack_round_trips_to_the_device() {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    let mut s = server();
    let mark_id: String = s
        .query_row("SELECT m.id FROM attendance_mark m JOIN attendance_sheet sh ON sh.id=m.sheet_id WHERE sh.class_id='cls-5a' AND m.mark='A' LIMIT 1", [], |r| r.get(0))
        .unwrap();
    let mut ph = phone();
    let o = attendance_op("op-ack", &mark_id);
    queue(&ph, &o);
    let held = deliver(&s, "class:cls-5a");
    let drive = FakeDrive::new();
    let l = drive.provision_school(&[("stf-meena", "dev-phone")]);
    exchange::push_outbox(&mut ph, &drive.as_actor("stf-meena"), &l.ops["dev-phone"], &held).unwrap();
    let out = exchange::import_all(&mut s, &drive.as_actor(PRINCIPAL), &l.exchange).unwrap();

    // Server seals the ack with the device session key; the device reads it back.
    let session = STANDARD.encode([0x33u8; 32]);
    exchange::write_ack(&drive.as_actor(PRINCIPAL), &l.acks, "dev-phone", &out.acks["dev-phone"], &session).unwrap();
    let ack = exchange::read_ack(&drive.as_actor("stf-meena"), &l.acks, "dev-phone", &session).unwrap().unwrap();
    assert_eq!(ack.results[0].status, OpStatus::Confirmed);
    assert_eq!(ack.last_hlc, o.hlc);
}
