//! `alerts` — background notices for the office (overpayment, backup overdue…).

use rusqlite::{Connection, Transaction};

use crate::DbError;

#[derive(Debug, Clone)]
pub struct AlertRow {
    pub id: String,
    pub kind: String,
    pub entity: String,
    pub entity_id: String,
    pub message_key: String,
    pub params_json: String,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

fn map(row: &rusqlite::Row<'_>) -> rusqlite::Result<AlertRow> {
    Ok(AlertRow {
        id: row.get(0)?,
        kind: row.get(1)?,
        entity: row.get(2)?,
        entity_id: row.get(3)?,
        message_key: row.get(4)?,
        params_json: row.get(5)?,
        created_at: row.get(6)?,
        resolved_at: row.get(7)?,
    })
}

const COLS: &str = "id, kind, entity, entity_id, message_key, params_json, created_at, resolved_at";

/// Inserts an alert.
pub fn insert(tx: &Transaction<'_>, row: &AlertRow) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO alerts (id, kind, entity, entity_id, message_key, params_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            row.id,
            row.kind,
            row.entity,
            row.entity_id,
            row.message_key,
            row.params_json,
            row.created_at
        ],
    )?;
    Ok(())
}

/// Whether an unresolved alert of a kind already exists for an entity (so we
/// don't raise the same overpayment notice twice).
pub fn unresolved_exists(
    conn: &Connection,
    kind: &str,
    entity: &str,
    entity_id: &str,
) -> Result<bool, DbError> {
    let count: i64 = conn.query_row(
        "SELECT count(*) FROM alerts
         WHERE kind = ?1 AND entity = ?2 AND entity_id = ?3 AND resolved_at IS NULL",
        rusqlite::params![kind, entity, entity_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Unresolved alerts, newest first. When `kinds` is non-empty, only those kinds
/// are returned (accountants see overpayment alerts only).
pub fn list_unresolved(conn: &Connection, kinds: &[&str]) -> Result<Vec<AlertRow>, DbError> {
    if kinds.is_empty() {
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLS} FROM alerts WHERE resolved_at IS NULL ORDER BY created_at DESC"
        ))?;
        let rows = stmt.query_map([], map)?;
        return rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from);
    }
    let placeholders = kinds.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM alerts WHERE resolved_at IS NULL AND kind IN ({placeholders}) \
         ORDER BY created_at DESC"
    ))?;
    let rows = stmt.query_map(rusqlite::params_from_iter(kinds.iter()), map)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}

/// Marks an alert resolved. Returns the number of rows changed.
pub fn resolve(tx: &Transaction<'_>, alert_id: &str, user_id: &str, now: &str) -> Result<usize, DbError> {
    Ok(tx.execute(
        "UPDATE alerts SET resolved_at = ?2, resolved_by = ?3 WHERE id = ?1 AND resolved_at IS NULL",
        rusqlite::params![alert_id, now, user_id],
    )?)
}
