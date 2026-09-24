//! Backup scheduling + manual trigger (P08 Part B engine, wired in the P10
//! follow-up). Backups are encrypted with the school's BACKUP key
//! (`Argon2id(recovery key, salt)`, §9/§12), so restore on a fresh PC needs only
//! the recovery key. The key is derived once from the recovery key and cached in
//! `<data_dir>/backup-key` (same local trust level as the existing `db-key` file;
//! moves to the OS keychain with the Android hardening pass).
//!
//! The scheduler ticks every `TICK_SECS` and runs a backup only when the data has
//! CHANGED since the last successful backup (the audit-chain head is the change
//! signal) — so it never writes identical copies on every tick.

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

use crate::ctx::RtCtx;
use crate::error::{CmdError, CmdResult};
use crate::security::{audit, recovery};

use vidya_core::backup::{slugify, BackupOutcome};

/// How often the scheduler checks whether a fresh backup is due.
pub const TICK_SECS: u64 = 20;

const KEY_FILE: &str = "backup-key";

fn key_path(data_dir: &Path) -> PathBuf {
    data_dir.join(KEY_FILE)
}

/// The cached backup key (hex), if backups have been enabled on this PC.
pub fn read_cached_key(data_dir: &Path) -> Option<String> {
    std::fs::read_to_string(key_path(data_dir))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn cache_key(data_dir: &Path, hex: &str) -> CmdResult<()> {
    std::fs::write(key_path(data_dir), hex).map_err(|e| CmdError::internal(e.to_string()))
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

fn school_name_salt(conn: &Connection) -> CmdResult<(String, Vec<u8>)> {
    conn.query_row("SELECT name, backup_salt FROM school LIMIT 1", [], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))
    })
    .map_err(|_| CmdError::new("NO_SCHOOL", "error.NO_SCHOOL", serde_json::Value::Null))
}

/// Derive the backup key from the recovery key + school salt and cache it.
pub fn derive_and_cache(conn: &Connection, data_dir: &Path, recovery_key: &str) -> CmdResult<String> {
    let (_name, salt) = school_name_salt(conn)?;
    let norm = recovery::normalize(recovery_key);
    let key = recovery::derive_backup_key(&norm, &salt).map_err(CmdError::internal)?;
    let hex = to_hex(&key);
    cache_key(data_dir, &hex)?;
    Ok(hex)
}

/// Export + verify a backup and record the run. Returns nothing; the row in
/// `backup_run` is the record of truth (Principal Home + Backups read it).
pub fn run_and_record(conn: &mut Connection, data_dir: &Path, key_hex: &str) -> CmdResult<()> {
    let (name, _salt) = school_name_salt(conn)?;
    let slug = slugify(&name);
    let at = now_rfc3339();
    let today = at.get(0..10).unwrap_or("").to_string();
    let dir = data_dir.join("backups");

    let res = super::run_backup(conn, &slug, key_hex, &at, &dir, None)
        .map_err(|e| CmdError::internal(format!("backup: {e:?}")))?;

    let status = match res.outcome {
        BackupOutcome::Verified => "verified",
        BackupOutcome::Partial => "partial",
        BackupOutcome::Failed => "failed",
    };
    let destination = res.local_verified.then(|| "this PC".to_string());
    conn.execute(
        "INSERT INTO backup_run (id, started_at, finished_at, status, destination, checksum, chain_head) \
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6)",
        params![uuid::Uuid::now_v7().to_string(), at, now_rfc3339(), status, destination, res.chain_head],
    )?;
    // Prune to 30 daily + 12 monthly (§12).
    let _ = super::retain_local(&dir, &today);
    Ok(())
}

/// Manual "Back up now" / "Turn on backups". Resolves the key (cached, or derived
/// from `recovery_key` and cached), then runs one backup.
pub fn backup_now(ctx: &RtCtx, recovery_key: Option<&str>) -> CmdResult<()> {
    let data_dir = ctx.data_dir.clone();
    let cached = read_cached_key(&data_dir);
    ctx.with_db(|conn| {
        let key_hex = match cached {
            Some(k) => k,
            None => match recovery_key {
                Some(rk) if !rk.trim().is_empty() => derive_and_cache(conn, &data_dir, rk)?,
                _ => return Err(CmdError::new("RECOVERY_KEY_REQUIRED", "error.RECOVERY_KEY_REQUIRED", serde_json::Value::Null)),
            },
        };
        run_and_record(conn, &data_dir, &key_hex)
    })
}

/// One scheduler tick: back up only if enabled AND the data changed since the last
/// successful backup. Errors are logged, never propagated (a background task).
pub fn scheduler_tick(ctx: &RtCtx) {
    let data_dir = ctx.data_dir.clone();
    let Some(key_hex) = read_cached_key(&data_dir) else {
        return; // backups not enabled on this PC yet
    };
    let result = ctx.with_db(|conn| {
        let head = audit::head_hash(conn).unwrap_or_default();
        let last_head: Option<String> = conn
            .query_row(
                "SELECT chain_head FROM backup_run WHERE finished_at IS NOT NULL AND status != 'failed' \
                 ORDER BY started_at DESC LIMIT 1",
                [],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();
        if last_head.as_deref() == Some(head.as_str()) {
            return Ok(()); // no new audited changes → nothing to back up
        }
        run_and_record(conn, &data_dir, &key_hex)
    });
    if let Err(e) = result {
        tracing::warn!(error = %e.code, "scheduled backup failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn seeded_dir() -> (tempdir_like::Dir, Connection) {
        let dir = tempdir_like::Dir::new();
        let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let mut conn = db::open_encrypted(&dir.path().join("vidya.db"), key).unwrap();
        db::run_migrations(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO school(id,name,backup_salt,server_epoch,created_at,updated_at) \
             VALUES ('s','Test School', x'0011223344556677', 1, '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        (dir, conn)
    }

    #[test]
    fn run_and_record_writes_a_verified_local_backup() {
        let (dir, mut conn) = seeded_dir();
        let backup_key = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        run_and_record(&mut conn, dir.path(), backup_key).unwrap();

        // A backup file exists and a run was recorded (local-only → "partial").
        let files: Vec<_> = std::fs::read_dir(dir.path().join("backups")).unwrap().flatten().collect();
        assert_eq!(files.len(), 1, "one .vbak written");
        let (status, dest): (String, Option<String>) = conn
            .query_row("SELECT status, destination FROM backup_run LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!(status, "partial");
        assert_eq!(dest.as_deref(), Some("this PC"));
    }

    // Minimal scoped temp dir (no external crate).
    mod tempdir_like {
        use std::path::{Path, PathBuf};
        pub struct Dir(PathBuf);
        impl Dir {
            pub fn new() -> Self {
                let base = std::env::temp_dir().join(format!(
                    "vidya-bk-test-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
                ));
                std::fs::create_dir_all(&base).unwrap();
                Dir(base)
            }
            pub fn path(&self) -> &Path {
                &self.0
            }
        }
        impl Drop for Dir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }
}
