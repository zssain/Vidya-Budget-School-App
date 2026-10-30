//! Attendance end to end across devices (P21, spec Phases 5–6). A teacher's phone
//! (a joined client) takes attendance; the change leaves as ONE `attendance_sheet`
//! action op stamped with the phone's real device id + packed HLC; the school PC
//! applies it atomically through the same domain write, and another device pulls the
//! canonical result. Plus the sheet rules: enrolment set, no duplicates, no other
//! class's students, all-or-nothing validation, submitted-sheet lock.

use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};
use time::OffsetDateTime;

use vidya_lib::commands::logic::{attendance_write, MarkInput};
use vidya_lib::server::service::{self, DeviceAuth};
use vidya_lib::state::SessionStaff;
use vidya_lib::sync::apply::load_actor;
use vidya_lib::sync::client::{self, ClientIdentity};
use vidya_lib::sync::engine::{apply_snapshot, sync_once};
use vidya_lib::sync::protocol::OpStatus;
use vidya_lib::sync::scope;
use vidya_lib::sync::transport::LoopbackTransport;
use vidya_lib::write::DeviceMode;
use vidya_lib::{db, seed};

const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const DAY: &str = "2026-10-15";

fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

fn meena() -> SessionStaff {
    SessionStaff { id: "stf-meena".into(), name: "Meena Iyer".into(), role: "teacher".into() }
}

fn server() -> Connection {
    let mut c = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut c).unwrap();
    seed::seed_demo_school(&mut c, OffsetDateTime::now_utc()).unwrap();
    c.execute(
        "INSERT INTO device(id,staff_id,platform,name,token_hash) VALUES ('dev-phone','stf-meena','android','Phone','h')",
        [],
    )
    .unwrap();
    c
}

/// A joined phone for `staff` with the server's role-scoped snapshot applied.
fn joined_phone(server: &Connection, device: &str, staff: &str) -> Connection {
    let mut c = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut c).unwrap();
    let actor = load_actor(server, staff).unwrap().unwrap();
    apply_snapshot(&mut c, &scope::snapshot(server, &actor).unwrap()).unwrap();
    client::store(
        &c,
        &ClientIdentity {
            device_id: device.into(),
            device_token: "tok".into(),
            staff_id: staff.into(),
            staff_name: "x".into(),
            staff_role: "teacher".into(),
            session_key: String::new(),
            school_id: "sch-demo".into(),
            school_name: "Demo".into(),
            receipt_series: "A9".into(),
            admission_series: "A9".into(),
            audience_keys: vec![],
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

fn roster(conn: &Connection, class_id: &str) -> Vec<String> {
    let mut st = conn
        .prepare("SELECT student_id FROM enrollment WHERE class_id=?1 AND from_date<=?2 AND (to_date IS NULL OR to_date>=?2) ORDER BY student_id")
        .unwrap();
    st.query_map(params![class_id, DAY], |r| r.get(0)).unwrap().collect::<Result<_, _>>().unwrap()
}

fn marks(ids: &[String], mark: &str) -> Vec<MarkInput> {
    ids.iter().map(|id| MarkInput { student_id: id.clone(), mark: mark.into() }).collect()
}

#[tokio::test]
async fn phone_attendance_reaches_the_school_pc_and_other_devices() {
    let server = server();
    let ids = roster(&server, "cls-5a");
    assert!(!ids.is_empty());
    let mut phone = joined_phone(&server, "dev-phone", "stf-meena");

    // The teacher submits on the phone (client mode).
    let mut ms = marks(&ids, "P");
    ms[0].mark = "A".into();
    attendance_write(&mut phone, &meena(), Some("dev-phone"), DeviceMode::Client, "cls-5a", DAY, &ms, true).unwrap();

    // ONE complete action op, stamped with the phone's identity and a packed HLC.
    let (n, dev, hlc, kind, epoch): (i64, String, String, String, i64) = phone
        .query_row("SELECT COUNT(*), device_id, hlc, kind, server_epoch FROM outbox", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap();
    assert_eq!((n, dev.as_str(), kind.as_str(), epoch), (1, "dev-phone", "action", 1));
    assert!(vidya_core::hlc::Hlc::parse(&hlc).is_ok(), "packed HLC, not an ISO timestamp: {hlc}");

    // Push over the (loopback) LAN as the authenticated phone → confirmed.
    let server = Arc::new(Mutex::new(server));
    let t = LoopbackTransport::new(server.clone(), DeviceAuth { device_id: "dev-phone".into(), staff_id: "stf-meena".into() }, now());
    let out = sync_once(&mut phone, &t).await.unwrap();
    assert_eq!((out.confirmed, out.rejected), (1, 0));
    let left: i64 = phone.query_row("SELECT COUNT(*) FROM outbox", [], |r| r.get(0)).unwrap();
    assert_eq!(left, 0, "outbox drained");

    // The school PC holds the submitted sheet with every mark.
    {
        let s = server.lock().unwrap();
        let (status, by): (String, String) = s
            .query_row("SELECT status, submitted_by FROM attendance_sheet WHERE class_id='cls-5a' AND date=?1", [DAY], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!((status.as_str(), by.as_str()), ("submitted", "stf-meena"));
        let cnt: i64 = s
            .query_row("SELECT COUNT(*) FROM attendance_mark m JOIN attendance_sheet sh ON sh.id=m.sheet_id WHERE sh.class_id='cls-5a' AND sh.date=?1", [DAY], |r| r.get(0))
            .unwrap();
        assert_eq!(cnt as usize, ids.len());
    }

    // The Principal's device pulls the canonical sheet + marks.
    let mut p_dev = {
        let s = server.lock().unwrap();
        s.execute("INSERT INTO device(id,staff_id,platform,name,token_hash) VALUES ('dev-p','stf-priya','android','P','h')", []).unwrap();
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        c
    };
    let tp = LoopbackTransport::new(server.clone(), DeviceAuth { device_id: "dev-p".into(), staff_id: "stf-priya".into() }, now());
    sync_once(&mut p_dev, &tp).await.unwrap();
    let pulled: i64 = p_dev
        .query_row("SELECT COUNT(*) FROM attendance_mark WHERE student_id=?1 AND mark='A'", [&ids[0]], |r| r.get(0))
        .unwrap();
    assert!(pulled >= 1, "another device received the canonical mark");
}

#[test]
fn sheet_rules_are_enforced_before_any_write() {
    let mut s = server();
    let ids = roster(&s, "cls-5a");
    let other: String = s
        .query_row("SELECT student_id FROM enrollment WHERE class_id<>'cls-5a' AND to_date IS NULL LIMIT 1", [], |r| r.get(0))
        .unwrap();
    let w = |s: &mut Connection, ms: &[MarkInput], submit: bool| attendance_write(s, &meena(), None, DeviceMode::Server, "cls-5a", DAY, ms, submit);

    // Duplicate entries padding the count to the class size.
    let mut dup = marks(&ids, "P");
    let last = dup.len() - 1;
    dup[last].student_id = ids[0].clone();
    assert!(w(&mut s, &dup, true).is_err());
    // Another class's student.
    let mut foreign = marks(&ids, "P");
    foreign[0].student_id = other.clone();
    assert!(w(&mut s, &foreign, true).is_err());
    // Missing students on submit.
    assert_eq!(w(&mut s, &marks(&ids[1..], "P"), true).unwrap_err().code, "INCOMPLETE_SHEET");
    // A legacy/invalid final mark after valid ones → nothing written at all.
    let mut bad = marks(&ids, "P");
    bad[last].mark = "L".into();
    assert!(w(&mut s, &bad, true).is_err());
    let n: i64 = s.query_row("SELECT COUNT(*) FROM attendance_sheet WHERE class_id='cls-5a' AND date=?1", [DAY], |r| r.get(0)).unwrap();
    assert_eq!(n, 0, "no partial sheet from any rejected attempt");

    // A valid submit, then a direct draft save of the submitted sheet is refused.
    w(&mut s, &marks(&ids, "P"), true).unwrap();
    assert_eq!(w(&mut s, &marks(&ids, "A"), false).unwrap_err().code, "SHEET_LOCKED");
    let absent: i64 = s
        .query_row("SELECT COUNT(*) FROM attendance_mark m JOIN attendance_sheet sh ON sh.id=m.sheet_id WHERE sh.class_id='cls-5a' AND sh.date=?1 AND m.mark='A'", [DAY], |r| r.get(0))
        .unwrap();
    assert_eq!(absent, 0, "locked sheet unchanged");
}

#[test]
fn a_teacher_cannot_take_another_class_attendance_via_sync() {
    let mut s = server();
    // A class Meena is not the class teacher of (and holds no duty for).
    let class: String = s
        .query_row("SELECT id FROM class WHERE class_teacher_id<>'stf-meena' LIMIT 1", [], |r| r.get(0))
        .unwrap();
    let ids = roster(&s, &class);
    let op = vidya_lib::sync::protocol::Op {
        op_id: "op-att-x".into(),
        hlc: "0001700000000000000-dev-phone".into(),
        device_id: "dev-phone".into(),
        staff_id: "stf-meena".into(),
        audience: format!("class:{class}"),
        table: "attendance_sheet".into(),
        record_id: format!("sheet-{class}-{DAY}"),
        kind: "action".into(),
        payload: serde_json::json!({ "class_id": class, "date": DAY, "submit": true,
            "marks": ids.iter().map(|i| serde_json::json!({ "student_id": i, "mark": "P" })).collect::<Vec<_>>() }),
        base_version: None,
        server_epoch: 1,
    };
    let auth = DeviceAuth { device_id: "dev-phone".into(), staff_id: "stf-meena".into() };
    let r = service::push(&mut s, &auth, std::slice::from_ref(&op), now()).unwrap().results[0].clone();
    assert_eq!(r.status, OpStatus::Rejected);
    assert_eq!(r.reason_code.as_deref(), Some("FORBIDDEN"), "refused for permission, not malformation");
    let n: i64 = s.query_row("SELECT COUNT(*) FROM attendance_sheet WHERE class_id=?1 AND date=?2", params![class, DAY], |r| r.get(0)).unwrap();
    assert_eq!(n, 0);
}

// ---------------------------------------------------------------------------
// Requests (leave / correction / cancel) raised on a phone reach the school PC.

use vidya_lib::commands::logic::{request_command, LeaveRequestInput, RequestCmd};

#[tokio::test]
async fn phone_leave_request_and_cancel_reach_the_school_pc() {
    let server = server();
    let leave_type: String = server.query_row("SELECT id FROM leave_type WHERE active=1 LIMIT 1", [], |r| r.get(0)).unwrap();
    let mut phone = joined_phone(&server, "dev-phone", "stf-meena");

    let cmd = RequestCmd::Leave {
        id: "req-phone-1".into(),
        input: LeaveRequestInput { leave_type_id: leave_type, from: "2026-10-19".into(), to: "2026-10-20".into(), reason: "Family function".into() },
    };
    let dto = request_command(&mut phone, &meena(), Some("dev-phone"), DeviceMode::Client, &cmd).unwrap().unwrap();
    assert_eq!(dto.id, "req-phone-1");
    let kind: String = phone.query_row("SELECT kind FROM outbox WHERE \"table\"='request'", [], |r| r.get(0)).unwrap();
    assert_eq!(kind, "action");

    let server = Arc::new(Mutex::new(server));
    let t = LoopbackTransport::new(server.clone(), DeviceAuth { device_id: "dev-phone".into(), staff_id: "stf-meena".into() }, now());
    let out = sync_once(&mut phone, &t).await.unwrap();
    assert_eq!((out.confirmed, out.rejected), (1, 0));
    let (status, by): (String, String) = server
        .lock()
        .unwrap()
        .query_row("SELECT status, requested_by FROM request WHERE id='req-phone-1'", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!((status.as_str(), by.as_str()), ("pending", "stf-meena"), "same id, pending, on the school PC");

    // Cancel it from the phone.
    request_command(&mut phone, &meena(), Some("dev-phone"), DeviceMode::Client, &RequestCmd::Cancel { id: "req-phone-1".into() }).unwrap();
    sync_once(&mut phone, &t).await.unwrap();
    let status: String = server.lock().unwrap().query_row("SELECT status FROM request WHERE id='req-phone-1'", [], |r| r.get(0)).unwrap();
    assert_eq!(status, "cancelled");
}

#[test]
fn a_teacher_cannot_cancel_someone_elses_request_via_sync() {
    let mut s = server();
    s.execute(
        "INSERT INTO request(id,type,target_table,target_id,base_version,reason,requested_by,revision,status,apply_state,created_at,updated_at,sync_state) \
         VALUES ('req-other','student_details','student','stu-x',1,'r','stf-anita',1,'pending','not_applied','t','t','confirmed')",
        [],
    )
    .unwrap();
    let op = vidya_lib::sync::protocol::Op {
        op_id: "op-cancel-x".into(),
        hlc: "0001700000000000000-dev-phone".into(),
        device_id: "dev-phone".into(),
        staff_id: "stf-meena".into(),
        audience: "admin".into(),
        table: "request".into(),
        record_id: "req-other".into(),
        kind: "action".into(),
        payload: serde_json::to_value(RequestCmd::Cancel { id: "req-other".into() }).unwrap(),
        base_version: None,
        server_epoch: 1,
    };
    let auth = DeviceAuth { device_id: "dev-phone".into(), staff_id: "stf-meena".into() };
    let r = service::push(&mut s, &auth, std::slice::from_ref(&op), now()).unwrap().results[0].clone();
    assert_eq!(r.status, OpStatus::Rejected);
    assert_eq!(r.reason_code.as_deref(), Some("FORBIDDEN"));
    let status: String = s.query_row("SELECT status FROM request WHERE id='req-other'", [], |r| r.get(0)).unwrap();
    assert_eq!(status, "pending");
}

// ---------------------------------------------------------------------------
// Staff check-in: replayed as the author; the ROUTE comes from how it arrived.

use vidya_lib::commands::logic::{staff_check_in_logic, CheckInInput};

fn check_in_input(route: &str) -> CheckInInput {
    CheckInInput { route: route.into(), date: "2026-11-03".into(), minute: 8 * 60 + 20, device_ms: OffsetDateTime::now_utc().unix_timestamp() * 1000 }
}

#[tokio::test]
async fn lan_check_in_counts_at_school() {
    let server = server();
    let mut phone = joined_phone(&server, "dev-phone", "stf-meena");
    staff_check_in_logic(&mut phone, &meena(), Some("dev-phone"), DeviceMode::Client, &check_in_input("lan")).unwrap();
    let server = Arc::new(Mutex::new(server));
    let t = LoopbackTransport::new(server.clone(), DeviceAuth { device_id: "dev-phone".into(), staff_id: "stf-meena".into() }, now());
    let out = sync_once(&mut phone, &t).await.unwrap();
    assert_eq!((out.confirmed, out.rejected), (1, 0));
    let (status, min, route): (String, i64, String) = server
        .lock()
        .unwrap()
        .query_row("SELECT status, check_in_min, route FROM staff_attendance WHERE staff_id='stf-meena' AND date='2026-11-03'", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap();
    assert_eq!((status.as_str(), min, route.as_str()), ("present", 500, "lan"), "full row on the school PC");
}

#[test]
fn a_drive_check_in_claiming_lan_is_recorded_as_away() {
    use base64::Engine;
    use vidya_lib::sync::drive::exchange;
    use vidya_lib::sync::drive::fake::{FakeDrive, PRINCIPAL};
    let mut s = server();
    let sk = base64::engine::general_purpose::STANDARD.encode([0x61u8; 32]);
    s.execute("UPDATE device SET session_key=?1 WHERE id='dev-phone'", [&sk]).unwrap();
    let mut phone = joined_phone(&s, "dev-phone", "stf-meena");
    // The phone CLAIMS it is on the school Wi-Fi…
    staff_check_in_logic(&mut phone, &meena(), Some("dev-phone"), DeviceMode::Client, &check_in_input("lan")).unwrap();
    // …but the op actually travels through Google Drive.
    let drive = FakeDrive::new();
    let l = drive.provision_school(&[("stf-meena", "dev-phone")]);
    exchange::push_outbox(&mut phone, &drive.as_actor("stf-meena"), &l.ops["dev-phone"], "dev-phone", &sk).unwrap();
    let out = exchange::import_all(&mut s, &drive.as_actor(PRINCIPAL), &l.exchange).unwrap();
    assert_eq!(out.confirmed, 1);
    let (status, route): (String, String) = s
        .query_row("SELECT status, route FROM staff_attendance WHERE staff_id='stf-meena' AND date='2026-11-03'", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!(route, "drive", "server-side route, not the device's claim");
    assert_ne!(status, "present", "not counted 'at school' without the LAN");
}

// ---------------------------------------------------------------------------
// Marks: one action op with the whole subject sheet; entries reach the school PC.

use vidya_lib::commands::logic::{upsert_marks, MarkEntryInput};

#[tokio::test]
async fn phone_marks_reach_the_school_pc_and_submitted_sheet_locks() {
    let server = server();
    // An exam subject Meena teaches whose sheet is not yet submitted.
    let es: String = server
        .query_row(
            "SELECT es.id FROM exam_subject es JOIN class_subject cs ON cs.id=es.class_subject_id \
             LEFT JOIN marks_sheet ms ON ms.exam_subject_id=es.id \
             WHERE cs.teacher_id='stf-meena' AND (ms.status IS NULL OR ms.status='draft') LIMIT 1",
            [], |r| r.get(0),
        )
        .unwrap();
    let class: String = server
        .query_row("SELECT cs.class_id FROM exam_subject es JOIN class_subject cs ON cs.id=es.class_subject_id WHERE es.id=?1", [&es], |r| r.get(0))
        .unwrap();
    let students: Vec<String> = {
        let mut st = server.prepare("SELECT student_id FROM enrollment WHERE class_id=?1 AND to_date IS NULL").unwrap();
        st.query_map([&class], |r| r.get(0)).unwrap().collect::<Result<_, _>>().unwrap()
    };
    let entries: Vec<MarkEntryInput> = students.iter().map(|s| MarkEntryInput { student_id: s.clone(), marks: Some(37), absent: false }).collect();
    let mut phone = joined_phone(&server, "dev-phone", "stf-meena");
    upsert_marks(&mut phone, &meena(), Some("dev-phone"), DeviceMode::Client, &es, &entries, true).unwrap();

    let server = Arc::new(Mutex::new(server));
    let t = LoopbackTransport::new(server.clone(), DeviceAuth { device_id: "dev-phone".into(), staff_id: "stf-meena".into() }, now());
    let out = sync_once(&mut phone, &t).await.unwrap();
    assert_eq!((out.confirmed, out.rejected), (1, 0));
    let s = server.lock().unwrap();
    let (status, n): (String, i64) = s
        .query_row(
            "SELECT ms.status, (SELECT COUNT(*) FROM mark_entry me WHERE me.sheet_id=ms.id AND me.marks=37) FROM marks_sheet ms WHERE ms.exam_subject_id=?1",
            [&es], |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((status.as_str(), n as usize), ("submitted", students.len()), "every entry on the school PC");
    drop(s);

    // A later direct edit of the submitted sheet (e.g. a second phone) is refused.
    let mut again = entries.clone();
    again[0].marks = Some(10);
    let op = vidya_lib::sync::protocol::Op {
        op_id: "op-marks-late".into(),
        hlc: "0001800000000000000-dev-phone".into(),
        device_id: "dev-phone".into(),
        staff_id: "stf-meena".into(),
        audience: format!("class:{class}"),
        table: "marks_sheet".into(),
        record_id: format!("msheet-{es}"),
        kind: "action".into(),
        payload: serde_json::json!({ "exam_subject_id": es, "submit": false, "entries": again }),
        base_version: None,
        server_epoch: 1,
    };
    let mut s = server.lock().unwrap();
    let auth = DeviceAuth { device_id: "dev-phone".into(), staff_id: "stf-meena".into() };
    let r = service::push(&mut s, &auth, std::slice::from_ref(&op), now()).unwrap().results[0].clone();
    assert_eq!(r.reason_code.as_deref(), Some("NEEDS_REQUEST"));
}

// ---------------------------------------------------------------------------
// Payments from an accountant's second PC (§8.6 / §8.7).

use vidya_lib::commands::logic::{record_payment_logic, PaymentInput};

#[tokio::test]
async fn second_pc_payment_uses_its_own_series_and_lands_once_with_a_voucher() {
    let server = server();
    server.execute("INSERT INTO device(id,staff_id,platform,name,token_hash,receipt_series) VALUES ('dev-acc2','stf-suresh','windows','PC2','h','A9')", []).unwrap();
    let student: String = server.query_row("SELECT student_id FROM fee_due WHERE cancelled_at IS NULL LIMIT 1", [], |r| r.get(0)).unwrap();
    let mut pc2 = joined_phone(&server, "dev-acc2", "stf-suresh"); // identity series = A9
    let suresh = SessionStaff { id: "stf-suresh".into(), name: "Suresh".into(), role: "accountant".into() };

    let dto = record_payment_logic(&mut pc2, &suresh, Some("dev-acc2"), DeviceMode::Client, &PaymentInput {
        student_id: student.clone(), amount_paise: 10_000, mode: "cash".into(), reference: None,
    })
    .unwrap();
    assert!(dto.receipt_no.starts_with("R-A9-"), "the device's own series, not the school PC's A1: {}", dto.receipt_no);

    let server = Arc::new(Mutex::new(server));
    let t = LoopbackTransport::new(server.clone(), DeviceAuth { device_id: "dev-acc2".into(), staff_id: "stf-suresh".into() }, now());
    // Re-deliver the same outbox twice (a retry after a lost response): money lands once.
    let op = pc2.query_row("SELECT op_id FROM outbox", [], |r| r.get::<_, String>(0)).unwrap();
    let pending: Vec<vidya_lib::sync::protocol::Op> = vidya_lib::sync::engine::outbox_ops(&pc2).unwrap();
    {
        let mut s = server.lock().unwrap();
        let auth = DeviceAuth { device_id: "dev-acc2".into(), staff_id: "stf-suresh".into() };
        service::push(&mut s, &auth, &pending, now()).unwrap();
    }
    let out = sync_once(&mut pc2, &t).await.unwrap();
    assert_eq!(out.rejected, 0);

    let s = server.lock().unwrap();
    let (n, receipt): (i64, String) = s
        .query_row("SELECT COUNT(*), MAX(receipt_no) FROM payment WHERE id=?1", [&dto.id], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!((n, receipt.as_str()), (1, dto.receipt_no.as_str()), "exactly one payment, same receipt number");
    let alloc: i64 = s.query_row("SELECT COALESCE(SUM(amount_paise),0) FROM payment_allocation WHERE payment_id=?1", [&dto.id], |r| r.get(0)).unwrap();
    assert_eq!(alloc, 10_000, "fully allocated on the school PC");
    let vouchers: i64 = s.query_row("SELECT COUNT(*) FROM voucher WHERE source_table='payment' AND source_id=?1", [&dto.id], |r| r.get(0)).unwrap();
    assert_eq!(vouchers, 1, "one balanced voucher, posted with the payment");
    let applied: i64 = s.query_row("SELECT COUNT(*) FROM applied_ops WHERE op_id=?1", [&op], |r| r.get(0)).unwrap();
    assert_eq!(applied, 1);
}

// ---------------------------------------------------------------------------
// Homework note from the native phone app: the complete row reaches the school PC.

use vidya_lib::commands::logic::{save_homework_note_logic, HomeworkNoteInput};

#[tokio::test]
async fn phone_homework_note_lands_complete() {
    let server = server();
    let mut phone = joined_phone(&server, "dev-phone", "stf-meena");
    let input: HomeworkNoteInput = serde_json::from_value(serde_json::json!({
        "class_id": "cls-5a", "class_subject_id": "cs-5a-eng", "kind": "homework", "text": "Read chapter 4",
    }))
    .unwrap();
    let dto = save_homework_note_logic(&mut phone, &meena(), Some("dev-phone"), DeviceMode::Client, &input).unwrap();
    let server = Arc::new(Mutex::new(server));
    let t = LoopbackTransport::new(server.clone(), DeviceAuth { device_id: "dev-phone".into(), staff_id: "stf-meena".into() }, now());
    let out = sync_once(&mut phone, &t).await.unwrap();
    assert_eq!((out.confirmed, out.rejected), (1, 0));
    let (kind, text, by): (String, String, String) = server
        .lock()
        .unwrap()
        .query_row("SELECT kind, text, created_by FROM homework_note WHERE id=?1", [&dto.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap();
    assert_eq!((kind.as_str(), text.as_str(), by.as_str()), ("homework", "Read chapter 4", "stf-meena"));
}

// ---------------------------------------------------------------------------
// Teacher phone: absence alert, circular read, report remark, delete own note.

use vidya_lib::commands::logic::{delete_homework_note_with, mark_circular_read_logic, record_message_logic, save_report_remark_logic, RecordMessageInput, ReportRemarkInput};

#[tokio::test]
async fn teacher_phone_messages_reads_remarks_and_note_deletes_sync() {
    let server = server();
    let student = roster(&server, "cls-5a")[0].clone();
    // Fixtures the seed lacks: messages consent, a sent circular, an exam.
    server.execute("INSERT INTO consent(id,student_id,purpose,method,recorded_at,recorded_by) VALUES ('cn1',?1,'messages','signed_form','t','stf-priya')", [&student]).unwrap();
    server.execute("INSERT INTO circular(id,number,title,body,status,created_at,updated_at) VALUES ('circ-1','C/1','Holiday','Closed Friday','sent','t','t')", []).unwrap();
    let exam: String = server.query_row("SELECT id FROM exam LIMIT 1", [], |r| r.get(0)).unwrap();
    let mut phone = joined_phone(&server, "dev-phone", "stf-meena");

    // 1) Absence alert (tap-to-WhatsApp) for a student in her class.
    let msg = record_message_logic(&mut phone, &meena(), Some("dev-phone"), DeviceMode::Client, &serde_json::from_value::<RecordMessageInput>(serde_json::json!({
        "channel": "wa_tap", "kind": "absence_alert", "language": "en", "to_address": "9820011111",
        "body": "Absent today", "related_table": "student", "related_id": student,
    })).unwrap()).unwrap();
    // 2) Circular read. 3) Report remark. 4) Own note created then deleted.
    mark_circular_read_logic(&mut phone, &meena(), Some("dev-phone"), DeviceMode::Client, "circ-1").unwrap();
    save_report_remark_logic(&mut phone, &meena(), Some("dev-phone"), DeviceMode::Client, &ReportRemarkInput {
        exam_id: exam.clone(), student_id: student.clone(), text: "Works hard".into(), template_key: None,
    }).unwrap();
    let note = vidya_lib::commands::logic::save_homework_note_logic(&mut phone, &meena(), Some("dev-phone"), DeviceMode::Client,
        &serde_json::from_value(serde_json::json!({ "class_id": "cls-5a", "class_subject_id": "cs-5a-eng", "kind": "notes", "text": "x" })).unwrap()).unwrap();

    let server = Arc::new(Mutex::new(server));
    let t = LoopbackTransport::new(server.clone(), DeviceAuth { device_id: "dev-phone".into(), staff_id: "stf-meena".into() }, now());
    let out = sync_once(&mut phone, &t).await.unwrap();
    assert_eq!((out.confirmed, out.rejected), (4, 0), "{out:?}");

    delete_homework_note_with(&mut phone, &meena(), Some("dev-phone"), DeviceMode::Client, &note.id).unwrap();
    let out = sync_once(&mut phone, &t).await.unwrap();
    assert_eq!((out.confirmed, out.rejected), (1, 0));

    {
        let s = server.lock().unwrap();
        let st: String = s.query_row("SELECT status FROM message WHERE id=?1", [&msg.id], |r| r.get(0)).unwrap();
        assert_eq!(st, "tapped", "same message id on the school PC, honest status");
        let read: i64 = s.query_row("SELECT COUNT(*) FROM circular_read WHERE circular_id='circ-1' AND staff_id='stf-meena' AND read_at IS NOT NULL", [], |r| r.get(0)).unwrap();
        assert_eq!(read, 1);
        let text: String = s.query_row("SELECT text FROM report_remark WHERE exam_id=?1 AND student_id=?2", params![exam, student], |r| r.get(0)).unwrap();
        assert_eq!(text, "Works hard");
        let notes: i64 = s.query_row("SELECT COUNT(*) FROM homework_note WHERE id=?1", [&note.id], |r| r.get(0)).unwrap();
        assert_eq!(notes, 0, "deleted on the school PC");
    }

    // Another teacher device that had the note receives the tombstone.
    let mut other = joined_phone(&server.lock().unwrap(), "dev-phone2", "stf-meena");
    other.execute("INSERT INTO homework_note(id,class_id,kind,text,created_at,updated_at) VALUES (?1,'cls-5a','notes','x','t','t')", [&note.id]).unwrap();
    server.lock().unwrap().execute("INSERT INTO device(id,staff_id,platform,name,token_hash) VALUES ('dev-phone2','stf-meena','android','P2','h')", []).unwrap();
    let t2 = LoopbackTransport::new(server.clone(), DeviceAuth { device_id: "dev-phone2".into(), staff_id: "stf-meena".into() }, now());
    sync_once(&mut other, &t2).await.unwrap();
    let left: i64 = other.query_row("SELECT COUNT(*) FROM homework_note WHERE id=?1", [&note.id], |r| r.get(0)).unwrap();
    assert_eq!(left, 0, "tombstone applied on another device");
}

// ---------------------------------------------------------------------------
// Accountant's second PC: admission (provisional → official) and an expense.

use vidya_lib::commands::logic::{create_student_logic, record_expense_logic, ExpenseInput, NewStudentInput};

#[tokio::test]
async fn second_pc_admission_and_expense_reach_the_school_pc() {
    let server = server();
    server.execute("INSERT INTO device(id,staff_id,platform,name,token_hash,receipt_series,admission_series) VALUES ('dev-acc2','stf-suresh','windows','PC2','h','A9','A9')", []).unwrap();
    let account: String = server
        .query_row("SELECT id FROM ledger_account WHERE kind='expense' LIMIT 1", [], |r| r.get(0))
        .unwrap();
    let mut pc2 = joined_phone(&server, "dev-acc2", "stf-suresh");
    let suresh = SessionStaff { id: "stf-suresh".into(), name: "Suresh".into(), role: "accountant".into() };

    let input: NewStudentInput = serde_json::from_value(serde_json::json!({
        "name": "Offline Admit", "class_id": "cls-5a", "guardian_name": "Parent", "guardian_mobile": "9876500011",
        "transport": false, "rte": false,
    }))
    .unwrap();
    let stu = create_student_logic(&mut pc2, &suresh, Some("dev-acc2"), DeviceMode::Client, "2026-10-15", &input).unwrap();
    let prov: String = pc2.query_row("SELECT provisional_no FROM student WHERE id=?1", [&stu.id], |r| r.get(0)).unwrap();
    assert!(prov.contains("A9"), "provisional number from the device's own series: {prov}");
    let exp: ExpenseInput = serde_json::from_value(serde_json::json!({ "category_account_id": account, "amount_paise": 25_000, "paid_via": "cash" })).unwrap();
    let e = record_expense_logic(&mut pc2, &suresh, Some("dev-acc2"), DeviceMode::Client, "2026-10-15", &exp).unwrap();

    let server = Arc::new(Mutex::new(server));
    let t = LoopbackTransport::new(server.clone(), DeviceAuth { device_id: "dev-acc2".into(), staff_id: "stf-suresh".into() }, now());
    let out = sync_once(&mut pc2, &t).await.unwrap();
    assert_eq!((out.confirmed, out.rejected), (2, 0), "{out:?}");

    {
        let s = server.lock().unwrap();
        let (adm, p): (Option<String>, String) = s.query_row("SELECT admission_no, provisional_no FROM student WHERE id=?1", [&stu.id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert!(adm.is_some(), "the school PC assigned the official admission number");
        assert_eq!(p, prov, "same provisional number");
        let enr: i64 = s.query_row("SELECT COUNT(*) FROM enrollment WHERE student_id=?1", [&stu.id], |r| r.get(0)).unwrap();
        assert_eq!(enr, 1);
        let v: i64 = s.query_row("SELECT COUNT(*) FROM expense WHERE id=?1 AND voucher_id IS NOT NULL", [&e.id], |r| r.get(0)).unwrap();
        assert_eq!(v, 1, "expense recorded with its voucher on the school PC");
    }
    // After pulling, the PC2 has ONE student row (now with the official number) and
    // one enrolment — no duplicates from the round trip.
    sync_once(&mut pc2, &t).await.unwrap();
    let (n, adm): (i64, Option<String>) = pc2.query_row("SELECT COUNT(*), MAX(admission_no) FROM student WHERE name='Offline Admit'", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
    assert_eq!(n, 1);
    assert!(adm.is_some(), "official number reached the device");
    let enr: i64 = pc2.query_row("SELECT COUNT(*) FROM enrollment WHERE student_id=?1", [&stu.id], |r| r.get(0)).unwrap();
    assert_eq!(enr, 1, "no duplicate enrolment");
}
