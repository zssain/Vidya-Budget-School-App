//! Sync integration harness (prompts/P04 Step 12). In-process server + clients
//! over the real service layer (server::service + sync::apply/scope), with the
//! "network" modelled by controlling push order/duplication/delay. Proves the
//! DONE-MEANS: attendance confirm, conflict, revoke, teacher scope, plus §8.6
//! payments, idempotency, and 5,000 queued ops after an outage.
//!
//! A real TLS transport is verified separately (server::cert pinned-verifier test).

use rusqlite::Connection;
use time::OffsetDateTime;

use vidya_lib::server::service;
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
