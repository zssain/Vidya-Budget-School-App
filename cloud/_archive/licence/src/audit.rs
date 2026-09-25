//! Append-only, hash-chained admin audit (Step 5; same recipe as the app §9):
//! `hash = SHA-256(prev_hash ‖ canonical-JSON of the entry)`. The `admin_audit`
//! table has triggers that abort UPDATE/DELETE, so the chain is tamper-evident.

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::crypto::sha256_hex;
use crate::db::now_iso;

/// The canonical, order-fixed payload the chain hash is computed over (excludes
/// `seq`, which the chain makes implicit).
#[derive(Serialize)]
struct Payload<'a> {
    at: &'a str,
    admin_email: &'a str,
    action: &'a str,
    target_type: Option<&'a str>,
    target_id: Option<&'a str>,
    reason: Option<&'a str>,
    details_json: &'a str,
}

fn chain_hash(prev_hash: &str, p: &Payload) -> String {
    let canonical = serde_json::to_string(p).expect("serialize audit payload");
    sha256_hex(format!("{prev_hash}{canonical}").as_bytes())
}

/// Append one audit entry. Every mutating admin action calls this with a `reason`.
pub fn append(
    conn: &Connection,
    admin_email: &str,
    action: &str,
    target_type: Option<&str>,
    target_id: Option<&str>,
    reason: Option<&str>,
    details: serde_json::Value,
) -> rusqlite::Result<()> {
    let prev_hash: String = conn
        .query_row("SELECT hash FROM admin_audit ORDER BY seq DESC LIMIT 1", [], |r| r.get(0))
        .optional()?
        .unwrap_or_default();
    let at = now_iso();
    let details_json = details.to_string();
    let payload = Payload {
        at: &at,
        admin_email,
        action,
        target_type,
        target_id,
        reason,
        details_json: &details_json,
    };
    let hash = chain_hash(&prev_hash, &payload);
    conn.execute(
        "INSERT INTO admin_audit
           (at, admin_email, action, target_type, target_id, reason, details_json, prev_hash, hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![at, admin_email, action, target_type, target_id, reason, details_json, prev_hash, hash],
    )?;
    Ok(())
}

/// The result of walking the whole chain.
pub struct ChainStatus {
    pub ok: bool,
    pub count: i64,
    /// The `seq` of the first entry whose recomputed hash does not match.
    pub first_bad_seq: Option<i64>,
}

/// Recompute the entire chain and report whether it is intact.
pub fn verify_chain(conn: &Connection) -> rusqlite::Result<ChainStatus> {
    let mut stmt = conn.prepare(
        "SELECT seq, at, admin_email, action, target_type, target_id, reason, details_json, prev_hash, hash
           FROM admin_audit ORDER BY seq ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, Option<String>>(5)?,
            r.get::<_, Option<String>>(6)?,
            r.get::<_, String>(7)?,
            r.get::<_, String>(8)?,
            r.get::<_, String>(9)?,
        ))
    })?;

    let mut prev = String::new();
    let mut count = 0i64;
    for row in rows {
        let (seq, at, admin_email, action, tt, tid, reason, details_json, stored_prev, stored_hash) = row?;
        count += 1;
        let payload = Payload {
            at: &at,
            admin_email: &admin_email,
            action: &action,
            target_type: tt.as_deref(),
            target_id: tid.as_deref(),
            reason: reason.as_deref(),
            details_json: &details_json,
        };
        let expect = chain_hash(&prev, &payload);
        if stored_prev != prev || stored_hash != expect {
            return Ok(ChainStatus { ok: false, count, first_bad_seq: Some(seq) });
        }
        prev = stored_hash;
    }
    Ok(ChainStatus { ok: true, count, first_bad_seq: None })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn chain_appends_and_verifies() {
        let conn = db::open_memory().unwrap();
        append(&conn, "a@x", "revoke_licence", Some("licence"), Some("lic_1"), Some("fraud"), serde_json::json!({"k":1})).unwrap();
        append(&conn, "a@x", "add_note", Some("licence"), Some("lic_1"), Some("call"), serde_json::json!({})).unwrap();
        let s = verify_chain(&conn).unwrap();
        assert!(s.ok);
        assert_eq!(s.count, 2);
        assert!(s.first_bad_seq.is_none());
    }

    #[test]
    fn update_and_delete_are_blocked() {
        let conn = db::open_memory().unwrap();
        append(&conn, "a@x", "note", None, None, None, serde_json::json!({})).unwrap();
        assert!(conn.execute("UPDATE admin_audit SET reason='x' WHERE seq=1", []).is_err());
        assert!(conn.execute("DELETE FROM admin_audit WHERE seq=1", []).is_err());
    }
}
