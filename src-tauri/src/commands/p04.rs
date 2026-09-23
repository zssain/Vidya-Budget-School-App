//! Phase-4 command logic (prompts/P04): staff & access, invitations, devices,
//! sync status, and conflict/review-flag resolution. Pure functions over
//! `&mut Connection` (unit-testable). Admin gates go through vidya-core
//! `permissions::can` (rule §6).

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use vidya_core::permissions::{self, Action, Actor, Target, TargetKind};
use vidya_core::types::{Role, StaffState};

use crate::db::now_iso;
use crate::error::{CmdError, CmdResult};
use crate::state::SessionStaff;

fn new_id(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::now_v7())
}

fn role_of(s: &str) -> Role {
    match s {
        "principal" => Role::Principal,
        "accountant" => Role::Accountant,
        _ => Role::Teacher,
    }
}

/// Gate an admin action through vidya-core (class assignments irrelevant here).
fn require(actor: &SessionStaff, action: Action, kind: TargetKind) -> CmdResult<()> {
    let a = Actor { staff_id: actor.id.clone(), role: role_of(&actor.role), state: StaffState::Active, class_teacher_of: vec![], class_subjects: vec![] };
    let d = permissions::can(&a, action, &Target::of(kind));
    if d.is_allow() {
        Ok(())
    } else {
        Err(CmdError::forbidden("not_permitted"))
    }
}

// ============================================================ staff ==========

#[derive(Debug, Serialize)]
pub struct StaffFullDto {
    pub id: String,
    pub name: String,
    pub role: String,
    pub mobile: Option<String>,
    pub google_email: Option<String>,
    pub state: String,
    pub class_teacher_of: Vec<String>,   // class displays
    pub class_subjects: Vec<String>,     // "VI-B Maths"
}

pub fn list_staff_full_logic(conn: &mut Connection) -> CmdResult<Vec<StaffFullDto>> {
    struct Base {
        id: String,
        name: String,
        role: String,
        mobile: Option<String>,
        google_email: Option<String>,
        state: String,
    }
    let mut stmt = conn.prepare("SELECT id, name, role, mobile, google_email, state FROM staff WHERE state != 'removed' ORDER BY role, name")?;
    let base: Vec<Base> = stmt
        .query_map([], |r| Ok(Base { id: r.get(0)?, name: r.get(1)?, role: r.get(2)?, mobile: r.get(3)?, google_email: r.get(4)?, state: r.get(5)? }))?
        .collect::<rusqlite::Result<_>>()?;
    drop(stmt);
    let mut out = Vec::new();
    for Base { id, name, role, mobile, google_email, state } in base {
        let ct: Vec<String> = conn
            .prepare("SELECT display FROM class WHERE class_teacher_id=?1 ORDER BY sort_order")?
            .query_map(params![id], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<_>>()?;
        let cs: Vec<String> = conn
            .prepare("SELECT c.display || ' ' || sub.name FROM class_subject cs JOIN class c ON c.id=cs.class_id JOIN subject sub ON sub.id=cs.subject_id WHERE cs.teacher_id=?1")?
            .query_map(params![id], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<_>>()?;
        out.push(StaffFullDto { id, name, role, mobile, google_email, state, class_teacher_of: ct, class_subjects: cs });
    }
    Ok(out)
}

#[derive(Debug, Deserialize)]
pub struct AddStaffInput {
    pub name: String,
    pub role: String, // accountant | teacher (never a second principal)
    pub mobile: String,
    pub google_email: Option<String>,
}

pub fn add_staff_logic(conn: &mut Connection, actor: &SessionStaff, input: &AddStaffInput, now: time::OffsetDateTime) -> CmdResult<InviteDto> {
    require(actor, Action::InviteStaff, TargetKind::Staff)?;
    // Principal cannot create a second Principal here (§5).
    if input.role == "principal" {
        return Err(CmdError::forbidden("cannot_create_principal"));
    }
    if input.role != "accountant" && input.role != "teacher" {
        return Err(CmdError::validation("role", "invalid"));
    }
    let name = vidya_core::validation::validate_name(&input.name)?;
    vidya_core::validation::validate_mobile(&input.mobile)?;
    let id = new_id("stf");
    let now_s = now_iso();
    conn.execute(
        "INSERT INTO staff(id,name,role,mobile,google_email,state,created_at,updated_at,sync_state) \
         VALUES (?1,?2,?3,?4,?5,'invited',?6,?6,'on_device')",
        params![id, name, input.role, input.mobile, input.google_email, now_s],
    )?;
    create_invite_logic(conn, actor, &id, now)
}

pub fn suspend_staff_logic(conn: &mut Connection, actor: &SessionStaff, id: &str) -> CmdResult<()> {
    require(actor, Action::SuspendStaff, TargetKind::Staff)?;
    conn.execute("UPDATE staff SET state='suspended', updated_at=?1 WHERE id=?2 AND role!='principal'", params![now_iso(), id])?;
    Ok(())
}

pub fn remove_staff_logic(conn: &mut Connection, actor: &SessionStaff, id: &str) -> CmdResult<()> {
    require(actor, Action::RemoveStaff, TargetKind::Staff)?;
    conn.execute("UPDATE staff SET state='removed', updated_at=?1 WHERE id=?2 AND role!='principal'", params![now_iso(), id])?;
    // Revoke the staff member's devices immediately.
    conn.execute("UPDATE device SET revoked_at=?1 WHERE staff_id=?2 AND revoked_at IS NULL", params![now_iso(), id])?;
    Ok(())
}

// ========================================================== invites ==========

#[derive(Debug, Serialize)]
pub struct InviteDto {
    pub staff_id: String,
    pub code: String,
    pub link: String,
    pub qr_svg: String,
    pub short_fingerprint: String,
    pub expires_at: String,
}

/// Server LAN facts for the invite payload, read from app_kv (set when the server
/// starts). Falls back to the default port + empty addrs before the server runs.
fn server_facts(conn: &Connection) -> (Vec<String>, u16, String) {
    let lan: Vec<String> = crate::kv::get(conn, "server_lan_addrs").ok().flatten().unwrap_or_default();
    let port: u16 = crate::kv::get(conn, "server_port").ok().flatten().unwrap_or(crate::sync::protocol::DEFAULT_PORT);
    let fp: String = crate::kv::get_raw(conn, "server_fingerprint").ok().flatten().unwrap_or_default();
    (lan, port, fp)
}

pub fn create_invite_logic(conn: &mut Connection, actor: &SessionStaff, staff_id: &str, now: time::OffsetDateTime) -> CmdResult<InviteDto> {
    require(actor, Action::InviteStaff, TargetKind::Staff)?;
    let code = crate::server::service::create_invite(conn, staff_id, now)?;
    let (school_id, school_name): (String, String) = conn
        .query_row("SELECT id, name FROM school LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    let (lan_addrs, port, fp) = server_facts(conn);
    let payload = crate::sync::protocol::JoinPayload {
        school_id,
        school_name,
        lan_addrs,
        port,
        cert_sha256: fp.clone(),
        relay_url: None,
        code: code.clone(),
    };
    let link = crate::server::invite::build_join_link(&payload).map_err(CmdError::internal)?;
    let qr_svg = crate::server::invite::qr_svg(&link).map_err(CmdError::internal)?;
    let expires_at: String = conn
        .query_row("SELECT expires_at FROM invite WHERE code_hash=?1", params![sha256_hex(code.as_bytes())], |r| r.get(0))
        .optional()?
        .unwrap_or_default();
    Ok(InviteDto { staff_id: staff_id.to_string(), code, link, qr_svg, short_fingerprint: crate::server::invite::short_fingerprint(&fp), expires_at })
}

pub fn revoke_invite_logic(conn: &mut Connection, actor: &SessionStaff, staff_id: &str) -> CmdResult<()> {
    require(actor, Action::InviteStaff, TargetKind::Staff)?;
    conn.execute("UPDATE invite SET revoked_at=?1 WHERE staff_id=?2 AND used_at IS NULL AND revoked_at IS NULL", params![now_iso(), staff_id])?;
    Ok(())
}

fn sha256_hex(s: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(s);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

// ====================================================== assignments ==========

pub fn set_class_teacher_logic(conn: &mut Connection, actor: &SessionStaff, class_id: &str, staff_id: Option<&str>) -> CmdResult<()> {
    require(actor, Action::ManageStaff, TargetKind::Staff)?;
    conn.execute("UPDATE class SET class_teacher_id=?1 WHERE id=?2", params![staff_id, class_id])?;
    Ok(())
}

pub fn assign_subject_teacher_logic(conn: &mut Connection, actor: &SessionStaff, class_subject_id: &str, staff_id: Option<&str>) -> CmdResult<()> {
    require(actor, Action::ManageStaff, TargetKind::Staff)?;
    conn.execute("UPDATE class_subject SET teacher_id=?1 WHERE id=?2", params![staff_id, class_subject_id])?;
    Ok(())
}

/// Plain effective-access lines for a staff member (§5 preview).
pub fn effective_access_logic(conn: &mut Connection, staff_id: &str) -> CmdResult<Vec<String>> {
    let role: String = conn.query_row("SELECT role FROM staff WHERE id=?1", params![staff_id], |r| r.get(0)).optional()?.ok_or_else(CmdError::not_found)?;
    let mut lines = Vec::new();
    match role.as_str() {
        "principal" => lines.push("Full access to everything".to_string()),
        "accountant" => {
            lines.push("Admissions and student details".to_string());
            lines.push("Fees: dues, collect payments, receipts, day book".to_string());
        }
        _ => {
            let ct: Vec<String> = conn.prepare("SELECT display FROM class WHERE class_teacher_id=?1 ORDER BY sort_order")?.query_map(params![staff_id], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<_>>()?;
            for c in ct {
                lines.push(format!("Can take attendance for {c}"));
            }
            let cs: Vec<String> = conn.prepare("SELECT c.display || ' ' || sub.name FROM class_subject cs JOIN class c ON c.id=cs.class_id JOIN subject sub ON sub.id=cs.subject_id WHERE cs.teacher_id=?1")?.query_map(params![staff_id], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<_>>()?;
            for s in cs {
                lines.push(format!("Can enter marks: {s}"));
            }
            if lines.is_empty() {
                lines.push("No classes assigned yet".to_string());
            }
        }
    }
    Ok(lines)
}

// ========================================================== devices ==========

#[derive(Debug, Serialize)]
pub struct DeviceDto {
    pub id: String,
    pub owner_name: String,
    pub platform: String,
    pub series: Option<String>,
    pub last_seen_at: Option<String>,
    pub revoked: bool,
    pub needs_rejoin: bool,
}

pub fn list_devices_logic(conn: &mut Connection) -> CmdResult<Vec<DeviceDto>> {
    let mut stmt = conn.prepare(
        "SELECT d.id, COALESCE(s.name,'?'), d.platform, d.receipt_series, d.last_seen_at, d.revoked_at, d.needs_rejoin \
         FROM device d LEFT JOIN staff s ON s.id=d.staff_id ORDER BY d.receipt_series",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(DeviceDto {
                id: r.get(0)?,
                owner_name: r.get(1)?,
                platform: r.get(2)?,
                series: r.get(3)?,
                last_seen_at: r.get(4)?,
                revoked: r.get::<_, Option<String>>(5)?.is_some(),
                needs_rejoin: r.get::<_, i64>(6)? == 1,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

pub fn revoke_device_logic(conn: &mut Connection, actor: &SessionStaff, id: &str, now: time::OffsetDateTime) -> CmdResult<()> {
    require(actor, Action::ManageDevices, TargetKind::Staff)?;
    crate::server::service::revoke_device(conn, id, now)?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct ServerStatusDto {
    pub school_name: String,
    pub server_epoch: i64,
    pub port: u16,
    pub fingerprint: String,
    pub lan_addrs: Vec<String>,
    pub device_count: i64,
}

pub fn server_status_logic(conn: &mut Connection) -> CmdResult<ServerStatusDto> {
    let (name, epoch): (String, i64) = conn.query_row("SELECT name, server_epoch FROM school LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?))).optional()?.ok_or_else(CmdError::not_found)?;
    let (lan_addrs, port, fingerprint) = server_facts(conn);
    let device_count: i64 = conn.query_row("SELECT COUNT(*) FROM device WHERE revoked_at IS NULL", [], |r| r.get(0))?;
    Ok(ServerStatusDto { school_name: name, server_epoch: epoch, port, fingerprint, lan_addrs, device_count })
}

#[derive(Debug, Serialize)]
pub struct SyncStatusDto {
    pub pending_count: i64,
    pub pending_paise: i64,
    pub last_confirmed_at: Option<String>,
}

pub fn sync_status_logic(conn: &mut Connection) -> CmdResult<SyncStatusDto> {
    let pending_count: i64 = conn.query_row("SELECT COUNT(*) FROM outbox", [], |r| r.get(0)).unwrap_or(0);
    let pending_paise: i64 = conn
        .query_row("SELECT COALESCE(SUM(amount_paise),0) FROM payment WHERE sync_state != 'confirmed'", [], |r| r.get(0))
        .unwrap_or(0);
    let last_confirmed_at: Option<String> = conn
        .query_row("SELECT updated_at FROM sync_cursor WHERE peer='server'", [], |r| r.get(0))
        .optional()?
        .flatten();
    Ok(SyncStatusDto { pending_count, pending_paise, last_confirmed_at })
}

// ========================================================= conflicts =========

#[derive(Debug, Serialize)]
pub struct ConflictDto {
    pub id: String,
    pub table: String,
    pub record_id: String,
    pub field: String,
    pub value_a: Option<String>,
    pub value_b: Option<String>,
    pub staff_b: Option<String>,
    pub hlc_b: Option<String>,
}

pub fn list_conflicts_logic(conn: &mut Connection) -> CmdResult<Vec<ConflictDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, \"table\", record_id, field, value_a, value_b, staff_b, hlc_b FROM conflict WHERE status='open' ORDER BY id",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ConflictDto {
                id: r.get(0)?,
                table: r.get(1)?,
                record_id: r.get(2)?,
                field: r.get(3)?,
                value_a: r.get(4)?,
                value_b: r.get(5)?,
                staff_b: r.get(6)?,
                hlc_b: r.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

/// Resolve a conflict as a NORMAL op (never auto): apply the chosen value + audit,
/// then mark the conflict resolved (§8.5 / Step 8).
pub fn resolve_conflict_logic(conn: &mut Connection, actor: &SessionStaff, id: &str, choice: &str, value: Option<&str>) -> CmdResult<()> {
    require(actor, Action::ApproveRequest, TargetKind::Request)?;
    let (table, record_id, field, value_a, value_b): (String, String, String, Option<String>, Option<String>) = conn
        .query_row("SELECT \"table\", record_id, field, value_a, value_b FROM conflict WHERE id=?1 AND status='open'", params![id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
        .optional()?
        .ok_or_else(CmdError::not_found)?;
    let chosen: Option<String> = match choice {
        "keep_a" => value_a,
        "keep_b" => value_b,
        "edit" => value.map(str::to_string),
        _ => return Err(CmdError::validation("choice", "invalid")),
    };
    let epoch: i64 = conn.query_row("SELECT server_epoch FROM school LIMIT 1", [], |r| r.get(0)).unwrap_or(1);
    let op = crate::sync::protocol::Op {
        op_id: new_id("op"),
        hlc: now_iso(),
        device_id: "server".into(),
        staff_id: actor.id.clone(),
        audience: "admin".into(),
        table: table.clone(),
        record_id: record_id.clone(),
        kind: "update".into(),
        payload: serde_json::json!({ field: chosen }),
        base_version: None, // resolution is authoritative — force-apply
        server_epoch: epoch,
    };
    let op_id = op.op_id.clone();
    crate::sync::apply::apply_op(conn, &op)?;
    conn.execute(
        "UPDATE conflict SET status='resolved', resolved_by=?1, resolution_op_id=?2 WHERE id=?3",
        params![actor.id, op_id, id],
    )?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct ReviewFlagDto {
    pub id: String,
    pub kind: String,
    pub ref_table: String,
    pub ref_id: String,
    pub details_json: Option<String>,
}

pub fn list_review_flags_logic(conn: &mut Connection) -> CmdResult<Vec<ReviewFlagDto>> {
    let mut stmt = conn.prepare("SELECT id, kind, ref_table, ref_id, details_json FROM review_flag WHERE status='open' ORDER BY id")?;
    let rows = stmt
        .query_map([], |r| Ok(ReviewFlagDto { id: r.get(0)?, kind: r.get(1)?, ref_table: r.get(2)?, ref_id: r.get(3)?, details_json: r.get(4)? }))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

pub fn resolve_review_flag_logic(conn: &mut Connection, actor: &SessionStaff, id: &str) -> CmdResult<()> {
    require(actor, Action::ApproveRequest, TargetKind::Request)?;
    conn.execute("UPDATE review_flag SET status='resolved', resolved_by=?1 WHERE id=?2", params![actor.id, id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn now() -> time::OffsetDateTime {
        time::OffsetDateTime::parse("2026-09-23T12:00:00Z", &time::format_description::well_known::Rfc3339).unwrap()
    }
    fn seeded() -> Connection {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        crate::seed::seed_demo_school(&mut c, now()).unwrap();
        c
    }
    fn principal() -> SessionStaff {
        SessionStaff { id: "stf-priya".into(), name: "Priya".into(), role: "principal".into() }
    }

    #[test]
    fn add_staff_then_invite_and_join_would_work() {
        let mut c = seeded();
        let inv = add_staff_logic(&mut c, &principal(), &AddStaffInput { name: "New Teacher".into(), role: "teacher".into(), mobile: "9876543210".into(), google_email: None }, now()).unwrap();
        assert!(inv.link.starts_with("vidya://join?d="));
        assert!(inv.qr_svg.contains("<svg"));
        // The invite is valid for a real join.
        let jr = crate::server::service::join(&mut c, &crate::sync::protocol::JoinReq { invite_code: inv.code, device_name: "Phone".into(), platform: "android".into(), google_email: None }, now());
        assert!(jr.is_ok());
    }

    #[test]
    fn cannot_add_second_principal() {
        let mut c = seeded();
        let err = add_staff_logic(&mut c, &principal(), &AddStaffInput { name: "X".into(), role: "principal".into(), mobile: "9876543210".into(), google_email: None }, now()).unwrap_err();
        assert_eq!(err.code, "FORBIDDEN");
    }

    #[test]
    fn teacher_cannot_manage_staff() {
        let mut c = seeded();
        let teacher = SessionStaff { id: "stf-meena".into(), name: "Meena".into(), role: "teacher".into() };
        assert_eq!(add_staff_logic(&mut c, &teacher, &AddStaffInput { name: "X".into(), role: "teacher".into(), mobile: "9876543210".into(), google_email: None }, now()).unwrap_err().code, "FORBIDDEN");
    }

    #[test]
    fn list_devices_and_revoke() {
        let mut c = seeded();
        let devs = list_devices_logic(&mut c).unwrap();
        assert!(devs.len() >= 3); // seed's A1/A2/A3
        revoke_device_logic(&mut c, &principal(), "dev-a3", now()).unwrap();
        let after = list_devices_logic(&mut c).unwrap();
        assert!(after.iter().find(|d| d.id == "dev-a3").unwrap().revoked);
    }

    #[test]
    fn conflict_resolution_applies_and_closes() {
        let mut c = seeded();
        // Seed an open conflict on Kavya's address.
        c.execute(
            "INSERT INTO conflict(id,\"table\",record_id,field,value_a,value_b,staff_b,hlc_b,status) \
             VALUES ('cf1','student','stu-kavya-singh','address','A road','B lane','stf-priya','h','open')",
            [],
        ).unwrap();
        assert_eq!(list_conflicts_logic(&mut c).unwrap().len(), 1);
        resolve_conflict_logic(&mut c, &principal(), "cf1", "keep_b", None).unwrap();
        let addr: String = c.query_row("SELECT address FROM student WHERE id='stu-kavya-singh'", [], |r| r.get(0)).unwrap();
        assert_eq!(addr, "B lane", "chosen value applied");
        assert_eq!(list_conflicts_logic(&mut c).unwrap().len(), 0, "conflict closed");
    }

    #[test]
    fn effective_access_lines() {
        let mut c = seeded();
        let lines = effective_access_logic(&mut c, "stf-meena").unwrap();
        assert!(lines.iter().any(|l| l.contains("Can take attendance for V-A")));
    }
}
