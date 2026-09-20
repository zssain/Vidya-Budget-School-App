//! `receipt_cancellations` (append-only).

use rusqlite::{Connection, Transaction};

use crate::DbError;

/// Inserts a receipt cancellation.
pub fn insert(
    tx: &Transaction<'_>,
    receipt_id: &str,
    cancelled_by: &str,
    now: &str,
    reason: &str,
    hlc: &str,
) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO receipt_cancellations (receipt_id, cancelled_by, cancelled_at, reason, hlc)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![receipt_id, cancelled_by, now, reason, hlc],
    )?;
    Ok(())
}

/// Whether a receipt has been cancelled.
pub fn exists(conn: &Connection, receipt_id: &str) -> Result<bool, DbError> {
    let count: i64 = conn.query_row(
        "SELECT count(*) FROM receipt_cancellations WHERE receipt_id = ?1",
        [receipt_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}
