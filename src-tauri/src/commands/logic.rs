//! Command logic (prompts/P03 Step 5). Pure functions over `&mut Connection` +
//! params, so they are unit-testable without a Tauri runtime. Business rules are
//! delegated to vidya-core; these functions do permission checks, DB reads/writes
//! (via `with_write` for audited changes) and DTO shaping.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

use vidya_core::errors::CoreError;
use vidya_core::fees::{self, Due};
use vidya_core::money::Paise;
use vidya_core::permissions::{self, Action, Actor, Target, TargetKind};
use vidya_core::receipts;
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

pub async fn activate_licence_impl(state: &State<'_, RtCtx>, code: &str, school_name: &str) -> CmdResult<AppStateResponse> {
    let public_key = crate::config::licence_public_key()
        .ok_or_else(|| CmdError::new("LICENCE_INVALID", "licence.invalid", serde_json::Value::Null))?;
    let api = crate::config::get().licence_api.clone();
    let app_version = env!("CARGO_PKG_VERSION");

    let activation = crate::licence::activate(
        &state.http,
        &api,
        code,
        school_name,
        &state.machine_id,
        app_version,
        &public_key,
    )
    .await?;

    // Hold the verified licence until the wizard creates the school row (the
    // licence table FK needs a school). Persist encrypted in app_kv.
    let pending = serde_json::json!({
        "licence_id": activation.licence.licence_id,
        "school_id": activation.licence.school_id,
        "plan": activation.licence.plan,
        "max_students": activation.licence.max_students,
        "max_devices": activation.licence.max_devices,
        "issued_at": activation.licence.issued_at,
        "signature": activation.signature_b64,
        "raw_json": activation.licence_b64,
    });
    state.with_db(|conn| {
        kv::set(conn, KV_PENDING_LICENCE, &pending)?;
        // Keep the relay secret issued at activation (P05 §10). Only store a
        // non-empty one so a re-activation against a pre-P05 service can't wipe it.
        if !activation.relay_secret.is_empty() {
            kv::set(conn, crate::state::KV_RELAY_SECRET, &activation.relay_secret)?;
        }
        Ok(())
    })?;
    // Return the new app state (should be `activated`).
    let session = state.session.lock().map_err(|_| CmdError::internal("lock"))?.clone();
    state.with_db(|conn| Ok(crate::state::compute(conn, session)?))
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
        conn.execute(
            "INSERT OR IGNORE INTO licence(licence_id,school_id,plan,issued_at,max_students,max_devices,signature,raw_json,status,last_check_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'active',?9)",
            params![get("licence_id"), school_id, get("plan"), get("issued_at"),
                get_u("max_students"), get_u("max_devices"), get("signature"), get("raw_json"), now],
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
    let first = name.trim().split_whitespace().next().unwrap_or("");
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
                })
            },
        )
        .optional()?
        .ok_or_else(CmdError::not_found)?;

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
        ..profile
    })
}

// ---- Transfer section + Mark as left ---------------------------------------

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
    let data = if records.is_empty() { &records[..] } else { &records[1..] };
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

    // Receipt number from this device's series.
    let (series, last_seq) = receipt_series_and_last(conn, device_id)?;
    let receipt_no = receipts::next_receipt_no(&series, last_seq);

    let now = now_iso();
    let pid = new_id("pay");
    let confirmed = device_mode == DeviceMode::Server;
    let sync_state = if confirmed { "confirmed" } else { "on_device" };
    let confirmed_at = if confirmed { Some(now.clone()) } else { None };

    let ctx = WriteCtx { mode: device_mode };
    let dto = {
        let pid = pid.clone();
        let receipt_no = receipt_no.clone();
        let student_id = input.student_id.clone();
        let mode_s = input.mode.clone();
        let allocation = allocation.clone();
        let dev = device_id.map(str::to_string);
        with_write(conn, &ctx, move |tx| {
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

    let applied: bool = matches!(req.kind.as_str(), "attendance_correction" | "payment_reversal");
    let ctx = WriteCtx { mode: device_mode };
    let req_id = id.to_string();
    let kind = req.kind.clone();
    let target_table = req.target_table.clone();
    let target_id = req.target_id.clone();
    let note_owned = note.map(str::to_string);
    let actor_id = actor_s.id.clone();

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
                    params![new_id("rev"), target_id, req.reason, req_id, actor_id, now],
                )?;
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
}
