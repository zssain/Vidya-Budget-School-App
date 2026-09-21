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

/// Updates a student's editable profile fields.
#[allow(clippy::too_many_arguments)]
pub fn update(tx: &Transaction<'_>, row: &StudentRow, now: &str, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "UPDATE students SET name = ?2, gender = ?3, dob = ?4, father = ?5, mother = ?6, mobile = ?7,
         category = ?8, locality = ?9, aadhaar_collected = ?10, apaar_created = ?11,
         updated_at = ?12, updated_hlc = ?13 WHERE id = ?1",
        rusqlite::params![
            row.id,
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
            now,
            hlc
        ],
    )?;
    Ok(())
}

/// A student with its enrollment for a session (joined with class/section).
#[derive(Debug, Clone)]
pub struct EnrolledStudent {
    pub student: StudentRow,
    pub enrollment_id: String,
    pub class_id: String,
    pub class_name: String,
    pub sort_order: i64,
    pub section_id: String,
    pub section_name: String,
    pub roll: i64,
    pub status: String,
    pub rte: bool,
    pub transport: bool,
    pub concession: i64,
}

const ENROLLED_COLS: &str = "s.id, s.adm_no, s.name, s.gender, s.dob, s.father, s.mother, s.mobile, \
    s.category, s.locality, s.aadhaar_collected, s.apaar_created, s.admitted_on, \
    e.id, e.class_id, c.name, c.sort_order, e.section_id, sec.name, e.roll, e.status, e.rte, e.transport, e.concession";

fn map_enrolled(row: &rusqlite::Row<'_>) -> rusqlite::Result<EnrolledStudent> {
    Ok(EnrolledStudent {
        student: StudentRow {
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
        },
        enrollment_id: row.get(13)?,
        class_id: row.get(14)?,
        class_name: row.get(15)?,
        sort_order: row.get(16)?,
        section_id: row.get(17)?,
        section_name: row.get(18)?,
        roll: row.get(19)?,
        status: row.get(20)?,
        rte: row.get::<_, i64>(21)? != 0,
        transport: row.get::<_, i64>(22)? != 0,
        concession: row.get(23)?,
    })
}

/// Enrolled students for a session, ordered by class, section, roll. When
/// `status` is non-empty it filters to that enrollment status.
pub fn list_enrolled(
    conn: &Connection,
    session_id: &str,
    status: &str,
) -> Result<Vec<EnrolledStudent>, DbError> {
    let sql = format!(
        "SELECT {ENROLLED_COLS} FROM enrollments e
         JOIN students s ON s.id = e.student_id
         JOIN classes c ON c.id = e.class_id
         JOIN sections sec ON sec.id = e.section_id
         WHERE e.session_id = ?1 AND (?2 = '' OR e.status = ?2)
         ORDER BY c.sort_order, sec.name, e.roll"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params![session_id, status], map_enrolled)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}

/// One enrolled student by id for a session.
pub fn enrolled_by_id(
    conn: &Connection,
    session_id: &str,
    student_id: &str,
) -> Result<Option<EnrolledStudent>, DbError> {
    let sql = format!(
        "SELECT {ENROLLED_COLS} FROM enrollments e
         JOIN students s ON s.id = e.student_id
         JOIN classes c ON c.id = e.class_id
         JOIN sections sec ON sec.id = e.section_id
         WHERE e.session_id = ?1 AND e.student_id = ?2"
    );
    Ok(conn
        .query_row(&sql, rusqlite::params![session_id, student_id], map_enrolled)
        .optional()?)
}

/// An active enrolled student in the session with the same name and father
/// (case-insensitive) — the possible-duplicate check.
pub fn find_active_duplicate(
    conn: &Connection,
    session_id: &str,
    name: &str,
    father: &str,
) -> Result<Option<EnrolledStudent>, DbError> {
    let sql = format!(
        "SELECT {ENROLLED_COLS} FROM enrollments e
         JOIN students s ON s.id = e.student_id
         JOIN classes c ON c.id = e.class_id
         JOIN sections sec ON sec.id = e.section_id
         WHERE e.session_id = ?1 AND e.status = 'active'
           AND lower(s.name) = lower(?2) AND lower(s.father) = lower(?3) LIMIT 1"
    );
    Ok(conn
        .query_row(&sql, rusqlite::params![session_id, name, father], map_enrolled)
        .optional()?)
}
