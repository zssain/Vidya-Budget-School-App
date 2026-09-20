//! `grade_scale` (seeded by `seed_defaults`, edited in P3.6).

use rusqlite::Connection;

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
