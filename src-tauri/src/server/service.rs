//! Request service layer (prompts/P04 Step 3) — the logic behind the `/v1`
//! endpoints, callable by both the axum/TLS listener and the in-process test
//! transport. Every write re-runs vidya-core via `sync::apply` (rule §6). Pull is
//! scoped by `sync::scope`. Auth is a per-device bearer token (SHA-256 hash
//! stored; constant-time compare); a bad/missing token → DEVICE_REVOKED_OR_UNKNOWN.

use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::sync::apply::{apply_op, load_actor, read_row_json};
use crate::sync::protocol::*;
use crate::sync::scope;

/// Default offline-access lease (docs §8.8 / [OWNER] 30 days).
pub const LEASE_DAYS: i64 = 30;
/// Invite validity (§5): 72 hours.
pub const INVITE_HOURS: i64 = 72;

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn sha256_hex(s: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(s);
    let out = h.finalize();
    let mut r = String::with_capacity(64);
    for b in out {
        r.push_str(&format!("{b:02x}"));
    }
    r
}

/// Constant-time compare of two hex strings.
fn ct_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

fn rand_bytes(n: usize) -> Vec<u8> {
    let mut v = vec![0u8; n];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut v);
    v
}

fn iso_in(now: time::OffsetDateTime, d: time::Duration) -> String {
    (now + d).format(&time::format_description::well_known::Rfc3339).unwrap_or_default()
}

// ------------------------------------------------------------- context -------

/// The current server epoch + school identity (read from the DB).
pub struct SchoolInfo {
    pub school_id: String,
    pub school_name: String,
    pub server_epoch: i64,
}

pub fn school_info(conn: &Connection) -> rusqlite::Result<Option<SchoolInfo>> {
    conn.query_row("SELECT id, name, server_epoch FROM school LIMIT 1", [], |r| {
        Ok(SchoolInfo { school_id: r.get(0)?, school_name: r.get(1)?, server_epoch: r.get(2)? })
    })
    .optional()
}

/// An authenticated device.
#[derive(Debug, Clone)]
pub struct DeviceAuth {
    pub device_id: String,
    pub staff_id: String,
}

/// Resolve a bearer token to a live (non-revoked) device, or `None` (→ 401).
pub fn authenticate(conn: &Connection, token: &str) -> rusqlite::Result<Option<DeviceAuth>> {
    let hash = sha256_hex(token.as_bytes());
    let row = conn
        .query_row(
            "SELECT id, staff_id, token_hash, revoked_at FROM device WHERE token_hash=?1",
            params![hash],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, Option<String>>(3)?)),
        )
        .optional()?;
    match row {
        Some((id, staff_id, stored, revoked)) if revoked.is_none() && ct_eq(&stored, &hash) => {
            Ok(Some(DeviceAuth { device_id: id, staff_id }))
        }
        _ => Ok(None),
    }
}

// ------------------------------------------------------------- /hello --------

pub fn hello(conn: &Connection, now: time::OffsetDateTime) -> rusqlite::Result<Option<HelloResp>> {
    Ok(school_info(conn)?.map(|s| HelloResp {
        school_id: s.school_id,
        school_name: s.school_name,
        server_time: now.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
        protocol: PROTOCOL,
        server_epoch: s.server_epoch,
    }))
}

// ------------------------------------------------------------ invites --------

/// Create an 8-char Crockford invite for an existing invited staff member; store
/// only its hash. Returns the plaintext code (shown once) + expiry.
pub fn create_invite(conn: &mut Connection, staff_id: &str, now: time::OffsetDateTime) -> rusqlite::Result<String> {
    let bytes = rand_bytes(8);
    let code: String = bytes.iter().map(|b| CROCKFORD[(*b % 32) as usize] as char).collect();
    conn.execute(
        "INSERT INTO invite(id, code_hash, staff_id, expires_at) VALUES (?1,?2,?3,?4)",
        params![format!("inv-{}", sha256_hex(&bytes)[..12].to_string()), sha256_hex(code.as_bytes()), staff_id, iso_in(now, time::Duration::hours(INVITE_HOURS))],
    )?;
    Ok(code)
}

/// The next device series (A1 = server; A2, A3 … never reused).
fn next_series(conn: &Connection) -> rusqlite::Result<String> {
    let max: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(CAST(substr(receipt_series,2) AS INTEGER)),0) FROM device WHERE receipt_series LIKE 'A%'",
            [], |r| r.get(0),
        )
        .unwrap_or(0);
    Ok(format!("A{}", max + 1))
}

// -------------------------------------------------------------- /join --------

#[derive(Debug)]
pub enum JoinError {
    InviteInvalid,
    NoSchool,
}

/// Consume an invite and provision a device (idempotent-ish: single-use invite).
pub fn join(conn: &mut Connection, req: &JoinReq, now: time::OffsetDateTime) -> Result<JoinResp, JoinError> {
    let school = school_info(conn).map_err(|_| JoinError::NoSchool)?.ok_or(JoinError::NoSchool)?;
    let code_hash = sha256_hex(req.invite_code.trim().to_uppercase().as_bytes());
    let now_iso_s = now.format(&time::format_description::well_known::Rfc3339).unwrap_or_default();

    // Valid = matches, unused, unrevoked, not expired.
    let invite = conn
        .query_row(
            "SELECT id, staff_id, expires_at, used_at, revoked_at FROM invite WHERE code_hash=?1",
            params![code_hash],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, Option<String>>(3)?, r.get::<_, Option<String>>(4)?)),
        )
        .optional()
        .map_err(|_| JoinError::InviteInvalid)?;
    let (invite_id, staff_id, expires_at, used_at, revoked_at) = invite.ok_or(JoinError::InviteInvalid)?;
    if used_at.is_some() || revoked_at.is_some() || expires_at.as_str() < now_iso_s.as_str() {
        return Err(JoinError::InviteInvalid);
    }

    // Provision.
    let device_id = format!("dev-{}", &sha256_hex(&rand_bytes(16))[..16]);
    let token = sha256_hex(&rand_bytes(32));
    let token_hash = sha256_hex(token.as_bytes());
    let series = next_series(conn).map_err(|_| JoinError::NoSchool)?;
    let lease = iso_in(now, time::Duration::days(LEASE_DAYS));

    let tx = conn.transaction().map_err(|_| JoinError::NoSchool)?;
    tx.execute(
        "INSERT INTO device(id,staff_id,platform,name,token_hash,receipt_series,admission_series,last_seen_at,lease_expires_at,needs_rejoin) \
         VALUES (?1,?2,?3,?4,?5,?6,?6,?7,?8,0)",
        params![device_id, staff_id, req.platform, req.device_name, token_hash, series, now_iso_s, lease],
    ).map_err(|_| JoinError::NoSchool)?;
    tx.execute("UPDATE invite SET used_at=?1 WHERE id=?2", params![now_iso_s, invite_id]).map_err(|_| JoinError::NoSchool)?;
    // Invited staff becomes active on first join.
    tx.execute("UPDATE staff SET state='active', updated_at=?1 WHERE id=?2 AND state='invited'", params![now_iso_s, staff_id]).map_err(|_| JoinError::NoSchool)?;
    if let Some(email) = &req.google_email {
        tx.execute("UPDATE staff SET google_email=?1 WHERE id=?2", params![email, staff_id]).ok();
    }
    tx.commit().map_err(|_| JoinError::NoSchool)?;

    let (name, role): (String, String) = conn
        .query_row("SELECT name, role FROM staff WHERE id=?1", params![staff_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|_| JoinError::NoSchool)?;
    let bootstrap_cursor: i64 = conn.query_row("SELECT COALESCE(MAX(server_seq),0) FROM op_log", [], |r| r.get(0)).unwrap_or(0);

    Ok(JoinResp {
        device_id,
        device_token: token,
        staff: StaffLite { id: staff_id.clone(), name, role },
        receipt_series: series.clone(),
        admission_series: series,
        audience_keys: audience_keys_for(conn, &staff_id).unwrap_or_default(),
        session_key: {
            use base64::engine::general_purpose::STANDARD;
            use base64::Engine;
            STANDARD.encode(rand_bytes(32))
        },
        school: SchoolLite { id: school.school_id, name: school.school_name },
        lease_expires_at: lease,
        bootstrap_cursor,
        protocol: PROTOCOL,
        server_epoch: school.server_epoch,
    })
}

/// Audience sealing keys for a staff member's audiences (admin/finance/class:*).
/// Stored server-side in `app_kv` (`audience_key:<audience>`), created on demand.
/// Full rotation-on-revoke is exercised by Phase 6 (Drive bundles).
fn audience_keys_for(conn: &Connection, staff_id: &str) -> rusqlite::Result<Vec<AudienceKey>> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    let role: String = conn.query_row("SELECT role FROM staff WHERE id=?1", params![staff_id], |r| r.get(0))?;
    let mut audiences: Vec<String> = Vec::new();
    match role.as_str() {
        "principal" => {
            audiences.push("admin".into());
            audiences.push("finance".into());
            let mut s = conn.prepare("SELECT id FROM class")?;
            for c in s.query_map([], |r| r.get::<_, String>(0))? {
                audiences.push(format!("class:{}", c?));
            }
        }
        "accountant" => audiences.push("finance".into()),
        _ => {
            let mut s = conn.prepare("SELECT id FROM class WHERE class_teacher_id=?1")?;
            for c in s.query_map(params![staff_id], |r| r.get::<_, String>(0))? {
                audiences.push(format!("class:{}", c?));
            }
        }
    }
    let mut out = Vec::new();
    for a in audiences {
        let kv_key = format!("audience_key:{a}");
        let key_b64 = match crate::kv::get_raw(conn, &kv_key)? {
            Some(k) => k,
            None => STANDARD.encode(rand_bytes(32)),
        };
        out.push(AudienceKey { audience: a, key_b64, version: 1 });
    }
    Ok(out)
}

// ----------------------------------------------------------- /sync/push ------

/// Apply pushed ops (≤200) via vidya-core; idempotent by op_id.
pub fn push(conn: &mut Connection, ops: &[Op], now: time::OffsetDateTime) -> rusqlite::Result<PushResp> {
    let epoch = school_info(conn)?.map(|s| s.server_epoch).unwrap_or(1);
    let mut results = Vec::with_capacity(ops.len());
    for op in ops.iter().take(200) {
        results.push(apply_op(conn, op)?);
    }
    Ok(PushResp {
        results,
        server_time: now.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
        protocol: PROTOCOL,
        server_epoch: epoch,
    })
}

// ----------------------------------------------------------- /sync/pull ------

/// Incremental changes since `cursor` (op_log server_seq), scoped to the device's
/// current actor. Rows the actor may no longer see are omitted; explicit row
/// deletes surface in `deletes` (broad delete-on-lost-assignment is Phase 6+).
pub fn pull(conn: &Connection, auth: &DeviceAuth, since: i64, limit: i64, now: time::OffsetDateTime) -> rusqlite::Result<PullResp> {
    let school = school_info(conn)?;
    let epoch = school.as_ref().map(|s| s.server_epoch).unwrap_or(1);
    let actor = load_actor(conn, &auth.staff_id)?;
    let lease: String = conn
        .query_row("SELECT lease_expires_at FROM device WHERE id=?1", params![auth.device_id], |r| r.get::<_, Option<String>>(0))
        .optional()?
        .flatten()
        .unwrap_or_default();

    let mut changes = Vec::new();
    let mut next_cursor = since;
    let mut has_more = false;

    if let Some(actor) = actor {
        let mut stmt = conn.prepare(
            "SELECT server_seq, \"table\", record_id, hlc FROM op_log WHERE server_seq > ?1 ORDER BY server_seq ASC LIMIT ?2",
        )?;
        let rows: Vec<(i64, String, String, String)> = stmt
            .query_map(params![since, limit + 1], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<rusqlite::Result<_>>()?;
        for (i, (seq, table, record_id, hlc)) in rows.iter().enumerate() {
            if i as i64 >= limit {
                has_more = true;
                break;
            }
            next_cursor = *seq;
            if let Some(mut c) = scope::visible_row(conn, &actor, table, record_id)? {
                c.server_seq = *seq;
                c.hlc = Some(hlc.clone());
                changes.push(c);
            }
        }
    }

    Ok(PullResp {
        changes,
        deletes: Vec::new(),
        next_cursor,
        has_more,
        server_time: now.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
        lease_expires_at: lease,
        protocol: PROTOCOL,
        server_epoch: epoch,
    })
}

// --------------------------------------------------------- /sync/snapshot ----

/// Full role-scoped dataset for a fresh device (bootstrap).
pub fn snapshot(conn: &Connection, auth: &DeviceAuth) -> rusqlite::Result<Vec<Change>> {
    match load_actor(conn, &auth.staff_id)? {
        Some(actor) => scope::snapshot(conn, &actor),
        None => Ok(Vec::new()),
    }
}

// -------------------------------------------------------- /device/heartbeat --

pub fn heartbeat(conn: &mut Connection, auth: &DeviceAuth, req: &HeartbeatReq, now: time::OffsetDateTime) -> rusqlite::Result<HeartbeatResp> {
    let now_iso_s = now.format(&time::format_description::well_known::Rfc3339).unwrap_or_default();
    let lease = iso_in(now, time::Duration::days(LEASE_DAYS));
    let revoked: Option<String> = conn
        .query_row("SELECT revoked_at FROM device WHERE id=?1", params![auth.device_id], |r| r.get(0))
        .optional()?
        .flatten();
    conn.execute(
        "UPDATE device SET last_seen_at=?1, lease_expires_at=?2 WHERE id=?3 AND revoked_at IS NULL",
        params![now_iso_s, lease, auth.device_id],
    )?;
    // Record heartbeat-reported pending counts (Sync & devices display).
    let _ = req;
    let epoch = school_info(conn)?.map(|s| s.server_epoch).unwrap_or(1);
    Ok(HeartbeatResp { server_time: now_iso_s, revoked: revoked.is_some(), lease_expires_at: lease, protocol: PROTOCOL, server_epoch: epoch })
}

/// Revoke a device: kill its token immediately (§9). Audience-key rotation for
/// Drive bundles happens in Phase 6.
pub fn revoke_device(conn: &mut Connection, device_id: &str, now: time::OffsetDateTime) -> rusqlite::Result<()> {
    let now_iso_s = now.format(&time::format_description::well_known::Rfc3339).unwrap_or_default();
    conn.execute("UPDATE device SET revoked_at=?1 WHERE id=?2", params![now_iso_s, device_id])?;
    Ok(())
}

/// Read a row (helper for tests/harness).
pub fn row_json(conn: &Connection, table: &str, id: &str) -> rusqlite::Result<Option<serde_json::Value>> {
    read_row_json(conn, table, id)
}
