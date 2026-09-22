//! `classes` and `sections`.

use rusqlite::{Connection, Transaction};

use crate::DbError;

#[derive(Debug, Clone)]
pub struct ClassRow {
    pub id: String,
    pub name: String,
    pub sort_order: i64,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct SectionRow {
    pub id: String,
    pub class_id: String,
    pub name: String,
    pub active: bool,
}

/// Inserts a class.
pub fn insert_class(tx: &Transaction<'_>, row: &ClassRow, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO classes (id, name, sort_order, active, updated_hlc) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![row.id, row.name, row.sort_order, i64::from(row.active), hlc],
    )?;
    Ok(())
}

/// Inserts a section.
pub fn insert_section(tx: &Transaction<'_>, row: &SectionRow, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO sections (id, class_id, name, active, updated_hlc) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![row.id, row.class_id, row.name, i64::from(row.active), hlc],
    )?;
    Ok(())
}

/// Active classes ordered by `sort_order`.
pub fn list_active_classes(conn: &Connection) -> Result<Vec<ClassRow>, DbError> {
    let mut stmt = conn
        .prepare("SELECT id, name, sort_order, active FROM classes WHERE active = 1 ORDER BY sort_order")?;
    let rows = stmt.query_map([], |row| {
        Ok(ClassRow {
            id: row.get(0)?,
            name: row.get(1)?,
            sort_order: row.get(2)?,
            active: row.get::<_, i64>(3)? != 0,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}

/// The display label of a section (`"V-A"`), or `None` if it does not exist.
pub fn section_label(conn: &Connection, section_id: &str) -> Result<Option<String>, DbError> {
    use rusqlite::OptionalExtension;
    Ok(conn
        .query_row(
            "SELECT c.name || '-' || s.name FROM sections s JOIN classes c ON c.id = s.class_id WHERE s.id = ?1",
            [section_id],
            |row| row.get(0),
        )
        .optional()?)
}

/// The class id of an active section, or `None` if it does not exist or is inactive.
pub fn active_section_class(conn: &Connection, section_id: &str) -> Result<Option<String>, DbError> {
    use rusqlite::OptionalExtension;
    Ok(conn
        .query_row(
            "SELECT class_id FROM sections WHERE id = ?1 AND active = 1",
            [section_id],
            |row| row.get(0),
        )
        .optional()?)
}

/// Whether a section exists and is active.
pub fn section_is_active(conn: &Connection, section_id: &str) -> Result<bool, DbError> {
    let count: i64 = conn.query_row(
        "SELECT count(*) FROM sections WHERE id = ?1 AND active = 1",
        [section_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Renames a class and sets its sort order.
pub fn update_class(tx: &Transaction<'_>, id: &str, name: &str, sort_order: i64, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "UPDATE classes SET name = ?2, sort_order = ?3, updated_hlc = ?4 WHERE id = ?1",
        rusqlite::params![id, name, sort_order, hlc],
    )?;
    Ok(())
}

/// Activates or deactivates a class (never deleted, so history keeps its id).
pub fn set_class_active(tx: &Transaction<'_>, id: &str, active: bool, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "UPDATE classes SET active = ?2, updated_hlc = ?3 WHERE id = ?1",
        rusqlite::params![id, i64::from(active), hlc],
    )?;
    Ok(())
}

/// Activates or deactivates a section.
pub fn set_section_active(tx: &Transaction<'_>, id: &str, active: bool, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "UPDATE sections SET active = ?2, updated_hlc = ?3 WHERE id = ?1",
        rusqlite::params![id, i64::from(active), hlc],
    )?;
    Ok(())
}

/// All sections of a class, active or not, ordered by name.
pub fn list_all_sections(conn: &Connection, class_id: &str) -> Result<Vec<SectionRow>, DbError> {
    let mut stmt =
        conn.prepare("SELECT id, class_id, name, active FROM sections WHERE class_id = ?1 ORDER BY name")?;
    let rows = stmt.query_map([class_id], |row| {
        Ok(SectionRow {
            id: row.get(0)?,
            class_id: row.get(1)?,
            name: row.get(2)?,
            active: row.get::<_, i64>(3)? != 0,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}

/// All classes, active or not, ordered by sort order.
pub fn list_all_classes(conn: &Connection) -> Result<Vec<ClassRow>, DbError> {
    let mut stmt = conn.prepare("SELECT id, name, sort_order, active FROM classes ORDER BY sort_order")?;
    let rows = stmt.query_map([], |row| {
        Ok(ClassRow {
            id: row.get(0)?,
            name: row.get(1)?,
            sort_order: row.get(2)?,
            active: row.get::<_, i64>(3)? != 0,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}

/// The number of active enrollments in a section in the current session.
pub fn active_enrollments_in_section(conn: &Connection, section_id: &str) -> Result<i64, DbError> {
    Ok(conn.query_row(
        "SELECT count(*) FROM enrollments WHERE section_id = ?1 AND status = 'active'",
        [section_id],
        |row| row.get(0),
    )?)
}

/// Names of active users assigned to a section (teachers), for safe removal.
pub fn teacher_names_for_section(conn: &Connection, section_id: &str) -> Result<Vec<String>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT u.name FROM user_sections us JOIN users u ON u.id = us.user_id
         WHERE us.section_id = ?1 AND u.active = 1 ORDER BY u.name",
    )?;
    let rows = stmt.query_map([section_id], |row| row.get::<_, String>(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}

/// Active sections of a class ordered by name.
pub fn list_active_sections(conn: &Connection, class_id: &str) -> Result<Vec<SectionRow>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, class_id, name, active FROM sections WHERE class_id = ?1 AND active = 1 ORDER BY name",
    )?;
    let rows = stmt.query_map([class_id], |row| {
        Ok(SectionRow {
            id: row.get(0)?,
            class_id: row.get(1)?,
            name: row.get(2)?,
            active: row.get::<_, i64>(3)? != 0,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}
