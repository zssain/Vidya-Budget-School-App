//! `academic_sessions`.

use rusqlite::{Connection, OptionalExtension, Transaction};

use crate::DbError;

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: String,
    pub name: String,
    pub starts_on: String,
    pub ends_on: String,
    pub is_current: bool,
    pub terms: i64,
    pub transport_fee_per_term: i64,
}

fn map(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionRow> {
    Ok(SessionRow {
        id: row.get(0)?,
        name: row.get(1)?,
        starts_on: row.get(2)?,
        ends_on: row.get(3)?,
        is_current: row.get::<_, i64>(4)? != 0,
        terms: row.get(5)?,
        transport_fee_per_term: row.get(6)?,
    })
}

const COLS: &str = "id, name, starts_on, ends_on, is_current, terms, transport_fee_per_term";

/// Inserts a session.
pub fn insert(tx: &Transaction<'_>, row: &SessionRow, now: &str, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO academic_sessions
         (id, name, starts_on, ends_on, is_current, terms, transport_fee_per_term, created_at, updated_hlc)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            row.id,
            row.name,
            row.starts_on,
            row.ends_on,
            i64::from(row.is_current),
            row.terms,
            row.transport_fee_per_term,
            now,
            hlc
        ],
    )?;
    Ok(())
}

/// The current session, or `None` if none is marked current.
pub fn current(conn: &Connection) -> Result<Option<SessionRow>, DbError> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLS} FROM academic_sessions WHERE is_current = 1"),
            [],
            map,
        )
        .optional()?)
}

/// A session by id.
pub fn get(conn: &Connection, id: &str) -> Result<Option<SessionRow>, DbError> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLS} FROM academic_sessions WHERE id = ?1"),
            [id],
            map,
        )
        .optional()?)
}
