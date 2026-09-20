//! `fee_plans` (per session and class).

use rusqlite::{Connection, Transaction};

use crate::DbError;

#[derive(Debug, Clone)]
pub struct FeePlanRow {
    pub session_id: String,
    pub class_id: String,
    pub tuition: i64,
    pub exam: i64,
    pub other: i64,
}

/// Inserts or replaces a fee plan for a session and class.
pub fn upsert(tx: &Transaction<'_>, row: &FeePlanRow, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO fee_plans (session_id, class_id, tuition, exam, other, updated_hlc)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(session_id, class_id) DO UPDATE SET
             tuition = excluded.tuition, exam = excluded.exam, other = excluded.other,
             updated_hlc = excluded.updated_hlc",
        rusqlite::params![
            row.session_id,
            row.class_id,
            row.tuition,
            row.exam,
            row.other,
            hlc
        ],
    )?;
    Ok(())
}

/// Every fee plan for a session.
pub fn list_for_session(conn: &Connection, session_id: &str) -> Result<Vec<FeePlanRow>, DbError> {
    let mut stmt = conn
        .prepare("SELECT session_id, class_id, tuition, exam, other FROM fee_plans WHERE session_id = ?1")?;
    let rows = stmt.query_map([session_id], |row| {
        Ok(FeePlanRow {
            session_id: row.get(0)?,
            class_id: row.get(1)?,
            tuition: row.get(2)?,
            exam: row.get(3)?,
            other: row.get(4)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}
