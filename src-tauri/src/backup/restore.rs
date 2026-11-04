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

use super::{parse_backup_at, read_meta_sidecar, BackupError, BackupResult};
use crate::db;
use crate::security::{audit, recovery};
use vidya_core::restore::RestoreSummary;

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

/// Derive the backup-key hex from the recovery key + the `.vbak.meta` salt beside the
/// backup (§12). The key is never stored; the salt is the non-secret KDF salt. Errors
/// if the sidecar is missing or corrupt — the caller surfaces "needs the .meta file".
pub fn backup_key_from_meta(vbak: &Path, recovery_key: &str) -> BackupResult<String> {
    let meta = read_meta_sidecar(vbak)
        .ok_or_else(|| BackupError::Unreadable("backup metadata (.meta) missing".into()))?;
    let salt = hex_decode(&meta.salt)
        .ok_or_else(|| BackupError::Unreadable("backup metadata salt invalid".into()))?;
    let norm = recovery::normalize(recovery_key);
    let key = recovery::derive_backup_key(&norm, &salt).map_err(BackupError::Verify)?;
    Ok(hex_encode(&key))
}

/// Read the pre-install summary using the recovery key (derives the backup key from
/// the `.meta` salt first). Returns the summary + the derived backup-key hex so the
/// caller can pass it straight to [`install`] without re-deriving.
pub fn summary_with_recovery(vbak: &Path, recovery_key: &str) -> BackupResult<(RestoreSummary, String)> {
    let key_hex = backup_key_from_meta(vbak, recovery_key)?;
    let summary = build_summary(vbak, &key_hex)?;
    Ok((summary, key_hex))
}

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
            // The most recently COLLECTED receipt: with several device series
            // (R-A1/R-A2/R-A3…), the text-max receipt_no is just the highest series,
            // not the last receipt the school issued. Confirmed receipts first.
            "SELECT receipt_no FROM payment \
             ORDER BY (sync_state = 'confirmed') DESC, collected_at DESC, receipt_no DESC LIMIT 1",
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
    install_then(vbak_path, backup_key_hex, live_db_path, new_db_key_hex, now, || Ok(()))
}

/// [`install`], plus a `commit` step run right after the swap — the caller persists
/// the new DB key there (keychain / AndroidKeyStore). If `commit` fails, the swap is
/// **rolled back** (the restored DB is discarded and the previous live DB is moved
/// back into place), so a key-store failure can never leave a live DB whose key was
/// lost. Returns [`BackupError::Io`] carrying the commit error.
pub fn install_then(
    vbak_path: &Path,
    backup_key_hex: &str,
    live_db_path: &Path,
    new_db_key_hex: &str,
    now: &str,
    commit: impl FnOnce() -> Result<(), String>,
) -> BackupResult<InstallReport> {
    let report = stage_and_swap(vbak_path, backup_key_hex, live_db_path, new_db_key_hex, now, &|a, b| {
        std::fs::rename(a, b)
    })?;
    if let Err(e) = commit() {
        rollback_swap(live_db_path, report.safety_copy.as_deref());
        return Err(BackupError::Io(format!("restore not committed: {e}")));
    }
    Ok(report)
}

/// A rename primitive (real `std::fs::rename`, or a failing one in tests).
type RenameFn<'a> = &'a dyn Fn(&Path, &Path) -> std::io::Result<()>;

fn stage_and_swap(
    vbak_path: &Path,
    backup_key_hex: &str,
    live_db_path: &Path,
    new_db_key_hex: &str,
    now: &str,
    rename: RenameFn<'_>,
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
    // A replacement PC may not have created its data dir yet (only after the backup
    // verified: a wrong key touches nothing).
    if let Some(parent) = live_db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| BackupError::Io(e.to_string()))?;
    }
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
    let safety_copy = match swap_into_place(&staged, live_db_path, now, rename) {
        Ok(s) => s,
        Err(e) => {
            remove_db_files(&staged);
            return Err(e);
        }
    };

    Ok(InstallReport { new_epoch, devices_flagged, safety_copy })
}

/// Undo a completed swap: discard the restored live DB and move the safety copy (the
/// previous live DB) back into place. With no safety copy (first-time restore) the
/// restored DB is simply removed — the PC is back to "no school".
fn rollback_swap(live: &Path, safety_copy: Option<&Path>) {
    remove_db_files(live);
    if let Some(safety) = safety_copy {
        let _ = std::fs::rename(safety, live);
        for ext in ["-wal", "-shm"] {
            let side = sibling(safety, ext);
            if side.exists() {
                let _ = std::fs::rename(&side, sibling(live, ext));
            }
        }
    }
}

/// Boot-time repair for a restore interrupted by a crash / power cut (§12). Call
/// BEFORE opening the live DB. Two leftovers are possible:
///
/// * a `.<name>.restore-staged` temp DB — encrypted under a new key that was never
///   persisted, so it is useless: removed;
/// * the live DB missing while a `<name>.pre-restore-<stamp>` safety copy exists —
///   the crash fell between moving the old DB aside and renaming the restored one
///   in. The key store still holds the OLD key (it is only replaced after a
///   successful swap), so the newest safety copy is moved back as the live DB.
///
/// Returns true when the live DB was put back.
pub fn recover_interrupted_install(live: &Path) -> bool {
    remove_db_files(&staged_path(live));
    if live.exists() {
        return false;
    }
    match newest_safety_copy(live) {
        Some(copy) => {
            rollback_swap(live, Some(&copy));
            live.exists()
        }
        None => false,
    }
}

/// The newest `<name>.pre-restore-<stamp>` safety copy next to `live`, if any.
fn newest_safety_copy(live: &Path) -> Option<PathBuf> {
    let dir = live.parent()?;
    let name = live.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let prefix = format!("{name}.pre-restore-");
    std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| {
            n.strip_prefix(&prefix).is_some_and(|stamp| !stamp.is_empty() && stamp.bytes().all(|b| b.is_ascii_digit()))
        })
        .max()
        .map(|n| dir.join(n))
}

/// Boot-time repair for a crash in the narrow window AFTER the restored DB was
/// swapped in but BEFORE its new key reached the key store: the live DB is then
/// encrypted under a key nobody holds, while the stored key still opens the newest
/// safety copy (the pre-restore DB). In exactly that case the unreadable restored DB
/// is moved aside (`<name>.unkeyed-restore-<stamp>`, kept, never deleted) and the
/// safety copy goes back live, so the PC starts on its previous data and the restore
/// can simply be run again. A live DB the stored key opens is never touched.
/// Returns true when the previous DB was put back.
pub fn recover_unkeyed_restore(live: &Path, stored_key_hex: &str, now_stamp: &str) -> bool {
    if !live.exists() || db::open_encrypted_readonly(live, stored_key_hex).and_then(|c| c.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0))).is_ok() {
        return false;
    }
    let Some(copy) = newest_safety_copy(live) else { return false };
    let copy_opens = db::open_encrypted_readonly(&copy, stored_key_hex)
        .and_then(|c| c.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0)))
        .is_ok();
    if !copy_opens {
        return false;
    }
    let aside = sibling(live, &format!(".unkeyed-restore-{now_stamp}"));
    if std::fs::rename(live, &aside).is_err() {
        return false;
    }
    for ext in ["-wal", "-shm"] {
        let side = sibling(live, ext);
        if side.exists() {
            let _ = std::fs::rename(&side, sibling(&aside, ext));
        }
    }
    rollback_swap(live, Some(&copy));
    live.exists()
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
/// staged DB into place. Best-effort moves the `-wal`/`-shm` sidecars too. If the
/// second rename fails, the previous live DB is moved back, so a failed swap never
/// leaves the PC without its live database.
fn swap_into_place(staged: &Path, live: &Path, now: &str, rename: RenameFn<'_>) -> BackupResult<Option<PathBuf>> {
    let mut safety_copy = None;
    if live.exists() {
        let stamp: String = now.chars().filter(|c| c.is_ascii_digit()).collect();
        let name = live.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let safety = live.with_file_name(format!("{name}.pre-restore-{stamp}"));
        rename(live, &safety).map_err(|e| BackupError::Io(e.to_string()))?;
        for ext in ["-wal", "-shm"] {
            let side = sibling(live, ext);
            if side.exists() {
                let _ = rename(&side, sibling(&safety, ext).as_path());
            }
        }
        safety_copy = Some(safety);
    }
    if let Err(e) = rename(staged, live) {
        rollback_swap(live, safety_copy.as_deref());
        return Err(BackupError::Io(e.to_string()));
    }
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

    /// A real previous live DB (its own key) with a marker school name.
    fn make_old_live(live: &Path) {
        let mut c = db::open_encrypted(live, DB_KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        c.execute(
            "INSERT INTO school(id,name,backup_salt,server_epoch,created_at,updated_at) VALUES ('old','Old Live School', x'00', 1, 't', 't')",
            [],
        )
        .unwrap();
        c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();
    }

    fn old_live_usable(live: &Path) -> bool {
        let Ok(c) = db::open_encrypted(live, DB_KEY) else { return false };
        db::is_readable(&c)
            && c.query_row("SELECT name FROM school", [], |r| r.get::<_, String>(0)).ok().as_deref() == Some("Old Live School")
    }

    fn no_staged_left(dir: &Path) -> bool {
        std::fs::read_dir(dir).unwrap().flatten().all(|e| !e.file_name().to_string_lossy().contains("restore-staged"))
    }

    #[test]
    fn failed_rename_mid_swap_puts_the_previous_live_db_back() {
        let dir = tmp_dir("midswap");
        let vbak = make_vbak(&dir);
        let live = dir.join("vidya.db");
        make_old_live(&live);
        // The live DB is moved aside fine, then renaming the staged DB in fails.
        let failing = |a: &Path, b: &Path| -> std::io::Result<()> {
            if a.to_string_lossy().contains("restore-staged") {
                Err(std::io::Error::other("disk yanked"))
            } else {
                std::fs::rename(a, b)
            }
        };
        let err = stage_and_swap(&vbak, BK_KEY, &live, NEW_KEY, "2026-09-23T10:00:00Z", &failing).unwrap_err();
        assert!(matches!(err, BackupError::Io(_)), "{err:?}");
        assert!(old_live_usable(&live), "previous live DB back in place and usable");
        assert!(no_staged_left(&dir), "staged DB cleaned up");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn failed_commit_rolls_the_swap_back() {
        let dir = tmp_dir("commitfail");
        let vbak = make_vbak(&dir);
        let live = dir.join("vidya.db");
        make_old_live(&live);
        let err = install_then(&vbak, BK_KEY, &live, NEW_KEY, "2026-09-23T10:00:00Z", || Err("keychain locked".into()))
            .unwrap_err();
        assert!(matches!(err, BackupError::Io(ref m) if m.contains("keychain locked")), "{err:?}");
        assert!(old_live_usable(&live), "rolled back to the previous live DB under the OLD key");
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.contains("pre-restore") || n.contains("restore-staged"))
            .collect();
        assert!(leftovers.is_empty(), "no safety copy / staged DB left: {leftovers:?}");

        // First-time restore whose commit fails → back to "no live DB" (no school).
        let fresh = dir.join("fresh").join("vidya.db");
        assert!(install_then(&vbak, BK_KEY, &fresh, NEW_KEY, "2026-09-23T10:00:00Z", || Err("x".into())).is_err());
        assert!(!fresh.exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn boot_recovery_removes_staged_and_restores_a_missing_live_db() {
        let dir = tmp_dir("bootrecover");
        let live = dir.join("vidya.db");
        make_old_live(&live);
        // Crash window: the live DB was moved aside, a staged DB was left behind.
        let safety = dir.join("vidya.db.pre-restore-20260923100000");
        std::fs::rename(&live, &safety).unwrap();
        std::fs::write(staged_path(&live), b"half-written staged db").unwrap();
        assert!(recover_interrupted_install(&live), "live DB put back");
        assert!(old_live_usable(&live));
        assert!(!safety.exists());
        assert!(no_staged_left(&dir));
        // A normal boot (live present) is a no-op apart from dropping a stale stage.
        std::fs::write(staged_path(&live), b"stale").unwrap();
        assert!(!recover_interrupted_install(&live));
        assert!(old_live_usable(&live) && no_staged_left(&dir));
        // Nothing to recover on a fresh PC.
        let fresh = dir.join("fresh");
        std::fs::create_dir_all(&fresh).unwrap();
        assert!(!recover_interrupted_install(&fresh.join("vidya.db")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn summary_with_recovery_derives_key_from_the_meta_salt() {
        let dir = tmp_dir("recovery");
        let recovery = "ABCDE-FGHIJ-KLMNO-PQRST-UVWXY-23456";
        let salt = [9u8; 16];
        let key = recovery::derive_backup_key(&recovery::normalize(recovery), &salt).unwrap();
        let key_hex = hex_encode(&key);

        // A source school exported under the RECOVERY-derived key, with a matching
        // `.meta` sidecar (what run_backup writes).
        let src_path = dir.join("source.db");
        let mut c = db::open_encrypted(&src_path, DB_KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        let now = "2026-09-23T06:00:00Z";
        c.execute(
            "INSERT INTO school(id,name,backup_salt,server_epoch,created_at,updated_at) VALUES ('s','Recovery School', ?2, 1, ?1, ?1)",
            params![now, salt.to_vec()],
        )
        .unwrap();
        {
            let tx = c.transaction().unwrap();
            audit::append(&tx, &audit::AuditEntry { at: now.into(), action: "seed".into(), ..Default::default() }).unwrap();
            tx.commit().unwrap();
        }
        let vbak = dir.join("vidya-recovery-school-20260922-0600.vbak");
        crate::backup::export_backup(&c, &vbak, &key_hex).unwrap();
        std::fs::write(
            crate::backup::meta_path(&vbak),
            serde_json::to_string(&crate::backup::BackupMeta { v: 1, salt: hex_encode(&salt), school_id: "s".into() }).unwrap(),
        )
        .unwrap();

        // The recovery key + the .meta salt derive the right backup key → summary reads.
        let (summary, derived_hex) = summary_with_recovery(&vbak, recovery).unwrap();
        assert_eq!(summary.school_name, "Recovery School");
        assert_eq!(derived_hex, key_hex);
        // A wrong recovery key derives a different key → the backup is unreadable.
        assert!(summary_with_recovery(&vbak, "WRONG-WRONG-WRONG-WRONG-WRONG-00000").is_err());
        // Missing .meta → a clear error (not a panic).
        std::fs::remove_file(crate::backup::meta_path(&vbak)).unwrap();
        assert!(matches!(backup_key_from_meta(&vbak, recovery), Err(BackupError::Unreadable(_))));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_crash_before_the_new_key_was_saved_puts_the_previous_db_back() {
        let dir = tmp_dir("unkeyed");
        let live = dir.join("vidya.db");
        // The pre-restore DB (stored key) as the safety copy, the restored DB live
        // under a key that never reached the key store.
        {
            let mut c = db::open_encrypted(&dir.join("vidya.db.pre-restore-20261105010101"), DB_KEY).unwrap();
            db::run_migrations(&mut c).unwrap();
            c.execute("INSERT INTO app_kv(key,value_json,updated_at) VALUES ('marker','old','t')", []).unwrap();
        }
        {
            let mut c = db::open_encrypted(&live, NEW_KEY).unwrap();
            db::run_migrations(&mut c).unwrap();
        }
        assert!(recover_unkeyed_restore(&live, DB_KEY, "20261105020202"));
        let c = db::open_encrypted(&live, DB_KEY).unwrap();
        let v: String = c.query_row("SELECT value_json FROM app_kv WHERE key='marker'", [], |r| r.get(0)).unwrap();
        assert_eq!(v, "old");
        assert!(dir.join("vidya.db.unkeyed-restore-20261105020202").exists(), "the restored copy is kept");
        // A live DB the stored key opens is left alone.
        assert!(!recover_unkeyed_restore(&live, DB_KEY, "x"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
