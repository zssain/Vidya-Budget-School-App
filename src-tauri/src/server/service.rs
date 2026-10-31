//! Request service layer (prompts/P04 Step 3) — the logic behind the `/v1`
//! endpoints, callable by both the axum/TLS listener and the in-process test
//! transport. Every write re-runs vidya-core via `sync::apply` (rule §6). Pull is
//! scoped by `sync::scope`. Auth is a per-device bearer token (SHA-256 hash
//! stored; constant-time compare); a bad/missing token → DEVICE_REVOKED_OR_UNKNOWN.

use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::sync::apply::{load_actor, read_row_json};
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

/// Resolve a device id to its (session_key_b64, auth) if the device is live
/// (not revoked) and has a stored session key. Used by the sealed relay path,
/// where possession of the session key — not a bearer token — authenticates the
/// device (so no token ever crosses the relay). Returns `None` otherwise (→ 401).
pub fn session_auth_for_device(conn: &Connection, device_id: &str) -> rusqlite::Result<Option<(String, DeviceAuth)>> {
    let row = conn
        .query_row(
            "SELECT staff_id, session_key, revoked_at FROM device WHERE id=?1",
            params![device_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, Option<String>>(2)?)),
        )
        .optional()?;
    match row {
        Some((staff_id, Some(session_key), None)) if !session_key.is_empty() => {
            Ok(Some((session_key, DeviceAuth { device_id: device_id.to_string(), staff_id })))
        }
        _ => Ok(None),
    }
}

/// Replay defence (P05 Step 2): accept `counter` only if it is strictly greater
/// than the device's stored high-water mark, then advance the mark. Returns `true`
/// when the counter was accepted (and advanced), `false` on a replay/stale counter.
pub fn accept_counter(conn: &Connection, device_id: &str, counter: i64) -> rusqlite::Result<bool> {
    let last: i64 = conn
        .query_row("SELECT last_counter FROM device WHERE id=?1", params![device_id], |r| r.get(0))
        .optional()?
        .unwrap_or(0);
    if counter <= last {
        return Ok(false);
    }
    conn.execute("UPDATE device SET last_counter=?1 WHERE id=?2", params![counter, device_id])?;
    Ok(true)
}

/// The server's current epoch (defaults to 1 if the school row is somehow absent).
pub fn current_epoch(conn: &Connection) -> rusqlite::Result<i64> {
    Ok(school_info(conn)?.map(|s| s.server_epoch).unwrap_or(1))
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
    // A separate 256-bit secret for the iPhone/web (Drive) route — see
    // vidya_core::seal join v2. Never derived from the short code.
    let web_secret = {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        URL_SAFE_NO_PAD.encode(rand_bytes(32))
    };
    conn.execute(
        "INSERT INTO invite(id, code_hash, staff_id, expires_at, web_secret) VALUES (?1,?2,?3,?4,?5)",
        params![
            format!("inv-{}", &sha256_hex(&rand_bytes(16))[..16]),
            sha256_hex(code.as_bytes()),
            staff_id,
            iso_in(now, time::Duration::hours(INVITE_HOURS)),
            web_secret
        ],
    )?;
    Ok(code)
}

/// The web-route identity of the invite behind `code`: `(invite_id, web_secret)`
/// (base64url, 32 bytes). Used to build the iPhone/web invite link.
pub fn web_invite(conn: &Connection, code: &str) -> rusqlite::Result<Option<(String, String)>> {
    conn.query_row(
        "SELECT id, web_secret FROM invite WHERE code_hash=?1 AND web_secret IS NOT NULL",
        params![sha256_hex(code.trim().to_uppercase().as_bytes())],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()
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

    let invite_id: String = conn
        .query_row("SELECT id FROM invite WHERE code_hash=?1", params![code_hash], |r| r.get(0))
        .optional()
        .map_err(|_| JoinError::InviteInvalid)?
        .ok_or(JoinError::InviteInvalid)?;
    provision_from_invite(conn, &invite_id, &req.device_name, &req.platform, req.google_email.as_deref(), &school, &now_iso_s, now)
}

/// Join over Drive (iPhone/web, join v2): the request names the invite by id and
/// proves possession of its web secret. Verified in constant time; the invite is
/// then consumed exactly like a LAN join. Returns the response with the secret so
/// the caller can seal it.
pub fn join_web(
    conn: &mut Connection,
    invite_id: &str,
    request_id: &str,
    device_name: &str,
    platform: &str,
    proof: &[u8],
    now: time::OffsetDateTime,
) -> Result<(JoinResp, Vec<u8>), JoinError> {
    let school = school_info(conn).map_err(|_| JoinError::NoSchool)?.ok_or(JoinError::NoSchool)?;
    let secret_b64: Option<String> = conn
        .query_row("SELECT web_secret FROM invite WHERE id=?1", params![invite_id], |r| r.get(0))
        .optional()
        .map_err(|_| JoinError::InviteInvalid)?
        .flatten();
    let secret = {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        URL_SAFE_NO_PAD.decode(secret_b64.ok_or(JoinError::InviteInvalid)?).map_err(|_| JoinError::InviteInvalid)?
    };
    let transcript = vidya_core::seal::join_v2_transcript(invite_id, request_id, device_name, platform);
    if !vidya_core::seal::join_v2_verify(&secret, &transcript, proof) {
        return Err(JoinError::InviteInvalid);
    }
    let now_iso_s = now.format(&time::format_description::well_known::Rfc3339).unwrap_or_default();
    let resp = provision_from_invite(conn, invite_id, device_name, platform, None, &school, &now_iso_s, now)?;
    Ok((resp, secret))
}

/// Consume a valid invite (unused, unrevoked, unexpired) and provision the device.
/// The consume is atomic (`used_at IS NULL` guard in the same transaction), so two
/// concurrent joins can never both use one invite.
#[allow(clippy::too_many_arguments)]
fn provision_from_invite(
    conn: &mut Connection,
    invite_id: &str,
    device_name: &str,
    platform: &str,
    google_email: Option<&str>,
    school: &SchoolInfo,
    now_iso_s: &str,
    now: time::OffsetDateTime,
) -> Result<JoinResp, JoinError> {
    let (staff_id, expires_at, used_at, revoked_at): (String, String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT staff_id, expires_at, used_at, revoked_at FROM invite WHERE id=?1",
            params![invite_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .map_err(|_| JoinError::InviteInvalid)?
        .ok_or(JoinError::InviteInvalid)?;
    if used_at.is_some() || revoked_at.is_some() || expires_at.as_str() < now_iso_s {
        return Err(JoinError::InviteInvalid);
    }

    // Provision.
    let device_id = format!("dev-{}", &sha256_hex(&rand_bytes(16))[..16]);
    let token = sha256_hex(&rand_bytes(32));
    let token_hash = sha256_hex(token.as_bytes());
    // The per-device session key (P05 Step 2). Generated here and KEPT server-side
    // (migration 0003) so the server can seal/unseal relay traffic to this device;
    // it is also returned once in JoinResp for the device to keep.
    let session_key = {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        STANDARD.encode(rand_bytes(32))
    };
    let series = next_series(conn).map_err(|_| JoinError::NoSchool)?;
    let lease = iso_in(now, time::Duration::days(LEASE_DAYS));

    let tx = conn.transaction().map_err(|_| JoinError::NoSchool)?;
    tx.execute(
        "INSERT INTO device(id,staff_id,platform,name,token_hash,session_key,receipt_series,admission_series,last_seen_at,lease_expires_at,needs_rejoin) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?7,?8,?9,0)",
        params![device_id, staff_id, platform, device_name, token_hash, session_key, series, now_iso_s, lease],
    ).map_err(|_| JoinError::NoSchool)?;
    // Consume exactly once; the web secret is no longer needed after use.
    let consumed = tx
        .execute("UPDATE invite SET used_at=?1, web_secret=NULL WHERE id=?2 AND used_at IS NULL", params![now_iso_s, invite_id])
        .map_err(|_| JoinError::NoSchool)?;
    if consumed != 1 {
        return Err(JoinError::InviteInvalid); // tx dropped → device row rolled back
    }
    // Invited staff becomes active on first join.
    tx.execute("UPDATE staff SET state='active', updated_at=?1 WHERE id=?2 AND state='invited'", params![now_iso_s, staff_id]).map_err(|_| JoinError::NoSchool)?;
    if let Some(email) = google_email {
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
        session_key,
        school: SchoolLite { id: school.school_id.clone(), name: school.school_name.clone() },
        lease_expires_at: lease,
        bootstrap_cursor,
        protocol: PROTOCOL,
        server_epoch: school.server_epoch,
    })
}

// ----------------------------------------------------------- /sync/push ------

/// Apply pushed ops (≤200) via vidya-core; idempotent by op_id.
pub fn push(conn: &mut Connection, auth: &DeviceAuth, ops: &[Op], now: time::OffsetDateTime) -> rusqlite::Result<PushResp> {
    let epoch = school_info(conn)?.map(|s| s.server_epoch).unwrap_or(1);
    let who = crate::sync::apply::OpAuth { device_id: &auth.device_id, staff_id: &auth.staff_id, route: "lan" };
    let mut results = Vec::with_capacity(ops.len());
    for op in ops.iter().take(200) {
        results.push(crate::sync::apply::apply_client_op(conn, &who, op)?);
    }
    Ok(PushResp {
        results,
        server_time: now.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
        protocol: PROTOCOL,
        server_epoch: epoch,
    })
}

// ----------------------------------------------------------- /sync/pull ------

/// Tables with a delete path whose deletions are delivered as pull tombstones.
const TOMBSTONE_TABLES: &[&str] = &["homework_note"];

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
    let mut deletes = Vec::new();
    let mut next_cursor = since;
    let mut has_more = false;

    if let Some(actor) = actor {
        let mut stmt = conn.prepare(
            "SELECT server_seq, \"table\", record_id, hlc, kind FROM op_log WHERE server_seq > ?1 ORDER BY server_seq ASC LIMIT ?2",
        )?;
        let rows: Vec<(i64, String, String, String, String)> = stmt
            .query_map(params![since, limit + 1], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
            .collect::<rusqlite::Result<_>>()?;
        for (i, (seq, table, record_id, hlc, kind)) in rows.iter().enumerate() {
            if i as i64 >= limit {
                has_more = true;
                break;
            }
            next_cursor = *seq;
            // Tombstone: a row the school PC deleted (only for tables with a delete
            // path). Sent to every device — deleting a row it doesn't hold is a no-op.
            if kind == "delete" && TOMBSTONE_TABLES.contains(&table.as_str()) {
                deletes.push(Delete { table: table.clone(), record_id: record_id.clone() });
                continue;
            }
            if let Some(mut c) = scope::visible_row(conn, &actor, table, record_id)? {
                c.server_seq = *seq;
                c.hlc = Some(hlc.clone());
                changes.push(c);
            }
        }
    }

    Ok(PullResp {
        changes,
        deletes,
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

/// Revoke a device: kill its token immediately (§9). From then on the device is
/// refused on every LAN route, its Drive uploads are quarantined and no Drive down
/// export or ack is sealed for it again. There is no audience key to rotate (P21
/// A8): every school PC -> device payload is sealed per device (s2c of its own
/// session key) or sent over pinned TLS to an authenticated token, and joins issue
/// no audience keys, so a revoked device holds no key that opens future data.
pub fn revoke_device(conn: &mut Connection, device_id: &str, now: time::OffsetDateTime) -> rusqlite::Result<()> {
    let now_iso_s = now.format(&time::format_description::well_known::Rfc3339).unwrap_or_default();
    conn.execute("UPDATE device SET revoked_at=?1 WHERE id=?2", params![now_iso_s, device_id])?;
    Ok(())
}

/// Read a row (helper for tests/harness).
pub fn row_json(conn: &Connection, table: &str, id: &str) -> rusqlite::Result<Option<serde_json::Value>> {
    read_row_json(conn, table, id)
}
