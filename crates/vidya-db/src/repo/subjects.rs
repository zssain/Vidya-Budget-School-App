//! `subjects` (per class).

use rusqlite::{Connection, Transaction};

use crate::DbError;

#[derive(Debug, Clone)]
pub struct SubjectRow {
    pub id: String,
    pub class_id: String,
    pub name: String,
    pub sort_order: i64,
    pub active: bool,
}

/// Inserts a subject.
pub fn insert(tx: &Transaction<'_>, row: &SubjectRow, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO subjects (id, class_id, name, sort_order, active, updated_hlc)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            row.id,
            row.class_id,
            row.name,
            row.sort_order,
            i64::from(row.active),
            hlc
        ],
    )?;
    Ok(())
}

/// Active subjects of a class ordered by `sort_order`.
pub fn list_active(conn: &Connection, class_id: &str) -> Result<Vec<SubjectRow>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, class_id, name, sort_order, active FROM subjects
         WHERE class_id = ?1 AND active = 1 ORDER BY sort_order",
    )?;
    let rows = stmt.query_map([class_id], |row| {
        Ok(SubjectRow {
            id: row.get(0)?,
            class_id: row.get(1)?,
            name: row.get(2)?,
            sort_order: row.get(3)?,
            active: row.get::<_, i64>(4)? != 0,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}

/// All subjects of a class, active or not, ordered by `sort_order`.
pub fn list_all(conn: &Connection, class_id: &str) -> Result<Vec<SubjectRow>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, class_id, name, sort_order, active FROM subjects WHERE class_id = ?1 ORDER BY sort_order",
    )?;
    let rows = stmt.query_map([class_id], |row| {
        Ok(SubjectRow {
            id: row.get(0)?,
            class_id: row.get(1)?,
            name: row.get(2)?,
            sort_order: row.get(3)?,
            active: row.get::<_, i64>(4)? != 0,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}

/// Renames/reorders/reactivates a subject.
pub fn update(
    tx: &Transaction<'_>,
    id: &str,
    name: &str,
    sort_order: i64,
    active: bool,
    hlc: &str,
) -> Result<(), DbError> {
    tx.execute(
        "UPDATE subjects SET name = ?2, sort_order = ?3, active = ?4, updated_hlc = ?5 WHERE id = ?1",
        rusqlite::params![id, name, sort_order, i64::from(active), hlc],
    )?;
    Ok(())
}
