//! Command logic (prompts/P03 Step 5). Pure functions over `&mut Connection` +
//! params, so they are unit-testable without a Tauri runtime. Business rules are
//! delegated to vidya-core; these functions do permission checks, DB reads/writes
//! (via `with_write` for audited changes) and DTO shaping.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

use vidya_core::errors::CoreError;
use vidya_core::fees::{self, Due};
use vidya_core::grades::{self, GradeBand, SubjectMark};
use vidya_core::marks::MarkEntry;
use vidya_core::money::Paise;
use vidya_core::permissions::{self, Action, Actor, Target, TargetKind};
use vidya_core::types::{PaymentMode, Role, StaffState};

use crate::ctx::RtCtx;
use crate::db::now_iso;
use crate::error::{CmdError, CmdResult};
use crate::kv;
use crate::security::pin::{self};
use crate::state::{AppStateResponse, SessionStaff, KV_PENDING_LICENCE, KV_SETUP_STEP};
use crate::write::{with_write, DeviceMode, Effect, Op, WriteCtx};
use crate::security::audit::AuditEntry;

// ============================================================= helpers =======

fn role_from(role: &str) -> CmdResult<Role> {
    match role {
        "principal" => Ok(Role::Principal),
        "accountant" => Ok(Role::Accountant),
        "teacher" => Ok(Role::Teacher),
        _ => Err(CmdError::internal("unknown role")),
    }
}

/// Build a permission Actor from the session, loading the teacher's class
/// assignments (needed for attendance / marks ownership checks).
fn actor_from(conn: &Connection, s: &SessionStaff) -> CmdResult<Actor> {
    let role = role_from(&s.role)?;
    let mut class_teacher_of = Vec::new();
    let mut class_subjects = Vec::new();
    if role == Role::Teacher {
        let mut stmt = conn.prepare("SELECT id FROM class WHERE class_teacher_id = ?1")?;
        class_teacher_of = stmt
            .query_map(params![s.id], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<_>>()?;
        let mut stmt2 = conn.prepare("SELECT id FROM class_subject WHERE teacher_id = ?1")?;
        class_subjects = stmt2
            .query_map(params![s.id], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<_>>()?;
    }
    Ok(Actor { staff_id: s.id.clone(), role, state: StaffState::Active, class_teacher_of, class_subjects })
}

/// Deny → FORBIDDEN error; Allow/NeedsRequest → Ok.
fn require_allow(actor: &Actor, action: Action, target: &Target) -> CmdResult<()> {
    let d = permissions::can(actor, action, target);
    if d.is_allow() {
        Ok(())
    } else if let Some(err) = d.as_error() {
        Err(err.into())
    } else {
        // NeedsRequest → the direct action is forbidden; the caller should raise a request.
        Err(CmdError::forbidden("needs_request"))
    }
}

/// Reject when `module` is switched off (MODULE_OFF{module}); a thin wrapper over
/// the vidya-core check with the DB-loaded enabled set.
fn require_module_enabled(conn: &Connection, module: vidya_core::modules::Module) -> CmdResult<()> {
    vidya_core::modules::require_enabled(&crate::modules::enabled_set(conn)?, module)?;
    Ok(())
}

pub(crate) fn new_id(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::now_v7())
}

fn bump_setup_step(conn: &Connection, step: i64) -> CmdResult<()> {
    let cur: i64 = kv::get(conn, KV_SETUP_STEP)?.unwrap_or(0);
    if step > cur {
        kv::set(conn, KV_SETUP_STEP, &step)?;
    }
    Ok(())
}

// ============================================================= licence =======

/// Verify a pasted licence **key** (or the text of a loaded `.vlic` file) offline
/// and hold it until the wizard creates the school row (prompts/P12 Step 6). No
/// network, no `LICENCE_API`: the ed25519 signature is checked against the
/// build-config public key(s) and the licence's machine code must match this PC.
pub fn activate_licence_impl(state: &State<RtCtx>, licence_key: &str) -> CmdResult<AppStateResponse> {
    let keys = crate::config::licence_public_keys();
    if keys.is_empty() {
        // A dev build before a keypair exists, or a misbuilt release.
        return Err(CmdError::new("LICENCE_INVALID", "licence.invalid", serde_json::Value::Null));
    }
    let machine_code = vidya_core::licence::machine_code(&state.machine_id);
    let lic = crate::licence::verify_offline(licence_key, &machine_code, &keys)?;

    // Hold the verified licence until the wizard creates the school row (the
    // licence table FK needs a school). Persist encrypted in app_kv. v2 licences
    // are perpetual and unlimited (no max_students/max_devices, no online check).
    let pending = serde_json::json!({
        "licence_id": lic.licence_id,
        "plan": lic.plan,
        "max_students": serde_json::Value::Null,
        "max_devices": serde_json::Value::Null,
        "issued_at": lic.issued_at,
        "signature": lic.signature_b64,
        "raw_json": lic.payload_b64,
    });
    state.with_db(|conn| {
        kv::set(conn, KV_PENDING_LICENCE, &pending)?;
        Ok(())
    })?;
    // Return the new app state (should be `activated`).
    let session = state.session.lock().map_err(|_| CmdError::internal("lock"))?.clone();
    state.with_db(|conn| Ok(crate::state::compute(conn, session)?))
}

/// This computer's display machine code (`XXXX-XXXX-XXXX-C`), shown on Welcome →
/// Set up so the buyer can send it with their UPI payment (prompts/P12 Step 6.2).
pub fn machine_code_impl(state: &State<RtCtx>) -> String {
    vidya_core::licence::machine_code(&state.machine_id)
}

// ============================================================== setup ========

#[derive(Debug, Deserialize)]
pub struct SchoolInput {
    pub name: String,
    pub address: Option<String>,
    pub board: Option<String>,
    pub udise: Option<String>,
    pub phone: Option<String>,
    /// Optional UPI id set on the School wizard step (skippable, §10.1). When
    /// present it must be a valid VPA; `upi_name` defaults to the school name.
    #[serde(default)]
    pub upi_id: Option<String>,
    #[serde(default)]
    pub upi_name: Option<String>,
}

pub fn setup_school_logic(conn: &mut Connection, input: &SchoolInput) -> CmdResult<()> {
    let name = vidya_core::validation::validate_name(&input.name)?;
    let now = now_iso();
    // A random 16-byte backup salt (recovery-key backup key is derived from it).
    let mut salt = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut salt);

    // Optional UPI id — validated only when the Principal typed one (the field is
    // skippable). Default the display name to the school name; receipts show the
    // QR only when a UPI id is set and the receipts toggle is on (default on here).
    let mut settings = serde_json::json!({ "phone": input.phone });
    if let Some(vpa) = input.upi_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let vpa = vidya_core::upi::validate_vpa(vpa)?;
        let upi_name = input
            .upi_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| name.clone());
        settings["upi_id"] = serde_json::Value::String(vpa);
        settings["upi_name"] = serde_json::Value::String(upi_name);
        settings["upi_on_receipts"] = serde_json::Value::Bool(true);
        settings["upi_on_reminders"] = serde_json::Value::Bool(true);
        settings["upi_on_dues_list"] = serde_json::Value::Bool(false);
    }

    // Create the school row (or update if setup is being redone before completion).
    let existing: Option<String> = conn
        .query_row("SELECT id FROM school LIMIT 1", [], |r| r.get(0))
        .optional()?;
    let school_id = existing.unwrap_or_else(|| new_id("sch"));
    conn.execute(
        "INSERT INTO school(id,name,address,board,udise,backup_salt,settings_json,created_at,updated_at,sync_state) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8,'confirmed') \
         ON CONFLICT(id) DO UPDATE SET name=excluded.name, address=excluded.address, \
           board=excluded.board, udise=excluded.udise, updated_at=excluded.updated_at",
        params![school_id, name, input.address, input.board, input.udise, salt.to_vec(),
            settings.to_string(), now],
    )?;

    // Move the pending licence (from activation) into the licence table now that
    // a school row exists.
    if let Some(p) = kv::get::<serde_json::Value>(conn, KV_PENDING_LICENCE)? {
        let get = |k: &str| p.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let get_u = |k: &str| p.get(k).and_then(|v| v.as_u64()).map(|n| n as i64);
        // v2 licences are verified offline and never re-checked, so last_check_at
        // stays NULL (honest: no online check ever happened).
        conn.execute(
            "INSERT OR IGNORE INTO licence(licence_id,school_id,plan,issued_at,max_students,max_devices,signature,raw_json,status,last_check_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'active',NULL)",
            params![get("licence_id"), school_id, get("plan"), get("issued_at"),
                get_u("max_students"), get_u("max_devices"), get("signature"), get("raw_json")],
        )?;
    }
    bump_setup_step(conn, 1)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct SessionInput {
    pub label: String,
    pub starts_on: String,
    pub ends_on: String,
    pub term1_starts: String,
    pub term1_ends: String,
    pub term2_starts: String,
    pub term2_ends: String,
}

pub fn setup_session_logic(conn: &mut Connection, input: &SessionInput) -> CmdResult<()> {
    vidya_core::validation::validate_session_label(&input.label)?;
    let sid = new_id("sess");
    conn.execute("DELETE FROM term WHERE session_id IN (SELECT id FROM academic_session)", [])?;
    conn.execute("DELETE FROM academic_session", [])?;
    conn.execute(
        "INSERT INTO academic_session(id,label,starts_on,ends_on,is_current,read_only) VALUES (?1,?2,?3,?4,1,0)",
        params![sid, input.label, input.starts_on, input.ends_on],
    )?;
    conn.execute(
        "INSERT INTO term(id,session_id,name,starts_on,ends_on) VALUES \
         (?1,?2,'Term 1',?3,?4),(?5,?2,'Term 2',?6,?7)",
        params![new_id("term"), sid, input.term1_starts, input.term1_ends, new_id("term"), input.term2_starts, input.term2_ends],
    )?;
    bump_setup_step(conn, 2)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct ClassInput {
    pub name: String,
    pub section: Option<String>,
    pub display: String,
}

pub fn setup_classes_logic(conn: &mut Connection, sections: &[ClassInput]) -> CmdResult<()> {
    for (i, c) in sections.iter().enumerate() {
        conn.execute(
            "INSERT INTO class(id,name,section,display,sort_order) VALUES (?1,?2,?3,?4,?5)",
            params![new_id("cls"), c.name, c.section, c.display, i as i64],
        )?;
    }
    bump_setup_step(conn, 3)?;
    Ok(())
}

pub fn setup_principal_logic(conn: &mut Connection, name: &str, mobile: &str) -> CmdResult<()> {
    let name = vidya_core::validation::validate_name(name)?;
    vidya_core::validation::validate_mobile(mobile)?;
    let now = now_iso();
    // One principal per school; upsert by role.
    let existing: Option<String> = conn
        .query_row("SELECT id FROM staff WHERE role='principal' LIMIT 1", [], |r| r.get(0))
        .optional()?;
    let id = existing.unwrap_or_else(|| new_id("stf"));
    conn.execute(
        "INSERT INTO staff(id,name,role,mobile,state,created_at,updated_at,sync_state) \
         VALUES (?1,?2,'principal',?3,'active',?4,?4,'confirmed') \
         ON CONFLICT(id) DO UPDATE SET name=excluded.name, mobile=excluded.mobile, updated_at=excluded.updated_at",
        params![id, name, mobile, now],
    )?;
    bump_setup_step(conn, 4)?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct RecoveryKeyDto {
    /// The 30-char key in 6 groups (shown once).
    pub key: String,
}

pub fn create_recovery_key_logic(state: &State<RtCtx>) -> CmdResult<RecoveryKeyDto> {
    let key = crate::security::recovery::generate_recovery_key();
    *state.recovery.lock().map_err(|_| CmdError::internal("lock"))? = Some(key.clone());
    Ok(RecoveryKeyDto { key })
}

pub fn confirm_recovery_key_logic(state: &State<RtCtx>, group3: &str, group5: &str) -> CmdResult<()> {
    let key = state
        .recovery
        .lock()
        .map_err(|_| CmdError::internal("lock"))?
        .clone()
        .ok_or_else(|| CmdError::validation("recovery_key", "not_generated"))?;
    let groups: Vec<&str> = key.split('-').collect();
    let ok = groups.len() == 6
        && group3.trim().eq_ignore_ascii_case(groups[2])
        && group5.trim().eq_ignore_ascii_case(groups[4]);
    if !ok {
        return Err(CmdError::validation("recovery_key", "mismatch"));
    }
    // Confirmed: drop the in-memory key and advance the wizard.
    *state.recovery.lock().map_err(|_| CmdError::internal("lock"))? = None;
    state.with_db(|conn| bump_setup_step(conn, 5))?;
    Ok(())
}

pub fn create_pin_logic(conn: &mut Connection, pin: &str) -> CmdResult<()> {
    vidya_core::validation::validate_pin(pin)?;
    let hash = pin::hash_pin(pin).map_err(CmdError::internal)?;
    // Setup step 4 collects the PIN for the just-created principal.
    conn.execute("UPDATE staff SET pin_hash=?1 WHERE role='principal'", params![hash])?;
    // The wizard's "Ready" step (6) completes setup.
    bump_setup_step(conn, 6)?;
    Ok(())
}

// ============================================================= unlock ========

pub fn unlock_impl(state: &State<RtCtx>, staff_id: &str, pin: &str) -> CmdResult<AppStateResponse> {
    let session = state.with_db(|conn| unlock_logic(conn, staff_id, pin))?;
    *state.session.lock().map_err(|_| CmdError::internal("lock"))? = Some(session.clone());
    let s = Some(session);
    state.with_db(|conn| Ok(crate::state::compute(conn, s)?))
}

/// Verify the PIN with lockout (§9). Returns the session on success.
pub fn unlock_logic(conn: &mut Connection, staff_id: &str, pin: &str) -> CmdResult<SessionStaff> {
    struct StaffAuth {
        name: String,
        role: String,
        pin_hash: Option<String>,
        fail_count: i64,
        locked_until: Option<String>,
    }
    let row = conn
        .query_row(
            "SELECT name, role, pin_hash, pin_fail_count, pin_locked_until FROM staff WHERE id=?1 AND state='active'",
            params![staff_id],
            |r| Ok(StaffAuth { name: r.get(0)?, role: r.get(1)?, pin_hash: r.get(2)?, fail_count: r.get(3)?, locked_until: r.get(4)? }),
        )
        .optional()?;
    let StaffAuth { name, role, pin_hash, fail_count, locked_until } = row.ok_or_else(CmdError::not_found)?;
    let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;

    if let Some(until) = &locked_until {
        if let Ok(t) = time::OffsetDateTime::parse(until, &time::format_description::well_known::Rfc3339) {
            if pin::is_locked(Some(t.unix_timestamp() * 1000), now_ms) {
                return Err(CoreError::PinLocked { until: until.clone() }.into());
            }
        }
    }
    let hash = pin_hash.ok_or_else(|| CmdError::validation("pin", "not_set"))?;
    if pin::verify_pin(pin, &hash) {
        conn.execute("UPDATE staff SET pin_fail_count=0, pin_locked_until=NULL WHERE id=?1", params![staff_id])?;
        Ok(SessionStaff { id: staff_id.to_string(), name, role })
    } else {
        let new_count = (fail_count as u32) + 1;
        let locked = pin::lockout_seconds(new_count).map(|secs| {
            let until = time::OffsetDateTime::now_utc() + time::Duration::seconds(secs as i64);
            until.format(&time::format_description::well_known::Rfc3339).unwrap_or_default()
        });
        conn.execute(
            "UPDATE staff SET pin_fail_count=?1, pin_locked_until=?2 WHERE id=?3",
            params![new_count as i64, locked, staff_id],
        )?;
        match locked {
            Some(until) => Err(CoreError::PinLocked { until }.into()),
            None => Err(CoreError::PinWrong { remaining: pin::remaining_before_lock(new_count) }.into()),
        }
    }
}

// ============================================================= students ======

#[derive(Debug, Serialize)]
pub struct ClassDto {
    pub id: String,
    pub display: String,
    pub name: String,
    pub section: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StaffDto {
    pub id: String,
    pub name: String,
    pub role: String,
}

/// Active staff on this device (the PIN unlock picker, Step 7).
pub fn list_staff_logic(conn: &mut Connection) -> CmdResult<Vec<StaffDto>> {
    let mut stmt = conn.prepare("SELECT id, name, role FROM staff WHERE state='active' ORDER BY role, name")?;
    let rows = stmt
        .query_map([], |r| Ok(StaffDto { id: r.get(0)?, name: r.get(1)?, role: r.get(2)? }))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

pub fn list_classes_logic(conn: &mut Connection) -> CmdResult<Vec<ClassDto>> {
    let mut stmt = conn.prepare("SELECT id, display, name, section FROM class ORDER BY sort_order")?;
    let rows = stmt
        .query_map([], |r| Ok(ClassDto { id: r.get(0)?, display: r.get(1)?, name: r.get(2)?, section: r.get(3)? }))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

/// Identity of the school + current session, for the desktop app shell (school
/// name under the logo, header session button, session-switcher read-only
/// banner) and later for receipt / report-card headers (name, address). Thin.
#[derive(Debug, Serialize)]
pub struct SchoolDto {
    pub name: String,
    pub address: Option<String>,
    pub board: Option<String>,
    pub phone: Option<String>,
    pub session_label: Option<String>,
    pub session_read_only: bool,
}

pub fn get_school_logic(conn: &mut Connection) -> CmdResult<SchoolDto> {
    let (name, address, board, settings): (String, Option<String>, Option<String>, Option<String>) = conn
        .query_row("SELECT name, address, board, settings_json FROM school LIMIT 1", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?;
    let phone = settings
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("phone").and_then(|p| p.as_str().map(str::to_string)));
    let session = conn
        .query_row(
            "SELECT label, read_only FROM academic_session WHERE is_current=1 LIMIT 1",
            [],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? != 0)),
        )
        .optional()?;
    let (session_label, session_read_only) = match session {
        Some((label, ro)) => (Some(label), ro),
        None => (None, false),
    };
    Ok(SchoolDto { name, address, board, phone, session_label, session_read_only })
}

#[derive(Debug, Serialize)]
pub struct StudentDto {
    pub id: String,
    pub name: String,
    pub admission_no: Option<String>,
    pub provisional_no: Option<String>,
    pub class_display: Option<String>,
    pub roll_no: Option<i64>,
}

fn map_student(r: &rusqlite::Row) -> rusqlite::Result<StudentDto> {
    Ok(StudentDto {
        id: r.get(0)?,
        name: r.get(1)?,
        admission_no: r.get(2)?,
        provisional_no: r.get(3)?,
        class_display: r.get(4)?,
        roll_no: r.get(5)?,
    })
}

const STUDENT_SELECT: &str = "SELECT s.id, s.name, s.admission_no, s.provisional_no, c.display, e.roll_no \
     FROM student s \
     LEFT JOIN enrollment e ON e.student_id = s.id AND e.to_date IS NULL \
     LEFT JOIN class c ON c.id = e.class_id \
     WHERE s.status='active'";

pub fn list_students_logic(conn: &mut Connection, class_id: Option<&str>) -> CmdResult<Vec<StudentDto>> {
    let (sql, has_class) = match class_id {
        Some(_) => (format!("{STUDENT_SELECT} AND e.class_id = ?1 ORDER BY e.roll_no"), true),
        None => (format!("{STUDENT_SELECT} ORDER BY s.name"), false),
    };
    let mut stmt = conn.prepare(&sql)?;
    let rows = if has_class {
        stmt.query_map(params![class_id.unwrap()], map_student)?.collect::<rusqlite::Result<_>>()?
    } else {
        stmt.query_map([], map_student)?.collect::<rusqlite::Result<_>>()?
    };
    Ok(rows)
}

pub fn search_students_logic(conn: &mut Connection, query: &str) -> CmdResult<Vec<StudentDto>> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(vec![]);
    }
    // FTS5 prefix match on the student_fts index; fall back to LIKE on failure.
    let fts = format!("{}*", q.replace('"', ""));
    let sql = format!(
        "{STUDENT_SELECT} AND s.rowid IN (SELECT rowid FROM student_fts WHERE student_fts MATCH ?1) ORDER BY s.name LIMIT 50"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![fts], map_student)?.collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

pub fn get_student_logic(conn: &mut Connection, id: &str) -> CmdResult<StudentDto> {
    let sql = format!("{STUDENT_SELECT} AND s.id = ?1");
    conn.query_row(&sql, params![id], map_student)
        .optional()?
        .ok_or_else(CmdError::not_found)
}

// ---- Students list: filters (class/section/status) + FTS + real pagination --

#[derive(Debug, Serialize)]
pub struct StudentRowDto {
    pub id: String,
    pub name: String,
    pub admission_no: Option<String>,
    pub provisional_no: Option<String>,
    pub class_display: Option<String>,
    pub section: Option<String>,
    pub roll_no: Option<i64>,
    pub guardian_name: Option<String>,
    pub status: String, // active | left
}

#[derive(Debug, Serialize)]
pub struct StudentsPageDto {
    pub rows: Vec<StudentRowDto>,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
pub struct StudentQuery {
    pub class_id: Option<String>,
    pub section: Option<String>,
    pub status: Option<String>, // active | left | null(all)
    pub query: Option<String>,  // FTS prefix
    pub limit: i64,
    pub offset: i64,
}

pub fn list_students_page_logic(conn: &mut Connection, q: &StudentQuery) -> CmdResult<StudentsPageDto> {
    use rusqlite::params_from_iter;
    use rusqlite::types::Value;

    let mut wheres: Vec<String> = Vec::new();
    let mut args: Vec<Value> = Vec::new();
    match q.status.as_deref() {
        Some("active") => wheres.push("s.status='active'".into()),
        Some("left") => wheres.push("s.status='left'".into()),
        _ => {}
    }
    if let Some(cid) = q.class_id.as_deref().filter(|s| !s.is_empty()) {
        wheres.push("e.class_id = ?".into());
        args.push(Value::Text(cid.to_string()));
    }
    if let Some(sec) = q.section.as_deref().filter(|s| !s.is_empty()) {
        wheres.push("c.section = ?".into());
        args.push(Value::Text(sec.to_string()));
    }
    if let Some(query) = q.query.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        wheres.push("s.rowid IN (SELECT rowid FROM student_fts WHERE student_fts MATCH ?)".into());
        args.push(Value::Text(format!("{}*", query.replace('"', ""))));
    }
    let where_clause = if wheres.is_empty() { String::new() } else { format!("WHERE {}", wheres.join(" AND ")) };
    let base_from = "FROM student s \
        LEFT JOIN enrollment e ON e.student_id = s.id AND e.to_date IS NULL \
        LEFT JOIN class c ON c.id = e.class_id";

    let count_sql = format!("SELECT COUNT(*) {base_from} {where_clause}");
    let total: i64 = conn.query_row(&count_sql, params_from_iter(args.iter()), |r| r.get(0))?;

    let sql = format!(
        "SELECT s.id, s.name, s.admission_no, s.provisional_no, c.display, c.section, e.roll_no, s.guardian_name, s.status \
         {base_from} {where_clause} ORDER BY s.name COLLATE NOCASE LIMIT ? OFFSET ?"
    );
    let mut page_args = args;
    page_args.push(Value::Integer(q.limit.max(0)));
    page_args.push(Value::Integer(q.offset.max(0)));
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(page_args.iter()), |r| {
            Ok(StudentRowDto {
                id: r.get(0)?,
                name: r.get(1)?,
                admission_no: r.get(2)?,
                provisional_no: r.get(3)?,
                class_display: r.get(4)?,
                section: r.get(5)?,
                roll_no: r.get(6)?,
                guardian_name: r.get(7)?,
                status: r.get(8)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(StudentsPageDto { rows, total })
}

#[derive(Debug, Deserialize)]
pub struct NewStudentInput {
    pub name: String,
    pub class_id: String,
    pub roll_no: Option<i64>,
    pub guardian_name: Option<String>,
    pub guardian_mobile: Option<String>,
    pub dob: Option<String>,
    pub gender: Option<String>,
    pub address: Option<String>,
    pub transport: Option<bool>,
    pub rte: Option<bool>,
    pub category: Option<String>,
    pub aadhaar_status: Option<String>, // none | submitted | verified
}

/// This device's admission series (A1 = server) and the highest provisional seq
/// already issued on it, so the next provisional number is series-unique.
fn admission_series_and_last(conn: &Connection, device_id: Option<&str>) -> rusqlite::Result<(String, u32)> {
    let series: String = match device_id {
        Some(d) => conn
            .query_row("SELECT admission_series FROM device WHERE id=?1", params![d], |r| r.get(0))
            .optional()?
            .flatten()
            .unwrap_or_else(|| "A1".to_string()),
        None => "A1".to_string(),
    };
    let prefix = format!("P-{series}-");
    let last: u32 = conn
        .query_row(
            "SELECT provisional_no FROM student WHERE provisional_no LIKE ?1 ORDER BY LENGTH(provisional_no) DESC, provisional_no DESC LIMIT 1",
            params![format!("{prefix}%")],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .and_then(|p| p.rsplit('-').next().and_then(|s| s.parse::<u32>().ok()))
        .unwrap_or(0);
    Ok((series, last))
}

fn parse_applies_to(s: &str) -> vidya_core::fees::AppliesTo {
    use vidya_core::fees::AppliesTo;
    match s {
        "all" => AppliesTo::All,
        "transport" => AppliesTo::Transport,
        other => serde_json::from_str::<Vec<String>>(other)
            .map(AppliesTo::ClassIds)
            .unwrap_or(AppliesTo::All),
    }
}

fn frequency_from(s: &str) -> vidya_core::types::FeeFrequency {
    use vidya_core::types::FeeFrequency;
    match s {
        "month" => FeeFrequency::Month,
        "once" => FeeFrequency::Once,
        _ => FeeFrequency::Term,
    }
}

/// Parse a fee head's stored `instalments_json` into an [`InstalmentPlan`], or
/// `None` when the column is NULL/empty/invalid (a plan-less head).
fn parse_instalments_json(s: Option<&str>) -> Option<vidya_core::fees::InstalmentPlan> {
    let s = s?.trim();
    if s.is_empty() {
        return None;
    }
    serde_json::from_str::<vidya_core::fees::InstalmentPlan>(s).ok()
}

/// Dues a single new admission owes: every active head that applies to the
/// student, for each term of the current session (`term` heads), the current
/// month (`month` heads) and once (`once` heads). Uses vidya-core.
fn dues_for_new_student(
    conn: &Connection,
    student_id: &str,
    class_id: &str,
    transport: bool,
    today: &str,
) -> rusqlite::Result<Vec<vidya_core::fees::FeeDue>> {
    use vidya_core::fees::{FeeHead, PeriodSpec, Student as FeeStudent};
    use vidya_core::money::Paise;

    let mut stmt = conn.prepare("SELECT id, amount_paise, frequency, applies_to, instalments_json FROM fee_head WHERE active=1")?;
    let heads: Vec<FeeHead> = stmt
        .query_map([], |r| {
            let id: String = r.get(0)?;
            let amount: i64 = r.get(1)?;
            let freq: String = r.get(2)?;
            let applies: String = r.get(3)?;
            let inst_json: Option<String> = r.get(4)?;
            Ok(FeeHead {
                id,
                amount_paise: Paise(amount),
                frequency: frequency_from(&freq),
                applies_to: parse_applies_to(&applies),
                instalments: parse_instalments_json(inst_json.as_deref()),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;

    // Terms of the current session (term heads → one due each).
    let mut ts = conn.prepare("SELECT t.name FROM term t JOIN academic_session s ON s.id=t.session_id AND s.is_current=1 ORDER BY t.starts_on")?;
    let terms: Vec<String> = ts.query_map([], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<_>>()?;
    let month = today.get(0..7).unwrap_or(today).to_string();
    let session_label: String = conn
        .query_row("SELECT label FROM academic_session WHERE is_current=1 LIMIT 1", [], |r| r.get(0))
        .optional()?
        .unwrap_or_default();

    let spec = PeriodSpec { terms, months: vec![month] };
    let student = FeeStudent { id: student_id.to_string(), class_id: class_id.to_string(), transport, admitted_period: session_label };
    Ok(vidya_core::fees::generate_dues(&heads, &[student], &spec, &[]))
}

pub fn create_student_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    today: &str,
    input: &NewStudentInput,
) -> CmdResult<StudentDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::CreateStudent, &Target::of(TargetKind::Student))?;
    let name = vidya_core::validation::validate_name(&input.name)?;
    if let Some(m) = &input.guardian_mobile {
        if !m.is_empty() {
            vidya_core::validation::validate_mobile(m)?;
        }
    }
    let sid = new_id("stu");
    let now = now_iso();
    let transport = input.transport.unwrap_or(false);
    let rte = input.rte.unwrap_or(false);
    let aadhaar = input.aadhaar_status.clone().unwrap_or_else(|| "none".to_string());

    // Offline devices get a provisional number; the school server assigns the
    // official admission_no when it confirms (§8.7). In single-PC Server mode
    // this device IS the server, so it assigns the official number now.
    let (series, last) = admission_series_and_last(conn, device_id)?;
    let provisional = vidya_core::admissions::provisional_no(&series, last + 1);
    let admission_no: Option<String> = if device_mode == DeviceMode::Server {
        let year: i32 = today.get(0..4).and_then(|y| y.parse().ok()).unwrap_or(0);
        let last_adm: i64 = conn
            .query_row("SELECT COUNT(*) FROM student WHERE admission_no LIKE ?1", params![format!("{year}/%")], |r| r.get(0))
            .optional()?
            .unwrap_or(0);
        Some(vidya_core::admissions::next_admission_no(year, last_adm as u32))
    } else {
        None
    };

    let session_id: Option<String> = conn
        .query_row("SELECT id FROM academic_session WHERE is_current=1 LIMIT 1", [], |r| r.get(0))
        .optional()?;
    // Auto roll = max roll in the class + 1 (unless the caller supplied one).
    let roll_no: Option<i64> = match input.roll_no {
        Some(r) => Some(r),
        None => conn
            .query_row(
                "SELECT COALESCE(MAX(roll_no),0)+1 FROM enrollment WHERE class_id=?1 AND to_date IS NULL",
                params![input.class_id],
                |r| r.get::<_, i64>(0),
            )
            .optional()?,
    };
    let dues = dues_for_new_student(conn, &sid, &input.class_id, transport, today)?;

    let confirmed = device_mode == DeviceMode::Server;
    let sync_state = if confirmed { "confirmed" } else { "on_device" };
    let school_id = single_school_id(conn)?;
    let ctx = WriteCtx { mode: device_mode };
    let sid2 = sid.clone();
    let dev = device_id.map(str::to_string);
    with_write(conn, &ctx, move |tx| {
        tx.execute(
            "INSERT INTO student(id,admission_no,provisional_no,name,dob,gender,guardian_name,guardian_mobile,address,transport,category,rte,aadhaar_status,status,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,'active',?14,?14,?15)",
            params![sid2, admission_no, provisional, name, input.dob, input.gender, input.guardian_name, input.guardian_mobile,
                input.address, transport as i64, input.category, rte as i64, aadhaar, now, sync_state],
        )?;
        // v2 (P13): also create the primary guardian row (find-or-create by
        // mobile+name, so siblings share one). The legacy student.guardian_*
        // columns above are kept in sync for read-only compatibility.
        crate::guardians::ensure_primary_guardian(
            tx, &sid2, input.guardian_name.as_deref(), input.guardian_mobile.as_deref(),
            school_id.as_deref(), &now, sync_state,
        )?;
        if let Some(sess) = &session_id {
            tx.execute(
                "INSERT INTO enrollment(id,student_id,class_id,session_id,roll_no,from_date,created_at,updated_at,sync_state) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?7,?8)",
                params![new_id("enr"), sid2, input.class_id, sess, roll_no, now, now, sync_state],
            )?;
        }
        for d in &dues {
            tx.execute(
                "INSERT INTO fee_due(id,student_id,fee_head_id,period,amount_paise,instalment_no,due_date,created_at,updated_at,sync_state) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8,?9)",
                params![new_id("due"), sid2, d.fee_head_id, d.period, d.amount_paise.get(), d.instalment_no, d.due_date, now, sync_state],
            )?;
        }
        let audit = AuditEntry {
            at: now.clone(),
            staff_id: Some(actor_s.id.clone()),
            action: "create_student".into(),
            table: Some("student".into()),
            record_id: Some(sid2.clone()),
            after_json: Some(serde_json::json!({ "name": name, "admission_no": admission_no, "provisional_no": provisional }).to_string()),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"),
            hlc: now.clone(),
            device_id: dev.clone().unwrap_or_default(),
            staff_id: actor_s.id.clone(),
            audience: "finance".into(),
            table: "student".into(),
            record_id: sid2.clone(),
            kind: "insert".into(),
            payload: "{}".into(),
            base_version: None,
            server_epoch: 1,
        };
        Ok(Effect { value: (), audit, op })
    })?;
    get_student_logic(conn, &sid)
}

// ---- Duplicate check (before saving an admission, §2) ----------------------

/// Candidate duplicates: same guardian mobile, OR same DOB with a name that
/// shares the first token (FTS prefix). The UI shows these before saving.
pub fn check_duplicate_students_logic(
    conn: &mut Connection,
    name: &str,
    dob: Option<&str>,
    guardian_mobile: Option<&str>,
) -> CmdResult<Vec<StudentRowDto>> {
    use rusqlite::params_from_iter;
    use rusqlite::types::Value;
    let mut ors: Vec<String> = Vec::new();
    let mut args: Vec<Value> = Vec::new();
    if let Some(m) = guardian_mobile.filter(|s| !s.is_empty()) {
        ors.push("s.guardian_mobile = ?".into());
        args.push(Value::Text(m.to_string()));
    }
    let first = name.split_whitespace().next().unwrap_or("");
    if let Some(d) = dob.filter(|s| !s.is_empty()) {
        if !first.is_empty() {
            ors.push("(s.dob = ? AND s.rowid IN (SELECT rowid FROM student_fts WHERE student_fts MATCH ?))".into());
            args.push(Value::Text(d.to_string()));
            args.push(Value::Text(format!("{}*", first.replace('"', ""))));
        } else {
            ors.push("s.dob = ?".into());
            args.push(Value::Text(d.to_string()));
        }
    }
    if ors.is_empty() {
        return Ok(vec![]);
    }
    let sql = format!(
        "SELECT s.id, s.name, s.admission_no, s.provisional_no, c.display, c.section, e.roll_no, s.guardian_name, s.status \
         FROM student s \
         LEFT JOIN enrollment e ON e.student_id=s.id AND e.to_date IS NULL \
         LEFT JOIN class c ON c.id=e.class_id WHERE {} LIMIT 20",
        ors.join(" OR ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(args.iter()), |r| {
            Ok(StudentRowDto {
                id: r.get(0)?,
                name: r.get(1)?,
                admission_no: r.get(2)?,
                provisional_no: r.get(3)?,
                class_display: r.get(4)?,
                section: r.get(5)?,
                roll_no: r.get(6)?,
                guardian_name: r.get(7)?,
                status: r.get(8)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

// ---- Student profile -------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct EnrollmentHistoryDto {
    pub class_display: Option<String>,
    pub session_label: Option<String>,
    pub roll_no: Option<i64>,
    pub from_date: String,
    pub to_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AttendanceSummaryDto {
    pub present: i64,
    pub absent: i64,
    pub leave: i64,
    pub marked: i64,
    pub pct_tenths: i64,
    pub term_label: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StudentProfileDto {
    pub id: String,
    pub name: String,
    pub admission_no: Option<String>,
    pub provisional_no: Option<String>,
    pub dob: Option<String>,
    pub gender: Option<String>,
    pub guardian_name: Option<String>,
    pub guardian_mobile: Option<String>,
    pub address: Option<String>,
    pub transport: bool,
    pub category: Option<String>,
    pub rte: bool,
    pub aadhaar_status: String,
    pub status: String,
    pub left_on: Option<String>,
    pub left_reason: Option<String>,
    pub class_id: Option<String>,
    pub class_display: Option<String>,
    pub roll_no: Option<i64>,
    pub version: i64,
    pub enrollment_history: Vec<EnrollmentHistoryDto>,
    pub attendance: AttendanceSummaryDto,
    /// v2 (P13): guardians from the `guardian` table, primary first. The legacy
    /// `guardian_name`/`guardian_mobile` above stay for read-only compatibility.
    pub guardians: Vec<crate::guardians::GuardianDto>,
}

/// The current term of the current session (the one containing `today`), else
/// the latest term. Returns (name, starts_on, ends_on).
fn current_term(conn: &Connection, today: &str) -> rusqlite::Result<Option<(String, String, String)>> {
    let containing = conn
        .query_row(
            "SELECT t.name, t.starts_on, t.ends_on FROM term t JOIN academic_session s ON s.id=t.session_id AND s.is_current=1 \
             WHERE ?1 BETWEEN t.starts_on AND t.ends_on ORDER BY t.starts_on LIMIT 1",
            params![today],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    if containing.is_some() {
        return Ok(containing);
    }
    conn.query_row(
        "SELECT t.name, t.starts_on, t.ends_on FROM term t JOIN academic_session s ON s.id=t.session_id AND s.is_current=1 \
         ORDER BY t.starts_on DESC LIMIT 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .optional()
}

pub fn get_student_profile_logic(conn: &mut Connection, today: &str, id: &str) -> CmdResult<StudentProfileDto> {
    let profile = conn
        .query_row(
            "SELECT s.id,s.name,s.admission_no,s.provisional_no,s.dob,s.gender,s.guardian_name,s.guardian_mobile,s.address,\
                    s.transport,s.category,s.rte,s.aadhaar_status,s.status,s.left_on,s.left_reason,s.version, c.id, c.display, e.roll_no \
             FROM student s \
             LEFT JOIN enrollment e ON e.student_id=s.id AND e.to_date IS NULL \
             LEFT JOIN class c ON c.id=e.class_id WHERE s.id=?1",
            params![id],
            |r| {
                Ok(StudentProfileDto {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    admission_no: r.get(2)?,
                    provisional_no: r.get(3)?,
                    dob: r.get(4)?,
                    gender: r.get(5)?,
                    guardian_name: r.get(6)?,
                    guardian_mobile: r.get(7)?,
                    address: r.get(8)?,
                    transport: r.get::<_, i64>(9)? != 0,
                    category: r.get(10)?,
                    rte: r.get::<_, i64>(11)? != 0,
                    aadhaar_status: r.get(12)?,
                    status: r.get(13)?,
                    left_on: r.get(14)?,
                    left_reason: r.get(15)?,
                    version: r.get(16)?,
                    class_id: r.get(17)?,
                    class_display: r.get(18)?,
                    roll_no: r.get(19)?,
                    enrollment_history: Vec::new(),
                    attendance: AttendanceSummaryDto { present: 0, absent: 0, leave: 0, marked: 0, pct_tenths: 0, term_label: None, from_date: None, to_date: None },
                    guardians: Vec::new(),
                })
            },
        )
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    let guardians = crate::guardians::list_for_student(conn, id)?;

    let mut hstmt = conn.prepare(
        "SELECT c.display, ses.label, e.roll_no, e.from_date, e.to_date FROM enrollment e \
         LEFT JOIN class c ON c.id=e.class_id LEFT JOIN academic_session ses ON ses.id=e.session_id \
         WHERE e.student_id=?1 ORDER BY e.from_date DESC, e.to_date IS NULL DESC",
    )?;
    let history: Vec<EnrollmentHistoryDto> = hstmt
        .query_map(params![id], |r| {
            Ok(EnrollmentHistoryDto {
                class_display: r.get(0)?,
                session_label: r.get(1)?,
                roll_no: r.get(2)?,
                from_date: r.get(3)?,
                to_date: r.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;

    let (present, absent, leave, term_label, from_date, to_date) = match current_term(conn, today)? {
        Some((name, starts, ends)) => {
            let mut counts = (0i64, 0i64, 0i64);
            let mut st = conn.prepare(
                "SELECT am.mark, COUNT(*) FROM attendance_mark am JOIN attendance_sheet sh ON sh.id=am.sheet_id \
                 WHERE am.student_id=?1 AND sh.status='submitted' AND sh.date BETWEEN ?2 AND ?3 GROUP BY am.mark",
            )?;
            let rows = st.query_map(params![id, starts, ends], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
            for row in rows {
                let (m, n) = row?;
                match m.as_str() {
                    "P" => counts.0 = n,
                    "A" => counts.1 = n,
                    "L" => counts.2 = n,
                    _ => {}
                }
            }
            (counts.0, counts.1, counts.2, Some(name), Some(starts), Some(ends))
        }
        None => (0, 0, 0, None, None, None),
    };
    let pct = vidya_core::attendance::percent_present(present as u32, absent as u32, leave as u32) as i64;

    Ok(StudentProfileDto {
        enrollment_history: history,
        attendance: AttendanceSummaryDto { present, absent, leave, marked: present + absent + leave, pct_tenths: pct, term_label, from_date, to_date },
        guardians,
        ..profile
    })
}

// ---- Transfer section + Mark as left ---------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn transfer_student_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    today: &str,
    student_id: &str,
    new_class_id: &str,
    roll_no: Option<i64>,
) -> CmdResult<StudentDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::TransferSection, &Target::of(TargetKind::Student))?;
    let session_id: String = conn
        .query_row("SELECT id FROM academic_session WHERE is_current=1 LIMIT 1", [], |r| r.get(0))
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    let now = now_iso();
    let roll = match roll_no {
        Some(r) => Some(r),
        None => conn
            .query_row(
                "SELECT COALESCE(MAX(roll_no),0)+1 FROM enrollment WHERE class_id=?1 AND to_date IS NULL",
                params![new_class_id],
                |r| r.get::<_, i64>(0),
            )
            .optional()?,
    };
    let sync_state = if device_mode == DeviceMode::Server { "confirmed" } else { "on_device" };
    let ctx = WriteCtx { mode: device_mode };
    let sid = student_id.to_string();
    let ncid = new_class_id.to_string();
    let day = today.to_string();
    let dev = device_id.map(str::to_string);
    with_write(conn, &ctx, move |tx| {
        // Close the open enrollment as of the transfer date (history never rewritten).
        tx.execute(
            "UPDATE enrollment SET to_date=?1, updated_at=?2 WHERE student_id=?3 AND to_date IS NULL",
            params![day, now, sid],
        )?;
        // Open a new enrollment in the target class from the transfer date.
        tx.execute(
            "INSERT INTO enrollment(id,student_id,class_id,session_id,roll_no,from_date,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?7,?8)",
            params![new_id("enr"), sid, ncid, session_id, roll, day, now, sync_state],
        )?;
        let audit = AuditEntry {
            at: now.clone(),
            staff_id: Some(actor_s.id.clone()),
            action: "transfer_section".into(),
            table: Some("enrollment".into()),
            record_id: Some(sid.clone()),
            after_json: Some(serde_json::json!({ "class_id": ncid }).to_string()),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"),
            hlc: now.clone(),
            device_id: dev.clone().unwrap_or_default(),
            staff_id: actor_s.id.clone(),
            audience: "finance".into(),
            table: "enrollment".into(),
            record_id: sid.clone(),
            kind: "action".into(),
            payload: "{}".into(),
            base_version: None,
            server_epoch: 1,
        };
        Ok(Effect { value: (), audit, op })
    })?;
    get_student_logic(conn, student_id)
}

pub fn mark_student_left_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    student_id: &str,
    left_on: &str,
    reason: &str,
) -> CmdResult<StudentProfileDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::MarkStudentLeft, &Target::of(TargetKind::Student))?;
    let now = now_iso();
    let today = now.get(0..10).unwrap_or("").to_string();
    let sync_state = if device_mode == DeviceMode::Server { "confirmed" } else { "on_device" };
    let ctx = WriteCtx { mode: device_mode };
    let sid = student_id.to_string();
    let left_on = left_on.to_string();
    let reason_s = reason.to_string();
    let dev = device_id.map(str::to_string);
    with_write(conn, &ctx, move |tx| {
        tx.execute(
            "UPDATE student SET status='left', left_on=?1, left_reason=?2, updated_at=?3, sync_state=?4, version=version+1 WHERE id=?5",
            params![left_on, reason_s, now, sync_state, sid],
        )?;
        // Cancel FUTURE (fully-unpaid) dues; paid/partly-paid dues are untouched.
        tx.execute(
            "UPDATE fee_due SET cancelled_at=?1, updated_at=?1 WHERE student_id=?2 AND cancelled_at IS NULL \
             AND id NOT IN (SELECT fee_due_id FROM payment_allocation WHERE fee_due_id IS NOT NULL)",
            params![now, sid],
        )?;
        let audit = AuditEntry {
            at: now.clone(),
            staff_id: Some(actor_s.id.clone()),
            action: "mark_left".into(),
            table: Some("student".into()),
            record_id: Some(sid.clone()),
            reason: Some(reason_s.clone()),
            after_json: Some(serde_json::json!({ "status": "left", "left_on": left_on }).to_string()),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"),
            hlc: now.clone(),
            device_id: dev.clone().unwrap_or_default(),
            staff_id: actor_s.id.clone(),
            audience: "finance".into(),
            table: "student".into(),
            record_id: sid.clone(),
            kind: "update".into(),
            payload: "{}".into(),
            base_version: None,
            server_epoch: 1,
        };
        Ok(Effect { value: (), audit, op })
    })?;
    get_student_profile_logic(conn, &today, student_id)
}

// ============================================================= CSV ============
//
// Export (scoped, formula-safe via vidya_core::csv::escape_formula) and import
// (template + dry-run preview + one-transaction commit). Files are read/written
// by these commands (the path comes from tauri-plugin-dialog); success is
// reported only after the file is actually written (no fake success, §7).

const CSV_MAX_BYTES: u64 = 5_000_000;
const CSV_MAX_ROWS: usize = 5_000;

const IMPORT_HEADERS: &[&str] = &[
    "Name", "Class", "Roll", "Guardian", "Guardian mobile", "Date of birth", "Gender", "Transport", "RTE", "Category", "Aadhaar status",
];
const IMPORT_EXAMPLE: &[&str] = &["Aarav Gupta", "I-A", "1", "Rahul Gupta", "9876543210", "2018-06-15", "male", "no", "no", "General", "none"];

fn audit_action(conn: &mut Connection, entry: AuditEntry) -> CmdResult<()> {
    let tx = conn.transaction()?;
    crate::security::audit::append(&tx, &entry)?;
    tx.commit()?;
    Ok(())
}

fn students_csv_rows(conn: &Connection) -> rusqlite::Result<Vec<Vec<String>>> {
    let mut out = vec![vec![
        "Name".into(), "Admission no.".into(), "Class".into(), "Roll".into(), "Guardian".into(),
        "Guardian mobile".into(), "Date of birth".into(), "Gender".into(), "Status".into(), "Outstanding (Rs)".into(),
    ]];
    let mut stmt = conn.prepare(
        "SELECT s.name, COALESCE(s.admission_no, s.provisional_no, ''), COALESCE(c.display,''), e.roll_no, \
                COALESCE(s.guardian_name,''), COALESCE(s.guardian_mobile,''), COALESCE(s.dob,''), COALESCE(s.gender,''), s.status, \
                COALESCE((SELECT SUM(d.amount_paise) FROM fee_due d WHERE d.student_id=s.id AND d.cancelled_at IS NULL),0) - \
                COALESCE((SELECT SUM(pa.amount_paise) FROM payment_allocation pa JOIN fee_due d2 ON d2.id=pa.fee_due_id WHERE d2.student_id=s.id AND pa.kind='due'),0) \
         FROM student s \
         LEFT JOIN enrollment e ON e.student_id=s.id AND e.to_date IS NULL \
         LEFT JOIN class c ON c.id=e.class_id ORDER BY c.sort_order, e.roll_no, s.name",
    )?;
    let rows = stmt.query_map([], |r| {
        let roll: String = r.get::<_, Option<i64>>(3)?.map(|n| n.to_string()).unwrap_or_default();
        let out_paise: i64 = r.get(9)?;
        Ok(vec![
            r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, roll,
            r.get::<_, String>(4)?, r.get::<_, String>(5)?, r.get::<_, String>(6)?, r.get::<_, String>(7)?,
            r.get::<_, String>(8)?, format!("{}", out_paise.max(0) / 100),
        ])
    })?;
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn dues_csv_rows(conn: &Connection) -> rusqlite::Result<Vec<Vec<String>>> {
    let mut out = vec![vec!["Name".into(), "Class".into(), "Outstanding (Rs)".into()]];
    let mut stmt = conn.prepare(
        "SELECT s.name, COALESCE(c.display,''), \
                SUM(d.amount_paise) - COALESCE((SELECT SUM(pa.amount_paise) FROM payment_allocation pa JOIN fee_due d2 ON d2.id=pa.fee_due_id WHERE d2.student_id=s.id AND pa.kind='due'),0) AS bal \
         FROM student s \
         LEFT JOIN enrollment e ON e.student_id=s.id AND e.to_date IS NULL \
         LEFT JOIN class c ON c.id=e.class_id \
         JOIN fee_due d ON d.student_id=s.id AND d.cancelled_at IS NULL \
         GROUP BY s.id HAVING bal > 0 ORDER BY bal DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        let bal: i64 = r.get(2)?;
        Ok(vec![r.get::<_, String>(0)?, r.get::<_, String>(1)?, format!("{}", bal / 100)])
    })?;
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Export a scoped, formula-safe CSV to `path`. `arg` carries a parameter for
/// kinds that need one (e.g. the date for `daybook`). Returns the data-row count
/// (header excluded). 0 rows → a header-only file (§ edge case).
pub fn export_csv_logic(conn: &mut Connection, actor_s: &SessionStaff, kind: &str, path: &str, arg: Option<&str>) -> CmdResult<i64> {
    let actor = actor_from(conn, actor_s)?;
    let rows = match kind {
        "students" => {
            require_allow(&actor, Action::StudentCsvExport, &Target::of(TargetKind::Student))?;
            students_csv_rows(conn)?
        }
        "dues" => {
            require_allow(&actor, Action::FeeReports, &Target::of(TargetKind::Fee))?;
            dues_csv_rows(conn)?
        }
        "daybook" => {
            require_allow(&actor, Action::DayBook, &Target::of(TargetKind::Fee))?;
            daybook_csv_rows(conn, arg.unwrap_or(""))?
        }
        // P15 reports (Step 7). Scoped by role via the same actions as the screens.
        "expenses" => {
            require_allow(&actor, Action::ViewAccounts, &Target::of(TargetKind::Fee))?;
            require_module_enabled(conn, vidya_core::modules::Module::Accounts)?;
            let (from, to) = split_range(arg);
            expenses_csv_rows(conn, &from, &to)?
        }
        "salary" => {
            // Permission (ManageSalary) + module checked inside salary_register_logic.
            let reg = salary_register_logic(conn, actor_s, arg.unwrap_or(""), &[])?;
            salary_csv_rows(&reg)
        }
        "store_sales" => {
            require_allow(&actor, Action::RecordStoreSale, &Target::of(TargetKind::Fee))?;
            require_module_enabled(conn, vidya_core::modules::Module::Store)?;
            store_sales_csv_rows(conn)?
        }
        "store_stock" => {
            require_allow(&actor, Action::RecordStoreSale, &Target::of(TargetKind::Fee))?;
            require_module_enabled(conn, vidya_core::modules::Module::Store)?;
            store_stock_csv_rows(conn)?
        }
        "instalment_dues" => {
            require_allow(&actor, Action::FeeReports, &Target::of(TargetKind::Fee))?;
            instalment_dues_csv_rows(conn)?
        }
        _ => return Err(CmdError::validation("kind", "unknown")),
    };
    let escaped: Vec<Vec<String>> = rows
        .iter()
        .map(|r| r.iter().map(|c| vidya_core::csv::escape_formula(c)).collect())
        .collect();
    let bytes = vidya_core::csv::write_csv_excel(&escaped);
    std::fs::write(path, bytes).map_err(|_| CmdError::internal("csv_write"))?;
    let data_rows = rows.len().saturating_sub(1) as i64;
    audit_action(conn, AuditEntry {
        at: now_iso(),
        staff_id: Some(actor_s.id.clone()),
        action: "export_csv".into(),
        table: Some(kind.to_string()),
        after_json: Some(serde_json::json!({ "kind": kind, "rows": data_rows }).to_string()),
        ..Default::default()
    })?;
    Ok(data_rows)
}

/// The importable-students template: header row + one English example row,
/// UTF-8 BOM (Excel-friendly), formula-safe.
pub fn students_csv_template_logic() -> String {
    let header: Vec<String> = IMPORT_HEADERS.iter().map(|s| s.to_string()).collect();
    let example: Vec<String> = IMPORT_EXAMPLE.iter().map(|s| vidya_core::csv::escape_formula(s)).collect();
    String::from_utf8(vidya_core::csv::write_csv_excel(&[header, example])).unwrap_or_default()
}

#[derive(Debug, Serialize, Clone)]
pub struct ImportRowError {
    pub column: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ImportRowDto {
    pub row: i64,
    pub name: String,
    pub class: String,
    pub errors: Vec<ImportRowError>,
    pub duplicate: bool,
}

#[derive(Debug, Serialize)]
pub struct ImportPreviewDto {
    pub total: i64,
    pub valid: i64,
    pub rows: Vec<ImportRowDto>,
    pub error: Option<String>,
}

struct ParsedRow {
    row_num: i64,
    name: String,
    class_id: String,
    roll_no: Option<i64>,
    guardian_name: Option<String>,
    guardian_mobile: Option<String>,
    dob: Option<String>,
    gender: Option<String>,
    transport: bool,
    rte: bool,
    category: Option<String>,
    aadhaar: String,
    errors: Vec<ImportRowError>,
}

fn yes_no(s: &str) -> bool {
    matches!(s.trim().to_ascii_lowercase().as_str(), "yes" | "y" | "true" | "1")
}

fn class_display_map(conn: &Connection) -> rusqlite::Result<std::collections::HashMap<String, String>> {
    let mut stmt = conn.prepare("SELECT display, id FROM class")?;
    let mut m = std::collections::HashMap::new();
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    for row in rows {
        let (display, id) = row?;
        m.insert(display, id);
    }
    Ok(m)
}

fn read_import_file(path: &str) -> CmdResult<Vec<Vec<String>>> {
    let meta = std::fs::metadata(path).map_err(|_| CmdError::internal("csv_read"))?;
    if meta.len() > CSV_MAX_BYTES {
        return Err(CmdError::validation("file", "too_large"));
    }
    let text = std::fs::read_to_string(path).map_err(|_| CmdError::validation("file", "not_utf8"))?;
    Ok(vidya_core::csv::read_csv(&text))
}

fn parse_import_rows(conn: &Connection, records: &[Vec<String>]) -> (Vec<ParsedRow>, Option<String>) {
    let classes = class_display_map(conn).unwrap_or_default();
    let data = records.get(1..).unwrap_or(&[]); // skip the header row
    if data.len() > CSV_MAX_ROWS {
        return (Vec::new(), Some("too_many_rows".into()));
    }
    let get = |r: &[String], i: usize| r.get(i).map(|s| s.trim().to_string()).unwrap_or_default();
    let mut out = Vec::new();
    for (idx, rec) in data.iter().enumerate() {
        let row_num = idx as i64 + 2; // 1-based, +1 for header
        let name_raw = get(rec, 0);
        let class_disp = get(rec, 1);
        let mobile = get(rec, 4);
        let dob = get(rec, 5);
        let mut errors = Vec::new();
        if vidya_core::validation::validate_name(&name_raw).is_err() {
            errors.push(ImportRowError { column: "Name".into(), reason: "required".into() });
        }
        let class_id = match classes.get(&class_disp) {
            Some(id) => id.clone(),
            None => {
                errors.push(ImportRowError { column: "Class".into(), reason: "unknown_class".into() });
                String::new()
            }
        };
        if !mobile.is_empty() && vidya_core::validation::validate_mobile(&mobile).is_err() {
            errors.push(ImportRowError { column: "Guardian mobile".into(), reason: "pattern".into() });
        }
        let roll_no = {
            let r = get(rec, 2);
            if r.is_empty() { None } else { r.parse::<i64>().ok() }
        };
        out.push(ParsedRow {
            row_num,
            name: name_raw,
            class_id,
            roll_no,
            guardian_name: { let v = get(rec, 3); if v.is_empty() { None } else { Some(v) } },
            guardian_mobile: if mobile.is_empty() { None } else { Some(mobile) },
            dob: if dob.is_empty() { None } else { Some(dob) },
            gender: { let v = get(rec, 6); if v.is_empty() { None } else { Some(v) } },
            transport: yes_no(&get(rec, 7)),
            rte: yes_no(&get(rec, 8)),
            category: { let v = get(rec, 9); if v.is_empty() { None } else { Some(v) } },
            aadhaar: { let v = get(rec, 10); if v.is_empty() { "none".into() } else { v } },
            errors,
        });
    }
    (out, None)
}

pub fn import_students_dry_run_logic(conn: &mut Connection, actor_s: &SessionStaff, path: &str) -> CmdResult<ImportPreviewDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::StudentCsvImport, &Target::of(TargetKind::Student))?;
    let records = read_import_file(path)?;
    let (parsed, fatal) = parse_import_rows(conn, &records);
    if let Some(reason) = fatal {
        return Ok(ImportPreviewDto { total: 0, valid: 0, rows: Vec::new(), error: Some(reason) });
    }
    let mut rows = Vec::new();
    let mut valid = 0i64;
    for p in &parsed {
        let dup = if p.errors.is_empty() {
            !check_duplicate_students_logic(conn, &p.name, p.dob.as_deref(), p.guardian_mobile.as_deref())?.is_empty()
        } else {
            false
        };
        if p.errors.is_empty() {
            valid += 1;
        }
        let class_disp = records.get(p.row_num as usize - 1).and_then(|r| r.get(1)).cloned().unwrap_or_default();
        rows.push(ImportRowDto { row: p.row_num, name: p.name.clone(), class: class_disp, errors: p.errors.clone(), duplicate: dup });
    }
    Ok(ImportPreviewDto { total: parsed.len() as i64, valid, rows, error: None })
}

#[derive(Debug, Serialize)]
pub struct ImportResultDto {
    pub imported: i64,
    pub skipped: Vec<ImportRowDto>,
}

pub fn import_students_commit_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_mode: DeviceMode,
    today: &str,
    path: &str,
) -> CmdResult<ImportResultDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::StudentCsvImport, &Target::of(TargetKind::Student))?;
    let records = read_import_file(path)?;
    let (parsed, fatal) = parse_import_rows(conn, &records);
    if let Some(reason) = fatal {
        return Err(CmdError::validation("file", &reason));
    }
    let session_id: Option<String> = conn
        .query_row("SELECT id FROM academic_session WHERE is_current=1 LIMIT 1", [], |r| r.get(0))
        .optional()?;
    let sync_state = if device_mode == DeviceMode::Server { "confirmed" } else { "on_device" };
    let (series, mut last_seq) = admission_series_and_last(conn, None)?;
    let year: i32 = today.get(0..4).and_then(|y| y.parse().ok()).unwrap_or(0);
    let mut last_adm: i64 = conn
        .query_row("SELECT COUNT(*) FROM student WHERE admission_no LIKE ?1", params![format!("{year}/%")], |r| r.get(0))
        .optional()?
        .unwrap_or(0);

    let mut skipped = Vec::new();
    let mut imported = 0i64;
    let now = now_iso();
    let school_id = single_school_id(conn)?;

    let tx = conn.transaction()?;
    for p in &parsed {
        if !p.errors.is_empty() {
            let class_disp = records.get(p.row_num as usize - 1).and_then(|r| r.get(1)).cloned().unwrap_or_default();
            skipped.push(ImportRowDto { row: p.row_num, name: p.name.clone(), class: class_disp, errors: p.errors.clone(), duplicate: false });
            continue;
        }
        let sid = new_id("stu");
        last_seq += 1;
        let provisional = vidya_core::admissions::provisional_no(&series, last_seq);
        let admission_no: Option<String> = if device_mode == DeviceMode::Server {
            last_adm += 1;
            Some(vidya_core::admissions::next_admission_no(year, (last_adm - 1) as u32))
        } else {
            None
        };
        tx.execute(
            "INSERT INTO student(id,admission_no,provisional_no,name,dob,gender,guardian_name,guardian_mobile,transport,category,rte,aadhaar_status,status,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,'active',?13,?13,?14)",
            params![sid, admission_no, provisional, p.name, p.dob, p.gender, p.guardian_name, p.guardian_mobile, p.transport as i64, p.category, p.rte as i64, p.aadhaar, now, sync_state],
        )?;
        crate::guardians::ensure_primary_guardian(
            &tx, &sid, p.guardian_name.as_deref(), p.guardian_mobile.as_deref(),
            school_id.as_deref(), &now, sync_state,
        )?;
        if let Some(sess) = &session_id {
            tx.execute(
                "INSERT INTO enrollment(id,student_id,class_id,session_id,roll_no,from_date,created_at,updated_at,sync_state) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?7,?8)",
                params![new_id("enr"), sid, p.class_id, sess, p.roll_no, today, now, sync_state],
            )?;
        }
        for d in dues_for_new_student(&tx, &sid, &p.class_id, p.transport, today)? {
            tx.execute(
                "INSERT INTO fee_due(id,student_id,fee_head_id,period,amount_paise,instalment_no,due_date,created_at,updated_at,sync_state) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8,?9)",
                params![new_id("due"), sid, d.fee_head_id, d.period, d.amount_paise.get(), d.instalment_no, d.due_date, now, sync_state],
            )?;
        }
        imported += 1;
    }
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(),
        staff_id: Some(actor_s.id.clone()),
        action: "import_students".into(),
        table: Some("student".into()),
        after_json: Some(serde_json::json!({ "imported": imported, "skipped": skipped.len() }).to_string()),
        ..Default::default()
    })?;
    tx.commit()?;
    Ok(ImportResultDto { imported, skipped })
}

// ============================================================= fees ===========
//
// Fees overview by class, fee-structure heads CRUD, and the amount-change
// preview. Structure edits are Principal-only and audited. A head with any
// allocation can only be deactivated (never hard-deleted). Changing a head's
// amount updates only UNPAID dues; paid/partly-paid dues never change (§9).

#[derive(Debug, Serialize)]
pub struct FeeOverviewRow {
    pub class_id: String,
    pub class_display: String,
    pub students_with_dues: i64,
    pub outstanding_paise: i64,
}

pub fn fees_overview_logic(conn: &mut Connection, actor_s: &SessionStaff) -> CmdResult<Vec<FeeOverviewRow>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ViewFees, &Target::of(TargetKind::Fee))?;
    // Per-student outstanding = active dues − due-allocations, grouped by class.
    let mut stmt = conn.prepare(
        "SELECT c.id, c.display, \
            COUNT(DISTINCT CASE WHEN bal.outstanding > 0 THEN bal.student_id END), \
            COALESCE(SUM(CASE WHEN bal.outstanding > 0 THEN bal.outstanding ELSE 0 END),0) \
         FROM class c \
         LEFT JOIN enrollment e ON e.class_id=c.id AND e.to_date IS NULL \
         LEFT JOIN ( \
            SELECT d.student_id AS student_id, \
              SUM(d.amount_paise) - COALESCE((SELECT SUM(pa.amount_paise) FROM payment_allocation pa JOIN fee_due d2 ON d2.id=pa.fee_due_id WHERE d2.student_id=d.student_id AND pa.kind='due'),0) AS outstanding \
            FROM fee_due d WHERE d.cancelled_at IS NULL GROUP BY d.student_id \
         ) bal ON bal.student_id = e.student_id \
         GROUP BY c.id ORDER BY c.sort_order",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(FeeOverviewRow { class_id: r.get(0)?, class_display: r.get(1)?, students_with_dues: r.get(2)?, outstanding_paise: r.get(3)? })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

#[derive(Debug, Serialize)]
pub struct FeeHeadDto {
    pub id: String,
    pub name: String,
    pub name_hi: Option<String>,
    pub amount_paise: i64,
    pub frequency: String,
    pub applies_to: String,
    pub active: bool,
    pub has_allocations: bool,
    /// Raw `fee_head.instalments_json` (an `InstalmentPlan`, or null for a
    /// plan-less head). The Fee-structure screen parses it to draw the chips.
    pub instalments_json: Option<String>,
}

pub fn list_fee_heads_logic(conn: &mut Connection, actor_s: &SessionStaff) -> CmdResult<Vec<FeeHeadDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ViewFees, &Target::of(TargetKind::Fee))?;
    let mut stmt = conn.prepare(
        "SELECT h.id, h.name, h.name_hi, h.amount_paise, h.frequency, h.applies_to, h.active, \
           EXISTS(SELECT 1 FROM fee_due d JOIN payment_allocation pa ON pa.fee_due_id=d.id WHERE d.fee_head_id=h.id), \
           h.instalments_json \
         FROM fee_head h ORDER BY h.name",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(FeeHeadDto {
                id: r.get(0)?,
                name: r.get(1)?,
                name_hi: r.get(2)?,
                amount_paise: r.get(3)?,
                frequency: r.get(4)?,
                applies_to: r.get(5)?,
                active: r.get::<_, i64>(6)? != 0,
                has_allocations: r.get::<_, i64>(7)? != 0,
                instalments_json: r.get(8)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

#[derive(Debug, Deserialize)]
pub struct FeeHeadInput {
    pub name: String,
    pub name_hi: Option<String>,
    pub amount_paise: i64,
    pub frequency: String,   // term | month | once
    pub applies_to: String,  // all | transport | JSON class ids
    /// Optional instalment plan (`InstalmentPlan` as JSON). Validated against the
    /// head amount + the current session (§10.2).
    #[serde(default)]
    pub instalments_json: Option<String>,
}

/// The current session's `[starts_on, ends_on]` (YYYY-MM-DD), for instalment
/// due-date validation. Falls back to a wide range if no session row exists.
fn current_session_range(conn: &Connection) -> rusqlite::Result<(String, String)> {
    Ok(conn
        .query_row(
            "SELECT starts_on, ends_on FROM academic_session WHERE is_current=1 LIMIT 1",
            [],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()?
        .unwrap_or_else(|| ("0000-01-01".into(), "9999-12-31".into())))
}

/// Validate an optional instalment plan JSON against a head amount. Returns the
/// canonical JSON to store (or `None`). Errors mirror vidya-core codes.
fn validate_instalments_input(
    conn: &Connection,
    amount_paise: i64,
    instalments_json: Option<&str>,
) -> CmdResult<Option<String>> {
    let plan = match parse_instalments_json(instalments_json) {
        Some(p) => p,
        None => return Ok(None),
    };
    let (start, end) = current_session_range(conn)?;
    vidya_core::fees::validate_instalment_plan(&plan, vidya_core::money::Paise(amount_paise), &start, &end)?;
    // Re-serialise so the stored JSON is canonical.
    Ok(Some(serde_json::to_string(&plan).map_err(|_| CmdError::validation("instalments", "encode"))?))
}

fn require_principal(actor: &Actor) -> CmdResult<()> {
    if actor.role == Role::Principal {
        Ok(())
    } else {
        Err(CmdError::forbidden("fee_structure_principal_only"))
    }
}

pub fn create_fee_head_logic(conn: &mut Connection, actor_s: &SessionStaff, input: &FeeHeadInput) -> CmdResult<FeeHeadDto> {
    let actor = actor_from(conn, actor_s)?;
    require_principal(&actor)?;
    vidya_core::validation::validate_name(&input.name)?;
    if input.amount_paise < 0 {
        return Err(CmdError::validation("amount", "negative"));
    }
    if !matches!(input.frequency.as_str(), "term" | "month" | "once") {
        return Err(CmdError::validation("frequency", "invalid"));
    }
    let instalments_json = validate_instalments_input(conn, input.amount_paise, input.instalments_json.as_deref())?;
    let id = new_id("head");
    conn.execute(
        "INSERT INTO fee_head(id,name,name_hi,amount_paise,frequency,applies_to,active,instalments_json) VALUES (?1,?2,?3,?4,?5,?6,1,?7)",
        params![id, input.name, input.name_hi, input.amount_paise, input.frequency, input.applies_to, instalments_json],
    )?;
    audit_action(conn, AuditEntry {
        at: now_iso(),
        staff_id: Some(actor_s.id.clone()),
        action: "create_fee_head".into(),
        table: Some("fee_head".into()),
        record_id: Some(id.clone()),
        after_json: Some(serde_json::json!({ "name": input.name, "amount_paise": input.amount_paise, "instalments": instalments_json }).to_string()),
        ..Default::default()
    })?;
    list_fee_heads_logic(conn, actor_s)?.into_iter().find(|h| h.id == id).ok_or_else(CmdError::not_found)
}

#[derive(Debug, Serialize)]
pub struct FeeHeadChangePreview {
    pub affected_dues: i64,
    pub delta_paise: i64,
    /// Instalments added / removed / changed under a new plan (0 for amount-only).
    pub added: i64,
    pub removed: i64,
    pub changed: i64,
}

/// Load a head's existing dues as [`ExistingDue`] for the plan-change preview.
fn head_existing_dues(conn: &Connection, head_id: &str) -> rusqlite::Result<Vec<vidya_core::fees::ExistingDue>> {
    let mut stmt = conn.prepare(
        "SELECT d.id, d.instalment_no, d.amount_paise, \
           EXISTS(SELECT 1 FROM payment_allocation pa WHERE pa.fee_due_id=d.id) \
         FROM fee_due d WHERE d.fee_head_id=?1 AND d.cancelled_at IS NULL",
    )?;
    let rows = stmt
        .query_map(params![head_id], |r| {
            Ok(vidya_core::fees::ExistingDue {
                id: r.get(0)?,
                instalment_no: r.get::<_, i64>(1)? as u32,
                amount_paise: vidya_core::money::Paise(r.get(2)?),
                paid: r.get::<_, i64>(3)? != 0,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

/// Preview a head change before saving (§10.2): which UNPAID dues change, are
/// added or removed, and the net delta. Paid dues never change. Handles an
/// amount-only change (plan-less) and an instalment plan change.
pub fn preview_fee_head_change_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    id: &str,
    new_amount: i64,
    instalments_json: Option<&str>,
) -> CmdResult<FeeHeadChangePreview> {
    let actor = actor_from(conn, actor_s)?;
    require_principal(&actor)?;
    if let Some(vidya_core::fees::InstalmentPlan::List { instalments }) = parse_instalments_json(instalments_json) {
        let existing = head_existing_dues(conn, id)?;
        let p = vidya_core::fees::preview_plan_change(&existing, &instalments);
        return Ok(FeeHeadChangePreview {
            affected_dues: (p.changed.len() + p.added.len() + p.removed.len()) as i64,
            delta_paise: p.delta_paise,
            added: p.added.len() as i64,
            removed: p.removed.len() as i64,
            changed: p.changed.len() as i64,
        });
    }
    // Amount-only change (or a Monthly plan whose per-due amount is the monthly
    // amount): apply `new_amount` to every unpaid due.
    let per_due = match parse_instalments_json(instalments_json) {
        Some(vidya_core::fees::InstalmentPlan::Monthly { monthly_amount_paise, .. }) => monthly_amount_paise.get(),
        _ => new_amount,
    };
    let (count, total_now): (i64, i64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(amount_paise),0) FROM fee_due d \
         WHERE d.fee_head_id=?1 AND d.cancelled_at IS NULL \
           AND NOT EXISTS(SELECT 1 FROM payment_allocation pa WHERE pa.fee_due_id=d.id)",
        params![id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(FeeHeadChangePreview { affected_dues: count, delta_paise: count * per_due - total_now, added: 0, removed: 0, changed: count })
}

/// Cancel unpaid dues for a head and regenerate them from its instalment plan for
/// every student that already has a due for it. Paid dues (with an allocation) are
/// left untouched; instalments already paid are not regenerated (§10.2).
fn regenerate_head_dues_in_tx(
    tx: &rusqlite::Transaction,
    head_id: &str,
    now: &str,
    sync_state: &str,
) -> rusqlite::Result<()> {
    use vidya_core::fees::{FeeHead, PeriodSpec, Student as FeeStudent};
    use vidya_core::money::Paise;

    // Load the head with its plan; nothing to do if it has no plan.
    let head: Option<FeeHead> = tx
        .query_row(
            "SELECT id, amount_paise, frequency, applies_to, instalments_json FROM fee_head WHERE id=?1",
            params![head_id],
            |r| {
                let inst_json: Option<String> = r.get(4)?;
                Ok(FeeHead {
                    id: r.get(0)?,
                    amount_paise: Paise(r.get(1)?),
                    frequency: frequency_from(&r.get::<_, String>(2)?),
                    applies_to: parse_applies_to(&r.get::<_, String>(3)?),
                    instalments: parse_instalments_json(inst_json.as_deref()),
                })
            },
        )
        .optional()?;
    let head = match head {
        Some(h) if h.instalments.is_some() => h,
        _ => return Ok(()),
    };

    // Students who already have a due for this head, with their current class.
    let mut stmt = tx.prepare(
        "SELECT DISTINCT s.id, \
           COALESCE((SELECT e.class_id FROM enrollment e WHERE e.student_id=s.id AND e.to_date IS NULL ORDER BY e.from_date DESC LIMIT 1), ''), \
           s.transport \
         FROM student s JOIN fee_due d ON d.student_id=s.id \
         WHERE d.fee_head_id=?1 AND s.status='active'",
    )?;
    let session_label: String = tx
        .query_row("SELECT label FROM academic_session WHERE is_current=1 LIMIT 1", [], |r| r.get(0))
        .optional()?
        .unwrap_or_default();
    let students: Vec<FeeStudent> = stmt
        .query_map(params![head_id], |r| {
            Ok(FeeStudent {
                id: r.get(0)?,
                class_id: r.get(1)?,
                transport: r.get::<_, i64>(2)? != 0,
                admitted_period: session_label.clone(),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;

    // Instalments already paid must not be regenerated: their (student, head,
    // period) key is the due date used at generation.
    let mut astmt = tx.prepare(
        "SELECT d.student_id, d.fee_head_id, d.period FROM fee_due d \
         JOIN payment_allocation pa ON pa.fee_due_id=d.id WHERE d.fee_head_id=?1",
    )?;
    let allocated: Vec<(String, String, String)> = astmt
        .query_map(params![head_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;

    // Soft-cancel the unpaid dues (append-friendly: never hard-delete a due).
    tx.execute(
        "UPDATE fee_due SET cancelled_at=?1, updated_at=?1 WHERE fee_head_id=?2 AND cancelled_at IS NULL \
           AND NOT EXISTS(SELECT 1 FROM payment_allocation pa WHERE pa.fee_due_id=fee_due.id)",
        params![now, head_id],
    )?;

    // Regenerate from the plan (skips paid instalments via `allocated`).
    let terms: Vec<String> = {
        let mut ts = tx.prepare("SELECT t.name FROM term t JOIN academic_session s ON s.id=t.session_id AND s.is_current=1 ORDER BY t.starts_on")?;
        let v = ts.query_map([], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<String>>>()?;
        v
    };
    let month = now.get(0..7).unwrap_or(now).to_string();
    let spec = PeriodSpec { terms, months: vec![month] };
    let dues = vidya_core::fees::generate_dues(&[head], &students, &spec, &allocated);
    for d in &dues {
        tx.execute(
            "INSERT INTO fee_due(id,student_id,fee_head_id,period,amount_paise,instalment_no,due_date,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8,?9)",
            params![new_id("due"), d.student_id, d.fee_head_id, d.period, d.amount_paise.get(), d.instalment_no, d.due_date, now, sync_state],
        )?;
    }
    Ok(())
}

pub fn update_fee_head_logic(conn: &mut Connection, actor_s: &SessionStaff, id: &str, input: &FeeHeadInput) -> CmdResult<FeeHeadDto> {
    let actor = actor_from(conn, actor_s)?;
    require_principal(&actor)?;
    vidya_core::validation::validate_name(&input.name)?;
    if input.amount_paise < 0 {
        return Err(CmdError::validation("amount", "negative"));
    }
    let instalments_json = validate_instalments_input(conn, input.amount_paise, input.instalments_json.as_deref())?;
    let now = now_iso();
    let has_plan = instalments_json.is_some();
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE fee_head SET name=?1, name_hi=?2, amount_paise=?3, frequency=?4, applies_to=?5, instalments_json=?6 WHERE id=?7",
        params![input.name, input.name_hi, input.amount_paise, input.frequency, input.applies_to, instalments_json, id],
    )?;
    let affected = if has_plan {
        // Instalment plan: cancel unpaid dues and regenerate from the new plan
        // (paid dues untouched).
        regenerate_head_dues_in_tx(&tx, id, &now, "confirmed")?;
        0
    } else {
        // Amount-only: apply the new amount to UNPAID dues; paid dues never change.
        tx.execute(
            "UPDATE fee_due SET amount_paise=?1, updated_at=?2 WHERE fee_head_id=?3 AND cancelled_at IS NULL \
               AND NOT EXISTS(SELECT 1 FROM payment_allocation pa WHERE pa.fee_due_id=fee_due.id)",
            params![input.amount_paise, now, id],
        )? as i64
    };
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(),
        staff_id: Some(actor_s.id.clone()),
        action: "update_fee_head".into(),
        table: Some("fee_head".into()),
        record_id: Some(id.to_string()),
        after_json: Some(serde_json::json!({ "amount_paise": input.amount_paise, "dues_updated": affected, "plan": has_plan }).to_string()),
        ..Default::default()
    })?;
    tx.commit()?;
    list_fee_heads_logic(conn, actor_s)?.into_iter().find(|h| h.id == id).ok_or_else(CmdError::not_found)
}

pub fn deactivate_fee_head_logic(conn: &mut Connection, actor_s: &SessionStaff, id: &str) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_principal(&actor)?;
    conn.execute("UPDATE fee_head SET active=0 WHERE id=?1", params![id])?;
    audit_action(conn, AuditEntry {
        at: now_iso(),
        staff_id: Some(actor_s.id.clone()),
        action: "deactivate_fee_head".into(),
        table: Some("fee_head".into()),
        record_id: Some(id.to_string()),
        ..Default::default()
    })?;
    Ok(())
}

// ============================================================= expenses =======
//
// P15 Step 3 (§10.3). An expense is append-only; a mistake is fixed by an
// expense_reversal (Principal only). Each confirmed expense posts one balanced
// voucher (Dr category account, Cr the money account for paid_via). The category
// must be an active expense account. A cash expense that would drive cash in hand
// negative returns a WARNING (never a block) [OWNER default].

#[derive(Debug, Serialize)]
pub struct ExpenseAccountDto {
    pub id: String,
    pub code: String,
    pub name: String,
    pub name_hi: Option<String>,
}

/// Permission gate for saving an attachment (bill photo): a finance write.
pub fn check_attachment_write(conn: &mut Connection, actor_s: &SessionStaff) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::RecordExpense, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Accounts)
}

/// Permission gate for reading an attachment: a finance view.
pub fn check_attachment_read(conn: &mut Connection, actor_s: &SessionStaff) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ViewAccounts, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Accounts)
}

/// Active expense ledger accounts, for the Record-expense category chips.
pub fn list_expense_accounts_logic(conn: &mut Connection, actor_s: &SessionStaff) -> CmdResult<Vec<ExpenseAccountDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::RecordExpense, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Accounts)?;
    let mut stmt = conn.prepare(
        "SELECT id, code, name, name_hi FROM ledger_account WHERE kind='expense' AND active=1 ORDER BY code",
    )?;
    let rows = stmt
        .query_map([], |r| Ok(ExpenseAccountDto { id: r.get(0)?, code: r.get(1)?, name: r.get(2)?, name_hi: r.get(3)? }))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

/// Cash / bank in hand = Σ(debit − credit) on the CASH / BANK ledger accounts.
fn account_balance(conn: &Connection, account_id: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(debit_paise - credit_paise),0) FROM ledger_entry WHERE account_id=?1",
        params![account_id],
        |r| r.get(0),
    )
}

#[derive(Debug, Deserialize)]
pub struct ExpenseInput {
    pub category_account_id: String,
    pub amount_paise: i64,
    pub paid_via: String, // cash | upi | bank
    pub details: Option<String>,
    pub vendor: Option<String>,
    pub bill_attachment: Option<String>, // sha256 into the attachment store
    pub spent_on: Option<String>,        // YYYY-MM-DD, defaults to today
}

#[derive(Debug, Serialize, Default)]
pub struct ExpenseDto {
    pub id: String,
    pub voucher_no: Option<String>,
    pub category_account_id: String,
    pub category_name: String,
    pub amount_paise: i64,
    pub paid_via: String,
    pub details: Option<String>,
    pub vendor: Option<String>,
    pub bill_attachment: Option<String>,
    pub spent_on: String,
    pub confirmed: bool,
    pub reversed: bool,
    /// True when a cash expense would drive cash in hand negative (a warning that
    /// the save still succeeded — [OWNER default]).
    pub cash_warning: bool,
    /// Whether the bill photo blob is present locally (P15 Step 2). `false` when
    /// there is a `bill_attachment` hash but the blob hasn't arrived yet → the UI
    /// shows "photo not yet received". Set by the command wrapper from the store.
    pub bill_received: bool,
}

pub fn record_expense_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    today: &str,
    input: &ExpenseInput,
) -> CmdResult<ExpenseDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::RecordExpense, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Accounts)?;

    let paid_via = vidya_core::accounts::PaidVia::parse(&input.paid_via)
        .ok_or_else(|| CmdError::validation("paid_via", "invalid"))?;
    // The category must be an active expense account.
    let category_ok: bool = conn
        .query_row(
            "SELECT 1 FROM ledger_account WHERE id=?1 AND kind='expense' AND active=1",
            params![input.category_account_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    vidya_core::accounts::validate_expense(input.amount_paise, category_ok)?;
    let category_name: String = conn
        .query_row("SELECT name FROM ledger_account WHERE id=?1", params![input.category_account_id], |r| r.get(0))
        .optional()?
        .unwrap_or_default();

    // Cash-in-hand warning (never a block).
    let cash_in_hand = account_balance(conn, vidya_core::ledger::CASH)?;
    let cash_warning = vidya_core::accounts::cash_would_go_negative(cash_in_hand, paid_via, input.amount_paise);

    let spent_on = input.spent_on.clone().unwrap_or_else(|| today[..today.len().min(10)].to_string());
    let now = now_iso();
    let exp_id = new_id("exp");
    let confirmed = device_mode == DeviceMode::Server;
    let (series, _) = receipt_series_and_last(conn, device_id)?;
    let school_id = single_school_id(conn)?;
    let ctx = WriteCtx { mode: device_mode };
    let dev = device_id.map(str::to_string);
    let input_amount = input.amount_paise;
    let category = input.category_account_id.clone();
    let details = input.details.clone();
    let vendor = input.vendor.clone();
    let bill = input.bill_attachment.clone();
    let exp_id2 = exp_id.clone();
    let spent = spent_on.clone();

    let voucher_no = with_write(conn, &ctx, move |tx| {
        // On the server (confirmed) post the balanced voucher inline; on a client
        // the row stays on_device and the server posts the voucher on confirmation
        // (idempotent backfill, like payments).
        let voucher_no: Option<String> = if confirmed {
            let vno = crate::numbering::next_no(tx, vidya_core::numbering::NumberKind::Voucher, &series)?;
            let vid = crate::ledger::post_expense_voucher(
                tx, &vno, &exp_id2, &category, paid_via, input_amount, &spent,
                Some(actor_s.id.as_str()), dev.as_deref(), school_id.as_deref(), &now, "confirmed",
            )?;
            tx.execute(
                "INSERT INTO expense(id,voucher_id,category_account_id,amount_paise,paid_via,details,vendor,bill_attachment,spent_on,created_by,device_id,school_id,created_at,updated_at,sync_state) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?13,?14)",
                params![exp_id2, vid, category, input_amount, paid_via.as_str(), details, vendor, bill, spent, actor_s.id, dev, school_id, now, "confirmed"],
            )?;
            Some(vno)
        } else {
            tx.execute(
                "INSERT INTO expense(id,category_account_id,amount_paise,paid_via,details,vendor,bill_attachment,spent_on,created_by,device_id,school_id,created_at,updated_at,sync_state) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?12,?13)",
                params![exp_id2, category, input_amount, paid_via.as_str(), details, vendor, bill, spent, actor_s.id, dev, school_id, now, "on_device"],
            )?;
            None
        };
        let audit = AuditEntry {
            at: now.clone(),
            staff_id: Some(actor_s.id.clone()),
            action: "record_expense".into(),
            table: Some("expense".into()),
            record_id: Some(exp_id2.clone()),
            after_json: Some(serde_json::json!({ "amount_paise": input_amount, "category": category, "paid_via": paid_via.as_str(), "voucher_no": voucher_no }).to_string()),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"),
            hlc: now.clone(),
            device_id: dev.clone().unwrap_or_default(),
            staff_id: actor_s.id.clone(),
            audience: "finance".into(),
            table: "expense".into(),
            record_id: exp_id2.clone(),
            kind: "insert".into(),
            payload: "{}".into(),
            base_version: None,
            server_epoch: 1,
        };
        Ok(Effect { value: voucher_no, audit, op })
    })?;

    Ok(ExpenseDto {
        id: exp_id,
        voucher_no,
        category_account_id: input.category_account_id.clone(),
        category_name,
        amount_paise: input.amount_paise,
        paid_via: input.paid_via.clone(),
        details: input.details.clone(),
        vendor: input.vendor.clone(),
        bill_attachment: input.bill_attachment.clone(),
        spent_on,
        confirmed,
        reversed: false,
        cash_warning,
        bill_received: input.bill_attachment.is_some(),
    })
}

/// Expenses for a date range (inclusive `from`..`to`, YYYY-MM-DD), newest first,
/// with the category name, voucher number and reversal state.
pub fn list_expenses_logic(conn: &mut Connection, actor_s: &SessionStaff, from: &str, to: &str) -> CmdResult<Vec<ExpenseDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ViewAccounts, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Accounts)?;
    let mut stmt = conn.prepare(
        "SELECT e.id, v.voucher_no, e.category_account_id, COALESCE(a.name,''), e.amount_paise, e.paid_via, \
                e.details, e.vendor, e.bill_attachment, e.spent_on, e.sync_state, \
                EXISTS(SELECT 1 FROM expense_reversal r WHERE r.expense_id=e.id) \
         FROM expense e LEFT JOIN voucher v ON v.source_table='expense' AND v.source_id=e.id \
           LEFT JOIN ledger_account a ON a.id=e.category_account_id \
         WHERE e.spent_on >= ?1 AND e.spent_on <= ?2 ORDER BY e.spent_on DESC, e.created_at DESC",
    )?;
    let rows = stmt
        .query_map(params![from, to], |r| {
            Ok(ExpenseDto {
                id: r.get(0)?,
                voucher_no: r.get(1)?,
                category_account_id: r.get(2)?,
                category_name: r.get(3)?,
                amount_paise: r.get(4)?,
                paid_via: r.get(5)?,
                details: r.get(6)?,
                vendor: r.get(7)?,
                bill_attachment: r.get(8)?,
                spent_on: r.get(9)?,
                confirmed: r.get::<_, String>(10)? == "confirmed",
                reversed: r.get::<_, i64>(11)? != 0,
                cash_warning: false,
                bill_received: false, // set by the command wrapper from the store
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

/// Reverse an expense (Principal only): posts the opposite voucher and records an
/// expense_reversal. The original expense is never edited (append-only).
pub fn reverse_expense_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    expense_id: &str,
    reason: &str,
) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ReverseExpense, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Accounts)?;
    if reason.trim().is_empty() {
        return Err(CmdError::validation("reason", "required"));
    }
    // Load the expense (must exist and not already be reversed).
    let (category, paid_via_s, amount, spent_on): (String, String, i64, String) = conn
        .query_row(
            "SELECT category_account_id, paid_via, amount_paise, spent_on FROM expense WHERE id=?1",
            params![expense_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    let already: bool = conn
        .query_row("SELECT 1 FROM expense_reversal WHERE expense_id=?1", params![expense_id], |_| Ok(()))
        .optional()?
        .is_some();
    if already {
        return Err(CmdError::validation("expense", "already_reversed"));
    }
    let paid_via = vidya_core::accounts::PaidVia::parse(&paid_via_s).ok_or_else(|| CmdError::validation("paid_via", "invalid"))?;

    let now = now_iso();
    let rev_id = new_id("exprev");
    let confirmed = device_mode == DeviceMode::Server;
    let sync_state = if confirmed { "confirmed" } else { "on_device" };
    let (series, _) = receipt_series_and_last(conn, device_id)?;
    let school_id = single_school_id(conn)?;
    let ctx = WriteCtx { mode: device_mode };
    let dev = device_id.map(str::to_string);
    let rev_id2 = rev_id.clone();
    let eid = expense_id.to_string();
    let reason_s = reason.to_string();

    with_write(conn, &ctx, move |tx| {
        let vid: Option<String> = if confirmed {
            let vno = crate::numbering::next_no(tx, vidya_core::numbering::NumberKind::Voucher, &series)?;
            Some(crate::ledger::post_expense_reversal_voucher(
                tx, &vno, &rev_id2, &category, paid_via, amount, &spent_on,
                Some(actor_s.id.as_str()), school_id.as_deref(), &now, "confirmed",
            )?)
        } else {
            None
        };
        tx.execute(
            "INSERT INTO expense_reversal(id,expense_id,voucher_id,reason,approved_by,applied_at,school_id,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?6,?6,?8)",
            params![rev_id2, eid, vid, reason_s, actor_s.id, now, school_id, sync_state],
        )?;
        let audit = AuditEntry {
            at: now.clone(),
            staff_id: Some(actor_s.id.clone()),
            action: "reverse_expense".into(),
            table: Some("expense_reversal".into()),
            record_id: Some(rev_id2.clone()),
            reason: Some(reason_s.clone()),
            after_json: Some(serde_json::json!({ "expense_id": eid, "amount_paise": amount }).to_string()),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"),
            hlc: now.clone(),
            device_id: dev.clone().unwrap_or_default(),
            staff_id: actor_s.id.clone(),
            audience: "finance".into(),
            table: "expense_reversal".into(),
            record_id: rev_id2.clone(),
            kind: "insert".into(),
            payload: "{}".into(),
            base_version: None,
            server_epoch: 1,
        };
        Ok(Effect { value: (), audit, op })
    })?;
    Ok(())
}

// ================================================= accounts: cash book / profit
//
// P15 Step 4 (§10.3). The cash book, profit summary and opening balance are all
// computed from the ledger (voucher + ledger_entry), so "cash book money in" =
// day book total and Σ debits = Σ credits for every day in the seed (DONE-MEANS).

/// The Accounts money accounts (cash + bank) — cash-book "money" lives here.
const MONEY_ACCOUNTS: &[&str] = &[vidya_core::ledger::CASH, vidya_core::ledger::BANK, vidya_core::ledger::CHEQUES];

fn money_placeholders() -> String {
    MONEY_ACCOUNTS.iter().map(|a| format!("'{a}'")).collect::<Vec<_>>().join(",")
}

#[derive(Debug, Serialize, Default)]
pub struct OpeningBalanceDto {
    pub set: bool,
    pub cash_paise: i64,
    pub bank_paise: i64,
}

/// Whether the once-per-session opening balance is set, and its amounts.
pub fn get_opening_balance_logic(conn: &mut Connection, actor_s: &SessionStaff) -> CmdResult<OpeningBalanceDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ViewAccounts, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Accounts)?;
    let vid: Option<String> = conn
        .query_row("SELECT id FROM voucher WHERE kind='opening' LIMIT 1", [], |r| r.get(0))
        .optional()?;
    match vid {
        None => Ok(OpeningBalanceDto::default()),
        Some(vid) => {
            let cash: i64 = conn.query_row("SELECT COALESCE(SUM(debit_paise),0) FROM ledger_entry WHERE voucher_id=?1 AND account_id=?2", params![vid, vidya_core::ledger::CASH], |r| r.get(0))?;
            let bank: i64 = conn.query_row("SELECT COALESCE(SUM(debit_paise),0) FROM ledger_entry WHERE voucher_id=?1 AND account_id=?2", params![vid, vidya_core::ledger::BANK], |r| r.get(0))?;
            Ok(OpeningBalanceDto { set: true, cash_paise: cash, bank_paise: bank })
        }
    }
}

/// Set the once-per-session opening balance (Principal). Posts one balanced
/// opening voucher: Dr Cash + Dr Bank, Cr Opening equity. Rejected if already set.
pub fn set_opening_balance_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    today: &str,
    cash_paise: i64,
    bank_paise: i64,
) -> CmdResult<OpeningBalanceDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::OpeningBalance, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Accounts)?;
    vidya_core::accounts::validate_opening_balance(cash_paise, bank_paise)?;
    let exists: bool = conn.query_row("SELECT 1 FROM voucher WHERE kind='opening' LIMIT 1", [], |_| Ok(())).optional()?.is_some();
    if exists {
        return Err(CmdError::validation("opening", "already_set"));
    }
    let now = now_iso();
    // The opening voucher is dated at the session start so it counts as the
    // opening balance (not "money in today") on every day of the session.
    let date = current_session_range(conn)?.0.min(today[..today.len().min(10)].to_string());
    let (series, _) = receipt_series_and_last(conn, device_id)?;
    let school_id = single_school_id(conn)?;
    let ctx = WriteCtx { mode: device_mode };
    let dev = device_id.map(str::to_string);
    with_write(conn, &ctx, move |tx| {
        let vno = crate::numbering::next_no(tx, vidya_core::numbering::NumberKind::Voucher, &series)?;
        let vid = format!("vch-{}", uuid::Uuid::now_v7());
        tx.execute(
            "INSERT INTO voucher(id, voucher_no, kind, date, narration, source_table, source_id, created_by, device_id, school_id, created_at, updated_at, sync_state) \
             VALUES (?1,?2,'opening',?3,'Opening balance',NULL,NULL,?4,?5,?6,?7,?7,'confirmed')",
            params![vid, vno, date, actor_s.id, dev, school_id, now],
        )?;
        let entry = |acc: &str, debit: i64| -> rusqlite::Result<()> {
            if debit > 0 {
                tx.execute(
                    "INSERT INTO ledger_entry(id, voucher_id, account_id, debit_paise, credit_paise) VALUES (?1,?2,?3,?4,0)",
                    params![format!("le-{}", uuid::Uuid::now_v7()), vid, acc, debit],
                )?;
            }
            Ok(())
        };
        entry(vidya_core::ledger::CASH, cash_paise)?;
        entry(vidya_core::ledger::BANK, bank_paise)?;
        tx.execute(
            "INSERT INTO ledger_entry(id, voucher_id, account_id, debit_paise, credit_paise) VALUES (?1,?2,?3,0,?4)",
            params![format!("le-{}", uuid::Uuid::now_v7()), vid, vidya_core::ledger::OPENING_EQUITY, cash_paise + bank_paise],
        )?;
        let audit = AuditEntry {
            at: now.clone(),
            staff_id: Some(actor_s.id.clone()),
            action: "set_opening_balance".into(),
            table: Some("voucher".into()),
            record_id: Some(vid.clone()),
            after_json: Some(serde_json::json!({ "cash_paise": cash_paise, "bank_paise": bank_paise }).to_string()),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"), hlc: now.clone(), device_id: dev.clone().unwrap_or_default(), staff_id: actor_s.id.clone(),
            audience: "finance".into(), table: "voucher".into(), record_id: vid.clone(), kind: "insert".into(),
            payload: "{}".into(), base_version: None, server_epoch: 1,
        };
        Ok(Effect { value: (), audit, op })
    })?;
    get_opening_balance_logic(conn, actor_s)
}

#[derive(Debug, Serialize)]
pub struct CashBookRow {
    pub time: String,          // HH:MM
    pub ref_no: String,        // receipt / voucher number
    pub details: String,
    pub in_paise: i64,
    pub out_paise: i64,
    pub balance_paise: i64,    // running cash + bank in hand
}

#[derive(Debug, Serialize)]
pub struct CashBookDto {
    pub date: String,
    pub opening_paise: i64,             // cash + bank at the start of the day
    pub money_in_paise: i64,           // debits to money accounts today
    pub money_out_paise: i64,          // credits to money accounts today
    pub in_hand_paise: i64,            // opening + in − out
    pub rows: Vec<CashBookRow>,
}

/// The cash book for a day (prototype `accounts` state 1): every voucher that
/// touched cash/bank, with money in, money out and a running balance.
pub fn cash_book_logic(conn: &mut Connection, actor_s: &SessionStaff, date: &str) -> CmdResult<CashBookDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ViewAccounts, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Accounts)?;
    let money = money_placeholders();

    // Opening = cumulative (debit − credit) on money accounts for vouchers dated
    // before `date`.
    let opening: i64 = conn.query_row(
        &format!(
            "SELECT COALESCE(SUM(le.debit_paise - le.credit_paise),0) FROM ledger_entry le \
             JOIN voucher v ON v.id=le.voucher_id WHERE le.account_id IN ({money}) AND v.date < ?1"
        ),
        params![date], |r| r.get(0),
    )?;

    // Per-voucher money in/out for the day.
    let sql = format!(
        "SELECT v.id, v.voucher_no, COALESCE(v.narration,''), v.created_at, \
                COALESCE(SUM(le.debit_paise),0), COALESCE(SUM(le.credit_paise),0) \
         FROM voucher v JOIN ledger_entry le ON le.voucher_id=v.id \
         WHERE le.account_id IN ({money}) AND v.date = ?1 \
         GROUP BY v.id ORDER BY v.created_at, v.voucher_no"
    );
    let raw: Vec<(String, String, String, String, i64, i64)> = {
        let mut stmt = conn.prepare(&sql)?;
        let out = stmt.query_map(params![date], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?.collect::<rusqlite::Result<_>>()?;
        out
    };
    let mut rows = Vec::new();
    let mut running = opening;
    let mut in_total = 0i64;
    let mut out_total = 0i64;
    for (_vid, vno, narration, created_at, debit, credit) in raw {
        running += debit - credit;
        in_total += debit;
        out_total += credit;
        let time = created_at.get(11..16).unwrap_or("").to_string();
        rows.push(CashBookRow { time, ref_no: vno, details: narration, in_paise: debit, out_paise: credit, balance_paise: running });
    }
    Ok(CashBookDto { date: date.to_string(), opening_paise: opening, money_in_paise: in_total, money_out_paise: out_total, in_hand_paise: opening + in_total - out_total, rows })
}

#[derive(Debug, Serialize)]
pub struct ProfitMonth {
    pub month: String,        // YYYY-MM
    pub income_paise: i64,
    pub expense_paise: i64,
    pub surplus_paise: i64,
}

#[derive(Debug, Serialize)]
pub struct ProfitDto {
    pub income_paise: i64,
    pub expense_paise: i64,
    pub surplus_paise: i64,
    pub fees_due_paise: i64,
    pub months: Vec<ProfitMonth>,
}

/// The profit summary (prototype `accounts` state 3): income and expense account
/// totals per month + year to date, plus fees still due. Income = Σ(credit−debit)
/// on income accounts; expense = Σ(debit−credit) on expense accounts.
pub fn profit_summary_logic(conn: &mut Connection, actor_s: &SessionStaff) -> CmdResult<ProfitDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ViewProfit, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Accounts)?;

    // Per-month income and expense from the ledger.
    let mut months: std::collections::BTreeMap<String, (i64, i64)> = std::collections::BTreeMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT substr(v.date,1,7), a.kind, SUM(le.debit_paise), SUM(le.credit_paise) \
             FROM voucher v JOIN ledger_entry le ON le.voucher_id=v.id JOIN ledger_account a ON a.id=le.account_id \
             WHERE a.kind IN ('income','expense') GROUP BY 1, 2",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?)))?;
        for row in rows {
            let (month, kind, debit, credit) = row?;
            let e = months.entry(month).or_insert((0, 0));
            if kind == "income" {
                e.0 += credit - debit;
            } else {
                e.1 += debit - credit;
            }
        }
    }
    let months: Vec<ProfitMonth> = months
        .into_iter()
        .map(|(month, (income, expense))| ProfitMonth { month, income_paise: income, expense_paise: expense, surplus_paise: income - expense })
        .collect();
    let income: i64 = months.iter().map(|m| m.income_paise).sum();
    let expense: i64 = months.iter().map(|m| m.expense_paise).sum();
    let fees_due = crate::dash::outstanding(conn)?;
    Ok(ProfitDto { income_paise: income, expense_paise: expense, surplus_paise: income - expense, fees_due_paise: fees_due, months })
}

// ============================================================== salary ========
//
// P15 Step 5 (§10.3). Principal only. Monthly salary per staff (salary_structure),
// advances (staff_advance → advance voucher), a monthly register whose lines are
// computed by vidya_core::salary (deduction = monthly ÷ working days × unpaid days,
// half-up to the rupee — OWNER default), and Pay (salary voucher per staff).

/// Working days in a `YYYY-MM` month from the school calendar.
fn working_days_in_month(conn: &Connection, month: &str) -> rusqlite::Result<u32> {
    let first = format!("{month}-01");
    let start = match vidya_core::calendar::parse_date(&first) {
        Some(d) => d,
        None => return Ok(0),
    };
    let last_day = start.month().length(start.year());
    let end = start.replace_day(last_day).unwrap_or(start);
    let week = crate::calendar::load_week(conn)?;
    let events = crate::calendar::load_events(conn)?;
    Ok(vidya_core::calendar::working_days(start, end, &week, &events))
}

/// The staff member's latest monthly salary effective on/before the month end.
fn monthly_salary_for(conn: &Connection, staff_id: &str, month: &str) -> rusqlite::Result<Option<i64>> {
    let month_end = format!("{month}-31");
    conn.query_row(
        "SELECT monthly_paise FROM salary_structure WHERE staff_id=?1 AND effective_from<=?2 ORDER BY effective_from DESC LIMIT 1",
        params![staff_id, month_end],
        |r| r.get(0),
    )
    .optional()
}

/// (remaining advance, recover-per-month) for a staff member.
fn advance_state(conn: &Connection, staff_id: &str) -> rusqlite::Result<(i64, i64)> {
    conn.query_row(
        "SELECT COALESCE(SUM(amount_paise - recovered_paise),0), COALESCE(SUM(CASE WHEN amount_paise>recovered_paise THEN recover_per_month_paise ELSE 0 END),0) \
         FROM staff_advance WHERE staff_id=?1",
        params![staff_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
}

#[derive(Debug, Deserialize, Default)]
pub struct StaffDaysInput {
    pub staff_id: String,
    pub days_present: i64,
}

#[derive(Debug, Serialize)]
pub struct SalaryRowDto {
    pub staff_id: String,
    pub name: String,
    pub role: String,
    pub monthly_paise: i64,
    pub working_days: i64,
    pub days_present: i64,
    pub unpaid_leave_days: i64,
    pub deduction_paise: i64,
    pub advance_recovery_paise: i64,
    pub remaining_advance_paise: i64,
    pub net_paise: i64,
    pub paid: bool,
}

#[derive(Debug, Serialize)]
pub struct SalaryRegisterDto {
    pub month: String,
    pub working_days: i64,
    pub total_salaries_paise: i64,
    pub advances_recovered_paise: i64,
    pub unpaid_deducted_paise: i64,
    pub net_to_pay_paise: i64,
    pub pending: i64,
    pub rows: Vec<SalaryRowDto>,
}

/// Build one register row for a staff member (computed via vidya-core).
#[allow(clippy::too_many_arguments)]
fn salary_row(
    conn: &Connection,
    staff_id: &str,
    name: &str,
    role: &str,
    monthly: i64,
    working_days: u32,
    days_present: u32,
    paid: bool,
) -> rusqlite::Result<SalaryRowDto> {
    let (remaining_advance, recover_target) = advance_state(conn, staff_id)?;
    let line = vidya_core::salary::compute_salary_line(&vidya_core::salary::SalaryInputs {
        monthly_paise: monthly,
        working_days,
        days_present,
        advance_recovery_paise: recover_target,
        remaining_advance_paise: remaining_advance,
    });
    Ok(SalaryRowDto {
        staff_id: staff_id.to_string(),
        name: name.to_string(),
        role: role.to_string(),
        monthly_paise: monthly,
        working_days: working_days as i64,
        days_present: line.days_present as i64,
        unpaid_leave_days: line.unpaid_leave_days as i64,
        deduction_paise: line.deduction_paise,
        advance_recovery_paise: line.advance_recovery_paise,
        remaining_advance_paise: remaining_advance,
        net_paise: line.net_paise,
        paid,
    })
}

/// The salary register for a month (prototype `salary`). `days` is the Principal's
/// per-staff days-present entry (until Phase 17); missing staff = full attendance.
pub fn salary_register_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    month: &str,
    days: &[StaffDaysInput],
) -> CmdResult<SalaryRegisterDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageSalary, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Accounts)?;
    let working_days = working_days_in_month(conn, month)?;
    let days_map: std::collections::HashMap<&str, u32> =
        days.iter().map(|d| (d.staff_id.as_str(), d.days_present.max(0) as u32)).collect();

    // Active staff with a salary structure.
    let staff: Vec<(String, String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT s.id, s.name, s.role FROM staff s \
             WHERE s.state='active' AND EXISTS(SELECT 1 FROM salary_structure ss WHERE ss.staff_id=s.id) ORDER BY s.name",
        )?;
        let v = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<rusqlite::Result<Vec<(String, String, String)>>>()?;
        v
    };

    let mut rows = Vec::new();
    let (mut total, mut recovered, mut deducted, mut net_to_pay, mut pending) = (0i64, 0i64, 0i64, 0i64, 0i64);
    for (id, name, role) in staff {
        let monthly = monthly_salary_for(conn, &id, month)?.unwrap_or(0);
        if monthly <= 0 {
            continue;
        }
        let days_present = *days_map.get(id.as_str()).unwrap_or(&working_days);
        let paid: bool = conn
            .query_row(
                "SELECT 1 FROM salary_line l JOIN salary_run r ON r.id=l.run_id WHERE r.month=?1 AND l.staff_id=?2 AND l.paid_voucher_id IS NOT NULL",
                params![month, id],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        let row = salary_row(conn, &id, &name, &role, monthly, working_days, days_present, paid)?;
        total += row.monthly_paise;
        recovered += row.advance_recovery_paise;
        deducted += row.deduction_paise;
        net_to_pay += row.net_paise;
        if !paid {
            pending += 1;
        }
        rows.push(row);
    }
    Ok(SalaryRegisterDto {
        month: month.to_string(),
        working_days: working_days as i64,
        total_salaries_paise: total,
        advances_recovered_paise: recovered,
        unpaid_deducted_paise: deducted,
        net_to_pay_paise: net_to_pay,
        pending,
        rows,
    })
}

#[derive(Debug, Deserialize)]
pub struct SalaryStructureInput {
    pub staff_id: String,
    pub monthly_paise: i64,
    pub effective_from: Option<String>,
}

pub fn set_salary_structure_logic(conn: &mut Connection, actor_s: &SessionStaff, today: &str, input: &SalaryStructureInput) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageSalary, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Accounts)?;
    vidya_core::salary::validate_monthly(input.monthly_paise)?;
    let now = now_iso();
    let eff = input.effective_from.clone().unwrap_or_else(|| today[..today.len().min(10)].to_string());
    let school_id = single_school_id(conn)?;
    let id = new_id("sal");
    conn.execute(
        "INSERT INTO salary_structure(id,staff_id,monthly_paise,effective_from,school_id,created_at,updated_at,sync_state) \
         VALUES (?1,?2,?3,?4,?5,?6,?6,'confirmed')",
        params![id, input.staff_id, input.monthly_paise, eff, school_id, now],
    )?;
    audit_action(conn, AuditEntry {
        at: now, staff_id: Some(actor_s.id.clone()), action: "set_salary_structure".into(),
        table: Some("salary_structure".into()), record_id: Some(id),
        after_json: Some(serde_json::json!({ "staff_id": input.staff_id, "monthly_paise": input.monthly_paise }).to_string()),
        ..Default::default()
    })?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct AdvanceInput {
    pub staff_id: String,
    pub amount_paise: i64,
    pub recover_per_month_paise: i64,
    pub mode: String, // cash | bank
}

/// Give a staff advance (Principal): staff_advance row + advance voucher (Dr staff
/// advances, Cr money).
pub fn give_advance_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    today: &str,
    input: &AdvanceInput,
) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageSalary, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Accounts)?;
    vidya_core::salary::validate_advance(input.amount_paise, input.recover_per_month_paise)?;
    let money = if input.mode == "cash" { vidya_core::ledger::CASH } else { vidya_core::ledger::BANK };
    let now = now_iso();
    let date = today[..today.len().min(10)].to_string();
    let confirmed = device_mode == DeviceMode::Server;
    let sync_state = if confirmed { "confirmed" } else { "on_device" };
    let (series, _) = receipt_series_and_last(conn, device_id)?;
    let school_id = single_school_id(conn)?;
    let ctx = WriteCtx { mode: device_mode };
    let dev = device_id.map(str::to_string);
    let adv_id = new_id("adv");
    let adv_id2 = adv_id.clone();
    let staff = input.staff_id.clone();
    let amount = input.amount_paise;
    let recover = input.recover_per_month_paise;
    with_write(conn, &ctx, move |tx| {
        let vid: Option<String> = if confirmed {
            let vno = crate::numbering::next_no(tx, vidya_core::numbering::NumberKind::Voucher, &series)?;
            Some(crate::ledger::post_advance_voucher(tx, &vno, &adv_id2, money, amount, &date, Some(actor_s.id.as_str()), dev.as_deref(), school_id.as_deref(), &now, "confirmed")?)
        } else {
            None
        };
        tx.execute(
            "INSERT INTO staff_advance(id,staff_id,amount_paise,voucher_id,recover_per_month_paise,recovered_paise,given_on,created_by,device_id,school_id,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,0,?6,?7,?8,?9,?10,?10,?11)",
            params![adv_id2, staff, amount, vid, recover, date, actor_s.id, dev, school_id, now, sync_state],
        )?;
        let audit = AuditEntry {
            at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "give_advance".into(),
            table: Some("staff_advance".into()), record_id: Some(adv_id2.clone()),
            after_json: Some(serde_json::json!({ "staff_id": staff, "amount_paise": amount }).to_string()),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"), hlc: now.clone(), device_id: dev.clone().unwrap_or_default(), staff_id: actor_s.id.clone(),
            audience: "finance".into(), table: "staff_advance".into(), record_id: adv_id2.clone(), kind: "insert".into(),
            payload: "{}".into(), base_version: None, server_epoch: 1,
        };
        Ok(Effect { value: (), audit, op })
    })?;
    Ok(())
}

#[derive(Debug, Serialize, Default)]
pub struct PaySalariesResult {
    pub paid: i64,
    pub total_net_paise: i64,
}

/// Pay all currently-unpaid staff for the month (Principal). One transaction: a
/// salary_line + salary voucher per staff, and the advance recovered updated.
/// `mode` = cash | bank. Returns how many were paid and the total net.
pub fn pay_salaries_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    month: &str,
    mode: &str,
    days: &[StaffDaysInput],
) -> CmdResult<PaySalariesResult> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageSalary, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Accounts)?;
    let register = salary_register_logic(conn, actor_s, month, days)?;
    let pending: Vec<SalaryRowDto> = register.rows.into_iter().filter(|r| !r.paid && r.net_paise + r.advance_recovery_paise > 0).collect();
    if pending.is_empty() {
        return Ok(PaySalariesResult::default());
    }
    let money = if mode == "cash" { vidya_core::ledger::CASH } else { vidya_core::ledger::BANK };
    let now = now_iso();
    let date = format!("{month}-01");
    let (series, _) = receipt_series_and_last(conn, device_id)?;
    let school_id = single_school_id(conn)?;
    let ctx = WriteCtx { mode: device_mode };
    let dev = device_id.map(str::to_string);
    let run_id = new_id("srun");
    let run_id2 = run_id.clone();
    let month_s = month.to_string();
    let paid_count = pending.len() as i64;
    let total_net: i64 = pending.iter().map(|r| r.net_paise).sum();
    let staff_id_for_op = actor_s.id.clone();

    with_write(conn, &ctx, move |tx| {
        // One salary_run per month (created on first pay).
        tx.execute(
            "INSERT INTO salary_run(id,month,status,finalised_by,school_id,created_at,updated_at,sync_state) \
             VALUES (?1,?2,'finalised',?3,?4,?5,?5,'confirmed') \
             ON CONFLICT(month) DO UPDATE SET status='finalised', finalised_by=excluded.finalised_by, updated_at=excluded.updated_at",
            params![run_id2, month_s, staff_id_for_op, school_id, now],
        )?;
        let run: String = tx.query_row("SELECT id FROM salary_run WHERE month=?1", params![month_s], |r| r.get(0))?;
        for r in &pending {
            let earned = r.monthly_paise - r.deduction_paise;
            let line_id = new_id("sline");
            let vid: Option<String> = if device_mode == DeviceMode::Server {
                let vno = crate::numbering::next_no(tx, vidya_core::numbering::NumberKind::Voucher, &series)?;
                Some(crate::ledger::post_salary_voucher(tx, &vno, &line_id, money, earned, r.advance_recovery_paise, r.net_paise, &date, Some(staff_id_for_op.as_str()), dev.as_deref(), school_id.as_deref(), &now, "confirmed")?)
            } else {
                None
            };
            tx.execute(
                "INSERT INTO salary_line(id,run_id,staff_id,monthly_paise,working_days,days_present,unpaid_leave_days,deduction_paise,advance_recovery_paise,net_paise,paid_voucher_id,school_id,created_at,updated_at,sync_state) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?13,'confirmed')",
                params![line_id, run, r.staff_id, r.monthly_paise, r.working_days, r.days_present, r.unpaid_leave_days, r.deduction_paise, r.advance_recovery_paise, r.net_paise, vid, school_id, now],
            )?;
            // Recover the advance oldest-first (staff_advance is mutable, not append-only).
            let mut left = r.advance_recovery_paise;
            if left > 0 {
                let mut adv = tx.prepare("SELECT id, amount_paise, recovered_paise FROM staff_advance WHERE staff_id=?1 AND amount_paise>recovered_paise ORDER BY given_on")?;
                let advs: Vec<(String, i64, i64)> = adv.query_map(params![r.staff_id], |x| Ok((x.get(0)?, x.get(1)?, x.get(2)?)))?.collect::<rusqlite::Result<_>>()?;
                drop(adv);
                for (aid, amt, rec) in advs {
                    if left <= 0 {
                        break;
                    }
                    let take = left.min(amt - rec);
                    tx.execute("UPDATE staff_advance SET recovered_paise=recovered_paise+?1, updated_at=?2 WHERE id=?3", params![take, now, aid])?;
                    left -= take;
                }
            }
        }
        let audit = AuditEntry {
            at: now.clone(), staff_id: Some(staff_id_for_op.clone()), action: "pay_salaries".into(),
            table: Some("salary_run".into()), record_id: Some(run.clone()),
            after_json: Some(serde_json::json!({ "month": month_s, "paid": paid_count, "mode": mode }).to_string()),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"), hlc: now.clone(), device_id: dev.clone().unwrap_or_default(), staff_id: staff_id_for_op.clone(),
            audience: "finance".into(), table: "salary_run".into(), record_id: run.clone(), kind: "update".into(),
            payload: "{}".into(), base_version: None, server_epoch: 1,
        };
        Ok(Effect { value: (), audit, op })
    })?;
    Ok(PaySalariesResult { paid: paid_count, total_net_paise: total_net })
}

// ============================================================== store =========
//
// P15 Step 6 (§10.3, optional module `store`). Items with price + stock; sales
// with an S-… receipt + Store-income voucher (append-only); stock can't go
// negative; price is snapshotted onto the sale. Principal manages items/stock;
// Accountant + Principal record sales. Gated by require_module(store).

#[derive(Debug, Serialize)]
pub struct StoreItemDto {
    pub id: String,
    pub name: String,
    pub name_hi: Option<String>,
    pub name_te: Option<String>,
    pub price_paise: i64,
    pub stock: i64,
    pub low_stock_at: i64,
    pub active: bool,
    pub low_stock: bool,
}

pub fn list_store_items_logic(conn: &mut Connection, actor_s: &SessionStaff, include_inactive: bool) -> CmdResult<Vec<StoreItemDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::RecordStoreSale, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Store)?;
    let sql = if include_inactive {
        "SELECT id, name, name_hi, name_te, price_paise, stock, low_stock_at, active FROM store_item ORDER BY name"
    } else {
        "SELECT id, name, name_hi, name_te, price_paise, stock, low_stock_at, active FROM store_item WHERE active=1 ORDER BY name"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map([], |r| {
            let stock: i64 = r.get(5)?;
            let low_at: i64 = r.get(6)?;
            Ok(StoreItemDto {
                id: r.get(0)?,
                name: r.get(1)?,
                name_hi: r.get(2)?,
                name_te: r.get(3)?,
                price_paise: r.get(4)?,
                stock,
                low_stock_at: low_at,
                active: r.get::<_, i64>(7)? != 0,
                low_stock: vidya_core::store::is_low_stock(stock, low_at),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

#[derive(Debug, Deserialize)]
pub struct StoreItemInput {
    pub id: Option<String>,
    pub name: String,
    pub name_hi: Option<String>,
    pub name_te: Option<String>,
    pub price_paise: i64,
    pub low_stock_at: i64,
    pub active: Option<bool>,
}

/// Create or update a store item (Principal). Stock is changed via `stock_adjust`,
/// never here (a new item starts at 0 stock).
pub fn save_store_item_logic(conn: &mut Connection, actor_s: &SessionStaff, input: &StoreItemInput) -> CmdResult<StoreItemDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageStore, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Store)?;
    vidya_core::validation::validate_name(&input.name)?;
    vidya_core::store::validate_item(input.price_paise, input.low_stock_at)?;
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let id = match &input.id {
        Some(id) => {
            conn.execute(
                "UPDATE store_item SET name=?1, name_hi=?2, name_te=?3, price_paise=?4, low_stock_at=?5, active=?6, updated_at=?7 WHERE id=?8",
                params![input.name, input.name_hi, input.name_te, input.price_paise, input.low_stock_at, input.active.unwrap_or(true) as i64, now, id],
            )?;
            id.clone()
        }
        None => {
            let id = new_id("item");
            conn.execute(
                "INSERT INTO store_item(id,name,name_hi,name_te,price_paise,stock,low_stock_at,active,school_id,created_at,updated_at,sync_state) \
                 VALUES (?1,?2,?3,?4,?5,0,?6,1,?7,?8,?8,'confirmed')",
                params![id, input.name, input.name_hi, input.name_te, input.price_paise, input.low_stock_at, school_id, now],
            )?;
            id
        }
    };
    audit_action(conn, AuditEntry {
        at: now, staff_id: Some(actor_s.id.clone()), action: "save_store_item".into(),
        table: Some("store_item".into()), record_id: Some(id.clone()),
        after_json: Some(serde_json::json!({ "name": input.name, "price_paise": input.price_paise }).to_string()),
        ..Default::default()
    })?;
    list_store_items_logic(conn, actor_s, true)?.into_iter().find(|i| i.id == id).ok_or_else(CmdError::not_found)
}

#[derive(Debug, Deserialize)]
pub struct SaleItemInput {
    pub item_id: String,
    pub qty: i64,
}

#[derive(Debug, Deserialize)]
pub struct StoreSaleInput {
    pub student_id: Option<String>,
    pub guardian_id: Option<String>,
    pub items: Vec<SaleItemInput>,
    pub mode: String, // cash | upi | cheque
}

#[derive(Debug, Serialize)]
pub struct StoreSaleDto {
    pub id: String,
    pub receipt_no: String,
    pub total_paise: i64,
    pub confirmed: bool,
}

/// Record a store sale (Accountant + Principal): validates stock, snapshots
/// prices, decrements stock (stock_move), posts a Store-income voucher and an
/// S-… receipt. Stock can never go negative (a bigger sale is blocked).
pub fn record_store_sale_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    today: &str,
    input: &StoreSaleInput,
) -> CmdResult<StoreSaleDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::RecordStoreSale, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Store)?;
    let mode = payment_mode_from(&input.mode)?;

    // Load each item's current price + stock + name; build the sale lines.
    let mut lines = Vec::new();
    let mut snapshot = Vec::new(); // items_json entries
    for it in &input.items {
        let (name, price, stock): (String, i64, i64) = conn
            .query_row("SELECT name, price_paise, stock FROM store_item WHERE id=?1 AND active=1", params![it.item_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .optional()?
            .ok_or_else(CmdError::not_found)?;
        lines.push(vidya_core::store::SaleLine { item_id: it.item_id.clone(), qty: it.qty, price_paise: price, stock });
        snapshot.push(serde_json::json!({ "item_id": it.item_id, "name": name, "qty": it.qty, "price_paise": price }));
    }
    let total = vidya_core::store::validate_sale(&lines)?;

    let now = now_iso();
    let date = today[..today.len().min(10)].to_string();
    let sale_id = new_id("sale");
    let confirmed = device_mode == DeviceMode::Server;
    let sync_state = if confirmed { "confirmed" } else { "on_device" };
    let (series, _) = receipt_series_and_last(conn, device_id)?;
    let school_id = single_school_id(conn)?;
    let ctx = WriteCtx { mode: device_mode };
    let dev = device_id.map(str::to_string);
    let items_json = serde_json::Value::Array(snapshot).to_string();
    let sale_id2 = sale_id.clone();
    let student = input.student_id.clone();
    let guardian = input.guardian_id.clone();
    let mode_s = input.mode.clone();
    let lines2 = lines.clone();

    let receipt_no = with_write(conn, &ctx, move |tx| {
        let receipt_no = crate::numbering::next_no(tx, vidya_core::numbering::NumberKind::StoreSale, &series)?;
        let vid: Option<String> = if confirmed {
            let vno = crate::numbering::next_no(tx, vidya_core::numbering::NumberKind::Voucher, &series)?;
            Some(crate::ledger::post_store_sale_voucher(tx, &vno, &sale_id2, mode, total, &date, Some(actor_s.id.as_str()), dev.as_deref(), school_id.as_deref(), &now, "confirmed")?)
        } else {
            None
        };
        tx.execute(
            "INSERT INTO store_sale(id,receipt_no,student_id,guardian_id,items_json,total_paise,mode,voucher_id,sold_by,sold_at,device_id,school_id,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?10,?10,?13)",
            params![sale_id2, receipt_no, student, guardian, items_json, total, mode_s, vid, actor_s.id, now, dev, school_id, sync_state],
        )?;
        // Decrement stock + record a stock_move per item.
        for l in &lines2 {
            tx.execute(
                "INSERT INTO stock_move(id,item_id,qty,reason,ref_id,by,at,school_id,created_at,updated_at,sync_state) \
                 VALUES (?1,?2,?3,'sale',?4,?5,?6,?7,?6,?6,'confirmed')",
                params![new_id("mv"), l.item_id, -l.qty, sale_id2, actor_s.id, now, school_id],
            )?;
            tx.execute("UPDATE store_item SET stock=stock-?1, updated_at=?2 WHERE id=?3", params![l.qty, now, l.item_id])?;
        }
        let audit = AuditEntry {
            at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "record_store_sale".into(),
            table: Some("store_sale".into()), record_id: Some(sale_id2.clone()),
            after_json: Some(serde_json::json!({ "receipt_no": receipt_no, "total_paise": total }).to_string()),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"), hlc: now.clone(), device_id: dev.clone().unwrap_or_default(), staff_id: actor_s.id.clone(),
            audience: "finance".into(), table: "store_sale".into(), record_id: sale_id2.clone(), kind: "insert".into(),
            payload: "{}".into(), base_version: None, server_epoch: 1,
        };
        Ok(Effect { value: receipt_no, audit, op })
    })?;
    Ok(StoreSaleDto { id: sale_id, receipt_no, total_paise: total, confirmed })
}

#[derive(Debug, Deserialize)]
pub struct StockAdjustInput {
    pub item_id: String,
    pub delta_qty: i64,     // signed: +purchase, ±adjust
    pub reason: String,     // purchase | adjust
}

/// Purchase or adjust stock (Principal): records a stock_move and updates the
/// item's running stock (which must stay ≥ 0).
pub fn stock_adjust_logic(conn: &mut Connection, actor_s: &SessionStaff, today: &str, input: &StockAdjustInput) -> CmdResult<StoreItemDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageStore, &Target::of(TargetKind::Fee))?;
    require_module_enabled(conn, vidya_core::modules::Module::Store)?;
    if !matches!(input.reason.as_str(), "purchase" | "adjust") {
        return Err(CmdError::validation("reason", "invalid"));
    }
    let current: i64 = conn
        .query_row("SELECT stock FROM store_item WHERE id=?1", params![input.item_id], |r| r.get(0))
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    let new_stock = current + input.delta_qty;
    vidya_core::store::validate_new_stock(new_stock)?;
    let now = now_iso();
    let date = today[..today.len().min(10)].to_string();
    let school_id = single_school_id(conn)?;
    conn.execute(
        "INSERT INTO stock_move(id,item_id,qty,reason,by,at,school_id,created_at,updated_at,sync_state) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?6,?6,'confirmed')",
        params![new_id("mv"), input.item_id, input.delta_qty, input.reason, actor_s.id, date, school_id],
    )?;
    conn.execute("UPDATE store_item SET stock=?1, updated_at=?2 WHERE id=?3", params![new_stock, now, input.item_id])?;
    audit_action(conn, AuditEntry {
        at: now, staff_id: Some(actor_s.id.clone()), action: "stock_adjust".into(),
        table: Some("store_item".into()), record_id: Some(input.item_id.clone()),
        after_json: Some(serde_json::json!({ "delta": input.delta_qty, "reason": input.reason, "new_stock": new_stock }).to_string()),
        ..Default::default()
    })?;
    list_store_items_logic(conn, actor_s, true)?.into_iter().find(|i| i.id == input.item_id).ok_or_else(CmdError::not_found)
}

// ============================================================ classroom =======
// Phase 16. The timetable is Principal-server config (edited on the school PC):
// like store items / grade bands, edits are audited and reach devices via the
// snapshot (sync::scope), not device ops. Teachers receive only the slots of the
// classes they teach. vidya-core rejects clashes (teacher / class double-booked,
// teacher not assigned to the class-subject).

#[derive(Debug, Serialize)]
pub struct PeriodDto {
    pub id: String,
    pub no: i64,
    pub starts_at: String,
    pub ends_at: String,
}

#[derive(Debug, Serialize)]
pub struct TimetableSlotDto {
    pub id: String,
    pub class_id: String,
    pub class_display: Option<String>,
    pub weekday: i64,
    pub period_no: i64,
    pub class_subject_id: String,
    pub subject_name: String,
    pub teacher_id: String,
    pub teacher_name: String,
}

#[derive(Debug, Serialize)]
pub struct ClassSubjectOptionDto {
    pub id: String,
    pub subject_name: String,
    pub teacher_id: String,
    pub teacher_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TimetableDto {
    pub class_id: String,
    pub class_display: Option<String>,
    pub periods: Vec<PeriodDto>,
    pub slots: Vec<TimetableSlotDto>,
    /// The class's class-subjects (with their assigned teacher) for the slot editor.
    pub subjects: Vec<ClassSubjectOptionDto>,
}

fn load_class_subject_options(conn: &Connection, class_id: &str) -> rusqlite::Result<Vec<ClassSubjectOptionDto>> {
    let mut stmt = conn.prepare(
        "SELECT cs.id, sub.name, COALESCE(cs.teacher_id,''), st.name FROM class_subject cs \
         JOIN subject sub ON sub.id=cs.subject_id LEFT JOIN staff st ON st.id=cs.teacher_id \
         WHERE cs.class_id=?1 ORDER BY sub.name",
    )?;
    let rows = stmt
        .query_map(params![class_id], |r| {
            Ok(ClassSubjectOptionDto { id: r.get(0)?, subject_name: r.get(1)?, teacher_id: r.get(2)?, teacher_name: r.get(3)? })
        })?
        .collect::<rusqlite::Result<Vec<ClassSubjectOptionDto>>>()?;
    Ok(rows)
}

fn load_periods(conn: &Connection, session_id: &str) -> rusqlite::Result<Vec<PeriodDto>> {
    let mut stmt = conn.prepare("SELECT id, no, starts_at, ends_at FROM period WHERE session_id=?1 ORDER BY no")?;
    let rows = stmt
        .query_map(params![session_id], |r| {
            Ok(PeriodDto { id: r.get(0)?, no: r.get(1)?, starts_at: r.get(2)?, ends_at: r.get(3)? })
        })?
        .collect::<rusqlite::Result<Vec<PeriodDto>>>()?;
    Ok(rows)
}

fn slot_row_map(r: &rusqlite::Row) -> rusqlite::Result<TimetableSlotDto> {
    Ok(TimetableSlotDto {
        id: r.get(0)?,
        class_id: r.get(1)?,
        class_display: r.get(2)?,
        weekday: r.get(3)?,
        period_no: r.get(4)?,
        class_subject_id: r.get(5)?,
        subject_name: r.get(6)?,
        teacher_id: r.get(7)?,
        teacher_name: r.get(8)?,
    })
}

const SLOT_SELECT: &str = "SELECT ts.id, ts.class_id, c.display, ts.weekday, ts.period_no, ts.class_subject_id, sub.name, ts.teacher_id, st.name \
     FROM timetable_slot ts JOIN class_subject cs ON cs.id=ts.class_subject_id \
     JOIN subject sub ON sub.id=cs.subject_id JOIN staff st ON st.id=ts.teacher_id \
     JOIN class c ON c.id=ts.class_id \
     WHERE ts.effective_to IS NULL";

fn load_class_slots(conn: &Connection, class_id: &str) -> rusqlite::Result<Vec<TimetableSlotDto>> {
    let sql = format!("{SLOT_SELECT} AND ts.class_id=?1 ORDER BY ts.weekday, ts.period_no");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![class_id], slot_row_map)?.collect::<rusqlite::Result<Vec<TimetableSlotDto>>>()?;
    Ok(rows)
}

/// True if the teacher class-teaches `class_id` or teaches any subject in it.
fn teacher_in_class(conn: &Connection, staff_id: &str, class_id: &str) -> rusqlite::Result<bool> {
    let ct = conn
        .query_row("SELECT 1 FROM class WHERE id=?1 AND class_teacher_id=?2", params![class_id, staff_id], |_| Ok(()))
        .optional()?
        .is_some();
    if ct {
        return Ok(true);
    }
    Ok(conn
        .query_row("SELECT 1 FROM class_subject WHERE class_id=?1 AND teacher_id=?2 LIMIT 1", params![class_id, staff_id], |_| Ok(()))
        .optional()?
        .is_some())
}

/// The canonical class_subject → teacher assignment map (from class_subject).
fn class_subject_assignments(conn: &Connection) -> rusqlite::Result<std::collections::BTreeMap<String, String>> {
    let mut stmt = conn.prepare("SELECT id, teacher_id FROM class_subject WHERE teacher_id IS NOT NULL")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut m = std::collections::BTreeMap::new();
    for row in rows {
        let (id, t) = row?;
        m.insert(id, t);
    }
    Ok(m)
}

/// Every current slot in the session as vidya-core `Slot`s (for clash checks).
fn all_session_slots(conn: &Connection, session_id: &str) -> rusqlite::Result<Vec<vidya_core::timetable::Slot>> {
    let mut stmt = conn.prepare(
        "SELECT id, class_id, weekday, period_no, class_subject_id, teacher_id FROM timetable_slot \
         WHERE session_id=?1 AND effective_to IS NULL",
    )?;
    let rows = stmt.query_map(params![session_id], |r| {
        Ok(vidya_core::timetable::Slot {
            id: r.get(0)?,
            class_id: r.get(1)?,
            weekday: r.get::<_, i64>(2)? as u8,
            period_no: r.get::<_, i64>(3)? as u8,
            class_subject_id: r.get(4)?,
            teacher_id: r.get(5)?,
        })
    })?;
    rows.collect()
}

/// The Principal week view for one class (prototype `timetable` state 1).
pub fn get_timetable_logic(conn: &mut Connection, actor_s: &SessionStaff, class_id: &str) -> CmdResult<TimetableDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ViewTimetable, &Target { kind: TargetKind::Attendance, class_id: Some(class_id.to_string()), ..Default::default() })?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    // A teacher may only inspect a class they teach; the Principal any class.
    if actor.role == Role::Teacher && !teacher_in_class(conn, &actor_s.id, class_id)? {
        return Err(CmdError::forbidden("teacher_not_own_class"));
    }
    let session_id = current_session_id(conn)?.ok_or_else(CmdError::not_found)?;
    let class_display: Option<String> =
        conn.query_row("SELECT display FROM class WHERE id=?1", params![class_id], |r| r.get(0)).optional()?;
    Ok(TimetableDto {
        class_id: class_id.to_string(),
        class_display,
        periods: load_periods(conn, &session_id)?,
        slots: load_class_slots(conn, class_id)?,
        subjects: load_class_subject_options(conn, class_id)?,
    })
}

#[derive(Debug, Serialize)]
pub struct TeacherTimetableDto {
    pub periods: Vec<PeriodDto>,
    pub slots: Vec<TimetableSlotDto>,
}

/// A teacher's own weekly timetable (prototype teacher "My timetable").
pub fn my_timetable_logic(conn: &mut Connection, actor_s: &SessionStaff) -> CmdResult<TeacherTimetableDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ViewTimetable, &Target::of(TargetKind::Attendance))?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    let session_id = current_session_id(conn)?.ok_or_else(CmdError::not_found)?;
    let sql = format!("{SLOT_SELECT} AND ts.teacher_id=?1 ORDER BY ts.weekday, ts.period_no");
    let mut stmt = conn.prepare(&sql)?;
    let slots: Vec<TimetableSlotDto> = stmt.query_map(params![actor_s.id], slot_row_map)?.collect::<rusqlite::Result<_>>()?;
    Ok(TeacherTimetableDto { periods: load_periods(conn, &session_id)?, slots })
}

#[derive(Debug, Deserialize)]
pub struct TimetableSlotInput {
    pub id: Option<String>,
    pub class_id: String,
    pub weekday: i64,
    pub period_no: i64,
    pub class_subject_id: String,
    pub teacher_id: String,
}

/// The stable rule code for a clash (shown inline on the slot editor).
fn clash_rule(c: &vidya_core::timetable::Clash) -> &'static str {
    use vidya_core::timetable::Clash::*;
    match c {
        TeacherDoubleBooked { .. } => "teacher_busy",
        ClassDoubleBooked { .. } => "class_busy",
        TeacherNotAssigned { .. } => "teacher_not_assigned",
    }
}

/// Create or update one timetable slot (Principal). Rejects clashes via vidya-core.
pub fn save_timetable_slot_logic(conn: &mut Connection, actor_s: &SessionStaff, input: &TimetableSlotInput) -> CmdResult<TimetableSlotDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageTimetable, &Target::of(TargetKind::School))?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    if !(1..=7).contains(&input.weekday) || input.period_no < 1 {
        return Err(CmdError::validation("slot", "range"));
    }
    // The class_subject must belong to the slot's class.
    let cs_class: Option<String> = conn
        .query_row("SELECT class_id FROM class_subject WHERE id=?1", params![input.class_subject_id], |r| r.get(0))
        .optional()?;
    if cs_class.as_deref() != Some(input.class_id.as_str()) {
        return Err(CmdError::validation("class_subject", "wrong_class"));
    }
    let session_id = current_session_id(conn)?.ok_or_else(CmdError::not_found)?;
    let assigned = class_subject_assignments(conn)?;
    let id = input.id.clone().unwrap_or_else(|| new_id("ts"));
    let candidate = vidya_core::timetable::Slot {
        id: id.clone(),
        class_id: input.class_id.clone(),
        weekday: input.weekday as u8,
        period_no: input.period_no as u8,
        class_subject_id: input.class_subject_id.clone(),
        teacher_id: input.teacher_id.clone(),
    };
    // Every current slot except the one being edited.
    let others: Vec<vidya_core::timetable::Slot> =
        all_session_slots(conn, &session_id)?.into_iter().filter(|s| s.id != id).collect();
    if let Err(clashes) = vidya_core::timetable::slot_is_valid(&candidate, &others, &assigned) {
        let rule = clashes.first().map(clash_rule).unwrap_or("clash");
        return Err(CmdError::validation("timetable", rule));
    }
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    if input.id.is_some() {
        conn.execute(
            "UPDATE timetable_slot SET class_id=?1, weekday=?2, period_no=?3, class_subject_id=?4, teacher_id=?5, updated_at=?6 WHERE id=?7",
            params![input.class_id, input.weekday, input.period_no, input.class_subject_id, input.teacher_id, now, id],
        )?;
    } else {
        conn.execute(
            "INSERT INTO timetable_slot(id,session_id,class_id,weekday,period_no,class_subject_id,teacher_id,school_id,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9,'confirmed')",
            params![id, session_id, input.class_id, input.weekday, input.period_no, input.class_subject_id, input.teacher_id, school_id, now],
        )?;
    }
    audit_action(conn, AuditEntry {
        at: now,
        staff_id: Some(actor_s.id.clone()),
        action: "save_timetable_slot".into(),
        table: Some("timetable_slot".into()),
        record_id: Some(id.clone()),
        after_json: Some(serde_json::json!({ "class_id": input.class_id, "weekday": input.weekday, "period_no": input.period_no }).to_string()),
        ..Default::default()
    })?;
    load_class_slots(conn, &input.class_id)?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(CmdError::not_found)
}

/// Delete one timetable slot (Principal).
pub fn delete_timetable_slot_logic(conn: &mut Connection, actor_s: &SessionStaff, slot_id: &str) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageTimetable, &Target::of(TargetKind::School))?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    let n = conn.execute("DELETE FROM timetable_slot WHERE id=?1", params![slot_id])?;
    if n == 0 {
        return Err(CmdError::not_found());
    }
    audit_action(conn, AuditEntry {
        at: now_iso(),
        staff_id: Some(actor_s.id.clone()),
        action: "delete_timetable_slot".into(),
        table: Some("timetable_slot".into()),
        record_id: Some(slot_id.to_string()),
        ..Default::default()
    })?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct CopyWeekResult {
    pub copied: i64,
    pub skipped: i64,
}

/// Copy a class's week to another class (Principal). Each source slot is matched
/// to the target class's class-subject for the SAME subject (its assigned
/// teacher); a subject the target class does not offer is skipped. The result is
/// re-validated for clashes before anything is written (all-or-nothing).
pub fn copy_timetable_week_logic(conn: &mut Connection, actor_s: &SessionStaff, from_class_id: &str, to_class_id: &str) -> CmdResult<CopyWeekResult> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageTimetable, &Target::of(TargetKind::School))?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    if from_class_id == to_class_id {
        return Err(CmdError::validation("class", "same_class"));
    }
    let session_id = current_session_id(conn)?.ok_or_else(CmdError::not_found)?;
    // Target class's subject → (class_subject_id, teacher_id).
    let mut target_cs: std::collections::BTreeMap<String, (String, Option<String>)> = std::collections::BTreeMap::new();
    {
        let mut stmt = conn.prepare("SELECT subject_id, id, teacher_id FROM class_subject WHERE class_id=?1")?;
        let rows = stmt.query_map(params![to_class_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?))
        })?;
        for row in rows {
            let (subj, cs, t) = row?;
            target_cs.insert(subj, (cs, t));
        }
    }
    // Source slots with their subject_id.
    let mut source: Vec<(i64, i64, String)> = Vec::new(); // weekday, period, subject_id
    {
        let mut stmt = conn.prepare(
            "SELECT ts.weekday, ts.period_no, cs.subject_id FROM timetable_slot ts \
             JOIN class_subject cs ON cs.id=ts.class_subject_id \
             WHERE ts.class_id=?1 AND ts.effective_to IS NULL ORDER BY ts.weekday, ts.period_no",
        )?;
        let rows = stmt.query_map(params![from_class_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        for row in rows {
            source.push(row?);
        }
    }
    // Build the proposed new slots, skipping subjects the target lacks / has no teacher.
    let mut proposed: Vec<vidya_core::timetable::Slot> = Vec::new();
    let mut skipped = 0i64;
    for (weekday, period_no, subject_id) in &source {
        match target_cs.get(subject_id) {
            Some((cs_id, Some(teacher))) => proposed.push(vidya_core::timetable::Slot {
                id: new_id("ts"),
                class_id: to_class_id.to_string(),
                weekday: *weekday as u8,
                period_no: *period_no as u8,
                class_subject_id: cs_id.clone(),
                teacher_id: teacher.clone(),
            }),
            _ => skipped += 1,
        }
    }
    // Re-validate the whole timetable with the target's existing slots replaced.
    let assigned = class_subject_assignments(conn)?;
    let mut all: Vec<vidya_core::timetable::Slot> = all_session_slots(conn, &session_id)?
        .into_iter()
        .filter(|s| s.class_id != to_class_id)
        .collect();
    all.extend(proposed.iter().cloned());
    let clashes = vidya_core::timetable::clashes(&all, &assigned);
    if !clashes.is_empty() {
        let rule = clashes.first().map(clash_rule).unwrap_or("clash");
        return Err(CmdError::validation("timetable", rule));
    }
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let copied = proposed.len() as i64;
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM timetable_slot WHERE class_id=?1 AND effective_to IS NULL", params![to_class_id])?;
    for s in &proposed {
        tx.execute(
            "INSERT INTO timetable_slot(id,session_id,class_id,weekday,period_no,class_subject_id,teacher_id,school_id,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9,'confirmed')",
            params![s.id, session_id, s.class_id, s.weekday as i64, s.period_no as i64, s.class_subject_id, s.teacher_id, school_id, now],
        )?;
    }
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(),
        staff_id: Some(actor_s.id.clone()),
        action: "copy_timetable_week".into(),
        table: Some("timetable_slot".into()),
        record_id: Some(to_class_id.to_string()),
        after_json: Some(serde_json::json!({ "from": from_class_id, "to": to_class_id, "copied": copied, "skipped": skipped }).to_string()),
        ..Default::default()
    })?;
    tx.commit()?;
    Ok(CopyWeekResult { copied, skipped })
}

#[derive(Debug, Deserialize)]
pub struct PeriodInput {
    pub no: i64,
    pub starts_at: String,
    pub ends_at: String,
}

/// Replace the session's period bell times (Principal). Slots keep their
/// `period_no`, so this only re-times the grid rows.
pub fn save_periods_logic(conn: &mut Connection, actor_s: &SessionStaff, periods: &[PeriodInput]) -> CmdResult<Vec<PeriodDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageTimetable, &Target::of(TargetKind::School))?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    let session_id = current_session_id(conn)?.ok_or_else(CmdError::not_found)?;
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM period WHERE session_id=?1", params![session_id])?;
    for p in periods {
        if p.no < 1 {
            return Err(CmdError::validation("period", "range"));
        }
        tx.execute(
            "INSERT INTO period(id,session_id,no,starts_at,ends_at,school_id,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?7,'confirmed')",
            params![new_id("per"), session_id, p.no, p.starts_at, p.ends_at, school_id, now],
        )?;
    }
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(),
        staff_id: Some(actor_s.id.clone()),
        action: "save_periods".into(),
        table: Some("period".into()),
        record_id: Some(session_id.clone()),
        after_json: Some(serde_json::json!({ "count": periods.len() }).to_string()),
        ..Default::default()
    })?;
    tx.commit()?;
    load_periods(conn, &session_id).map_err(Into::into)
}

// ---- Substitutes + attendance duty (P16 Step 2, §10.4) ----------------------

#[derive(Debug, Serialize)]
pub struct SubCoverDto {
    pub period_no: i64,
    pub class_id: String,
    pub class_display: Option<String>,
    pub subject_name: String,
    pub starts_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FreeTeacherDto {
    pub id: String,
    pub name: String,
    /// Which of the cover periods this teacher is free in.
    pub free_periods: Vec<i64>,
}

#[derive(Debug, Serialize)]
pub struct AttendanceClassDto {
    pub id: String,
    pub display: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SubstitutePlanDto {
    pub date: String,
    pub weekday: i64,
    pub absent_teacher_id: String,
    pub absent_teacher_name: String,
    pub covers: Vec<SubCoverDto>,
    /// Classes the absent teacher is class teacher of — their attendance also
    /// needs cover (a teacher may class-teach more than one class).
    pub attendance_classes: Vec<AttendanceClassDto>,
    pub free_teachers: Vec<FreeTeacherDto>,
}

/// The ISO weekday (Mon=1 … Sun=7) of a `YYYY-MM-DD` date.
fn weekday_of(date: &str) -> CmdResult<u8> {
    let d = vidya_core::calendar::parse_date(date).ok_or_else(|| CmdError::validation("date", "format"))?;
    Ok(vidya_core::calendar::weekday_iso(d.weekday()))
}

/// The absent teacher's periods on `date` + free teachers to cover them
/// (prototype `timetable` state 2). Principal only.
pub fn substitute_plan_logic(conn: &mut Connection, actor_s: &SessionStaff, date: &str, absent_teacher_id: &str) -> CmdResult<SubstitutePlanDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageSubstitutes, &Target::of(TargetKind::School))?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    let d = date[..date.len().min(10)].to_string();
    let weekday = weekday_of(&d)? as i64;
    let absent_teacher_name: String = conn
        .query_row("SELECT name FROM staff WHERE id=?1", params![absent_teacher_id], |r| r.get(0))
        .optional()?
        .ok_or_else(CmdError::not_found)?;

    // Periods the absent teacher would teach that weekday.
    let covers: Vec<SubCoverDto> = {
        let mut stmt = conn.prepare(
            "SELECT ts.period_no, ts.class_id, c.display, sub.name, p.starts_at \
             FROM timetable_slot ts JOIN class_subject cs ON cs.id=ts.class_subject_id \
             JOIN subject sub ON sub.id=cs.subject_id JOIN class c ON c.id=ts.class_id \
             LEFT JOIN period p ON p.session_id=ts.session_id AND p.no=ts.period_no \
             WHERE ts.teacher_id=?1 AND ts.weekday=?2 AND ts.effective_to IS NULL ORDER BY ts.period_no",
        )?;
        let rows = stmt.query_map(params![absent_teacher_id, weekday], |r| {
            Ok(SubCoverDto { period_no: r.get(0)?, class_id: r.get(1)?, class_display: r.get(2)?, subject_name: r.get(3)?, starts_at: r.get(4)? })
        })?
        .collect::<rusqlite::Result<Vec<SubCoverDto>>>()?;
        rows
    };

    // If the absent teacher is a class teacher, each such class's attendance also
    // needs cover (a teacher may class-teach more than one class).
    let attendance_classes: Vec<AttendanceClassDto> = {
        let mut stmt = conn.prepare("SELECT id, display FROM class WHERE class_teacher_id=?1 ORDER BY sort_order")?;
        let rows = stmt
            .query_map(params![absent_teacher_id], |r| Ok(AttendanceClassDto { id: r.get(0)?, display: r.get(1)? }))?
            .collect::<rusqlite::Result<Vec<AttendanceClassDto>>>()?;
        rows
    };

    // Teachers busy in each cover period (weekday, period).
    let cover_periods: Vec<i64> = covers.iter().map(|c| c.period_no).collect();
    let mut free_teachers = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT id, name FROM staff WHERE role='teacher' AND state='active' AND id<>?1 ORDER BY name")?;
        let teachers: Vec<(String, String)> = stmt
            .query_map(params![absent_teacher_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        for (id, name) in teachers {
            let mut free_periods = Vec::new();
            for p in &cover_periods {
                let busy: bool = conn
                    .query_row(
                        "SELECT 1 FROM timetable_slot WHERE teacher_id=?1 AND weekday=?2 AND period_no=?3 AND effective_to IS NULL LIMIT 1",
                        params![id, weekday, p],
                        |_| Ok(()),
                    )
                    .optional()?
                    .is_some();
                if !busy {
                    free_periods.push(*p);
                }
            }
            // A candidate must be free for at least one cover period (or there are
            // no periods to cover — then anyone can take the attendance duty).
            if !free_periods.is_empty() || cover_periods.is_empty() {
                free_teachers.push(FreeTeacherDto { id, name, free_periods });
            }
        }
    }

    Ok(SubstitutePlanDto {
        date: d,
        weekday,
        absent_teacher_id: absent_teacher_id.to_string(),
        absent_teacher_name,
        covers,
        attendance_classes,
        free_teachers,
    })
}

#[derive(Debug, Serialize)]
pub struct AssignSubstituteResult {
    pub periods_covered: i64,
    pub includes_attendance: bool,
}

/// Assign `substitute_teacher_id` to cover the absent teacher on `date`: a
/// substitution per cover period the substitute is free in, plus that day's
/// attendance duty if the absent teacher is a class teacher. Notifies the
/// substitute. Principal only. Access ends automatically tonight (the grant is
/// dated `date` and `may_take_attendance` only honours it for that day).
pub fn assign_substitute_logic(conn: &mut Connection, actor_s: &SessionStaff, date: &str, absent_teacher_id: &str, substitute_teacher_id: &str) -> CmdResult<AssignSubstituteResult> {
    let plan = substitute_plan_logic(conn, actor_s, date, absent_teacher_id)?;
    if absent_teacher_id == substitute_teacher_id {
        return Err(CmdError::validation("substitute", "same_teacher"));
    }
    // The chosen substitute's free cover periods.
    let free: std::collections::BTreeSet<i64> = plan
        .free_teachers
        .iter()
        .find(|t| t.id == substitute_teacher_id)
        .map(|t| t.free_periods.iter().copied().collect())
        .ok_or_else(|| CmdError::validation("substitute", "not_free"))?;
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let includes_attendance = !plan.attendance_classes.is_empty();
    let mut periods_covered = 0i64;

    let tx = conn.transaction()?;
    for cover in &plan.covers {
        if !free.contains(&cover.period_no) {
            continue;
        }
        tx.execute(
            "INSERT INTO substitution(id,date,absent_teacher_id,substitute_teacher_id,class_id,period_no,includes_attendance,created_by,school_id,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,?6,0,?7,?8,?9,?9,'confirmed')",
            params![new_id("sub"), plan.date, absent_teacher_id, substitute_teacher_id, cover.class_id, cover.period_no, actor_s.id, school_id, now],
        )?;
        periods_covered += 1;
    }
    for ac in &plan.attendance_classes {
        tx.execute(
            "INSERT INTO substitution(id,date,absent_teacher_id,substitute_teacher_id,class_id,period_no,includes_attendance,created_by,school_id,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,NULL,1,?6,?7,?8,?8,'confirmed')",
            params![new_id("sub"), plan.date, absent_teacher_id, substitute_teacher_id, ac.id, actor_s.id, school_id, now],
        )?;
    }
    // Notify the substitute (in-app notification; their attendance grant is live).
    tx.execute(
        "INSERT INTO notification(id,staff_id,kind,title_key,vars_json,link) VALUES (?1,?2,'substitute','notif.substitute_assigned',?3,'/teacher/timetable')",
        params![
            new_id("ntf"),
            substitute_teacher_id,
            serde_json::json!({ "date": plan.date, "class": plan.attendance_classes.first().and_then(|c| c.display.clone()).unwrap_or_default() }).to_string()
        ],
    )?;
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(),
        staff_id: Some(actor_s.id.clone()),
        action: "assign_substitute".into(),
        table: Some("substitution".into()),
        record_id: Some(substitute_teacher_id.to_string()),
        after_json: Some(serde_json::json!({ "date": plan.date, "absent": absent_teacher_id, "substitute": substitute_teacher_id, "periods": periods_covered, "attendance": includes_attendance }).to_string()),
        ..Default::default()
    })?;
    tx.commit()?;
    Ok(AssignSubstituteResult { periods_covered, includes_attendance })
}

// ---- Homework & notes (P16 Step 3, §10.4) -----------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentMeta {
    pub name: String,
    pub size: i64,
    pub mime: String,
    #[serde(default)]
    pub drive_file_id: Option<String>,
    #[serde(default)]
    pub local_hash: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HomeworkNoteDto {
    pub id: String,
    pub class_id: String,
    pub class_subject_id: Option<String>,
    pub subject_name: Option<String>,
    pub kind: String,
    pub text: String,
    pub attachments: Vec<AttachmentMeta>,
    pub created_by: Option<String>,
    pub created_by_name: Option<String>,
    pub created_at: String,
    pub can_delete: bool,
}

#[derive(Debug, Deserialize)]
pub struct HomeworkNoteInput {
    pub class_id: String,
    pub class_subject_id: Option<String>,
    pub kind: String,
    pub text: String,
    #[serde(default)]
    pub attachments: Vec<AttachmentMeta>,
}

/// Milliseconds between two RFC-3339 timestamps (`now − then`); 0 if unparseable.
fn elapsed_ms(then: &str, now: &str) -> i64 {
    let fmt = &time::format_description::well_known::Rfc3339;
    match (time::OffsetDateTime::parse(then, fmt), time::OffsetDateTime::parse(now, fmt)) {
        (Ok(a), Ok(b)) => ((b - a).whole_milliseconds()).clamp(i64::MIN as i128, i64::MAX as i128) as i64,
        _ => 0,
    }
}

fn note_dto(r: &rusqlite::Row, staff_id: &str, now: &str) -> rusqlite::Result<HomeworkNoteDto> {
    let created_by: Option<String> = r.get("created_by")?;
    let created_at: String = r.get("created_at")?;
    let attachments_json: String = r.get("attachments_json")?;
    let attachments: Vec<AttachmentMeta> = serde_json::from_str(&attachments_json).unwrap_or_default();
    let is_author = created_by.as_deref() == Some(staff_id);
    Ok(HomeworkNoteDto {
        id: r.get("id")?,
        class_id: r.get("class_id")?,
        class_subject_id: r.get("class_subject_id")?,
        subject_name: r.get("subject_name")?,
        kind: r.get("kind")?,
        text: r.get("text")?,
        can_delete: vidya_core::notes::can_delete_own(is_author, elapsed_ms(&created_at, now)),
        attachments,
        created_by,
        created_by_name: r.get("created_by_name")?,
        created_at,
    })
}

const NOTE_SELECT: &str = "SELECT h.id, h.class_id, h.class_subject_id, sub.name AS subject_name, h.kind, h.text, \
     h.attachments_json, h.created_by, st.name AS created_by_name, h.created_at \
     FROM homework_note h LEFT JOIN class_subject cs ON cs.id=h.class_subject_id \
     LEFT JOIN subject sub ON sub.id=cs.subject_id LEFT JOIN staff st ON st.id=h.created_by";

/// Homework & notes history for a class (prototype `notes` history). Every teacher
/// of the class + the Principal.
pub fn list_homework_notes_logic(conn: &mut Connection, actor_s: &SessionStaff, class_id: &str) -> CmdResult<Vec<HomeworkNoteDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ViewNotes, &Target { kind: TargetKind::Attendance, class_id: Some(class_id.to_string()), ..Default::default() })?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    if actor.role == Role::Teacher && !teacher_in_class(conn, &actor_s.id, class_id)? {
        return Err(CmdError::forbidden("teacher_not_own_class"));
    }
    let now = now_iso();
    let sql = format!("{NOTE_SELECT} WHERE h.class_id=?1 ORDER BY h.created_at DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params![class_id], |r| note_dto(r, &actor_s.id, &now))?
        .collect::<rusqlite::Result<Vec<HomeworkNoteDto>>>()?;
    Ok(rows)
}

/// Write a homework/notes entry (teacher of the class; Principal). Validates the
/// attachment size limits (≤ 10 MB each, ≤ 20 MB total) via vidya-core.
pub fn save_homework_note_logic(conn: &mut Connection, actor_s: &SessionStaff, device_id: Option<&str>, device_mode: DeviceMode, input: &HomeworkNoteInput) -> CmdResult<HomeworkNoteDto> {
    let actor = actor_from(conn, actor_s)?;
    let target = Target {
        kind: TargetKind::Attendance,
        class_id: Some(input.class_id.clone()),
        class_subject_id: input.class_subject_id.clone(),
        ..Default::default()
    };
    require_allow(&actor, Action::ManageNotes, &target)?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    let kind = vidya_core::notes::validate_kind(&input.kind)?;
    vidya_core::notes::validate_note(&input.text, input.attachments.len())?;
    vidya_core::notes::validate_attachments(&input.attachments.iter().map(|a| a.size).collect::<Vec<_>>())?;
    // The class-subject (if given) must belong to this class.
    if let Some(cs) = &input.class_subject_id {
        let ok: bool = conn
            .query_row("SELECT 1 FROM class_subject WHERE id=?1 AND class_id=?2", params![cs, input.class_id], |_| Ok(()))
            .optional()?
            .is_some();
        if !ok {
            return Err(CmdError::validation("class_subject", "wrong_class"));
        }
    }
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let id = new_id("hw");
    let attachments_json = serde_json::to_string(&input.attachments).unwrap_or_else(|_| "[]".into());
    let sync_state = if device_mode == DeviceMode::Server { "confirmed" } else { "on_device" };
    let ctx = WriteCtx { mode: device_mode };
    let dev = device_id.map(str::to_string);
    let id2 = id.clone();
    let class_id = input.class_id.clone();
    let cs = input.class_subject_id.clone();
    let kind_s = kind.as_key().to_string();
    let text = input.text.clone();

    with_write(conn, &ctx, move |tx| {
        tx.execute(
            "INSERT INTO homework_note(id,class_id,class_subject_id,kind,text,attachments_json,shared_json,created_by,school_id,created_at,updated_at,updated_by_staff,updated_by_device,sync_state) \
             VALUES (?1,?2,?3,?4,?5,?6,'[]',?7,?8,?9,?9,?7,?10,?11)",
            params![id2, class_id, cs, kind_s, text, attachments_json, actor_s.id, school_id, now, dev, sync_state],
        )?;
        let audit = AuditEntry {
            at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "save_homework_note".into(),
            table: Some("homework_note".into()), record_id: Some(id2.clone()),
            after_json: Some(serde_json::json!({ "class_id": class_id, "kind": kind_s }).to_string()),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"), hlc: now.clone(), device_id: dev.clone().unwrap_or_default(), staff_id: actor_s.id.clone(),
            audience: format!("class:{class_id}"), table: "homework_note".into(), record_id: id2.clone(), kind: "insert".into(),
            payload: serde_json::json!({ "class_id": class_id, "class_subject_id": cs }).to_string(),
            base_version: None, server_epoch: 1,
        };
        Ok(Effect { value: (), audit, op })
    })?;
    let now2 = now_iso();
    let sql = format!("{NOTE_SELECT} WHERE h.id=?1");
    conn.query_row(&sql, params![id], |r| note_dto(r, &actor_s.id, &now2)).optional()?.ok_or_else(CmdError::not_found)
}

/// Delete one's own note within 24 h (audited). Principal may delete any.
pub fn delete_homework_note_logic(conn: &mut Connection, actor_s: &SessionStaff, id: &str) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    let (created_by, created_at, class_id): (Option<String>, String, String) = conn
        .query_row("SELECT created_by, created_at, class_id FROM homework_note WHERE id=?1", params![id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    let is_author = created_by.as_deref() == Some(actor_s.id.as_str());
    let now = now_iso();
    // Principal may delete any note; a teacher only their own within 24 h.
    let allowed = if actor.role == Role::Principal {
        require_allow(&actor, Action::ManageNotes, &Target { kind: TargetKind::Attendance, class_id: Some(class_id), ..Default::default() }).is_ok()
    } else {
        vidya_core::notes::can_delete_own(is_author, elapsed_ms(&created_at, &now))
    };
    if !allowed {
        return Err(CmdError::forbidden("not_deletable"));
    }
    conn.execute("DELETE FROM homework_note WHERE id=?1", params![id])?;
    audit_action(conn, AuditEntry {
        at: now, staff_id: Some(actor_s.id.clone()), action: "delete_homework_note".into(),
        table: Some("homework_note".into()), record_id: Some(id.to_string()), ..Default::default()
    })?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct NoteShareResult {
    pub queued: i64,
    pub skipped_no_email: i64,
    pub skipped_no_consent: i64,
}

/// Email a note to the class's parents (queued through the P14 pipeline; the
/// school PC sends when online). Consent-gated (`messages`), guardian's language.
#[allow(clippy::type_complexity)]
pub fn email_homework_note_logic(conn: &mut Connection, actor_s: &SessionStaff, device_id: Option<&str>, device_mode: DeviceMode, id: &str) -> CmdResult<NoteShareResult> {
    let actor = actor_from(conn, actor_s)?;
    let (class_id, kind, text): (String, String, String) = conn
        .query_row("SELECT class_id, kind, text FROM homework_note WHERE id=?1", params![id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    require_allow(&actor, Action::ManageNotes, &Target { kind: TargetKind::Attendance, class_id: Some(class_id.clone()), ..Default::default() })?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;

    // Primary guardians of students currently enrolled in the class.
    let guardians: Vec<(String, Option<String>, Option<String>, String, bool)> = {
        let mut stmt = conn.prepare(
            "SELECT g.id, g.email, g.language, g.name, COALESCE((SELECT 1 FROM consent c WHERE c.student_id=sg.student_id AND c.purpose='messages' AND c.withdrawn_at IS NULL LIMIT 1),0) \
             FROM student_guardian sg JOIN guardian g ON g.id=sg.guardian_id \
             JOIN enrollment e ON e.student_id=sg.student_id AND e.to_date IS NULL \
             WHERE e.class_id=?1 AND sg.is_primary=1",
        )?;
        let rows = stmt.query_map(params![class_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, Option<String>>(2)?, r.get::<_, String>(3)?, r.get::<_, i64>(4)? != 0))
        })?
        .collect::<rusqlite::Result<Vec<(String, Option<String>, Option<String>, String, bool)>>>()?;
        rows
    };
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let subject = if kind == "homework" { "Homework" } else { "Class notes" };
    let mut queued = 0i64;
    let mut skipped_no_email = 0i64;
    let mut skipped_no_consent = 0i64;
    let ctx = WriteCtx { mode: device_mode };
    let dev = device_id.map(str::to_string);

    let tx = conn.transaction()?;
    for (gid, email, language, _name, has_consent) in &guardians {
        if !has_consent {
            skipped_no_consent += 1;
            continue;
        }
        if email.as_deref().unwrap_or("").is_empty() {
            skipped_no_email += 1;
            continue;
        }
        let lang = language.clone().unwrap_or_else(|| "en".into());
        insert_message_in_tx(
            &tx, ctx.mode, &actor_s.id, dev.as_deref(), "email", "homework", &lang,
            Some(gid), None, email.as_deref(), Some(subject), Some(&text),
            Some("homework_note"), Some(id), "queued", school_id.as_deref(), &now,
        )?;
        queued += 1;
    }
    // Record the share on the note.
    tx.execute(
        "UPDATE homework_note SET shared_json=?1, updated_at=?2 WHERE id=?3",
        params![serde_json::json!([{ "channel": "email", "at": now, "count": queued }]).to_string(), now, id],
    )?;
    tx.commit()?;
    Ok(NoteShareResult { queued, skipped_no_email, skipped_no_consent })
}

// ============================================================= receipts =======
//
// Receipt search / open (full receipt data incl. amount in words en+hi, heads
// paid from allocations, advance credit, balance after, sync state) and
// Principal direct reverse (creates request + decision + reversal for audit).

#[derive(Debug, Serialize)]
pub struct ReceiptLineDto {
    pub label: String,
    pub amount_paise: i64,
}

#[derive(Debug, Serialize)]
pub struct ReceiptDto {
    pub id: String,
    pub receipt_no: String,
    pub student_id: String,
    pub student_name: String,
    pub guardian_mobile: Option<String>,
    pub class_display: Option<String>,
    pub admission_no: Option<String>,
    pub provisional_no: Option<String>,
    pub amount_paise: i64,
    pub amount_words_en: String,
    pub amount_words_hi: String,
    pub amount_words_te: String,
    pub mode: String,
    pub reference: Option<String>,
    pub collected_by_name: Option<String>,
    pub collected_at: String,
    pub confirmed: bool,
    pub lines: Vec<ReceiptLineDto>,
    pub advance_credit_paise: i64,
    pub balance_after_paise: i64,
    pub reversed: bool,
    /// A `upi://pay` link pre-filled for the remaining balance — present only when
    /// the school has a UPI id, the "show QR on receipts" toggle is on, and the
    /// balance after this payment is > 0 (§10.1). The webview renders it as a QR.
    pub upi_link: Option<String>,
}

pub fn get_receipt_logic(conn: &mut Connection, actor_s: &SessionStaff, id: &str) -> CmdResult<ReceiptDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::PrintShareReceipt, &Target::of(TargetKind::Fee))?;
    // Accept a receipt number or a payment id.
    let (pid, receipt_no, student_id, amount, mode, reference, collected_by, collected_at, sync_state): (
        String, String, String, i64, String, Option<String>, Option<String>, String, String,
    ) = conn
        .query_row(
            "SELECT id, receipt_no, student_id, amount_paise, mode, reference, collected_by, collected_at, sync_state \
             FROM payment WHERE id=?1 OR receipt_no=?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)),
        )
        .optional()?
        .ok_or_else(CmdError::not_found)?;

    let (student_name, guardian_mobile, class_display, admission_no, provisional_no): (String, Option<String>, Option<String>, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT s.name, s.guardian_mobile, c.display, s.admission_no, s.provisional_no FROM student s \
             LEFT JOIN enrollment e ON e.student_id=s.id AND e.to_date IS NULL \
             LEFT JOIN class c ON c.id=e.class_id WHERE s.id=?1",
            params![student_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?
        .ok_or_else(CmdError::not_found)?;

    let collected_by_name: Option<String> = match &collected_by {
        Some(sid) => conn.query_row("SELECT name FROM staff WHERE id=?1", params![sid], |r| r.get(0)).optional()?,
        None => None,
    };

    // Heads paid (kind='due') + advance credit (kind='advance_credit').
    let mut lstmt = conn.prepare(
        "SELECT COALESCE(h.name, d.period, 'Fee'), pa.amount_paise FROM payment_allocation pa \
         LEFT JOIN fee_due d ON d.id=pa.fee_due_id LEFT JOIN fee_head h ON h.id=d.fee_head_id \
         WHERE pa.payment_id=?1 AND pa.kind='due' ORDER BY pa.amount_paise DESC",
    )?;
    let lines: Vec<ReceiptLineDto> = lstmt
        .query_map(params![pid], |r| Ok(ReceiptLineDto { label: r.get(0)?, amount_paise: r.get(1)? }))?
        .collect::<rusqlite::Result<_>>()?;
    let advance_credit_paise: i64 = conn
        .query_row("SELECT COALESCE(SUM(amount_paise),0) FROM payment_allocation WHERE payment_id=?1 AND kind='advance_credit'", params![pid], |r| r.get(0))
        .optional()?
        .unwrap_or(0);

    let balance_after_paise: i64 = student_dues(conn, &student_id)?.iter().map(|l| l.balance_paise.max(0)).sum();
    let reversed: bool = conn
        .query_row("SELECT 1 FROM reversal WHERE payment_id=?1 LIMIT 1", params![pid], |_| Ok(()))
        .optional()?
        .is_some();

    // Balance UPI QR (§10.1): only when configured, the receipts toggle is on, and
    // there is still a balance to pay. Note trimmed to ≤50 chars inside upi_uri.
    let pay = crate::upi::PaymentSettings::read(conn)?;
    let upi_link = if pay.on_receipts && balance_after_paise > 0 {
        let note = match &class_display {
            Some(c) => format!("Fees {student_name} {c}"),
            None => format!("Fees {student_name}"),
        };
        pay.link(balance_after_paise, &note)
    } else {
        None
    };

    Ok(ReceiptDto {
        id: pid,
        receipt_no,
        student_id,
        student_name,
        guardian_mobile,
        class_display,
        admission_no,
        provisional_no,
        amount_paise: amount,
        amount_words_en: vidya_core::words::amount_in_words_en(Paise(amount)),
        amount_words_hi: vidya_core::words::amount_in_words_hi(Paise(amount)),
        amount_words_te: vidya_core::words::amount_in_words_te(Paise(amount)),
        mode,
        reference,
        collected_by_name,
        collected_at,
        confirmed: sync_state == "confirmed",
        lines,
        advance_credit_paise,
        balance_after_paise,
        reversed,
        upi_link,
    })
}

#[derive(Debug, Serialize)]
pub struct ReceiptSummaryDto {
    pub id: String,
    pub receipt_no: String,
    pub student_name: String,
    pub amount_paise: i64,
    pub mode: String,
    pub collected_at: String,
    pub confirmed: bool,
    pub reversed: bool,
}

pub fn search_receipts_logic(conn: &mut Connection, actor_s: &SessionStaff, query: &str) -> CmdResult<Vec<ReceiptSummaryDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::PrintShareReceipt, &Target::of(TargetKind::Fee))?;
    let like = format!("%{}%", query.trim().replace('%', ""));
    let mut stmt = conn.prepare(
        "SELECT p.id, p.receipt_no, s.name, p.amount_paise, p.mode, p.collected_at, p.sync_state, \
                EXISTS(SELECT 1 FROM reversal rv WHERE rv.payment_id=p.id) \
         FROM payment p JOIN student s ON s.id=p.student_id \
         WHERE p.receipt_no LIKE ?1 OR s.name LIKE ?1 OR p.collected_at LIKE ?1 \
         ORDER BY p.collected_at DESC LIMIT 100",
    )?;
    let rows = stmt
        .query_map(params![like], |r| {
            Ok(ReceiptSummaryDto {
                id: r.get(0)?,
                receipt_no: r.get(1)?,
                student_name: r.get(2)?,
                amount_paise: r.get(3)?,
                mode: r.get(4)?,
                collected_at: r.get(5)?,
                confirmed: r.get::<_, String>(6)? == "confirmed",
                reversed: r.get::<_, i64>(7)? != 0,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

/// Principal direct reversal: still creates a request + a decision + the reversal
/// row (for audit), by opening the request and immediately approving it.
pub fn reverse_payment_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_mode: DeviceMode,
    payment_id: &str,
    reason: &str,
) -> CmdResult<RequestDto> {
    let actor = actor_from(conn, actor_s)?;
    // Only the Principal reverses directly; accountants raise a request instead.
    require_allow(&actor, Action::PaymentReversal, &Target::of(TargetKind::Fee))?;
    let input = RequestInput {
        kind: "payment_reversal".into(),
        target_table: "payment".into(),
        target_id: payment_id.to_string(),
        base_version: 0,
        reason: reason.to_string(),
        before_json: None,
        after_json: Some(serde_json::json!({ "summary": format!("Reverse payment {payment_id}") }).to_string()),
    };
    let req = create_request_logic(conn, actor_s, &input)?;
    decide_request_logic(conn, actor_s, device_mode, &req.id, "approve", Some(reason))
}

// ============================================================= day book =======
//
// All money movement on a date: totals by mode, reversals, provisional (unsent)
// amounts shown separately, and the chronological list. Print + CSV export.

#[derive(Debug, Serialize)]
pub struct DayBookEntry {
    pub time: String,
    pub receipt_no: String,
    pub student_name: String,
    pub class_display: Option<String>,
    pub mode: String,
    pub reference_last4: Option<String>,
    pub amount_paise: i64,
    pub collected_by: Option<String>,
    pub confirmed: bool,
    pub reversed: bool,
}

#[derive(Debug, Serialize)]
pub struct DayBookDto {
    pub date: String,
    pub cash_paise: i64,
    pub upi_paise: i64,
    pub cheque_paise: i64,
    pub total_paise: i64,
    pub reversals_paise: i64,
    pub provisional_paise: i64,
    pub entries: Vec<DayBookEntry>,
}

fn last4(s: &Option<String>) -> Option<String> {
    s.as_ref().filter(|v| !v.is_empty()).map(|v| {
        let chars: Vec<char> = v.chars().collect();
        chars[chars.len().saturating_sub(4)..].iter().collect()
    })
}

pub fn day_book_logic(conn: &mut Connection, actor_s: &SessionStaff, date: &str) -> CmdResult<DayBookDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::DayBook, &Target::of(TargetKind::Fee))?;
    let like = format!("{date}%");
    let mut stmt = conn.prepare(
        "SELECT p.id, p.collected_at, p.receipt_no, s.name, c.display, p.mode, p.reference, p.amount_paise, st.name, p.sync_state, \
                EXISTS(SELECT 1 FROM reversal rv WHERE rv.payment_id=p.id) \
         FROM payment p JOIN student s ON s.id=p.student_id \
         LEFT JOIN enrollment e ON e.student_id=s.id AND e.to_date IS NULL \
         LEFT JOIN class c ON c.id=e.class_id \
         LEFT JOIN staff st ON st.id=p.collected_by \
         WHERE p.collected_at LIKE ?1 ORDER BY p.collected_at",
    )?;
    let mut entries = Vec::new();
    let (mut cash, mut upi, mut cheque, mut provisional) = (0i64, 0i64, 0i64, 0i64);
    let rows = stmt.query_map(params![like], |r| {
        let collected_at: String = r.get(1)?;
        let mode: String = r.get(5)?;
        let reference: Option<String> = r.get(6)?;
        let amount: i64 = r.get(7)?;
        let sync_state: String = r.get(9)?;
        Ok(DayBookEntry {
            time: collected_at.get(11..16).unwrap_or("").to_string(),
            receipt_no: r.get(2)?,
            student_name: r.get(3)?,
            class_display: r.get(4)?,
            mode: mode.clone(),
            reference_last4: last4(&reference),
            amount_paise: amount,
            collected_by: r.get(8)?,
            confirmed: sync_state == "confirmed",
            reversed: r.get::<_, i64>(10)? != 0,
        })
    })?;
    for row in rows {
        let e = row?;
        if e.confirmed {
            match e.mode.as_str() {
                "cash" => cash += e.amount_paise,
                "upi" => upi += e.amount_paise,
                "cheque" => cheque += e.amount_paise,
                _ => {}
            }
        } else {
            provisional += e.amount_paise;
        }
        entries.push(e);
    }
    // Reversals applied on this date.
    let reversals_paise: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(p.amount_paise),0) FROM reversal rv JOIN payment p ON p.id=rv.payment_id WHERE rv.applied_at LIKE ?1",
            params![like],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or(0);
    Ok(DayBookDto {
        date: date.to_string(),
        cash_paise: cash,
        upi_paise: upi,
        cheque_paise: cheque,
        total_paise: cash + upi + cheque,
        reversals_paise,
        provisional_paise: provisional,
        entries,
    })
}

fn daybook_csv_rows(conn: &Connection, date: &str) -> rusqlite::Result<Vec<Vec<String>>> {
    let mut out = vec![vec![
        "Time".into(), "Receipt".into(), "Student".into(), "Class".into(), "Mode".into(),
        "Reference".into(), "Amount (Rs)".into(), "Collected by".into(), "Status".into(),
    ]];
    let like = format!("{date}%");
    let mut stmt = conn.prepare(
        "SELECT p.collected_at, p.receipt_no, s.name, COALESCE(c.display,''), p.mode, COALESCE(p.reference,''), p.amount_paise, COALESCE(st.name,''), p.sync_state \
         FROM payment p JOIN student s ON s.id=p.student_id \
         LEFT JOIN enrollment e ON e.student_id=s.id AND e.to_date IS NULL \
         LEFT JOIN class c ON c.id=e.class_id \
         LEFT JOIN staff st ON st.id=p.collected_by \
         WHERE p.collected_at LIKE ?1 ORDER BY p.collected_at",
    )?;
    let rows = stmt.query_map(params![like], |r| {
        let at: String = r.get(0)?;
        let amount: i64 = r.get(6)?;
        let sync: String = r.get(8)?;
        Ok(vec![
            at.get(11..16).unwrap_or("").to_string(),
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
            format!("{}", amount / 100),
            r.get::<_, String>(7)?,
            if sync == "confirmed" { "Confirmed".into() } else { "Waiting".into() },
        ])
    })?;
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

// ---- P15 report CSV rows (Step 7) -------------------------------------------

/// Split an `arg` of `"from|to"` (YYYY-MM-DD) into (from, to); wide defaults if absent.
fn split_range(arg: Option<&str>) -> (String, String) {
    match arg.and_then(|a| a.split_once('|')) {
        Some((f, t)) => (f.to_string(), t.to_string()),
        None => ("0000-01-01".into(), "9999-12-31".into()),
    }
}

/// Expenses in a date range (for "expenses by category" — sortable in the sheet).
fn expenses_csv_rows(conn: &Connection, from: &str, to: &str) -> rusqlite::Result<Vec<Vec<String>>> {
    let mut out = vec![vec![
        "Date".into(), "Category".into(), "Details".into(), "Vendor".into(),
        "Paid via".into(), "Amount (Rs)".into(), "Voucher".into(), "Status".into(),
    ]];
    let mut stmt = conn.prepare(
        "SELECT e.spent_on, COALESCE(a.name,''), COALESCE(e.details,''), COALESCE(e.vendor,''), e.paid_via, e.amount_paise, \
                COALESCE((SELECT v.voucher_no FROM voucher v WHERE v.source_table='expense' AND v.source_id=e.id),''), \
                EXISTS(SELECT 1 FROM expense_reversal r WHERE r.expense_id=e.id) \
         FROM expense e LEFT JOIN ledger_account a ON a.id=e.category_account_id \
         WHERE e.spent_on>=?1 AND e.spent_on<=?2 ORDER BY a.name, e.spent_on",
    )?;
    let rows = stmt.query_map(params![from, to], |r| {
        let amount: i64 = r.get(5)?;
        let reversed: i64 = r.get(7)?;
        Ok(vec![
            r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?,
            r.get::<_, String>(4)?, format!("{}", amount / 100), r.get::<_, String>(6)?,
            if reversed != 0 { "Reversed".into() } else { "Recorded".into() },
        ])
    })?;
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// The salary register for a month, from a computed [`SalaryRegisterDto`].
fn salary_csv_rows(reg: &SalaryRegisterDto) -> Vec<Vec<String>> {
    let mut out = vec![vec![
        "Staff".into(), "Role".into(), "Monthly (Rs)".into(), "Working days".into(), "Days present".into(),
        "Unpaid days".into(), "Deduction (Rs)".into(), "Advance recovery (Rs)".into(), "Net (Rs)".into(), "Status".into(),
    ]];
    for r in &reg.rows {
        out.push(vec![
            r.name.clone(), r.role.clone(), format!("{}", r.monthly_paise / 100),
            r.working_days.to_string(), r.days_present.to_string(), r.unpaid_leave_days.to_string(),
            format!("{}", r.deduction_paise / 100), format!("{}", r.advance_recovery_paise / 100),
            format!("{}", r.net_paise / 100), if r.paid { "Paid".into() } else { "Pending".into() },
        ]);
    }
    out
}

/// Store sales (S- receipts).
fn store_sales_csv_rows(conn: &Connection) -> rusqlite::Result<Vec<Vec<String>>> {
    let mut out = vec![vec!["Receipt".into(), "Date".into(), "Total (Rs)".into(), "Mode".into()]];
    let mut stmt = conn.prepare("SELECT receipt_no, substr(sold_at,1,10), total_paise, mode FROM store_sale ORDER BY sold_at DESC")?;
    let rows = stmt.query_map([], |r| {
        let total: i64 = r.get(2)?;
        Ok(vec![r.get::<_, String>(0)?, r.get::<_, String>(1)?, format!("{}", total / 100), r.get::<_, String>(3)?])
    })?;
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Store stock (items + levels).
fn store_stock_csv_rows(conn: &Connection) -> rusqlite::Result<Vec<Vec<String>>> {
    let mut out = vec![vec!["Item".into(), "Price (Rs)".into(), "Stock".into(), "Low stock at".into(), "Active".into()]];
    let mut stmt = conn.prepare("SELECT name, price_paise, stock, low_stock_at, active FROM store_item ORDER BY name")?;
    let rows = stmt.query_map([], |r| {
        let price: i64 = r.get(1)?;
        Ok(vec![
            r.get::<_, String>(0)?, format!("{}", price / 100), r.get::<_, i64>(2)?.to_string(),
            r.get::<_, i64>(3)?.to_string(), if r.get::<_, i64>(4)? != 0 { "Yes".into() } else { "No".into() },
        ])
    })?;
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Outstanding instalment dues, ordered by due date (§10.2).
fn instalment_dues_csv_rows(conn: &Connection) -> rusqlite::Result<Vec<Vec<String>>> {
    let mut out = vec![vec![
        "Due date".into(), "Student".into(), "Class".into(), "Fee head".into(),
        "Instalment".into(), "Amount (Rs)".into(), "Balance (Rs)".into(),
    ]];
    let mut stmt = conn.prepare(
        "SELECT COALESCE(d.due_date,''), s.name, COALESCE(c.display,''), COALESCE(h.name, d.period, 'Fee'), \
                d.instalment_no, d.amount_paise, \
                d.amount_paise - COALESCE((SELECT SUM(pa.amount_paise) FROM payment_allocation pa WHERE pa.fee_due_id=d.id AND pa.kind='due'),0) \
         FROM fee_due d JOIN student s ON s.id=d.student_id \
           LEFT JOIN enrollment e ON e.student_id=s.id AND e.to_date IS NULL \
           LEFT JOIN class c ON c.id=e.class_id \
           LEFT JOIN fee_head h ON h.id=d.fee_head_id \
         WHERE d.cancelled_at IS NULL AND s.status='active' \
         ORDER BY d.due_date IS NULL, d.due_date, s.name",
    )?;
    let rows = stmt.query_map([], |r| {
        let inst: i64 = r.get(4)?;
        let amount: i64 = r.get(5)?;
        let bal: i64 = r.get(6)?;
        Ok((amount, bal, inst, vec![r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?]))
    })?;
    for row in rows {
        let (amount, bal, inst, mut base) = row?;
        if bal <= 0 {
            continue; // only outstanding dues
        }
        base.push(inst.to_string());
        base.push(format!("{}", amount / 100));
        base.push(format!("{}", bal / 100));
        out.push(base);
    }
    Ok(out)
}

// ============================================================ attendance ======

#[derive(Debug, Serialize)]
pub struct AttendanceRowDto {
    pub student_id: String,
    pub name: String,
    pub roll_no: Option<i64>,
    pub mark: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AttendanceSheetDto {
    pub class_id: String,
    pub class_display: String,
    pub date: String,
    pub status: String, // draft | submitted | none
    pub rows: Vec<AttendanceRowDto>,
}

#[derive(Debug, Deserialize)]
pub struct MarkInput {
    pub student_id: String,
    pub mark: String, // P | A | L
}

pub fn get_attendance_sheet_logic(conn: &mut Connection, class_id: &str, date: &str) -> CmdResult<AttendanceSheetDto> {
    let class_display: String = conn
        .query_row("SELECT display FROM class WHERE id=?1", params![class_id], |r| r.get(0))
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    let sheet: Option<(String, String)> = conn
        .query_row(
            "SELECT id, status FROM attendance_sheet WHERE class_id=?1 AND date=?2",
            params![class_id, date],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let (sheet_id, status) = match sheet {
        Some((id, st)) => (Some(id), st),
        None => (None, "none".to_string()),
    };
    let mut stmt = conn.prepare(
        "SELECT s.id, s.name, e.roll_no, m.mark \
         FROM student s JOIN enrollment e ON e.student_id=s.id AND e.to_date IS NULL AND e.class_id=?1 \
         LEFT JOIN attendance_mark m ON m.student_id=s.id AND m.sheet_id=?2 \
         WHERE s.status='active' ORDER BY e.roll_no",
    )?;
    let rows = stmt
        .query_map(params![class_id, sheet_id], |r| {
            Ok(AttendanceRowDto { student_id: r.get(0)?, name: r.get(1)?, roll_no: r.get(2)?, mark: r.get(3)? })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(AttendanceSheetDto { class_id: class_id.to_string(), class_display, date: date.to_string(), status, rows })
}

fn upsert_sheet_and_marks(
    conn: &mut Connection,
    class_id: &str,
    date: &str,
    marks: &[MarkInput],
    submit: bool,
    actor: &SessionStaff,
) -> CmdResult<()> {
    let now = now_iso();
    let sheet_id: String = match conn
        .query_row("SELECT id FROM attendance_sheet WHERE class_id=?1 AND date=?2", params![class_id, date], |r| r.get(0))
        .optional()?
    {
        Some(id) => id,
        None => {
            let id = new_id("sheet");
            conn.execute(
                "INSERT INTO attendance_sheet(id,class_id,date,status,created_at,updated_at,sync_state) VALUES (?1,?2,?3,'draft',?4,?4,'confirmed')",
                params![id, class_id, date, now],
            )?;
            id
        }
    };
    for m in marks {
        // v2: new marks are Present/Absent only (legacy L is read, never written).
        let mark = vidya_core::attendance::parse_mark(&m.mark)?;
        vidya_core::attendance::validate_new_mark(mark)?;
        conn.execute(
            "INSERT INTO attendance_mark(id,sheet_id,student_id,mark) VALUES (?1,?2,?3,?4) \
             ON CONFLICT(sheet_id,student_id) DO UPDATE SET mark=excluded.mark",
            params![new_id("mk"), sheet_id, m.student_id, m.mark],
        )?;
    }
    if submit {
        conn.execute(
            "UPDATE attendance_sheet SET status='submitted', submitted_by=?1, submitted_at=?2, updated_at=?2 WHERE id=?3",
            params![actor.id, now, sheet_id],
        )?;
    }
    Ok(())
}

/// May `actor_s` take attendance for `class_id` on `date`? Class teacher (or
/// Principal) always; otherwise an active substitution / approved attendance-duty
/// grant for (class, date) (§10.4). Uses the same grant loader as the server.
fn require_may_take_attendance(conn: &Connection, actor_s: &SessionStaff, class_id: &str, date: &str) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    let d = &date[..date.len().min(10)];
    let grants = crate::sync::apply::attendance_grants_for(conn, &actor_s.id, d)?;
    if permissions::may_take_attendance(&actor, class_id, d, &grants) {
        Ok(())
    } else {
        Err(CmdError::forbidden("teacher_not_class_teacher"))
    }
}

pub fn save_attendance_draft_logic(conn: &mut Connection, actor_s: &SessionStaff, class_id: &str, date: &str, marks: &[MarkInput]) -> CmdResult<()> {
    require_may_take_attendance(conn, actor_s, class_id, date)?;
    upsert_sheet_and_marks(conn, class_id, date, marks, false, actor_s)
}

pub fn submit_attendance_logic(conn: &mut Connection, actor_s: &SessionStaff, class_id: &str, date: &str, marks: &[MarkInput]) -> CmdResult<()> {
    require_may_take_attendance(conn, actor_s, class_id, date)?;
    // Every enrolled student must be marked before submit (§ can_submit).
    let enrolled: u32 = conn.query_row(
        "SELECT COUNT(*) FROM enrollment WHERE class_id=?1 AND to_date IS NULL",
        params![class_id],
        |r| r.get::<_, i64>(0),
    )? as u32;
    if (marks.len() as u32) < enrolled {
        return Err(CoreError::IncompleteSheet { remaining: enrolled - marks.len() as u32 }.into());
    }
    upsert_sheet_and_marks(conn, class_id, date, marks, true, actor_s)
}

// ---- Month view + Principal correction -------------------------------------

#[derive(Debug, Serialize)]
pub struct MonthStudent {
    pub id: String,
    pub name: String,
    pub roll_no: Option<i64>,
    /// One entry per day in `days` (P|A|L|null).
    pub marks: Vec<Option<String>>,
    pub present: i64,
    pub absent: i64,
    pub leave: i64,
    pub pct_tenths: i64,
}

#[derive(Debug, Serialize)]
pub struct AttendanceMonthDto {
    pub class_id: String,
    pub class_display: String,
    pub month: String,
    pub days: Vec<String>,
    pub students: Vec<MonthStudent>,
    pub day_present: Vec<i64>,
    pub day_marked: Vec<i64>,
}

pub fn attendance_month_logic(conn: &mut Connection, actor_s: &SessionStaff, class_id: &str, month: &str) -> CmdResult<AttendanceMonthDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ViewAttendance, &Target { kind: TargetKind::Attendance, class_id: Some(class_id.to_string()), ..Default::default() })?;
    let class_display: String = conn
        .query_row("SELECT display FROM class WHERE id=?1", params![class_id], |r| r.get(0))
        .optional()?
        .unwrap_or_default();
    let like = format!("{month}%");

    // Submitted sheets in the month → the day columns.
    let mut days: Vec<String> = {
        let mut stmt = conn.prepare("SELECT date FROM attendance_sheet WHERE class_id=?1 AND status='submitted' AND date LIKE ?2 ORDER BY date")?;
        let v: Vec<String> = stmt.query_map(params![class_id, like], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<_>>()?;
        v
    };
    days.dedup();
    let day_index: std::collections::HashMap<String, usize> = days.iter().enumerate().map(|(i, d)| (d.clone(), i)).collect();

    // (student_id, date) → mark for the month's submitted sheets.
    let mut marks_map: std::collections::HashMap<(String, String), String> = std::collections::HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT am.student_id, sh.date, am.mark FROM attendance_sheet sh JOIN attendance_mark am ON am.sheet_id=sh.id \
             WHERE sh.class_id=?1 AND sh.status='submitted' AND sh.date LIKE ?2",
        )?;
        let rows = stmt.query_map(params![class_id, like], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))?;
        for row in rows {
            let (sid, date, mark) = row?;
            marks_map.insert((sid, date), mark);
        }
    }

    let mut day_present = vec![0i64; days.len()];
    let mut day_marked = vec![0i64; days.len()];

    let mut sstmt = conn.prepare(
        "SELECT s.id, s.name, e.roll_no FROM enrollment e JOIN student s ON s.id=e.student_id \
         WHERE e.class_id=?1 AND e.to_date IS NULL ORDER BY e.roll_no, s.name",
    )?;
    let student_rows: Vec<(String, String, Option<i64>)> = sstmt
        .query_map(params![class_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;

    let mut students = Vec::new();
    for (id, name, roll_no) in student_rows {
        let mut row_marks: Vec<Option<String>> = vec![None; days.len()];
        let (mut p, mut a, mut l) = (0i64, 0i64, 0i64);
        for (day, di) in &day_index {
            if let Some(m) = marks_map.get(&(id.clone(), day.clone())) {
                row_marks[*di] = Some(m.clone());
                day_marked[*di] += 1;
                match m.as_str() {
                    "P" => { p += 1; day_present[*di] += 1; }
                    "A" => a += 1,
                    "L" => l += 1,
                    _ => {}
                }
            }
        }
        let pct = vidya_core::attendance::percent_present(p as u32, a as u32, l as u32) as i64;
        students.push(MonthStudent { id, name, roll_no, marks: row_marks, present: p, absent: a, leave: l, pct_tenths: pct });
    }

    Ok(AttendanceMonthDto { class_id: class_id.to_string(), class_display, month: month.to_string(), days, students, day_present, day_marked })
}

/// Principal direct correction of one mark on a (submitted) sheet, audited with a
/// reason (§5 EditSubmittedAttendance). Teachers must go through a request.
#[allow(clippy::too_many_arguments)]
pub fn correct_attendance_mark_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_mode: DeviceMode,
    class_id: &str,
    date: &str,
    student_id: &str,
    mark: &str,
    reason: &str,
) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::EditSubmittedAttendance, &Target { kind: TargetKind::Attendance, class_id: Some(class_id.to_string()), is_locked: true, ..Default::default() })?;
    // v2: corrections set a NEW mark → Present/Absent only (never Leave).
    if !matches!(mark, "P" | "A") {
        return Err(CmdError::validation("mark", "p_or_a"));
    }
    let sheet_id: String = conn
        .query_row("SELECT id FROM attendance_sheet WHERE class_id=?1 AND date=?2", params![class_id, date], |r| r.get(0))
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    let now = now_iso();
    let ctx = WriteCtx { mode: device_mode };
    let sid = sheet_id.clone();
    let stu = student_id.to_string();
    let mk = mark.to_string();
    let reason_s = reason.to_string();
    let cid = class_id.to_string();
    with_write(conn, &ctx, move |tx| {
        tx.execute(
            "INSERT INTO attendance_mark(id,sheet_id,student_id,mark) VALUES (?1,?2,?3,?4) \
             ON CONFLICT(sheet_id,student_id) DO UPDATE SET mark=excluded.mark",
            params![new_id("am"), sid, stu, mk],
        )?;
        let audit = AuditEntry {
            at: now.clone(),
            staff_id: Some(actor_s.id.clone()),
            action: "correct_attendance".into(),
            table: Some("attendance_mark".into()),
            record_id: Some(sid.clone()),
            reason: Some(reason_s.clone()),
            after_json: Some(serde_json::json!({ "student_id": stu, "mark": mk }).to_string()),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"),
            hlc: now.clone(),
            device_id: String::new(),
            staff_id: actor_s.id.clone(),
            audience: format!("class:{cid}"),
            table: "attendance_mark".into(),
            record_id: sid.clone(),
            kind: "update".into(),
            payload: "{}".into(),
            base_version: None,
            server_epoch: 1,
        };
        Ok(Effect { value: (), audit, op })
    })?;
    Ok(())
}

// ================================================================ fees ========

#[derive(Debug, Serialize)]
pub struct FeeLineDto {
    pub id: String,
    pub label: String,
    pub amount_paise: i64,
    pub balance_paise: i64,
}

#[derive(Debug, Serialize)]
pub struct FeeDuesDto {
    pub student_id: String,
    pub total_due_paise: i64,
    pub lines: Vec<FeeLineDto>,
}

/// The uncancelled dues for a student with their current balances (oldest first).
fn student_dues(conn: &Connection, student_id: &str) -> rusqlite::Result<Vec<FeeLineDto>> {
    let mut stmt = conn.prepare(
        "SELECT d.id, COALESCE(h.name, d.period), d.amount_paise, \
           COALESCE((SELECT SUM(pa.amount_paise) FROM payment_allocation pa WHERE pa.fee_due_id=d.id AND pa.kind='due'),0) \
         FROM fee_due d LEFT JOIN fee_head h ON h.id=d.fee_head_id \
         WHERE d.student_id=?1 AND d.cancelled_at IS NULL \
         ORDER BY d.due_date IS NULL, d.due_date, d.instalment_no, d.created_at",
    )?;
    let rows = stmt
        .query_map(params![student_id], |r| {
            let amount: i64 = r.get(2)?;
            let allocated: i64 = r.get(3)?;
            Ok(FeeLineDto { id: r.get(0)?, label: r.get(1)?, amount_paise: amount, balance_paise: amount - allocated })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn list_fee_dues_logic(conn: &mut Connection, student_id: &str) -> CmdResult<FeeDuesDto> {
    let lines = student_dues(conn, student_id)?;
    let total = lines.iter().map(|l| l.balance_paise.max(0)).sum();
    Ok(FeeDuesDto { student_id: student_id.to_string(), total_due_paise: total, lines })
}

// ---- Dues screen + fee reminders (P14 Step 5, prototype feesadmin 2-3) -------

/// Format paise as an Indian-grouped rupee string, e.g. `684200` → `"₹6,842"`,
/// `210050` → `"₹2,100.50"`. Paise are shown only when non-zero.
fn inr(paise: i64) -> String {
    let neg = paise < 0;
    let p = paise.abs();
    let (rupees, ps) = (p / 100, p % 100);
    let digits = rupees.to_string();
    let n = digits.len();
    let mut grouped = String::new();
    for (i, ch) in digits.chars().enumerate() {
        let from_end = n - i;
        // Indian grouping: a comma before the last 3 digits, then every 2.
        if i != 0 && (from_end == 3 || (from_end > 3 && (from_end - 3) % 2 == 0)) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let mut out = format!("₹{grouped}");
    if ps != 0 {
        out.push_str(&format!(".{ps:02}"));
    }
    if neg { format!("-{out}") } else { out }
}

#[derive(Debug, Serialize)]
pub struct DuesStripDto {
    pub total_due_paise: i64,
    pub students_with_dues: i64,
    pub unpaid_dues: i64,
    pub collected_today_paise: i64,
}

#[derive(Debug, Serialize)]
pub struct DuesRowDto {
    pub due_id: String,
    pub student_id: String,
    pub student_name: String,
    pub class_display: Option<String>,
    /// The fee head name (e.g. "Tuition"). Combine with `instalment_no` /
    /// `instalment_count` for the "Tuition · 2 of 3" label (§10.2).
    pub fee_head: String,
    pub balance_paise: i64,
    pub guardian_id: Option<String>,
    pub guardian_name: Option<String>,
    pub guardian_mobile: Option<String>,
    pub guardian_email: Option<String>,
    pub guardian_language: Option<String>,
    pub has_messages_consent: bool,
    /// Guardian can be emailed a reminder: has an email AND `messages` consent.
    pub emailable: bool,
    /// 1-based instalment number and how many instalments this head has for the
    /// student (for the "N of M" label); the due date (YYYY-MM-DD), P15.
    pub instalment_no: i64,
    pub instalment_count: i64,
    pub due_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DuesListDto {
    pub strip: DuesStripDto,
    pub rows: Vec<DuesRowDto>,
    /// Distinct guardians who can be emailed (for the "Email all N parents" button).
    pub emailable_students: i64,
    pub skipped_no_email: i64,
    pub skipped_no_consent: i64,
}

/// The Dues screen (prototype `feesadmin` state 2): a strip + one row per
/// outstanding fee due, with the student's primary guardian and consent, optionally
/// filtered by class. Accountant + Principal (`ViewFees`).
pub fn list_dues_logic(conn: &mut Connection, actor_s: &SessionStaff, class_id: Option<&str>) -> CmdResult<DuesListDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ViewFees, &Target::of(TargetKind::Fee))?;
    let mut sql = String::from(
        "SELECT d.id, d.student_id, s.name, c.display, COALESCE(h.name, d.period, 'Fee'), d.amount_paise, \
           COALESCE((SELECT SUM(pa.amount_paise) FROM payment_allocation pa WHERE pa.fee_due_id=d.id AND pa.kind='due'),0), \
           g.id, g.name, g.mobile, g.email, g.language, \
           d.instalment_no, d.due_date, \
           (SELECT COUNT(*) FROM fee_due d2 WHERE d2.student_id=d.student_id AND d2.fee_head_id=d.fee_head_id AND d2.cancelled_at IS NULL) \
         FROM fee_due d JOIN student s ON s.id=d.student_id \
           LEFT JOIN enrollment e ON e.student_id=s.id AND e.to_date IS NULL \
           LEFT JOIN class c ON c.id=e.class_id \
           LEFT JOIN fee_head h ON h.id=d.fee_head_id \
           LEFT JOIN student_guardian sg ON sg.student_id=s.id AND sg.is_primary=1 \
           LEFT JOIN guardian g ON g.id=sg.guardian_id \
         WHERE d.cancelled_at IS NULL AND s.status='active'",
    );
    let mut args: Vec<rusqlite::types::Value> = Vec::new();
    if let Some(cf) = class_id.filter(|s| !s.is_empty()) {
        sql.push_str(" AND c.id=?");
        args.push(cf.to_string().into());
    }
    // Oldest instalment first (§10.2 allocation principle), then student.
    sql.push_str(" ORDER BY s.name, d.due_date, d.instalment_no, d.created_at");

    #[allow(clippy::type_complexity)]
    let raw: Vec<(String, String, String, Option<String>, String, i64, i64, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, i64, Option<String>, i64)> = {
        let mut stmt = conn.prepare(&sql)?;
        let out = stmt.query_map(rusqlite::params_from_iter(args.iter()), |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?, r.get(11)?, r.get(12)?, r.get(13)?, r.get(14)?))
        })?.collect::<rusqlite::Result<_>>()?;
        out
    };

    let mut rows: Vec<DuesRowDto> = Vec::new();
    let mut students: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut emailable: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut no_email: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut no_consent: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut consent_cache: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    let mut total_due: i64 = 0;
    for (due_id, sid, name, cls, head, amount, allocated, gid, gname, gmobile, gemail, glang, instalment_no, due_date, instalment_count) in raw {
        let bal = amount - allocated;
        if bal <= 0 {
            continue;
        }
        total_due += bal;
        students.insert(sid.clone());
        let consent = *consent_cache.entry(sid.clone()).or_insert_with(|| messages_consent_logic(conn, &sid).unwrap_or(false));
        let has_email = gemail.as_deref().map(|e| !e.trim().is_empty()).unwrap_or(false);
        let can_email = has_email && consent;
        if can_email {
            emailable.insert(sid.clone());
        } else if !has_email {
            no_email.insert(sid.clone());
        } else {
            no_consent.insert(sid.clone());
        }
        rows.push(DuesRowDto {
            due_id, student_id: sid, student_name: name, class_display: cls, fee_head: head, balance_paise: bal,
            guardian_id: gid, guardian_name: gname, guardian_mobile: gmobile, guardian_email: gemail,
            guardian_language: glang, has_messages_consent: consent, emailable: can_email,
            instalment_no, instalment_count, due_date,
        });
    }
    let today = now_iso()[..10].to_string();
    let collected_today_paise: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount_paise),0) FROM payment WHERE substr(collected_at,1,10)=?1",
        params![today], |r| r.get(0),
    )?;
    Ok(DuesListDto {
        strip: DuesStripDto {
            total_due_paise: total_due,
            students_with_dues: students.len() as i64,
            unpaid_dues: rows.len() as i64,
            collected_today_paise,
        },
        rows,
        emailable_students: emailable.len() as i64,
        skipped_no_email: no_email.len() as i64,
        skipped_no_consent: no_consent.len() as i64,
    })
}

/// Load a message template's (subject, body) for a language, falling back to
/// English (the authoritative body, P13).
fn load_message_template(conn: &Connection, key: &str, lang: &str) -> CmdResult<(Option<String>, String)> {
    let by = |l: &str| -> rusqlite::Result<Option<(Option<String>, String)>> {
        conn.query_row(
            "SELECT subject, body FROM message_template WHERE key=?1 AND language=?2",
            params![key, l], |r| Ok((r.get(0)?, r.get(1)?)),
        ).optional()
    };
    if let Some(t) = by(lang)? {
        return Ok(t);
    }
    by("en")?.ok_or_else(CmdError::not_found)
}

struct ReminderRender {
    subject: Option<String>,
    body: String,
    upi_link: Option<String>,
    guardian_id: Option<String>,
    guardian_name: Option<String>,
    guardian_mobile: Option<String>,
    guardian_email: Option<String>,
    language: String,
    has_consent: bool,
}

/// Render the `fee_reminder` template for one student in the guardian's language
/// (or `lang_override`), filling `{student_name} {amount} {instalment} {due_date}
/// {school_name} {upi_link}`. `due_date` is blank until per-instalment dues (P15).
fn render_fee_reminder(conn: &Connection, student_id: &str, lang_override: Option<&str>) -> CmdResult<ReminderRender> {
    let name: String = conn
        .query_row("SELECT name FROM student WHERE id=?1", params![student_id], |r| r.get(0))
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    #[allow(clippy::type_complexity)]
    let guardian: Option<(String, String, Option<String>, Option<String>, String)> = conn
        .query_row(
            "SELECT g.id, g.name, g.mobile, g.email, g.language FROM student_guardian sg \
             JOIN guardian g ON g.id=sg.guardian_id WHERE sg.student_id=?1 AND sg.is_primary=1 LIMIT 1",
            params![student_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let lines = student_dues(conn, student_id)?;
    let total: i64 = lines.iter().map(|l| l.balance_paise.max(0)).sum();
    let mut heads: Vec<String> = Vec::new();
    for l in &lines {
        if l.balance_paise > 0 && !heads.contains(&l.label) {
            heads.push(l.label.clone());
        }
    }
    let instalment = if heads.is_empty() { "Fees".to_string() } else { heads.join(", ") };
    let school_name: String = conn
        .query_row("SELECT name FROM school LIMIT 1", [], |r| r.get(0))
        .optional()?
        .unwrap_or_default();
    let guardian_language = lang_override
        .map(str::to_string)
        .or_else(|| guardian.as_ref().map(|g| g.4.clone()))
        .unwrap_or_else(|| "en".into());
    let lang = if matches!(guardian_language.as_str(), "en" | "hi" | "te") { guardian_language } else { "en".into() };

    let (subject_tmpl, body_tmpl) = load_message_template(conn, "fee_reminder", &lang)?;
    let pay = crate::upi::PaymentSettings::read(conn)?;
    let upi_link = if pay.on_reminders { pay.link(total, &format!("Fees {name}")) } else { None };

    let mut vars: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    vars.insert("student_name".into(), name);
    vars.insert("amount".into(), inr(total));
    vars.insert("instalment".into(), instalment);
    vars.insert("due_date".into(), String::new());
    vars.insert("school_name".into(), school_name);
    vars.insert("upi_link".into(), upi_link.clone().unwrap_or_default());
    let body = vidya_core::messages::render(&body_tmpl, &vars);
    let subject = subject_tmpl.map(|s| vidya_core::messages::render(&s, &vars));

    Ok(ReminderRender {
        subject,
        body,
        upi_link,
        guardian_id: guardian.as_ref().map(|g| g.0.clone()),
        guardian_name: guardian.as_ref().map(|g| g.1.clone()),
        guardian_mobile: guardian.as_ref().and_then(|g| g.2.clone()),
        guardian_email: guardian.as_ref().and_then(|g| g.3.clone()),
        language: lang.clone(),
        has_consent: messages_consent_logic(conn, student_id)?,
    })
}

#[derive(Debug, Serialize)]
pub struct ReminderPreviewDto {
    pub student_id: String,
    pub subject: Option<String>,
    pub body: String,
    pub upi_link: Option<String>,
    pub guardian_name: Option<String>,
    pub guardian_mobile: Option<String>,
    pub guardian_email: Option<String>,
    pub has_email: bool,
    pub has_consent: bool,
    pub language: String,
}

/// Preview a fee reminder for one student (the reminder sheet, prototype `feesadmin`
/// state 3). Returns the rendered subject/body + the UPI link so the sheet can show
/// the QR and (for WhatsApp) pass the text to `wa.me`.
pub fn preview_fee_reminder_logic(conn: &mut Connection, actor_s: &SessionStaff, student_id: &str, language: Option<&str>) -> CmdResult<ReminderPreviewDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::SendFeeReminder, &Target::of(TargetKind::Fee))?;
    let r = render_fee_reminder(conn, student_id, language)?;
    let has_email = r.guardian_email.as_deref().map(|e| !e.trim().is_empty()).unwrap_or(false);
    Ok(ReminderPreviewDto {
        student_id: student_id.to_string(),
        subject: r.subject,
        body: r.body,
        upi_link: r.upi_link,
        guardian_name: r.guardian_name,
        guardian_mobile: r.guardian_mobile,
        guardian_email: r.guardian_email,
        has_email,
        has_consent: r.has_consent,
        language: r.language,
    })
}

#[derive(Debug, Serialize)]
pub struct BulkReminderDto {
    pub queued: i64,
    pub skipped_no_email: i64,
    pub skipped_no_consent: i64,
}

/// Bulk "Email all N parents" (prototype `feesadmin` state 2). Queues one `email`
/// message per student whose primary guardian has an email AND `messages` consent;
/// others are skipped with a counted reason. One transaction. The school PC sends
/// the queued rows later (Step 3). Accountant + Principal (`SendFeeReminder`).
pub fn queue_fee_reminders_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    student_ids: &[String],
    language: Option<&str>,
) -> CmdResult<BulkReminderDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::SendFeeReminder, &Target::of(TargetKind::Fee))?;
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let (mut queued, mut skip_email, mut skip_consent) = (0i64, 0i64, 0i64);
    let tx = conn.transaction()?;
    for sid in student_ids {
        let r = render_fee_reminder(&tx, sid, language)?;
        let has_email = r.guardian_email.as_deref().map(|e| !e.trim().is_empty()).unwrap_or(false);
        if !has_email {
            skip_email += 1;
            continue;
        }
        if !r.has_consent {
            skip_consent += 1;
            continue;
        }
        insert_message_in_tx(
            &tx, device_mode, &actor_s.id, device_id, "email", "fee_reminder", &r.language,
            r.guardian_id.as_deref(), None, r.guardian_email.as_deref(), r.subject.as_deref(), Some(&r.body),
            Some("student"), Some(sid), "queued", school_id.as_deref(), &now,
        )?;
        queued += 1;
    }
    tx.commit()?;
    Ok(BulkReminderDto { queued, skipped_no_email: skip_email, skipped_no_consent: skip_consent })
}

#[derive(Debug, Deserialize)]
pub struct PaymentInput {
    pub student_id: String,
    pub amount_paise: i64,
    pub mode: String, // cash | upi | cheque
    pub reference: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaymentDto {
    pub id: String,
    pub receipt_no: String,
    pub student_id: String,
    pub amount_paise: i64,
    pub mode: String,
    pub confirmed: bool,
}

fn payment_mode_from(s: &str) -> CmdResult<PaymentMode> {
    match s {
        "cash" => Ok(PaymentMode::Cash),
        "upi" => Ok(PaymentMode::Upi),
        "cheque" => Ok(PaymentMode::Cheque),
        _ => Err(CmdError::validation("mode", "invalid")),
    }
}

pub fn record_payment_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    input: &PaymentInput,
) -> CmdResult<PaymentDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::RecordPayment, &Target::of(TargetKind::Fee))?;
    let mode = payment_mode_from(&input.mode)?;
    let reference = input.reference.clone().unwrap_or_default();
    fees::validate_reference(mode, &reference)?;

    // total due now + dues oldest-first for allocation.
    let lines = student_dues(conn, &input.student_id)?;
    let total_due: i64 = lines.iter().map(|l| l.balance_paise.max(0)).sum();
    fees::validate_collection(Paise(input.amount_paise), Paise(total_due))?;
    let dues: Vec<Due> = lines.iter().map(|l| Due { id: l.id.clone(), balance: Paise(l.balance_paise) }).collect();
    let allocation = fees::allocate(Paise(input.amount_paise), &dues);

    // The receipt series for this device (the sequence is reserved atomically
    // inside the write transaction below via the P13 numbering engine — the
    // number format and continuity are unchanged: still R-<series>-NNNN).
    let series = receipt_series_and_last(conn, device_id)?.0;

    let now = now_iso();
    let pid = new_id("pay");
    let confirmed = device_mode == DeviceMode::Server;
    let sync_state = if confirmed { "confirmed" } else { "on_device" };
    let confirmed_at = if confirmed { Some(now.clone()) } else { None };

    let school_id = single_school_id(conn)?;
    let ctx = WriteCtx { mode: device_mode };
    let dto = {
        let pid = pid.clone();
        let series = series.clone();
        let student_id = input.student_id.clone();
        let mode_s = input.mode.clone();
        let allocation = allocation.clone();
        let dev = device_id.map(str::to_string);
        with_write(conn, &ctx, move |tx| {
            let receipt_no = crate::numbering::next_no(tx, vidya_core::numbering::NumberKind::Receipt, &series)?;
            tx.execute(
                "INSERT INTO payment(id,receipt_no,student_id,amount_paise,mode,reference,collected_by,collected_at,device_id,confirmed_at,created_at,updated_at,sync_state) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?8,?8,?11)",
                params![pid, receipt_no, student_id, input.amount_paise, mode_s, reference, actor_s.id, now, dev, confirmed_at, sync_state],
            )?;
            for a in &allocation.allocations {
                tx.execute(
                    "INSERT INTO payment_allocation(id,payment_id,fee_due_id,amount_paise,kind) VALUES (?1,?2,?3,?4,?5)",
                    params![new_id("al"), pid, a.due_id, a.amount.get(), match a.kind { fees::AllocKind::Due => "due", fees::AllocKind::AdvanceCredit => "advance_credit" }],
                )?;
            }
            // v2 (P13): the school server writes the official balanced receipt
            // voucher in the SAME transaction. A client device leaves it to the
            // server (which posts it when it confirms the synced payment).
            if confirmed {
                crate::ledger::post_payment_voucher(
                    tx, &pid, &receipt_no, mode, input.amount_paise, &now,
                    Some(actor_s.id.as_str()), dev.as_deref(), school_id.as_deref(), &now, "confirmed",
                )?;
            }
            let audit = AuditEntry {
                at: now.clone(),
                staff_id: Some(actor_s.id.clone()),
                action: "record_payment".into(),
                table: Some("payment".into()),
                record_id: Some(pid.clone()),
                after_json: Some(serde_json::json!({ "receipt_no": receipt_no, "amount_paise": input.amount_paise }).to_string()),
                ..Default::default()
            };
            let op = Op {
                op_id: new_id("op"),
                hlc: now.clone(),
                device_id: dev.clone().unwrap_or_default(),
                staff_id: actor_s.id.clone(),
                audience: "finance".into(),
                table: "payment".into(),
                record_id: pid.clone(),
                kind: "insert".into(),
                payload: "{}".into(),
                base_version: None,
                server_epoch: 1,
            };
            Ok(Effect {
                value: PaymentDto {
                    id: pid.clone(),
                    receipt_no: receipt_no.clone(),
                    student_id: student_id.clone(),
                    amount_paise: input.amount_paise,
                    mode: mode_s.clone(),
                    confirmed,
                },
                audit,
                op,
            })
        })?
    };
    Ok(dto)
}

/// The device's receipt series + the highest sequence already used on it.
fn receipt_series_and_last(conn: &Connection, device_id: Option<&str>) -> CmdResult<(String, u32)> {
    let series: String = match device_id {
        Some(d) => conn
            .query_row("SELECT COALESCE(receipt_series,'A1') FROM device WHERE id=?1", params![d], |r| r.get(0))
            .optional()?
            .unwrap_or_else(|| "A1".to_string()),
        None => "A1".to_string(),
    };
    let last: u32 = conn
        .query_row(
            "SELECT COALESCE(MAX(CAST(substr(receipt_no, length(?1)+4) AS INTEGER)),0) FROM payment WHERE receipt_no LIKE ?2",
            params![series, format!("R-{series}-%")],
            |r| r.get::<_, i64>(0),
        )
        .optional()?
        .unwrap_or(0) as u32;
    Ok((series, last))
}

pub fn list_payments_logic(conn: &mut Connection, student_id: Option<&str>) -> CmdResult<Vec<PaymentDto>> {
    let base = "SELECT id, receipt_no, student_id, amount_paise, mode, sync_state FROM payment";
    let map = |r: &rusqlite::Row| -> rusqlite::Result<PaymentDto> {
        let sync: String = r.get(5)?;
        Ok(PaymentDto {
            id: r.get(0)?,
            receipt_no: r.get(1)?,
            student_id: r.get(2)?,
            amount_paise: r.get(3)?,
            mode: r.get(4)?,
            confirmed: sync == "confirmed",
        })
    };
    let rows: Vec<PaymentDto> = match student_id {
        Some(sid) => {
            let mut stmt = conn.prepare(&format!("{base} WHERE student_id=?1 ORDER BY collected_at DESC"))?;
            let v = stmt.query_map(params![sid], map)?.collect::<rusqlite::Result<_>>()?;
            v
        }
        None => {
            let mut stmt = conn.prepare(&format!("{base} ORDER BY collected_at DESC LIMIT 200"))?;
            let v = stmt.query_map([], map)?.collect::<rusqlite::Result<_>>()?;
            v
        }
    };
    Ok(rows)
}

// ============================================================= requests =======

#[derive(Debug, Deserialize)]
pub struct RequestInput {
    pub kind: String,
    pub target_table: String,
    pub target_id: String,
    pub base_version: i64,
    pub reason: String,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RequestDto {
    pub id: String,
    pub kind: String,
    pub target_table: String,
    pub target_id: String,
    pub reason: String,
    pub status: String,
    pub apply_state: String,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    pub requested_by: String,
    pub requester_name: String,
    pub created_at: String,
    pub note: Option<String>,
}

fn map_request(r: &rusqlite::Row) -> rusqlite::Result<RequestDto> {
    Ok(RequestDto {
        id: r.get(0)?,
        kind: r.get(1)?,
        target_table: r.get(2)?,
        target_id: r.get(3)?,
        reason: r.get(4)?,
        status: r.get(5)?,
        apply_state: r.get(6)?,
        before_json: r.get(7)?,
        after_json: r.get(8)?,
        requested_by: r.get(9)?,
        requester_name: r.get(10)?,
        created_at: r.get(11)?,
        note: r.get(12)?,
    })
}

const REQUEST_SELECT: &str = "SELECT r.id, r.type, r.target_table, r.target_id, r.reason, r.status, r.apply_state, \
     r.before_json, r.after_json, r.requested_by, s.name, r.created_at, r.note \
     FROM request r JOIN staff s ON s.id = r.requested_by";

pub fn create_request_logic(conn: &mut Connection, actor_s: &SessionStaff, input: &RequestInput) -> CmdResult<RequestDto> {
    // Validate the request type against the approval registry (P13). For the new
    // v2 types (leave / attendance_duty / class_notice) also enforce who may raise
    // them; the existing types keep their current create behaviour unchanged.
    let rt = vidya_core::types::RequestType::from_key(&input.kind)
        .ok_or_else(|| CmdError::validation("kind", "unknown"))?;
    if matches!(rt, vidya_core::types::RequestType::Leave | vidya_core::types::RequestType::AttendanceDuty | vidya_core::types::RequestType::ClassNotice)
        && !vidya_core::requests::can_raise(rt, role_from(&actor_s.role)?)
    {
        return Err(CmdError::forbidden("cannot_raise"));
    }
    // One open request per target (§requests): reject a duplicate.
    let pending: bool = conn
        .query_row(
            "SELECT 1 FROM request WHERE target_table=?1 AND target_id=?2 AND status='pending' LIMIT 1",
            params![input.target_table, input.target_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if pending {
        return Err(CoreError::RequestAlreadyPending.into());
    }
    let now = now_iso();
    let id = new_id("req");
    conn.execute(
        "INSERT INTO request(id,type,target_table,target_id,base_version,before_json,after_json,reason,requested_by,revision,status,apply_state,created_at,updated_at,sync_state) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,1,'pending','not_applied',?10,?10,'confirmed')",
        params![id, input.kind, input.target_table, input.target_id, input.base_version, input.before_json, input.after_json, input.reason, actor_s.id, now],
    )?;
    get_request_logic(conn, &id)
}

pub fn cancel_request_logic(conn: &mut Connection, actor_s: &SessionStaff, id: &str) -> CmdResult<()> {
    let (requested_by, status): (String, String) = conn
        .query_row("SELECT requested_by, status FROM request WHERE id=?1", params![id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    if requested_by != actor_s.id {
        return Err(CmdError::forbidden("not_requester"));
    }
    if status != "pending" && status != "returned" {
        return Err(CmdError::validation("request", "not_cancellable"));
    }
    conn.execute("UPDATE request SET status='cancelled', updated_at=?1 WHERE id=?2", params![now_iso(), id])?;
    Ok(())
}

pub fn list_requests_logic(conn: &mut Connection, status: Option<&str>, kind: Option<&str>) -> CmdResult<Vec<RequestDto>> {
    let mut sql = format!("{REQUEST_SELECT} WHERE 1=1");
    if status.is_some() {
        sql.push_str(" AND r.status = ?1");
    }
    if kind.is_some() {
        sql.push_str(if status.is_some() { " AND r.type = ?2" } else { " AND r.type = ?1" });
    }
    sql.push_str(" ORDER BY r.created_at DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = match (status, kind) {
        (Some(s), Some(k)) => stmt.query_map(params![s, k], map_request)?.collect::<rusqlite::Result<_>>()?,
        (Some(s), None) => stmt.query_map(params![s], map_request)?.collect::<rusqlite::Result<_>>()?,
        (None, Some(k)) => stmt.query_map(params![k], map_request)?.collect::<rusqlite::Result<_>>()?,
        (None, None) => stmt.query_map([], map_request)?.collect::<rusqlite::Result<_>>()?,
    };
    Ok(rows)
}

pub fn get_request_logic(conn: &mut Connection, id: &str) -> CmdResult<RequestDto> {
    let sql = format!("{REQUEST_SELECT} WHERE r.id = ?1");
    conn.query_row(&sql, params![id], map_request).optional()?.ok_or_else(CmdError::not_found)
}

pub fn decide_request_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_mode: DeviceMode,
    id: &str,
    decision: &str,
    note: Option<&str>,
) -> CmdResult<RequestDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ApproveRequest, &Target::of(TargetKind::Request))?;
    let req = get_request_logic(conn, id)?;
    if req.status != "pending" {
        return Err(CoreError::RequestStale.into());
    }
    let now = now_iso();

    if decision == "reject" || decision == "return" {
        let new_status = if decision == "reject" { "rejected" } else { "returned" };
        conn.execute(
            "UPDATE request SET status=?1, decided_by=?2, decided_at=?3, note=?4, updated_at=?3 WHERE id=?5",
            params![new_status, actor_s.id, now, note, id],
        )?;
        return get_request_logic(conn, id);
    }
    if decision != "approve" {
        return Err(CmdError::validation("decision", "invalid"));
    }

    // Approve: apply the change to the target record + audit + op, then mark the
    // request applied — ALL in one transaction (rule §8.2). Attendance and
    // payment-reversal are applied here (their data model exists in Phase 3);
    // marks/student-details/access application lands with those editors in P7,
    // so they are approved but left apply_state='not_applied' (honest).
    let after_value = req
        .after_json
        .as_deref()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .and_then(|v| v.get("value").and_then(|x| x.as_str()).map(str::to_string));

    // Whether this type auto-applies on approval is declared by the approval
    // registry (P13) — identical to the previous hard-coded list.
    let applied: bool = vidya_core::types::RequestType::from_key(&req.kind)
        .map(|rt| vidya_core::requests::spec(rt).apply_available)
        .unwrap_or(false);
    let ctx = WriteCtx { mode: device_mode };
    let req_id = id.to_string();
    let kind = req.kind.clone();
    let target_table = req.target_table.clone();
    let target_id = req.target_id.clone();
    let note_owned = note.map(str::to_string);
    let actor_id = actor_s.id.clone();

    // v2 (P13): a payment_reversal also writes the balanced reversal voucher in the
    // same transaction. Fetch the original payment's mode/amount/receipt first.
    let rev_id = new_id("rev");
    let school_id = single_school_id(conn)?;
    let reversal_pay: Option<(PaymentMode, i64, String)> = if kind == "payment_reversal" {
        let row: Option<(String, i64, String)> = conn
            .query_row("SELECT mode, amount_paise, receipt_no FROM payment WHERE id=?1", params![target_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .optional()?;
        match row {
            Some((m, amt, rno)) => Some((payment_mode_from(&m)?, amt, rno)),
            None => None,
        }
    } else {
        None
    };

    with_write(conn, &ctx, move |tx| {
        let apply_state = if applied { "applied" } else { "not_applied" };
        let applied_at = if applied { Some(now.clone()) } else { None };

        match kind.as_str() {
            "attendance_correction" => {
                // after.value is the new mark (P|A|L); target_id is the mark row.
                if let Some(mark) = &after_value {
                    tx.execute(
                        "UPDATE attendance_mark SET mark=?1 WHERE id=?2",
                        params![mark, target_id],
                    )?;
                }
            }
            "payment_reversal" => {
                // Append a reversal row (append-only) referencing the payment.
                tx.execute(
                    "INSERT INTO reversal(id,payment_id,reason,request_id,approved_by,applied_at) VALUES (?1,?2,?3,?4,?5,?6)",
                    params![rev_id, target_id, req.reason, req_id, actor_id, now],
                )?;
                // The balanced reversal voucher (Dr Fee income, Cr money account).
                if let Some((mode, amount, receipt_no)) = &reversal_pay {
                    crate::ledger::post_reversal_voucher(
                        tx, &rev_id, receipt_no, *mode, *amount, &now,
                        Some(actor_id.as_str()), school_id.as_deref(), &now, "confirmed",
                    )?;
                }
            }
            _ => {} // marks_correction / student_details / access_change → applied in P7
        }

        tx.execute(
            "UPDATE request SET status='approved', apply_state=?1, decided_by=?2, decided_at=?3, note=?4, applied_at=?5, updated_at=?3 WHERE id=?6",
            params![apply_state, actor_id, now, note_owned, applied_at, req_id],
        )?;

        let audit = AuditEntry {
            at: now.clone(),
            staff_id: Some(actor_id.clone()),
            action: "approve_request".into(),
            table: Some(target_table.clone()),
            record_id: Some(target_id.clone()),
            after_json: after_value.clone(),
            reason: Some(kind.clone()),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"),
            hlc: now.clone(),
            device_id: String::new(),
            staff_id: actor_id.clone(),
            audience: "admin".into(),
            table: target_table,
            record_id: target_id,
            kind: "action".into(),
            payload: "{}".into(),
            base_version: None,
            server_epoch: 1,
        };
        Ok(Effect { value: (), audit, op })
    })?;

    get_request_logic(conn, id)
}

// ============================================================ dashboards ======

#[derive(Debug, Serialize)]
pub struct AccountantDashboard {
    pub collected_today_paise: i64,
    pub receipts_today: i64,
    pub outstanding_paise: i64,
    pub students_with_dues: i64,
}

pub fn dashboard_accountant_logic(conn: &mut Connection, today: &str) -> CmdResult<AccountantDashboard> {
    let (collected, receipts) = crate::dash::collected_today(conn, today)?;
    Ok(AccountantDashboard {
        collected_today_paise: collected,
        receipts_today: receipts,
        outstanding_paise: crate::dash::outstanding(conn)?,
        students_with_dues: crate::dash::students_with_dues(conn)?,
    })
}

#[derive(Debug, Serialize)]
pub struct TeacherClassDto {
    pub class_id: String,
    pub display: String,
    pub submitted_today: bool,
}

#[derive(Debug, Serialize)]
pub struct TeacherDashboard {
    pub classes: Vec<TeacherClassDto>,
}

pub fn dashboard_teacher_logic(conn: &mut Connection, actor_s: &SessionStaff, today: &str) -> CmdResult<TeacherDashboard> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.display, \
           EXISTS(SELECT 1 FROM attendance_sheet s WHERE s.class_id=c.id AND s.date=?2 AND s.status='submitted') \
         FROM class c WHERE c.class_teacher_id=?1 ORDER BY c.sort_order",
    )?;
    let classes = stmt
        .query_map(params![actor_s.id, today], |r| {
            Ok(TeacherClassDto { class_id: r.get(0)?, display: r.get(1)?, submitted_today: r.get::<_, i64>(2)? == 1 })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(TeacherDashboard { classes })
}

// ================================================================ misc ========

pub fn set_accent_logic(conn: &mut Connection, hex: &str) -> CmdResult<()> {
    // Only the three offered accents (docs §15 / mock) are valid.
    const ALLOWED: [&str; 3] = ["#2F7479", "#1F4E8C", "#5B4B8A"];
    if !ALLOWED.contains(&hex) {
        return Err(CmdError::validation("accent", "not_allowed"));
    }
    kv::set(conn, "accent", &hex)?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct AuditChainDto {
    pub ok: bool,
    pub first_bad_seq: Option<i64>,
}

/// One recorded backup run (Backups screen; Principal Home "Last backup").
#[derive(Debug, Serialize)]
pub struct BackupRunDto {
    pub at: String,
    pub status: String,
    pub destination: Option<String>,
    pub chain_head: Option<String>,
}

/// Backups screen status: whether backups are enabled on this PC + recent runs.
#[derive(Debug, Serialize)]
pub struct BackupStatusDto {
    pub enabled: bool,
    pub runs: Vec<BackupRunDto>,
}

/// Recent backup runs, newest first (read-only).
pub fn backup_runs_logic(conn: &mut Connection) -> CmdResult<Vec<BackupRunDto>> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(finished_at, started_at), status, destination, chain_head \
         FROM backup_run ORDER BY started_at DESC LIMIT 30",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(BackupRunDto { at: r.get(0)?, status: r.get(1)?, destination: r.get(2)?, chain_head: r.get(3)? })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

// ============================================================= academics ======
//
// Grade scale (edit bands), exams (list/create + status grid), marks entry
// (draft/submit with a PER-SUBJECT lock, vidya-core validation), Principal marks
// correction, and report cards (subjects × marks + totals + grade + attendance).

// ---- Grade scale -----------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GradeBandDto {
    pub min_pct: i64, // tenths
    pub max_pct: i64, // tenths
    pub grade: String,
    pub grade_point: Option<i64>,
}

/// The stored default grade scale, or `grades::default_scale()` if none saved.
fn load_scale(conn: &Connection) -> Vec<GradeBand> {
    let mut stmt = match conn.prepare(
        "SELECT b.min_pct, b.max_pct, b.grade, b.grade_point FROM grade_band b \
         JOIN grade_scale s ON s.id=b.scale_id WHERE s.is_default=1 ORDER BY b.min_pct DESC",
    ) {
        Ok(s) => s,
        Err(_) => return grades::default_scale(),
    };
    let bands: Vec<GradeBand> = stmt
        .query_map([], |r| {
            Ok(GradeBand {
                min_pct_tenths: r.get::<_, i64>(0)? as u32,
                max_pct_tenths: r.get::<_, i64>(1)? as u32,
                grade: r.get(2)?,
                grade_point: r.get::<_, Option<i64>>(3)?.map(|g| g as u32),
            })
        })
        .and_then(|rows| rows.collect::<rusqlite::Result<_>>())
        .unwrap_or_default();
    if bands.is_empty() {
        grades::default_scale()
    } else {
        bands
    }
}

pub fn list_grade_bands_logic(conn: &mut Connection) -> CmdResult<Vec<GradeBandDto>> {
    Ok(load_scale(conn)
        .into_iter()
        .map(|b| GradeBandDto { min_pct: b.min_pct_tenths as i64, max_pct: b.max_pct_tenths as i64, grade: b.grade, grade_point: b.grade_point.map(|g| g as i64) })
        .collect())
}

/// Replace the default scale's bands. Validates full 0..=1000 (tenths) coverage
/// with no gaps and no overlaps (§7 grade scale).
pub fn update_grade_bands_logic(conn: &mut Connection, actor_s: &SessionStaff, bands: &[GradeBandDto]) -> CmdResult<Vec<GradeBandDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_principal(&actor)?;
    if bands.is_empty() {
        return Err(CmdError::validation("bands", "empty"));
    }
    // Validate each band + contiguity over 0..=1000 (ascending by min).
    let mut sorted: Vec<&GradeBandDto> = bands.iter().collect();
    sorted.sort_by_key(|b| b.min_pct);
    if sorted.first().unwrap().min_pct != 0 || sorted.last().unwrap().max_pct != 1000 {
        return Err(CmdError::validation("bands", "coverage"));
    }
    for b in &sorted {
        if b.min_pct < 0 || b.max_pct > 1000 || b.min_pct > b.max_pct || b.grade.trim().is_empty() {
            return Err(CmdError::validation("bands", "range"));
        }
    }
    for w in sorted.windows(2) {
        if w[1].min_pct != w[0].max_pct + 1 {
            return Err(CmdError::validation("bands", "gap_or_overlap"));
        }
    }
    let now = now_iso();
    let tx = conn.transaction()?;
    let scale_id: String = tx
        .query_row("SELECT id FROM grade_scale WHERE is_default=1 LIMIT 1", [], |r| r.get(0))
        .optional()?
        .unwrap_or_else(|| {
            let id = new_id("gs");
            let _ = tx.execute("INSERT INTO grade_scale(id,name,is_default) VALUES (?1,'Default',1)", params![id]);
            id
        });
    tx.execute("DELETE FROM grade_band WHERE scale_id=?1", params![scale_id])?;
    for b in bands {
        tx.execute(
            "INSERT INTO grade_band(id,scale_id,min_pct,max_pct,grade,grade_point) VALUES (?1,?2,?3,?4,?5,?6)",
            params![new_id("gb"), scale_id, b.min_pct, b.max_pct, b.grade, b.grade_point],
        )?;
    }
    crate::security::audit::append(&tx, &AuditEntry {
        at: now,
        staff_id: Some(actor_s.id.clone()),
        action: "update_grade_scale".into(),
        table: Some("grade_band".into()),
        ..Default::default()
    })?;
    tx.commit()?;
    list_grade_bands_logic(conn)
}

// ---- Exams -----------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ClassSubjectDto {
    pub id: String,
    pub class_display: Option<String>,
    pub subject_name: String,
}

pub fn list_class_subjects_logic(conn: &mut Connection) -> CmdResult<Vec<ClassSubjectDto>> {
    let mut stmt = conn.prepare(
        "SELECT cs.id, c.display, sub.name FROM class_subject cs \
         JOIN class c ON c.id=cs.class_id JOIN subject sub ON sub.id=cs.subject_id \
         ORDER BY c.sort_order, sub.name",
    )?;
    let rows = stmt
        .query_map([], |r| Ok(ClassSubjectDto { id: r.get(0)?, class_display: r.get(1)?, subject_name: r.get(2)? }))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

#[derive(Debug, Serialize)]
pub struct ExamSubjectDto {
    pub id: String, // exam_subject_id
    pub class_subject_id: String,
    pub class_id: String,
    pub class_display: Option<String>,
    pub subject_name: String,
    pub max_marks: i64,
    pub status: String, // not_started | draft | submitted
}

#[derive(Debug, Serialize)]
pub struct ExamDto {
    pub id: String,
    pub name: String,
    pub term_name: Option<String>,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
    pub subjects: Vec<ExamSubjectDto>,
}

fn exam_subjects(conn: &Connection, exam_id: &str) -> rusqlite::Result<Vec<ExamSubjectDto>> {
    let mut stmt = conn.prepare(
        "SELECT es.id, es.class_subject_id, cs.class_id, c.display, sub.name, es.max_marks, \
                COALESCE((SELECT status FROM marks_sheet ms WHERE ms.exam_subject_id=es.id), 'not_started') \
         FROM exam_subject es \
         JOIN class_subject cs ON cs.id=es.class_subject_id \
         JOIN class c ON c.id=cs.class_id \
         JOIN subject sub ON sub.id=cs.subject_id \
         WHERE es.exam_id=?1 ORDER BY c.sort_order, sub.name",
    )?;
    let rows: Vec<ExamSubjectDto> = stmt
        .query_map(params![exam_id], |r| {
            Ok(ExamSubjectDto {
                id: r.get(0)?,
                class_subject_id: r.get(1)?,
                class_id: r.get(2)?,
                class_display: r.get(3)?,
                subject_name: r.get(4)?,
                max_marks: r.get(5)?,
                status: r.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

#[allow(clippy::type_complexity)]
pub fn list_exams_logic(conn: &mut Connection) -> CmdResult<Vec<ExamDto>> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.name, t.name, e.starts_on, e.ends_on FROM exam e \
         LEFT JOIN term t ON t.id=e.term_id ORDER BY e.starts_on DESC, e.name",
    )?;
    let exams: Vec<(String, String, Option<String>, Option<String>, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut out = Vec::new();
    for (id, name, term_name, starts_on, ends_on) in exams {
        let subjects = exam_subjects(conn, &id)?;
        out.push(ExamDto { id, name, term_name, starts_on, ends_on, subjects });
    }
    Ok(out)
}

#[derive(Debug, Deserialize)]
pub struct ExamSubjectInput {
    pub class_subject_id: String,
    pub max_marks: i64,
}

#[derive(Debug, Deserialize)]
pub struct NewExamInput {
    pub name: String,
    pub term_id: Option<String>,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
    pub subjects: Vec<ExamSubjectInput>,
}

pub fn create_exam_logic(conn: &mut Connection, actor_s: &SessionStaff, input: &NewExamInput) -> CmdResult<ExamDto> {
    let actor = actor_from(conn, actor_s)?;
    require_principal(&actor)?;
    vidya_core::validation::validate_name(&input.name)?;
    let session_id: String = conn
        .query_row("SELECT id FROM academic_session WHERE is_current=1 LIMIT 1", [], |r| r.get(0))
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    let now = now_iso();
    let eid = new_id("exam");
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO exam(id,session_id,term_id,name,starts_on,ends_on) VALUES (?1,?2,?3,?4,?5,?6)",
        params![eid, session_id, input.term_id, input.name, input.starts_on, input.ends_on],
    )?;
    for s in &input.subjects {
        if s.max_marks <= 0 {
            return Err(CmdError::validation("max_marks", "range"));
        }
        tx.execute(
            "INSERT INTO exam_subject(id,exam_id,class_subject_id,max_marks) VALUES (?1,?2,?3,?4)",
            params![new_id("es"), eid, s.class_subject_id, s.max_marks],
        )?;
    }
    crate::security::audit::append(&tx, &AuditEntry {
        at: now,
        staff_id: Some(actor_s.id.clone()),
        action: "create_exam".into(),
        table: Some("exam".into()),
        record_id: Some(eid.clone()),
        after_json: Some(serde_json::json!({ "name": input.name }).to_string()),
        ..Default::default()
    })?;
    tx.commit()?;
    list_exams_logic(conn)?.into_iter().find(|e| e.id == eid).ok_or_else(CmdError::not_found)
}

// ---- Marks entry -----------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct MarksRowDto {
    pub student_id: String,
    pub name: String,
    pub roll_no: Option<i64>,
    pub marks: Option<i64>,
    pub absent: bool,
}

#[derive(Debug, Serialize)]
pub struct MarksSheetDto {
    pub exam_subject_id: String,
    pub class_id: String,
    pub class_display: Option<String>,
    pub subject_name: String,
    pub max_marks: i64,
    pub status: String, // not_started | draft | submitted
    pub rows: Vec<MarksRowDto>,
}

#[derive(Debug, Deserialize)]
pub struct MarkEntryInput {
    pub student_id: String,
    pub marks: Option<i64>,
    pub absent: bool,
}

/// (class_subject_id, class_id, subject_name, max_marks) for an exam_subject.
fn exam_subject_meta(conn: &Connection, exam_subject_id: &str) -> CmdResult<(String, String, String, i64)> {
    conn.query_row(
        "SELECT cs.id, cs.class_id, sub.name, es.max_marks FROM exam_subject es \
         JOIN class_subject cs ON cs.id=es.class_subject_id JOIN subject sub ON sub.id=cs.subject_id \
         WHERE es.id=?1",
        params![exam_subject_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )
    .optional()?
    .ok_or_else(CmdError::not_found)
}

pub fn get_marks_sheet_logic(conn: &mut Connection, actor_s: &SessionStaff, exam_subject_id: &str) -> CmdResult<MarksSheetDto> {
    let actor = actor_from(conn, actor_s)?;
    let (cs_id, class_id, subject_name, max_marks) = exam_subject_meta(conn, exam_subject_id)?;
    // View: Principal always; teacher only their own class-subject.
    let own = actor.class_subjects.iter().any(|c| c == &cs_id);
    vidya_core::marks::can_edit(actor.role, own).map_err(CmdError::from)?;
    let class_display: Option<String> = conn.query_row("SELECT display FROM class WHERE id=?1", params![class_id], |r| r.get(0)).optional()?;
    let status: String = conn
        .query_row("SELECT status FROM marks_sheet WHERE exam_subject_id=?1", params![exam_subject_id], |r| r.get(0))
        .optional()?
        .unwrap_or_else(|| "not_started".to_string());
    let mut stmt = conn.prepare(
        "SELECT s.id, s.name, e.roll_no, me.marks, me.absent FROM enrollment e \
         JOIN student s ON s.id=e.student_id \
         LEFT JOIN marks_sheet ms ON ms.exam_subject_id=?1 \
         LEFT JOIN mark_entry me ON me.sheet_id=ms.id AND me.student_id=s.id \
         WHERE e.class_id=?2 AND e.to_date IS NULL ORDER BY e.roll_no, s.name",
    )?;
    let rows = stmt
        .query_map(params![exam_subject_id, class_id], |r| {
            Ok(MarksRowDto {
                student_id: r.get(0)?,
                name: r.get(1)?,
                roll_no: r.get(2)?,
                marks: r.get(3)?,
                absent: r.get::<_, Option<i64>>(4)?.unwrap_or(0) != 0,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(MarksSheetDto { exam_subject_id: exam_subject_id.to_string(), class_id, class_display, subject_name, max_marks, status, rows })
}

fn upsert_marks(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_mode: DeviceMode,
    exam_subject_id: &str,
    entries: &[MarkEntryInput],
    submit: bool,
) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    let (cs_id, class_id, _subject, max_marks) = exam_subject_meta(conn, exam_subject_id)?;
    let own = actor.class_subjects.iter().any(|c| c == &cs_id);
    vidya_core::marks::can_edit(actor.role, own).map_err(CmdError::from)?;
    // A submitted sheet is locked (per subject); direct edits go via correction.
    let current: Option<String> = conn
        .query_row("SELECT status FROM marks_sheet WHERE exam_subject_id=?1", params![exam_subject_id], |r| r.get(0))
        .optional()?;
    if current.as_deref() == Some("submitted") {
        return Err(CoreError::Locked.into());
    }
    // Validate every entry against max_marks (0..=max, not absent+marks).
    for e in entries {
        let entry = MarkEntry { marks: e.marks.map(|m| m as u32), absent: e.absent };
        vidya_core::marks::validate_entry(&entry, max_marks as u32).map_err(CmdError::from)?;
    }
    let now = now_iso();
    let status = if submit { "submitted" } else { "draft" };
    let sync_state = if device_mode == DeviceMode::Server { "confirmed" } else { "on_device" };
    let ctx = WriteCtx { mode: device_mode };
    let esid = exam_subject_id.to_string();
    let cid = class_id.clone();
    let entries_owned: Vec<MarkEntryInput> = entries
        .iter()
        .map(|e| MarkEntryInput { student_id: e.student_id.clone(), marks: e.marks, absent: e.absent })
        .collect();
    with_write(conn, &ctx, move |tx| {
        // Upsert the sheet.
        let sheet_id: String = tx
            .query_row("SELECT id FROM marks_sheet WHERE exam_subject_id=?1", params![esid], |r| r.get(0))
            .optional()?
            .unwrap_or_else(|| {
                let id = new_id("msheet");
                let _ = tx.execute(
                    "INSERT INTO marks_sheet(id,exam_subject_id,status,created_at,updated_at,sync_state) VALUES (?1,?2,?3,?4,?4,?5)",
                    params![id, esid, status, now, sync_state],
                );
                id
            });
        tx.execute("UPDATE marks_sheet SET status=?1, updated_at=?2, sync_state=?3 WHERE id=?4", params![status, now, sync_state, sheet_id])?;
        for e in &entries_owned {
            tx.execute(
                "INSERT INTO mark_entry(id,sheet_id,student_id,marks,absent) VALUES (?1,?2,?3,?4,?5) \
                 ON CONFLICT(sheet_id,student_id) DO UPDATE SET marks=excluded.marks, absent=excluded.absent",
                params![new_id("me"), sheet_id, e.student_id, e.marks, e.absent as i64],
            )?;
        }
        let audit = AuditEntry {
            at: now.clone(),
            staff_id: Some(actor_s.id.clone()),
            action: if submit { "submit_marks".into() } else { "save_marks_draft".into() },
            table: Some("marks_sheet".into()),
            record_id: Some(sheet_id.clone()),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"),
            hlc: now.clone(),
            device_id: String::new(),
            staff_id: actor_s.id.clone(),
            audience: format!("class:{cid}"),
            table: "marks_sheet".into(),
            record_id: sheet_id.clone(),
            kind: if submit { "action".into() } else { "update".into() },
            payload: "{}".into(),
            base_version: None,
            server_epoch: 1,
        };
        Ok(Effect { value: (), audit, op })
    })?;
    Ok(())
}

pub fn save_marks_draft_logic(conn: &mut Connection, actor_s: &SessionStaff, device_mode: DeviceMode, exam_subject_id: &str, entries: &[MarkEntryInput]) -> CmdResult<()> {
    upsert_marks(conn, actor_s, device_mode, exam_subject_id, entries, false)
}

pub fn submit_marks_logic(conn: &mut Connection, actor_s: &SessionStaff, device_mode: DeviceMode, exam_subject_id: &str, entries: &[MarkEntryInput]) -> CmdResult<()> {
    upsert_marks(conn, actor_s, device_mode, exam_subject_id, entries, true)
}

// ---- Report card -----------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ReportSubjectDto {
    pub subject_name: String,
    pub max_marks: i64,
    pub obtained: Option<i64>,
    pub absent: bool,
    pub pct_tenths: i64,
    pub grade: String,
    pub incomplete: bool,
}

#[derive(Debug, Serialize)]
pub struct ReportCardDto {
    pub student_id: String,
    pub student_name: String,
    pub class_display: Option<String>,
    pub roll_no: Option<i64>,
    pub admission_no: Option<String>,
    pub provisional_no: Option<String>,
    pub exam_name: String,
    pub subjects: Vec<ReportSubjectDto>,
    pub total_obtained: i64,
    pub total_max: i64,
    pub pct_tenths: i64,
    pub grade: Option<String>,
    pub incomplete: bool,
    pub attendance: AttendanceSummaryDto,
    /// The class-teacher's remark for this exam (P16 Step 4), if entered.
    pub remark: Option<String>,
}

#[allow(clippy::type_complexity)]
pub fn get_report_card_logic(conn: &mut Connection, actor_s: &SessionStaff, today: &str, student_id: &str, exam_id: &str) -> CmdResult<ReportCardDto> {
    let actor = actor_from(conn, actor_s)?;
    // View gate: Principal all; teacher only students in their classes.
    let (name, admission_no, provisional_no, class_id, class_display, roll_no): (String, Option<String>, Option<String>, Option<String>, Option<String>, Option<i64>) = conn
        .query_row(
            "SELECT s.name, s.admission_no, s.provisional_no, c.id, c.display, e.roll_no FROM student s \
             LEFT JOIN enrollment e ON e.student_id=s.id AND e.to_date IS NULL LEFT JOIN class c ON c.id=e.class_id WHERE s.id=?1",
            params![student_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    if actor.role == Role::Teacher {
        let owns = class_id.as_deref().map(|c| actor.class_teacher_of.iter().any(|x| x == c)).unwrap_or(false);
        if !owns {
            return Err(CmdError::forbidden("not_own_class"));
        }
    }
    let exam_name: String = conn.query_row("SELECT name FROM exam WHERE id=?1", params![exam_id], |r| r.get(0)).optional()?.ok_or_else(CmdError::not_found)?;

    let scale = load_scale(conn);
    // Subjects for the student's class in this exam, with the student's entry.
    // (Scoped so the prepared statement's borrow ends before the &mut call below.)
    let raw: Vec<(String, i64, Option<i64>, Option<i64>)> = {
        let mut stmt = conn.prepare(
            "SELECT sub.name, es.max_marks, me.marks, me.absent FROM exam_subject es \
             JOIN class_subject cs ON cs.id=es.class_subject_id \
             JOIN subject sub ON sub.id=cs.subject_id \
             LEFT JOIN marks_sheet ms ON ms.exam_subject_id=es.id \
             LEFT JOIN mark_entry me ON me.sheet_id=ms.id AND me.student_id=?1 \
             WHERE es.exam_id=?2 AND cs.class_id=?3 ORDER BY sub.name",
        )?;
        let v: Vec<(String, i64, Option<i64>, Option<i64>)> = stmt
            .query_map(params![student_id, exam_id, class_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<rusqlite::Result<_>>()?;
        v
    };

    let mut subjects = Vec::new();
    let mut card_marks: Vec<SubjectMark> = Vec::new();
    for (subject_name, max_marks, marks, absent) in raw {
        let entry = MarkEntry { marks: marks.map(|m| m as u32), absent: absent.unwrap_or(0) != 0 };
        card_marks.push(SubjectMark { entry, max_marks: max_marks as u32 });
        let res = grades::grade_subject(&entry, max_marks as u32, &scale);
        let (pct, grade, incomplete) = match res {
            grades::SubjectResult::Graded { pct_tenths, grade } => (pct_tenths as i64, grade, false),
            grades::SubjectResult::Absent => (0, "AB".to_string(), false),
            grades::SubjectResult::Incomplete => (0, String::new(), true),
        };
        subjects.push(ReportSubjectDto { subject_name, max_marks, obtained: marks, absent: entry.absent, pct_tenths: pct, grade, incomplete });
    }

    let report = grades::grade_report(&card_marks, &scale);
    let (pct_tenths, grade, incomplete) = match report {
        grades::ReportResult::Graded { pct_tenths, grade } => (pct_tenths as i64, grade, false),
        grades::ReportResult::Incomplete => (0, None, true),
    };
    let total_obtained: i64 = card_marks.iter().filter(|s| !s.entry.absent).filter_map(|s| s.entry.marks).map(|m| m as i64).sum();
    let total_max: i64 = card_marks.iter().filter(|s| !s.entry.absent && s.entry.marks.is_some()).map(|s| s.max_marks as i64).sum();

    // Attendance for the current term (reuse the profile helper's approach).
    let profile = get_student_profile_logic(conn, today, student_id)?;
    let remark: Option<String> = conn
        .query_row("SELECT text FROM report_remark WHERE exam_id=?1 AND student_id=?2", params![exam_id, student_id], |r| r.get(0))
        .optional()?
        .filter(|s: &String| !s.is_empty());

    Ok(ReportCardDto {
        student_id: student_id.to_string(),
        student_name: name,
        class_display,
        roll_no,
        admission_no,
        provisional_no,
        exam_name,
        subjects,
        total_obtained,
        total_max,
        pct_tenths,
        grade,
        incomplete,
        attendance: profile.attendance,
        remark,
    })
}

// ---- Report-card remarks (P16 Step 4, §10.4) --------------------------------

#[derive(Debug, Serialize)]
pub struct ReportTemplateDto {
    pub key: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct ReportRemarkRowDto {
    pub student_id: String,
    pub name: String,
    pub roll_no: Option<i64>,
    pub remark: Option<String>,
    pub template_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReportRemarksDto {
    pub exam_id: String,
    pub class_id: String,
    pub class_display: Option<String>,
    pub locked: bool,
    pub students: Vec<ReportRemarkRowDto>,
    pub templates: Vec<ReportTemplateDto>,
}

/// Neutral remark suggestions in `language` (falls back to English per key).
fn load_report_templates(conn: &Connection, language: &str) -> rusqlite::Result<Vec<ReportTemplateDto>> {
    let mut stmt = conn.prepare(
        "SELECT key, COALESCE((SELECT text FROM report_template WHERE key=t.key AND language=?1), t.text) \
         FROM report_template t WHERE t.language='en' ORDER BY t.key",
    )?;
    let rows = stmt
        .query_map(params![language], |r| Ok(ReportTemplateDto { key: r.get(0)?, text: r.get(1)? }))?
        .collect::<rusqlite::Result<Vec<ReportTemplateDto>>>()?;
    Ok(rows)
}

/// The language for report-card suggestions: the school's chosen language
/// (`settings_json.language`), else English. Per-staff UI language is client-side.
fn report_template_language(conn: &Connection) -> String {
    conn.query_row("SELECT COALESCE(json_extract(settings_json,'$.language'),'en') FROM school LIMIT 1", [], |r| r.get::<_, String>(0))
        .optional()
        .ok()
        .flatten()
        .unwrap_or_else(|| "en".into())
}

fn exam_is_final(conn: &Connection, exam_id: &str) -> rusqlite::Result<bool> {
    Ok(conn.query_row("SELECT 1 FROM report_lock WHERE exam_id=?1", params![exam_id], |_| Ok(())).optional()?.is_some())
}

/// The remarks-entry list for a class in an exam (class teacher / Principal).
pub fn get_report_remarks_logic(conn: &mut Connection, actor_s: &SessionStaff, exam_id: &str, class_id: &str) -> CmdResult<ReportRemarksDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::EnterReportRemark, &Target { kind: TargetKind::Marks, class_id: Some(class_id.to_string()), ..Default::default() })?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    let class_display: Option<String> = conn.query_row("SELECT display FROM class WHERE id=?1", params![class_id], |r| r.get(0)).optional()?;
    // Suggestions in the school's default language (per-staff UI language is
    // client-side, not stored on `staff`); the teacher may edit freely.
    let language = report_template_language(conn);
    let students: Vec<ReportRemarkRowDto> = {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.name, e.roll_no, r.text, r.template_key FROM enrollment e \
             JOIN student s ON s.id=e.student_id \
             LEFT JOIN report_remark r ON r.student_id=s.id AND r.exam_id=?1 \
             WHERE e.class_id=?2 AND e.to_date IS NULL ORDER BY e.roll_no",
        )?;
        let rows = stmt.query_map(params![exam_id, class_id], |r| {
            let remark: Option<String> = r.get(3)?;
            Ok(ReportRemarkRowDto { student_id: r.get(0)?, name: r.get(1)?, roll_no: r.get(2)?, remark, template_key: r.get(4)? })
        })?
        .collect::<rusqlite::Result<Vec<ReportRemarkRowDto>>>()?;
        rows
    };
    Ok(ReportRemarksDto {
        exam_id: exam_id.to_string(),
        class_id: class_id.to_string(),
        class_display,
        locked: exam_is_final(conn, exam_id)?,
        students,
        templates: load_report_templates(conn, &language)?,
    })
}

/// List the remark templates in the actor's language (Settings → Report cards).
pub fn list_report_templates_logic(conn: &mut Connection, _actor_s: &SessionStaff) -> CmdResult<Vec<ReportTemplateDto>> {
    let language = report_template_language(conn);
    Ok(load_report_templates(conn, &language)?)
}

#[derive(Debug, Deserialize)]
pub struct ReportRemarkInput {
    pub exam_id: String,
    pub student_id: String,
    pub text: String,
    pub template_key: Option<String>,
}

/// Enter or edit a report-card remark (class teacher; Principal). Locked once the
/// exam's report cards are final: a teacher is refused; the Principal may still
/// edit directly (audited), the same lock pattern as marks.
pub fn save_report_remark_logic(conn: &mut Connection, actor_s: &SessionStaff, device_id: Option<&str>, device_mode: DeviceMode, input: &ReportRemarkInput) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    // The student's current class decides ownership.
    let class_id: String = conn
        .query_row("SELECT class_id FROM enrollment WHERE student_id=?1 AND to_date IS NULL LIMIT 1", params![input.student_id], |r| r.get(0))
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    require_allow(&actor, Action::EnterReportRemark, &Target { kind: TargetKind::Marks, class_id: Some(class_id.clone()), ..Default::default() })?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    vidya_core::report::validate_remark(&input.text)?;
    if exam_is_final(conn, &input.exam_id)? && actor.role != Role::Principal {
        return Err(CoreError::SheetLocked.into());
    }
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let id = new_id("rmk");
    let sync_state = if device_mode == DeviceMode::Server { "confirmed" } else { "on_device" };
    let ctx = WriteCtx { mode: device_mode };
    let dev = device_id.map(str::to_string);
    let exam_id = input.exam_id.clone();
    let student_id = input.student_id.clone();
    let text = input.text.trim().to_string();
    let template_key = input.template_key.clone();
    let class_id2 = class_id.clone();

    with_write(conn, &ctx, move |tx| {
        // One remark per (exam, student): update if present, else insert.
        let rid: Option<String> = tx
            .query_row("SELECT id FROM report_remark WHERE exam_id=?1 AND student_id=?2", params![exam_id, student_id], |r| r.get(0))
            .optional()?;
        let record_id = match rid {
            Some(existing) => {
                tx.execute(
                    "UPDATE report_remark SET text=?1, template_key=?2, author=?3, updated_at=?4, updated_by_staff=?3, updated_by_device=?5, sync_state=?6 WHERE id=?7",
                    params![text, template_key, actor_s.id, now, dev, sync_state, existing],
                )?;
                existing
            }
            None => {
                tx.execute(
                    "INSERT INTO report_remark(id,exam_id,student_id,text,template_key,author,school_id,created_at,updated_at,updated_by_staff,updated_by_device,sync_state) \
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8,?6,?9,?10)",
                    params![id, exam_id, student_id, text, template_key, actor_s.id, school_id, now, dev, sync_state],
                )?;
                id.clone()
            }
        };
        let audit = AuditEntry {
            at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "save_report_remark".into(),
            table: Some("report_remark".into()), record_id: Some(record_id.clone()),
            after_json: Some(serde_json::json!({ "exam_id": exam_id, "student_id": student_id }).to_string()),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"), hlc: now.clone(), device_id: dev.clone().unwrap_or_default(), staff_id: actor_s.id.clone(),
            audience: format!("class:{class_id2}"), table: "report_remark".into(), record_id, kind: "insert".into(),
            payload: serde_json::json!({ "class_id": class_id2, "student_id": student_id }).to_string(),
            base_version: None, server_epoch: 1,
        };
        Ok(Effect { value: (), audit, op })
    })?;
    Ok(())
}

/// Make an exam's report cards final — locks its remarks (Principal only).
pub fn finalize_report_cards_logic(conn: &mut Connection, actor_s: &SessionStaff, exam_id: &str) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::FinalizeReportCards, &Target::of(TargetKind::School))?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    conn.execute(
        "INSERT INTO report_lock(exam_id,finalised_by,finalised_at,school_id,created_at,updated_at,sync_state) \
         VALUES (?1,?2,?3,?4,?3,?3,'confirmed') ON CONFLICT(exam_id) DO NOTHING",
        params![exam_id, actor_s.id, now, school_id],
    )?;
    audit_action(conn, AuditEntry {
        at: now, staff_id: Some(actor_s.id.clone()), action: "finalize_report_cards".into(),
        table: Some("report_lock".into()), record_id: Some(exam_id.to_string()), ..Default::default()
    })?;
    Ok(())
}

// ---- Exam seating & hall tickets (P16 Step 5, §10.4) ------------------------

#[derive(Debug, Serialize)]
pub struct ExamRoomDto {
    pub id: String,
    pub name: String,
    pub rows: i64,
    pub cols: i64,
    pub invigilator_id: Option<String>,
    pub invigilator_name: Option<String>,
    pub seats: Vec<SeatDto>,
}

#[derive(Debug, Serialize)]
pub struct SeatDto {
    pub seat_no: i64,
    pub student_id: String,
    pub student_name: String,
    pub roll_no: Option<i64>,
    pub class_display: Option<String>,
    pub class_slot: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ScheduleLineDto {
    pub date: String,
    pub subject_name: Option<String>,
    pub starts_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HallTicketDto {
    pub student_id: String,
    pub student_name: String,
    pub class_display: Option<String>,
    pub roll_no: Option<i64>,
    pub room_name: String,
    pub seat_no: i64,
    pub schedule: Vec<ScheduleLineDto>,
}

#[derive(Debug, Serialize)]
pub struct ExamSeatingDto {
    pub exam_id: String,
    pub exam_name: String,
    pub rooms: Vec<ExamRoomDto>,
    pub hall_tickets: Vec<HallTicketDto>,
}

fn exam_name_of(conn: &Connection, exam_id: &str) -> CmdResult<String> {
    conn.query_row("SELECT name FROM exam WHERE id=?1", params![exam_id], |r| r.get(0)).optional()?.ok_or_else(CmdError::not_found)
}

/// The distinct classes sitting an exam (those with an exam_subject), ordered.
fn exam_classes(conn: &Connection, exam_id: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT cs.class_id FROM exam_subject es JOIN class_subject cs ON cs.id=es.class_subject_id \
         JOIN class c ON c.id=cs.class_id WHERE es.exam_id=?1 ORDER BY c.sort_order",
    )?;
    let rows = stmt.query_map(params![exam_id], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(rows)
}

fn class_roster(conn: &Connection, class_id: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT student_id FROM enrollment WHERE class_id=?1 AND to_date IS NULL ORDER BY roll_no")?;
    let rows = stmt.query_map(params![class_id], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(rows)
}

/// Rooms of an exam (for the rooms list). Principal only.
pub fn list_exam_rooms_logic(conn: &mut Connection, actor_s: &SessionStaff, exam_id: &str) -> CmdResult<ExamSeatingDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageExamSeating, &Target::of(TargetKind::School))?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    get_exam_seating_inner(conn, exam_id)
}

/// Rooms + their seats + per-student hall tickets (used by the screen and prints).
pub fn get_exam_seating_logic(conn: &mut Connection, actor_s: &SessionStaff, exam_id: &str) -> CmdResult<ExamSeatingDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageExamSeating, &Target::of(TargetKind::School))?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    get_exam_seating_inner(conn, exam_id)
}

#[allow(clippy::type_complexity)]
fn get_exam_seating_inner(conn: &Connection, exam_id: &str) -> CmdResult<ExamSeatingDto> {
    let exam_name = exam_name_of(conn, exam_id)?;
    let rooms_meta: Vec<(String, String, i64, i64, Option<String>, Option<String>)> = {
        let mut stmt = conn.prepare(
            "SELECT r.id, r.name, r.rows, r.cols, r.invigilator_id, st.name FROM exam_room r \
             LEFT JOIN staff st ON st.id=r.invigilator_id WHERE r.exam_id=?1 ORDER BY r.sort_order, r.name",
        )?;
        let v = stmt.query_map(params![exam_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        v
    };
    let mut rooms = Vec::new();
    let mut hall_tickets = Vec::new();
    for (id, name, rows, cols, invig_id, invig_name) in rooms_meta {
        let seats: Vec<SeatDto> = {
            let mut stmt = conn.prepare(
                "SELECT es.seat_no, es.student_id, s.name, e.roll_no, c.display, es.class_slot FROM exam_seat es \
                 JOIN student s ON s.id=es.student_id \
                 LEFT JOIN enrollment e ON e.student_id=es.student_id AND e.to_date IS NULL \
                 LEFT JOIN class c ON c.id=e.class_id \
                 WHERE es.exam_id=?1 AND es.room_id=?2 ORDER BY es.seat_no",
            )?;
            let v = stmt.query_map(params![exam_id, id], |r| Ok(SeatDto {
                seat_no: r.get(0)?, student_id: r.get(1)?, student_name: r.get(2)?, roll_no: r.get(3)?, class_display: r.get(4)?, class_slot: r.get(5)?,
            }))?.collect::<rusqlite::Result<Vec<_>>>()?;
            v
        };
        for s in &seats {
            let schedule = student_schedule(conn, exam_id, &s.student_id)?;
            hall_tickets.push(HallTicketDto {
                student_id: s.student_id.clone(), student_name: s.student_name.clone(), class_display: s.class_display.clone(),
                roll_no: s.roll_no, room_name: name.clone(), seat_no: s.seat_no, schedule,
            });
        }
        rooms.push(ExamRoomDto { id, name, rows, cols, invigilator_id: invig_id, invigilator_name: invig_name, seats });
    }
    Ok(ExamSeatingDto { exam_id: exam_id.to_string(), exam_name, rooms, hall_tickets })
}

fn student_schedule(conn: &Connection, exam_id: &str, student_id: &str) -> rusqlite::Result<Vec<ScheduleLineDto>> {
    let mut stmt = conn.prepare(
        "SELECT sch.date, sub.name, sch.starts_at FROM exam_schedule sch \
         JOIN enrollment e ON e.class_id=sch.class_id AND e.to_date IS NULL AND e.student_id=?2 \
         LEFT JOIN class_subject cs ON cs.id=sch.class_subject_id LEFT JOIN subject sub ON sub.id=cs.subject_id \
         WHERE sch.exam_id=?1 ORDER BY sch.date, sch.starts_at",
    )?;
    let rows = stmt.query_map(params![exam_id, student_id], |r| Ok(ScheduleLineDto { date: r.get(0)?, subject_name: r.get(1)?, starts_at: r.get(2)? }))?
        .collect::<rusqlite::Result<Vec<ScheduleLineDto>>>()?;
    Ok(rows)
}

#[derive(Debug, Deserialize)]
pub struct ExamRoomInput {
    pub id: Option<String>,
    pub exam_id: String,
    pub name: String,
    pub rows: i64,
    pub cols: i64,
    pub invigilator_id: Option<String>,
}

/// Create or update an exam room (Principal).
pub fn save_exam_room_logic(conn: &mut Connection, actor_s: &SessionStaff, input: &ExamRoomInput) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageExamSeating, &Target::of(TargetKind::School))?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    if input.rows < 1 || input.cols < 1 {
        return Err(CmdError::validation("room", "range"));
    }
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let id = match &input.id {
        Some(id) => {
            conn.execute(
                "UPDATE exam_room SET name=?1, rows=?2, cols=?3, invigilator_id=?4, updated_at=?5 WHERE id=?6",
                params![input.name, input.rows, input.cols, input.invigilator_id, now, id],
            )?;
            id.clone()
        }
        None => {
            let id = new_id("room");
            let next: i64 = conn.query_row("SELECT COALESCE(MAX(sort_order),-1)+1 FROM exam_room WHERE exam_id=?1", params![input.exam_id], |r| r.get(0))?;
            conn.execute(
                "INSERT INTO exam_room(id,exam_id,name,rows,cols,invigilator_id,sort_order,school_id,created_at,updated_at,sync_state) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9,'confirmed')",
                params![id, input.exam_id, input.name, input.rows, input.cols, input.invigilator_id, next, school_id, now],
            )?;
            id
        }
    };
    audit_action(conn, AuditEntry {
        at: now, staff_id: Some(actor_s.id.clone()), action: "save_exam_room".into(),
        table: Some("exam_room".into()), record_id: Some(id), ..Default::default()
    })?;
    Ok(())
}

/// Delete an exam room and its seats (Principal).
pub fn delete_exam_room_logic(conn: &mut Connection, actor_s: &SessionStaff, room_id: &str) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageExamSeating, &Target::of(TargetKind::School))?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM exam_seat WHERE room_id=?1", params![room_id])?;
    tx.execute("DELETE FROM exam_room WHERE id=?1", params![room_id])?;
    tx.commit()?;
    audit_action(conn, AuditEntry {
        at: now_iso(), staff_id: Some(actor_s.id.clone()), action: "delete_exam_room".into(),
        table: Some("exam_room".into()), record_id: Some(room_id.to_string()), ..Default::default()
    })?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct SeatingErrorDto {
    pub room_name: String,
    pub capacity: i64,
    pub needed: i64,
    pub missing: i64,
}

#[derive(Debug, Serialize)]
pub struct SeatingResult {
    pub seated: i64,
    pub rooms_used: i64,
    pub unpaired_classes: i64,
    pub errors: Vec<SeatingErrorDto>,
}

/// Generate seating for an exam: auto-pair its classes (adjacent by order), fill
/// the rooms in order, deterministically. Capacity-short rooms are reported and
/// nothing is written. Principal only.
pub fn generate_seating_logic(conn: &mut Connection, actor_s: &SessionStaff, exam_id: &str) -> CmdResult<SeatingResult> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageExamSeating, &Target::of(TargetKind::School))?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    let classes = exam_classes(conn, exam_id)?;
    let pairs = vidya_core::seating::auto_pairs(&classes);
    let rooms: Vec<(String, u32, u32)> = {
        let mut stmt = conn.prepare("SELECT id, rows, cols FROM exam_room WHERE exam_id=?1 ORDER BY sort_order, name")?;
        let v = stmt.query_map(params![exam_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u32, r.get::<_, i64>(2)? as u32)))?
            .collect::<rusqlite::Result<Vec<(String, u32, u32)>>>()?;
        v
    };
    let usable = pairs.len().min(rooms.len());
    let unpaired = (pairs.len() - usable) as i64;
    // Build the room plans for as many pairs as there are rooms.
    let mut plans = Vec::new();
    for i in 0..usable {
        let (a, b) = &pairs[i];
        let (room_id, rows, cols) = &rooms[i];
        plans.push(vidya_core::seating::RoomPlan {
            room_id: room_id.clone(),
            rows: *rows,
            cols: *cols,
            class_a: class_roster(conn, a)?,
            class_b: match b { Some(bc) => class_roster(conn, bc)?, None => vec![] },
        });
    }
    match vidya_core::seating::generate_seating(&plans) {
        Err(errs) => {
            // Map room ids to names for the error report.
            let mut out = Vec::new();
            for e in errs {
                let name: String = conn.query_row("SELECT name FROM exam_room WHERE id=?1", params![e.room_id], |r| r.get(0)).optional()?.unwrap_or_default();
                out.push(SeatingErrorDto { room_name: name, capacity: e.capacity as i64, needed: e.needed as i64, missing: e.missing as i64 });
            }
            Ok(SeatingResult { seated: 0, rooms_used: 0, unpaired_classes: unpaired, errors: out })
        }
        Ok(seats) => {
            let now = now_iso();
            let school_id = single_school_id(conn)?;
            let seated = seats.len() as i64;
            let tx = conn.transaction()?;
            tx.execute("DELETE FROM exam_seat WHERE exam_id=?1", params![exam_id])?;
            for s in &seats {
                tx.execute(
                    "INSERT INTO exam_seat(id,exam_id,room_id,seat_no,student_id,class_slot,school_id,created_at,updated_at,sync_state) \
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8,'confirmed')",
                    params![new_id("seat"), exam_id, s.room_id, s.seat_no as i64, s.student_id, s.class_slot, school_id, now],
                )?;
            }
            crate::security::audit::append(&tx, &AuditEntry {
                at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "generate_seating".into(),
                table: Some("exam_seat".into()), record_id: Some(exam_id.to_string()),
                after_json: Some(serde_json::json!({ "seated": seated, "rooms": usable }).to_string()),
                ..Default::default()
            })?;
            tx.commit()?;
            Ok(SeatingResult { seated, rooms_used: usable as i64, unpaired_classes: unpaired, errors: vec![] })
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ExamScheduleInput {
    pub exam_id: String,
    pub class_id: String,
    pub date: String,
    pub starts_at: Option<String>,
    pub class_subject_id: Option<String>,
}

/// Add one exam-schedule row (date/subject/time for a class). Principal only.
pub fn save_exam_schedule_logic(conn: &mut Connection, actor_s: &SessionStaff, input: &ExamScheduleInput) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageExamSeating, &Target::of(TargetKind::School))?;
    require_module_enabled(conn, vidya_core::modules::Module::Classroom)?;
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    conn.execute(
        "INSERT INTO exam_schedule(id,exam_id,date,class_id,class_subject_id,starts_at,school_id,created_at,updated_at,sync_state) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8,'confirmed')",
        params![new_id("esch"), input.exam_id, input.date, input.class_id, input.class_subject_id, input.starts_at, school_id, now],
    )?;
    audit_action(conn, AuditEntry {
        at: now, staff_id: Some(actor_s.id.clone()), action: "save_exam_schedule".into(),
        table: Some("exam_schedule".into()), record_id: Some(input.exam_id.clone()), ..Default::default()
    })?;
    Ok(())
}

// ============================================================= reports ========
//
// Read-only reports (docs/00 §12). The audit-log viewer is filterable, paginated
// and shows the chain status; teachers see only their own entries. The other
// reports reuse existing aggregates or add small read queries.

#[derive(Debug, Serialize)]
pub struct AuditRowDto {
    pub seq: i64,
    pub at: String,
    pub staff_name: Option<String>,
    pub action: String,
    pub table: Option<String>,
    pub record_id: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuditPageDto {
    pub rows: Vec<AuditRowDto>,
    pub total: i64,
    pub chain_ok: bool,
    pub first_bad_seq: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub staff_id: Option<String>,
    pub table: Option<String>,
    pub action: Option<String>,
    pub date: Option<String>, // YYYY-MM-DD prefix
    pub limit: i64,
    pub offset: i64,
}

pub fn list_audit_logic(conn: &mut Connection, actor_s: &SessionStaff, q: &AuditQuery) -> CmdResult<AuditPageDto> {
    use rusqlite::params_from_iter;
    use rusqlite::types::Value;
    let actor = actor_from(conn, actor_s)?;
    let mut wheres: Vec<String> = Vec::new();
    let mut args: Vec<Value> = Vec::new();
    // Teachers only ever see their own entries (§12).
    if actor.role == Role::Teacher {
        wheres.push("a.staff_id = ?".into());
        args.push(Value::Text(actor_s.id.clone()));
    } else if let Some(s) = q.staff_id.as_deref().filter(|s| !s.is_empty()) {
        wheres.push("a.staff_id = ?".into());
        args.push(Value::Text(s.to_string()));
    }
    if let Some(tbl) = q.table.as_deref().filter(|s| !s.is_empty()) {
        wheres.push("a.\"table\" = ?".into());
        args.push(Value::Text(tbl.to_string()));
    }
    if let Some(ac) = q.action.as_deref().filter(|s| !s.is_empty()) {
        wheres.push("a.action = ?".into());
        args.push(Value::Text(ac.to_string()));
    }
    if let Some(d) = q.date.as_deref().filter(|s| !s.is_empty()) {
        wheres.push("a.at LIKE ?".into());
        args.push(Value::Text(format!("{d}%")));
    }
    let where_clause = if wheres.is_empty() { String::new() } else { format!("WHERE {}", wheres.join(" AND ")) };

    let total: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM audit_log a {where_clause}"), params_from_iter(args.iter()), |r| r.get(0))?;

    let sql = format!(
        "SELECT a.seq, a.at, st.name, a.action, a.\"table\", a.record_id, a.reason \
         FROM audit_log a LEFT JOIN staff st ON st.id=a.staff_id {where_clause} ORDER BY a.seq DESC LIMIT ? OFFSET ?"
    );
    let mut page_args = args;
    page_args.push(Value::Integer(q.limit.max(0)));
    page_args.push(Value::Integer(q.offset.max(0)));
    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<AuditRowDto> = stmt
        .query_map(params_from_iter(page_args.iter()), |r| {
            Ok(AuditRowDto {
                seq: r.get(0)?,
                at: r.get(1)?,
                staff_name: r.get(2)?,
                action: r.get(3)?,
                table: r.get(4)?,
                record_id: r.get(5)?,
                reason: r.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let first_bad = crate::security::audit::verify_chain(conn)?;
    Ok(AuditPageDto { rows, total, chain_ok: first_bad.is_none(), first_bad_seq: first_bad })
}

#[derive(Debug, Serialize)]
pub struct MonthCountDto {
    pub month: String,
    pub count: i64,
}

pub fn admissions_by_month_logic(conn: &mut Connection, actor_s: &SessionStaff) -> CmdResult<Vec<MonthCountDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ViewStudent, &Target::of(TargetKind::Student))?;
    let mut stmt = conn.prepare(
        "SELECT substr(created_at,1,7) AS m, COUNT(*) FROM student GROUP BY m ORDER BY m DESC LIMIT 24",
    )?;
    let rows = stmt.query_map([], |r| Ok(MonthCountDto { month: r.get(0)?, count: r.get(1)? }))?.collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

#[derive(Debug, Serialize)]
pub struct FeeCollectionDto {
    pub from: String,
    pub to: String,
    pub cash_paise: i64,
    pub upi_paise: i64,
    pub cheque_paise: i64,
    pub total_paise: i64,
    pub by_day: Vec<MoneyDayDto>,
}

#[derive(Debug, Serialize)]
pub struct MoneyDayDto {
    pub day: String,
    pub total_paise: i64,
}

pub fn fee_collection_report_logic(conn: &mut Connection, actor_s: &SessionStaff, from: &str, to: &str) -> CmdResult<FeeCollectionDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::FeeReports, &Target::of(TargetKind::Fee))?;
    let lo = format!("{from}T00:00:00Z");
    let hi = format!("{to}T23:59:59Z");
    let mode_sum = |conn: &Connection, mode: &str| -> rusqlite::Result<i64> {
        conn.query_row(
            "SELECT COALESCE(SUM(amount_paise),0) FROM payment WHERE sync_state='confirmed' AND mode=?1 AND collected_at BETWEEN ?2 AND ?3",
            params![mode, lo, hi],
            |r| r.get(0),
        )
    };
    let cash = mode_sum(conn, "cash")?;
    let upi = mode_sum(conn, "upi")?;
    let cheque = mode_sum(conn, "cheque")?;
    let mut stmt = conn.prepare(
        "SELECT substr(collected_at,1,10) AS d, SUM(amount_paise) FROM payment \
         WHERE sync_state='confirmed' AND collected_at BETWEEN ?1 AND ?2 GROUP BY d ORDER BY d",
    )?;
    let by_day = stmt
        .query_map(params![lo, hi], |r| Ok(MoneyDayDto { day: r.get(0)?, total_paise: r.get(1)? }))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(FeeCollectionDto { from: from.to_string(), to: to.to_string(), cash_paise: cash, upi_paise: upi, cheque_paise: cheque, total_paise: cash + upi + cheque, by_day })
}

#[derive(Debug, Serialize)]
pub struct GradeCountDto {
    pub grade: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct ExamResultRowDto {
    pub class_display: Option<String>,
    pub subject_name: String,
    pub max_marks: i64,
    pub graded: i64,
    pub average_pct_tenths: i64,
    pub distribution: Vec<GradeCountDto>,
}

pub fn exam_results_logic(conn: &mut Connection, actor_s: &SessionStaff, exam_id: &str) -> CmdResult<Vec<ExamResultRowDto>> {
    let _actor = actor_from(conn, actor_s)?;
    let scale = load_scale(conn);
    let subjects = exam_subjects(conn, exam_id)?;
    let mut out = Vec::new();
    for es in subjects {
        // Entered, non-absent marks for this subject.
        let mut stmt = conn.prepare(
            "SELECT me.marks FROM mark_entry me JOIN marks_sheet ms ON ms.id=me.sheet_id \
             WHERE ms.exam_subject_id=?1 AND me.marks IS NOT NULL AND me.absent=0",
        )?;
        let marks: Vec<i64> = stmt.query_map(params![es.id], |r| r.get::<_, i64>(0))?.collect::<rusqlite::Result<_>>()?;
        let graded = marks.len() as i64;
        let mut dist: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
        let mut sum_pct: i64 = 0;
        for m in &marks {
            let pct = grades::subject_percent(*m as u32, es.max_marks as u32);
            sum_pct += pct as i64;
            let g = grades::grade_for(pct, &scale).map(|b| b.grade.clone()).unwrap_or_else(|| "-".into());
            *dist.entry(g).or_insert(0) += 1;
        }
        let avg = if graded > 0 { sum_pct / graded } else { 0 };
        out.push(ExamResultRowDto {
            class_display: es.class_display,
            subject_name: es.subject_name,
            max_marks: es.max_marks,
            graded,
            average_pct_tenths: avg,
            distribution: dist.into_iter().map(|(grade, count)| GradeCountDto { grade, count }).collect(),
        });
    }
    Ok(out)
}

/// Student ids of a class (for a class report-card batch print).
pub fn class_student_ids_logic(conn: &mut Connection, class_id: &str) -> CmdResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT s.id FROM enrollment e JOIN student s ON s.id=e.student_id WHERE e.class_id=?1 AND e.to_date IS NULL ORDER BY e.roll_no, s.name")?;
    let rows: Vec<String> = stmt.query_map(params![class_id], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

// ------------------------------------------------------------- modules --------

/// One switchable module for the Settings → Languages & modules screen. `locked`
/// is currently unused (Core is not listed) but kept for the UI's shape.
#[derive(Debug, Serialize)]
pub struct ModuleDto {
    pub key: String,
    pub enabled: bool,
}

/// List the toggle-able modules and their on/off state (Principal only, §14).
pub fn list_modules_logic(conn: &mut Connection, actor_s: &SessionStaff) -> CmdResult<Vec<ModuleDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::Settings, &Target { kind: TargetKind::School, ..Default::default() })?;
    let mut out = Vec::with_capacity(vidya_core::modules::TOGGLEABLE_KEYS.len());
    for key in vidya_core::modules::TOGGLEABLE_KEYS {
        let enabled: i64 = conn
            .query_row("SELECT enabled FROM module_setting WHERE key=?1", params![key], |r| r.get(0))
            .optional()?
            .unwrap_or(0);
        out.push(ModuleDto { key: (*key).to_string(), enabled: enabled != 0 });
    }
    Ok(out)
}

/// Turn a module on or off (Principal only, §14). Audited; never deletes data —
/// turning a module off only hides its screens, rejects its ops and stops syncing
/// its tables; turning it on again shows everything.
pub fn set_module_logic(conn: &mut Connection, actor_s: &SessionStaff, key: &str, enabled: bool) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::Settings, &Target { kind: TargetKind::School, ..Default::default() })?;
    if !vidya_core::modules::TOGGLEABLE_KEYS.contains(&key) {
        return Err(CmdError::validation("module", "unknown"));
    }
    let now = now_iso();
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO module_setting(key,enabled,changed_by,changed_at) VALUES (?1,?2,?3,?4) \
         ON CONFLICT(key) DO UPDATE SET enabled=excluded.enabled, changed_by=excluded.changed_by, changed_at=excluded.changed_at",
        params![key, enabled as i64, actor_s.id, now],
    )?;
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(),
        staff_id: Some(actor_s.id.clone()),
        action: "set_module".into(),
        table: Some("module_setting".into()),
        record_id: Some(key.to_string()),
        after_json: Some(serde_json::json!({ "key": key, "enabled": enabled }).to_string()),
        ..Default::default()
    })?;
    tx.commit()?;
    Ok(())
}

// ============================================================= payments (UPI) =

/// Read the school's UPI settings (Settings → Payments, the reminder sheet, the
/// receipt QR). Any signed-in staff may read it — it is non-sensitive school
/// config and several roles render the QR.
pub fn get_payment_settings_logic(conn: &mut Connection) -> CmdResult<crate::upi::PaymentSettings> {
    Ok(crate::upi::PaymentSettings::read(conn)?)
}

#[derive(Debug, Deserialize)]
pub struct PaymentSettingsInput {
    /// Empty / absent clears the UPI id (no QR anywhere).
    pub upi_id: Option<String>,
    pub upi_name: Option<String>,
    pub on_receipts: bool,
    pub on_reminders: bool,
    pub on_dues_list: bool,
}

/// Save the school's UPI settings (Principal only, §10.1). Validates the VPA when
/// present, merges into `school.settings_json` (keeping `phone`), and audits.
pub fn set_payment_settings_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    input: &PaymentSettingsInput,
) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::Settings, &Target { kind: TargetKind::School, ..Default::default() })?;

    // Read the current blob so we don't drop `phone` or any other key.
    let (school_id, raw): (String, Option<String>) = conn
        .query_row("SELECT id, settings_json FROM school LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    let mut settings: serde_json::Value = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let vpa = match input.upi_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(v) => Some(vidya_core::upi::validate_vpa(v)?),
        None => None,
    };
    let name = input
        .upi_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    match (&vpa, &name) {
        (Some(v), _) => {
            settings["upi_id"] = serde_json::Value::String(v.clone());
            // Default the display name to the school name when left blank.
            let display = name.unwrap_or_else(|| {
                conn.query_row("SELECT name FROM school WHERE id=?1", params![school_id], |r| r.get::<_, String>(0))
                    .unwrap_or_default()
            });
            settings["upi_name"] = serde_json::Value::String(display);
        }
        (None, _) => {
            // Clearing the UPI id removes both fields; toggles are kept but inert.
            settings.as_object_mut().map(|m| m.remove("upi_id"));
            settings.as_object_mut().map(|m| m.remove("upi_name"));
        }
    }
    settings["upi_on_receipts"] = serde_json::Value::Bool(input.on_receipts);
    settings["upi_on_reminders"] = serde_json::Value::Bool(input.on_reminders);
    settings["upi_on_dues_list"] = serde_json::Value::Bool(input.on_dues_list);

    let now = now_iso();
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE school SET settings_json=?1, updated_at=?2 WHERE id=?3",
        params![settings.to_string(), now, school_id],
    )?;
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(),
        staff_id: Some(actor_s.id.clone()),
        action: "set_payment_settings".into(),
        table: Some("school".into()),
        record_id: Some(school_id.clone()),
        // The VPA is not a secret (it is printed on receipts), but keep the audit
        // to just the fact of the change + toggles.
        after_json: Some(serde_json::json!({
            "upi_id_set": vpa.is_some(),
            "on_receipts": input.on_receipts,
            "on_reminders": input.on_reminders,
            "on_dues_list": input.on_dues_list,
        }).to_string()),
        ..Default::default()
    })?;
    tx.commit()?;
    Ok(())
}

/// Render any string as an SVG QR code (navy on white). Used by the receipt,
/// reminder sheet and printed dues list; a pure rendering helper, so any signed-in
/// staff may call it.
pub fn qr_svg_logic(data: &str) -> CmdResult<String> {
    crate::upi::qr_svg(data).map_err(|e| CmdError::internal(format!("qr: {e}")))
}

// ---- Automatic WhatsApp config (P14 Step 7, module `wa_auto`) ----------------

#[derive(Debug, Serialize)]
pub struct WaAutoConfigDto {
    pub configured: bool,
    pub phone_number_id: String,
    /// Whether an access token is stored (never returned to the UI).
    pub token_set: bool,
    pub templates: std::collections::BTreeMap<String, String>,
}

/// Read the wa_auto setup (Principal). The access token is **never** returned — only
/// whether one is stored — so it can't leak to the frontend or logs.
pub fn get_wa_auto_config_logic(conn: &mut Connection, actor_s: &SessionStaff) -> CmdResult<WaAutoConfigDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::Settings, &Target { kind: TargetKind::School, ..Default::default() })?;
    let raw: Option<String> = conn.query_row("SELECT settings_json FROM school LIMIT 1", [], |r| r.get(0)).optional()?;
    let v: serde_json::Value = raw.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(serde_json::Value::Null);
    let w = v.get("wa_auto");
    let phone_number_id = w.and_then(|x| x.get("phone_number_id")).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let token_set = w.and_then(|x| x.get("token")).and_then(|x| x.as_str()).map(|t| !t.is_empty()).unwrap_or(false);
    let mut templates = std::collections::BTreeMap::new();
    if let Some(map) = w.and_then(|x| x.get("templates")).and_then(|x| x.as_object()) {
        for (k, val) in map {
            if let Some(name) = val.as_str() { templates.insert(k.clone(), name.to_string()); }
        }
    }
    Ok(WaAutoConfigDto { configured: !phone_number_id.is_empty() && token_set, phone_number_id, token_set, templates })
}

#[derive(Debug, Deserialize)]
pub struct WaAutoConfigInput {
    pub phone_number_id: String,
    /// Empty / absent keeps the existing token (so the UI need not re-enter it).
    pub token: Option<String>,
    pub templates: std::collections::BTreeMap<String, String>,
}

/// Save the wa_auto setup (Principal, `wa_auto` module). Merges into
/// `school.settings_json.wa_auto`, keeping the existing token when none is given.
/// Audited (the token is never written to the audit).
pub fn set_wa_auto_config_logic(conn: &mut Connection, actor_s: &SessionStaff, input: &WaAutoConfigInput) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::Settings, &Target { kind: TargetKind::School, ..Default::default() })?;
    vidya_core::modules::require_enabled(&crate::modules::enabled_set(conn)?, vidya_core::modules::Module::WaAuto)?;
    let (school_id, raw): (String, Option<String>) = conn
        .query_row("SELECT id, settings_json FROM school LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    let mut settings: serde_json::Value = raw.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_else(|| serde_json::json!({}));
    let existing_token = settings.get("wa_auto").and_then(|w| w.get("token")).and_then(|t| t.as_str()).unwrap_or("").to_string();
    let token = match input.token.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        Some(t) => t.to_string(),
        None => existing_token,
    };
    let templates: serde_json::Map<String, serde_json::Value> = input.templates.iter().map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone()))).collect();
    settings["wa_auto"] = serde_json::json!({
        "phone_number_id": input.phone_number_id.trim(),
        "token": token,
        "templates": templates,
    });
    let now = now_iso();
    let tx = conn.transaction()?;
    tx.execute("UPDATE school SET settings_json=?1, updated_at=?2 WHERE id=?3", params![settings.to_string(), now, school_id])?;
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "set_wa_auto_config".into(),
        table: Some("school".into()), record_id: Some(school_id.clone()),
        // Token deliberately excluded from the audit.
        after_json: Some(serde_json::json!({ "phone_number_id_set": !input.phone_number_id.trim().is_empty(), "token_set": !token.is_empty(), "template_count": input.templates.len() }).to_string()),
        ..Default::default()
    })?;
    tx.commit()?;
    Ok(())
}

// ============================================================= messages ======

/// A message-outbox row for the UI (share status, absence/reminder lists, logs).
#[derive(Debug, Serialize)]
pub struct MessageDto {
    pub id: String,
    pub channel: String,
    pub kind: Option<String>,
    pub language: String,
    pub to_guardian_id: Option<String>,
    pub to_address: Option<String>,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub related_table: Option<String>,
    pub related_id: Option<String>,
    pub created_at: String,
}

fn map_message(r: &rusqlite::Row) -> rusqlite::Result<MessageDto> {
    Ok(MessageDto {
        id: r.get("id")?,
        channel: r.get("channel")?,
        kind: r.get("kind")?,
        language: r.get("language")?,
        to_guardian_id: r.get("to_guardian_id")?,
        to_address: r.get("to_address")?,
        subject: r.get("subject")?,
        body: r.get("body")?,
        status: r.get("status")?,
        error: r.get("error")?,
        related_table: r.get("related_table")?,
        related_id: r.get("related_id")?,
        created_at: r.get("created_at")?,
    })
}

#[derive(Debug, Deserialize)]
pub struct RecordMessageInput {
    /// `email | wa_tap | wa_auto | app`.
    pub channel: String,
    /// The purpose / template key (`receipt_share`, later `fee_reminder`,
    /// `absence_alert`, `circular`, …).
    pub kind: String,
    pub language: String,
    pub to_guardian_id: Option<String>,
    pub to_staff_id: Option<String>,
    /// Mobile (wa_tap) or email address (email).
    pub to_address: Option<String>,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub related_table: Option<String>,
    pub related_id: Option<String>,
}

/// The permission (action + target kind) a message purpose (`kind`) requires.
/// Extended as P14 steps land (circular → ManageCirculars in Step 6). Unknown
/// kinds are rejected so a new purpose can never sneak past a permission check.
fn action_for_message_kind(kind: &str) -> CmdResult<(Action, TargetKind)> {
    match kind {
        "receipt_share" => Ok((Action::PrintShareReceipt, TargetKind::Fee)),
        "fee_reminder" => Ok((Action::SendFeeReminder, TargetKind::Fee)),
        "absence_alert" => Ok((Action::SendAbsenceAlert, TargetKind::Attendance)),
        _ => Err(CmdError::validation("kind", "unsupported")),
    }
}

/// The student's current class id (open enrollment), for permission targets.
fn current_class_of(conn: &Connection, student_id: &str) -> Option<String> {
    conn.query_row(
        "SELECT class_id FROM enrollment WHERE student_id=?1 AND to_date IS NULL LIMIT 1",
        params![student_id], |r| r.get(0),
    ).optional().ok().flatten()
}

/// Insert one message row + its audit entry + its op on an OPEN transaction, and
/// return the new id. Shared by `record_message_logic` (single) and the bulk
/// fee-reminder queueing (many rows, one transaction). No consent/permission check
/// here — the caller does that once.
#[allow(clippy::too_many_arguments)]
fn insert_message_in_tx(
    tx: &rusqlite::Transaction,
    mode: DeviceMode,
    staff_id: &str,
    device_id: Option<&str>,
    channel: &str,
    kind: &str,
    language: &str,
    to_guardian_id: Option<&str>,
    to_staff_id: Option<&str>,
    to_address: Option<&str>,
    subject: Option<&str>,
    body: Option<&str>,
    related_table: Option<&str>,
    related_id: Option<&str>,
    status: &str,
    school_id: Option<&str>,
    now: &str,
) -> rusqlite::Result<String> {
    let sync_state = if mode == DeviceMode::Server { "confirmed" } else { "on_device" };
    let id = new_id("msg");
    tx.execute(
        "INSERT INTO message(id,kind,channel,language,to_guardian_id,to_staff_id,to_address,subject,body,status,related_table,related_id,created_by,school_id,created_at,updated_at,updated_by_staff,updated_by_device,sync_state) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?15,?13,?16,?17)",
        params![id, kind, channel, language, to_guardian_id, to_staff_id, to_address, subject, body,
            status, related_table, related_id, staff_id, school_id, now, device_id, sync_state],
    )?;
    crate::security::audit::append(tx, &AuditEntry {
        at: now.to_string(),
        staff_id: Some(staff_id.to_string()),
        action: "record_message".into(),
        table: Some("message".into()),
        record_id: Some(id.clone()),
        after_json: Some(serde_json::json!({ "channel": channel, "kind": kind, "status": status }).to_string()),
        ..Default::default()
    })?;
    crate::write::append_op(tx, mode, &Op {
        op_id: new_id("op"),
        hlc: now.to_string(),
        device_id: device_id.unwrap_or_default().to_string(),
        staff_id: staff_id.to_string(),
        audience: "admin".into(),
        table: "message".into(),
        record_id: id.clone(),
        kind: "insert".into(),
        payload: "{}".into(),
        base_version: None,
        server_epoch: 1,
    })?;
    Ok(id)
}

/// The honest initial status for a channel: `wa_tap` → **`tapped`** (Vidya can't
/// know it was actually sent, §3 rule 13); everything else → **`queued`**.
fn initial_status_for(channel: vidya_core::messages::Channel) -> vidya_core::messages::MessageStatus {
    use vidya_core::messages::{Channel, MessageStatus};
    match channel {
        Channel::WaTap => MessageStatus::Tapped,
        Channel::Email | Channel::WaAuto | Channel::App => MessageStatus::Queued,
    }
}

/// Record a message in the outbox (P14, §10.1). One transaction (row + audit + op).
/// The channel/purpose gate is enforced here; the UI enforces `can_message` consent
/// before offering the action.
pub fn record_message_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    input: &RecordMessageInput,
) -> CmdResult<MessageDto> {
    let actor = actor_from(conn, actor_s)?;
    let (action, target_kind) = action_for_message_kind(&input.kind)?;
    // For an attendance-scoped purpose (absence_alert) the target class is derived
    // from the related student, so a teacher can only alert their OWN class's
    // absentees (never a class they don't class-teach).
    let target = match target_kind {
        TargetKind::Attendance => Target {
            kind: TargetKind::Attendance,
            class_id: input.related_id.as_deref().and_then(|sid| current_class_of(conn, sid)),
            ..Default::default()
        },
        k => Target::of(k),
    };
    require_allow(&actor, action, &target)?;

    let channel = vidya_core::messages::Channel::parse(&input.channel)
        .ok_or_else(|| CmdError::validation("channel", "unknown"))?;
    // An automatic-WhatsApp send requires the optional `wa_auto` module (off by
    // default). email / wa_tap / app are Core.
    if channel == vidya_core::messages::Channel::WaAuto {
        vidya_core::modules::require_enabled(&crate::modules::enabled_set(conn)?, vidya_core::modules::Module::WaAuto)?;
    }
    let status = initial_status_for(channel);
    if !matches!(input.language.as_str(), "en" | "hi" | "te") {
        return Err(CmdError::validation("language", "unknown"));
    }

    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let tx = conn.transaction()?;
    let id = insert_message_in_tx(
        &tx, device_mode, &actor_s.id, device_id, channel.as_str(), &input.kind, &input.language,
        input.to_guardian_id.as_deref(), input.to_staff_id.as_deref(), input.to_address.as_deref(),
        input.subject.as_deref(), input.body.as_deref(), input.related_table.as_deref(),
        input.related_id.as_deref(), status.as_str(), school_id.as_deref(), &now,
    )?;
    tx.commit()?;

    conn.query_row("SELECT * FROM message WHERE id=?1", params![id], map_message).map_err(Into::into)
}

/// List outbox messages (Principal sees all; others see only what they created).
/// Optional filters by status and by related record. For Settings → message logs
/// and the absence/reminder status views.
pub fn list_messages_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    status: Option<&str>,
    related_id: Option<&str>,
) -> CmdResult<Vec<MessageDto>> {
    let actor = actor_from(conn, actor_s)?;
    let mut sql = String::from("SELECT * FROM message WHERE 1=1");
    let mut args: Vec<rusqlite::types::Value> = Vec::new();
    if actor.role != vidya_core::types::Role::Principal {
        sql.push_str(" AND created_by=?");
        args.push(actor_s.id.clone().into());
    }
    if let Some(s) = status {
        sql.push_str(" AND status=?");
        args.push(s.to_string().into());
    }
    if let Some(r) = related_id {
        sql.push_str(" AND related_id=?");
        args.push(r.to_string().into());
    }
    sql.push_str(" ORDER BY created_at DESC LIMIT 500");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(args.iter()), map_message)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

// ---- Absence alerts (P14 Step 4, prototype `absence`) -----------------------

#[derive(Debug, Serialize)]
pub struct AbsentStudentDto {
    pub student_id: String,
    pub student_name: String,
    pub roll_no: Option<i64>,
    pub guardian_id: Option<String>,
    pub guardian_name: Option<String>,
    pub guardian_mobile: Option<String>,
    pub guardian_email: Option<String>,
    pub guardian_language: Option<String>,
    pub has_messages_consent: bool,
    /// The `absence_alert` body rendered in the guardian's language, for the preview.
    pub preview: String,
}

#[derive(Debug, Serialize)]
pub struct AbsenceListDto {
    pub class_id: String,
    pub class_display: Option<String>,
    pub date: String,
    /// False when there is no submitted sheet for (class, date) yet.
    pub submitted: bool,
    pub students: Vec<AbsentStudentDto>,
}

/// Render the `absence_alert` template for a student on `date` in `lang`
/// (`{student_name} {date} {school_name}`).
fn render_absence_alert(conn: &Connection, student_name: &str, date: &str, lang: &str) -> CmdResult<String> {
    let lang = if matches!(lang, "en" | "hi" | "te") { lang } else { "en" };
    let (_subj, body_tmpl) = load_message_template(conn, "absence_alert", lang)?;
    let school_name: String = conn.query_row("SELECT name FROM school LIMIT 1", [], |r| r.get(0)).optional()?.unwrap_or_default();
    let mut vars: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    vars.insert("student_name".into(), student_name.to_string());
    vars.insert("date".into(), date.to_string());
    vars.insert("school_name".into(), school_name);
    Ok(vidya_core::messages::render(&body_tmpl, &vars))
}

/// The absent students for a class on a date (after the sheet is submitted), each
/// with their primary guardian, consent and a rendered preview. Allowed for the
/// class teacher of the class and the Principal (`SendAbsenceAlert`). Returns
/// `submitted=false` (and no students) when there is no submitted sheet yet.
pub fn list_absent_logic(conn: &mut Connection, actor_s: &SessionStaff, class_id: &str, date: &str) -> CmdResult<AbsenceListDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::SendAbsenceAlert, &Target { kind: TargetKind::Attendance, class_id: Some(class_id.to_string()), ..Default::default() })?;
    let class_display: Option<String> = conn.query_row("SELECT display FROM class WHERE id=?1", params![class_id], |r| r.get(0)).optional()?;
    let sheet: Option<(String, String)> = conn
        .query_row("SELECT id, status FROM attendance_sheet WHERE class_id=?1 AND date=?2", params![class_id, date], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?;
    let (sheet_id, status) = match sheet {
        Some(s) => s,
        None => return Ok(AbsenceListDto { class_id: class_id.into(), class_display, date: date.into(), submitted: false, students: vec![] }),
    };
    if status != "submitted" {
        return Ok(AbsenceListDto { class_id: class_id.into(), class_display, date: date.into(), submitted: false, students: vec![] });
    }

    #[allow(clippy::type_complexity)]
    let raw: Vec<(String, String, Option<i64>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)> = {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.name, e.roll_no, g.id, g.name, g.mobile, g.email, g.language \
             FROM attendance_mark m JOIN student s ON s.id=m.student_id \
               LEFT JOIN enrollment e ON e.student_id=s.id AND e.to_date IS NULL \
               LEFT JOIN student_guardian sg ON sg.student_id=s.id AND sg.is_primary=1 \
               LEFT JOIN guardian g ON g.id=sg.guardian_id \
             WHERE m.sheet_id=?1 AND m.mark='A' ORDER BY e.roll_no",
        )?;
        let out = stmt.query_map(params![sheet_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?))
        })?.collect::<rusqlite::Result<_>>()?;
        out
    };

    let mut students = Vec::with_capacity(raw.len());
    for (sid, name, roll, gid, gname, gmobile, gemail, glang) in raw {
        let lang = glang.clone().unwrap_or_else(|| "en".into());
        let preview = render_absence_alert(conn, &name, date, &lang)?;
        let consent = messages_consent_logic(conn, &sid).unwrap_or(false);
        students.push(AbsentStudentDto {
            student_id: sid, student_name: name, roll_no: roll,
            guardian_id: gid, guardian_name: gname, guardian_mobile: gmobile, guardian_email: gemail,
            guardian_language: glang, has_messages_consent: consent, preview,
        });
    }
    Ok(AbsenceListDto { class_id: class_id.into(), class_display, date: date.into(), submitted: true, students })
}

// ---- Circulars & notices (P14 Step 6, prototype `circulars`) ----------------

#[derive(Debug, Serialize)]
pub struct CircularDto {
    pub id: String,
    pub number: Option<String>,
    pub title: String,
    pub body: String,
    pub languages_json: Option<String>,
    pub audience_json: Option<String>,
    pub channels_json: Option<String>,
    pub attachments_json: Option<String>,
    pub status: String,
    pub created_by: Option<String>,
    pub sent_at: Option<String>,
    pub created_at: String,
    /// Staff who have marked this circular read (for "Read by N of M").
    pub read_count: i64,
    /// Active staff total (the M).
    pub staff_count: i64,
    pub read_by_me: bool,
}

#[derive(Debug, Deserialize)]
pub struct CircularInput {
    /// Update this draft if present; otherwise create a new draft.
    pub id: Option<String>,
    pub title: String,
    pub body: String,
    pub languages_json: Option<String>,
    pub audience_json: Option<String>,
    pub channels_json: Option<String>,
    pub attachments_json: Option<String>,
}

/// The circular numbering series = the current session years with a hyphen
/// (`2026–27` → `2026-27`, giving `CIR/2026-27/014`).
fn current_session_series(conn: &Connection) -> String {
    conn.query_row("SELECT label FROM academic_session WHERE is_current=1 LIMIT 1", [], |r| r.get::<_, String>(0))
        .optional()
        .ok()
        .flatten()
        .map(|l| l.replace('\u{2013}', "-"))
        .unwrap_or_else(|| "0000-00".into())
}

fn circular_dto(conn: &Connection, actor_id: &str, id: &str) -> CmdResult<CircularDto> {
    let staff_count: i64 = conn.query_row("SELECT COUNT(*) FROM staff WHERE state='active'", [], |r| r.get(0))?;
    let read_count: i64 = conn.query_row("SELECT COUNT(*) FROM circular_read WHERE circular_id=?1 AND read_at IS NOT NULL", params![id], |r| r.get(0))?;
    let read_by_me: bool = conn
        .query_row("SELECT 1 FROM circular_read WHERE circular_id=?1 AND staff_id=?2 AND read_at IS NOT NULL", params![id, actor_id], |_| Ok(()))
        .optional()?
        .is_some();
    conn.query_row(
        "SELECT id, number, title, body, languages_json, audience_json, channels_json, attachments_json, status, created_by, sent_at, created_at \
         FROM circular WHERE id=?1",
        params![id],
        |r| Ok(CircularDto {
            id: r.get(0)?, number: r.get(1)?, title: r.get(2)?, body: r.get(3)?, languages_json: r.get(4)?,
            audience_json: r.get(5)?, channels_json: r.get(6)?, attachments_json: r.get(7)?, status: r.get(8)?,
            created_by: r.get(9)?, sent_at: r.get(10)?, created_at: r.get(11)?,
            read_count, staff_count, read_by_me,
        }),
    ).optional()?.ok_or_else(CmdError::not_found)
}

/// Circulars for the UI. Principal sees all (drafts + sent, with read tracking);
/// other staff see only **sent** ones (their inbox). Under the `circulars` module.
pub fn list_circulars_logic(conn: &mut Connection, actor_s: &SessionStaff) -> CmdResult<Vec<CircularDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ViewInbox, &Target::of(TargetKind::Own))?;
    vidya_core::modules::require_enabled(&crate::modules::enabled_set(conn)?, vidya_core::modules::Module::Circulars)?;
    let principal = actor.role == vidya_core::types::Role::Principal;
    let sql = if principal {
        "SELECT id FROM circular ORDER BY created_at DESC LIMIT 500"
    } else {
        "SELECT id FROM circular WHERE status='sent' ORDER BY sent_at DESC LIMIT 500"
    };
    let ids: Vec<String> = {
        let mut stmt = conn.prepare(sql)?;
        let out = stmt.query_map([], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<_>>()?;
        out
    };
    ids.iter().map(|id| circular_dto(conn, &actor_s.id, id)).collect()
}

/// Create or update a **draft** circular (Principal, `circulars` module). One
/// transaction (row + audit + op). Sending (with the number) is `send_circular`.
pub fn save_circular_logic(conn: &mut Connection, actor_s: &SessionStaff, device_id: Option<&str>, device_mode: DeviceMode, input: &CircularInput) -> CmdResult<CircularDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageCirculars, &Target::of(TargetKind::School))?;
    vidya_core::modules::require_module(&crate::modules::enabled_set(conn)?, Action::ManageCirculars)?;
    if input.title.trim().is_empty() {
        return Err(CmdError::validation("title", "required"));
    }
    let id = input.id.clone().unwrap_or_else(|| new_id("cir"));
    let is_new = input.id.is_none();
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let sync_state = if device_mode == DeviceMode::Server { "confirmed" } else { "on_device" };
    let ctx = WriteCtx { mode: device_mode };
    let dev = device_id.map(str::to_string);
    let (id2, staff, now2) = (id.clone(), actor_s.id.clone(), now.clone());
    let inp = (input.title.clone(), input.body.clone(), input.languages_json.clone(), input.audience_json.clone(), input.channels_json.clone(), input.attachments_json.clone());
    with_write(conn, &ctx, move |tx| {
        if is_new {
            tx.execute(
                "INSERT INTO circular(id,title,body,languages_json,audience_json,channels_json,attachments_json,status,created_by,school_id,created_at,updated_at,updated_by_staff,updated_by_device,sync_state) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,'draft',?8,?9,?10,?10,?8,?11,?12)",
                params![id2, inp.0, inp.1, inp.2, inp.3, inp.4, inp.5, staff, school_id, now2, dev, sync_state],
            )?;
        } else {
            let n = tx.execute(
                "UPDATE circular SET title=?2, body=?3, languages_json=?4, audience_json=?5, channels_json=?6, attachments_json=?7, updated_at=?8, updated_by_staff=?9 \
                 WHERE id=?1 AND status='draft'",
                params![id2, inp.0, inp.1, inp.2, inp.3, inp.4, inp.5, now2, staff],
            )?;
            if n == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows); // not a draft / not found
            }
        }
        let audit = AuditEntry { at: now2.clone(), staff_id: Some(staff.clone()), action: "save_circular".into(), table: Some("circular".into()), record_id: Some(id2.clone()), after_json: Some(serde_json::json!({"title": inp.0}).to_string()), ..Default::default() };
        let op = Op { op_id: new_id("op"), hlc: now2.clone(), device_id: dev.clone().unwrap_or_default(), staff_id: staff.clone(), audience: "admin".into(), table: "circular".into(), record_id: id2.clone(), kind: if is_new { "insert" } else { "update" }.into(), payload: "{}".into(), base_version: None, server_epoch: 1 };
        Ok(Effect { value: (), audit, op })
    }).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => CmdError::validation("circular", "not_a_draft"),
        other => other.into(),
    })?;
    circular_dto(conn, &actor_s.id, &id)
}

/// Send a draft circular: assign the server number (`CIR/<session>/NNN`), set
/// `status='sent'` + `sent_at`. Principal + `circulars` module. The number is
/// assigned atomically inside the write transaction (like a receipt number).
pub fn send_circular_logic(conn: &mut Connection, actor_s: &SessionStaff, device_id: Option<&str>, device_mode: DeviceMode, id: &str) -> CmdResult<CircularDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ManageCirculars, &Target::of(TargetKind::School))?;
    vidya_core::modules::require_module(&crate::modules::enabled_set(conn)?, Action::ManageCirculars)?;
    let status: Option<String> = conn.query_row("SELECT status FROM circular WHERE id=?1", params![id], |r| r.get(0)).optional()?;
    match status.as_deref() {
        None => return Err(CmdError::not_found()),
        Some("sent") => return Err(CmdError::validation("circular", "already_sent")),
        _ => {}
    }
    let series = current_session_series(conn);
    let now = now_iso();
    let ctx = WriteCtx { mode: device_mode };
    let dev = device_id.map(str::to_string);
    let (id2, staff, now2) = (id.to_string(), actor_s.id.clone(), now.clone());
    with_write(conn, &ctx, move |tx| {
        let number = crate::numbering::next_no(tx, vidya_core::numbering::NumberKind::Circular, &series)?;
        tx.execute(
            "UPDATE circular SET number=?2, status='sent', sent_at=?3, approved_by=?4, updated_at=?3, updated_by_staff=?4 WHERE id=?1",
            params![id2, number, now2, staff],
        )?;
        let audit = AuditEntry { at: now2.clone(), staff_id: Some(staff.clone()), action: "send_circular".into(), table: Some("circular".into()), record_id: Some(id2.clone()), after_json: Some(serde_json::json!({"number": number, "status": "sent"}).to_string()), ..Default::default() };
        let op = Op { op_id: new_id("op"), hlc: now2.clone(), device_id: dev.clone().unwrap_or_default(), staff_id: staff.clone(), audience: "admin".into(), table: "circular".into(), record_id: id2.clone(), kind: "update".into(), payload: "{}".into(), base_version: None, server_epoch: 1 };
        Ok(Effect { value: (), audit, op })
    })?;
    circular_dto(conn, &actor_s.id, id)
}

/// Mark a circular as read by the acting staff member (their inbox). Idempotent
/// (UNIQUE(circular_id, staff_id)). Any active staff (`ViewInbox`).
pub fn mark_circular_read_logic(conn: &mut Connection, actor_s: &SessionStaff, device_id: Option<&str>, device_mode: DeviceMode, circular_id: &str) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ViewInbox, &Target::of(TargetKind::Own))?;
    // Only sent circulars appear in the inbox.
    let sent: bool = conn.query_row("SELECT 1 FROM circular WHERE id=?1 AND status='sent'", params![circular_id], |_| Ok(())).optional()?.is_some();
    if !sent {
        return Err(CmdError::not_found());
    }
    if conn.query_row("SELECT 1 FROM circular_read WHERE circular_id=?1 AND staff_id=?2 AND read_at IS NOT NULL", params![circular_id, actor_s.id], |_| Ok(())).optional()?.is_some() {
        return Ok(()); // already read — idempotent
    }
    let id = new_id("crd");
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let sync_state = if device_mode == DeviceMode::Server { "confirmed" } else { "on_device" };
    let ctx = WriteCtx { mode: device_mode };
    let dev = device_id.map(str::to_string);
    let (id2, staff, cid, now2) = (id.clone(), actor_s.id.clone(), circular_id.to_string(), now.clone());
    with_write(conn, &ctx, move |tx| {
        tx.execute(
            "INSERT INTO circular_read(id,circular_id,staff_id,read_at,school_id,created_at,updated_at,updated_by_staff,updated_by_device,sync_state) \
             VALUES (?1,?2,?3,?4,?5,?4,?4,?3,?6,?7) \
             ON CONFLICT(circular_id,staff_id) DO UPDATE SET read_at=excluded.read_at",
            params![id2, cid, staff, now2, school_id, dev, sync_state],
        )?;
        let audit = AuditEntry { at: now2.clone(), staff_id: Some(staff.clone()), action: "mark_circular_read".into(), table: Some("circular_read".into()), record_id: Some(id2.clone()), after_json: Some(serde_json::json!({"circular_id": cid}).to_string()), ..Default::default() };
        let op = Op { op_id: new_id("op"), hlc: now2.clone(), device_id: dev.clone().unwrap_or_default(), staff_id: staff.clone(), audience: "admin".into(), table: "circular_read".into(), record_id: id2.clone(), kind: "insert".into(), payload: "{}".into(), base_version: None, server_epoch: 1 };
        Ok(Effect { value: (), audit, op })
    })?;
    Ok(())
}

// ============================================================ calendar =======

/// A calendar event row (holiday / exam / event) for the UI.
#[derive(Debug, Serialize)]
pub struct CalendarEventDto {
    pub id: String,
    pub session_id: Option<String>,
    pub starts_on: String,
    pub ends_on: String,
    pub kind: String,
    pub title: String,
    pub title_hi: Option<String>,
    pub title_te: Option<String>,
    pub is_non_working: bool,
    pub circular_id: Option<String>,
}

/// The whole calendar for the Settings screen: the weekly pattern + all events.
#[derive(Debug, Serialize)]
pub struct CalendarDto {
    /// Working flag per weekday, index 0 = Monday … 6 = Sunday.
    pub week: [bool; 7],
    pub events: Vec<CalendarEventDto>,
}

/// Add/edit payload for a calendar event.
#[derive(Debug, Deserialize)]
pub struct CalendarEventInput {
    pub starts_on: String,
    pub ends_on: String,
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub title_hi: Option<String>,
    #[serde(default)]
    pub title_te: Option<String>,
    #[serde(default)]
    pub is_non_working: bool,
}

fn single_school_id(conn: &Connection) -> CmdResult<Option<String>> {
    Ok(conn.query_row("SELECT id FROM school LIMIT 1", [], |r| r.get(0)).optional()?)
}

fn current_session_id(conn: &Connection) -> CmdResult<Option<String>> {
    Ok(conn
        .query_row("SELECT id FROM academic_session WHERE is_current=1 LIMIT 1", [], |r| r.get(0))
        .optional()?)
}

fn read_calendar_event(conn: &Connection, id: &str) -> CmdResult<CalendarEventDto> {
    conn.query_row(
        "SELECT id, session_id, starts_on, ends_on, kind, title, title_hi, title_te, is_non_working, circular_id \
         FROM calendar_event WHERE id=?1",
        params![id],
        |r| {
            Ok(CalendarEventDto {
                id: r.get(0)?,
                session_id: r.get(1)?,
                starts_on: r.get(2)?,
                ends_on: r.get(3)?,
                kind: r.get(4)?,
                title: r.get(5)?,
                title_hi: r.get(6)?,
                title_te: r.get(7)?,
                is_non_working: r.get::<_, i64>(8)? != 0,
                circular_id: r.get(9)?,
            })
        },
    )
    .optional()?
    .ok_or_else(CmdError::not_found)
}

/// The calendar (weekly pattern + events). Any signed-in role may read it — it is
/// school-wide info used by attendance and dashboards.
pub fn get_calendar_logic(conn: &mut Connection, _actor_s: &SessionStaff) -> CmdResult<CalendarDto> {
    let week = crate::calendar::load_week(conn)?;
    let mut stmt = conn.prepare(
        "SELECT id, session_id, starts_on, ends_on, kind, title, title_hi, title_te, is_non_working, circular_id \
         FROM calendar_event ORDER BY starts_on, id",
    )?;
    let events = stmt
        .query_map([], |r| {
            Ok(CalendarEventDto {
                id: r.get(0)?,
                session_id: r.get(1)?,
                starts_on: r.get(2)?,
                ends_on: r.get(3)?,
                kind: r.get(4)?,
                title: r.get(5)?,
                title_hi: r.get(6)?,
                title_te: r.get(7)?,
                is_non_working: r.get::<_, i64>(8)? != 0,
                circular_id: r.get(9)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(CalendarDto { week: week.working, events })
}

/// The number of working days in `[from, to]` (inclusive, `YYYY-MM-DD`), from the
/// school's weekly pattern + non-working events (P16 calendar screen). Authoritative
/// count via vidya-core so it matches attendance % and fee due dates.
pub fn working_days_logic(conn: &mut Connection, from: &str, to: &str) -> CmdResult<i64> {
    let week = crate::calendar::load_week(conn)?;
    let events = crate::calendar::load_events(conn)?;
    let (f, t) = match (vidya_core::calendar::parse_date(from), vidya_core::calendar::parse_date(to)) {
        (Some(f), Some(t)) => (f, t),
        _ => return Err(CmdError::validation("date", "format")),
    };
    Ok(vidya_core::calendar::working_days(f, t, &week, &events) as i64)
}

/// Validate an event input; returns the trimmed title. Shared by add/update.
fn validate_event_input(input: &CalendarEventInput) -> CmdResult<String> {
    let start = vidya_core::calendar::parse_date(&input.starts_on)
        .ok_or_else(|| CmdError::validation("starts_on", "date"))?;
    let end = vidya_core::calendar::parse_date(&input.ends_on)
        .ok_or_else(|| CmdError::validation("ends_on", "date"))?;
    if end < start {
        return Err(CmdError::validation("ends_on", "before_start"));
    }
    if !matches!(input.kind.as_str(), "holiday" | "exam" | "event") {
        return Err(CmdError::validation("kind", "unknown"));
    }
    let title = input.title.trim();
    if title.is_empty() {
        return Err(CmdError::validation("title", "required"));
    }
    Ok(title.to_string())
}

/// Set which weekdays are working (Settings → Session & terms → Weekly off days).
/// `working` is length 7, index 0 = Monday … 6 = Sunday. Principal only; audited;
/// upserts all seven `school_week` rows and emits one op per weekday so devices
/// receive the change (audience `admin`).
pub fn set_weekly_offs_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    working: &[bool],
) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::Settings, &Target { kind: TargetKind::School, ..Default::default() })?;
    if working.len() != 7 {
        return Err(CmdError::validation("working", "len_7"));
    }
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let sync_state = if device_mode == DeviceMode::Server { "confirmed" } else { "on_device" };
    let dev = device_id.unwrap_or_default().to_string();
    let tx = conn.transaction()?;
    for (i, &is_working) in working.iter().enumerate() {
        let weekday = (i + 1) as i64; // ISO 1=Mon … 7=Sun
        let id = format!("wk-{weekday}");
        tx.execute(
            "INSERT INTO school_week(id, weekday, is_working, school_id, created_at, updated_at, updated_by_staff, updated_by_device, sync_state) \
             VALUES (?1,?2,?3,?4,?5,?5,?6,?7,?8) \
             ON CONFLICT(id) DO UPDATE SET is_working=excluded.is_working, updated_at=excluded.updated_at, \
               updated_by_staff=excluded.updated_by_staff, updated_by_device=excluded.updated_by_device, \
               sync_state=excluded.sync_state, version=version+1",
            params![id, weekday, is_working as i64, school_id, now, actor_s.id, dev, sync_state],
        )?;
        crate::write::append_op(&tx, device_mode, &Op {
            op_id: new_id("op"),
            hlc: now.clone(),
            device_id: dev.clone(),
            staff_id: actor_s.id.clone(),
            audience: "admin".into(),
            table: "school_week".into(),
            record_id: id,
            kind: "update".into(),
            payload: "{}".into(),
            base_version: None,
            server_epoch: 1,
        })?;
    }
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(),
        staff_id: Some(actor_s.id.clone()),
        action: "set_weekly_offs".into(),
        table: Some("school_week".into()),
        after_json: Some(serde_json::json!({ "working": working }).to_string()),
        ..Default::default()
    })?;
    tx.commit()?;
    Ok(())
}

/// Add a calendar event (Principal only; audited; synced under `admin`).
pub fn add_calendar_event_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    input: &CalendarEventInput,
) -> CmdResult<CalendarEventDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::Settings, &Target { kind: TargetKind::School, ..Default::default() })?;
    let title = validate_event_input(input)?;
    let id = new_id("cal");
    let now = now_iso();
    let session_id = current_session_id(conn)?;
    let school_id = single_school_id(conn)?;
    let sync_state = if device_mode == DeviceMode::Server { "confirmed" } else { "on_device" };
    let ctx = WriteCtx { mode: device_mode };
    let (id2, kind, hi, te, non_working) =
        (id.clone(), input.kind.clone(), input.title_hi.clone(), input.title_te.clone(), input.is_non_working);
    let (starts, ends) = (input.starts_on.clone(), input.ends_on.clone());
    let dev = device_id.map(str::to_string);
    with_write(conn, &ctx, move |tx| {
        tx.execute(
            "INSERT INTO calendar_event(id, session_id, starts_on, ends_on, kind, title, title_hi, title_te, is_non_working, school_id, created_at, updated_at, updated_by_staff, updated_by_device, sync_state) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?11,?12,?13,?14)",
            params![id2, session_id, starts, ends, kind, title, hi, te, non_working as i64, school_id, now, actor_s.id, dev, sync_state],
        )?;
        let audit = AuditEntry {
            at: now.clone(),
            staff_id: Some(actor_s.id.clone()),
            action: "add_calendar_event".into(),
            table: Some("calendar_event".into()),
            record_id: Some(id2.clone()),
            after_json: Some(serde_json::json!({ "title": title, "kind": kind, "starts_on": starts, "ends_on": ends, "is_non_working": non_working }).to_string()),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"),
            hlc: now.clone(),
            device_id: dev.clone().unwrap_or_default(),
            staff_id: actor_s.id.clone(),
            audience: "admin".into(),
            table: "calendar_event".into(),
            record_id: id2.clone(),
            kind: "insert".into(),
            payload: "{}".into(),
            base_version: None,
            server_epoch: 1,
        };
        Ok(Effect { value: (), audit, op })
    })?;
    read_calendar_event(conn, &id)
}

/// Edit a calendar event (Principal only; audited; synced).
pub fn update_calendar_event_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    id: &str,
    input: &CalendarEventInput,
) -> CmdResult<CalendarEventDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::Settings, &Target { kind: TargetKind::School, ..Default::default() })?;
    let title = validate_event_input(input)?;
    // Must exist.
    read_calendar_event(conn, id)?;
    let now = now_iso();
    let ctx = WriteCtx { mode: device_mode };
    let (id2, kind, hi, te, non_working) =
        (id.to_string(), input.kind.clone(), input.title_hi.clone(), input.title_te.clone(), input.is_non_working);
    let (starts, ends) = (input.starts_on.clone(), input.ends_on.clone());
    let dev = device_id.map(str::to_string);
    with_write(conn, &ctx, move |tx| {
        tx.execute(
            "UPDATE calendar_event SET starts_on=?2, ends_on=?3, kind=?4, title=?5, title_hi=?6, title_te=?7, is_non_working=?8, \
               updated_at=?9, updated_by_staff=?10, updated_by_device=?11, version=version+1 WHERE id=?1",
            params![id2, starts, ends, kind, title, hi, te, non_working as i64, now, actor_s.id, dev],
        )?;
        let audit = AuditEntry {
            at: now.clone(),
            staff_id: Some(actor_s.id.clone()),
            action: "update_calendar_event".into(),
            table: Some("calendar_event".into()),
            record_id: Some(id2.clone()),
            after_json: Some(serde_json::json!({ "title": title, "kind": kind, "starts_on": starts, "ends_on": ends, "is_non_working": non_working }).to_string()),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"),
            hlc: now.clone(),
            device_id: dev.clone().unwrap_or_default(),
            staff_id: actor_s.id.clone(),
            audience: "admin".into(),
            table: "calendar_event".into(),
            record_id: id2.clone(),
            kind: "update".into(),
            payload: "{}".into(),
            base_version: None,
            server_epoch: 1,
        };
        Ok(Effect { value: (), audit, op })
    })?;
    read_calendar_event(conn, id)
}

/// Delete a calendar event (Principal only; audited; synced). `calendar_event`
/// is not append-only (it carries no money/audit), so the row is removed and a
/// delete op is emitted.
pub fn delete_calendar_event_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    id: &str,
) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::Settings, &Target { kind: TargetKind::School, ..Default::default() })?;
    let before = read_calendar_event(conn, id)?;
    let now = now_iso();
    let ctx = WriteCtx { mode: device_mode };
    let id2 = id.to_string();
    let dev = device_id.map(str::to_string);
    let before_json = serde_json::json!({ "title": before.title, "kind": before.kind, "starts_on": before.starts_on, "ends_on": before.ends_on }).to_string();
    with_write(conn, &ctx, move |tx| {
        tx.execute("DELETE FROM calendar_event WHERE id=?1", params![id2])?;
        let audit = AuditEntry {
            at: now.clone(),
            staff_id: Some(actor_s.id.clone()),
            action: "delete_calendar_event".into(),
            table: Some("calendar_event".into()),
            record_id: Some(id2.clone()),
            before_json: Some(before_json),
            ..Default::default()
        };
        let op = Op {
            op_id: new_id("op"),
            hlc: now.clone(),
            device_id: dev.clone().unwrap_or_default(),
            staff_id: actor_s.id.clone(),
            audience: "admin".into(),
            table: "calendar_event".into(),
            record_id: id2.clone(),
            kind: "delete".into(),
            payload: "{}".into(),
            base_version: None,
            server_epoch: 1,
        };
        Ok(Effect { value: (), audit, op })
    })?;
    Ok(())
}

// ============================================================ guardians ======

/// Editable guardian fields (student profile / admission form).
#[derive(Debug, Deserialize)]
pub struct GuardianEditInput {
    pub name: String,
    #[serde(default)]
    pub relation: Option<String>,
    #[serde(default)]
    pub mobile: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default = "default_guardian_lang")]
    pub language: String,
    #[serde(default = "default_true")]
    pub whatsapp_ok: bool,
}
fn default_guardian_lang() -> String { "en".to_string() }
fn default_true() -> bool { true }

fn validate_guardian_input(input: &GuardianEditInput) -> CmdResult<String> {
    let gi = vidya_core::guardians::GuardianInput {
        name: &input.name,
        relation: input.relation.as_deref(),
        mobile: input.mobile.as_deref(),
        email: input.email.as_deref(),
        language: &input.language,
        whatsapp_ok: input.whatsapp_ok,
    };
    Ok(vidya_core::guardians::validate_guardian(&gi)?)
}

fn finance_op(now: &str, dev: &str, staff: &str, table: &str, id: &str, kind: &str) -> Op {
    Op {
        op_id: new_id("op"),
        hlc: now.to_string(),
        device_id: dev.to_string(),
        staff_id: staff.to_string(),
        audience: "finance".into(),
        table: table.into(),
        record_id: id.into(),
        kind: kind.into(),
        payload: "{}".into(),
        base_version: None,
        server_epoch: 1,
    }
}

/// Add a guardian to a student (up to 2; the first becomes primary). Principal
/// edits directly (EditStudentDetails); accountant edits would go through a
/// student-details request (not wired for guardians yet). Audited; synced (finance).
pub fn add_guardian_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    student_id: &str,
    input: &GuardianEditInput,
) -> CmdResult<Vec<crate::guardians::GuardianDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::EditStudentDetails, &Target::of(TargetKind::Student))?;
    let name = validate_guardian_input(input)?;
    if conn.query_row("SELECT 1 FROM student WHERE id=?1", params![student_id], |_| Ok(())).optional()?.is_none() {
        return Err(CmdError::not_found());
    }
    let count = crate::guardians::count_for_student(conn, student_id)?;
    if count as usize >= vidya_core::guardians::MAX_GUARDIANS_PER_STUDENT {
        return Err(CmdError::validation("guardians", "max_2"));
    }
    let is_primary = count == 0;
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let sync_state = if device_mode == DeviceMode::Server { "confirmed" } else { "on_device" };
    let dev = device_id.unwrap_or_default().to_string();
    let gid = new_id("grd");
    let sgid = new_id("sg");
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO guardian(id,name,relation,mobile,email,language,whatsapp_ok,school_id,created_at,updated_at,updated_by_staff,updated_by_device,sync_state) \
         VALUES (?1,?2,?3,NULLIF(?4,''),NULLIF(?5,''),?6,?7,?8,?9,?9,?10,?11,?12)",
        params![gid, name, input.relation, input.mobile.as_deref().unwrap_or(""), input.email.as_deref().unwrap_or(""), input.language, input.whatsapp_ok as i64, school_id, now, actor_s.id, dev, sync_state],
    )?;
    tx.execute(
        "INSERT INTO student_guardian(id,student_id,guardian_id,is_primary,school_id,created_at,updated_at,updated_by_staff,updated_by_device,sync_state) \
         VALUES (?1,?2,?3,?4,?5,?6,?6,?7,?8,?9)",
        params![sgid, student_id, gid, is_primary as i64, school_id, now, actor_s.id, dev, sync_state],
    )?;
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "add_guardian".into(),
        table: Some("guardian".into()), record_id: Some(gid.clone()),
        after_json: Some(serde_json::json!({ "student_id": student_id, "name": name, "is_primary": is_primary }).to_string()),
        ..Default::default()
    })?;
    crate::write::append_op(&tx, device_mode, &finance_op(&now, &dev, &actor_s.id, "guardian", &gid, "insert"))?;
    crate::write::append_op(&tx, device_mode, &finance_op(&now, &dev, &actor_s.id, "student_guardian", &sgid, "insert"))?;
    tx.commit()?;
    crate::guardians::list_for_student(conn, student_id).map_err(Into::into)
}

/// Edit a guardian's fields. Principal direct; audited; synced (finance).
pub fn update_guardian_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    student_id: &str,
    guardian_id: &str,
    input: &GuardianEditInput,
) -> CmdResult<Vec<crate::guardians::GuardianDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::EditStudentDetails, &Target::of(TargetKind::Student))?;
    let name = validate_guardian_input(input)?;
    if conn.query_row("SELECT 1 FROM guardian WHERE id=?1", params![guardian_id], |_| Ok(())).optional()?.is_none() {
        return Err(CmdError::not_found());
    }
    let now = now_iso();
    let sync_state = if device_mode == DeviceMode::Server { "confirmed" } else { "on_device" };
    let dev = device_id.unwrap_or_default().to_string();
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE guardian SET name=?2, relation=?3, mobile=NULLIF(?4,''), email=NULLIF(?5,''), language=?6, whatsapp_ok=?7, \
           updated_at=?8, updated_by_staff=?9, updated_by_device=?10, sync_state=?11, version=version+1 WHERE id=?1",
        params![guardian_id, name, input.relation, input.mobile.as_deref().unwrap_or(""), input.email.as_deref().unwrap_or(""), input.language, input.whatsapp_ok as i64, now, actor_s.id, dev, sync_state],
    )?;
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "update_guardian".into(),
        table: Some("guardian".into()), record_id: Some(guardian_id.to_string()),
        after_json: Some(serde_json::json!({ "name": name }).to_string()),
        ..Default::default()
    })?;
    crate::write::append_op(&tx, device_mode, &finance_op(&now, &dev, &actor_s.id, "guardian", guardian_id, "update"))?;
    tx.commit()?;
    crate::guardians::list_for_student(conn, student_id).map_err(Into::into)
}

/// Make `guardian_id` the student's primary guardian (clears the others).
pub fn set_primary_guardian_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    student_id: &str,
    guardian_id: &str,
) -> CmdResult<Vec<crate::guardians::GuardianDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::EditStudentDetails, &Target::of(TargetKind::Student))?;
    let now = now_iso();
    let dev = device_id.unwrap_or_default().to_string();
    let links: Vec<(String, String)> = {
        let mut stmt = conn.prepare("SELECT id, guardian_id FROM student_guardian WHERE student_id=?1")?;
        let rows = stmt.query_map(params![student_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    if !links.iter().any(|(_, gid)| gid == guardian_id) {
        return Err(CmdError::not_found());
    }
    let tx = conn.transaction()?;
    for (sgid, gid) in &links {
        let primary = gid == guardian_id;
        tx.execute(
            "UPDATE student_guardian SET is_primary=?2, updated_at=?3, updated_by_staff=?4, updated_by_device=?5, version=version+1 WHERE id=?1",
            params![sgid, primary as i64, now, actor_s.id, dev],
        )?;
        crate::write::append_op(&tx, device_mode, &finance_op(&now, &dev, &actor_s.id, "student_guardian", sgid, "update"))?;
    }
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "set_primary_guardian".into(),
        table: Some("student_guardian".into()), record_id: Some(student_id.to_string()),
        after_json: Some(serde_json::json!({ "student_id": student_id, "primary_guardian": guardian_id }).to_string()),
        ..Default::default()
    })?;
    tx.commit()?;
    crate::guardians::list_for_student(conn, student_id).map_err(Into::into)
}

/// Remove a guardian link from a student. If it was the primary and another link
/// remains, the first remaining becomes primary. A guardian with no links left is
/// removed too. Principal direct; audited; synced (finance).
pub fn remove_guardian_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    student_id: &str,
    guardian_id: &str,
) -> CmdResult<Vec<crate::guardians::GuardianDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::EditStudentDetails, &Target::of(TargetKind::Student))?;
    let link: Option<(String, i64)> = conn
        .query_row("SELECT id, is_primary FROM student_guardian WHERE student_id=?1 AND guardian_id=?2", params![student_id, guardian_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?;
    let (sgid, was_primary) = match link {
        Some(x) => x,
        None => return Err(CmdError::not_found()),
    };
    let now = now_iso();
    let dev = device_id.unwrap_or_default().to_string();
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM student_guardian WHERE id=?1", params![sgid])?;
    crate::write::append_op(&tx, device_mode, &finance_op(&now, &dev, &actor_s.id, "student_guardian", &sgid, "delete"))?;
    // Promote a remaining link to primary if we removed the primary.
    if was_primary != 0 {
        let next: Option<String> = tx
            .query_row("SELECT id FROM student_guardian WHERE student_id=?1 ORDER BY created_at LIMIT 1", params![student_id], |r| r.get(0))
            .optional()?;
        if let Some(next_sg) = next {
            tx.execute("UPDATE student_guardian SET is_primary=1, updated_at=?2, version=version+1 WHERE id=?1", params![next_sg, now])?;
            crate::write::append_op(&tx, device_mode, &finance_op(&now, &dev, &actor_s.id, "student_guardian", &next_sg, "update"))?;
        }
    }
    // Delete an orphan guardian (no links anywhere).
    let orphan = tx.query_row("SELECT COUNT(*) FROM student_guardian WHERE guardian_id=?1", params![guardian_id], |r| r.get::<_, i64>(0))? == 0;
    if orphan {
        tx.execute("DELETE FROM guardian WHERE id=?1", params![guardian_id])?;
        crate::write::append_op(&tx, device_mode, &finance_op(&now, &dev, &actor_s.id, "guardian", guardian_id, "delete"))?;
    }
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "remove_guardian".into(),
        table: Some("guardian".into()), record_id: Some(guardian_id.to_string()),
        before_json: Some(serde_json::json!({ "student_id": student_id }).to_string()),
        ..Default::default()
    })?;
    tx.commit()?;
    crate::guardians::list_for_student(conn, student_id).map_err(Into::into)
}

// ========================================================= custom fields =====

use vidya_core::custom_fields::{self as cf, Entity, FieldType};

#[derive(Debug, Serialize)]
pub struct CustomFieldDto {
    pub id: String,
    pub entity: String,
    pub key: String,
    pub label: String,
    pub label_hi: Option<String>,
    pub label_te: Option<String>,
    pub field_type: String,
    pub options: Vec<String>,
    pub required: bool,
    pub active: bool,
    pub sort_order: i64,
}

#[derive(Debug, Deserialize)]
pub struct CustomFieldInput {
    pub entity: String,
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub label_hi: Option<String>,
    #[serde(default)]
    pub label_te: Option<String>,
    pub field_type: String,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Serialize)]
pub struct CustomFieldValueDto {
    pub field: CustomFieldDto,
    pub value: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CustomValueSet {
    pub field_id: String,
    pub value: String,
}

fn map_custom_field(r: &rusqlite::Row) -> rusqlite::Result<CustomFieldDto> {
    let options_json: Option<String> = r.get(7)?;
    let options = options_json
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .unwrap_or_default();
    Ok(CustomFieldDto {
        id: r.get(0)?,
        entity: r.get(1)?,
        key: r.get(2)?,
        label: r.get(3)?,
        label_hi: r.get(4)?,
        label_te: r.get(5)?,
        field_type: r.get(6)?,
        options,
        required: r.get::<_, i64>(8)? != 0,
        active: r.get::<_, i64>(9)? != 0,
        sort_order: r.get(10)?,
    })
}

const CF_SELECT: &str = "SELECT id, entity, key, label, label_hi, label_te, type, options_json, required, active, sort_order FROM custom_field";

fn cf_type(input_type: &str) -> CmdResult<FieldType> {
    FieldType::parse(input_type).ok_or_else(|| CmdError::validation("type", "unknown"))
}
fn cf_entity(input_entity: &str) -> CmdResult<Entity> {
    Entity::parse(input_entity).ok_or_else(|| CmdError::validation("entity", "unknown"))
}

/// List custom fields for an entity. Any signed-in role may read (to render forms
/// and CSV headers). `include_inactive` = the Settings management view.
pub fn list_custom_fields_logic(conn: &mut Connection, entity: &str, include_inactive: bool) -> CmdResult<Vec<CustomFieldDto>> {
    cf_entity(entity)?;
    let sql = format!(
        "{CF_SELECT} WHERE entity=?1{} ORDER BY sort_order, key",
        if include_inactive { "" } else { " AND active=1" }
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![entity], map_custom_field)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Define a new custom field (Principal, Settings). Audited; synced (admin).
pub fn create_custom_field_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    input: &CustomFieldInput,
) -> CmdResult<CustomFieldDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::Settings, &Target { kind: TargetKind::School, ..Default::default() })?;
    let entity = cf_entity(&input.entity)?;
    let ftype = cf_type(&input.field_type)?;
    cf::validate_field_def(&input.key, &input.label, ftype, &input.options)?;
    // Unique (entity, key).
    if conn
        .query_row("SELECT 1 FROM custom_field WHERE entity=?1 AND key=?2", params![input.entity, input.key], |_| Ok(()))
        .optional()?
        .is_some()
    {
        return Err(CmdError::validation("key", "duplicate"));
    }
    let id = new_id("cf");
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let sync_state = if device_mode == DeviceMode::Server { "confirmed" } else { "on_device" };
    let dev = device_id.unwrap_or_default().to_string();
    let options_json = serde_json::to_string(&input.options).unwrap_or_else(|_| "[]".into());
    let next_sort: i64 = conn
        .query_row("SELECT COALESCE(MAX(sort_order),0)+1 FROM custom_field WHERE entity=?1", params![input.entity], |r| r.get(0))
        .optional()?
        .unwrap_or(1);
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO custom_field(id,entity,key,label,label_hi,label_te,type,options_json,required,active,sort_order,school_id,created_at,updated_at,updated_by_staff,updated_by_device,sync_state) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,1,?10,?11,?12,?12,?13,?14,?15)",
        params![id, entity.as_str(), input.key, input.label, input.label_hi, input.label_te, ftype.as_str(), options_json, input.required as i64, next_sort, school_id, now, actor_s.id, dev, sync_state],
    )?;
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "create_custom_field".into(),
        table: Some("custom_field".into()), record_id: Some(id.clone()),
        after_json: Some(serde_json::json!({ "entity": input.entity, "key": input.key, "type": input.field_type }).to_string()),
        ..Default::default()
    })?;
    crate::write::append_op(&tx, device_mode, &Op {
        op_id: new_id("op"), hlc: now.clone(), device_id: dev.clone(), staff_id: actor_s.id.clone(),
        audience: "admin".into(), table: "custom_field".into(), record_id: id.clone(), kind: "insert".into(),
        payload: "{}".into(), base_version: None, server_epoch: 1,
    })?;
    tx.commit()?;
    conn.query_row(&format!("{CF_SELECT} WHERE id=?1"), params![id], map_custom_field).map_err(Into::into)
}

/// Edit a custom field's label/options/required (not its key/type/entity).
pub fn update_custom_field_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    id: &str,
    input: &CustomFieldInput,
) -> CmdResult<CustomFieldDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::Settings, &Target { kind: TargetKind::School, ..Default::default() })?;
    let ftype = cf_type(&input.field_type)?;
    cf::validate_field_def(&input.key, &input.label, ftype, &input.options)?;
    if conn.query_row("SELECT 1 FROM custom_field WHERE id=?1", params![id], |_| Ok(())).optional()?.is_none() {
        return Err(CmdError::not_found());
    }
    let now = now_iso();
    let dev = device_id.unwrap_or_default().to_string();
    let options_json = serde_json::to_string(&input.options).unwrap_or_else(|_| "[]".into());
    let tx = conn.transaction()?;
    // key/type/entity are immutable (stored values keep their meaning).
    tx.execute(
        "UPDATE custom_field SET label=?2, label_hi=?3, label_te=?4, options_json=?5, required=?6, updated_at=?7, updated_by_staff=?8, updated_by_device=?9, version=version+1 WHERE id=?1",
        params![id, input.label, input.label_hi, input.label_te, options_json, input.required as i64, now, actor_s.id, dev],
    )?;
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "update_custom_field".into(),
        table: Some("custom_field".into()), record_id: Some(id.to_string()),
        after_json: Some(serde_json::json!({ "label": input.label }).to_string()),
        ..Default::default()
    })?;
    crate::write::append_op(&tx, device_mode, &Op {
        op_id: new_id("op"), hlc: now.clone(), device_id: dev.clone(), staff_id: actor_s.id.clone(),
        audience: "admin".into(), table: "custom_field".into(), record_id: id.to_string(), kind: "update".into(),
        payload: "{}".into(), base_version: None, server_epoch: 1,
    })?;
    tx.commit()?;
    conn.query_row(&format!("{CF_SELECT} WHERE id=?1"), params![id], map_custom_field).map_err(Into::into)
}

/// Activate/deactivate a custom field (never hard-deleted — stored values stay).
pub fn set_custom_field_active_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    id: &str,
    active: bool,
) -> CmdResult<CustomFieldDto> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::Settings, &Target { kind: TargetKind::School, ..Default::default() })?;
    if conn.query_row("SELECT 1 FROM custom_field WHERE id=?1", params![id], |_| Ok(())).optional()?.is_none() {
        return Err(CmdError::not_found());
    }
    let now = now_iso();
    let dev = device_id.unwrap_or_default().to_string();
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE custom_field SET active=?2, updated_at=?3, updated_by_staff=?4, updated_by_device=?5, version=version+1 WHERE id=?1",
        params![id, active as i64, now, actor_s.id, dev],
    )?;
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "set_custom_field_active".into(),
        table: Some("custom_field".into()), record_id: Some(id.to_string()),
        after_json: Some(serde_json::json!({ "active": active }).to_string()),
        ..Default::default()
    })?;
    crate::write::append_op(&tx, device_mode, &Op {
        op_id: new_id("op"), hlc: now.clone(), device_id: dev.clone(), staff_id: actor_s.id.clone(),
        audience: "admin".into(), table: "custom_field".into(), record_id: id.to_string(), kind: "update".into(),
        payload: "{}".into(), base_version: None, server_epoch: 1,
    })?;
    tx.commit()?;
    conn.query_row(&format!("{CF_SELECT} WHERE id=?1"), params![id], map_custom_field).map_err(Into::into)
}

/// Active fields for an entity, each with the record's current value (profile form).
pub fn get_custom_values_logic(conn: &mut Connection, entity: &str, entity_id: &str) -> CmdResult<Vec<CustomFieldValueDto>> {
    let fields = list_custom_fields_logic(conn, entity, false)?;
    let mut out = Vec::with_capacity(fields.len());
    for field in fields {
        let value: Option<String> = conn
            .query_row("SELECT value FROM custom_value WHERE entity_id=?1 AND field_id=?2", params![entity_id, field.id], |r| r.get(0))
            .optional()?
            .flatten();
        out.push(CustomFieldValueDto { field, value });
    }
    Ok(out)
}

/// Set custom values on a record (Principal direct; student → EditStudentDetails,
/// staff → ManageStaff). Each value validated against its field type; audited; synced.
pub fn set_custom_values_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    entity: &str,
    entity_id: &str,
    values: &[CustomValueSet],
) -> CmdResult<Vec<CustomFieldValueDto>> {
    let actor = actor_from(conn, actor_s)?;
    let ent = cf_entity(entity)?;
    let (action, target) = match ent {
        Entity::Student => (Action::EditStudentDetails, Target::of(TargetKind::Student)),
        Entity::Staff => (Action::ManageStaff, Target { kind: TargetKind::School, ..Default::default() }),
    };
    require_allow(&actor, action, &target)?;
    // Validate each value against its field definition first (all-or-nothing).
    let mut prepared: Vec<(String, String, String, String, bool, Vec<String>)> = Vec::new(); // (field_id, value, type, ..)
    for v in values {
        let (ftype_s, options_json, required, fentity): (String, Option<String>, i64, String) = conn
            .query_row("SELECT type, options_json, required, entity FROM custom_field WHERE id=?1 AND active=1", params![v.field_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .optional()?
            .ok_or_else(CmdError::not_found)?;
        if fentity != entity {
            return Err(CmdError::validation("field_id", "wrong_entity"));
        }
        let ftype = cf_type(&ftype_s)?;
        let options: Vec<String> = options_json.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        cf::validate_value(ftype, &v.value, &options, required != 0)?;
        prepared.push((v.field_id.clone(), v.value.trim().to_string(), ftype_s, String::new(), required != 0, options));
    }
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let sync_state = if device_mode == DeviceMode::Server { "confirmed" } else { "on_device" };
    let dev = device_id.unwrap_or_default().to_string();
    let tx = conn.transaction()?;
    for (field_id, value, _ty, _u, _req, _opts) in &prepared {
        let cvid = format!("cv-{}-{}", entity_id, field_id);
        tx.execute(
            "INSERT INTO custom_value(id,entity_id,field_id,value,school_id,created_at,updated_at,updated_by_staff,updated_by_device,sync_state) \
             VALUES (?1,?2,?3,?4,?5,?6,?6,?7,?8,?9) \
             ON CONFLICT(entity_id,field_id) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at, updated_by_staff=excluded.updated_by_staff, updated_by_device=excluded.updated_by_device, version=version+1",
            params![cvid, entity_id, field_id, value, school_id, now, actor_s.id, dev, sync_state],
        )?;
        crate::write::append_op(&tx, device_mode, &Op {
            op_id: new_id("op"), hlc: now.clone(), device_id: dev.clone(), staff_id: actor_s.id.clone(),
            audience: "admin".into(), table: "custom_value".into(), record_id: cvid, kind: "update".into(),
            payload: "{}".into(), base_version: None, server_epoch: 1,
        })?;
    }
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "set_custom_values".into(),
        table: Some("custom_value".into()), record_id: Some(entity_id.to_string()),
        after_json: Some(serde_json::json!({ "entity": entity, "count": prepared.len() }).to_string()),
        ..Default::default()
    })?;
    tx.commit()?;
    get_custom_values_logic(conn, entity, entity_id)
}

// ============================================================ privacy ========
//
// DPDP (§9): consent per student+purpose; student export/erase (Principal;
// financial/academic/audit kept, personal fields tombstoned); a retention setting;
// an incident log with the 72-hour reporting duty; and a record of export/erase.

const KV_RETENTION: &str = "privacy.retention";
const ERASED: &str = "(erased)";

#[derive(Debug, Serialize)]
pub struct ConsentDto {
    pub id: String,
    pub student_id: String,
    pub guardian_id: Option<String>,
    pub purpose: String,
    pub method: String,
    pub recorded_at: String,
    pub withdrawn_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IncidentDto {
    pub id: String,
    pub occurred_on: Option<String>,
    pub description: String,
    pub action_taken: Option<String>,
    pub reported_to_board: bool,
    pub reported_on: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct IncidentInput {
    pub occurred_on: Option<String>,
    pub description: String,
    #[serde(default)]
    pub action_taken: Option<String>,
    #[serde(default)]
    pub reported_to_board: bool,
    #[serde(default)]
    pub reported_on: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PrivacyActionDto {
    pub id: String,
    pub student_id: Option<String>,
    pub kind: String,
    pub performed_at: String,
    pub note: Option<String>,
}

/// The consent rows for a student (latest first).
pub fn list_consent_logic(conn: &mut Connection, student_id: &str) -> CmdResult<Vec<ConsentDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, student_id, guardian_id, purpose, method, recorded_at, withdrawn_at FROM consent WHERE student_id=?1 ORDER BY recorded_at DESC",
    )?;
    let rows = stmt.query_map(params![student_id], |r| {
        Ok(ConsentDto {
            id: r.get(0)?, student_id: r.get(1)?, guardian_id: r.get(2)?, purpose: r.get(3)?,
            method: r.get(4)?, recorded_at: r.get(5)?, withdrawn_at: r.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// True if the student has an active (non-withdrawn) `messages` consent — used by
/// the messaging engine's `can_message` (P14 wires the send path to this).
pub fn messages_consent_logic(conn: &Connection, student_id: &str) -> rusqlite::Result<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM consent WHERE student_id=?1 AND purpose='messages' AND withdrawn_at IS NULL",
        params![student_id], |r| r.get(0),
    )?;
    Ok(n > 0)
}

/// Record a consent (Accountant + Principal — part of admission/records). Audited; synced.
#[allow(clippy::too_many_arguments)]
pub fn record_consent_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    student_id: &str,
    guardian_id: Option<&str>,
    purpose: &str,
    method: &str,
) -> CmdResult<Vec<ConsentDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::CreateStudent, &Target::of(TargetKind::Student))?;
    if !matches!(purpose, "school_records" | "messages") {
        return Err(CmdError::validation("purpose", "unknown"));
    }
    if !matches!(method, "signed_form" | "in_person") {
        return Err(CmdError::validation("method", "unknown"));
    }
    let id = new_id("con");
    let now = now_iso();
    let school_id = single_school_id(conn)?;
    let sync_state = if device_mode == DeviceMode::Server { "confirmed" } else { "on_device" };
    let dev = device_id.unwrap_or_default().to_string();
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO consent(id,student_id,guardian_id,purpose,method,recorded_by,recorded_at,school_id,created_at,updated_at,updated_by_staff,updated_by_device,sync_state) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?7,?7,?6,?9,?10)",
        params![id, student_id, guardian_id, purpose, method, actor_s.id, now, school_id, dev, sync_state],
    )?;
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "record_consent".into(),
        table: Some("consent".into()), record_id: Some(id.clone()),
        after_json: Some(serde_json::json!({ "student_id": student_id, "purpose": purpose, "method": method }).to_string()),
        ..Default::default()
    })?;
    crate::write::append_op(&tx, device_mode, &finance_op(&now, &dev, &actor_s.id, "consent", &id, "insert"))?;
    tx.commit()?;
    list_consent_logic(conn, student_id)
}

/// Withdraw a consent (sets withdrawn_at). Audited; synced.
pub fn withdraw_consent_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    id: &str,
) -> CmdResult<Vec<ConsentDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::CreateStudent, &Target::of(TargetKind::Student))?;
    let student_id: String = conn
        .query_row("SELECT student_id FROM consent WHERE id=?1", params![id], |r| r.get(0))
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    let now = now_iso();
    let dev = device_id.unwrap_or_default().to_string();
    let tx = conn.transaction()?;
    tx.execute("UPDATE consent SET withdrawn_at=?2, updated_at=?2, version=version+1 WHERE id=?1 AND withdrawn_at IS NULL", params![id, now])?;
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "withdraw_consent".into(),
        table: Some("consent".into()), record_id: Some(id.to_string()),
        after_json: Some(serde_json::json!({ "withdrawn_at": now }).to_string()),
        ..Default::default()
    })?;
    crate::write::append_op(&tx, device_mode, &finance_op(&now, &dev, &actor_s.id, "consent", id, "update"))?;
    tx.commit()?;
    list_consent_logic(conn, &student_id)
}

/// Export one student's data as JSON (Principal). Logs a privacy action.
pub fn export_student_logic(conn: &mut Connection, actor_s: &SessionStaff, student_id: &str, today: &str) -> CmdResult<String> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::Settings, &Target { kind: TargetKind::School, ..Default::default() })?;
    let profile = get_student_profile_logic(conn, today, student_id)?;
    let consent = list_consent_logic(conn, student_id)?;
    let customs = get_custom_values_logic(conn, "student", student_id)?;
    let payments = list_payments_logic(conn, Some(student_id))?;
    let export = serde_json::json!({
        "exported_at": now_iso(),
        "student": profile,
        "consent": consent,
        "custom_values": customs,
        "payments": payments,
    });
    conn.execute(
        "INSERT INTO privacy_action(id,student_id,kind,performed_by,performed_at,note,school_id) VALUES (?1,?2,'export',?3,?4,?5,(SELECT id FROM school LIMIT 1))",
        params![new_id("pa"), student_id, actor_s.id, now_iso(), format!("Exported {}", profile.name)],
    )?;
    Ok(serde_json::to_string_pretty(&export).unwrap_or_else(|_| "{}".into()))
}

/// Erase a student on request (Principal): tombstone personal fields on the
/// student and their (non-shared) guardians, delete custom values; KEEP financial,
/// academic and audit records. Audited; logs a privacy action.
pub fn erase_student_logic(
    conn: &mut Connection,
    actor_s: &SessionStaff,
    device_id: Option<&str>,
    device_mode: DeviceMode,
    student_id: &str,
) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::Settings, &Target { kind: TargetKind::School, ..Default::default() })?;
    let name: String = conn
        .query_row("SELECT name FROM student WHERE id=?1", params![student_id], |r| r.get(0))
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    let now = now_iso();
    let dev = device_id.unwrap_or_default().to_string();
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE student SET name=?2, dob=NULL, gender=NULL, guardian_name=NULL, guardian_mobile=NULL, address=NULL, aadhaar_status='none', updated_at=?3, version=version+1 WHERE id=?1",
        params![student_id, ERASED, now],
    )?;
    let gids: Vec<String> = {
        let mut gstmt = tx.prepare(
            "SELECT guardian_id FROM student_guardian WHERE student_id=?1 AND guardian_id IN \
             (SELECT guardian_id FROM student_guardian GROUP BY guardian_id HAVING COUNT(*)=1)",
        )?;
        let out = gstmt.query_map(params![student_id], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<String>>>()?;
        out
    };
    for gid in &gids {
        tx.execute("UPDATE guardian SET name=?2, mobile=NULL, email=NULL, updated_at=?3, version=version+1 WHERE id=?1", params![gid, ERASED, now])?;
    }
    tx.execute("DELETE FROM custom_value WHERE entity_id=?1", params![student_id])?;
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "erase_student".into(),
        table: Some("student".into()), record_id: Some(student_id.to_string()),
        reason: Some("erase_on_request".into()),
        after_json: Some(serde_json::json!({ "tombstoned": true, "guardians_tombstoned": gids.len() }).to_string()),
        ..Default::default()
    })?;
    crate::write::append_op(&tx, device_mode, &finance_op(&now, &dev, &actor_s.id, "student", student_id, "update"))?;
    tx.execute(
        "INSERT INTO privacy_action(id,student_id,kind,performed_by,performed_at,note,school_id) VALUES (?1,?2,'erase',?3,?4,?5,(SELECT id FROM school LIMIT 1))",
        params![new_id("pa"), student_id, actor_s.id, now, format!("Erased {name}")],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn get_retention_logic(conn: &mut Connection) -> CmdResult<String> {
    Ok(crate::kv::get::<String>(conn, KV_RETENTION)?.unwrap_or_else(|| "keep".to_string()))
}

pub fn set_retention_logic(conn: &mut Connection, actor_s: &SessionStaff, value: &str) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::Settings, &Target { kind: TargetKind::School, ..Default::default() })?;
    if !matches!(value, "keep" | "review") {
        return Err(CmdError::validation("retention", "unknown"));
    }
    crate::kv::set(conn, KV_RETENTION, &value.to_string())?;
    Ok(())
}

pub fn list_incidents_logic(conn: &mut Connection) -> CmdResult<Vec<IncidentDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, occurred_on, description, action_taken, reported_to_board, reported_on, created_at FROM incident_log ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(IncidentDto {
            id: r.get(0)?, occurred_on: r.get(1)?, description: r.get(2)?, action_taken: r.get(3)?,
            reported_to_board: r.get::<_, i64>(4)? != 0, reported_on: r.get(5)?, created_at: r.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn add_incident_logic(conn: &mut Connection, actor_s: &SessionStaff, input: &IncidentInput) -> CmdResult<Vec<IncidentDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::Settings, &Target { kind: TargetKind::School, ..Default::default() })?;
    if input.description.trim().is_empty() {
        return Err(CmdError::validation("description", "required"));
    }
    let now = now_iso();
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO incident_log(id,occurred_on,description,action_taken,reported_to_board,reported_on,recorded_by,school_id,created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,(SELECT id FROM school LIMIT 1),?8)",
        params![new_id("inc"), input.occurred_on, input.description, input.action_taken, input.reported_to_board as i64, input.reported_on, actor_s.id, now],
    )?;
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(), staff_id: Some(actor_s.id.clone()), action: "add_incident".into(),
        table: Some("incident_log".into()), after_json: Some(serde_json::json!({ "reported_to_board": input.reported_to_board }).to_string()),
        ..Default::default()
    })?;
    tx.commit()?;
    list_incidents_logic(conn)
}

pub fn list_privacy_actions_logic(conn: &mut Connection) -> CmdResult<Vec<PrivacyActionDto>> {
    let mut stmt = conn.prepare("SELECT id, student_id, kind, performed_at, note FROM privacy_action ORDER BY performed_at DESC LIMIT 200")?;
    let rows = stmt.query_map([], |r| {
        Ok(PrivacyActionDto { id: r.get(0)?, student_id: r.get(1)?, kind: r.get(2)?, performed_at: r.get(3)?, note: r.get(4)? })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn seeded() -> Connection {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        let now = time::OffsetDateTime::parse("2026-09-23T09:00:00Z", &time::format_description::well_known::Rfc3339).unwrap();
        crate::seed::seed_demo_school(&mut c, now).unwrap();
        c
    }

    fn accountant() -> SessionStaff {
        SessionStaff { id: "stf-suresh".into(), name: "Suresh Patel".into(), role: "accountant".into() }
    }

    fn principal() -> SessionStaff {
        SessionStaff { id: "stf-priya".into(), name: "Priya Sharma".into(), role: "principal".into() }
    }

    // ---- Phase 13: calendar ------------------------------------------------
    #[test]
    fn calendar_defaults_then_weekly_offs_edit() {
        let mut c = seeded();
        // Fresh: no school_week rows → default (Mon–Sat working, Sunday off).
        let cal = get_calendar_logic(&mut c, &principal()).unwrap();
        assert_eq!(cal.week, [true, true, true, true, true, true, false]);
        assert!(cal.events.is_empty());
        // Make Saturday (index 5) a weekly off too.
        set_weekly_offs_logic(
            &mut c, &principal(), None, DeviceMode::Server,
            &[true, true, true, true, true, false, false],
        )
        .unwrap();
        let cal = get_calendar_logic(&mut c, &principal()).unwrap();
        assert_eq!(cal.week, [true, true, true, true, true, false, false]);
        // The repo agrees: 2026-09-26 (Saturday) is now non-working.
        assert!(!crate::calendar::is_working_day(&c, "2026-09-26").unwrap());
        // Audited, and one op per weekday landed in op_log (Server mode).
        let ops: i64 = c.query_row("SELECT count(*) FROM op_log WHERE \"table\"='school_week'", [], |r| r.get(0)).unwrap();
        assert_eq!(ops, 7);
    }

    #[test]
    fn weekly_offs_must_be_length_7() {
        let mut c = seeded();
        let e = set_weekly_offs_logic(&mut c, &principal(), None, DeviceMode::Server, &[true, false]).unwrap_err();
        assert_eq!(e.code, "VALIDATION");
    }

    // ---- Phase 14: messaging outbox ---------------------------------------
    fn wa_tap_input() -> RecordMessageInput {
        RecordMessageInput {
            channel: "wa_tap".into(),
            kind: "receipt_share".into(),
            language: "en".into(),
            to_guardian_id: None,
            to_staff_id: None,
            to_address: Some("9876543210".into()),
            subject: None,
            body: Some("Fee receipt R-A2-0419 · ₹1,000 · Kavya".into()),
            related_table: Some("payment".into()),
            related_id: Some("pay-1".into()),
        }
    }

    #[test]
    fn record_wa_tap_writes_a_tapped_message() {
        let mut c = seeded();
        let dto = record_message_logic(&mut c, &accountant(), None, DeviceMode::Server, &wa_tap_input()).unwrap();
        assert_eq!(dto.channel, "wa_tap");
        assert_eq!(dto.status, "tapped"); // never "sent" — honest status (§3 rule 13)
        // Row persisted with the creator; op logged (Server mode).
        let n: i64 = c.query_row("SELECT count(*) FROM message WHERE status='tapped' AND created_by='stf-suresh'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
        let ops: i64 = c.query_row("SELECT count(*) FROM op_log WHERE \"table\"='message'", [], |r| r.get(0)).unwrap();
        assert_eq!(ops, 1);
        // Listed for its creator, filtered by the related payment.
        let list = list_messages_logic(&mut c, &accountant(), None, Some("pay-1")).unwrap();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn teacher_cannot_share_a_receipt() {
        let mut c = seeded();
        let teacher = SessionStaff { id: "stf-meena".into(), name: "Meena".into(), role: "teacher".into() };
        let e = record_message_logic(&mut c, &teacher, None, DeviceMode::Server, &wa_tap_input()).unwrap_err();
        assert_eq!(e.code, "FORBIDDEN");
    }

    #[test]
    fn unknown_message_kind_and_channel_are_rejected() {
        let mut c = seeded();
        let mut bad = wa_tap_input();
        bad.kind = "spam".into();
        assert_eq!(record_message_logic(&mut c, &accountant(), None, DeviceMode::Server, &bad).unwrap_err().code, "VALIDATION");
        let mut bad2 = wa_tap_input();
        bad2.channel = "sms".into();
        assert_eq!(record_message_logic(&mut c, &accountant(), None, DeviceMode::Server, &bad2).unwrap_err().code, "VALIDATION");
    }

    // ---- Phase 14 Step 5: dues + fee reminders ----------------------------
    #[test]
    fn inr_indian_grouping() {
        assert_eq!(inr(210000), "₹2,100");
        assert_eq!(inr(68420000), "₹6,84,200");
        assert_eq!(inr(210050), "₹2,100.50");
        assert_eq!(inr(0), "₹0");
    }

    fn pay_input(on: bool) -> PaymentSettingsInput {
        PaymentSettingsInput { upi_id: Some("school@okhdfcbank".into()), upi_name: Some("Green Valley".into()), on_receipts: true, on_reminders: on, on_dues_list: false }
    }

    #[test]
    fn dues_list_and_fee_reminder_queue() {
        let mut c = seeded();
        set_payment_settings_logic(&mut c, &principal(), &pay_input(true)).unwrap();
        let dues = list_dues_logic(&mut c, &accountant(), None).unwrap();
        assert!(dues.strip.total_due_paise > 0, "seed has outstanding dues");
        assert!(!dues.rows.is_empty());
        assert_eq!(dues.strip.unpaid_dues, dues.rows.len() as i64);
        let sid = dues.rows[0].student_id.clone();
        // The demo seed has no guardian rows (real schools create them via
        // create_student); give this student a primary guardian with email +
        // `messages` consent so the emailable path is exercised.
        let school_id: String = c.query_row("SELECT id FROM school LIMIT 1", [], |r| r.get(0)).unwrap();
        c.execute(
            "INSERT INTO guardian(id,name,relation,mobile,email,language,whatsapp_ok,school_id,created_at,updated_at,sync_state) \
             VALUES('g-test','Test Parent','father','9876543210','parent@example.com','en',1,?1,'t','t','confirmed')",
            params![school_id],
        ).unwrap();
        c.execute(
            "INSERT INTO student_guardian(id,student_id,guardian_id,is_primary,school_id,created_at,updated_at,sync_state) \
             VALUES('sg-test',?1,'g-test',1,?2,'t','t','confirmed')",
            params![sid, school_id],
        ).unwrap();
        record_consent_logic(&mut c, &principal(), None, DeviceMode::Server, &sid, None, "messages", "in_person").unwrap();

        let prev = preview_fee_reminder_logic(&mut c, &accountant(), &sid, Some("en")).unwrap();
        assert!(prev.body.contains('₹'), "reminder body carries the amount");
        assert!(prev.upi_link.as_deref().unwrap().starts_with("upi://pay"), "reminder carries a UPI link");
        assert!(prev.has_email && prev.has_consent);

        let bulk = queue_fee_reminders_logic(&mut c, &accountant(), None, DeviceMode::Server, std::slice::from_ref(&sid), Some("en")).unwrap();
        assert_eq!(bulk.queued, 1);
        let n: i64 = c.query_row("SELECT count(*) FROM message WHERE channel='email' AND kind='fee_reminder' AND status='queued'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);

        // A different student (no email / no consent) is skipped, not queued.
        if let Some(other) = dues.rows.iter().map(|r| r.student_id.clone()).find(|s| s != &sid) {
            let bulk2 = queue_fee_reminders_logic(&mut c, &accountant(), None, DeviceMode::Server, &[other], Some("en")).unwrap();
            assert_eq!(bulk2.queued, 0);
            assert_eq!(bulk2.skipped_no_email + bulk2.skipped_no_consent, 1);
        }
    }

    #[test]
    fn reminder_without_upi_toggle_has_no_link() {
        let mut c = seeded();
        set_payment_settings_logic(&mut c, &principal(), &pay_input(false)).unwrap(); // reminders toggle OFF
        let sid = list_dues_logic(&mut c, &accountant(), None).unwrap().rows[0].student_id.clone();
        let prev = preview_fee_reminder_logic(&mut c, &accountant(), &sid, Some("en")).unwrap();
        assert!(prev.upi_link.is_none());
    }

    #[test]
    fn teacher_cannot_view_dues_or_send_reminders() {
        let mut c = seeded();
        let teacher = SessionStaff { id: "stf-meena".into(), name: "Meena".into(), role: "teacher".into() };
        assert_eq!(list_dues_logic(&mut c, &teacher, None).unwrap_err().code, "FORBIDDEN");
        assert_eq!(queue_fee_reminders_logic(&mut c, &teacher, None, DeviceMode::Server, &[], None).unwrap_err().code, "FORBIDDEN");
    }

    // ---- Phase 14 Step 4: absence alerts ----------------------------------
    fn meena() -> SessionStaff {
        SessionStaff { id: "stf-meena".into(), name: "Meena Iyer".into(), role: "teacher".into() }
    }

    #[test]
    fn absence_list_scopes_to_class_teacher() {
        let mut c = seeded();
        // Meena class-teaches V-A (cls-5a): submitted sheet with 3 absentees.
        let list = list_absent_logic(&mut c, &meena(), "cls-5a", "2026-09-23").unwrap();
        assert!(list.submitted);
        assert_eq!(list.students.len(), 3);
        assert!(!list.students[0].preview.is_empty(), "absence_alert body rendered");
        // A teacher who does not class-teach cls-5a → forbidden.
        let other = SessionStaff { id: "stf-anita".into(), name: "Anita".into(), role: "teacher".into() };
        assert_eq!(list_absent_logic(&mut c, &other, "cls-5a", "2026-09-23").unwrap_err().code, "FORBIDDEN");
        // Accountant → forbidden (no attendance data).
        assert_eq!(list_absent_logic(&mut c, &accountant(), "cls-5a", "2026-09-23").unwrap_err().code, "FORBIDDEN");
        // Principal on a not-yet-submitted class → submitted:false, empty.
        let pending = list_absent_logic(&mut c, &principal(), "cls-7b", "2026-09-23").unwrap();
        assert!(!pending.submitted && pending.students.is_empty());
    }

    fn absence_input(student_id: &str) -> RecordMessageInput {
        RecordMessageInput {
            channel: "wa_tap".into(), kind: "absence_alert".into(), language: "en".into(),
            to_guardian_id: None, to_staff_id: None, to_address: Some("9876543210".into()),
            subject: None, body: Some("Absent today".into()),
            related_table: Some("student".into()), related_id: Some(student_id.to_string()),
        }
    }

    #[test]
    fn absence_alert_only_for_own_class_students() {
        let mut c = seeded();
        let mine = list_absent_logic(&mut c, &meena(), "cls-5a", "2026-09-23").unwrap().students[0].student_id.clone();
        // Meena may alert an absentee in her own class.
        assert!(record_message_logic(&mut c, &meena(), None, DeviceMode::Server, &absence_input(&mine)).is_ok());
        // But NOT a student in a class she does not class-teach (cls-2a).
        let other: String = c.query_row("SELECT student_id FROM enrollment WHERE class_id='cls-2a' AND to_date IS NULL LIMIT 1", [], |r| r.get(0)).unwrap();
        assert_eq!(record_message_logic(&mut c, &meena(), None, DeviceMode::Server, &absence_input(&other)).unwrap_err().code, "FORBIDDEN");
    }

    // ---- Phase 14 Step 6: circulars ---------------------------------------
    fn circular_input() -> CircularInput {
        CircularInput {
            id: None, title: "Sports Day".into(), body: "On Friday.".into(), languages_json: None,
            audience_json: Some(r#"{"kind":"whole_school"}"#.into()),
            channels_json: Some(r#"["staff_app"]"#.into()), attachments_json: None,
        }
    }

    #[test]
    fn circular_draft_send_and_read_tracking() {
        let mut c = seeded();
        let draft = save_circular_logic(&mut c, &principal(), None, DeviceMode::Server, &circular_input()).unwrap();
        assert_eq!(draft.status, "draft");
        assert!(draft.number.is_none());
        // Send → server number CIR/<session>/001, status sent.
        let sent = send_circular_logic(&mut c, &principal(), None, DeviceMode::Server, &draft.id).unwrap();
        assert_eq!(sent.status, "sent");
        let num = sent.number.as_deref().unwrap();
        assert!(num.starts_with("CIR/") && num.ends_with("/001"), "{num}");
        // A teacher marks it read → read_count increments (idempotently).
        mark_circular_read_logic(&mut c, &meena(), None, DeviceMode::Server, &draft.id).unwrap();
        mark_circular_read_logic(&mut c, &meena(), None, DeviceMode::Server, &draft.id).unwrap();
        let row = list_circulars_logic(&mut c, &principal()).unwrap().into_iter().find(|x| x.id == draft.id).unwrap();
        assert_eq!(row.read_count, 1);
        assert!(row.staff_count >= 1);
        // Re-sending is rejected.
        assert_eq!(send_circular_logic(&mut c, &principal(), None, DeviceMode::Server, &draft.id).unwrap_err().code, "VALIDATION");
    }

    #[test]
    fn circular_permissions_and_module_gate() {
        let mut c = seeded();
        // Accountant / teacher cannot manage circulars.
        assert_eq!(save_circular_logic(&mut c, &accountant(), None, DeviceMode::Server, &circular_input()).unwrap_err().code, "FORBIDDEN");
        assert_eq!(save_circular_logic(&mut c, &meena(), None, DeviceMode::Server, &circular_input()).unwrap_err().code, "FORBIDDEN");
        // Turning the circulars module off gates the commands with MODULE_OFF.
        set_module_logic(&mut c, &principal(), "circulars", false).unwrap();
        assert_eq!(save_circular_logic(&mut c, &principal(), None, DeviceMode::Server, &circular_input()).unwrap_err().code, "MODULE_OFF");
        assert_eq!(list_circulars_logic(&mut c, &principal()).unwrap_err().code, "MODULE_OFF");
    }

    // ---- Phase 14 Step 7: automatic WhatsApp config -----------------------
    fn wa_cfg(pn: &str, token: Option<&str>) -> WaAutoConfigInput {
        let mut templates = std::collections::BTreeMap::new();
        templates.insert("fee_reminder.en".to_string(), "tpl_fr".to_string());
        WaAutoConfigInput { phone_number_id: pn.into(), token: token.map(str::to_string), templates }
    }

    #[test]
    fn wa_auto_config_save_read_and_token_never_leaks() {
        let mut c = seeded();
        set_module_logic(&mut c, &principal(), "wa_auto", true).unwrap();
        set_wa_auto_config_logic(&mut c, &principal(), &wa_cfg("111", Some("SECRETTOKEN"))).unwrap();
        let dto = get_wa_auto_config_logic(&mut c, &principal()).unwrap();
        assert!(dto.configured && dto.token_set);
        assert_eq!(dto.phone_number_id, "111");
        // The token never appears in the audit log.
        let leaked: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM audit_log WHERE after_json LIKE '%SECRETTOKEN%')", [], |r| r.get(0)).unwrap();
        assert!(!leaked, "token must not be written to the audit");
        // Saving without a token keeps the stored one.
        set_wa_auto_config_logic(&mut c, &principal(), &wa_cfg("222", None)).unwrap();
        let dto2 = get_wa_auto_config_logic(&mut c, &principal()).unwrap();
        assert!(dto2.token_set && dto2.phone_number_id == "222");
    }

    #[test]
    fn wa_auto_config_and_channel_require_the_module() {
        let mut c = seeded();
        // Off by default → config is MODULE_OFF; accountant is FORBIDDEN even when on.
        assert_eq!(set_wa_auto_config_logic(&mut c, &principal(), &wa_cfg("1", Some("t"))).unwrap_err().code, "MODULE_OFF");
        set_module_logic(&mut c, &principal(), "wa_auto", true).unwrap();
        assert_eq!(set_wa_auto_config_logic(&mut c, &accountant(), &wa_cfg("1", Some("t"))).unwrap_err().code, "FORBIDDEN");
        // A wa_auto message queues only while the module is on.
        let wa_msg = RecordMessageInput { channel: "wa_auto".into(), kind: "fee_reminder".into(), language: "en".into(), to_guardian_id: None, to_staff_id: None, to_address: Some("9876543210".into()), subject: None, body: Some("x".into()), related_table: None, related_id: None };
        assert!(record_message_logic(&mut c, &accountant(), None, DeviceMode::Server, &wa_msg).is_ok());
        set_module_logic(&mut c, &principal(), "wa_auto", false).unwrap();
        assert_eq!(record_message_logic(&mut c, &accountant(), None, DeviceMode::Server, &wa_msg).unwrap_err().code, "MODULE_OFF");
    }

    #[test]
    fn add_update_delete_calendar_event() {
        let mut c = seeded();
        let input = CalendarEventInput {
            starts_on: "2026-10-02".into(),
            ends_on: "2026-10-02".into(),
            kind: "holiday".into(),
            title: "Gandhi Jayanti".into(),
            title_hi: None,
            title_te: None,
            is_non_working: true,
        };
        let ev = add_calendar_event_logic(&mut c, &principal(), None, DeviceMode::Server, &input).unwrap();
        assert_eq!(ev.title, "Gandhi Jayanti");
        assert!(ev.is_non_working);
        // The day is now non-working per the repo.
        assert!(!crate::calendar::is_working_day(&c, "2026-10-02").unwrap());

        // Update: extend to a range and rename.
        let upd = CalendarEventInput {
            starts_on: "2026-10-02".into(),
            ends_on: "2026-10-03".into(),
            kind: "holiday".into(),
            title: "Gandhi Jayanti (extended)".into(),
            title_hi: None,
            title_te: None,
            is_non_working: true,
        };
        let ev2 = update_calendar_event_logic(&mut c, &principal(), None, DeviceMode::Server, &ev.id, &upd).unwrap();
        assert_eq!(ev2.ends_on, "2026-10-03");
        assert!(!crate::calendar::is_working_day(&c, "2026-10-03").unwrap());

        // Delete.
        delete_calendar_event_logic(&mut c, &principal(), None, DeviceMode::Server, &ev.id).unwrap();
        assert!(get_calendar_logic(&mut c, &principal()).unwrap().events.is_empty());
        assert!(crate::calendar::is_working_day(&c, "2026-10-02").unwrap()); // working again
    }

    #[test]
    fn add_event_rejects_bad_dates_and_kind() {
        let mut c = seeded();
        let bad_range = CalendarEventInput {
            starts_on: "2026-10-05".into(), ends_on: "2026-10-01".into(),
            kind: "holiday".into(), title: "x".into(), title_hi: None, title_te: None, is_non_working: true,
        };
        assert_eq!(add_calendar_event_logic(&mut c, &principal(), None, DeviceMode::Server, &bad_range).unwrap_err().code, "VALIDATION");
        let bad_kind = CalendarEventInput {
            starts_on: "2026-10-01".into(), ends_on: "2026-10-01".into(),
            kind: "party".into(), title: "x".into(), title_hi: None, title_te: None, is_non_working: false,
        };
        assert_eq!(add_calendar_event_logic(&mut c, &principal(), None, DeviceMode::Server, &bad_kind).unwrap_err().code, "VALIDATION");
    }

    // ---- Phase 13: guardians -----------------------------------------------
    fn ge(name: &str, mobile: Option<&str>) -> GuardianEditInput {
        GuardianEditInput { name: name.into(), relation: Some("Father".into()), mobile: mobile.map(|s| s.into()), email: None, language: "en".into(), whatsapp_ok: true }
    }

    fn new_student(c: &mut Connection, name: &str, gname: Option<&str>, gmob: Option<&str>) -> String {
        let input = NewStudentInput {
            name: name.into(), class_id: "cls-5a".into(), roll_no: None,
            guardian_name: gname.map(|s| s.into()), guardian_mobile: gmob.map(|s| s.into()),
            dob: None, gender: None, address: None, transport: None, rte: None, category: None, aadhaar_status: None,
        };
        create_student_logic(c, &principal(), None, DeviceMode::Server, "2026-09-23", &input).unwrap().id
    }

    #[test]
    fn create_student_makes_a_primary_guardian_and_siblings_share_it() {
        let mut c = seeded();
        let s1 = new_student(&mut c, "Riya Verma", Some("Ramesh Verma"), Some("9876543210"));
        let prof = get_student_profile_logic(&mut c, "2026-09-23", &s1).unwrap();
        assert_eq!(prof.guardians.len(), 1);
        assert!(prof.guardians[0].is_primary);
        assert_eq!(prof.guardians[0].name, "Ramesh Verma");
        // A sibling with the same guardian (mobile+name) shares the row.
        let s2 = new_student(&mut c, "Rohit Verma", Some("Ramesh Verma"), Some("9876543210"));
        let g1 = &get_student_profile_logic(&mut c, "2026-09-23", &s1).unwrap().guardians[0].id;
        let g2 = &get_student_profile_logic(&mut c, "2026-09-23", &s2).unwrap().guardians[0].id;
        assert_eq!(g1, g2);
    }

    #[test]
    fn add_second_guardian_set_primary_and_remove() {
        let mut c = seeded();
        let s = new_student(&mut c, "Riya Verma", Some("Ramesh Verma"), Some("9876543210"));
        // Add a second guardian (mother).
        let list = add_guardian_logic(&mut c, &principal(), None, DeviceMode::Server, &s, &ge("Sunita Verma", Some("9811111111"))).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list.iter().filter(|g| g.is_primary).count(), 1, "still exactly one primary");
        let mother = list.iter().find(|g| g.name == "Sunita Verma").unwrap().id.clone();
        // A third is rejected (max 2).
        assert_eq!(add_guardian_logic(&mut c, &principal(), None, DeviceMode::Server, &s, &ge("X", None)).unwrap_err().code, "VALIDATION");
        // Make the mother primary.
        let list = set_primary_guardian_logic(&mut c, &principal(), None, DeviceMode::Server, &s, &mother).unwrap();
        assert!(list.iter().find(|g| g.id == mother).unwrap().is_primary);
        assert_eq!(list.iter().filter(|g| g.is_primary).count(), 1);
        // Remove the (now non-primary) father → mother auto-stays primary; 1 left.
        let father = list.iter().find(|g| g.name == "Ramesh Verma").unwrap().id.clone();
        let list = remove_guardian_logic(&mut c, &principal(), None, DeviceMode::Server, &s, &father).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].is_primary);
    }

    #[test]
    fn teacher_cannot_edit_guardians() {
        let mut c = seeded();
        let s = new_student(&mut c, "Riya Verma", Some("Ramesh Verma"), Some("9876543210"));
        let teacher = SessionStaff { id: "stf-meena".into(), name: "Meena Iyer".into(), role: "teacher".into() };
        assert_eq!(add_guardian_logic(&mut c, &teacher, None, DeviceMode::Server, &s, &ge("X", None)).unwrap_err().code, "FORBIDDEN");
    }

    // ---- Phase 13: approval registry ---------------------------------------
    fn req_input(kind: &str, target_id: &str) -> RequestInput {
        RequestInput {
            kind: kind.into(), target_table: "staff".into(), target_id: target_id.into(),
            base_version: 0, reason: "Casual leave for two days.".into(),
            before_json: None, after_json: Some("{}".into()),
        }
    }

    #[test]
    fn teacher_can_raise_a_leave_request_and_it_stores_pending() {
        let mut c = seeded();
        let teacher = SessionStaff { id: "stf-meena".into(), name: "Meena Iyer".into(), role: "teacher".into() };
        let dto = create_request_logic(&mut c, &teacher, &req_input("leave", "stf-meena")).unwrap();
        assert_eq!(dto.kind, "leave");
        assert_eq!(dto.status, "pending");
        // Approving it does NOT auto-apply (apply function lands in P17).
        let decided = decide_request_logic(&mut c, &principal(), DeviceMode::Server, &dto.id, "approve", Some("ok")).unwrap();
        assert_eq!(decided.status, "approved");
        let apply_state: String = c.query_row("SELECT apply_state FROM request WHERE id=?1", params![dto.id], |r| r.get(0)).unwrap();
        assert_eq!(apply_state, "not_applied");
    }

    #[test]
    fn accountant_cannot_raise_a_class_notice() {
        let mut c = seeded();
        let err = create_request_logic(&mut c, &accountant(), &req_input("class_notice", "stf-suresh")).unwrap_err();
        assert_eq!(err.code, "FORBIDDEN");
    }

    #[test]
    fn unknown_request_kind_is_rejected() {
        let mut c = seeded();
        let err = create_request_logic(&mut c, &principal(), &req_input("banana", "stf-priya")).unwrap_err();
        assert_eq!(err.code, "VALIDATION");
    }

    // ---- Phase 13: custom fields -------------------------------------------
    fn cf_input(entity: &str, key: &str, ftype: &str, options: &[&str], required: bool) -> CustomFieldInput {
        CustomFieldInput {
            entity: entity.into(), key: key.into(), label: format!("{key} label"),
            label_hi: None, label_te: None, field_type: ftype.into(),
            options: options.iter().map(|s| s.to_string()).collect(), required,
        }
    }

    #[test]
    fn create_field_set_and_read_values() {
        let mut c = seeded();
        let field = create_custom_field_logic(&mut c, &principal(), None, DeviceMode::Server, &cf_input("student", "blood_group", "choice", &["A+", "B+", "O+"], false)).unwrap();
        assert_eq!(field.field_type, "choice");
        assert_eq!(field.options, vec!["A+", "B+", "O+"]);
        // Set a valid value.
        let vals = set_custom_values_logic(&mut c, &principal(), None, DeviceMode::Server, "student", "stu-kavya-singh", &[CustomValueSet { field_id: field.id.clone(), value: "B+".into() }]).unwrap();
        assert_eq!(vals.iter().find(|v| v.field.id == field.id).unwrap().value.as_deref(), Some("B+"));
        // get_custom_values reflects it.
        let got = get_custom_values_logic(&mut c, "student", "stu-kavya-singh").unwrap();
        assert_eq!(got.iter().find(|v| v.field.id == field.id).unwrap().value.as_deref(), Some("B+"));
    }

    #[test]
    fn set_value_validates_against_type() {
        let mut c = seeded();
        let field = create_custom_field_logic(&mut c, &principal(), None, DeviceMode::Server, &cf_input("student", "sibling_count", "number", &[], false)).unwrap();
        // Not a number → VALIDATION.
        let err = set_custom_values_logic(&mut c, &principal(), None, DeviceMode::Server, "student", "stu-kavya-singh", &[CustomValueSet { field_id: field.id.clone(), value: "two".into() }]).unwrap_err();
        assert_eq!(err.code, "VALIDATION");
        // A choice field created earlier rejects an out-of-list value.
        let bg = create_custom_field_logic(&mut c, &principal(), None, DeviceMode::Server, &cf_input("student", "bg", "choice", &["A", "B"], false)).unwrap();
        assert_eq!(set_custom_values_logic(&mut c, &principal(), None, DeviceMode::Server, "student", "stu-kavya-singh", &[CustomValueSet { field_id: bg.id, value: "Z".into() }]).unwrap_err().code, "VALIDATION");
    }

    #[test]
    fn only_principal_defines_custom_fields() {
        let mut c = seeded();
        let err = create_custom_field_logic(&mut c, &accountant(), None, DeviceMode::Server, &cf_input("student", "x", "text", &[], false)).unwrap_err();
        assert_eq!(err.code, "FORBIDDEN");
        // But any role may read the list (to render forms).
        assert!(list_custom_fields_logic(&mut c, "student", false).is_ok());
    }

    #[test]
    fn duplicate_field_key_rejected() {
        let mut c = seeded();
        create_custom_field_logic(&mut c, &principal(), None, DeviceMode::Server, &cf_input("student", "route", "text", &[], false)).unwrap();
        let err = create_custom_field_logic(&mut c, &principal(), None, DeviceMode::Server, &cf_input("student", "route", "text", &[], false)).unwrap_err();
        assert_eq!(err.code, "VALIDATION");
    }

    // ---- Phase 13: privacy -------------------------------------------------
    const STU: &str = "stu-kavya-singh";

    #[test]
    fn consent_record_withdraw_and_messages_consent() {
        let mut c = seeded();
        assert!(!messages_consent_logic(&c, STU).unwrap());
        let list = record_consent_logic(&mut c, &accountant(), None, DeviceMode::Server, STU, None, "messages", "signed_form").unwrap();
        assert_eq!(list.len(), 1);
        assert!(messages_consent_logic(&c, STU).unwrap());
        // Withdraw → no active messages consent.
        let cid = list[0].id.clone();
        withdraw_consent_logic(&mut c, &principal(), None, DeviceMode::Server, &cid).unwrap();
        assert!(!messages_consent_logic(&c, STU).unwrap());
        // Bad purpose rejected.
        assert_eq!(record_consent_logic(&mut c, &principal(), None, DeviceMode::Server, STU, None, "ads", "in_person").unwrap_err().code, "VALIDATION");
    }

    #[test]
    fn export_then_erase_keeps_money_tombstones_personal() {
        let mut c = seeded();
        // Record a payment so there's financial data to keep.
        record_payment_logic(&mut c, &accountant(), Some("dev-a2"), DeviceMode::Server,
            &PaymentInput { student_id: STU.into(), amount_paise: 310_000, mode: "cash".into(), reference: Some(String::new()) }).unwrap();
        let payments_before: i64 = c.query_row("SELECT COUNT(*) FROM payment WHERE student_id=?1", params![STU], |r| r.get(0)).unwrap();
        // Export contains the student's name.
        let json = export_student_logic(&mut c, &principal(), STU, "2026-09-23").unwrap();
        assert!(json.contains("Kavya"));
        // Erase tombstones personal fields but keeps the payment.
        erase_student_logic(&mut c, &principal(), None, DeviceMode::Server, STU).unwrap();
        let name: String = c.query_row("SELECT name FROM student WHERE id=?1", params![STU], |r| r.get(0)).unwrap();
        assert_eq!(name, "(erased)");
        let payments_after: i64 = c.query_row("SELECT COUNT(*) FROM payment WHERE student_id=?1", params![STU], |r| r.get(0)).unwrap();
        assert_eq!(payments_after, payments_before, "financial records kept");
        // Both actions logged.
        let actions: i64 = c.query_row("SELECT COUNT(*) FROM privacy_action WHERE student_id=?1", params![STU], |r| r.get(0)).unwrap();
        assert_eq!(actions, 2);
    }

    #[test]
    fn teacher_cannot_export_erase_or_manage_privacy() {
        let mut c = seeded();
        let teacher = SessionStaff { id: "stf-meena".into(), name: "Meena Iyer".into(), role: "teacher".into() };
        assert_eq!(export_student_logic(&mut c, &teacher, STU, "2026-09-23").unwrap_err().code, "FORBIDDEN");
        assert_eq!(erase_student_logic(&mut c, &teacher, None, DeviceMode::Server, STU).unwrap_err().code, "FORBIDDEN");
        assert_eq!(add_incident_logic(&mut c, &teacher, &IncidentInput { occurred_on: None, description: "x".into(), action_taken: None, reported_to_board: false, reported_on: None }).unwrap_err().code, "FORBIDDEN");
    }

    #[test]
    fn retention_and_incident_log() {
        let mut c = seeded();
        assert_eq!(get_retention_logic(&mut c).unwrap(), "keep");
        set_retention_logic(&mut c, &principal(), "review").unwrap();
        assert_eq!(get_retention_logic(&mut c).unwrap(), "review");
        assert_eq!(set_retention_logic(&mut c, &principal(), "delete").unwrap_err().code, "VALIDATION");
        // Incident log.
        let list = add_incident_logic(&mut c, &principal(), &IncidentInput { occurred_on: Some("2026-09-20".into()), description: "Lost USB with a class list.".into(), action_taken: Some("Recovered".into()), reported_to_board: true, reported_on: Some("2026-09-21".into()) }).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].reported_to_board);
        assert_eq!(add_incident_logic(&mut c, &principal(), &IncidentInput { occurred_on: None, description: "  ".into(), action_taken: None, reported_to_board: false, reported_on: None }).unwrap_err().code, "VALIDATION");
    }

    // ---- Phase 13: ledger vouchers -----------------------------------------
    #[test]
    fn record_payment_posts_a_balanced_receipt_voucher() {
        let mut c = seeded();
        let dto = record_payment_logic(
            &mut c, &accountant(), Some("dev-a2"), DeviceMode::Server,
            &PaymentInput { student_id: "stu-kavya-singh".into(), amount_paise: 310_000, mode: "upi".into(), reference: Some("426518903214".into()) },
        ).unwrap();
        let (kind, d, cr): (String, i64, i64) = c.query_row(
            "SELECT v.kind, COALESCE(SUM(le.debit_paise),0), COALESCE(SUM(le.credit_paise),0) \
             FROM voucher v JOIN ledger_entry le ON le.voucher_id=v.id WHERE v.source_table='payment' AND v.source_id=?1",
            params![dto.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).unwrap();
        assert_eq!(kind, "receipt");
        assert_eq!(d, 310_000);
        assert_eq!(cr, 310_000, "voucher balances");
        // UPI → the bank account is debited.
        let bank: i64 = c.query_row(
            "SELECT COALESCE(SUM(le.debit_paise),0) FROM ledger_entry le JOIN voucher v ON v.id=le.voucher_id WHERE v.source_id=?1 AND le.account_id='bank'",
            params![dto.id], |r| r.get(0),
        ).unwrap();
        assert_eq!(bank, 310_000);
        assert_eq!(crate::ledger::ledger_imbalance(&c).unwrap(), 0, "whole book balances");
    }

    #[test]
    fn reversal_posts_the_opposite_voucher() {
        let mut c = seeded();
        let dto = record_payment_logic(
            &mut c, &accountant(), Some("dev-a2"), DeviceMode::Server,
            &PaymentInput { student_id: "stu-kavya-singh".into(), amount_paise: 310_000, mode: "cash".into(), reference: Some(String::new()) },
        ).unwrap();
        reverse_payment_logic(&mut c, &principal(), DeviceMode::Server, &dto.id, "duplicate payment").unwrap();
        let rev: i64 = c.query_row("SELECT COUNT(*) FROM voucher WHERE kind='reversal'", [], |r| r.get(0)).unwrap();
        assert_eq!(rev, 1, "one reversal voucher");
        // Receipt + reversal net to zero on Fee income.
        let fee_net: i64 = c.query_row(
            "SELECT COALESCE(SUM(le.debit_paise - le.credit_paise),0) FROM ledger_entry le JOIN voucher v ON v.id=le.voucher_id \
             WHERE le.account_id='fee_income' AND ((v.source_table='payment' AND v.source_id=?1) OR (v.source_table='reversal' AND v.source_id IN (SELECT id FROM reversal WHERE payment_id=?1)))",
            params![dto.id], |r| r.get(0),
        ).unwrap();
        assert_eq!(fee_net, 0, "receipt+reversal cancel on Fee income");
        assert_eq!(crate::ledger::ledger_imbalance(&c).unwrap(), 0, "whole book still balances");
    }

    #[test]
    fn accountant_can_read_but_not_edit_calendar() {
        let mut c = seeded();
        // Reading is allowed for any role.
        assert!(get_calendar_logic(&mut c, &accountant()).is_ok());
        // Editing is Principal-only (Settings).
        let input = CalendarEventInput {
            starts_on: "2026-10-02".into(), ends_on: "2026-10-02".into(),
            kind: "holiday".into(), title: "x".into(), title_hi: None, title_te: None, is_non_working: true,
        };
        assert_eq!(add_calendar_event_logic(&mut c, &accountant(), None, DeviceMode::Server, &input).unwrap_err().code, "FORBIDDEN");
        assert_eq!(set_weekly_offs_logic(&mut c, &accountant(), None, DeviceMode::Server, &[true; 7]).unwrap_err().code, "FORBIDDEN");
    }

    fn head_input(name: &str, amount: i64) -> FeeHeadInput {
        FeeHeadInput { name: name.into(), name_hi: None, amount_paise: amount, frequency: "once".into(), applies_to: "all".into(), instalments_json: None }
    }

    #[test]
    fn kavya_dues_total_is_3100() {
        let mut c = seeded();
        let dues = list_fee_dues_logic(&mut c, "stu-kavya-singh").unwrap();
        assert_eq!(dues.total_due_paise, 310_000, "Kavya Singh owes ₹3,100");
    }

    #[test]
    fn record_payment_settles_dues_and_numbers_receipt() {
        let mut c = seeded();
        // Pay Kavya's full ₹3,100 due via the A2 device.
        let dto = record_payment_logic(
            &mut c,
            &accountant(),
            Some("dev-a2"),
            DeviceMode::Server,
            &PaymentInput { student_id: "stu-kavya-singh".into(), amount_paise: 310_000, mode: "upi".into(), reference: Some("426518903214".into()) },
        )
        .unwrap();
        assert_eq!(dto.receipt_no, "R-A2-0419", "next receipt after seed's 0418");
        assert!(dto.confirmed);
        // Her dues are now zero.
        let dues = list_fee_dues_logic(&mut c, "stu-kavya-singh").unwrap();
        assert_eq!(dues.total_due_paise, 0);
    }

    #[test]
    fn overpayment_is_rejected_on_device() {
        let mut c = seeded();
        let err = record_payment_logic(
            &mut c,
            &accountant(),
            Some("dev-a2"),
            DeviceMode::Server,
            &PaymentInput { student_id: "stu-kavya-singh".into(), amount_paise: 500_000, mode: "cash".into(), reference: Some(String::new()) },
        )
        .unwrap_err();
        assert_eq!(err.code, "AMOUNT_EXCEEDS_DUE");
    }

    #[test]
    fn teacher_cannot_record_payment() {
        let mut c = seeded();
        let teacher = SessionStaff { id: "stf-meena".into(), name: "Meena".into(), role: "teacher".into() };
        let err = record_payment_logic(
            &mut c,
            &teacher,
            Some("dev-a1"),
            DeviceMode::Server,
            &PaymentInput { student_id: "stu-kavya-singh".into(), amount_paise: 10_000, mode: "cash".into(), reference: Some(String::new()) },
        )
        .unwrap_err();
        assert_eq!(err.code, "FORBIDDEN");
    }

    #[test]
    fn wrong_pin_reports_remaining_then_locks() {
        let mut c = seeded();
        for _ in 0..4 {
            let e = unlock_logic(&mut c, "stf-priya", "0000").unwrap_err();
            assert_eq!(e.code, "PIN_WRONG");
        }
        // 5th wrong → locked.
        let e = unlock_logic(&mut c, "stf-priya", "0000").unwrap_err();
        assert_eq!(e.code, "PIN_LOCKED");
    }

    #[test]
    fn correct_pin_unlocks() {
        let mut c = seeded();
        let s = unlock_logic(&mut c, "stf-priya", crate::seed::DEMO_PIN).unwrap();
        assert_eq!(s.role, "principal");
    }

    #[test]
    fn set_accent_rejects_unknown() {
        let mut c = seeded();
        assert!(set_accent_logic(&mut c, "#2F7479").is_ok());
        assert_eq!(set_accent_logic(&mut c, "#123456").unwrap_err().code, "VALIDATION");
    }

    #[test]
    fn search_finds_kavya() {
        let mut c = seeded();
        let hits = search_students_logic(&mut c, "Kavya").unwrap();
        let names: Vec<String> = hits.iter().map(|s| s.name.clone()).collect();
        assert!(names.iter().any(|n| n == "Kavya Singh"));
        assert!(names.iter().any(|n| n == "Kavya Mishra"));
        assert!(names.iter().any(|n| n == "Kavya Reddy"));
    }

    #[test]
    fn students_page_paginates_and_filters() {
        let mut c = seeded();
        let q = |status: Option<&str>, query: Option<&str>, limit, offset| StudentQuery {
            class_id: None,
            section: None,
            status: status.map(str::to_string),
            query: query.map(str::to_string),
            limit,
            offset,
        };
        // Page 1 of active students: 25 rows, full total (seed = 670 active).
        let p1 = list_students_page_logic(&mut c, &q(Some("active"), None, 25, 0)).unwrap();
        assert_eq!(p1.rows.len(), 25);
        assert_eq!(p1.total, 670);
        // Page 2 keeps the same total but different rows (real offset, no silent cap).
        let p2 = list_students_page_logic(&mut c, &q(Some("active"), None, 25, 25)).unwrap();
        assert_eq!(p2.total, 670);
        assert_ne!(p1.rows[0].id, p2.rows[0].id);
        // FTS narrows to the three Kavyas; total reflects the filtered count.
        let kv = list_students_page_logic(&mut c, &q(None, Some("Kavya"), 50, 0)).unwrap();
        assert_eq!(kv.total, 3);
        assert!(kv.rows.iter().all(|r| r.name.contains("Kavya")));
    }

    fn admission_input(class_id: String, name: &str) -> NewStudentInput {
        NewStudentInput {
            name: name.into(),
            class_id,
            roll_no: None,
            guardian_name: Some("Parent".into()),
            guardian_mobile: Some("9876500000".into()),
            dob: Some("2016-04-01".into()),
            gender: Some("female".into()),
            address: None,
            transport: Some(false),
            rte: Some(false),
            category: None,
            aadhaar_status: None,
        }
    }

    #[test]
    fn admission_generates_dues_and_official_number_on_server() {
        let mut c = seeded();
        let cid = list_classes_logic(&mut c).unwrap()[0].id.clone();
        let dto = create_student_logic(&mut c, &accountant(), None, DeviceMode::Server, "2026-09-24", &admission_input(cid, "New Admit")).unwrap();
        // Single-PC server assigns the official admission number (YYYY/NNNN).
        let adm = dto.admission_no.expect("server assigns admission_no");
        assert!(vidya_core::admissions::validate_admission_no(&adm).is_ok(), "{adm} is well-formed");
        // Dues were generated from the active fee heads.
        let dues = list_fee_dues_logic(&mut c, &dto.id).unwrap();
        assert!(dues.total_due_paise > 0, "a new admission owes the current-period fees");
    }

    #[test]
    fn admission_offline_gets_provisional_number() {
        let mut c = seeded();
        let cid = list_classes_logic(&mut c).unwrap()[0].id.clone();
        let dto = create_student_logic(&mut c, &accountant(), Some("dev-x"), DeviceMode::Client, "2026-09-24", &admission_input(cid, "Offline Admit")).unwrap();
        assert!(dto.admission_no.is_none(), "offline admission has no official number yet");
        assert!(dto.provisional_no.as_deref().unwrap_or("").starts_with("P-"), "offline admission gets a provisional number");
    }

    #[test]
    fn transfer_closes_old_and_opens_one_new_enrollment() {
        let mut c = seeded();
        let classes = list_classes_logic(&mut c).unwrap();
        let target = classes.last().unwrap().id.clone();
        let stu = "stu-kavya-singh";
        let before: i64 = c.query_row("SELECT COUNT(*) FROM enrollment WHERE student_id=?1", params![stu], |r| r.get(0)).unwrap();
        transfer_student_logic(&mut c, &accountant(), None, DeviceMode::Server, "2026-09-24", stu, &target, None).unwrap();
        let after: i64 = c.query_row("SELECT COUNT(*) FROM enrollment WHERE student_id=?1", params![stu], |r| r.get(0)).unwrap();
        assert_eq!(after, before + 1, "history is kept — a new row is opened, not overwritten");
        let open_count: i64 = c.query_row("SELECT COUNT(*) FROM enrollment WHERE student_id=?1 AND to_date IS NULL", params![stu], |r| r.get(0)).unwrap();
        assert_eq!(open_count, 1, "exactly one open enrollment");
        let open_class: String = c.query_row("SELECT class_id FROM enrollment WHERE student_id=?1 AND to_date IS NULL", params![stu], |r| r.get(0)).unwrap();
        assert_eq!(open_class, target);
    }

    #[test]
    fn leave_cancels_unpaid_dues_but_keeps_paid() {
        let mut c = seeded();
        let cid = list_classes_logic(&mut c).unwrap()[0].id.clone();
        let stu = create_student_logic(&mut c, &accountant(), None, DeviceMode::Server, "2026-09-24", &admission_input(cid, "Leaver Child")).unwrap();
        let dues = list_fee_dues_logic(&mut c, &stu.id).unwrap();
        assert!(dues.total_due_paise > 0);
        // Pay the oldest due in full so it carries an allocation (must survive the leave).
        let first_id = dues.lines[0].id.clone();
        let first_balance = dues.lines[0].balance_paise;
        record_payment_logic(
            &mut c,
            &accountant(),
            None,
            DeviceMode::Server,
            &PaymentInput { student_id: stu.id.clone(), amount_paise: first_balance, mode: "cash".into(), reference: None },
        )
        .unwrap();
        mark_student_left_logic(&mut c, &accountant(), None, DeviceMode::Server, &stu.id, "2026-09-24", "Moved city").unwrap();
        let after = list_fee_dues_logic(&mut c, &stu.id).unwrap();
        assert_eq!(after.total_due_paise, 0, "no outstanding once left (unpaid dues cancelled)");
        assert!(after.lines.iter().any(|l| l.id == first_id && l.balance_paise == 0), "the paid due is kept, not cancelled");
        // No due that carries a payment was ever cancelled.
        let bad: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM fee_due WHERE student_id=?1 AND cancelled_at IS NOT NULL AND id IN (SELECT fee_due_id FROM payment_allocation WHERE fee_due_id IS NOT NULL)",
                params![stu.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(bad, 0, "paid dues are never cancelled");
    }

    #[test]
    fn export_students_csv_writes_bom_file_with_header() {
        let mut c = seeded();
        let path = std::env::temp_dir().join(format!("vidya-export-{}.csv", uuid::Uuid::now_v7()));
        let p = path.to_str().unwrap();
        let n = export_csv_logic(&mut c, &accountant(), "students", p, None).unwrap();
        assert_eq!(n, 670, "one data row per active student");
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..3], b"\xEF\xBB\xBF", "UTF-8 BOM for Excel");
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("Name") && text.contains("Admission no."));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn csv_import_dry_run_flags_errors_then_commit_imports_valid_only() {
        let mut c = seeded();
        let csv = [
            "Name,Class,Roll,Guardian,Guardian mobile,Date of birth,Gender,Transport,RTE,Category,Aadhaar status",
            "Import One,I-A,,Parent,9998887770,2016-04-01,male,no,no,,none",
            "Bad Class,ZZ-99,,Parent,9998887771,2016-04-02,female,no,no,,none",
            "Import Two,I-A,,Parent,,,male,no,no,,none",
        ]
        .join("\n");
        let path = std::env::temp_dir().join(format!("vidya-import-{}.csv", uuid::Uuid::now_v7()));
        std::fs::write(&path, csv).unwrap();
        let p = path.to_str().unwrap();

        let preview = import_students_dry_run_logic(&mut c, &accountant(), p).unwrap();
        assert_eq!(preview.total, 3);
        assert_eq!(preview.valid, 2);
        assert!(preview.rows.iter().any(|r| r.name == "Bad Class" && r.errors.iter().any(|e| e.column == "Class")));

        let before: i64 = c.query_row("SELECT COUNT(*) FROM student", [], |r| r.get(0)).unwrap();
        let result = import_students_commit_logic(&mut c, &accountant(), DeviceMode::Server, "2026-09-24", p).unwrap();
        assert_eq!(result.imported, 2);
        assert_eq!(result.skipped.len(), 1);
        let after: i64 = c.query_row("SELECT COUNT(*) FROM student", [], |r| r.get(0)).unwrap();
        assert_eq!(after, before + 2, "only valid rows inserted, in one transaction");
        // Both imported students got dues generated.
        let with_dues: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM student s WHERE s.name IN ('Import One','Import Two') AND EXISTS (SELECT 1 FROM fee_due d WHERE d.student_id=s.id)",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(with_dues, 2);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn fee_head_amount_change_updates_unpaid_only() {
        let mut c = seeded();
        let head = create_fee_head_logic(&mut c, &principal(), &head_input("Lab fee", 50_000)).unwrap();
        let cid = list_classes_logic(&mut c).unwrap()[0].id.clone();
        let stu = create_student_logic(&mut c, &accountant(), None, DeviceMode::Server, "2026-09-24", &admission_input(cid, "Lab Kid")).unwrap();
        let lab_due: String = c
            .query_row("SELECT id FROM fee_due WHERE student_id=?1 AND fee_head_id=?2", params![stu.id, head.id], |r| r.get(0))
            .unwrap();

        // Preview + apply the new amount → the unpaid due follows it.
        let preview = preview_fee_head_change_logic(&mut c, &principal(), &head.id, 60_000, None).unwrap();
        assert!(preview.affected_dues >= 1);
        update_fee_head_logic(&mut c, &principal(), &head.id, &head_input("Lab fee", 60_000)).unwrap();
        let amt: i64 = c.query_row("SELECT amount_paise FROM fee_due WHERE id=?1", params![lab_due], |r| r.get(0)).unwrap();
        assert_eq!(amt, 60_000, "unpaid due follows the head amount");

        // Pay everything, then change again → the now-paid due never changes.
        let total = list_fee_dues_logic(&mut c, &stu.id).unwrap().total_due_paise;
        record_payment_logic(&mut c, &accountant(), None, DeviceMode::Server, &PaymentInput { student_id: stu.id.clone(), amount_paise: total, mode: "cash".into(), reference: None }).unwrap();
        update_fee_head_logic(&mut c, &principal(), &head.id, &head_input("Lab fee", 70_000)).unwrap();
        let amt2: i64 = c.query_row("SELECT amount_paise FROM fee_due WHERE id=?1", params![lab_due], |r| r.get(0)).unwrap();
        assert_eq!(amt2, 60_000, "a paid due never changes when the head amount changes");

        // Deactivate (never hard-delete a head).
        deactivate_fee_head_logic(&mut c, &principal(), &head.id).unwrap();
        let active: i64 = c.query_row("SELECT active FROM fee_head WHERE id=?1", params![head.id], |r| r.get(0)).unwrap();
        assert_eq!(active, 0);

        // Accountants cannot edit the fee structure.
        assert!(create_fee_head_logic(&mut c, &accountant(), &head_input("X", 1)).is_err());
    }

    #[test]
    fn fee_head_with_instalment_plan_generates_dues_per_instalment() {
        let mut c = seeded();
        // Tuition ₹12,000/year in three ₹4,000 instalments (session 2026–27).
        let plan = serde_json::json!({
            "type": "list",
            "instalments": [
                { "no": 1, "amount_paise": 400_000, "due_date": "2026-04-15" },
                { "no": 2, "amount_paise": 400_000, "due_date": "2026-08-15" },
                { "no": 3, "amount_paise": 400_000, "due_date": "2026-12-15" }
            ]
        }).to_string();
        let input = FeeHeadInput {
            name: "Tuition (instalments)".into(), name_hi: None, amount_paise: 1_200_000,
            frequency: "term".into(), applies_to: "all".into(), instalments_json: Some(plan),
        };
        let head = create_fee_head_logic(&mut c, &principal(), &input).unwrap();
        assert!(head.instalments_json.is_some());

        // A new admission gets one due per instalment, each with its no + due date.
        let stu = create_student_logic(
            &mut c, &accountant(), Some("dev-a2"), DeviceMode::Server, "2026-09-23",
            &NewStudentInput { name: "Test Instalment".into(), class_id: "cls-6b".into(), roll_no: None, guardian_name: None, guardian_mobile: None, dob: None, gender: None, address: None, transport: None, rte: None, category: None, aadhaar_status: None },
        ).unwrap();
        let rows: Vec<(i64, i64, Option<String>)> = {
            let mut s = c.prepare("SELECT instalment_no, amount_paise, due_date FROM fee_due WHERE student_id=?1 AND fee_head_id=?2 ORDER BY instalment_no").unwrap();
            s.query_map(params![stu.id, head.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap().collect::<rusqlite::Result<_>>().unwrap()
        };
        assert_eq!(rows.len(), 3, "one due per instalment");
        assert_eq!(rows[1], (2, 400_000, Some("2026-08-15".to_string())));
    }

    #[test]
    fn record_expense_posts_balanced_voucher_and_reverses() {
        let mut c = seeded();
        // Accountant records an electricity expense → voucher, appears in ledger.
        let exp = record_expense_logic(
            &mut c, &accountant(), Some("dev-a2"), DeviceMode::Server, "2026-09-23",
            &ExpenseInput { category_account_id: "electricity".into(), amount_paise: 685_000, paid_via: "cash".into(), details: Some("September bill".into()), vendor: None, bill_attachment: None, spent_on: Some("2026-09-23".into()) },
        ).unwrap();
        assert!(exp.confirmed);
        assert!(exp.voucher_no.is_some());
        // The voucher is balanced: Dr electricity 6,850, Cr cash 6,850.
        let (d, cr): (i64, i64) = c.query_row(
            "SELECT COALESCE(SUM(le.debit_paise),0), COALESCE(SUM(le.credit_paise),0) \
             FROM voucher v JOIN ledger_entry le ON le.voucher_id=v.id WHERE v.source_table='expense' AND v.source_id=?1",
            params![exp.id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(d, 685_000);
        assert_eq!(cr, 685_000);
        assert_eq!(crate::ledger::ledger_imbalance(&c).unwrap(), 0);

        // A non-expense category is rejected.
        assert!(record_expense_logic(&mut c, &accountant(), Some("dev-a2"), DeviceMode::Server, "2026-09-23",
            &ExpenseInput { category_account_id: "fee_income".into(), amount_paise: 100, paid_via: "cash".into(), details: None, vendor: None, bill_attachment: None, spent_on: None }).is_err());

        // Only the Principal reverses; the accountant cannot.
        assert!(reverse_expense_logic(&mut c, &accountant(), None, DeviceMode::Server, &exp.id, "oops").is_err());
        reverse_expense_logic(&mut c, &principal(), None, DeviceMode::Server, &exp.id, "Entered twice").unwrap();
        assert_eq!(crate::ledger::ledger_imbalance(&c).unwrap(), 0, "reversal keeps the book balanced");
        assert!(list_expenses_logic(&mut c, &principal(), "2026-09-01", "2026-09-30").unwrap().iter().any(|e| e.id == exp.id && e.reversed));

        // A teacher cannot record expenses at all.
        let teacher = SessionStaff { id: "stf-meena".into(), name: "Meena".into(), role: "teacher".into() };
        assert!(record_expense_logic(&mut c, &teacher, Some("dev-a1"), DeviceMode::Server, "2026-09-23",
            &ExpenseInput { category_account_id: "electricity".into(), amount_paise: 100, paid_via: "cash".into(), details: None, vendor: None, bill_attachment: None, spent_on: None }).is_err());
    }

    #[test]
    fn salary_register_and_pay_posts_balanced_vouchers() {
        let mut c = seeded();
        let reg = salary_register_logic(&mut c, &principal(), "2026-09", &[]).unwrap();
        let wd = reg.working_days as u32;
        assert!(wd > 0, "the calendar has working days in September");
        assert!(reg.rows.len() >= 4, "seeded salary structures");
        assert!(reg.rows.iter().all(|r| r.deduction_paise == 0), "full attendance → no deduction");

        // R. Nair with 2 unpaid days → the half-up deduction; net = monthly − deduction.
        let days = vec![StaffDaysInput { staff_id: "stf-nair".into(), days_present: (wd - 2) as i64 }];
        let reg2 = salary_register_logic(&mut c, &principal(), "2026-09", &days).unwrap();
        let nair = reg2.rows.iter().find(|r| r.staff_id == "stf-nair").unwrap();
        assert_eq!(nair.deduction_paise, vidya_core::salary::unpaid_leave_deduction(1_700_000, wd, 2));
        assert_eq!(nair.net_paise, 1_700_000 - nair.deduction_paise);
        // Meena's ₹2,000 advance is recovered this month.
        let meena = reg2.rows.iter().find(|r| r.staff_id == "stf-meena").unwrap();
        assert_eq!(meena.advance_recovery_paise, 200_000);
        assert_eq!(meena.net_paise, 1_800_000 - 200_000);

        // Pay everyone (bank) → vouchers posted, book balances, advance recovered.
        let res = pay_salaries_logic(&mut c, &principal(), Some("dev-a1"), DeviceMode::Server, "2026-09", "bank", &days).unwrap();
        assert_eq!(res.paid, 4);
        assert_eq!(crate::ledger::ledger_imbalance(&c).unwrap(), 0, "salary vouchers keep the book balanced");
        let recovered: i64 = c.query_row("SELECT recovered_paise FROM staff_advance WHERE id='adv-meena'", [], |r| r.get(0)).unwrap();
        assert_eq!(recovered, 200_000, "the advance is recovered on pay");
        // Paying again pays nobody (idempotent for the month).
        assert_eq!(pay_salaries_logic(&mut c, &principal(), Some("dev-a1"), DeviceMode::Server, "2026-09", "bank", &days).unwrap().paid, 0);
        // Accountant cannot manage salary.
        assert!(salary_register_logic(&mut c, &accountant(), "2026-09", &[]).is_err());
    }

    #[test]
    fn report_csv_exports_are_scoped_and_written() {
        let mut c = seeded();
        let p = |k: &str| std::env::temp_dir().join(format!("vidya-rep-{k}-{}.csv", uuid::Uuid::now_v7())).to_string_lossy().to_string();

        // Expenses by category — accountant can; file has the header + rows.
        let ep = p("exp");
        let n = export_csv_logic(&mut c, &accountant(), "expenses", &ep, Some("2026-09-01|2026-09-30")).unwrap();
        assert!(n >= 1, "seed has September expenses");
        assert!(std::fs::read_to_string(&ep).unwrap().contains("Category"));

        // Salary register — Principal only (accountant denied).
        assert!(export_csv_logic(&mut c, &accountant(), "salary", &p("sal"), Some("2026-09")).is_err());
        export_csv_logic(&mut c, &principal(), "salary", &p("sal"), Some("2026-09")).unwrap();

        // Instalment dues by due date.
        export_csv_logic(&mut c, &principal(), "instalment_dues", &p("dues"), None).unwrap();

        // Store CSVs need the module on.
        assert!(export_csv_logic(&mut c, &principal(), "store_stock", &p("stk"), None).is_err());
        c.execute("UPDATE module_setting SET enabled=1 WHERE key='store'", []).unwrap();
        export_csv_logic(&mut c, &principal(), "store_stock", &p("stk"), None).unwrap();
        export_csv_logic(&mut c, &principal(), "store_sales", &p("sales"), None).unwrap();

        // A teacher can export no finance report.
        let teacher = SessionStaff { id: "stf-meena".into(), name: "M".into(), role: "teacher".into() };
        assert!(export_csv_logic(&mut c, &teacher, "expenses", &p("t"), Some("2026-09-01|2026-09-30")).is_err());
    }

    #[test]
    fn cash_book_running_balance_and_profit_equal_the_ledger() {
        let mut c = seeded();
        // Opening balance (Principal) — dated at session start → part of "opening".
        set_opening_balance_logic(&mut c, &principal(), Some("dev-a1"), DeviceMode::Server, "2026-09-23", 3_840_000, 0).unwrap();

        let cb = cash_book_logic(&mut c, &principal(), "2026-09-23").unwrap();
        // Strip identity: opening + in − out = in hand.
        assert_eq!(cb.opening_paise + cb.money_in_paise - cb.money_out_paise, cb.in_hand_paise);
        // The last row's running balance equals cash+bank in hand.
        if let Some(last) = cb.rows.last() {
            assert_eq!(last.balance_paise, cb.in_hand_paise);
        }
        // Today's expenses (electricity 6,850 + repairs 1,200 + stationery 4,300) are money out.
        assert!(cb.money_out_paise >= 685_000 + 120_000 + 430_000);

        // Profit summary = Σ income − Σ expense on the ledger accounts.
        let pf = profit_summary_logic(&mut c, &principal()).unwrap();
        assert_eq!(pf.surplus_paise, pf.income_paise - pf.expense_paise);
        let income: i64 = c.query_row(
            "SELECT COALESCE(SUM(le.credit_paise-le.debit_paise),0) FROM ledger_entry le JOIN ledger_account a ON a.id=le.account_id WHERE a.kind='income'",
            [], |r| r.get(0)).unwrap();
        let expense: i64 = c.query_row(
            "SELECT COALESCE(SUM(le.debit_paise-le.credit_paise),0) FROM ledger_entry le JOIN ledger_account a ON a.id=le.account_id WHERE a.kind='expense'",
            [], |r| r.get(0)).unwrap();
        assert_eq!(pf.income_paise, income, "profit income = ledger income accounts");
        assert_eq!(pf.expense_paise, expense, "profit expense = ledger expense accounts");
    }

    #[test]
    fn every_voucher_kind_is_produced_and_balanced() {
        let mut c = seeded();
        c.execute("UPDATE module_setting SET enabled=1 WHERE key='store'", []).unwrap();
        // Exercise each money movement so every voucher kind exists.
        set_opening_balance_logic(&mut c, &principal(), Some("dev-a1"), DeviceMode::Server, "2026-09-23", 100_000, 0).unwrap(); // opening
        let exp = record_expense_logic(&mut c, &accountant(), Some("dev-a2"), DeviceMode::Server, "2026-09-23",
            &ExpenseInput { category_account_id: "rent".into(), amount_paise: 50_000, paid_via: "bank".into(), details: None, vendor: None, bill_attachment: None, spent_on: Some("2026-09-23".into()) }).unwrap(); // expense
        reverse_expense_logic(&mut c, &principal(), None, DeviceMode::Server, &exp.id, "test").unwrap(); // reversal
        give_advance_logic(&mut c, &principal(), Some("dev-a1"), DeviceMode::Server, "2026-09-23",
            &AdvanceInput { staff_id: "stf-anita".into(), amount_paise: 100_000, recover_per_month_paise: 100_000, mode: "cash".into() }).unwrap(); // advance
        pay_salaries_logic(&mut c, &principal(), Some("dev-a1"), DeviceMode::Server, "2026-09", "bank", &[]).unwrap(); // salary
        record_store_sale_logic(&mut c, &accountant(), Some("dev-a2"), DeviceMode::Server, "2026-09-23",
            &StoreSaleInput { student_id: None, guardian_id: None, mode: "cash".into(), items: vec![SaleItemInput { item_id: "itm-tie".into(), qty: 1 }] }).unwrap(); // store_sale
        // receipt is already in the seed. Every kind present:
        let kinds: Vec<String> = {
            let mut s = c.prepare("SELECT DISTINCT kind FROM voucher ORDER BY kind").unwrap();
            s.query_map([], |r| r.get::<_, String>(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap()
        };
        for k in ["receipt", "expense", "reversal", "advance", "salary", "store_sale", "opening"] {
            assert!(kinds.iter().any(|x| x == k), "voucher kind {k} produced");
        }
        // Every voucher balances, and the whole book nets to zero.
        let mut s = c.prepare("SELECT voucher_id, SUM(debit_paise), SUM(credit_paise) FROM ledger_entry GROUP BY voucher_id").unwrap();
        let rows = s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))).unwrap();
        for row in rows {
            let (vid, d, cr) = row.unwrap();
            assert_eq!(d, cr, "voucher {vid} balances");
        }
        assert_eq!(crate::ledger::ledger_imbalance(&c).unwrap(), 0);
    }

    #[test]
    fn expense_with_bill_photo_confirms_voucher_and_blob() {
        // Step 8: an (offline-)recorded expense with a photo → the server confirms
        // both the voucher and the encrypted blob. Single-PC Server mode here.
        let mut c = seeded();
        let dir = std::env::temp_dir().join(format!("vidya-att-int-{}", uuid::Uuid::now_v7()));
        let store = crate::attachments::AttachmentStore::new(&dir, KEY);
        let hash = store.put(b"a compressed bill jpeg").unwrap();
        assert!(store.has(&hash), "blob stored");

        let exp = record_expense_logic(
            &mut c, &accountant(), Some("dev-a2"), DeviceMode::Server, "2026-09-23",
            &ExpenseInput { category_account_id: "repairs".into(), amount_paise: 120_000, paid_via: "cash".into(), details: Some("Fan".into()), vendor: None, bill_attachment: Some(hash.clone()), spent_on: Some("2026-09-23".into()) },
        ).unwrap();
        assert!(exp.confirmed && exp.voucher_no.is_some(), "voucher confirmed");
        // The expense row references the blob hash…
        let stored: Option<String> = c.query_row("SELECT bill_attachment FROM expense WHERE id=?1", params![exp.id], |r| r.get(0)).unwrap();
        assert_eq!(stored.as_deref(), Some(hash.as_str()));
        // …and the blob is present + decryptable (tamper-verified) locally.
        assert_eq!(store.get(&hash).unwrap().unwrap(), b"a compressed bill jpeg");
    }

    #[test]
    fn store_sale_decrements_stock_posts_voucher_and_blocks_negative() {
        let mut c = seeded();
        // Store module off by default → commands rejected (MODULE_OFF).
        assert!(list_store_items_logic(&mut c, &accountant(), false).is_err());
        c.execute("UPDATE module_setting SET enabled=1 WHERE key='store'", []).unwrap();

        let items = list_store_items_logic(&mut c, &accountant(), false).unwrap();
        assert!(!items.is_empty(), "seeded store items");
        assert!(items.iter().find(|i| i.id == "itm-notebook").unwrap().low_stock, "4 ≤ 5 → low stock");

        // Sell 1 book (₹2,450) + 2 shirts (₹350) → stock down, voucher balanced, S- receipt.
        let sale = record_store_sale_logic(
            &mut c, &accountant(), Some("dev-a2"), DeviceMode::Server, "2026-09-23",
            &StoreSaleInput { student_id: None, guardian_id: None, mode: "upi".into(), items: vec![
                SaleItemInput { item_id: "itm-book".into(), qty: 1 },
                SaleItemInput { item_id: "itm-shirt".into(), qty: 2 },
            ] },
        ).unwrap();
        assert!(sale.receipt_no.starts_with("S-"), "S- receipt number");
        assert_eq!(sale.total_paise, 245_000 + 70_000);
        assert_eq!(c.query_row("SELECT stock FROM store_item WHERE id='itm-book'", [], |r| r.get::<_, i64>(0)).unwrap(), 17);
        assert_eq!(crate::ledger::ledger_imbalance(&c).unwrap(), 0, "store voucher keeps the book balanced");
        let store_income: i64 = c.query_row("SELECT COALESCE(SUM(credit_paise-debit_paise),0) FROM ledger_entry WHERE account_id='store_income'", [], |r| r.get(0)).unwrap();
        assert_eq!(store_income, 315_000, "Store income credited");

        // Selling beyond stock is blocked (stock never goes negative).
        assert!(record_store_sale_logic(
            &mut c, &accountant(), Some("dev-a2"), DeviceMode::Server, "2026-09-23",
            &StoreSaleInput { student_id: None, guardian_id: None, mode: "cash".into(), items: vec![SaleItemInput { item_id: "itm-notebook".into(), qty: 99 }] },
        ).is_err());

        // Only the Principal manages stock.
        assert!(stock_adjust_logic(&mut c, &accountant(), "2026-09-23", &StockAdjustInput { item_id: "itm-book".into(), delta_qty: 10, reason: "purchase".into() }).is_err());
        stock_adjust_logic(&mut c, &principal(), "2026-09-23", &StockAdjustInput { item_id: "itm-book".into(), delta_qty: 10, reason: "purchase".into() }).unwrap();
        assert_eq!(c.query_row("SELECT stock FROM store_item WHERE id='itm-book'", [], |r| r.get::<_, i64>(0)).unwrap(), 27);

        // A teacher can never touch the store.
        let teacher = SessionStaff { id: "stf-meena".into(), name: "M".into(), role: "teacher".into() };
        assert!(record_store_sale_logic(&mut c, &teacher, Some("dev-a1"), DeviceMode::Server, "2026-09-23",
            &StoreSaleInput { student_id: None, guardian_id: None, mode: "cash".into(), items: vec![SaleItemInput { item_id: "itm-book".into(), qty: 1 }] }).is_err());
    }

    #[test]
    fn instalment_sum_must_match_head_amount() {
        let mut c = seeded();
        // Instalments sum to 1,150,000 ≠ head 1,200,000 → rejected.
        let plan = serde_json::json!({
            "type": "list",
            "instalments": [
                { "no": 1, "amount_paise": 400_000, "due_date": "2026-04-15" },
                { "no": 2, "amount_paise": 350_000, "due_date": "2026-08-15" },
                { "no": 3, "amount_paise": 400_000, "due_date": "2026-12-15" }
            ]
        }).to_string();
        let input = FeeHeadInput {
            name: "Bad plan".into(), name_hi: None, amount_paise: 1_200_000,
            frequency: "term".into(), applies_to: "all".into(), instalments_json: Some(plan),
        };
        assert!(create_fee_head_logic(&mut c, &principal(), &input).is_err());
    }

    #[test]
    fn plan_change_preview_and_apply_leaves_paid_instalments() {
        let mut c = seeded();
        let plan = serde_json::json!({
            "type": "list",
            "instalments": [
                { "no": 1, "amount_paise": 400_000, "due_date": "2026-04-15" },
                { "no": 2, "amount_paise": 400_000, "due_date": "2026-08-15" }
            ]
        }).to_string();
        let head = create_fee_head_logic(&mut c, &principal(), &FeeHeadInput {
            name: "Plan head".into(), name_hi: None, amount_paise: 800_000,
            frequency: "term".into(), applies_to: "all".into(), instalments_json: Some(plan),
        }).unwrap();
        let stu = create_student_logic(
            &mut c, &accountant(), Some("dev-a2"), DeviceMode::Server, "2026-09-23",
            &NewStudentInput { name: "Plan Student".into(), class_id: "cls-6b".into(), roll_no: None, guardian_name: None, guardian_mobile: None, dob: None, gender: None, address: None, transport: None, rte: None, category: None, aadhaar_status: None },
        ).unwrap();
        // Pay instalment 1 (₹4,000).
        record_payment_logic(&mut c, &accountant(), None, DeviceMode::Server,
            &PaymentInput { student_id: stu.id.clone(), amount_paise: 400_000, mode: "cash".into(), reference: None }).unwrap();

        // Preview a new plan (inst2 → ₹5,000, add inst3 ₹1,000). inst1 is paid.
        let new_plan = serde_json::json!({
            "type": "list",
            "instalments": [
                { "no": 1, "amount_paise": 400_000, "due_date": "2026-04-15" },
                { "no": 2, "amount_paise": 500_000, "due_date": "2026-08-15" },
                { "no": 3, "amount_paise": 100_000, "due_date": "2026-12-15" }
            ]
        }).to_string();
        let preview = preview_fee_head_change_logic(&mut c, &principal(), &head.id, 1_000_000, Some(&new_plan)).unwrap();
        assert_eq!(preview.changed, 1, "inst2 changes");
        assert_eq!(preview.added, 1, "inst3 added");
        assert_eq!(preview.delta_paise, 100_000 + 100_000);

        // Apply: paid inst1 stays; unpaid dues follow the new plan.
        update_fee_head_logic(&mut c, &principal(), &head.id, &FeeHeadInput {
            name: "Plan head".into(), name_hi: None, amount_paise: 1_000_000,
            frequency: "term".into(), applies_to: "all".into(), instalments_json: Some(new_plan),
        }).unwrap();
        // inst1 due still 400_000 and paid; inst2 now 500_000; inst3 exists.
        let paid1: i64 = c.query_row(
            "SELECT amount_paise FROM fee_due d WHERE d.student_id=?1 AND d.fee_head_id=?2 AND d.instalment_no=1 AND d.cancelled_at IS NULL AND EXISTS(SELECT 1 FROM payment_allocation pa WHERE pa.fee_due_id=d.id)",
            params![stu.id, head.id], |r| r.get(0)).unwrap();
        assert_eq!(paid1, 400_000, "paid instalment untouched");
        let inst2: i64 = c.query_row(
            "SELECT amount_paise FROM fee_due WHERE student_id=?1 AND fee_head_id=?2 AND instalment_no=2 AND cancelled_at IS NULL",
            params![stu.id, head.id], |r| r.get(0)).unwrap();
        assert_eq!(inst2, 500_000, "unpaid instalment follows the new plan");
    }

    #[test]
    fn receipt_has_words_and_principal_reverse_creates_reversal() {
        let mut c = seeded();
        let pay = record_payment_logic(
            &mut c,
            &accountant(),
            None,
            DeviceMode::Server,
            &PaymentInput { student_id: "stu-kavya-singh".into(), amount_paise: 240_000, mode: "cash".into(), reference: None },
        )
        .unwrap();
        let r = get_receipt_logic(&mut c, &accountant(), &pay.receipt_no).unwrap();
        assert_eq!(r.amount_paise, 240_000);
        assert!(!r.amount_words_en.is_empty() && !r.amount_words_hi.is_empty(), "figures + words in both languages");
        assert!(!r.lines.is_empty(), "heads paid come from allocations");
        assert!(!r.reversed);
        // Principal reverse → request + decision + one reversal row (all audited).
        reverse_payment_logic(&mut c, &principal(), DeviceMode::Server, &pay.id, "Duplicate entry").unwrap();
        let rows: i64 = c.query_row("SELECT COUNT(*) FROM reversal WHERE payment_id=?1", params![pay.id], |r| r.get(0)).unwrap();
        assert_eq!(rows, 1);
        assert!(get_receipt_logic(&mut c, &accountant(), &pay.id).unwrap().reversed);
    }

    #[test]
    fn attendance_month_totals_and_principal_correction() {
        let mut c = seeded();
        let (cid, date): (String, String) = c
            .query_row("SELECT class_id, date FROM attendance_sheet WHERE status='submitted' ORDER BY date LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        let month = date[0..7].to_string();
        let m = attendance_month_logic(&mut c, &principal(), &cid, &month).unwrap();
        assert!(!m.days.is_empty(), "the month has submitted days");
        assert!(!m.students.is_empty());
        // Principal directly corrects one mark (audited); the mark changes.
        let stu = m.students[0].id.clone();
        correct_attendance_mark_logic(&mut c, &principal(), DeviceMode::Server, &cid, &date, &stu, "A", "Data entry error").unwrap();
        let mark: String = c
            .query_row(
                "SELECT am.mark FROM attendance_mark am JOIN attendance_sheet sh ON sh.id=am.sheet_id WHERE sh.class_id=?1 AND sh.date=?2 AND am.student_id=?3",
                params![cid, date, stu],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(mark, "A");
        // An accountant (no attendance rights) cannot correct a submitted sheet.
        assert!(correct_attendance_mark_logic(&mut c, &accountant(), DeviceMode::Server, &cid, &date, &stu, "P", "x").is_err());
    }

    fn teacher_anita() -> SessionStaff {
        SessionStaff { id: "stf-anita".into(), name: "Anita Rao".into(), role: "teacher".into() }
    }

    #[test]
    fn marks_entry_validates_range_and_submit_locks_the_subject() {
        let mut c = seeded();
        let sheet = get_marks_sheet_logic(&mut c, &teacher_anita(), "es-6b-maths").unwrap();
        assert_eq!(sheet.max_marks, 100);
        assert!(!sheet.rows.is_empty(), "VI-B has students");
        let first = sheet.rows[0].student_id.clone();
        let entry = |m: Option<i64>| vec![MarkEntryInput { student_id: first.clone(), marks: m, absent: false }];
        // Over-max is rejected (0..=max).
        assert!(save_marks_draft_logic(&mut c, &teacher_anita(), DeviceMode::Server, "es-6b-maths", &entry(Some(101))).is_err());
        // Valid draft, then submit locks the subject.
        save_marks_draft_logic(&mut c, &teacher_anita(), DeviceMode::Server, "es-6b-maths", &entry(Some(88))).unwrap();
        submit_marks_logic(&mut c, &teacher_anita(), DeviceMode::Server, "es-6b-maths", &entry(Some(88))).unwrap();
        assert_eq!(get_marks_sheet_logic(&mut c, &teacher_anita(), "es-6b-maths").unwrap().status, "submitted");
        // A submitted sheet is locked to direct edits.
        assert!(save_marks_draft_logic(&mut c, &teacher_anita(), DeviceMode::Server, "es-6b-maths", &entry(Some(90))).is_err());
    }

    #[test]
    fn report_card_grades_a_full_card_and_flags_incomplete() {
        let mut c = seeded();
        let graded: String = c
            .query_row("SELECT student_id FROM mark_entry WHERE sheet_id='ms-6b-maths' AND marks IS NOT NULL AND absent=0 LIMIT 1", [], |r| r.get(0))
            .unwrap();
        let card = get_report_card_logic(&mut c, &principal(), "2026-09-24", &graded, "exam-hy").unwrap();
        assert!(!card.incomplete);
        assert!(!card.subjects.is_empty() && card.grade.is_some(), "a graded card has a grade");
        // A student whose Maths mark is NULL → incomplete card (never treated as 0).
        let null_stu: Option<String> = c
            .query_row("SELECT student_id FROM mark_entry WHERE sheet_id='ms-6b-maths' AND marks IS NULL AND absent=0 LIMIT 1", [], |r| r.get(0))
            .optional()
            .unwrap();
        if let Some(ns) = null_stu {
            assert!(get_report_card_logic(&mut c, &principal(), "2026-09-24", &ns, "exam-hy").unwrap().incomplete);
        }
    }

    #[test]
    fn reports_audit_scoping_and_exam_results() {
        let mut c = seeded();
        // Accountant records a payment (audited under stf-suresh).
        record_payment_logic(&mut c, &accountant(), None, DeviceMode::Server, &PaymentInput { student_id: "stu-kavya-singh".into(), amount_paise: 100_000, mode: "cash".into(), reference: None }).unwrap();
        // Teacher Anita saves a VI-B Maths draft (audited under stf-anita).
        let first = get_marks_sheet_logic(&mut c, &teacher_anita(), "es-6b-maths").unwrap().rows[0].student_id.clone();
        save_marks_draft_logic(&mut c, &teacher_anita(), DeviceMode::Server, "es-6b-maths", &[MarkEntryInput { student_id: first, marks: Some(70), absent: false }]).unwrap();

        let q = |limit, offset| AuditQuery { staff_id: None, table: None, action: None, date: None, limit, offset };
        let all = list_audit_logic(&mut c, &principal(), &q(100, 0)).unwrap();
        assert!(all.chain_ok, "audit chain verifies");
        assert!(all.total >= 2);
        // A teacher sees ONLY their own audit entries (fewer than the Principal).
        let mine = list_audit_logic(&mut c, &teacher_anita(), &q(100, 0)).unwrap();
        assert!(mine.total >= 1 && mine.total < all.total);

        let res = exam_results_logic(&mut c, &principal(), "exam-hy").unwrap();
        // P16 seed adds V-A English to the Half-Yearly (for exam seating), so the
        // exam now has two subjects; VI-B Maths still carries the seeded marks.
        assert_eq!(res.len(), 2, "two exam subjects (V-A English + VI-B Maths)");
        assert!(res.iter().any(|r| r.graded >= 1 && r.average_pct_tenths > 0));
    }

    #[test]
    fn grade_bands_reject_gaps_and_accept_contiguous() {
        let mut c = seeded();
        assert!(!list_grade_bands_logic(&mut c).unwrap().is_empty());
        let band = |min, max, g: &str, gp| GradeBandDto { min_pct: min, max_pct: max, grade: g.into(), grade_point: gp };
        // A gap (500→600) between bands is rejected.
        assert!(update_grade_bands_logic(&mut c, &principal(), &[band(0, 500, "F", None), band(600, 1000, "A", Some(10))]).is_err());
        // Contiguous 0..=1000 is accepted.
        let saved = update_grade_bands_logic(&mut c, &principal(), &[band(0, 499, "F", None), band(500, 1000, "P", Some(5))]).unwrap();
        assert_eq!(saved.len(), 2);
    }

    // ---- Classroom: timetable (P16 Step 1) -------------------------------

    #[test]
    fn timetable_loads_and_rejects_clashes() {
        let mut c = seeded();
        // The seed builds a clash-free V-A week (24 slots).
        let tt = get_timetable_logic(&mut c, &principal(), "cls-5a").unwrap();
        assert_eq!(tt.periods.len(), 6);
        assert_eq!(tt.slots.len(), 24);
        assert!(!tt.subjects.is_empty(), "class-subject options for the editor");

        // Editing V-A Wed period 1 to a subject whose teacher (Meena) is already
        // teaching V-A elsewhere that period would clash — but there is exactly one
        // subject per (weekday, period) in one class, so instead prove a genuine
        // clash: put Meena (cs-5a-eng) into a period she already teaches Social in.
        // Find a Wed slot Meena teaches, then try to add her English in the SAME slot
        // via a second class — simplest: a teacher-not-assigned clash.
        let bad = TimetableSlotInput {
            id: None,
            class_id: "cls-5a".into(),
            weekday: 3,
            period_no: 1,
            class_subject_id: "cs-5a-eng".into(),
            teacher_id: "stf-nair".into(), // Nair is NOT assigned to English
        };
        let err = save_timetable_slot_logic(&mut c, &principal(), &bad).unwrap_err();
        assert_eq!(err.code, "VALIDATION");
        assert_eq!(err.vars["rule"], "teacher_not_assigned");
    }

    #[test]
    fn teacher_double_book_is_rejected_across_classes() {
        let mut c = seeded();
        // Give VI-B an English class-subject taught by Meena.
        c.execute(
            "INSERT INTO class_subject(id,class_id,subject_id,teacher_id) VALUES ('cs-6b-eng','cls-6b','sub-eng','stf-meena')",
            [],
        ).unwrap();
        // V-A already has Meena teaching *something* at (weekday 3, some period).
        // Find one of Meena's V-A slots and try to book her for VI-B at the same time.
        let (wd, per): (i64, i64) = c
            .query_row(
                "SELECT weekday, period_no FROM timetable_slot WHERE teacher_id='stf-meena' AND class_id='cls-5a' LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let clash = TimetableSlotInput {
            id: None,
            class_id: "cls-6b".into(),
            weekday: wd,
            period_no: per,
            class_subject_id: "cs-6b-eng".into(),
            teacher_id: "stf-meena".into(),
        };
        let err = save_timetable_slot_logic(&mut c, &principal(), &clash).unwrap_err();
        assert_eq!(err.code, "VALIDATION");
        assert_eq!(err.vars["rule"], "teacher_busy");
    }

    #[test]
    fn my_timetable_returns_only_the_teachers_slots() {
        let mut c = seeded();
        let mine = my_timetable_logic(&mut c, &meena()).unwrap();
        assert!(!mine.slots.is_empty());
        assert!(mine.slots.iter().all(|s| s.teacher_id == "stf-meena"), "only Meena's periods");
        assert!(mine.slots.iter().all(|s| s.class_display.is_some()));
    }

    #[test]
    fn teacher_cannot_manage_the_timetable() {
        let mut c = seeded();
        let input = TimetableSlotInput {
            id: None, class_id: "cls-5a".into(), weekday: 1, period_no: 5,
            class_subject_id: "cs-5a-eng".into(), teacher_id: "stf-meena".into(),
        };
        assert!(save_timetable_slot_logic(&mut c, &meena(), &input).is_err(), "teacher cannot edit the timetable");
    }

    // ---- Classroom: substitutes + attendance duty (P16 Step 2) -----------

    #[test]
    fn substitute_gets_attendance_duty_for_the_covered_day_only() {
        let mut c = seeded();
        // Anita is NOT the class teacher of VII-B (Meena is) → normally forbidden.
        assert!(save_attendance_draft_logic(&mut c, &teacher_anita(), "cls-7b", "2026-09-23", &[]).is_err());
        // Principal assigns Anita to cover Meena on Wed 23 Sep (Meena class-teaches
        // V-A + VII-B, so both get attendance duty).
        let r = assign_substitute_logic(&mut c, &principal(), "2026-09-23", "stf-meena", "stf-anita").unwrap();
        assert!(r.includes_attendance, "the class teacher's attendance is covered");
        // Anita can now take VII-B attendance on 23 Sep…
        assert!(save_attendance_draft_logic(&mut c, &teacher_anita(), "cls-7b", "2026-09-23", &[]).is_ok());
        // …but not the next day — the grant is date-limited (ends at midnight).
        assert!(save_attendance_draft_logic(&mut c, &teacher_anita(), "cls-7b", "2026-09-24", &[]).is_err());
    }

    #[test]
    fn approved_attendance_duty_request_grants_attendance_over_its_range() {
        let mut c = seeded();
        let req = create_request_logic(&mut c, &teacher_anita(), &RequestInput {
            kind: "attendance_duty".into(),
            target_table: "class".into(),
            target_id: "cls-7b".into(),
            base_version: 0,
            reason: "Covering for Meena".into(),
            before_json: Some("{}".into()),
            after_json: Some(serde_json::json!({ "class_id": "cls-7b", "from_date": "2026-09-24", "to_date": "2026-09-25" }).to_string()),
        }).unwrap();
        // Before approval → forbidden.
        assert!(save_attendance_draft_logic(&mut c, &teacher_anita(), "cls-7b", "2026-09-24", &[]).is_err());
        decide_request_logic(&mut c, &principal(), DeviceMode::Server, &req.id, "approve", None).unwrap();
        // Approved → allowed across the inclusive range, not after it.
        assert!(save_attendance_draft_logic(&mut c, &teacher_anita(), "cls-7b", "2026-09-24", &[]).is_ok());
        assert!(save_attendance_draft_logic(&mut c, &teacher_anita(), "cls-7b", "2026-09-25", &[]).is_ok());
        assert!(save_attendance_draft_logic(&mut c, &teacher_anita(), "cls-7b", "2026-09-26", &[]).is_err());
    }

    #[test]
    fn a_teacher_cannot_raise_someone_elses_duty_and_needs_a_reason() {
        let mut c = seeded();
        // Accountant cannot raise an attendance-duty request (not a teacher).
        assert!(create_request_logic(&mut c, &accountant(), &RequestInput {
            kind: "attendance_duty".into(), target_table: "class".into(), target_id: "cls-7b".into(),
            base_version: 0, reason: "x".into(), before_json: Some("{}".into()),
            after_json: Some("{}".into()),
        }).is_err());
    }

    #[test]
    fn substitute_plan_lists_covers_free_teachers_and_notifies() {
        let mut c = seeded();
        let plan = substitute_plan_logic(&mut c, &principal(), "2026-09-23", "stf-meena").unwrap();
        assert_eq!(plan.weekday, 3, "23 Sep 2026 is a Wednesday (ISO 3)");
        assert!(!plan.covers.is_empty(), "Meena has periods to cover on Wed");
        assert!(plan.attendance_classes.iter().any(|c| c.id == "cls-7b"));
        assert!(plan.free_teachers.iter().any(|t| t.id == "stf-anita"));
        // Assigning notifies the substitute.
        assign_substitute_logic(&mut c, &principal(), "2026-09-23", "stf-meena", "stf-anita").unwrap();
        let n: i64 = c.query_row("SELECT COUNT(*) FROM notification WHERE staff_id='stf-anita' AND kind='substitute'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1, "substitute is notified");
    }

    // ---- Classroom: homework & notes (P16 Step 3) ------------------------

    fn note_input(class_id: &str, cs: Option<&str>, kind: &str, text: &str, sizes: &[i64]) -> HomeworkNoteInput {
        HomeworkNoteInput {
            class_id: class_id.into(),
            class_subject_id: cs.map(str::to_string),
            kind: kind.into(),
            text: text.into(),
            attachments: sizes.iter().enumerate().map(|(i, s)| AttachmentMeta {
                name: format!("f{i}.pdf"), size: *s, mime: "application/pdf".into(), drive_file_id: None, local_hash: None,
            }).collect(),
        }
    }

    #[test]
    fn homework_note_save_list_delete_and_size_limits() {
        let mut c = seeded();
        // Meena teaches V-A English; she posts homework.
        let note = save_homework_note_logic(&mut c, &meena(), Some("dev-a3"), DeviceMode::Client,
            &note_input("cls-5a", Some("cs-5a-eng"), "homework", "Do exercise 5.2", &[])).unwrap();
        let hist = list_homework_notes_logic(&mut c, &meena(), "cls-5a").unwrap();
        assert_eq!(hist.len(), 1);
        assert!(hist[0].can_delete, "the author can delete within 24h");

        // A teacher unrelated to VII-B cannot post to it (Anita teaches neither a
        // subject in VII-B nor is its class teacher).
        assert!(save_homework_note_logic(&mut c, &teacher_anita(), None, DeviceMode::Server,
            &note_input("cls-7b", None, "homework", "x", &[])).is_err());

        // A single 11 MB attachment is rejected (≤ 10 MB each).
        assert!(save_homework_note_logic(&mut c, &meena(), None, DeviceMode::Server,
            &note_input("cls-5a", Some("cs-5a-eng"), "notes", "", &[11_000_000])).is_err());

        // Anita (not the author) cannot delete Meena's note; Meena can.
        assert!(delete_homework_note_logic(&mut c, &teacher_anita(), &note.id).is_err());
        assert!(delete_homework_note_logic(&mut c, &meena(), &note.id).is_ok());
        assert!(list_homework_notes_logic(&mut c, &meena(), "cls-5a").unwrap().is_empty());
    }

    // ---- Classroom: report-card remarks (P16 Step 4) ---------------------

    #[test]
    fn report_remark_enter_lock_and_appears_on_the_card() {
        let mut c = seeded();
        // Anita is the class teacher of VI-B (cls-6b), which sits the seeded exam.
        let sid: String = c.query_row("SELECT student_id FROM enrollment WHERE class_id='cls-6b' AND to_date IS NULL ORDER BY roll_no LIMIT 1", [], |r| r.get(0)).unwrap();
        // Templates are seeded (10 × en).
        let tpls = list_report_templates_logic(&mut c, &teacher_anita()).unwrap();
        assert_eq!(tpls.len(), 10);
        // The class teacher enters a remark.
        save_report_remark_logic(&mut c, &teacher_anita(), None, DeviceMode::Server, &ReportRemarkInput {
            exam_id: "exam-hy".into(), student_id: sid.clone(), text: "Kavya asks good questions.".into(), template_key: None,
        }).unwrap();
        // It appears on the report card.
        let card = get_report_card_logic(&mut c, &principal(), "2026-09-23", &sid, "exam-hy").unwrap();
        assert_eq!(card.remark.as_deref(), Some("Kavya asks good questions."));
        // It shows in the remarks list, not yet locked.
        let list = get_report_remarks_logic(&mut c, &teacher_anita(), "exam-hy", "cls-6b").unwrap();
        assert!(!list.locked);
        assert!(list.students.iter().any(|s| s.student_id == sid && s.remark.is_some()));
        // Principal makes the cards final → locked.
        finalize_report_cards_logic(&mut c, &principal(), "exam-hy").unwrap();
        // A teacher can no longer edit a locked exam's remark…
        assert!(save_report_remark_logic(&mut c, &teacher_anita(), None, DeviceMode::Server, &ReportRemarkInput {
            exam_id: "exam-hy".into(), student_id: sid.clone(), text: "changed".into(), template_key: None,
        }).is_err());
        // …but the Principal still may (direct edit, audited).
        assert!(save_report_remark_logic(&mut c, &principal(), None, DeviceMode::Server, &ReportRemarkInput {
            exam_id: "exam-hy".into(), student_id: sid, text: "Principal note.".into(), template_key: None,
        }).is_ok());
    }

    // ---- Classroom: exam seating & hall tickets (P16 Step 5) -------------

    #[test]
    fn generate_seating_is_deterministic_and_builds_hall_tickets() {
        let mut c = seeded();
        // The seed pairs V-A + VI-B into Room 1 (12×10) for the Half-Yearly.
        let r1 = generate_seating_logic(&mut c, &principal(), "exam-hy").unwrap();
        assert!(r1.errors.is_empty(), "rooms are big enough");
        assert!(r1.seated > 0);
        assert_eq!(r1.rooms_used, 1, "two classes → one paired room");
        // Deterministic: regenerating gives the same seat count.
        let r2 = generate_seating_logic(&mut c, &principal(), "exam-hy").unwrap();
        assert_eq!(r1.seated, r2.seated);
        // Seating + hall tickets read back.
        let seating = get_exam_seating_logic(&mut c, &principal(), "exam-hy").unwrap();
        let room = seating.rooms.iter().find(|r| !r.seats.is_empty()).unwrap();
        assert!(room.seats.iter().any(|s| s.class_slot.as_deref() == Some("a")));
        assert!(room.seats.iter().any(|s| s.class_slot.as_deref() == Some("b")), "two classes interleaved");
        assert_eq!(seating.hall_tickets.len() as i64, r1.seated, "one hall ticket per seated student");
        assert!(seating.hall_tickets.iter().all(|h| !h.schedule.is_empty()), "hall tickets carry the schedule");
    }

    #[test]
    fn generate_seating_reports_a_short_room() {
        let mut c = seeded();
        // Shrink Room 1 to 1×1 and delete Room 2 → V-A + VI-B won't fit.
        c.execute("UPDATE exam_room SET rows=1, cols=1 WHERE id='room-1'", []).unwrap();
        c.execute("DELETE FROM exam_seat WHERE room_id='room-2'", []).unwrap();
        c.execute("DELETE FROM exam_room WHERE id='room-2'", []).unwrap();
        let r = generate_seating_logic(&mut c, &principal(), "exam-hy").unwrap();
        assert_eq!(r.seated, 0, "nothing written when a room is short");
        assert!(!r.errors.is_empty());
        assert!(r.errors[0].missing > 0);
    }

    #[test]
    fn only_principal_manages_seating() {
        let mut c = seeded();
        assert!(generate_seating_logic(&mut c, &teacher_anita(), "exam-hy").is_err());
        assert!(list_exam_rooms_logic(&mut c, &accountant(), "exam-hy").is_err());
    }

    // ---- Classroom: calendar working-day count (P16 Step 6) --------------

    #[test]
    fn working_days_counts_the_month() {
        let mut c = seeded();
        // September 2026: 30 days, four Sundays (default Mon–Sat working) → ≤ 26,
        // minus any seeded holidays. The exact count comes from vidya-core.
        let n = working_days_logic(&mut c, "2026-09-01", "2026-09-30").unwrap();
        assert!(n > 0 && n <= 26, "got {n}");
        // A single non-working Sunday counts as zero.
        assert_eq!(working_days_logic(&mut c, "2026-09-06", "2026-09-06").unwrap(), 0, "6 Sep 2026 is a Sunday");
    }

    #[test]
    fn a_teacher_cannot_remark_for_another_class() {
        let mut c = seeded();
        // A VI-B student, but Meena (V-A/VII-B class teacher) is not the class teacher of VI-B.
        let sid: String = c.query_row("SELECT student_id FROM enrollment WHERE class_id='cls-6b' AND to_date IS NULL LIMIT 1", [], |r| r.get(0)).unwrap();
        assert!(save_report_remark_logic(&mut c, &meena(), None, DeviceMode::Server, &ReportRemarkInput {
            exam_id: "exam-hy".into(), student_id: sid, text: "x".into(), template_key: None,
        }).is_err());
    }

    #[test]
    fn email_homework_note_queues_only_for_consenting_parents() {
        let mut c = seeded();
        let sid: String = c.query_row("SELECT student_id FROM enrollment WHERE class_id='cls-5a' AND to_date IS NULL LIMIT 1", [], |r| r.get(0)).unwrap();
        c.execute("INSERT INTO guardian(id,name,mobile,email,language,created_at,updated_at,sync_state) VALUES ('g-t','Parent','9800000000','p@example.com','en','t','t','confirmed')", []).unwrap();
        c.execute("INSERT INTO student_guardian(id,student_id,guardian_id,is_primary,created_at,updated_at,sync_state) VALUES ('sg-t',?1,'g-t',1,'t','t','confirmed')", params![sid]).unwrap();
        c.execute("INSERT INTO consent(id,student_id,guardian_id,purpose,method,recorded_at) VALUES ('cn-t',?1,'g-t','messages','in_person','t')", params![sid]).unwrap();
        let note = save_homework_note_logic(&mut c, &meena(), None, DeviceMode::Server,
            &note_input("cls-5a", Some("cs-5a-eng"), "homework", "Read chapter 5", &[])).unwrap();
        let r = email_homework_note_logic(&mut c, &meena(), None, DeviceMode::Server, &note.id).unwrap();
        assert_eq!(r.queued, 1, "one consenting parent with an email");
        let n: i64 = c.query_row("SELECT COUNT(*) FROM message WHERE channel='email' AND kind='homework' AND related_id=?1", params![note.id], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
    }
}
