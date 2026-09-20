//! `enrollments` (a student in a session/class/section).

use rusqlite::{Connection, OptionalExtension, Transaction};

use crate::DbError;

#[derive(Debug, Clone)]
pub struct EnrollmentRow {
    pub id: String,
    pub student_id: String,
    pub session_id: String,
    pub class_id: String,
    pub section_id: String,
    pub roll: i64,
    pub rte: bool,
    pub transport: bool,
    pub concession: i64,
    pub status: String,
}

const COLS: &str =
    "id, student_id, session_id, class_id, section_id, roll, rte, transport, concession, status";

fn map(row: &rusqlite::Row<'_>) -> rusqlite::Result<EnrollmentRow> {
    Ok(EnrollmentRow {
        id: row.get(0)?,
        student_id: row.get(1)?,
        session_id: row.get(2)?,
        class_id: row.get(3)?,
        section_id: row.get(4)?,
        roll: row.get(5)?,
        rte: row.get::<_, i64>(6)? != 0,
        transport: row.get::<_, i64>(7)? != 0,
        concession: row.get(8)?,
        status: row.get(9)?,
    })
}

/// Inserts an enrollment.
pub fn insert(tx: &Transaction<'_>, row: &EnrollmentRow, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO enrollments
         (id, student_id, session_id, class_id, section_id, roll, rte, transport, concession, status, updated_hlc)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        rusqlite::params![
            row.id,
            row.student_id,
            row.session_id,
            row.class_id,
            row.section_id,
            row.roll,
            i64::from(row.rte),
            i64::from(row.transport),
            row.concession,
            row.status,
            hlc
        ],
    )?;
    Ok(())
}

/// The enrollment of a student in a session.
pub fn get_for_student(
    conn: &Connection,
    student_id: &str,
    session_id: &str,
) -> Result<Option<EnrollmentRow>, DbError> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLS} FROM enrollments WHERE student_id = ?1 AND session_id = ?2"),
            rusqlite::params![student_id, session_id],
            map,
        )
        .optional()?)
}

/// Active enrollments of a section in a session, ordered by roll.
pub fn list_active_in_section(
    conn: &Connection,
    session_id: &str,
    section_id: &str,
) -> Result<Vec<EnrollmentRow>, DbError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM enrollments
         WHERE session_id = ?1 AND section_id = ?2 AND status = 'active' ORDER BY roll"
    ))?;
    let rows = stmt.query_map(rusqlite::params![session_id, section_id], map)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}
