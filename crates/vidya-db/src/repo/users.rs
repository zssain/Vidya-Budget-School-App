//! `users` and `user_sections`.

use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension, Transaction};

use crate::DbError;

#[derive(Debug, Clone)]
pub struct UserRow {
    pub id: String,
    pub username: String,
    pub name: String,
    pub role: String,
    pub mobile: String,
    pub password_hash: String,
    pub must_change: bool,
    pub failed_count: i64,
    pub locked: bool,
    pub locked_until: Option<String>,
    pub active: bool,
    pub language: String,
    pub created_at: String,
    pub last_login_at: Option<String>,
    pub password_changed_at: Option<String>,
}

const COLS: &str = "id, username, name, role, mobile, password_hash, must_change, failed_count, \
                    locked, locked_until, active, language, created_at, last_login_at, password_changed_at";

fn map(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserRow> {
    Ok(UserRow {
        id: row.get(0)?,
        username: row.get(1)?,
        name: row.get(2)?,
        role: row.get(3)?,
        mobile: row.get(4)?,
        password_hash: row.get(5)?,
        must_change: row.get::<_, i64>(6)? != 0,
        failed_count: row.get(7)?,
        locked: row.get::<_, i64>(8)? != 0,
        locked_until: row.get(9)?,
        active: row.get::<_, i64>(10)? != 0,
        language: row.get(11)?,
        created_at: row.get(12)?,
        last_login_at: row.get(13)?,
        password_changed_at: row.get(14)?,
    })
}

/// Inserts a user.
pub fn insert(tx: &Transaction<'_>, row: &UserRow, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO users
         (id, username, name, role, mobile, password_hash, must_change, failed_count, locked,
          locked_until, active, language, created_at, last_login_at, password_changed_at, updated_hlc)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
        rusqlite::params![
            row.id,
            row.username,
            row.name,
            row.role,
            row.mobile,
            row.password_hash,
            i64::from(row.must_change),
            row.failed_count,
            i64::from(row.locked),
            row.locked_until,
            i64::from(row.active),
            row.language,
            row.created_at,
            row.last_login_at,
            row.password_changed_at,
            hlc
        ],
    )?;
    Ok(())
}

/// A user by id.
pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<UserRow>, DbError> {
    Ok(conn
        .query_row(&format!("SELECT {COLS} FROM users WHERE id = ?1"), [id], map)
        .optional()?)
}

/// A user by (unique) username.
pub fn get_by_username(conn: &Connection, username: &str) -> Result<Option<UserRow>, DbError> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLS} FROM users WHERE username = ?1"),
            [username],
            map,
        )
        .optional()?)
}

/// Every user, ordered by name.
pub fn list(conn: &Connection) -> Result<Vec<UserRow>, DbError> {
    let mut stmt = conn.prepare(&format!("SELECT {COLS} FROM users ORDER BY name"))?;
    let rows = stmt.query_map([], map)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(DbError::from)
}

/// Replaces the section assignments of a user.
pub fn set_sections(tx: &Transaction<'_>, user_id: &str, section_ids: &[String]) -> Result<(), DbError> {
    tx.execute("DELETE FROM user_sections WHERE user_id = ?1", [user_id])?;
    for section_id in section_ids {
        tx.execute(
            "INSERT OR IGNORE INTO user_sections (user_id, section_id) VALUES (?1, ?2)",
            rusqlite::params![user_id, section_id],
        )?;
    }
    Ok(())
}

/// The section ids assigned to a user.
pub fn sections_for(conn: &Connection, user_id: &str) -> Result<BTreeSet<String>, DbError> {
    let mut stmt = conn.prepare("SELECT section_id FROM user_sections WHERE user_id = ?1")?;
    let rows = stmt.query_map([user_id], |row| row.get::<_, String>(0))?;
    let mut set = BTreeSet::new();
    for row in rows {
        set.insert(row?);
    }
    Ok(set)
}

/// Sets the failed-attempt count and lock state (used by sign-in lockout).
pub fn set_login_state(
    tx: &Transaction<'_>,
    user_id: &str,
    failed_count: i64,
    locked: bool,
    locked_until: Option<&str>,
    hlc: &str,
) -> Result<(), DbError> {
    tx.execute(
        "UPDATE users SET failed_count = ?2, locked = ?3, locked_until = ?4, updated_hlc = ?5 WHERE id = ?1",
        rusqlite::params![user_id, failed_count, i64::from(locked), locked_until, hlc],
    )?;
    Ok(())
}

/// Records a successful sign-in: clears failures and stamps `last_login_at`.
pub fn record_login(
    tx: &Transaction<'_>,
    user_id: &str,
    last_login_at: &str,
    hlc: &str,
) -> Result<(), DbError> {
    tx.execute(
        "UPDATE users SET failed_count = 0, locked_until = NULL, last_login_at = ?2, updated_hlc = ?3 WHERE id = ?1",
        rusqlite::params![user_id, last_login_at, hlc],
    )?;
    Ok(())
}

/// Sets a new password hash, clears `must_change`, and stamps the change time.
pub fn set_password(
    tx: &Transaction<'_>,
    user_id: &str,
    password_hash: &str,
    changed_at: &str,
    hlc: &str,
) -> Result<(), DbError> {
    tx.execute(
        "UPDATE users SET password_hash = ?2, must_change = 0, password_changed_at = ?3, updated_hlc = ?4 WHERE id = ?1",
        rusqlite::params![user_id, password_hash, changed_at, hlc],
    )?;
    Ok(())
}

/// Sets the user's interface language.
pub fn set_language(tx: &Transaction<'_>, user_id: &str, language: &str, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "UPDATE users SET language = ?2, updated_hlc = ?3 WHERE id = ?1",
        rusqlite::params![user_id, language, hlc],
    )?;
    Ok(())
}

/// Sets whether the user is active (used by tests and P3.1 user management).
pub fn set_active(tx: &Transaction<'_>, user_id: &str, active: bool, hlc: &str) -> Result<(), DbError> {
    tx.execute(
        "UPDATE users SET active = ?2, updated_hlc = ?3 WHERE id = ?1",
        rusqlite::params![user_id, i64::from(active), hlc],
    )?;
    Ok(())
}
