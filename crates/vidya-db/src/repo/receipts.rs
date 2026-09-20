//! `receipts` (append-only).

use rusqlite::{Connection, Transaction};

use crate::DbError;

#[derive(Debug, Clone)]
pub struct ReceiptRow {
    pub id: String,
    pub receipt_no: String,
    pub session_id: String,
    pub student_id: String,
    pub amount: i64,
    pub mode: String,
    pub reference: String,
    pub note: String,
    pub paid_on: String,
    pub created_by: String,
    pub device_code: String,
    pub balance_after: i64,
    pub hlc: String,
}

/// Inserts a receipt (append-only; never updated or deleted).
pub fn insert(tx: &Transaction<'_>, row: &ReceiptRow, now: &str) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO receipts
         (id, receipt_no, session_id, student_id, amount, mode, reference, note, paid_on,
          created_at, created_by, device_code, balance_after, hlc)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        rusqlite::params![
            row.id,
            row.receipt_no,
            row.session_id,
            row.student_id,
            row.amount,
            row.mode,
            row.reference,
            row.note,
            row.paid_on,
            now,
            row.created_by,
            row.device_code,
            row.balance_after,
            row.hlc
        ],
    )?;
    Ok(())
}

/// Sum of amounts paid by a student in a session, excluding cancelled receipts.
pub fn sum_paid(conn: &Connection, session_id: &str, student_id: &str) -> Result<i64, DbError> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(r.amount), 0) FROM receipts r
         WHERE r.session_id = ?1 AND r.student_id = ?2
           AND NOT EXISTS (SELECT 1 FROM receipt_cancellations c WHERE c.receipt_id = r.id)",
        rusqlite::params![session_id, student_id],
        |row| row.get(0),
    )?)
}

/// The number of receipts (used by tests).
pub fn count(conn: &Connection) -> Result<i64, DbError> {
    Ok(conn.query_row("SELECT count(*) FROM receipts", [], |row| row.get(0))?)
}
