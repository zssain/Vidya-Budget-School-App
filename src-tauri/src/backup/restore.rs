//! Restore (docs/00-SYSTEM-CONTEXT.md §12, prompts/P08 Part C).
//!
//! Welcome → "Recover an existing school": pick a `.vbak` (from a file or Drive) →
//! recovery key → **verify** → **summary** → licence transfer → **install as
//! server**. This module builds the summary and installs the backup:
//!
//! * [`build_summary`] opens the `.vbak` (read-only, with the backup key) and reads
//!   the school, counts, last receipt number and whether its audit chain verifies —
//!   feeding [`vidya_core::restore::RestoreSummary`] /
//!   [`vidya_core::restore::validate_restore`] /
//!   [`vidya_core::restore::staleness_warning`];
//! * [`install`] performs the **staged** install the prompt requires: re-encrypt
//!   the backup into a **temp** live DB under a new DB key, verify it, apply the
//!   fencing mutations (`server_epoch + 1`, every device `needs_rejoin`, an audit
//!   entry), then **swap atomically**, keeping the previous DB as a safety copy.
//!
//! The recovery-key entry backoff (3 wrong → 30 s) and the "warn if older than a
//! day" text are pure and live in [`vidya_core::restore`]. New TLS cert, relay
//! re-registration and the `/v1/transfer` call are the server/licence layers'
//! jobs, wired by the command; this module owns the database staging + swap.

use std::path::{Path, PathBuf};

use rusqlite::{params, OptionalExtension};

use super::{parse_backup_at, BackupError, BackupResult};
use crate::db;
use crate::security::audit;
use vidya_core::restore::RestoreSummary;

/// Read the pre-install summary from a `.vbak` (§12). Opens read-only with the
/// backup key; a wrong key / corrupt file → [`BackupError::Unreadable`]. The
/// backup date is taken from the filename stamp
/// (`vidya-<slug>-YYYYMMDD-HHMM.vbak`); an unrecognised name yields an empty date
/// (so the staleness check simply does not fire).
pub fn build_summary(vbak_path: &Path, backup_key_hex: &str) -> BackupResult<RestoreSummary> {
    let conn = db::open_encrypted_readonly(vbak_path, backup_key_hex)
        .map_err(|e| BackupError::Unreadable(e.to_string()))?;
    if !db::is_readable(&conn) {
        return Err(BackupError::Unreadable("wrong key or not a database".into()));
    }

    let school_name: String = conn
        .query_row("SELECT name FROM school LIMIT 1", [], |r| r.get(0))
        .optional()
        .map_err(|e| BackupError::Verify(e.to_string()))?
        .unwrap_or_default();
    let students: u64 = conn
        .query_row("SELECT COUNT(*) FROM student", [], |r| r.get::<_, i64>(0))
        .map_err(|e| BackupError::Verify(e.to_string()))? as u64;
    let payments: u64 = conn
        .query_row("SELECT COUNT(*) FROM payment", [], |r| r.get::<_, i64>(0))
        .map_err(|e| BackupError::Verify(e.to_string()))? as u64;
    let last_receipt_no: Option<String> = conn
        .query_row(
            "SELECT receipt_no FROM payment ORDER BY receipt_no DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| BackupError::Verify(e.to_string()))?;
    let chain_ok = audit::verify_chain(&conn)
        .map_err(|e| BackupError::Verify(e.to_string()))?
        .is_none();

    let backup_date = vbak_path
        .file_name()
        .and_then(|n| parse_backup_at(&n.to_string_lossy()))
        .map(|at| at[0..10].to_string())
        .unwrap_or_default();

    Ok(RestoreSummary { school_name, backup_date, students, payments, last_receipt_no, chain_ok })
}

/// What an install did.
#[derive(Debug, Clone)]
pub struct InstallReport {
    /// The new `server_epoch` (old + 1) — fences the previous PC (§8.9).
    pub new_epoch: i64,
    /// How many devices were marked `needs_rejoin`.
    pub devices_flagged: u64,
    /// The retired previous DB, kept as a safety copy (empty if there was none).
    pub safety_copy: Option<PathBuf>,
}

/// Install a `.vbak` as this PC's live database (§12), staged + atomic:
///
/// 1. re-encrypt the backup into a **temp** live DB under `new_db_key_hex`;
/// 2. verify the temp DB (readable + audit chain intact);
/// 3. apply fencing: `server_epoch + 1`, every device `needs_rejoin`, one audit
///    entry (`restore`, `at = now`);
/// 4. swap atomically: move any existing `live_db_path` aside as a safety copy,
///    then rename the temp DB into place.
///
/// The backup must first pass [`vidya_core::restore::validate_restore`] (the caller
/// checks the summary). Returns the new epoch + how many devices were flagged.
pub fn install(
    vbak_path: &Path,
    backup_key_hex: &str,
    live_db_path: &Path,
    new_db_key_hex: &str,
    now: &str,
) -> BackupResult<InstallReport> {
    // 1. Verify the backup (read-only), without ever modifying it.
    {
        let vbak = db::open_encrypted_readonly(vbak_path, backup_key_hex)
            .map_err(|e| BackupError::Unreadable(e.to_string()))?;
        if !db::is_readable(&vbak) {
            return Err(BackupError::Unreadable("wrong key or not a database".into()));
        }
        if audit::verify_chain(&vbak).map_err(|e| BackupError::Verify(e.to_string()))?.is_some() {
            return Err(BackupError::Verify("audit chain broken in backup".into()));
        }
    }

    // 2. Re-encrypt the backup into a fresh, temp live DB under the new DB key. The
    //    staged DB is `main`; the backup is ATTACHed as a source and copied in with
    //    `sqlcipher_export('main','bak')` — so the `.vbak` is only ever read.
    let staged = staged_path(live_db_path);
    remove_db_files(&staged);
    let new_epoch;
    let devices_flagged;
    {
        let mut conn = db::open_encrypted(&staged, new_db_key_hex)
            .map_err(|e| BackupError::Unreadable(e.to_string()))?;
        let vbak_str = vbak_path.to_string_lossy().to_string();
        conn.execute(
            &format!("ATTACH DATABASE ?1 AS bak KEY \"x'{backup_key_hex}'\""),
            params![vbak_str],
        )
        .map_err(|e| BackupError::Export(e.to_string()))?;
        if let Err(e) = conn.execute_batch("SELECT sqlcipher_export('main','bak'); DETACH DATABASE bak;") {
            let _ = conn.execute_batch("DETACH DATABASE bak;");
            drop(conn);
            remove_db_files(&staged);
            return Err(BackupError::Export(e.to_string()));
        }
        if !db::is_readable(&conn) {
            drop(conn);
            remove_db_files(&staged);
            return Err(BackupError::Unreadable("staged DB unreadable".into()));
        }

        // 3. Fencing mutations + audit entry, in one transaction.
        let tx = conn.transaction().map_err(|e| BackupError::Verify(e.to_string()))?;
        tx.execute("UPDATE school SET server_epoch = server_epoch + 1", [])
            .map_err(|e| BackupError::Verify(e.to_string()))?;
        new_epoch = tx
            .query_row("SELECT server_epoch FROM school LIMIT 1", [], |r| r.get::<_, i64>(0))
            .map_err(|e| BackupError::Verify(e.to_string()))?;
        devices_flagged = tx
            .execute("UPDATE device SET needs_rejoin = 1", [])
            .map_err(|e| BackupError::Verify(e.to_string()))? as u64;
        audit::append(
            &tx,
            &audit::AuditEntry {
                at: now.to_string(),
                action: "restore".into(),
                reason: Some(format!("restored on a new server, epoch {new_epoch}")),
                ..Default::default()
            },
        )
        .map_err(|e| BackupError::Verify(e.to_string()))?;
        tx.commit().map_err(|e| BackupError::Verify(e.to_string()))?;

        // Chain must still verify after the mutations.
        let chain_bad = audit::verify_chain(&conn)
            .map_err(|e| BackupError::Verify(e.to_string()))?
            .is_some();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").ok();
        drop(conn);
        if chain_bad {
            remove_db_files(&staged);
            return Err(BackupError::Verify("chain broke after fencing".into()));
        }
    }

    // 4. Atomic swap, keeping the previous DB as a safety copy. Drop the staged
    //    WAL/SHM sidecars first so only the finished main file is renamed in.
    remove_sidecars(&staged);
    let safety_copy = swap_into_place(&staged, live_db_path, now)?;

    Ok(InstallReport { new_epoch, devices_flagged, safety_copy })
}

/// Remove a DB file and its `-wal`/`-shm` sidecars (error cleanup / pre-clean).
fn remove_db_files(path: &Path) {
    let _ = std::fs::remove_file(path);
    remove_sidecars(path);
}

/// Remove just the `-wal`/`-shm` sidecars of a DB file.
fn remove_sidecars(path: &Path) {
    let _ = std::fs::remove_file(sibling(path, "-wal"));
    let _ = std::fs::remove_file(sibling(path, "-shm"));
}

/// Move any existing live DB aside (`<name>.pre-restore-<stamp>`), then rename the
/// staged DB into place. Best-effort moves the `-wal`/`-shm` sidecars too.
fn swap_into_place(staged: &Path, live: &Path, now: &str) -> BackupResult<Option<PathBuf>> {
    let mut safety_copy = None;
    if live.exists() {
        let stamp: String = now.chars().filter(|c| c.is_ascii_digit()).collect();
        let name = live.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let safety = live.with_file_name(format!("{name}.pre-restore-{stamp}"));
        std::fs::rename(live, &safety).map_err(|e| BackupError::Io(e.to_string()))?;
        for ext in ["-wal", "-shm"] {
            let side = sibling(live, ext);
            if side.exists() {
                let _ = std::fs::rename(&side, sibling(&safety, ext));
            }
        }
        safety_copy = Some(safety);
    }
    std::fs::rename(staged, live).map_err(|e| BackupError::Io(e.to_string()))?;
    Ok(safety_copy)
}

fn sibling(base: &Path, suffix: &str) -> PathBuf {
    let name = base.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    base.with_file_name(format!("{name}{suffix}"))
}

fn staged_path(live: &Path) -> PathBuf {
    let name = live.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    live.with_file_name(format!(".{name}.restore-staged"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vidya_core::restore::{staleness_warning, validate_restore};

    const DB_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const BK_KEY: &str = "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100";
    const NEW_KEY: &str = "1111111111111111111111111111111111111111111111111111111111111111";

    fn tmp_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let d = std::env::temp_dir().join(format!("vidya-restore-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A file-backed source school with 3 devices, students + a payment + a chain,
    /// exported to a `.vbak`. Returns the dir and the vbak path.
    fn make_vbak(dir: &Path) -> PathBuf {
        let src_path = dir.join("source.db");
        let mut c = db::open_encrypted(&src_path, DB_KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        let now = "2026-09-23T06:00:00Z";
        c.execute(
            "INSERT INTO school(id,name,backup_salt,server_epoch,created_at,updated_at) VALUES ('s','Saraswati Public School', x'00', 3, ?1, ?1)",
            params![now],
        ).unwrap();
        c.execute("PRAGMA foreign_keys=OFF;", []).unwrap();
        for i in 0..5 {
            c.execute(
                "INSERT INTO student(id,name,created_at,updated_at) VALUES (?1,?2,?3,?3)",
                params![format!("stu{i}"), format!("Student {i}"), now],
            ).unwrap();
        }
        c.execute(
            "INSERT INTO staff(id,name,role,created_at,updated_at) VALUES ('st1','P','principal',?1,?1)",
            params![now],
        ).unwrap();
        c.execute(
            "INSERT INTO device(id,staff_id,platform,name,token_hash,needs_rejoin) VALUES \
             ('d1','st1','macos','PC','h',0),('d2','st1','android','Phone','h2',0)",
            [],
        ).unwrap();
        c.execute(
            "INSERT INTO payment(id,receipt_no,student_id,amount_paise,mode,collected_by,collected_at,created_at,updated_at) \
             VALUES ('p1','R-A2-0418','stu0',150000,'cash','st1',?1,?1,?1)",
            params![now],
        ).unwrap();
        {
            let tx = c.transaction().unwrap();
            audit::append(&tx, &audit::AuditEntry { at: now.into(), action: "seed".into(), ..Default::default() }).unwrap();
            tx.commit().unwrap();
        }
        let vbak = dir.join("vidya-saraswati-public-school-20260922-0600.vbak");
        crate::backup::export_backup(&c, &vbak, BK_KEY).unwrap();
        vbak
    }

    #[test]
    fn summary_reads_backup_facts() {
        let dir = tmp_dir("summary");
        let vbak = make_vbak(&dir);
        let s = build_summary(&vbak, BK_KEY).unwrap();
        assert_eq!(s.school_name, "Saraswati Public School");
        assert_eq!(s.students, 5);
        assert_eq!(s.payments, 1);
        assert_eq!(s.last_receipt_no.as_deref(), Some("R-A2-0418"));
        assert!(s.chain_ok);
        assert_eq!(s.backup_date, "2026-09-22");
        // vidya-core gates: a good chain restores; a day-old backup warns.
        assert!(validate_restore(&s).is_ok());
        assert!(staleness_warning(&s.backup_date, "2026-09-23").is_some());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn summary_wrong_key_is_unreadable() {
        let dir = tmp_dir("wrongkey");
        let vbak = make_vbak(&dir);
        assert!(matches!(build_summary(&vbak, NEW_KEY), Err(BackupError::Unreadable(_))));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn install_stages_bumps_epoch_flags_devices_and_keeps_safety_copy() {
        let dir = tmp_dir("install");
        let vbak = make_vbak(&dir);
        let live = dir.join("vidya.db");
        // A pre-existing live DB must be retained as a safety copy.
        std::fs::write(&live, b"OLD-LIVE-DB").unwrap();

        let report = install(&vbak, BK_KEY, &live, NEW_KEY, "2026-09-23T10:00:00Z").unwrap();
        assert_eq!(report.new_epoch, 4, "epoch 3 → 4");
        assert_eq!(report.devices_flagged, 2);
        let safety = report.safety_copy.expect("previous DB kept as a safety copy");
        assert!(safety.exists());
        assert_eq!(std::fs::read(&safety).unwrap(), b"OLD-LIVE-DB");

        // The new live DB opens under the NEW key with the restored data + fencing.
        let conn = db::open_encrypted(&live, NEW_KEY).unwrap();
        assert!(db::is_readable(&conn));
        let students: i64 = conn.query_row("SELECT COUNT(*) FROM student", [], |r| r.get(0)).unwrap();
        assert_eq!(students, 5, "data preserved");
        let epoch: i64 = conn.query_row("SELECT server_epoch FROM school", [], |r| r.get(0)).unwrap();
        assert_eq!(epoch, 4);
        let pending: i64 = conn.query_row("SELECT COUNT(*) FROM device WHERE needs_rejoin=1", [], |r| r.get(0)).unwrap();
        assert_eq!(pending, 2, "every device needs to rejoin");
        // Chain still verifies (restore entry appended cleanly).
        assert_eq!(audit::verify_chain(&conn).unwrap(), None);
        let restore_entries: i64 = conn
            .query_row("SELECT COUNT(*) FROM audit_log WHERE action='restore'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(restore_entries, 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn install_first_time_has_no_safety_copy() {
        let dir = tmp_dir("firsttime");
        let vbak = make_vbak(&dir);
        let live = dir.join("vidya.db"); // no pre-existing DB
        let report = install(&vbak, BK_KEY, &live, NEW_KEY, "2026-09-23T10:00:00Z").unwrap();
        assert!(report.safety_copy.is_none());
        assert!(live.exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn no_staged_file_left_after_install() {
        let dir = tmp_dir("nostage");
        let vbak = make_vbak(&dir);
        let live = dir.join("vidya.db");
        install(&vbak, BK_KEY, &live, NEW_KEY, "2026-09-23T10:00:00Z").unwrap();
        let stray: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.contains("restore-staged"))
            .collect();
        assert!(stray.is_empty(), "no staged temp DB should remain: {stray:?}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
