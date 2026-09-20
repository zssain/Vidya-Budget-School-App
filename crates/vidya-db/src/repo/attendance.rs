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
