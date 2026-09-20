//! `exams` (per session).

use rusqlite::{Connection, Transaction};

use crate::DbError;

#[derive(Debug, Clone)]
pub struct ExamRow {
    pub id: String,
    pub session_id: String,
    pub name: String,
    pub max_marks: i64,
    pub sort_order: i64,
    pub active: bool,
}

/// Inserts an exam.
pub fn insert(tx: &Transaction<'_>, row: &ExamRow, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO exams (id, session_id, name, max_marks, sort_order, active, updated_hlc)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            row.id,
            row.session_id,
            row.name,
            row.max_marks,
            row.sort_order,
            i64::from(row.active),
            hlc
        ],
    )?;
    Ok(())
}

/// Active exams of a session ordered by `sort_order`.
pub fn list_active(conn: &Connection, session_id: &str) -> Result<Vec<ExamRow>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, name, max_marks, sort_order, active FROM exams
         WHERE session_id = ?1 AND active = 1 ORDER BY sort_order",
    )?;
    let rows = stmt.query_map([session_id], |row| {
        Ok(ExamRow {
            id: row.get(0)?,
            session_id: row.get(1)?,
            name: row.get(2)?,
            max_marks: row.get(3)?,
            sort_order: row.get(4)?,
            active: row.get::<_, i64>(5)? != 0,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}
