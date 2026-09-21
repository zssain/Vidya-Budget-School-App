//! `attendance_days` and `attendance_marks`.

use rusqlite::{Connection, OptionalExtension, Transaction};

use crate::DbError;

#[derive(Debug, Clone)]
pub struct AttendanceDayRow {
    pub id: String,
    pub session_id: String,
    pub section_id: String,
    pub date: String,
    pub saved_by: String,
    pub saved_at: String,
}

/// Inserts an attendance day header.
pub fn insert_day(tx: &Transaction<'_>, row: &AttendanceDayRow, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO attendance_days (id, session_id, section_id, date, saved_by, saved_at, updated_hlc)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            row.id,
            row.session_id,
            row.section_id,
            row.date,
            row.saved_by,
            row.saved_at,
            hlc
        ],
    )?;
    Ok(())
}

/// Inserts one attendance mark (`P`/`A`/`L`).
pub fn insert_mark(
    tx: &Transaction<'_>,
    day_id: &str,
    student_id: &str,
    status: &str,
) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO attendance_marks (attendance_day_id, student_id, status) VALUES (?1, ?2, ?3)",
        rusqlite::params![day_id, student_id, status],
    )?;
    Ok(())
}

/// Updates an existing attendance day's saver and timestamp (a correction).
pub fn update_day(
    tx: &Transaction<'_>,
    id: &str,
    saved_by: &str,
    saved_at: &str,
    hlc: &str,
) -> Result<(), DbError> {
    tx.execute(
        "UPDATE attendance_days SET saved_by = ?2, saved_at = ?3, updated_hlc = ?4 WHERE id = ?1",
        rusqlite::params![id, saved_by, saved_at, hlc],
    )?;
    Ok(())
}

/// Deletes every mark for a day, so the day can be re-saved in full.
pub fn delete_marks(tx: &Transaction<'_>, day_id: &str) -> Result<(), DbError> {
    tx.execute(
        "DELETE FROM attendance_marks WHERE attendance_day_id = ?1",
        [day_id],
    )?;
    Ok(())
}

/// The saved marks for a day as `(student_id, status)` pairs.
pub fn marks_for_day(conn: &Connection, day_id: &str) -> Result<Vec<(String, String)>, DbError> {
    let mut stmt =
        conn.prepare("SELECT student_id, status FROM attendance_marks WHERE attendance_day_id = ?1")?;
    let rows = stmt.query_map([day_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}

/// Every mark in a month for a section as `(date, student_id, status)`.
/// `month` is a `YYYY-MM` prefix.
pub fn month_marks(
    conn: &Connection,
    section_id: &str,
    month: &str,
) -> Result<Vec<(String, String, String)>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT d.date, m.student_id, m.status
         FROM attendance_days d JOIN attendance_marks m ON m.attendance_day_id = d.id
         WHERE d.section_id = ?1 AND d.date LIKE ?2",
    )?;
    let like = format!("{month}%");
    let rows = stmt.query_map(rusqlite::params![section_id, like], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}

/// The attendance day for a section and date, if saved.
pub fn get_day(conn: &Connection, section_id: &str, date: &str) -> Result<Option<AttendanceDayRow>, DbError> {
    Ok(conn
        .query_row(
            "SELECT id, session_id, section_id, date, saved_by, saved_at FROM attendance_days
             WHERE section_id = ?1 AND date = ?2",
            rusqlite::params![section_id, date],
            |row| {
                Ok(AttendanceDayRow {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    section_id: row.get(2)?,
                    date: row.get(3)?,
                    saved_by: row.get(4)?,
                    saved_at: row.get(5)?,
                })
            },
        )
        .optional()?)
}
