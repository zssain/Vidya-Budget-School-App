//! `receipts` (append-only).

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, Transaction};

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

/// The number of receipts (cancelled or not) in a session — used to decide
/// whether changing the number of terms needs confirmation.
pub fn count_for_session(conn: &Connection, session_id: &str) -> Result<i64, DbError> {
    Ok(conn.query_row(
        "SELECT count(*) FROM receipts WHERE session_id = ?1",
        [session_id],
        |row| row.get(0),
    )?)
}

/// A receipt summary for the student detail (with a cancelled flag).
#[derive(Debug, Clone)]
pub struct ReceiptSummary {
    pub id: String,
    pub receipt_no: String,
    pub paid_on: String,
    pub amount: i64,
    pub mode: String,
    pub cancelled: bool,
}

/// Receipts for a student in a session, newest first, each flagged cancelled.
pub fn list_for_student(
    conn: &Connection,
    session_id: &str,
    student_id: &str,
) -> Result<Vec<ReceiptSummary>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT r.id, r.receipt_no, r.paid_on, r.amount, r.mode,
                EXISTS (SELECT 1 FROM receipt_cancellations c WHERE c.receipt_id = r.id)
         FROM receipts r WHERE r.session_id = ?1 AND r.student_id = ?2 ORDER BY r.paid_on DESC, r.receipt_no DESC",
    )?;
    let rows = stmt.query_map(rusqlite::params![session_id, student_id], |row| {
        Ok(ReceiptSummary {
            id: row.get(0)?,
            receipt_no: row.get(1)?,
            paid_on: row.get(2)?,
            amount: row.get(3)?,
            mode: row.get(4)?,
            cancelled: row.get::<_, i64>(5)? != 0,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}

/// Whether an active (not cancelled) UPI receipt already carries this reference.
/// Used to reject a reused UPI transaction id while the first receipt stands.
pub fn upi_reference_active(conn: &Connection, reference: &str) -> Result<bool, DbError> {
    let count: i64 = conn.query_row(
        "SELECT count(*) FROM receipts r
         WHERE r.mode = 'UPI' AND r.reference = ?1
           AND NOT EXISTS (SELECT 1 FROM receipt_cancellations c WHERE c.receipt_id = r.id)",
        [reference],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Non-cancelled amount paid per student in a session, keyed by student id.
pub fn paid_by_student(conn: &Connection, session_id: &str) -> Result<HashMap<String, i64>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT r.student_id, COALESCE(SUM(r.amount), 0) FROM receipts r
         WHERE r.session_id = ?1
           AND NOT EXISTS (SELECT 1 FROM receipt_cancellations c WHERE c.receipt_id = r.id)
         GROUP BY r.student_id",
    )?;
    let rows = stmt.query_map([session_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    rows.collect::<rusqlite::Result<HashMap<_, _>>>()
        .map_err(DbError::from)
}

/// Total and count of non-cancelled receipts paid on a date.
pub fn collected_on(conn: &Connection, date: &str) -> Result<(i64, i64), DbError> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(r.amount), 0), COUNT(*) FROM receipts r
         WHERE r.paid_on = ?1
           AND NOT EXISTS (SELECT 1 FROM receipt_cancellations c WHERE c.receipt_id = r.id)",
        [date],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?)
}

/// A cancellation attached to a receipt.
#[derive(Debug, Clone)]
pub struct ReceiptCancellation {
    pub by_name: String,
    pub at: String,
    pub reason: String,
}

/// A receipt joined with the student, class-section, cashier and any
/// cancellation — everything a printed receipt or the day book needs.
#[derive(Debug, Clone)]
pub struct ReceiptDetail {
    pub id: String,
    pub receipt_no: String,
    pub student_id: String,
    pub student_name: String,
    pub adm_no: String,
    pub father: String,
    pub class_name: String,
    pub section_name: String,
    pub roll: i64,
    pub amount: i64,
    pub mode: String,
    pub reference: String,
    pub note: String,
    pub paid_on: String,
    pub created_at: String,
    pub received_by_name: String,
    pub device_code: String,
    pub balance_after: i64,
    pub cancelled: Option<ReceiptCancellation>,
}

const DETAIL_SELECT: &str = "SELECT r.id, r.receipt_no, r.student_id, s.name, s.adm_no, s.father,
        cl.name, sec.name, e.roll,
        r.amount, r.mode, r.reference, r.note, r.paid_on, r.created_at,
        u.name, r.device_code, r.balance_after,
        c.cancelled_at, cu.name, c.reason
 FROM receipts r
 JOIN students s ON s.id = r.student_id
 LEFT JOIN enrollments e ON e.student_id = r.student_id AND e.session_id = r.session_id
 LEFT JOIN classes cl ON cl.id = e.class_id
 LEFT JOIN sections sec ON sec.id = e.section_id
 JOIN users u ON u.id = r.created_by
 LEFT JOIN receipt_cancellations c ON c.receipt_id = r.id
 LEFT JOIN users cu ON cu.id = c.cancelled_by";

fn map_detail(row: &rusqlite::Row<'_>) -> rusqlite::Result<ReceiptDetail> {
    let cancelled = match row.get::<_, Option<String>>(18)? {
        Some(at) => Some(ReceiptCancellation {
            at,
            by_name: row.get::<_, Option<String>>(19)?.unwrap_or_default(),
            reason: row.get::<_, Option<String>>(20)?.unwrap_or_default(),
        }),
        None => None,
    };
    Ok(ReceiptDetail {
        id: row.get(0)?,
        receipt_no: row.get(1)?,
        student_id: row.get(2)?,
        student_name: row.get(3)?,
        adm_no: row.get(4)?,
        father: row.get(5)?,
        class_name: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
        section_name: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
        roll: row.get::<_, Option<i64>>(8)?.unwrap_or_default(),
        amount: row.get(9)?,
        mode: row.get(10)?,
        reference: row.get(11)?,
        note: row.get(12)?,
        paid_on: row.get(13)?,
        created_at: row.get(14)?,
        received_by_name: row.get(15)?,
        device_code: row.get(16)?,
        balance_after: row.get(17)?,
        cancelled,
    })
}

/// One receipt with everything needed to print or cancel it.
pub fn get_detail(conn: &Connection, receipt_id: &str) -> Result<Option<ReceiptDetail>, DbError> {
    Ok(conn
        .query_row(
            &format!("{DETAIL_SELECT} WHERE r.id = ?1"),
            [receipt_id],
            map_detail,
        )
        .optional()?)
}

/// All receipts paid on a date (cancelled included and flagged), oldest first.
pub fn list_details_for_date(conn: &Connection, date: &str) -> Result<Vec<ReceiptDetail>, DbError> {
    let mut stmt = conn.prepare(&format!(
        "{DETAIL_SELECT} WHERE r.paid_on = ?1 ORDER BY r.created_at ASC, r.receipt_no ASC"
    ))?;
    let rows = stmt.query_map([date], map_detail)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}
