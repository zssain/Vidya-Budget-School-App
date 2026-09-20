//! `app_settings` key/value store.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, Transaction};

use crate::DbError;

/// Reads one setting, or `None` if absent.
pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, DbError> {
    Ok(conn
        .query_row("SELECT value FROM app_settings WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()?)
}

/// Reads every setting as a map.
pub fn all(conn: &Connection) -> Result<BTreeMap<String, String>, DbError> {
    let mut stmt = conn.prepare("SELECT key, value FROM app_settings ORDER BY key")?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
    let mut map = BTreeMap::new();
    for row in rows {
        let (key, value) = row?;
        map.insert(key, value);
    }
    Ok(map)
}

/// Inserts or replaces one setting.
pub fn set(tx: &Transaction<'_>, key: &str, value: &str) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )?;
    Ok(())
}
