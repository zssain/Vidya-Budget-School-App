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

fn new_id(prefix: &str) -> String {
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
}

pub fn setup_school_logic(conn: &mut Connection, input: &SchoolInput) -> CmdResult<()> {
    let name = vidya_core::validation::validate_name(&input.name)?;
    let now = now_iso();
    // A random 16-byte backup salt (recovery-key backup key is derived from it).
    let mut salt = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut salt);

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
            serde_json::json!({ "phone": input.phone }).to_string(), now],
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

    let mut stmt = conn.prepare("SELECT id, amount_paise, frequency, applies_to FROM fee_head WHERE active=1")?;
    let heads: Vec<FeeHead> = stmt
        .query_map([], |r| {
            let id: String = r.get(0)?;
            let amount: i64 = r.get(1)?;
            let freq: String = r.get(2)?;
            let applies: String = r.get(3)?;
            Ok(FeeHead { id, amount_paise: Paise(amount), frequency: frequency_from(&freq), applies_to: parse_applies_to(&applies) })
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
                "INSERT INTO fee_due(id,student_id,fee_head_id,period,amount_paise,created_at,updated_at,sync_state) \
                 VALUES (?1,?2,?3,?4,?5,?6,?6,?7)",
                params![new_id("due"), sid2, d.fee_head_id, d.period, d.amount_paise.get(), now, sync_state],
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
                "INSERT INTO fee_due(id,student_id,fee_head_id,period,amount_paise,created_at,updated_at,sync_state) \
                 VALUES (?1,?2,?3,?4,?5,?6,?6,?7)",
                params![new_id("due"), sid, d.fee_head_id, d.period, d.amount_paise.get(), now, sync_state],
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
}

pub fn list_fee_heads_logic(conn: &mut Connection, actor_s: &SessionStaff) -> CmdResult<Vec<FeeHeadDto>> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::ViewFees, &Target::of(TargetKind::Fee))?;
    let mut stmt = conn.prepare(
        "SELECT h.id, h.name, h.name_hi, h.amount_paise, h.frequency, h.applies_to, h.active, \
           EXISTS(SELECT 1 FROM fee_due d JOIN payment_allocation pa ON pa.fee_due_id=d.id WHERE d.fee_head_id=h.id) \
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
    let id = new_id("head");
    conn.execute(
        "INSERT INTO fee_head(id,name,name_hi,amount_paise,frequency,applies_to,active) VALUES (?1,?2,?3,?4,?5,?6,1)",
        params![id, input.name, input.name_hi, input.amount_paise, input.frequency, input.applies_to],
    )?;
    audit_action(conn, AuditEntry {
        at: now_iso(),
        staff_id: Some(actor_s.id.clone()),
        action: "create_fee_head".into(),
        table: Some("fee_head".into()),
        record_id: Some(id.clone()),
        after_json: Some(serde_json::json!({ "name": input.name, "amount_paise": input.amount_paise }).to_string()),
        ..Default::default()
    })?;
    list_fee_heads_logic(conn, actor_s)?.into_iter().find(|h| h.id == id).ok_or_else(CmdError::not_found)
}

#[derive(Debug, Serialize)]
pub struct FeeHeadChangePreview {
    pub affected_dues: i64,
    pub delta_paise: i64,
}

/// How many UNPAID dues change, and by how much in total, if this head's amount
/// becomes `new_amount`. Paid/partly-paid dues are excluded (never change).
pub fn preview_fee_head_change_logic(conn: &mut Connection, actor_s: &SessionStaff, id: &str, new_amount: i64) -> CmdResult<FeeHeadChangePreview> {
    let actor = actor_from(conn, actor_s)?;
    require_principal(&actor)?;
    let (count, total_now): (i64, i64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(amount_paise),0) FROM fee_due d \
         WHERE d.fee_head_id=?1 AND d.cancelled_at IS NULL \
           AND NOT EXISTS(SELECT 1 FROM payment_allocation pa WHERE pa.fee_due_id=d.id)",
        params![id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(FeeHeadChangePreview { affected_dues: count, delta_paise: count * new_amount - total_now })
}

pub fn update_fee_head_logic(conn: &mut Connection, actor_s: &SessionStaff, id: &str, input: &FeeHeadInput) -> CmdResult<FeeHeadDto> {
    let actor = actor_from(conn, actor_s)?;
    require_principal(&actor)?;
    vidya_core::validation::validate_name(&input.name)?;
    if input.amount_paise < 0 {
        return Err(CmdError::validation("amount", "negative"));
    }
    let now = now_iso();
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE fee_head SET name=?1, name_hi=?2, amount_paise=?3, frequency=?4, applies_to=?5 WHERE id=?6",
        params![input.name, input.name_hi, input.amount_paise, input.frequency, input.applies_to, id],
    )?;
    // Apply the new amount to UNPAID dues only; paid/partly-paid dues never change.
    let affected = tx.execute(
        "UPDATE fee_due SET amount_paise=?1, updated_at=?2 WHERE fee_head_id=?3 AND cancelled_at IS NULL \
           AND NOT EXISTS(SELECT 1 FROM payment_allocation pa WHERE pa.fee_due_id=fee_due.id)",
        params![input.amount_paise, now, id],
    )?;
    crate::security::audit::append(&tx, &AuditEntry {
        at: now.clone(),
        staff_id: Some(actor_s.id.clone()),
        action: "update_fee_head".into(),
        table: Some("fee_head".into()),
        record_id: Some(id.to_string()),
        after_json: Some(serde_json::json!({ "amount_paise": input.amount_paise, "dues_updated": affected }).to_string()),
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
    pub mode: String,
    pub reference: Option<String>,
    pub collected_by_name: Option<String>,
    pub collected_at: String,
    pub confirmed: bool,
    pub lines: Vec<ReceiptLineDto>,
    pub advance_credit_paise: i64,
    pub balance_after_paise: i64,
    pub reversed: bool,
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
        mode,
        reference,
        collected_by_name,
        collected_at,
        confirmed: sync_state == "confirmed",
        lines,
        advance_credit_paise,
        balance_after_paise,
        reversed,
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

pub fn save_attendance_draft_logic(conn: &mut Connection, actor_s: &SessionStaff, class_id: &str, date: &str, marks: &[MarkInput]) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::TakeAttendance, &Target { kind: TargetKind::Attendance, class_id: Some(class_id.to_string()), ..Default::default() })?;
    upsert_sheet_and_marks(conn, class_id, date, marks, false, actor_s)
}

pub fn submit_attendance_logic(conn: &mut Connection, actor_s: &SessionStaff, class_id: &str, date: &str, marks: &[MarkInput]) -> CmdResult<()> {
    let actor = actor_from(conn, actor_s)?;
    require_allow(&actor, Action::TakeAttendance, &Target { kind: TargetKind::Attendance, class_id: Some(class_id.to_string()), ..Default::default() })?;
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
         WHERE d.student_id=?1 AND d.cancelled_at IS NULL ORDER BY d.created_at",
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
    })
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
        FeeHeadInput { name: name.into(), name_hi: None, amount_paise: amount, frequency: "once".into(), applies_to: "all".into() }
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
        let preview = preview_fee_head_change_logic(&mut c, &principal(), &head.id, 60_000).unwrap();
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
        assert_eq!(res.len(), 1, "one exam subject (VI-B Maths)");
        assert!(res[0].graded >= 1 && res[0].average_pct_tenths > 0);
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
}
