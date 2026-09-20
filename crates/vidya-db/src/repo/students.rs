//! `students`.

use rusqlite::{Connection, OptionalExtension, Transaction};

use crate::DbError;

#[derive(Debug, Clone)]
pub struct StudentRow {
    pub id: String,
    pub adm_no: String,
    pub name: String,
    pub gender: String,
    pub dob: Option<String>,
    pub father: String,
    pub mother: String,
    pub mobile: String,
    pub category: String,
    pub locality: String,
    pub aadhaar_collected: bool,
    pub apaar_created: bool,
    pub admitted_on: String,
}

const COLS: &str = "id, adm_no, name, gender, dob, father, mother, mobile, category, locality, \
                    aadhaar_collected, apaar_created, admitted_on";

fn map(row: &rusqlite::Row<'_>) -> rusqlite::Result<StudentRow> {
    Ok(StudentRow {
        id: row.get(0)?,
        adm_no: row.get(1)?,
        name: row.get(2)?,
        gender: row.get(3)?,
        dob: row.get(4)?,
        father: row.get(5)?,
        mother: row.get(6)?,
        mobile: row.get(7)?,
        category: row.get(8)?,
        locality: row.get(9)?,
        aadhaar_collected: row.get::<_, i64>(10)? != 0,
        apaar_created: row.get::<_, i64>(11)? != 0,
        admitted_on: row.get(12)?,
    })
}

/// Inserts a student.
pub fn insert(tx: &Transaction<'_>, row: &StudentRow, now: &str, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO students
         (id, adm_no, name, gender, dob, father, mother, mobile, category, locality,
          aadhaar_collected, apaar_created, admitted_on, created_at, updated_at, updated_hlc)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?14,?15)",
        rusqlite::params![
            row.id,
            row.adm_no,
            row.name,
            row.gender,
            row.dob,
            row.father,
            row.mother,
            row.mobile,
            row.category,
            row.locality,
            i64::from(row.aadhaar_collected),
            i64::from(row.apaar_created),
            row.admitted_on,
            now,
            hlc
        ],
    )?;
    Ok(())
}

/// A student by id.
pub fn get(conn: &Connection, id: &str) -> Result<Option<StudentRow>, DbError> {
    Ok(conn
        .query_row(&format!("SELECT {COLS} FROM students WHERE id = ?1"), [id], map)
        .optional()?)
}

/// The number of students (used by tests and `app_status`).
pub fn count(conn: &Connection) -> Result<i64, DbError> {
    Ok(conn.query_row("SELECT count(*) FROM students", [], |row| row.get(0))?)
}
