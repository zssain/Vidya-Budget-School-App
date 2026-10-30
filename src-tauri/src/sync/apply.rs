//! Server-side op application (prompts/P04 Step 3). Applying an op = load the
//! op's staff member's CURRENT actor → vidya-core decision → `with_write`
//! (+ op_log + audit) + `applied_ops` (idempotency by op_id). Payments use
//! `apply_synced_payment` (never rejected for amount). A revoked/suspended author
//! raises `review_flag revoked_author` and the op is `flagged` (not applied).
//!
//! Rule logic is NEVER duplicated here — vidya-core decides (rule §6).

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use vidya_core::fees::{self, AllocKind, Due};
use vidya_core::money::Paise;
use vidya_core::permissions::{self, Action, Actor, Target, TargetKind};
use vidya_core::types::{Role, StaffState};

use crate::db::now_iso;
use crate::security::audit::{self, AuditEntry};
use crate::sync::protocol::{codes, Op, OpResult, OpStatus};

/// Outcome of applying one op (server-internal; maps to `OpResult`).
fn confirmed(op_id: &str, server_seq: i64, record: Option<serde_json::Value>) -> OpResult {
    OpResult { op_id: op_id.into(), status: OpStatus::Confirmed, server_seq: Some(server_seq), record, reason_code: None }
}
fn rejected(op_id: &str, code: &str) -> OpResult {
    OpResult { op_id: op_id.into(), status: OpStatus::Rejected, server_seq: None, record: None, reason_code: Some(code.into()) }
}
fn conflicted(op_id: &str) -> OpResult {
    OpResult { op_id: op_id.into(), status: OpStatus::Conflict, server_seq: None, record: None, reason_code: None }
}
fn flagged(op_id: &str, code: &str) -> OpResult {
    OpResult { op_id: op_id.into(), status: OpStatus::Flagged, server_seq: None, record: None, reason_code: Some(code.into()) }
}

/// Load a staff member's CURRENT actor from the DB (role + state + assignments).
pub fn load_actor(conn: &Connection, staff_id: &str) -> rusqlite::Result<Option<Actor>> {
    let row = conn
        .query_row(
            "SELECT role, state FROM staff WHERE id=?1",
            params![staff_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()?;
    let (role, state) = match row {
        Some(v) => v,
        None => return Ok(None),
    };
    let role = match role.as_str() {
        "principal" => Role::Principal,
        "accountant" => Role::Accountant,
        _ => Role::Teacher,
    };
    let state = match state.as_str() {
        "active" => StaffState::Active,
        "suspended" => StaffState::Suspended,
        "removed" => StaffState::Removed,
        _ => StaffState::Invited,
    };
    let mut class_teacher_of = Vec::new();
    let mut class_subjects = Vec::new();
    if role == Role::Teacher {
        let mut s = conn.prepare("SELECT id FROM class WHERE class_teacher_id=?1")?;
        class_teacher_of = s.query_map(params![staff_id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
        let mut s2 = conn.prepare("SELECT id FROM class_subject WHERE teacher_id=?1")?;
        class_subjects = s2.query_map(params![staff_id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
    }
    Ok(Some(Actor { staff_id: staff_id.into(), role, state, class_teacher_of, class_subjects }))
}

/// Map an op's table+kind to the vidya-core Action it needs (rule §5/§6).
fn action_for(table: &str, kind: &str) -> Option<Action> {
    match (table, kind) {
        ("student", "insert") => Some(Action::CreateStudent),
        ("student", "update") => Some(Action::EditStudentDetails),
        ("enrollment", _) => Some(Action::EnrollStudent),
        ("payment", _) => Some(Action::RecordPayment),
        ("attendance_sheet", _) | ("attendance_mark", _) => Some(Action::TakeAttendance),
        ("mark_entry", _) | ("marks_sheet", _) => Some(Action::EnterMarks),
        // School accounts (P15) — each maps to the SAME Action + target the command
        // layer uses, so the server re-validates permission for these ops too
        // (previously they were un-mapped and skipped the check — P18 security fix).
        ("expense", _) => Some(Action::RecordExpense),
        ("expense_reversal", _) => Some(Action::ReverseExpense),
        ("salary_structure", _) | ("salary_run", _) | ("salary_line", _) | ("staff_advance", _) => Some(Action::ManageSalary),
        // School store (P15).
        ("store_item", _) | ("stock_move", _) => Some(Action::ManageStore),
        ("store_sale", _) => Some(Action::RecordStoreSale),
        // Communication (P14): circular create/edit is Principal-managed (the teacher
        // class-notice flow goes through a request, not a direct circular op).
        ("circular", _) => Some(Action::ManageCirculars),
        // Classroom (P16): a teacher's homework/notes and report-card remarks…
        ("homework_note", _) => Some(Action::ManageNotes),
        ("report_remark", _) => Some(Action::EnterReportRemark),
        // …and the Principal-managed classroom config.
        ("period", _) | ("timetable_slot", _) => Some(Action::ManageTimetable),
        ("substitution", _) => Some(Action::ManageSubstitutes),
        ("exam_room", _) | ("exam_seat", _) | ("exam_schedule", _) => Some(Action::ManageExamSeating),
        // Staff HR (P17): leave types / leave records are Principal-managed.
        ("leave_type", _) | ("leave_record", _) => Some(Action::ManageStaffHr),
        // `staff_attendance` is decided per-row in `apply_op` (own = StaffCheckIn,
        // another staff's row = ManageStaffHr), so it is not mapped here.
        //
        // Left intentionally un-mapped (module-gated only, or Core): `circular_read`
        // (any staff marks a circular read), `report_template`/`report_lock`
        // (reference / server-authoritative, not device ops), `fee_due`, `guardian`,
        // `student_guardian`, `consent`, `message`/`message_template`. Adding a
        // precise Action for these is tracked in docs/phase-notes/phase-18.md.
        _ => None,
    }
}

fn target_for(table: &str, payload: &serde_json::Value) -> Target {
    let class_id = payload.get("class_id").and_then(|v| v.as_str()).map(str::to_string);
    let class_subject_id = payload.get("class_subject_id").and_then(|v| v.as_str()).map(str::to_string);
    let kind = match table {
        "student" | "enrollment" => TargetKind::Student,
        "payment" | "fee_due" => TargetKind::Fee,
        "attendance_sheet" | "attendance_mark" | "homework_note" => TargetKind::Attendance,
        "mark_entry" | "marks_sheet" | "report_remark" => TargetKind::Marks,
        // Match the command layer's target for these (accounts/store → Fee;
        // timetable/substitutes/exams/circular → School) so the server re-check
        // agrees with how the op was authorised on the device.
        "expense" | "expense_reversal" | "salary_structure" | "salary_run" | "salary_line"
        | "staff_advance" | "store_item" | "store_sale" | "stock_move" => TargetKind::Fee,
        "period" | "timetable_slot" | "substitution" | "exam_room" | "exam_seat"
        | "exam_schedule" | "circular" => TargetKind::School,
        _ => TargetKind::Own,
    };
    Target { kind, class_id, class_subject_id, ..Default::default() }
}

/// Resolve the class a target belongs to (attendance ops don't carry class_id).
fn resolve_class_id(conn: &Connection, table: &str, record_id: &str, payload: &serde_json::Value) -> rusqlite::Result<Option<String>> {
    // The server's stored row wins; the payload's class_id is used only when the row
    // is new (an attacker can't re-label someone else's row as their class).
    if let Some(c) = stored_class_id(conn, table, record_id, payload)? {
        return Ok(Some(c));
    }
    if row_exists(conn, table, record_id)? {
        return Ok(None);
    }
    Ok(payload.get("class_id").and_then(|v| v.as_str()).map(str::to_string))
}

/// The class of a row from the server's own data (None if unknown / new).
fn stored_class_id(conn: &Connection, table: &str, record_id: &str, payload: &serde_json::Value) -> rusqlite::Result<Option<String>> {
    match table {
        "homework_note" => conn
            .query_row("SELECT class_id FROM homework_note WHERE id=?1", params![record_id], |r| r.get::<_, Option<String>>(0))
            .optional()
            .map(Option::flatten),
        "attendance_sheet" => conn
            .query_row("SELECT class_id FROM attendance_sheet WHERE id=?1", params![record_id], |r| r.get::<_, String>(0))
            .optional(),
        "attendance_mark" => {
            // Class of the mark's existing sheet, or of the sheet named in the payload.
            if let Some(c) = conn
                .query_row(
                    "SELECT s.class_id FROM attendance_mark m JOIN attendance_sheet s ON s.id=m.sheet_id WHERE m.id=?1",
                    params![record_id], |r| r.get::<_, String>(0),
                )
                .optional()?
            {
                return Ok(Some(c));
            }
            if let Some(sheet_id) = payload.get("sheet_id").and_then(|v| v.as_str()) {
                return conn
                    .query_row("SELECT class_id FROM attendance_sheet WHERE id=?1", params![sheet_id], |r| r.get::<_, String>(0))
                    .optional();
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

/// True if `table` already holds a row with this id (identifier-checked by caller).
fn row_exists(conn: &Connection, table: &str, id: &str) -> rusqlite::Result<bool> {
    let sql = format!("SELECT 1 FROM {table} WHERE id=?1");
    Ok(conn.query_row(&sql, params![id], |_| Ok(())).optional()?.is_some())
}

/// The class_subject a marks op belongs to, from the server's rows (the sheet's
/// exam_subject). For a brand-new sheet/entry the payload names the exam_subject /
/// sheet, which must itself exist server-side. Other tables: the stored row, or the
/// payload only for a new row.
fn resolve_class_subject_id(conn: &Connection, table: &str, record_id: &str, payload: &serde_json::Value) -> rusqlite::Result<Option<String>> {
    let via_exam_subject = |es: &str| -> rusqlite::Result<Option<String>> {
        conn.query_row("SELECT class_subject_id FROM exam_subject WHERE id=?1", params![es], |r| r.get(0)).optional()
    };
    let via_sheet = |sheet: &str| -> rusqlite::Result<Option<String>> {
        let es: Option<String> =
            conn.query_row("SELECT exam_subject_id FROM marks_sheet WHERE id=?1", params![sheet], |r| r.get(0)).optional()?;
        match es {
            Some(es) => via_exam_subject(&es),
            None => Ok(None),
        }
    };
    match table {
        "marks_sheet" => {
            if let Some(cs) = via_sheet(record_id)? {
                return Ok(Some(cs));
            }
            match payload.get("exam_subject_id").and_then(|v| v.as_str()) {
                Some(es) => via_exam_subject(es),
                None => Ok(None),
            }
        }
        "mark_entry" => {
            let sheet: Option<String> =
                conn.query_row("SELECT sheet_id FROM mark_entry WHERE id=?1", params![record_id], |r| r.get(0)).optional()?;
            match sheet.or_else(|| payload.get("sheet_id").and_then(|v| v.as_str()).map(str::to_string)) {
                Some(sh) => via_sheet(&sh),
                None => Ok(None),
            }
        }
        "homework_note" => {
            let stored: Option<Option<String>> = conn
                .query_row("SELECT class_subject_id FROM homework_note WHERE id=?1", params![record_id], |r| r.get(0))
                .optional()?;
            Ok(match stored {
                Some(cs) => cs,
                None => payload.get("class_subject_id").and_then(|v| v.as_str()).map(str::to_string),
            })
        }
        _ => Ok(None),
    }
}

/// True when the attendance/marks sheet an op touches is already submitted (locked).
fn sheet_is_submitted(conn: &Connection, table: &str, record_id: &str, payload: &serde_json::Value) -> rusqlite::Result<bool> {
    let status: Option<String> = match table {
        "attendance_sheet" => conn
            .query_row("SELECT status FROM attendance_sheet WHERE id=?1", params![record_id], |r| r.get(0))
            .optional()?,
        "attendance_mark" => {
            let sheet: Option<String> = conn
                .query_row("SELECT sheet_id FROM attendance_mark WHERE id=?1", params![record_id], |r| r.get(0))
                .optional()?
                .or_else(|| payload.get("sheet_id").and_then(|v| v.as_str()).map(str::to_string));
            match sheet {
                Some(sh) => conn
                    .query_row("SELECT status FROM attendance_sheet WHERE id=?1", params![sh], |r| r.get(0))
                    .optional()?,
                None => None,
            }
        }
        "marks_sheet" => conn
            .query_row("SELECT status FROM marks_sheet WHERE id=?1", params![record_id], |r| r.get(0))
            .optional()?,
        "mark_entry" => {
            let sheet: Option<String> = conn
                .query_row("SELECT sheet_id FROM mark_entry WHERE id=?1", params![record_id], |r| r.get(0))
                .optional()?
                .or_else(|| payload.get("sheet_id").and_then(|v| v.as_str()).map(str::to_string));
            match sheet {
                Some(sh) => conn.query_row("SELECT status FROM marks_sheet WHERE id=?1", params![sh], |r| r.get(0)).optional()?,
                None => None,
            }
        }
        _ => None,
    };
    Ok(status.as_deref() == Some("submitted"))
}

/// Columns that tie a row to its class / student / sheet / owner. An update op may
/// not change them on an existing row.
const OWNERSHIP_COLS: &[&str] = &[
    "class_id", "class_subject_id", "sheet_id", "student_id", "exam_subject_id", "exam_id", "staff_id", "date",
    "teacher_id", "created_by", "requested_by", "collected_by",
];
/// Author columns: when an op carries one, it must name the op's own author.
const AUTHOR_COLS: &[&str] = &["created_by", "requested_by", "collected_by", "submitted_by", "updated_by_staff"];

/// False if the op re-points an existing row (an ownership column changes) or names
/// someone else in an author column / another device in `updated_by_device`.
fn ownership_ok(conn: &Connection, op: &Op) -> rusqlite::Result<bool> {
    let Some(obj) = op.payload.as_object() else { return Ok(true) };
    for col in AUTHOR_COLS {
        if let Some(v) = obj.get(*col) {
            if !v.is_null() && v.as_str() != Some(op.staff_id.as_str()) {
                return Ok(false);
            }
        }
    }
    if let Some(v) = obj.get("updated_by_device") {
        if !v.is_null() && v.as_str() != Some(op.device_id.as_str()) {
            return Ok(false);
        }
    }
    if !row_exists(conn, &op.table, &op.record_id)? {
        return Ok(true);
    }
    let cols = table_columns(conn, &op.table)?;
    for col in OWNERSHIP_COLS {
        let Some(incoming) = obj.get(*col) else { continue };
        if !cols.iter().any(|c| c == col) {
            continue;
        }
        let sql = format!("SELECT {col} FROM {} WHERE id=?1", op.table);
        let cur: Option<String> = conn.query_row(&sql, params![op.record_id], |r| r.get::<_, Option<String>>(0))?;
        if cur.as_deref() != incoming.as_str() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn table_columns(conn: &Connection, table: &str) -> rusqlite::Result<Vec<String>> {
    let mut st = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let cols = st.query_map([], |r| r.get::<_, String>(1))?.collect();
    cols
}

/// The v2 attendance Present/Absent cutover, in Unix ms (00-SYSTEM-CONTEXT §7a),
/// written into `schema_meta` by migration `0005_v2_attendance_pa.sql`. `None`
/// if the row is absent (older DBs) — then no L op is rejected on cutover.
fn attendance_pa_cutover_ms(conn: &Connection) -> rusqlite::Result<Option<u64>> {
    let v: Option<String> = conn
        .query_row("SELECT value FROM schema_meta WHERE key='attendance_pa_cutover_ms'", [], |r| r.get(0))
        .optional()?;
    Ok(v.and_then(|s| s.trim().parse::<u64>().ok()))
}

/// True iff `op` writes a Leave (`L`) `attendance_mark` and its HLC is at/after
/// the v2 cutover — i.e. a new L from an out-of-date device (§7a). Pre-cutover L
/// ops (older `wall_ms`) and non-L ops return `false` and apply normally. The
/// HLC's leading 13 digits are the `wall_ms` (see `vidya_core::hlc`).
fn leave_after_cutover(conn: &Connection, op: &Op) -> rusqlite::Result<bool> {
    if op.table != "attendance_mark" {
        return Ok(false);
    }
    if op.payload.get("mark").and_then(|v| v.as_str()) != Some("L") {
        return Ok(false);
    }
    let cutover = match attendance_pa_cutover_ms(conn)? {
        Some(c) => c,
        None => return Ok(false),
    };
    match vidya_core::hlc::Hlc::parse(&op.hlc) {
        Ok(hlc) => Ok(hlc.wall_ms >= cutover),
        // A non-packed HLC (e.g. a server-local ISO op) is never a device L write.
        Err(_) => Ok(false),
    }
}

/// The current `version` of a synced row, if it exists.
fn current_version(conn: &Connection, table: &str, id: &str) -> rusqlite::Result<Option<i64>> {
    // Only synced tables carry `version`; the caller only asks for those.
    let sql = format!("SELECT version FROM {table} WHERE id=?1");
    conn.query_row(&sql, params![id], |r| r.get::<_, i64>(0)).optional()
}

/// The time-limited attendance grants a teacher holds on `date` (§10.4):
/// substitutions that convey attendance + approved attendance-duty requests. Used
/// by both the attendance command path and this server re-validation, so the rule
/// is applied identically on the device and the server.
pub fn attendance_grants_for(conn: &Connection, staff_id: &str, date: &str) -> rusqlite::Result<Vec<permissions::AttendanceGrant>> {
    let mut grants = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT class_id FROM substitution WHERE substitute_teacher_id=?1 AND date=?2 AND includes_attendance=1",
    )?;
    let rows = stmt.query_map(params![staff_id, date], |r| r.get::<_, String>(0))?;
    for cid in rows {
        let class_id = cid?;
        grants.push(permissions::AttendanceGrant { class_id, from_date: date.to_string(), to_date: date.to_string() });
    }
    let mut stmt2 = conn.prepare(
        "SELECT after_json FROM request WHERE type='attendance_duty' AND requested_by=?1 AND status='approved'",
    )?;
    let rows2 = stmt2.query_map(params![staff_id], |r| r.get::<_, String>(0))?;
    for aj in rows2 {
        let aj = aj?;
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&aj) {
            let cid = v.get("class_id").and_then(|x| x.as_str());
            let from = v.get("from_date").and_then(|x| x.as_str());
            let to = v.get("to_date").and_then(|x| x.as_str());
            if let (Some(c), Some(f), Some(t)) = (cid, from, to) {
                grants.push(permissions::AttendanceGrant { class_id: c.into(), from_date: f.into(), to_date: t.into() });
            }
        }
    }
    Ok(grants)
}

/// The date (`YYYY-MM-DD`) an attendance op is for, from its sheet.
fn attendance_op_date(conn: &Connection, op: &Op) -> rusqlite::Result<Option<String>> {
    match op.table.as_str() {
        "attendance_sheet" => conn
            .query_row("SELECT date FROM attendance_sheet WHERE id=?1", params![op.record_id], |r| r.get(0))
            .optional(),
        "attendance_mark" => conn
            .query_row(
                "SELECT s.date FROM attendance_mark m JOIN attendance_sheet s ON s.id=m.sheet_id WHERE m.id=?1",
                params![op.record_id],
                |r| r.get(0),
            )
            .optional(),
        _ => Ok(None),
    }
}

/// True if `actor` may take attendance for the op's class on the op's date via a
/// substitution / attendance-duty grant (not as the class teacher).
fn attendance_op_grant_allows(conn: &Connection, actor: &Actor, target: &Target, op: &Op) -> rusqlite::Result<bool> {
    let class_id = match &target.class_id {
        Some(c) => c.clone(),
        None => return Ok(false),
    };
    let date = match attendance_op_date(conn, op)? {
        Some(d) => d,
        None => return Ok(false),
    };
    let grants = attendance_grants_for(conn, &actor.staff_id, &date)?;
    Ok(permissions::may_take_attendance(actor, &class_id, &date, &grants))
}

/// Who an inbound op provably came from: the device row behind the bearer token
/// (LAN), the session key that opened the sealed envelope (relay), or the device
/// whose Drive folder held the bundle.
pub struct OpAuth<'a> {
    pub device_id: &'a str,
    pub staff_id: &'a str,
    /// How the op actually reached the school PC: `"lan"` (pinned TLS on the school
    /// network / relay) or `"drive"`. Server-side truth — a device's own claim of its
    /// route is never trusted (staff check-in "at school" needs the LAN, §9 decision).
    pub route: &'a str,
}

/// SHA-256 over everything an op asserts, so a replayed op_id with different
/// content is detected.
fn op_content_hash(op: &Op) -> String {
    use sha2::{Digest, Sha256};
    let canon = serde_json::json!({
        "device_id": op.device_id, "staff_id": op.staff_id, "table": op.table, "record_id": op.record_id,
        "kind": op.kind, "payload": op.payload, "base_version": op.base_version, "hlc": op.hlc,
    });
    let digest = Sha256::digest(canon.to_string().as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Apply an op that arrived from ANOTHER device (LAN push, sealed relay, Drive
/// import). On top of `apply_op`'s rules this:
/// * binds the op to the authenticated device — its `device_id` / `staff_id` must
///   be that device and its staff (an op can't claim the Principal);
/// * refuses a revoked/unknown device and an op stamped with an older epoch;
/// * serves a cached result only for an identical retry from the same device —
///   a reused op_id with other content or from another device is refused and NOT
///   remembered (so it can't poison the real op);
/// * never fails the batch: an op that errors (its own write transaction rolls back)
///   is remembered as MALFORMED and the next op proceeds.
pub fn apply_client_op(conn: &mut Connection, auth: &OpAuth, op: &Op) -> rusqlite::Result<OpResult> {
    let hash = op_content_hash(op);

    // Identity binding first: nothing an impostor sends is cached under its op_id.
    if op.device_id != auth.device_id || op.staff_id != auth.staff_id {
        return Ok(rejected(&op.op_id, codes::FORBIDDEN));
    }

    // Owned, content-checked idempotency.
    let cached: Option<(Option<String>, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT result, device_id, payload_hash FROM applied_ops WHERE op_id=?1",
            params![op.op_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    if let Some((result, dev, ph)) = cached {
        let same = dev.as_deref() == Some(auth.device_id) && ph.as_deref() == Some(hash.as_str());
        if !same {
            return Ok(rejected(&op.op_id, codes::DUPLICATE_OP));
        }
        if let Some(v) = result.and_then(|r| serde_json::from_str::<OpResult>(&r).ok()) {
            return Ok(v);
        }
    }

    // The device must still be live and belong to the staff it claims.
    let dev: Option<(String, Option<String>)> = conn
        .query_row("SELECT staff_id, revoked_at FROM device WHERE id=?1", params![auth.device_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?;
    match dev {
        Some((staff, None)) if staff == auth.staff_id => {}
        _ => return Ok(rejected(&op.op_id, codes::DEVICE_REVOKED_OR_UNKNOWN)),
    }

    let remember = |conn: &mut Connection, res: OpResult| -> rusqlite::Result<OpResult> {
        conn.execute(
            "INSERT OR REPLACE INTO applied_ops(op_id, applied_at, result, device_id, payload_hash) VALUES (?1,?2,?3,?4,?5)",
            params![op.op_id, now_iso(), serde_json::to_string(&res).unwrap_or_default(), auth.device_id, hash],
        )?;
        Ok(res)
    };

    // Fencing (§8.9): an op written against an older server epoch is refused.
    let epoch: i64 = conn.query_row("SELECT server_epoch FROM school LIMIT 1", [], |r| r.get(0)).optional()?.unwrap_or(1);
    if op.server_epoch < epoch {
        return remember(conn, rejected(&op.op_id, codes::EPOCH_OLD));
    }

    // apply_op's domain writes run in their own transaction, so an error leaves no
    // partial row behind; it is turned into a remembered rejection instead of
    // aborting the whole push / Drive import.
    match apply_op_routed(conn, op, auth.route) {
        Ok(res) => remember(conn, res),
        Err(e) => {
            tracing::warn!(table = %op.table, kind = %op.kind, "client op failed to apply: {e}");
            remember(conn, rejected(&op.op_id, codes::MALFORMED))
        }
    }
}

/// Apply one op. Idempotent by `op_id`. Returns the per-op result. (Server-local
/// callers; inbound device ops go through `apply_client_op`, which knows the route.)
pub fn apply_op(conn: &mut Connection, op: &Op) -> rusqlite::Result<OpResult> {
    apply_op_routed(conn, op, "lan")
}

fn apply_op_routed(conn: &mut Connection, op: &Op, route: &str) -> rusqlite::Result<OpResult> {
    // 1) Idempotency — same op via LAN and Drive applies once (§8.4).
    if let Some(result) = conn
        .query_row("SELECT result FROM applied_ops WHERE op_id=?1", params![op.op_id], |r| r.get::<_, Option<String>>(0))
        .optional()?
        .flatten()
    {
        if let Ok(v) = serde_json::from_str::<OpResult>(&result) {
            return Ok(v);
        }
    }

    // 1b) Reject anything whose table or a payload column is not a plain SQL
    //     identifier BEFORE any dynamic-identifier SQL is built for it (§2).
    //     Identifiers can't be bound as parameters, so this is the injection gate.
    if !crate::sync::protocol::op_identifiers_safe(&op.table, &op.payload) {
        return finalize(conn, op, rejected(&op.op_id, codes::MALFORMED));
    }

    // 1c) v2 attendance cutover (§7a): a new `L` (Leave) mark carried by an op
    //     whose HLC is at/after the cutover is rejected (the phone runs old
    //     Vidya). Ops from before the cutover apply as legacy.
    if leave_after_cutover(conn, op)? {
        return finalize(conn, op, rejected(&op.op_id, codes::LEAVE_MARK_REMOVED));
    }

    // 1d) Deny by default: an op is applied only if the server has a rule for its
    //     (table, kind). Unknown tables (staff, device, invite, app_kv, request,
    //     class, …) and `delete` (no tombstone path yet) are refused, never upserted.
    // `request`/`action` is a replayed request command (raise / leave / cancel) whose
    // permission rules live in commands::logic::request_command (run as the author).
    let request_cmd = op.table == "request" && op.kind == "action";
    if !matches!(op.kind.as_str(), "insert" | "update" | "action")
        || (!request_cmd && op.table != "staff_attendance" && action_for(&op.table, &op.kind).is_none())
    {
        return finalize(conn, op, rejected(&op.op_id, codes::UNSUPPORTED_OP));
    }

    // 1e) An op may not re-point an existing row at another class/student/sheet/…,
    //     and may not claim another author in a `*_by` column.
    if !ownership_ok(conn, op)? {
        return finalize(conn, op, rejected(&op.op_id, codes::FORBIDDEN));
    }

    // 2) Author's CURRENT actor.
    let actor = match load_actor(conn, &op.staff_id)? {
        Some(a) => a,
        None => return finalize(conn, op, rejected(&op.op_id, codes::DEVICE_REVOKED_OR_UNKNOWN)),
    };

    // 3) Revoked/suspended author → review_flag + flagged (not applied). §3.
    if actor.state != StaffState::Active {
        let res = flagged(&op.op_id, "revoked_author");
        raise_flag(conn, "revoked_author", &op.table, &op.record_id, &op.op_id)?;
        return finalize(conn, op, res);
    }

    // 4) Module switch (§14) then permission (vidya-core decides). A disabled
    //    module's op is rejected server-side before the permission check.
    // Staff HR (P17): a `staff_attendance` op is the author's own check-in
    // (StaffCheckIn, any active staff) when the row's `staff_id` is the author,
    // otherwise a Principal manual entry (ManageStaffHr). This stops a staff
    // member forging another staff member's attendance.
    let hr_action = if op.table == "staff_attendance" {
        let own = op.payload.get("staff_id").and_then(|v| v.as_str()) == Some(op.staff_id.as_str());
        Some(if own { Action::StaffCheckIn } else { Action::ManageStaffHr })
    } else {
        None
    };
    // 4a) Module switch (§8.4/§14): reject a disabled module's op server-side,
    //     before ANY write, for EVERY one of that module's tables — not only the
    //     permission-mapped ones. `scope::module_of_table` is the authoritative
    //     table→module map (the same one that excludes these tables from pull), so
    //     an `expense`, `store_item`, `circular`, `timetable_slot` … op for a
    //     disabled module is rejected here even though `action_for` doesn't map it.
    //     (Previously the gate lived only inside the `action_for` block below, so
    //     un-mapped module tables bypassed it — P18 security fix.)
    if let Some(module) = crate::sync::scope::module_of_table(&op.table) {
        if !crate::modules::is_enabled(conn, module)? {
            return finalize(conn, op, rejected(&op.op_id, codes::MODULE_OFF));
        }
    }

    // 4b) Permission (vidya-core decides) for tables mapped to an Action. NOTE:
    //     tables `action_for` does not map still miss the server permission
    //     re-check — a known gap tracked in docs/phase-notes/phase-18.md (security
    //     review §7). 4a above now enforces the module half for all module tables.
    if let Some(mut action) = hr_action.or_else(|| action_for(&op.table, &op.kind)) {
        // The target is resolved from the SERVER's rows wherever they exist; the
        // payload only describes a brand-new row (and ownership_ok above stops an
        // update from moving one). A payload can't claim a class it doesn't own.
        let mut target = target_for(&op.table, &op.payload);
        target.class_id = resolve_class_id(conn, &op.table, &op.record_id, &op.payload)?;
        if let Some(cs) = resolve_class_subject_id(conn, &op.table, &op.record_id, &op.payload)? {
            target.class_subject_id = Some(cs);
            if target.class_id.is_none() {
                target.class_id = conn
                    .query_row("SELECT class_id FROM class_subject WHERE id=?1", params![target.class_subject_id], |r| r.get(0))
                    .optional()?;
            }
        } else if matches!(op.table.as_str(), "marks_sheet" | "mark_entry") {
            target.class_subject_id = None;
        }
        // Submitted sheets are locked (§3 rule 6): editing one is a different action.
        target.is_locked = sheet_is_submitted(conn, &op.table, &op.record_id, &op.payload)?;
        if target.is_locked {
            action = match action {
                Action::TakeAttendance => Action::EditSubmittedAttendance,
                Action::EnterMarks => Action::EditSubmittedMarks,
                other => other,
            };
        }
        match permissions::can(&actor, action, &target) {
            permissions::Decision::Allow { audit_reason_required: false } => {}
            // Locked-data edits by the Principal need an audited reason → school PC.
            permissions::Decision::Allow { audit_reason_required: true } => {
                return finalize(conn, op, rejected(&op.op_id, codes::REASON_REQUIRED));
            }
            permissions::Decision::NeedsRequest(_) => {
                return finalize(conn, op, rejected(&op.op_id, codes::NEEDS_REQUEST));
            }
            permissions::Decision::Deny { .. } => {
                // Substitutes / attendance duty (§10.4): a teacher who is not the
                // class teacher may still take attendance for a class on a date via an
                // active substitution or an approved attendance-duty request.
                let by_grant = matches!(action, Action::TakeAttendance)
                    && attendance_op_grant_allows(conn, &actor, &target, op)?;
                if !by_grant {
                    return finalize(conn, op, rejected(&op.op_id, codes::FORBIDDEN));
                }
            }
        }
    }

    // 4b'') Staff check-in / check-out commands, replayed as the author with the route
    //       the op really used.
    if op.table == "staff_attendance" && op.kind == "action" {
        return apply_staff_attendance_action(conn, op, route);
    }

    // 4b') A request command (raise / apply for leave / cancel own) is replayed as the
    //      author through the same transactional logic the device ran.
    if request_cmd {
        return apply_request_action(conn, op);
    }

    // 4c') Marks arrive the same way: one action op with the whole subject sheet.
    if op.table == "marks_sheet" && op.kind == "action" {
        return apply_marks_action(conn, op);
    }

    // 4c) Attendance arrives as ONE action op carrying the complete sheet; it is
    //     applied atomically through the same domain write the device used.
    if op.table == "attendance_sheet" && op.kind == "action" {
        return apply_attendance_action(conn, op);
    }

    // 5) Payments never rejected for amount (§8.6).
    if op.table == "payment" && op.kind == "insert" {
        return apply_payment(conn, op);
    }

    // 6) Conflict detection for updates (§8.5): base_version older than current
    //    AND a carried field differs from the current value → conflict.
    if op.kind == "update" {
        if let Some(base) = op.base_version {
            if let Some(cur) = current_version(conn, &op.table, &op.record_id)? {
                if base < cur && row_field_differs(conn, &op.table, &op.record_id, &op.payload)? {
                    record_conflict(conn, op)?;
                    return finalize(conn, op, conflicted(&op.op_id));
                }
            }
        }
    }

    apply_upsert(conn, op)
}

/// Replay a staff check-in / check-out as its author. The check-in route is the
/// transport the op arrived by (`route`), not what the device claimed: a check-in that
/// came through Drive is recorded as "away" for the Principal to accept (§9 default).
fn apply_staff_attendance_action(conn: &mut Connection, op: &Op, route: &str) -> rusqlite::Result<OpResult> {
    let staff: Option<(String, String)> = conn
        .query_row("SELECT name, role FROM staff WHERE id=?1", params![op.staff_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?;
    let Some((name, role)) = staff else {
        return finalize(conn, op, rejected(&op.op_id, codes::DEVICE_REVOKED_OR_UNKNOWN));
    };
    let author = crate::state::SessionStaff { id: op.staff_id.clone(), name, role };
    let mode = crate::write::DeviceMode::Server;
    let res = match op.payload.get("cmd").and_then(|v| v.as_str()) {
        Some("check_in") => {
            let Some(mut input) = op.payload.get("input").and_then(|v| serde_json::from_value::<crate::commands::logic::CheckInInput>(v.clone()).ok()) else {
                return finalize(conn, op, rejected(&op.op_id, codes::MALFORMED));
            };
            if route == "drive" {
                input.route = "drive".into();
            }
            crate::commands::logic::staff_check_in_logic(conn, &author, Some(&op.device_id), mode, &input).map(|_| ())
        }
        Some("check_out") => {
            let date = op.payload.get("date").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let minute = op.payload.get("minute").and_then(|v| v.as_i64()).unwrap_or(-1);
            crate::commands::logic::staff_check_out_logic(conn, &author, Some(&op.device_id), mode, &date, minute)
        }
        _ => return finalize(conn, op, rejected(&op.op_id, codes::MALFORMED)),
    };
    match res {
        Ok(()) => {
            let seq: i64 = conn.query_row("SELECT COALESCE(MAX(server_seq),0) FROM op_log", [], |r| r.get(0))?;
            finalize(conn, op, confirmed(&op.op_id, seq, None))
        }
        Err(e) => finalize(conn, op, rejected(&op.op_id, &e.code)),
    }
}

/// Replay a client's request command as its author (see commands::logic::RequestCmd).
/// The request id travels in the command, so the canonical row is the device's row.
fn apply_request_action(conn: &mut Connection, op: &Op) -> rusqlite::Result<OpResult> {
    let Ok(cmd) = serde_json::from_value::<crate::commands::logic::RequestCmd>(op.payload.clone()) else {
        return finalize(conn, op, rejected(&op.op_id, codes::MALFORMED));
    };
    let id_matches = match &cmd {
        crate::commands::logic::RequestCmd::Create { id, .. }
        | crate::commands::logic::RequestCmd::Leave { id, .. }
        | crate::commands::logic::RequestCmd::Cancel { id } => *id == op.record_id,
    };
    if !id_matches {
        return finalize(conn, op, rejected(&op.op_id, codes::MALFORMED));
    }
    let staff: Option<(String, String)> = conn
        .query_row("SELECT name, role FROM staff WHERE id=?1", params![op.staff_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?;
    let Some((name, role)) = staff else {
        return finalize(conn, op, rejected(&op.op_id, codes::DEVICE_REVOKED_OR_UNKNOWN));
    };
    let author = crate::state::SessionStaff { id: op.staff_id.clone(), name, role };
    match crate::commands::logic::request_command(conn, &author, Some(&op.device_id), crate::write::DeviceMode::Server, &cmd) {
        Ok(_) => {
            let seq: i64 = conn.query_row("SELECT COALESCE(MAX(server_seq),0) FROM op_log", [], |r| r.get(0))?;
            finalize(conn, op, confirmed(&op.op_id, seq, None))
        }
        Err(e) => finalize(conn, op, rejected(&op.op_id, &e.code)),
    }
}

/// Apply a client's marks sheet as its author through commands::logic::upsert_marks
/// (enrolment, max-marks validation and the per-subject submitted lock).
fn apply_marks_action(conn: &mut Connection, op: &Op) -> rusqlite::Result<OpResult> {
    #[derive(serde::Deserialize)]
    struct Sheet {
        exam_subject_id: String,
        submit: bool,
        entries: Vec<crate::commands::logic::MarkEntryInput>,
    }
    let Ok(sheet) = serde_json::from_value::<Sheet>(op.payload.clone()) else {
        return finalize(conn, op, rejected(&op.op_id, codes::MALFORMED));
    };
    let staff: Option<(String, String)> = conn
        .query_row("SELECT name, role FROM staff WHERE id=?1", params![op.staff_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?;
    let Some((name, role)) = staff else {
        return finalize(conn, op, rejected(&op.op_id, codes::DEVICE_REVOKED_OR_UNKNOWN));
    };
    let author = crate::state::SessionStaff { id: op.staff_id.clone(), name, role };
    match crate::commands::logic::upsert_marks(
        conn,
        &author,
        Some(&op.device_id),
        crate::write::DeviceMode::Server,
        &sheet.exam_subject_id,
        &sheet.entries,
        sheet.submit,
    ) {
        Ok(()) => {
            let seq: i64 = conn.query_row("SELECT COALESCE(MAX(server_seq),0) FROM op_log", [], |r| r.get(0))?;
            finalize(conn, op, confirmed(&op.op_id, seq, None))
        }
        Err(e) => {
            let code = if e.code == "LOCKED" { codes::NEEDS_REQUEST.to_string() } else { e.code.clone() };
            finalize(conn, op, rejected(&op.op_id, &code))
        }
    }
}

/// Apply a client's attendance sheet (draft or submit) as its author, through
/// `commands::logic::attendance_write` — the same validation (enrolment set, valid
/// marks, submitted-sheet lock) and the same single transaction as on the device.
fn apply_attendance_action(conn: &mut Connection, op: &Op) -> rusqlite::Result<OpResult> {
    #[derive(serde::Deserialize)]
    struct Sheet {
        class_id: String,
        date: String,
        submit: bool,
        marks: Vec<crate::commands::logic::MarkInput>,
    }
    let Ok(sheet) = serde_json::from_value::<Sheet>(op.payload.clone()) else {
        return finalize(conn, op, rejected(&op.op_id, codes::MALFORMED));
    };
    let staff: Option<(String, String)> = conn
        .query_row("SELECT name, role FROM staff WHERE id=?1", params![op.staff_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?;
    let Some((name, role)) = staff else {
        return finalize(conn, op, rejected(&op.op_id, codes::DEVICE_REVOKED_OR_UNKNOWN));
    };
    let author = crate::state::SessionStaff { id: op.staff_id.clone(), name, role };
    match crate::commands::logic::attendance_write(
        conn,
        &author,
        Some(&op.device_id),
        crate::write::DeviceMode::Server,
        &sheet.class_id,
        &sheet.date,
        &sheet.marks,
        sheet.submit,
    ) {
        Ok(()) => {
            let seq: i64 = conn.query_row("SELECT COALESCE(MAX(server_seq),0) FROM op_log", [], |r| r.get(0))?;
            finalize(conn, op, confirmed(&op.op_id, seq, None))
        }
        Err(e) => {
            // A submitted sheet → the teacher must raise a correction request.
            let code = if e.code == "SHEET_LOCKED" { codes::NEEDS_REQUEST.to_string() } else { e.code.clone() };
            finalize(conn, op, rejected(&op.op_id, &code))
        }
    }
}

/// True if any field carried in `payload` differs from the current DB value.
fn row_field_differs(conn: &Connection, table: &str, id: &str, payload: &serde_json::Value) -> rusqlite::Result<bool> {
    let obj = match payload.as_object() {
        Some(o) => o,
        None => return Ok(false),
    };
    for (col, val) in obj {
        if col == "id" || col == "version" || col == "hlc" {
            continue;
        }
        let sql = format!("SELECT {col} FROM {table} WHERE id=?1");
        let cur: Option<String> = conn.query_row(&sql, params![id], |r| r.get::<_, Option<String>>(0)).optional()?.flatten();
        let incoming = val.as_str().map(str::to_string);
        if cur != incoming {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Record a conflict row per §8.5 (kept current value, incoming as value_b).
fn record_conflict(conn: &mut Connection, op: &Op) -> rusqlite::Result<()> {
    let obj = op.payload.as_object().cloned().unwrap_or_default();
    let now = now_iso();
    for (field, incoming) in obj {
        if field == "id" || field == "version" || field == "hlc" {
            continue;
        }
        let sql = format!("SELECT {field} FROM {} WHERE id=?1", op.table);
        let cur: Option<String> = conn.query_row(&sql, params![op.record_id], |r| r.get::<_, Option<String>>(0)).optional()?.flatten();
        let incoming_s = incoming.as_str().map(str::to_string);
        if cur == incoming_s {
            continue;
        }
        conn.execute(
            "INSERT INTO conflict(id,\"table\",record_id,field,value_a,hlc_a,value_b,hlc_b,device_b,staff_b,status) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'open')",
            params![
                format!("cf-{}-{}", op.op_id, field), op.table, op.record_id, field,
                cur, now, incoming_s, op.hlc, op.device_id, op.staff_id
            ],
        )?;
    }
    Ok(())
}

fn raise_flag(conn: &mut Connection, kind: &str, table: &str, id: &str, op_id: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO review_flag(id,kind,ref_table,ref_id,details_json,status) VALUES (?1,?2,?3,?4,?5,'open')",
        params![format!("rf-{op_id}"), kind, table, id, serde_json::json!({ "op_id": op_id }).to_string()],
    )?;
    Ok(())
}

/// Tables that carry the §7 sync columns (version, hlc, created/updated_*, sync_state).
fn is_synced(table: &str) -> bool {
    matches!(
        table,
        "school" | "staff" | "student" | "enrollment" | "attendance_sheet" | "marks_sheet" | "fee_due" | "payment" | "request"
    )
}

/// Upsert the row carried in the op, bumping version/hlc for synced tables, in one
/// audited write. Child tables (attendance_mark, payment_allocation, …) carry no
/// sync columns, so only their payload columns are written.
fn apply_upsert(conn: &mut Connection, op: &Op) -> rusqlite::Result<OpResult> {
    let obj = op.payload.as_object().cloned().unwrap_or_default();
    let now = now_iso();
    let synced = is_synced(&op.table);
    let tx = conn.transaction()?;

    let exists: bool = tx
        .query_row(&format!("SELECT 1 FROM {} WHERE id=?1", op.table), params![op.record_id], |_| Ok(()))
        .optional()?
        .is_some();

    if exists {
        // UPDATE the carried columns (+ sync bookkeeping for synced tables).
        let mut sets: Vec<String> = Vec::new();
        let mut vals: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        for (col, val) in &obj {
            if col == "id" {
                continue;
            }
            sets.push(format!("{col}=?"));
            vals.push(json_to_sql(val));
        }
        if synced {
            sets.push("version=version+1".into());
            sets.push("hlc=?".into());
            vals.push(Box::new(op.hlc.clone()));
            sets.push("updated_at=?".into());
            vals.push(Box::new(now.clone()));
            sets.push("updated_by_staff=?".into());
            vals.push(Box::new(op.staff_id.clone()));
            sets.push("updated_by_device=?".into());
            vals.push(Box::new(op.device_id.clone()));
            sets.push("sync_state='confirmed'".into());
        }
        if sets.is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        vals.push(Box::new(op.record_id.clone()));
        let sql = format!("UPDATE {} SET {} WHERE id=?", op.table, sets.join(", "));
        let refs: Vec<&dyn rusqlite::ToSql> = vals.iter().map(|b| b.as_ref()).collect();
        tx.execute(&sql, refs.as_slice())?;
    } else {
        // INSERT the carried columns (+ required bookkeeping for synced tables).
        let mut cols: Vec<String> = vec!["id".into()];
        let mut place: Vec<String> = vec!["?".into()];
        let mut vals: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(op.record_id.clone())];
        for (col, val) in &obj {
            if col == "id" {
                continue;
            }
            cols.push(col.clone());
            place.push("?".into());
            vals.push(json_to_sql(val));
        }
        if synced {
            for (c, v) in [("hlc", op.hlc.clone()), ("created_at", now.clone()), ("updated_at", now.clone()), ("updated_by_staff", op.staff_id.clone()), ("updated_by_device", op.device_id.clone()), ("sync_state", "confirmed".to_string())] {
                cols.push(c.into());
                place.push("?".into());
                vals.push(Box::new(v));
            }
        }
        let sql = format!("INSERT INTO {} ({}) VALUES ({})", op.table, cols.join(","), place.join(","));
        let refs: Vec<&dyn rusqlite::ToSql> = vals.iter().map(|b| b.as_ref()).collect();
        tx.execute(&sql, refs.as_slice())?;
    }

    let server_seq = write_oplog_audit(&tx, op, &now)?;
    let record = read_row_json(&tx, &op.table, &op.record_id)?;
    let result = confirmed(&op.op_id, server_seq, record);
    remember(&tx, op, &result)?;
    tx.commit()?;
    Ok(result)
}

/// Payment insert (§8.6): always accepted; excess → advance_credit + review_flag.
/// The only columns a synced payment may set; everything else is server bookkeeping.
const PAYMENT_COLS: &[&str] =
    &["receipt_no", "student_id", "amount_paise", "mode", "reference", "collected_by", "collected_at", "device_id"];

fn apply_payment(conn: &mut Connection, op: &Op) -> rusqlite::Result<OpResult> {
    let now = now_iso();
    let student_id = op.payload.get("student_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let amount = op.payload.get("amount_paise").and_then(|v| v.as_i64()).unwrap_or(0);
    let mode_s = op.payload.get("mode").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let receipt_no = op.payload.get("receipt_no").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let mode = match mode_s.as_str() {
        "cash" => vidya_core::types::PaymentMode::Cash,
        "upi" => vidya_core::types::PaymentMode::Upi,
        "cheque" => vidya_core::types::PaymentMode::Cheque,
        _ => return finalize(conn, op, rejected(&op.op_id, codes::MALFORMED)),
    };
    // A real payment is always accepted (§8.6) — but it must BE one: a known student,
    // a positive amount, a receipt number, only payment columns, and the sending device.
    let student_known = conn.query_row("SELECT 1 FROM student WHERE id=?1", params![student_id], |_| Ok(())).optional()?.is_some();
    let cols_ok = op.payload.as_object().is_some_and(|o| o.keys().all(|k| PAYMENT_COLS.contains(&k.as_str())));
    let device_ok = op.payload.get("device_id").and_then(|v| v.as_str()).map_or(true, |d| d == op.device_id);
    if !student_known || amount <= 0 || receipt_no.is_empty() || !cols_ok || !device_ok {
        return finalize(conn, op, rejected(&op.op_id, codes::MALFORMED));
    }

    // Dues (oldest first) for allocation.
    let dues: Vec<Due> = {
        let mut stmt = conn.prepare(
            "SELECT d.id, d.amount_paise - COALESCE((SELECT SUM(pa.amount_paise) FROM payment_allocation pa WHERE pa.fee_due_id=d.id AND pa.kind='due'),0) \
             FROM fee_due d WHERE d.student_id=?1 AND d.cancelled_at IS NULL ORDER BY d.created_at",
        )?;
        let v = stmt.query_map(params![student_id], |r| Ok(Due { id: r.get(0)?, balance: Paise(r.get::<_, i64>(1)?) }))?
            .collect::<rusqlite::Result<_>>()?;
        v
    };
    let synced = fees::apply_synced_payment(Paise(amount), &dues);

    let tx = conn.transaction()?;
    let cols = op.payload.as_object().cloned().unwrap_or_default();
    // Insert the payment row from the payload (+ bookkeeping, confirmed).
    let mut colnames: Vec<String> = vec!["id".into()];
    let mut place: Vec<String> = vec!["?".into()];
    let mut vals: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(op.record_id.clone())];
    for (c, v) in &cols {
        if c == "id" { continue; }
        colnames.push(c.clone());
        place.push("?".into());
        vals.push(json_to_sql(v));
    }
    for (c, v) in [("confirmed_at", now.clone()), ("hlc", op.hlc.clone()), ("created_at", now.clone()), ("updated_at", now.clone()), ("updated_by_staff", op.staff_id.clone()), ("updated_by_device", op.device_id.clone()), ("sync_state", "confirmed".to_string())] {
        colnames.push(c.into());
        place.push("?".into());
        vals.push(Box::new(v));
    }
    let sql = format!("INSERT INTO payment ({}) VALUES ({})", colnames.join(","), place.join(","));
    let refs: Vec<&dyn rusqlite::ToSql> = vals.iter().map(|b| b.as_ref()).collect();
    tx.execute(&sql, refs.as_slice())?;

    // Allocations.
    for a in &synced.allocations {
        tx.execute(
            "INSERT INTO payment_allocation(id,payment_id,fee_due_id,amount_paise,kind) VALUES (?1,?2,?3,?4,?5)",
            params![format!("al-{}-{}", op.op_id, a.due_id.clone().unwrap_or_else(|| "adv".into())), op.record_id, a.due_id, a.amount.get(), match a.kind { AllocKind::Due => "due", AllocKind::AdvanceCredit => "advance_credit" }],
        )?;
    }
    // Excess → review_flag (§8.6).
    let status = if let Some(excess) = synced.excess_payment {
        tx.execute(
            "INSERT INTO review_flag(id,kind,ref_table,ref_id,details_json,status) VALUES (?1,'excess_payment','payment',?2,?3,'open')",
            params![format!("rf-{}", op.op_id), op.record_id, serde_json::json!({ "excess_paise": excess.get() }).to_string()],
        )?;
        OpStatus::Flagged
    } else {
        OpStatus::Confirmed
    };

    // The balanced receipt voucher, in the SAME transaction (idempotent per payment).
    let school_id: Option<String> = tx.query_row("SELECT id FROM school LIMIT 1", [], |r| r.get(0)).optional()?;
    crate::ledger::post_payment_voucher(
        &tx, &op.record_id, &receipt_no, mode, amount, &now,
        Some(op.staff_id.as_str()), Some(op.device_id.as_str()), school_id.as_deref(), &now, "confirmed",
    )?;

    let server_seq = write_oplog_audit(&tx, op, &now)?;
    let record = read_row_json(&tx, "payment", &op.record_id)?;
    let result = OpResult { op_id: op.op_id.clone(), status, server_seq: Some(server_seq), record, reason_code: if status == OpStatus::Flagged { Some("excess_payment".into()) } else { None } };
    remember(&tx, op, &result)?;
    tx.commit()?;
    Ok(result)
}

fn write_oplog_audit(tx: &Transaction, op: &Op, now: &str) -> rusqlite::Result<i64> {
    tx.execute(
        "INSERT INTO op_log(op_id,hlc,device_id,staff_id,\"table\",record_id,kind,payload,applied_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![op.op_id, op.hlc, op.device_id, op.staff_id, op.table, op.record_id, op.kind, op.payload.to_string(), now],
    )?;
    let server_seq = tx.last_insert_rowid();
    audit::append(tx, &AuditEntry {
        at: now.into(),
        staff_id: Some(op.staff_id.clone()),
        device_id: Some(op.device_id.clone()),
        action: format!("sync_{}", op.kind),
        table: Some(op.table.clone()),
        record_id: Some(op.record_id.clone()),
        after_json: Some(op.payload.to_string()),
        op_id: Some(op.op_id.clone()),
        ..Default::default()
    })?;
    Ok(server_seq)
}

fn remember(tx: &Transaction, op: &Op, result: &OpResult) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT OR REPLACE INTO applied_ops(op_id, applied_at, result) VALUES (?1,?2,?3)",
        params![op.op_id, now_iso(), serde_json::to_string(result).unwrap_or_default()],
    )?;
    Ok(())
}

fn finalize(conn: &mut Connection, op: &Op, result: OpResult) -> rusqlite::Result<OpResult> {
    // rejected/flagged ops are still remembered (idempotency) but change no domain row.
    conn.execute(
        "INSERT OR REPLACE INTO applied_ops(op_id, applied_at, result) VALUES (?1,?2,?3)",
        params![op.op_id, now_iso(), serde_json::to_string(&result).unwrap_or_default()],
    )?;
    Ok(result)
}

fn json_to_sql(v: &serde_json::Value) -> Box<dyn rusqlite::ToSql> {
    match v {
        serde_json::Value::Null => Box::new(Option::<String>::None),
        serde_json::Value::Bool(b) => Box::new(if *b { 1i64 } else { 0 }),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { Box::new(i) } else { Box::new(n.as_f64().unwrap_or(0.0).to_string()) }
        }
        serde_json::Value::String(s) => Box::new(s.clone()),
        other => Box::new(other.to_string()),
    }
}

/// Read a row as a flat JSON object (used for the canonical `record`).
pub fn read_row_json(conn: &Connection, table: &str, id: &str) -> rusqlite::Result<Option<serde_json::Value>> {
    let sql = format!("SELECT * FROM {table} WHERE id=?1");
    let mut stmt = conn.prepare(&sql)?;
    let cols: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        let mut map = serde_json::Map::new();
        for (i, col) in cols.iter().enumerate() {
            let v: rusqlite::types::Value = row.get(i)?;
            map.insert(col.clone(), sql_to_json(v));
        }
        Ok(Some(serde_json::Value::Object(map)))
    } else {
        Ok(None)
    }
}

fn sql_to_json(v: rusqlite::types::Value) -> serde_json::Value {
    use rusqlite::types::Value;
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Integer(i) => serde_json::Value::Number(i.into()),
        Value::Real(f) => serde_json::json!(f),
        Value::Text(s) => serde_json::Value::String(s),
        Value::Blob(b) => serde_json::Value::String(base64_of(&b)),
    }
}

fn base64_of(b: &[u8]) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    STANDARD.encode(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::seed::seed_demo_school;
    use crate::sync::protocol::OpStatus;
    use time::OffsetDateTime;
    use vidya_core::hlc::Hlc;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn seeded() -> Connection {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        let now = OffsetDateTime::parse("2026-09-23T09:00:00Z", &time::format_description::well_known::Rfc3339).unwrap();
        seed_demo_school(&mut c, now).unwrap();
        c
    }

    /// A draft (pending) sheet + one of its class's students — a fresh
    /// (sheet_id, student_id) pair so an inserted mark never collides.
    fn draft_sheet_and_student(conn: &Connection) -> (String, String) {
        let (sheet_id, class_id): (String, String) = conn
            .query_row("SELECT id, class_id FROM attendance_sheet WHERE status='draft' LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        let student_id: String = conn
            .query_row("SELECT student_id FROM enrollment WHERE class_id=?1 AND to_date IS NULL LIMIT 1", params![class_id], |r| r.get(0))
            .unwrap();
        (sheet_id, student_id)
    }

    /// An attendance_mark op authored by the Principal (passes permission + the
    /// core module), carrying `mark`, at HLC wall-time `wall_ms`.
    fn mark_op(sheet_id: &str, student_id: &str, mark: &str, wall_ms: u64) -> Op {
        Op {
            op_id: format!("op-{wall_ms}-{mark}"),
            hlc: Hlc::new(wall_ms, 0, "dev-a1").to_string_form(),
            device_id: "dev-a1".into(),
            staff_id: "stf-priya".into(),
            audience: "class:cls-7b".into(),
            table: "attendance_mark".into(),
            record_id: format!("mk-test-{wall_ms}-{mark}"),
            kind: "insert".into(),
            payload: serde_json::json!({ "sheet_id": sheet_id, "student_id": student_id, "mark": mark }),
            base_version: None,
            server_epoch: 1,
        }
    }

    // The 2026-09-25 cutover ≈ 1.7904e12 ms; these straddle it.
    const AFTER_CUTOVER_MS: u64 = 1_800_000_000_000;
    const BEFORE_CUTOVER_MS: u64 = 1_600_000_000_000;

    #[test]
    fn leave_op_after_cutover_is_rejected() {
        let mut c = seeded();
        let (sheet, student) = draft_sheet_and_student(&c);
        let res = apply_op(&mut c, &mark_op(&sheet, &student, "L", AFTER_CUTOVER_MS)).unwrap();
        assert_eq!(res.status, OpStatus::Rejected);
        assert_eq!(res.reason_code.as_deref(), Some(codes::LEAVE_MARK_REMOVED));
        // Rejected → nothing written.
        let n: i64 = c.query_row("SELECT COUNT(*) FROM attendance_mark WHERE sheet_id=?1 AND student_id=?2", params![sheet, student], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn leave_op_before_cutover_applies_as_legacy() {
        let mut c = seeded();
        let (sheet, student) = draft_sheet_and_student(&c);
        let res = apply_op(&mut c, &mark_op(&sheet, &student, "L", BEFORE_CUTOVER_MS)).unwrap();
        assert_eq!(res.status, OpStatus::Confirmed, "pre-cutover L applies as legacy");
        let mark: String = c.query_row("SELECT mark FROM attendance_mark WHERE sheet_id=?1 AND student_id=?2", params![sheet, student], |r| r.get(0)).unwrap();
        assert_eq!(mark, "L");
    }

    #[test]
    fn present_op_after_cutover_applies() {
        // Only Leave is removed; a P mark after the cutover applies normally.
        let mut c = seeded();
        let (sheet, student) = draft_sheet_and_student(&c);
        let res = apply_op(&mut c, &mark_op(&sheet, &student, "P", AFTER_CUTOVER_MS)).unwrap();
        assert_eq!(res.status, OpStatus::Confirmed);
    }

    /// A `store_item` op authored by the Principal. `store_item` is NOT mapped by
    /// `action_for`, so before the P18 fix its module switch was never checked
    /// server-side.
    fn store_item_op() -> Op {
        Op {
            op_id: "op-store-1".into(),
            hlc: Hlc::new(AFTER_CUTOVER_MS, 0, "dev-a1").to_string_form(),
            device_id: "dev-a1".into(),
            staff_id: "stf-priya".into(),
            audience: "finance".into(),
            table: "store_item".into(),
            record_id: "si-test-1".into(),
            kind: "insert".into(),
            payload: serde_json::json!({ "id": "si-test-1", "name": "Notebook", "price_paise": 5000, "stock": 10 }),
            base_version: None,
            server_epoch: 1,
        }
    }

    #[test]
    fn module_off_op_is_rejected_server_side_even_for_unmapped_tables() {
        // `store` is OFF by default (§14). A `store_item` op — a table `action_for`
        // does not map — must be rejected server-side with MODULE_OFF before any
        // write (§8.4), not silently applied. Regression for the P18 security fix.
        let mut c = seeded();
        assert!(!crate::modules::is_enabled(&c, "store").unwrap(), "store is off by default");
        let res = apply_op(&mut c, &store_item_op()).unwrap();
        assert_eq!(res.status, OpStatus::Rejected);
        assert_eq!(res.reason_code.as_deref(), Some(codes::MODULE_OFF));
        let n: i64 = c.query_row("SELECT COUNT(*) FROM store_item WHERE id='si-test-1'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0, "rejected → nothing written");
    }

    #[test]
    fn module_on_op_passes_the_module_gate() {
        // With the module ON the same op is no longer module-gated, and the Principal
        // (who holds ManageStore) passes the permission re-check too, so it applies.
        let mut c = seeded();
        c.execute("UPDATE module_setting SET enabled=1 WHERE key='store'", []).unwrap();
        let res = apply_op(&mut c, &store_item_op()).unwrap();
        assert_ne!(res.reason_code.as_deref(), Some(codes::MODULE_OFF), "module on → not module-gated");
    }

    /// An op on a P15/P16 module table authored by `author`. `accounts` is ON by
    /// default, so these reach the permission re-check.
    fn op_on(table: &str, record_id: &str, author: &str, payload: serde_json::Value) -> Op {
        Op {
            op_id: format!("op-{table}-{author}"),
            hlc: Hlc::new(AFTER_CUTOVER_MS, 0, "dev-a1").to_string_form(),
            device_id: "dev-a1".into(),
            staff_id: author.into(),
            audience: "finance".into(),
            table: table.into(),
            record_id: record_id.into(),
            kind: "insert".into(),
            payload,
            base_version: None,
            server_epoch: 1,
        }
    }

    #[test]
    fn expense_op_from_a_teacher_is_rejected_by_the_server() {
        // A teacher cannot RecordExpense. Before the P18 fix `expense` was un-mapped
        // by `action_for`, so a forged expense op skipped the permission re-check and
        // was written. Now the server rejects it (Rule 7 — re-validate every op).
        let mut c = seeded();
        let op = op_on("expense", "exp-forge", "stf-meena",
            serde_json::json!({ "id": "exp-forge", "amount_paise": 5000, "paid_via": "cash", "category_account_id": "5002", "spent_on": "2026-09-23" }));
        let res = apply_op(&mut c, &op).unwrap();
        assert_eq!(res.status, OpStatus::Rejected);
        assert_eq!(res.reason_code.as_deref(), Some(codes::FORBIDDEN));
        let n: i64 = c.query_row("SELECT COUNT(*) FROM expense WHERE id='exp-forge'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0, "rejected → nothing written");
    }

    #[test]
    fn salary_op_from_the_accountant_is_rejected_by_the_server() {
        // ManageSalary is Principal-only, so even the accountant's forged salary op is
        // rejected server-side (previously `salary_structure` skipped the re-check).
        let mut c = seeded();
        let op = op_on("salary_structure", "sal-forge", "stf-suresh",
            serde_json::json!({ "id": "sal-forge", "staff_id": "stf-meena", "monthly_paise": 2_000_000, "effective_from": "2026-04-01" }));
        let res = apply_op(&mut c, &op).unwrap();
        assert_eq!(res.status, OpStatus::Rejected);
        assert_eq!(res.reason_code.as_deref(), Some(codes::FORBIDDEN));
    }
}
