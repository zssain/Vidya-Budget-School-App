//! `marks` (one row per exam/student/subject; blank = no row).

use rusqlite::{Connection, Transaction};

use crate::DbError;

#[derive(Debug, Clone)]
pub struct MarkRow {
    pub id: String,
    pub exam_id: String,
    pub student_id: String,
    pub subject_id: String,
    /// `None` with `absent = true`, otherwise the score.
    pub value: Option<i64>,
    pub absent: bool,
    pub entered_by: String,
    pub entered_at: String,
}

/// Inserts or replaces a mark for an exam/student/subject.
pub fn upsert(tx: &Transaction<'_>, row: &MarkRow, now: &str, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO marks (id, exam_id, student_id, subject_id, value, absent, entered_by, entered_at, updated_hlc)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(exam_id, student_id, subject_id) DO UPDATE SET
             value = excluded.value, absent = excluded.absent, entered_by = excluded.entered_by,
             entered_at = excluded.entered_at, updated_hlc = excluded.updated_hlc",
        rusqlite::params![
            row.id,
            row.exam_id,
            row.student_id,
            row.subject_id,
            row.value,
            i64::from(row.absent),
            row.entered_by,
            now,
            hlc
        ],
    )?;
    Ok(())
}

/// All marks for an exam.
pub fn list_for_exam(conn: &Connection, exam_id: &str) -> Result<Vec<MarkRow>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, exam_id, student_id, subject_id, value, absent, entered_by, entered_at
         FROM marks WHERE exam_id = ?1",
    )?;
    let rows = stmt.query_map([exam_id], |row| {
        Ok(MarkRow {
            id: row.get(0)?,
            exam_id: row.get(1)?,
            student_id: row.get(2)?,
            subject_id: row.get(3)?,
            value: row.get(4)?,
            absent: row.get::<_, i64>(5)? != 0,
            entered_by: row.get(6)?,
            entered_at: row.get(7)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}

/// `(student_name, subject_name)` for marks above `max` in an exam — used to
/// block lowering an exam's maximum below marks already entered.
pub fn over_max(conn: &Connection, exam_id: &str, max: i64) -> Result<Vec<(String, String)>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT s.name, sub.name FROM marks m
         JOIN students s ON s.id = m.student_id
         JOIN subjects sub ON sub.id = m.subject_id
         WHERE m.exam_id = ?1 AND m.absent = 0 AND m.value > ?2 ORDER BY s.name",
    )?;
    let rows = stmt.query_map(rusqlite::params![exam_id, max], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}

/// Deletes a single mark (used when a cell is cleared).
pub fn delete(
    tx: &Transaction<'_>,
    exam_id: &str,
    student_id: &str,
    subject_id: &str,
) -> Result<(), DbError> {
    tx.execute(
        "DELETE FROM marks WHERE exam_id = ?1 AND student_id = ?2 AND subject_id = ?3",
        rusqlite::params![exam_id, student_id, subject_id],
    )?;
    Ok(())
}
