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
        _ => None,
    }
}

fn target_for(table: &str, payload: &serde_json::Value) -> Target {
    let class_id = payload.get("class_id").and_then(|v| v.as_str()).map(str::to_string);
    let kind = match table {
        "student" | "enrollment" => TargetKind::Student,
        "payment" | "fee_due" => TargetKind::Fee,
        "attendance_sheet" | "attendance_mark" => TargetKind::Attendance,
        "mark_entry" | "marks_sheet" => TargetKind::Marks,
        _ => TargetKind::Own,
    };
    Target { kind, class_id, ..Default::default() }
}

/// Resolve the class a target belongs to (attendance ops don't carry class_id).
fn resolve_class_id(conn: &Connection, table: &str, record_id: &str, payload: &serde_json::Value) -> rusqlite::Result<Option<String>> {
    if let Some(c) = payload.get("class_id").and_then(|v| v.as_str()) {
        return Ok(Some(c.to_string()));
    }
    match table {
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

/// Apply one op. Idempotent by `op_id`. Returns the per-op result.
pub fn apply_op(conn: &mut Connection, op: &Op) -> rusqlite::Result<OpResult> {
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
    if let Some(action) = action_for(&op.table, &op.kind) {
        let enabled = crate::modules::enabled_set(conn)?;
        if vidya_core::modules::require_module(&enabled, action).is_err() {
            return finalize(conn, op, rejected(&op.op_id, codes::MODULE_OFF));
        }
        let mut target = target_for(&op.table, &op.payload);
        if target.class_id.is_none() {
            target.class_id = resolve_class_id(conn, &op.table, &op.record_id, &op.payload)?;
        }
        let decision = permissions::can(&actor, action, &target);
        if !decision.is_allow() {
            return finalize(conn, op, rejected(&op.op_id, codes::FORBIDDEN));
        }
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
fn apply_payment(conn: &mut Connection, op: &Op) -> rusqlite::Result<OpResult> {
    let now = now_iso();
    let student_id = op.payload.get("student_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let amount = op.payload.get("amount_paise").and_then(|v| v.as_i64()).unwrap_or(0);

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
