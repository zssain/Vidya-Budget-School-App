//! `meta` key/value store and the human-number `counters`.

use rusqlite::{Connection, OptionalExtension, Transaction};

use crate::DbError;

/// Reads a `meta` value, or `None` if the key is absent.
pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, DbError> {
    Ok(conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| row.get(0))
        .optional()?)
}

/// Inserts or replaces a `meta` value.
pub fn set(tx: &Transaction<'_>, key: &str, value: &str) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

/// Atomically increments a counter (creating it at 0 first) and returns the new
/// value. Human numbers (`adm_no`, `tc_no`, `receipt:<prefix>`, `tc_no`) come
/// from here. Uses SQLite `RETURNING` (bundled SQLite 3.35+ supports it).
pub fn next_counter(tx: &Transaction<'_>, name: &str) -> Result<i64, DbError> {
    tx.execute(
        "INSERT OR IGNORE INTO counters (name, value) VALUES (?1, 0)",
        [name],
    )?;
    let value = tx.query_row(
        "UPDATE counters SET value = value + 1 WHERE name = ?1 RETURNING value",
        [name],
        |row| row.get(0),
    )?;
    Ok(value)
}
