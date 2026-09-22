//! `grade_scale` (seeded by `seed_defaults`, edited in P3.6).

use rusqlite::{Connection, Transaction};

use crate::DbError;

#[derive(Debug, Clone)]
pub struct GradeBandRow {
    pub grade: String,
    pub min_percent: i64,
}

/// The grade scale, highest minimum first.
pub fn list(conn: &Connection) -> Result<Vec<GradeBandRow>, DbError> {
    let mut stmt = conn.prepare("SELECT grade, min_percent FROM grade_scale ORDER BY min_percent DESC")?;
    let rows = stmt.query_map([], |row| {
        Ok(GradeBandRow {
            grade: row.get(0)?,
            min_percent: row.get(1)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}

/// Replaces the whole grade scale with `bands` (`(grade, min_percent)`).
pub fn replace_all(tx: &Transaction<'_>, bands: &[(String, i64)], hlc: &str) -> Result<(), DbError> {
    tx.execute("DELETE FROM grade_scale", [])?;
    for (grade, min_percent) in bands {
        tx.execute(
            "INSERT INTO grade_scale (grade, min_percent, updated_hlc) VALUES (?1, ?2, ?3)",
            rusqlite::params![grade, min_percent, hlc],
        )?;
    }
    Ok(())
}
