//! Server-side audience-key store with versioning + rotation (§8.8, P06 Step 3).
//!
//! The school server owns one 32-byte key per audience (`admin`, `finance`,
//! `class:<class_id>`), each with a monotonically increasing **version**. Keys are
//! stored in the encrypted `app_kv` table (`audience_key:<audience>`), created on
//! demand and **persisted** so every device in an audience is sealed the same key
//! (the pre-P06 code minted a fresh key each call and never stored it — so two
//! devices could get different `finance` keys and never read each other's
//! bundles). Rotation (on an assignment change) mints a new version while keeping
//! the old ones, so bundles written under an old key stay readable to the server
//! and to devices that already hold that version (§8.8).

use crate::kv;
use crate::sync::protocol::AudienceKey;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rand::RngCore;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Stored shape for one audience: the current version + every version's raw key.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AudienceKeyRecord {
    current: i64,
    /// version (as string, for JSON object keys) → base64 32-byte key.
    versions: BTreeMap<String, String>,
}

fn kv_key(audience: &str) -> String {
    format!("audience_key:{audience}")
}

fn random_key_b64() -> String {
    let mut k = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut k);
    STANDARD.encode(k)
}

fn load(conn: &Connection, audience: &str) -> rusqlite::Result<Option<AudienceKeyRecord>> {
    kv::get::<AudienceKeyRecord>(conn, &kv_key(audience))
}

fn store(conn: &Connection, audience: &str, rec: &AudienceKeyRecord) -> rusqlite::Result<()> {
    kv::set(conn, &kv_key(audience), rec)
}

/// The current key for `audience`, creating version 1 (persisted) if none exists.
pub fn get_or_create(conn: &Connection, audience: &str) -> rusqlite::Result<AudienceKey> {
    if let Some(rec) = load(conn, audience)? {
        if let Some(k) = rec.versions.get(&rec.current.to_string()) {
            return Ok(AudienceKey {
                audience: audience.to_string(),
                key_b64: k.clone(),
                version: rec.current,
            });
        }
    }
    let mut rec = AudienceKeyRecord::default();
    let key_b64 = random_key_b64();
    rec.current = 1;
    rec.versions.insert("1".to_string(), key_b64.clone());
    store(conn, audience, &rec)?;
    Ok(AudienceKey { audience: audience.to_string(), key_b64, version: 1 })
}

/// Rotate `audience` to a fresh key at `current + 1`, keeping the old versions.
/// Returns the new current key. (Assignment change → rotate that class key.)
pub fn rotate(conn: &Connection, audience: &str) -> rusqlite::Result<AudienceKey> {
    let mut rec = load(conn, audience)?.unwrap_or_default();
    if rec.versions.is_empty() {
        // Never created — treat rotation as first creation at v1.
        return get_or_create(conn, audience);
    }
    let next = rec.current + 1;
    let key_b64 = random_key_b64();
    rec.current = next;
    rec.versions.insert(next.to_string(), key_b64.clone());
    store(conn, audience, &rec)?;
    Ok(AudienceKey { audience: audience.to_string(), key_b64, version: next })
}

/// Every stored version for `audience`, oldest first — the server holds them all
/// so it can open bundles written under any version (Step 6 import).
pub fn all_versions(conn: &Connection, audience: &str) -> rusqlite::Result<Vec<AudienceKey>> {
    let Some(rec) = load(conn, audience)? else {
        return Ok(vec![]);
    };
    let mut out: Vec<AudienceKey> = rec
        .versions
        .into_iter()
        .filter_map(|(v, k)| {
            v.parse::<i64>().ok().map(|version| AudienceKey {
                audience: audience.to_string(),
                key_b64: k,
                version,
            })
        })
        .collect();
    out.sort_by_key(|k| k.version);
    Ok(out)
}

/// The raw 32-byte key for a specific `(audience, version)`, if held.
pub fn key_bytes(
    conn: &Connection,
    audience: &str,
    version: i64,
) -> rusqlite::Result<Option<[u8; 32]>> {
    let Some(rec) = load(conn, audience)? else {
        return Ok(None);
    };
    Ok(rec.versions.get(&version.to_string()).and_then(|b64| decode_key(b64)))
}

/// Decode a base64 key into exactly 32 bytes (`None` if malformed).
pub fn decode_key(key_b64: &str) -> Option<[u8; 32]> {
    let raw = STANDARD.decode(key_b64.as_bytes()).ok()?;
    <[u8; 32]>::try_from(raw.as_slice()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    const DBKEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn fresh() -> Connection {
        let mut c = db::open_in_memory(DBKEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        c
    }

    #[test]
    fn get_or_create_is_stable_across_calls() {
        // The pre-P06 bug: two calls returned different keys. Must be identical now.
        let conn = fresh();
        let a = get_or_create(&conn, "finance").unwrap();
        let b = get_or_create(&conn, "finance").unwrap();
        assert_eq!(a.version, 1);
        assert_eq!(a.key_b64, b.key_b64, "same audience → same persisted key");
    }

    #[test]
    fn distinct_audiences_get_distinct_keys() {
        let conn = fresh();
        let fin = get_or_create(&conn, "finance").unwrap();
        let cls = get_or_create(&conn, "class:c1").unwrap();
        assert_ne!(fin.key_b64, cls.key_b64);
    }

    #[test]
    fn rotation_bumps_version_and_keeps_old_keys_readable() {
        let conn = fresh();
        let v1 = get_or_create(&conn, "class:c1").unwrap();
        let v2 = rotate(&conn, "class:c1").unwrap();
        assert_eq!(v2.version, 2);
        assert_ne!(v1.key_b64, v2.key_b64);
        // current is now v2
        assert_eq!(get_or_create(&conn, "class:c1").unwrap().version, 2);
        // both versions remain retrievable for import of old bundles
        assert_eq!(key_bytes(&conn, "class:c1", 1).unwrap(), decode_key(&v1.key_b64));
        assert_eq!(key_bytes(&conn, "class:c1", 2).unwrap(), decode_key(&v2.key_b64));
        let all = all_versions(&conn, "class:c1").unwrap();
        assert_eq!(all.iter().map(|k| k.version).collect::<Vec<_>>(), vec![1, 2]);
    }

    #[test]
    fn rotate_before_create_makes_v1() {
        let conn = fresh();
        let k = rotate(&conn, "admin").unwrap();
        assert_eq!(k.version, 1);
    }

    #[test]
    fn key_bytes_absent_for_unknown() {
        let conn = fresh();
        assert_eq!(key_bytes(&conn, "class:none", 1).unwrap(), None);
        assert!(all_versions(&conn, "class:none").unwrap().is_empty());
    }
}
