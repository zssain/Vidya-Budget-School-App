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
