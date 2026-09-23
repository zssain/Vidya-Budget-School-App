//! Typed access to the encrypted `app_kv` table (prompts/P03, migration 0002).
//! Local-only app + setup-wizard state. Values are JSON strings.

use crate::db::now_iso;
use rusqlite::{params, Connection, OptionalExtension};

/// Read a raw JSON string for `key`.
pub fn get_raw(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT value_json FROM app_kv WHERE key = ?1", params![key], |r| {
        r.get::<_, String>(0)
    })
    .optional()
}

/// Read and deserialize the value for `key` (None if absent or malformed).
pub fn get<T: serde::de::DeserializeOwned>(conn: &Connection, key: &str) -> rusqlite::Result<Option<T>> {
    Ok(get_raw(conn, key)?.and_then(|s| serde_json::from_str(&s).ok()))
}

/// Upsert a serializable value under `key`.
pub fn set<T: serde::Serialize>(conn: &Connection, key: &str, value: &T) -> rusqlite::Result<()> {
    let json = serde_json::to_string(value).map_err(|e| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(e))
    })?;
    conn.execute(
        "INSERT INTO app_kv(key, value_json, updated_at) VALUES (?1, ?2, ?3) \
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
        params![key, json, now_iso()],
    )?;
    Ok(())
}

/// Delete a key (no error if absent).
pub fn delete(conn: &Connection, key: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM app_kv WHERE key = ?1", params![key])?;
    Ok(())
}

/// True if `key` is present.
pub fn exists(conn: &Connection, key: &str) -> rusqlite::Result<bool> {
    Ok(conn
        .query_row("SELECT 1 FROM app_kv WHERE key = ?1", params![key], |_| Ok(()))
        .optional()?
        .is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn kv_roundtrip() {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        assert!(!exists(&c, "setup_step").unwrap());
        set(&c, "setup_step", &3i64).unwrap();
        assert_eq!(get::<i64>(&c, "setup_step").unwrap(), Some(3));
        set(&c, "setup_step", &5i64).unwrap(); // upsert
        assert_eq!(get::<i64>(&c, "setup_step").unwrap(), Some(5));
        assert!(exists(&c, "setup_step").unwrap());
        delete(&c, "setup_step").unwrap();
        assert_eq!(get::<i64>(&c, "setup_step").unwrap(), None);
    }
}
