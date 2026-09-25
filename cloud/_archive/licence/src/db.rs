//! Database open + migrations + tiny shared helpers (ids, timestamps).

use rusqlite::{Connection, OptionalExtension};

/// Embedded, ordered migrations. New migrations are appended; existing SQL is
/// never edited (same rule as the app).
const MIGRATIONS: &[(&str, &str)] = &[("0001_init", include_str!("../migrations/0001_init.sql"))];

/// Open (creating if needed) the licence DB, set pragmas, and apply migrations.
pub fn open(path: &std::path::Path) -> rusqlite::Result<Connection> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    migrate(&conn)?;
    Ok(conn)
}

/// Open an in-memory DB (used by tests).
pub fn open_memory() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version TEXT PRIMARY KEY, applied_at TEXT NOT NULL)",
    )?;
    for (name, sql) in MIGRATIONS {
        let applied: Option<bool> = conn
            .query_row("SELECT 1 FROM schema_migrations WHERE version = ?1", [name], |_| Ok(true))
            .optional()?;
        if applied.is_none() {
            conn.execute_batch(sql)?;
            conn.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                rusqlite::params![name, now_iso()],
            )?;
        }
    }
    Ok(())
}

/// A fresh UUIDv7 (time-ordered) id.
pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// Current UTC time as an RFC-3339 string (matches the app's `created_at` format).
pub fn now_iso() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}
