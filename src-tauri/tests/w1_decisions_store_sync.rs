//! W1 (P21 repairs), the last open part: Principal decisions and store sales from a
//! joined second PC. Each one leaves the device as ONE action op and is replayed on
//! the school PC as its author through the SAME domain function
//! (`decide_request_with`, `accept/reject_away_checkin_logic`,
//! `record_store_sale_with`), re-checking the author's CURRENT role, the live device,
//! the pending state / module switch — and, for a sale, recording the money exactly
//! once even when stock no longer covers it (§8.6: flagged, never refused).

use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};
use time::OffsetDateTime;

use vidya_lib::commands::logic::{
    accept_away_checkin_logic, decide_request_logic, record_store_sale_logic, reject_away_checkin_logic, request_command,
    stock_adjust_logic, ClientCmd, LeaveRequestInput, RequestCmd, SaleItemInput, StockAdjustInput, StoreSaleInput,
};
use vidya_lib::server::service::{self, DeviceAuth};
use vidya_lib::state::SessionStaff;
use vidya_lib::sync::apply::load_actor;
use vidya_lib::sync::client::{self, ClientIdentity};
use vidya_lib::sync::engine::{apply_snapshot, outbox_ops, sync_once};
use vidya_lib::sync::protocol::{Op, OpResult, OpStatus};
use vidya_lib::sync::scope;
use vidya_lib::sync::transport::LoopbackTransport;
use vidya_lib::write::DeviceMode;
use vidya_lib::{db, seed};

const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const AWAY_DAY: &str = "2026-10-14";

fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

fn priya() -> SessionStaff {
    SessionStaff { id: "stf-priya".into(), name: "Priya".into(), role: "principal".into() }
}
fn suresh() -> SessionStaff {
    SessionStaff { id: "stf-suresh".into(), name: "Suresh".into(), role: "accountant".into() }
}
fn meena() -> SessionStaff {
    SessionStaff { id: "stf-meena".into(), name: "Meena Iyer".into(), role: "teacher".into() }
}

/// The school PC: demo school, HR + store modules on, three joined devices — the
/// Principal's second PC, the accountant's second PC (series A9), a teacher's phone.
fn server() -> Connection {
    let mut c = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut c).unwrap();
    seed::seed_demo_school(&mut c, OffsetDateTime::now_utc()).unwrap();
    c.execute("UPDATE module_setting SET enabled=1 WHERE key IN ('hr','store')", []).unwrap();
    // Published into the synced school row, so joined devices see the same switches.
    vidya_lib::modules::publish_to_school(&c).unwrap();
    c.execute_batch(
        "INSERT INTO device(id,staff_id,platform,name,token_hash,receipt_series) VALUES ('dev-ppc','stf-priya','windows','Office PC','h','A8');
         INSERT INTO device(id,staff_id,platform,name,token_hash,receipt_series) VALUES ('dev-acc2','stf-suresh','windows','PC2','h','A9');
         INSERT INTO device(id,staff_id,platform,name,token_hash) VALUES ('dev-phone','stf-meena','android','Phone','h');",
    )
    .unwrap();
    // An away check-in waiting for the Principal, and a pending details request.
    c.execute(
        "INSERT INTO staff_attendance(id,staff_id,date,check_in_min,route,status,created_at,updated_at,sync_state) \
         VALUES ('sat-away','stf-meena',?1,500,'drive','away_pending','t','t','confirmed')",
        [AWAY_DAY],
    )
    .unwrap();
    c.execute(
        "INSERT INTO request(id,type,target_table,target_id,base_version,reason,requested_by,revision,status,apply_state,created_at,updated_at,sync_state) \
         VALUES ('req-det','student_details','student','stu-x',1,'spelling','stf-meena',1,'pending','not_applied','t','t','confirmed')",
        [],
    )
    .unwrap();
    c
}

/// A pending leave request raised by Meena on the school PC.
fn pending_leave(s: &mut Connection) -> String {
    let leave_type: String = s.query_row("SELECT id FROM leave_type WHERE active=1 LIMIT 1", [], |r| r.get(0)).unwrap();
    let cmd = RequestCmd::Leave {
        id: "req-leave".into(),
        input: LeaveRequestInput { leave_type_id: leave_type, from: "2026-11-16".into(), to: "2026-11-17".into(), reason: "Family function".into() },
    };
    request_command(s, &meena(), None, DeviceMode::Server, &cmd).unwrap();
    "req-leave".into()
}

/// A joined device for `staff` with the school PC's role-scoped snapshot applied.
fn joined(server: &Connection, device: &str, staff: &str, role: &str, series: &str) -> Connection {
    let mut c = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut c).unwrap();
    let actor = load_actor(server, staff).unwrap().unwrap();
    let snap = service::snapshot_of(server, scope::snapshot(server, &actor).unwrap()).unwrap();
    apply_snapshot(&mut c, &snap.manifest, &snap.changes).unwrap();
    client::store(
        &c,
        &ClientIdentity {
            device_id: device.into(),
            device_token: "tok".into(),
            staff_id: staff.into(),
            staff_name: "x".into(),
            staff_role: role.into(),
            session_key: String::new(),
            school_id: "sch-demo".into(),
            school_name: "Demo".into(),
            receipt_series: series.into(),
            admission_series: series.into(),
            server_fingerprint: "fp".into(),
            server_addrs: vec![],
            server_port: 47650,
            server_epoch: 1,
            bootstrap_cursor: 0,
            lease_expires_at: String::new(),
            protocol: 1,
        },
    )
    .unwrap();
    c
}

fn auth(device: &str, staff: &str) -> DeviceAuth {
    DeviceAuth { device_id: device.into(), staff_id: staff.into() }
}

/// Push `ops` straight to the school PC as `auth` (a LAN push).
fn push(s: &mut Connection, a: &DeviceAuth, ops: &[Op]) -> Vec<OpResult> {
    service::push(s, a, ops, now()).unwrap().results
}

/// A hand-built op as a (possibly hostile) device would send it.
fn forged(op_id: &str, device: &str, staff: &str, table: &str, record_id: &str, payload: serde_json::Value) -> Op {
    Op {
        op_id: op_id.into(),
        hlc: format!("0001700000000000000-{device}"),
        device_id: device.into(),
        staff_id: staff.into(),
        audience: "admin".into(),
        table: table.into(),
        record_id: record_id.into(),
        kind: "action".into(),
        payload,
        base_version: None,
        server_epoch: 1,
    }
}

fn request_status(s: &Connection, id: &str) -> String {
    s.query_row("SELECT status FROM request WHERE id=?1", [id], |r| r.get(0)).unwrap()
}
fn count(s: &Connection, sql: &str, p: impl rusqlite::Params) -> i64 {
    s.query_row(sql, p, |r| r.get(0)).unwrap()
}

// ------------------------------------------------------------------ decisions --

#[tokio::test]
async fn principal_on_a_second_pc_approves_leave_and_accepts_an_away_checkin() {
    let mut s = server();
    let req = pending_leave(&mut s);
    let mut pc = joined(&s, "dev-ppc", "stf-priya", "principal", "A8");

    // On the joined PC: the decision lands locally (on_device) and queues ONE op each.
    let dto = request_command(&mut pc, &priya(), Some("dev-ppc"), DeviceMode::Client, &RequestCmd::Decide {
        id: req.clone(), decision: "approve".into(), note: Some("ok".into()),
    })
    .unwrap()
    .unwrap();
    assert_eq!(dto.status, "approved");
    accept_away_checkin_logic(&mut pc, &priya(), Some("dev-ppc"), DeviceMode::Client, "sat-away").unwrap();
    let ops = outbox_ops(&pc).unwrap();
    assert_eq!(ops.len(), 2);
    assert!(ops.iter().all(|o| o.kind == "action" && o.device_id == "dev-ppc"), "{ops:?}");
    assert_eq!((ops[0].table.as_str(), ops[1].table.as_str()), ("request", "staff_attendance"));
    // No effect is faked locally: the leave record is the school PC's to write.
    assert_eq!(count(&pc, "SELECT COUNT(*) FROM leave_record WHERE request_id=?1", [&req]), 0);

    let server = Arc::new(Mutex::new(s));
    let t = LoopbackTransport::new(server.clone(), auth("dev-ppc", "stf-priya"), now());
    let out = sync_once(&mut pc, &t).await.unwrap();
    assert_eq!((out.confirmed, out.rejected), (2, 0), "{out:?}");

    {
        let s = server.lock().unwrap();
        let (status, by, apply): (String, String, String) = s
            .query_row("SELECT status, decided_by, apply_state FROM request WHERE id=?1", [&req], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap();
        assert_eq!((status.as_str(), by.as_str(), apply.as_str()), ("approved", "stf-priya", "applied"));
        assert_eq!(count(&s, "SELECT COUNT(*) FROM leave_record WHERE request_id=?1 AND staff_id='stf-meena'", [&req]), 1, "effect applied on the school PC");
        let (st, acc): (String, String) =
            s.query_row("SELECT status, accepted_by FROM staff_attendance WHERE id='sat-away'", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert!(st == "present" || st == "late", "accepted: {st}");
        assert_eq!(acc, "stf-priya");
        // Audited as the Principal on her device.
        assert_eq!(count(&s, "SELECT COUNT(*) FROM audit_log WHERE action='approve_request' AND staff_id='stf-priya' AND device_id='dev-ppc'", []), 1);
        assert_eq!(count(&s, "SELECT COUNT(*) FROM audit_log WHERE action='accept_away_checkin' AND staff_id='stf-priya' AND device_id='dev-ppc'", []), 1);
    }
    // The pull brought the canonical effects back to the Principal's PC.
    assert_eq!(count(&pc, "SELECT COUNT(*) FROM leave_record WHERE request_id=?1", [&req]), 1);
    assert_eq!(request_status(&pc, &req), "approved");
}

#[tokio::test]
async fn a_decision_on_an_already_decided_request_is_refused_visibly_never_applied_twice() {
    let mut s = server();
    let req = pending_leave(&mut s);
    let mut pc = joined(&s, "dev-ppc", "stf-priya", "principal", "A8");

    // The Principal approves on the PC while offline…
    request_command(&mut pc, &priya(), Some("dev-ppc"), DeviceMode::Client, &RequestCmd::Decide { id: req.clone(), decision: "approve".into(), note: None }).unwrap();
    accept_away_checkin_logic(&mut pc, &priya(), Some("dev-ppc"), DeviceMode::Client, "sat-away").unwrap();
    // …but it was already rejected on the school PC (and the away day rejected too).
    decide_request_logic(&mut s, &priya(), DeviceMode::Server, &req, "reject", Some("no cover")).unwrap();
    reject_away_checkin_logic(&mut s, &priya(), None, DeviceMode::Server, "sat-away").unwrap();

    let server = Arc::new(Mutex::new(s));
    let t = LoopbackTransport::new(server.clone(), auth("dev-ppc", "stf-priya"), now());
    let out = sync_once(&mut pc, &t).await.unwrap();
    assert_eq!((out.confirmed, out.rejected), (0, 2), "{out:?}");
    {
        let s = server.lock().unwrap();
        assert_eq!(request_status(&s, &req), "rejected", "the first decision stands");
        assert_eq!(count(&s, "SELECT COUNT(*) FROM leave_record WHERE request_id=?1", [&req]), 0, "no effect from the refused approval");
        assert_eq!(count(&s, "SELECT COUNT(*) FROM audit_log WHERE action='approve_request'", []), 0);
        let st: String = s.query_row("SELECT status FROM staff_attendance WHERE id='sat-away'", [], |r| r.get(0)).unwrap();
        assert_eq!(st, "absent");
        let reasons: Vec<String> = s
            .prepare("SELECT result FROM applied_ops WHERE device_id='dev-ppc'")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|r| serde_json::from_str::<OpResult>(&r.unwrap()).unwrap().reason_code.unwrap_or_default())
            .collect();
        assert_eq!(reasons, vec!["ALREADY_DECIDED", "ALREADY_DECIDED"]);
    }
    // Visible to the Principal on the PC: an Inbox item with the reason…
    let n = count(&pc, "SELECT COUNT(*) FROM notification WHERE kind='rejected' AND vars_json LIKE '%ALREADY_DECIDED%'", []);
    assert_eq!(n, 2);
    // …and the pull replaced the local optimistic decision with the canonical one.
    assert_eq!(request_status(&pc, &req), "rejected");
    // Deciding again locally is refused the same way.
    let e = request_command(&mut pc, &priya(), Some("dev-ppc"), DeviceMode::Client, &RequestCmd::Decide { id: req.clone(), decision: "approve".into(), note: None }).unwrap_err();
    assert_eq!(e.code, "ALREADY_DECIDED");
}

#[test]
fn a_non_principal_cannot_send_a_decision_op() {
    let mut s = server();
    let a = auth("dev-phone", "stf-meena");
    let ops = [
        forged("op-t-dec", "dev-phone", "stf-meena", "request", "req-det",
            serde_json::to_value(RequestCmd::Decide { id: "req-det".into(), decision: "approve".into(), note: None }).unwrap()),
        forged("op-t-away", "dev-phone", "stf-meena", "staff_attendance", "sat-away",
            serde_json::json!({ "cmd": "accept_away_checkin", "id": "sat-away" })),
    ];
    let res = push(&mut s, &a, &ops);
    for r in &res {
        assert_eq!((r.status, r.reason_code.as_deref()), (OpStatus::Rejected, Some("FORBIDDEN")), "{r:?}");
    }
    assert_eq!(request_status(&s, "req-det"), "pending");
    let st: String = s.query_row("SELECT status FROM staff_attendance WHERE id='sat-away'", [], |r| r.get(0)).unwrap();
    assert_eq!(st, "away_pending");

    // An accountant is not a decider either.
    let res = push(&mut s, &auth("dev-acc2", "stf-suresh"), &[forged("op-a-dec", "dev-acc2", "stf-suresh", "request", "req-det",
        serde_json::to_value(RequestCmd::Decide { id: "req-det".into(), decision: "reject".into(), note: None }).unwrap())]);
    assert_eq!(res[0].reason_code.as_deref(), Some("FORBIDDEN"));
    // A teacher can't even claim to be the Principal's device.
    let res = push(&mut s, &a, &[forged("op-claim", "dev-ppc", "stf-priya", "request", "req-det",
        serde_json::to_value(RequestCmd::Decide { id: "req-det".into(), decision: "approve".into(), note: None }).unwrap())]);
    assert_eq!(res[0].reason_code.as_deref(), Some("FORBIDDEN"));
    assert_eq!(request_status(&s, "req-det"), "pending");
}

#[test]
fn a_revoked_device_or_a_suspended_principal_cannot_decide() {
    let mut s = server();
    let mut pc = joined(&s, "dev-ppc", "stf-priya", "principal", "A8");
    request_command(&mut pc, &priya(), Some("dev-ppc"), DeviceMode::Client, &RequestCmd::Decide { id: "req-det".into(), decision: "approve".into(), note: None }).unwrap();
    let ops = outbox_ops(&pc).unwrap();
    let a = auth("dev-ppc", "stf-priya");

    // Revoked device → refused, nothing decided.
    s.execute("UPDATE device SET revoked_at='2026-10-20T00:00:00Z' WHERE id='dev-ppc'", []).unwrap();
    let r = &push(&mut s, &a, &ops)[0];
    assert_eq!((r.status, r.reason_code.as_deref()), (OpStatus::Rejected, Some("DEVICE_REVOKED_OR_UNKNOWN")));
    assert_eq!(request_status(&s, "req-det"), "pending");

    // Live device, but the Principal's account is suspended → flagged, not applied.
    s.execute("UPDATE device SET revoked_at=NULL WHERE id='dev-ppc'", []).unwrap();
    s.execute("UPDATE staff SET state='suspended' WHERE id='stf-priya'", []).unwrap();
    let r = &push(&mut s, &a, &ops)[0];
    assert_eq!((r.status, r.reason_code.as_deref()), (OpStatus::Flagged, Some("revoked_author")));
    assert_eq!(request_status(&s, "req-det"), "pending");
}

#[test]
fn a_duplicate_decision_op_applies_once() {
    let mut s = server();
    let req = pending_leave(&mut s);
    let mut pc = joined(&s, "dev-ppc", "stf-priya", "principal", "A8");
    request_command(&mut pc, &priya(), Some("dev-ppc"), DeviceMode::Client, &RequestCmd::Decide { id: req.clone(), decision: "approve".into(), note: None }).unwrap();
    let ops = outbox_ops(&pc).unwrap();
    let a = auth("dev-ppc", "stf-priya");

    // The same op delivered twice (LAN retry, then the Drive copy): one decision.
    let r1 = push(&mut s, &a, &ops)[0].clone();
    let r2 = push(&mut s, &a, &ops)[0].clone();
    assert_eq!(r1.status, OpStatus::Confirmed);
    assert_eq!(r1, r2, "identical retry returns the stored result");
    assert_eq!(count(&s, "SELECT COUNT(*) FROM leave_record WHERE request_id=?1", [&req]), 1);
    assert_eq!(count(&s, "SELECT COUNT(*) FROM audit_log WHERE action='approve_request'", []), 1);

    // A second, different op deciding the same request → ALREADY_DECIDED.
    let mut again = ops[0].clone();
    again.op_id = "op-second-decision".into();
    let r3 = &push(&mut s, &a, &[again])[0];
    assert_eq!((r3.status, r3.reason_code.as_deref()), (OpStatus::Rejected, Some("ALREADY_DECIDED")));
    assert_eq!(count(&s, "SELECT COUNT(*) FROM leave_record WHERE request_id=?1", [&req]), 1);
}

#[test]
fn money_staff_and_device_approvals_stay_on_the_school_pc() {
    let mut s = server();
    for (id, kind) in [("req-rev", "payment_reversal"), ("req-acc", "access_change"), ("req-dev", "device_replacement")] {
        s.execute(
            "INSERT INTO request(id,type,target_table,target_id,base_version,reason,requested_by,revision,status,apply_state,created_at,updated_at,sync_state) \
             VALUES (?1,?2,'payment','pay-x',0,'r','stf-suresh',1,'pending','not_applied','t','t','confirmed')",
            params![id, kind],
        )
        .unwrap();
    }
    let mut pc = joined(&s, "dev-ppc", "stf-priya", "principal", "A8");
    // Locally refused (no fake success)…
    for id in ["req-rev", "req-acc", "req-dev"] {
        let e = request_command(&mut pc, &priya(), Some("dev-ppc"), DeviceMode::Client, &RequestCmd::Decide { id: id.into(), decision: "approve".into(), note: None }).unwrap_err();
        assert_eq!(e.code, "SERVER_ONLY", "{id}");
    }
    assert!(outbox_ops(&pc).unwrap().is_empty());
    // …and a forged op approving a payment reversal is refused by the school PC.
    let a = auth("dev-ppc", "stf-priya");
    let r = &push(&mut s, &a, &[forged("op-rev", "dev-ppc", "stf-priya", "request", "req-rev",
        serde_json::to_value(RequestCmd::Decide { id: "req-rev".into(), decision: "approve".into(), note: None }).unwrap())])[0];
    assert_eq!((r.status, r.reason_code.as_deref()), (OpStatus::Rejected, Some("SERVER_ONLY")));
    assert_eq!(count(&s, "SELECT COUNT(*) FROM reversal", []), 0);
    assert_eq!(request_status(&s, "req-rev"), "pending");
    // Rejecting one from the PC is fine (no money moves).
    request_command(&mut pc, &priya(), Some("dev-ppc"), DeviceMode::Client, &RequestCmd::Decide { id: "req-rev".into(), decision: "reject".into(), note: None }).unwrap();
    let r = &push(&mut s, &a, &outbox_ops(&pc).unwrap())[0];
    assert_eq!(r.status, OpStatus::Confirmed, "{r:?}");
    assert_eq!(request_status(&s, "req-rev"), "rejected");
    assert_eq!(count(&s, "SELECT COUNT(*) FROM audit_log WHERE action='reject_request' AND device_id='dev-ppc'", []), 1);
}

// ---------------------------------------------------------------- store sales --

fn sale(item: &str, qty: i64) -> StoreSaleInput {
    StoreSaleInput { student_id: None, guardian_id: None, mode: "cash".into(), items: vec![SaleItemInput { item_id: item.into(), qty, price_paise: None }] }
}
fn stock(s: &Connection, item: &str) -> i64 {
    s.query_row("SELECT stock FROM store_item WHERE id=?1", [item], |r| r.get(0)).unwrap()
}

#[tokio::test]
async fn a_second_pc_store_sale_replayed_twice_is_one_sale_with_one_voucher() {
    let s = server();
    let before = stock(&s, "itm-book");
    let mut pc2 = joined(&s, "dev-acc2", "stf-suresh", "accountant", "A9");

    let dto = record_store_sale_logic(&mut pc2, &suresh(), Some("dev-acc2"), DeviceMode::Client, "2026-10-15", &sale("itm-book", 1)).unwrap();
    assert!(!dto.confirmed);
    assert!(dto.receipt_no.starts_with("S-A9-"), "the device's own series: {}", dto.receipt_no);
    assert_eq!(stock(&pc2, "itm-book"), before - 1, "the device's copy moved too");
    let ops = outbox_ops(&pc2).unwrap();
    assert_eq!((ops.len(), ops[0].table.as_str(), ops[0].kind.as_str()), (1, "store_sale", "action"));

    let server = Arc::new(Mutex::new(s));
    let a = auth("dev-acc2", "stf-suresh");
    {
        let mut s = server.lock().unwrap();
        // Delivered twice before the device saw an answer, plus a re-queued copy with a
        // new op id: still exactly one sale, one voucher, one stock decrement.
        push(&mut s, &a, &ops);
        let mut requeued = ops[0].clone();
        requeued.op_id = "op-requeued".into();
        assert_eq!(push(&mut s, &a, &[requeued])[0].status, OpStatus::Confirmed);
    }
    let t = LoopbackTransport::new(server.clone(), a.clone(), now());
    let out = sync_once(&mut pc2, &t).await.unwrap();
    assert_eq!((out.confirmed, out.rejected), (1, 0), "{out:?}");

    {
        let s = server.lock().unwrap();
        let (n, receipt, total): (i64, String, i64) = s
            .query_row("SELECT COUNT(*), MAX(receipt_no), MAX(total_paise) FROM store_sale WHERE id=?1", [&dto.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap();
        assert_eq!((n, receipt.as_str(), total), (1, dto.receipt_no.as_str(), dto.total_paise));
        assert_eq!(count(&s, "SELECT COUNT(*) FROM voucher WHERE source_table='store_sale' AND source_id=?1", [&dto.id]), 1, "one voucher");
        assert_eq!(count(&s, "SELECT COUNT(*) FROM store_sale WHERE id=?1 AND voucher_id IS NOT NULL", [&dto.id]), 1);
        assert_eq!(count(&s, "SELECT COUNT(*) FROM stock_move WHERE ref_id=?1", [&dto.id]), 1);
        assert_eq!(stock(&s, "itm-book"), before - 1, "decremented once");
        assert_eq!(count(&s, "SELECT COUNT(*) FROM review_flag WHERE ref_id=?1", [&dto.id]), 0, "nothing to review");
        assert_eq!(count(&s, "SELECT COUNT(*) FROM audit_log WHERE action='record_store_sale' AND device_id='dev-acc2'", []), 1);
    }
    // After the pull the PC2 holds ONE sale (now with its voucher) and one move.
    assert_eq!(count(&pc2, "SELECT COUNT(*) FROM store_sale WHERE id=?1 AND voucher_id IS NOT NULL", [&dto.id]), 1);
    assert_eq!(count(&pc2, "SELECT COUNT(*) FROM store_sale", []), 1);
    assert_eq!(count(&pc2, "SELECT COUNT(*) FROM stock_move WHERE ref_id=?1", [&dto.id]), 1);
}

#[test]
fn an_oversold_replay_is_recorded_and_flagged_not_refused() {
    let mut s = server();
    let start = stock(&s, "itm-notebook");
    assert!(start >= 3);
    let price: i64 = s.query_row("SELECT price_paise FROM store_item WHERE id='itm-notebook'", [], |r| r.get(0)).unwrap();
    let mut pc2 = joined(&s, "dev-acc2", "stf-suresh", "accountant", "A9");

    // PC2 sells all but nothing of what the school PC sells at the same time.
    let dto = record_store_sale_logic(&mut pc2, &suresh(), Some("dev-acc2"), DeviceMode::Client, "2026-10-15", &sale("itm-notebook", start)).unwrap();
    record_store_sale_logic(&mut s, &suresh(), None, DeviceMode::Server, "2026-10-15", &sale("itm-notebook", 2)).unwrap();
    // …and the Principal changed the price on the school PC meanwhile.
    s.execute("UPDATE store_item SET price_paise=price_paise+500 WHERE id='itm-notebook'", []).unwrap();

    let r = &push(&mut s, &auth("dev-acc2", "stf-suresh"), &outbox_ops(&pc2).unwrap())[0];
    assert_eq!(r.status, OpStatus::Confirmed, "money already taken is never refused: {r:?}");
    let total: i64 = s.query_row("SELECT total_paise FROM store_sale WHERE id=?1", [&dto.id], |r| r.get(0)).unwrap();
    assert_eq!(total, price * start, "recorded at the price the device charged");
    assert_eq!(count(&s, "SELECT COUNT(*) FROM voucher WHERE source_table='store_sale' AND source_id=?1", [&dto.id]), 1);
    assert_eq!(stock(&s, "itm-notebook"), -2, "stock may go negative");
    let details: String = s
        .query_row("SELECT details_json FROM review_flag WHERE kind='store_oversold' AND ref_id=?1 AND status='open'", [&dto.id], |r| r.get(0))
        .unwrap();
    assert!(details.contains("itm-notebook") && details.contains("-2"), "{details}");
    assert_eq!(count(&s, "SELECT COUNT(*) FROM review_flag WHERE kind='store_sale_review' AND ref_id=?1", [&dto.id]), 1, "price change flagged");

    // A purchase that only partly covers the deficit is still recordable.
    let item = stock_adjust_logic(&mut s, &priya(), "2026-10-16", &StockAdjustInput { item_id: "itm-notebook".into(), delta_qty: 1, reason: "purchase".into() }).unwrap();
    assert_eq!(item.stock, -1);
}

#[test]
fn store_sale_ops_respect_the_module_switch_role_and_id() {
    let mut s = server();
    let mut pc2 = joined(&s, "dev-acc2", "stf-suresh", "accountant", "A9");
    let dto = record_store_sale_logic(&mut pc2, &suresh(), Some("dev-acc2"), DeviceMode::Client, "2026-10-15", &sale("itm-book", 1)).unwrap();
    let ops = outbox_ops(&pc2).unwrap();
    let a = auth("dev-acc2", "stf-suresh");

    // The Principal switched the store off on the school PC → refused, nothing written.
    s.execute("UPDATE module_setting SET enabled=0 WHERE key='store'", []).unwrap();
    let r = &push(&mut s, &a, &ops)[0];
    assert_eq!((r.status, r.reason_code.as_deref()), (OpStatus::Rejected, Some("MODULE_OFF")));
    assert_eq!(count(&s, "SELECT COUNT(*) FROM store_sale WHERE id=?1", [&dto.id]), 0);
    s.execute("UPDATE module_setting SET enabled=1 WHERE key='store'", []).unwrap();

    // Off as the device last heard from the school PC → refused locally.
    pc2.execute("UPDATE school SET settings_json=json_set(settings_json,'$.modules.store',json('false'))", []).unwrap();
    let e = record_store_sale_logic(&mut pc2, &suresh(), Some("dev-acc2"), DeviceMode::Client, "2026-10-15", &sale("itm-book", 1)).unwrap_err();
    assert_eq!(e.code, "MODULE_OFF");

    // A teacher may not sell; an op whose record isn't the sale id is malformed.
    let cmd = ClientCmd::RecordStoreSale {
        id: "sale-forged".into(), today: "2026-10-15".into(), receipt_no: "S-X-0001".into(),
        input: StoreSaleInput { items: vec![SaleItemInput { item_id: "itm-book".into(), qty: 1, price_paise: Some(1) }], ..sale("itm-book", 1) },
    };
    let r = &push(&mut s, &auth("dev-phone", "stf-meena"), &[forged("op-t-sale", "dev-phone", "stf-meena", "store_sale", "sale-forged", serde_json::to_value(&cmd).unwrap())])[0];
    assert_eq!(r.reason_code.as_deref(), Some("FORBIDDEN"), "{r:?}");
    let r = &push(&mut s, &a, &[forged("op-bad-id", "dev-acc2", "stf-suresh", "store_sale", "sale-other", serde_json::to_value(&cmd).unwrap())])[0];
    assert_eq!(r.reason_code.as_deref(), Some("MALFORMED"));
    assert_eq!(count(&s, "SELECT COUNT(*) FROM store_sale WHERE id IN ('sale-forged','sale-other')", []), 0);
}

#[test]
fn a_decision_and_a_store_sale_also_arrive_over_drive() {
    use base64::Engine;
    use vidya_lib::sync::drive::exchange;
    use vidya_lib::sync::drive::fake::{FakeDrive, PRINCIPAL};
    let mut s = server();
    let sk_p = base64::engine::general_purpose::STANDARD.encode([0x62u8; 32]);
    let sk_a = base64::engine::general_purpose::STANDARD.encode([0x63u8; 32]);
    s.execute("UPDATE device SET session_key=?1 WHERE id='dev-ppc'", [&sk_p]).unwrap();
    s.execute("UPDATE device SET session_key=?1 WHERE id='dev-acc2'", [&sk_a]).unwrap();
    let mut pc = joined(&s, "dev-ppc", "stf-priya", "principal", "A8");
    let mut pc2 = joined(&s, "dev-acc2", "stf-suresh", "accountant", "A9");
    request_command(&mut pc, &priya(), Some("dev-ppc"), DeviceMode::Client, &RequestCmd::Decide { id: "req-det".into(), decision: "reject".into(), note: None }).unwrap();
    let dto = record_store_sale_logic(&mut pc2, &suresh(), Some("dev-acc2"), DeviceMode::Client, "2026-10-15", &sale("itm-book", 1)).unwrap();

    let drive = FakeDrive::new();
    let l = drive.provision_school(&[("stf-priya", "dev-ppc"), ("stf-suresh", "dev-acc2")]);
    exchange::push_outbox(&mut pc, &drive.as_actor("stf-priya"), &l.ops["dev-ppc"], "dev-ppc", &sk_p).unwrap();
    exchange::push_outbox(&mut pc2, &drive.as_actor("stf-suresh"), &l.ops["dev-acc2"], "dev-acc2", &sk_a).unwrap();
    let out = exchange::import_all(&mut s, &drive.as_actor(PRINCIPAL), &l.exchange).unwrap();
    assert_eq!(out.confirmed, 2, "{out:?}");
    // A second import pass of the same folders changes nothing.
    exchange::import_all(&mut s, &drive.as_actor(PRINCIPAL), &l.exchange).unwrap();
    assert_eq!(request_status(&s, "req-det"), "rejected");
    assert_eq!(count(&s, "SELECT COUNT(*) FROM store_sale WHERE id=?1", [&dto.id]), 1);
    assert_eq!(count(&s, "SELECT COUNT(*) FROM voucher WHERE source_table='store_sale' AND source_id=?1", [&dto.id]), 1);
}
