//! The school's two Google Drive connections (prompts/P12 Step 1, docs §11):
//!
//! * `sync`   — the shared school **sync account** every staff device signs into
//!   (exchange bundles, acks, `epoch.json`, class notes).
//! * `backup` — the Principal's **private backup account** (encrypted daily
//!   backups), used only from the school PC.
//!
//! v1 installs had ONE Principal Drive connection used for both roles; it is
//! migrated to `kind='backup'` so backups keep working, and Home shows "Connect
//! the school sync account" ([`needs_sync_account`]) until a sync account is added.
//! `token_enc` holds the OAuth refresh token (the DB is SQLCipher-encrypted at rest).

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

/// A connected Drive account (secrets like `token_enc` are never surfaced).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DriveAccount {
    /// `"sync"` | `"backup"`.
    pub kind: String,
    pub email: Option<String>,
    pub connected_at: Option<String>,
    /// `"connected"` | `"disconnected"` | `"needs_reconnect"`.
    pub status: String,
}

/// Read one account by kind, if present.
pub fn get(conn: &Connection, kind: &str) -> rusqlite::Result<Option<DriveAccount>> {
    conn.query_row(
        "SELECT kind, email, connected_at, status FROM drive_account WHERE kind = ?1",
        params![kind],
        |r| {
            Ok(DriveAccount {
                kind: r.get(0)?,
                email: r.get(1)?,
                connected_at: r.get(2)?,
                status: r.get(3)?,
            })
        },
    )
    .optional()
}

/// `true` if an account of this kind is present and currently `connected`.
pub fn is_connected(conn: &Connection, kind: &str) -> rusqlite::Result<bool> {
    Ok(get(conn, kind)?.is_some_and(|a| a.status == "connected"))
}

/// Home "Needs attention" trigger (Step 1.2): no connected school **sync** account.
pub fn needs_sync_account(conn: &Connection) -> rusqlite::Result<bool> {
    Ok(!is_connected(conn, "sync")?)
}

/// Connect / reconnect an account (Settings → Google Drive, Step 2). Upserts the
/// row and marks it `connected`. Folder ids are set once the server has ensured the
/// `Vidya/<school>/…` tree; pass `None` to leave a column unchanged is NOT
/// supported here — callers pass the full set they know (empty string ok).
#[allow(clippy::too_many_arguments)]
pub fn connect(
    conn: &Connection,
    kind: &str,
    email: &str,
    token_enc: &[u8],
    root_folder_id: Option<&str>,
    exchange_folder_id: Option<&str>,
    notes_folder_id: Option<&str>,
    backups_folder_id: Option<&str>,
    now: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO drive_account
           (kind, email, token_enc, root_folder_id, exchange_folder_id, notes_folder_id, backups_folder_id, connected_at, status)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'connected')
         ON CONFLICT(kind) DO UPDATE SET
           email=excluded.email, token_enc=excluded.token_enc,
           root_folder_id=excluded.root_folder_id, exchange_folder_id=excluded.exchange_folder_id,
           notes_folder_id=excluded.notes_folder_id, backups_folder_id=excluded.backups_folder_id,
           connected_at=excluded.connected_at, status='connected'",
        params![kind, email, token_enc, root_folder_id, exchange_folder_id, notes_folder_id, backups_folder_id, now],
    )?;
    Ok(())
}

/// Disconnect an account (Settings → Google Drive). Keeps the row for its email/
/// history but clears the refresh token and marks it disconnected.
pub fn disconnect(conn: &Connection, kind: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE drive_account SET status='disconnected', token_enc=NULL WHERE kind=?1",
        params![kind],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn fresh() -> Connection {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        c
    }

    #[test]
    fn fresh_install_needs_sync_account() {
        let c = fresh();
        assert!(needs_sync_account(&c).unwrap());
        assert!(!is_connected(&c, "sync").unwrap());
        assert!(!is_connected(&c, "backup").unwrap());
    }

    #[test]
    fn connect_then_read_and_disconnect() {
        let c = fresh();
        connect(&c, "sync", "vidya.school@gmail.com", b"tok", Some("root"), Some("exch"), Some("notes"), None, "2026-09-25T00:00:00Z").unwrap();
        assert!(is_connected(&c, "sync").unwrap());
        assert!(!needs_sync_account(&c).unwrap());
        let a = get(&c, "sync").unwrap().unwrap();
        assert_eq!(a.email.as_deref(), Some("vidya.school@gmail.com"));
        assert_eq!(a.status, "connected");

        // Reconnect updates in place (still one row per kind).
        connect(&c, "sync", "vidya2@gmail.com", b"tok2", Some("root"), Some("exch"), Some("notes"), None, "2026-09-26T00:00:00Z").unwrap();
        assert_eq!(get(&c, "sync").unwrap().unwrap().email.as_deref(), Some("vidya2@gmail.com"));

        disconnect(&c, "sync").unwrap();
        assert!(!is_connected(&c, "sync").unwrap());
        assert!(needs_sync_account(&c).unwrap());
    }

    #[test]
    fn kind_check_constraint_rejects_bad_kind() {
        let c = fresh();
        let e = c.execute(
            "INSERT INTO drive_account(kind,status) VALUES ('bogus','connected')",
            [],
        );
        assert!(e.is_err(), "CHECK(kind IN ('sync','backup')) must reject other kinds");
    }

    #[test]
    fn sync_and_backup_are_independent_rows() {
        let c = fresh();
        connect(&c, "backup", "principal@gmail.com", b"t", None, None, None, Some("bk"), "2026-09-25T00:00:00Z").unwrap();
        // Backup connected, but sync still missing → Home still needs the sync account.
        assert!(is_connected(&c, "backup").unwrap());
        assert!(needs_sync_account(&c).unwrap());
    }
}
