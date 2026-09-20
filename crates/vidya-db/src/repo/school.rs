//! The single `school` row (id = 1).

use rusqlite::{Connection, OptionalExtension, Transaction};

use crate::DbError;

#[derive(Debug, Clone)]
pub struct SchoolRow {
    pub name: String,
    pub address: String,
    pub udise: String,
    pub board: String,
    pub phone: String,
}

/// Inserts or replaces the single school row.
pub fn upsert(tx: &Transaction<'_>, row: &SchoolRow, now: &str, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO school (id, name, address, udise, board, phone, updated_at, updated_hlc)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(id) DO UPDATE SET
             name = excluded.name, address = excluded.address, udise = excluded.udise,
             board = excluded.board, phone = excluded.phone,
             updated_at = excluded.updated_at, updated_hlc = excluded.updated_hlc",
        rusqlite::params![row.name, row.address, row.udise, row.board, row.phone, now, hlc],
    )?;
    Ok(())
}

/// Reads the school row, or `None` before setup.
pub fn get(conn: &Connection) -> Result<Option<SchoolRow>, DbError> {
    Ok(conn
        .query_row(
            "SELECT name, address, udise, board, phone FROM school WHERE id = 1",
            [],
            |row| {
                Ok(SchoolRow {
                    name: row.get(0)?,
                    address: row.get(1)?,
                    udise: row.get(2)?,
                    board: row.get(3)?,
                    phone: row.get(4)?,
                })
            },
        )
        .optional()?)
}
