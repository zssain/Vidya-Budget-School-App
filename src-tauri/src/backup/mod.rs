//! Backups (docs/00-SYSTEM-CONTEXT.md §12, prompts/P08 Part B). Server-only.
//!
//! The scheduler checks every minute (06:00 daily + on quit if none today); a run
//! is: **SQLCipher export** keyed by the backup key → **verify** (open,
//! `integrity_check`, row counts, audit-chain head) → temp name, fsync, atomic
//! rename → local `<AppData>/Vidya/backups` → **upload to Drive** `backups/`
//! (verify-by-readback) → apply **retention** (30 daily + 12 monthly per
//! destination, via [`vidya_core::backup::select_for_deletion`]).
//!
//! Honest status (§13, rule 7): a run is only [`Outcome::Verified`] when the local
//! copy AND the Drive copy verified; local-only is [`Outcome::Partial`]; a failed
//! or half-written export is [`Outcome::Failed`] and leaves nothing behind (the
//! export writes a temp file and only renames it into place after fsync).
//!
//! The pure decisions (retention selection, the filename, the Verified/Partial/
//! Failed matrix) live in [`vidya_core::backup`]; this module is their IO shell.
//!
//! **[OWNER] open design question (flagged, not invented):** how the backup key is
//! obtained for an *unattended* 06:00 run (the recovery key is shown once and never
//! stored), and where the KDF salt lives so a restore on a fresh PC can derive the
//! same key before opening the file. Recommended default: derive the backup key at
//! setup and cache it in the OS keychain (account `backup-key`), and write the
//! (non-secret) salt + school id in a plaintext `<file>.vbak.meta` sidecar next to
//! each backup so restore can derive-and-verify. Every function here takes the
//! backup key as a parameter, so this decision does not change the mechanics.

pub mod restore;

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

use crate::db;
use crate::security::audit;
use crate::sync::drive::{DriveApi, DriveError};
use vidya_core::backup::{backup_outcome, select_for_deletion, BackupOutcome, BackupRecord, DriveState};

/// A backup failure. Distinguished so the caller can map to the right "Needs
/// attention" copy and so tests can assert the exact failure path.
#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("export failed: {0}")]
    Export(String),
    #[error("backup unreadable (wrong key / not a database): {0}")]
    Unreadable(String),
    #[error("verify failed: {0}")]
    Verify(String),
    #[error("filesystem error: {0}")]
    Io(String),
}

pub type BackupResult<T> = Result<T, BackupError>;

// ---- scheduler decision (pure) ------------------------------------------

/// Whether a backup is due (§12: "06:00 daily + on quit if none today").
///
/// * a verified backup already ran today → **not** due;
/// * otherwise on quit → due (protect the day's work before closing);
/// * otherwise due once the local clock has reached 06:00.
///
/// Pure: the clock is passed in as `today` (`YYYY-MM-DD`) + `hour` (0..=23).
pub fn is_backup_due(last_verified_date: Option<&str>, today: &str, hour: u8, on_quit: bool) -> bool {
    if last_verified_date == Some(today) {
        return false; // already have a verified backup today
    }
    on_quit || hour >= 6
}

// ---- export --------------------------------------------------------------

/// Export `src` to a new SQLCipher database at `dest`, encrypted with
/// `backup_key_hex` (64 hex chars). Writes to a temp sibling, fsyncs it, then
/// atomically renames into place — so a reader never sees a half-written backup,
/// and a failure leaves nothing behind (§12, disk-full → clean failure).
pub fn export_backup(src: &Connection, dest: &Path, backup_key_hex: &str) -> BackupResult<()> {
    // Fold WAL into the main file so the export captures every committed change.
    let _ = src.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

    let tmp = temp_sibling(dest);
    let _ = std::fs::remove_file(&tmp);
    let tmp_str = tmp.to_string_lossy().to_string();

    // ATTACH the target with the backup key (key inlined per SQLCipher raw-key
    // syntax; the path is bound). Then copy the whole schema + data into it.
    src.execute(
        &format!("ATTACH DATABASE ?1 AS bak KEY \"x'{backup_key_hex}'\""),
        params![tmp_str],
    )
    .map_err(|e| BackupError::Export(e.to_string()))?;

    let export = src.execute_batch("SELECT sqlcipher_export('bak'); DETACH DATABASE bak;");
    if let Err(e) = export {
        let _ = src.execute_batch("DETACH DATABASE bak;");
        let _ = std::fs::remove_file(&tmp);
        return Err(BackupError::Export(e.to_string()));
    }

    fsync_file(&tmp)?;
    std::fs::rename(&tmp, dest).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        BackupError::Io(e.to_string())
    })?;
    Ok(())
}

// ---- verify --------------------------------------------------------------

/// The verifiable facts about a database: the row count of every domain table
/// (FTS shadow tables excluded — they are rebuildable) and the audit-chain head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expectations {
    pub counts: Vec<(String, i64)>,
    pub chain_head: String,
}

/// Compute the [`Expectations`] for an open connection (call on the source before
/// the export, then compare against the backup).
pub fn compute_expectations(conn: &Connection) -> rusqlite::Result<Expectations> {
    let tables = domain_tables(conn)?;
    let mut counts = Vec::with_capacity(tables.len());
    for t in &tables {
        let n: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM \"{t}\""), [], |r| r.get(0))?;
        counts.push((t.clone(), n));
    }
    Ok(Expectations { counts, chain_head: audit::head_hash(conn)? })
}

/// Domain tables (excludes `sqlite_*` internals and every `*_fts*` shadow table).
fn domain_tables(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master \
         WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '%\\_fts%' ESCAPE '\\' \
         ORDER BY name",
    )?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    rows.collect()
}

/// The result of verifying a backup file (§12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifyReport {
    pub integrity_ok: bool,
    pub counts_match: bool,
    pub chain_head_match: bool,
}

impl VerifyReport {
    /// True only when every check passed — the sole condition for a Verified run.
    pub fn verified(&self) -> bool {
        self.integrity_ok && self.counts_match && self.chain_head_match
    }
}

/// Open the backup with `backup_key_hex` (read-only) and verify it: `PRAGMA
/// integrity_check`, then domain row counts and the audit-chain head against
/// `expected` (§12). A wrong key or a corrupt file → [`BackupError::Unreadable`].
pub fn verify_backup(
    path: &Path,
    backup_key_hex: &str,
    expected: &Expectations,
) -> BackupResult<VerifyReport> {
    let conn = db::open_encrypted_readonly(path, backup_key_hex)
        .map_err(|e| BackupError::Unreadable(e.to_string()))?;
    if !db::is_readable(&conn) {
        return Err(BackupError::Unreadable("wrong key or not a database".into()));
    }
    let integrity_ok = integrity_check(&conn)?;
    // If the pages are damaged, the counts read below may themselves error →
    // surface as Unreadable rather than a silent false "verified".
    let actual = compute_expectations(&conn).map_err(|e| BackupError::Unreadable(e.to_string()))?;
    Ok(VerifyReport {
        integrity_ok,
        counts_match: actual.counts == expected.counts,
        chain_head_match: actual.chain_head == expected.chain_head,
    })
}

fn integrity_check(conn: &Connection) -> BackupResult<bool> {
    let first: String = conn
        .query_row("PRAGMA integrity_check(1)", [], |r| r.get(0))
        .map_err(|e| BackupError::Unreadable(e.to_string()))?;
    Ok(first == "ok")
}

// ---- Drive upload (verify-by-readback) ----------------------------------

/// Upload `bytes` as `filename` into the Drive `backups` folder and verify by
/// reading it back (re-download + byte compare — no md5 crate needed, matching the
/// P06 exchange engine). Uploads under a temp name then renames, so a reader never
/// sees a half file. Returns `Ok(true)` when the readback matches.
pub fn upload_and_verify(
    drive: &dyn DriveApi,
    backups_folder: &str,
    filename: &str,
    bytes: &[u8],
) -> Result<bool, DriveError> {
    let tmp_name = format!("{filename}.uploading");
    let props = std::collections::BTreeMap::new();
    let file = drive.create(backups_folder, &tmp_name, bytes, &props)?;
    let readback = drive.download(&file.id)?;
    if readback != bytes {
        let _ = drive.delete(&file.id);
        return Ok(false);
    }
    drive.rename(&file.id, filename)?;
    Ok(true)
}

// ---- retention (apply the pure selection) -------------------------------

/// The retained-file extension.
pub const BACKUP_EXT: &str = ".vbak";

/// Build a sortable `at` (`YYYY-MM-DDTHH:MM`) from a backup filename
/// `vidya-<slug>-YYYYMMDD-HHMM.vbak`. `None` if the name is not a backup file.
pub fn parse_backup_at(filename: &str) -> Option<String> {
    let stem = filename.strip_suffix(BACKUP_EXT)?;
    // The stamp is the last `YYYYMMDD-HHMM` (13 chars) of the stem.
    let (date, time) = stem.rsplit_once('-')?;
    let date = date.rsplit_once('-').map(|(_, d)| d).unwrap_or(date); // strip the slug's trailing '-'
    if date.len() != 8 || time.len() != 4 || !date.bytes().chain(time.bytes()).all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(format!(
        "{}-{}-{}T{}:{}",
        &date[0..4],
        &date[4..6],
        &date[6..8],
        &time[0..2],
        &time[2..4]
    ))
}

/// Apply retention to the local backups directory (§12). Lists `*.vbak`, asks
/// [`select_for_deletion`] which to drop (destination `"local"`), deletes them,
/// and returns the deleted filenames.
pub fn retain_local(dir: &Path, today: &str) -> BackupResult<Vec<String>> {
    let mut records = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(BackupError::Io(e.to_string())),
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(at) = parse_backup_at(&name) {
            records.push(BackupRecord { id: name, at, destination: "local".into() });
        }
    }
    let delete = select_for_deletion(&records, today);
    for name in &delete {
        std::fs::remove_file(dir.join(name)).map_err(|e| BackupError::Io(e.to_string()))?;
    }
    Ok(delete)
}

/// Apply retention to the Drive `backups/` folder (§12). Lists it, selects
/// (destination `"drive"`) and deletes; returns the deleted filenames.
pub fn retain_drive(
    drive: &dyn DriveApi,
    backups_folder: &str,
    today: &str,
) -> Result<Vec<String>, DriveError> {
    let files = drive.list(backups_folder)?;
    let mut records = Vec::new();
    let mut id_by_name = std::collections::HashMap::new();
    for f in &files {
        if let Some(at) = parse_backup_at(&f.name) {
            records.push(BackupRecord { id: f.name.clone(), at, destination: "drive".into() });
            id_by_name.insert(f.name.clone(), f.id.clone());
        }
    }
    let delete = select_for_deletion(&records, today);
    for name in &delete {
        if let Some(id) = id_by_name.get(name) {
            drive.delete(id)?;
        }
    }
    Ok(delete)
}

// ---- run a whole backup --------------------------------------------------

/// The result of one backup run (recorded to `backup_run` and shown on the
/// Backups screen / Home band by the thin command layer).
#[derive(Debug, Clone)]
pub struct RunResult {
    pub outcome: BackupOutcome,
    pub filename: String,
    pub chain_head: String,
    pub local_verified: bool,
    pub drive: DriveState,
}

/// Where a run should try to also place a Drive copy.
pub struct DriveTarget<'a> {
    pub client: &'a dyn DriveApi,
    pub backups_folder: &'a str,
}

/// Run a full backup (§12): export → local verify → optional Drive copy → decide
/// the [`BackupOutcome`]. `at` is an RFC-3339 timestamp (used for the filename;
/// no clock is read here). Retention is applied by the caller after the run is
/// recorded. Returns `Failed` semantics as an `Err` only when nothing was written;
/// a produced-but-unverified local copy is surfaced as `Failed` in the result.
pub fn run_backup(
    src: &Connection,
    slug: &str,
    backup_key_hex: &str,
    at: &str,
    local_dir: &Path,
    drive: Option<DriveTarget<'_>>,
) -> BackupResult<RunResult> {
    std::fs::create_dir_all(local_dir).map_err(|e| BackupError::Io(e.to_string()))?;
    let filename = vidya_core::backup::backup_filename(slug, at);
    let dest = local_dir.join(&filename);

    // Expectations from the live source, then export + verify the local copy.
    let expected = compute_expectations(src).map_err(|e| BackupError::Verify(e.to_string()))?;
    export_backup(src, &dest, backup_key_hex)?;
    let report = verify_backup(&dest, backup_key_hex, &expected)?;
    let local_verified = report.verified();
    if !local_verified {
        // The local copy failed verification — do not keep a bad file around.
        let _ = std::fs::remove_file(&dest);
        return Ok(RunResult {
            outcome: BackupOutcome::Failed,
            filename,
            chain_head: expected.chain_head,
            local_verified: false,
            drive: DriveState::NotConfigured,
        });
    }

    // Optional Drive copy.
    let bytes = std::fs::read(&dest).map_err(|e| BackupError::Io(e.to_string()))?;
    let drive_state = match drive {
        None => DriveState::NotConfigured,
        Some(t) => match upload_and_verify(t.client, t.backups_folder, &filename, &bytes) {
            Ok(true) => DriveState::Verified,
            Ok(false) | Err(_) => DriveState::Unavailable,
        },
    };

    Ok(RunResult {
        outcome: backup_outcome(local_verified, drive_state),
        filename,
        chain_head: expected.chain_head,
        local_verified,
        drive: drive_state,
    })
}

// ---- small IO helpers ----------------------------------------------------

fn temp_sibling(dest: &Path) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let name = dest.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    dest.with_file_name(format!(".{name}.tmp-{}-{nanos}", std::process::id()))
}

fn fsync_file(p: &Path) -> BackupResult<()> {
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(p)
        .map_err(|e| BackupError::Io(e.to_string()))?;
    f.sync_all().map_err(|e| BackupError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::drive::fake::{FakeDrive, PRINCIPAL};

    const DB_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const BK_KEY: &str = "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100";

    fn tmp_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let d = std::env::temp_dir().join(format!("vidya-bak-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A migrated in-memory source with a few rows (incl. a student → FTS + a real
    /// audit chain) so the export exercises triggers and the chain head.
    fn source() -> Connection {
        let mut c = db::open_in_memory(DB_KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        let now = "2026-09-23T06:00:00Z";
        c.execute(
            "INSERT INTO school(id,name,backup_salt,created_at,updated_at) VALUES ('s','Saraswati Public School', x'00', ?1, ?1)",
            params![now],
        )
        .unwrap();
        c.execute("PRAGMA foreign_keys=OFF;", []).unwrap();
        c.execute(
            "INSERT INTO student(id,name,created_at,updated_at) VALUES ('stu1','रिया वर्मा',?1,?1)",
            params![now],
        )
        .unwrap();
        c.execute(
            "INSERT INTO student(id,name,created_at,updated_at) VALUES ('stu2','Aarav Gupta',?1,?1)",
            params![now],
        )
        .unwrap();
        // A real 2-entry audit chain so head_hash is non-empty.
        {
            let tx = c.transaction().unwrap();
            audit::append(&tx, &audit::AuditEntry { at: now.into(), action: "a".into(), ..Default::default() }).unwrap();
            audit::append(&tx, &audit::AuditEntry { at: now.into(), action: "b".into(), ..Default::default() }).unwrap();
            tx.commit().unwrap();
        }
        c
    }

    // ---- is_backup_due ----

    #[test]
    fn due_matrix() {
        // Already backed up today → never due.
        assert!(!is_backup_due(Some("2026-09-23"), "2026-09-23", 6, false));
        assert!(!is_backup_due(Some("2026-09-23"), "2026-09-23", 23, true));
        // None today, before 06:00, not quitting → not due.
        assert!(!is_backup_due(Some("2026-09-22"), "2026-09-23", 5, false));
        assert!(!is_backup_due(None, "2026-09-23", 5, false));
        // None today, 06:00 or later → due.
        assert!(is_backup_due(Some("2026-09-22"), "2026-09-23", 6, false));
        // None today, quitting (even before 06:00) → due.
        assert!(is_backup_due(Some("2026-09-22"), "2026-09-23", 1, true));
    }

    // ---- export + verify round-trip ----

    #[test]
    fn export_then_verify_roundtrips() {
        let src = source();
        let expected = compute_expectations(&src).unwrap();
        let dir = tmp_dir("rt");
        let dest = dir.join("vidya-x-20260923-0600.vbak");
        export_backup(&src, &dest, BK_KEY).unwrap();
        assert!(dest.exists(), "backup file was written");
        let report = verify_backup(&dest, BK_KEY, &expected).unwrap();
        assert!(report.verified(), "roundtrip must verify: {report:?}");
        assert!(report.integrity_ok && report.counts_match && report.chain_head_match);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn no_temp_file_left_behind() {
        let src = source();
        let dir = tmp_dir("tmp");
        let dest = dir.join("vidya-x-20260923-0600.vbak");
        export_backup(&src, &dest, BK_KEY).unwrap();
        let stray: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.contains(".tmp-"))
            .collect();
        assert!(stray.is_empty(), "no temp file should remain: {stray:?}");
        std::fs::remove_dir_all(&dir).ok();
    }

    // ---- verify failure paths (prompt VERIFICATION) ----

    #[test]
    fn verify_fails_with_wrong_key() {
        let src = source();
        let expected = compute_expectations(&src).unwrap();
        let dir = tmp_dir("wrongkey");
        let dest = dir.join("b.vbak");
        export_backup(&src, &dest, BK_KEY).unwrap();
        let wrong = "00000000000000000000000000000000000000000000000000000000000000ff";
        assert!(matches!(verify_backup(&dest, wrong, &expected), Err(BackupError::Unreadable(_))));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn verify_fails_on_corrupted_file() {
        let src = source();
        let expected = compute_expectations(&src).unwrap();
        let dir = tmp_dir("corrupt");
        let dest = dir.join("b.vbak");
        export_backup(&src, &dest, BK_KEY).unwrap();
        // Flip bytes deep in the file (past the SQLCipher salt in page 1) → a page
        // HMAC / structure check must fail.
        let mut bytes = std::fs::read(&dest).unwrap();
        let mid = bytes.len() / 2;
        for b in bytes.iter_mut().skip(mid).take(64) {
            *b ^= 0xFF;
        }
        std::fs::write(&dest, &bytes).unwrap();
        let result = verify_backup(&dest, BK_KEY, &expected);
        let bad = match result {
            Err(_) => true,
            Ok(r) => !r.verified(),
        };
        assert!(bad, "a corrupted backup must not verify");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn verify_flags_row_count_mismatch() {
        let src = source();
        let mut expected = compute_expectations(&src).unwrap();
        let dir = tmp_dir("count");
        let dest = dir.join("b.vbak");
        export_backup(&src, &dest, BK_KEY).unwrap();
        // Pretend one more student than the backup holds.
        for (t, n) in expected.counts.iter_mut() {
            if t == "student" {
                *n += 1;
            }
        }
        let report = verify_backup(&dest, BK_KEY, &expected).unwrap();
        assert!(!report.counts_match, "row-count mismatch must be caught");
        assert!(report.integrity_ok, "the file itself is still structurally sound");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn verify_flags_chain_head_mismatch() {
        let src = source();
        let mut expected = compute_expectations(&src).unwrap();
        let dir = tmp_dir("chain");
        let dest = dir.join("b.vbak");
        export_backup(&src, &dest, BK_KEY).unwrap();
        expected.chain_head = "deadbeef".into();
        let report = verify_backup(&dest, BK_KEY, &expected).unwrap();
        assert!(!report.chain_head_match, "chain-head mismatch must be caught");
        std::fs::remove_dir_all(&dir).ok();
    }

    // ---- Drive upload ----

    #[test]
    fn upload_verifies_by_readback() {
        let drive = FakeDrive::new();
        let layout = drive.provision_school(&[("t", "d")]);
        let client = drive.as_actor(PRINCIPAL);
        let ok = upload_and_verify(&client, &layout.backups, "vidya-x-20260923-0600.vbak", b"BACKUPBYTES").unwrap();
        assert!(ok);
        // The final (renamed) file is present with the exact bytes.
        let files = client.list(&layout.backups).unwrap();
        let f = files.iter().find(|f| f.name == "vidya-x-20260923-0600.vbak").unwrap();
        assert_eq!(client.download(&f.id).unwrap(), b"BACKUPBYTES");
    }

    #[test]
    fn upload_surfaces_quota_full() {
        let drive = FakeDrive::new();
        let layout = drive.provision_school(&[("t", "d")]);
        drive.set_quota_full(true);
        let client = drive.as_actor(PRINCIPAL);
        assert!(matches!(
            upload_and_verify(&client, &layout.backups, "b.vbak", b"x"),
            Err(DriveError::QuotaFull)
        ));
    }

    // ---- run_backup end to end ----

    #[test]
    fn run_backup_local_only_is_partial() {
        let src = source();
        let dir = tmp_dir("run-local");
        let r = run_backup(&src, "saraswati", BK_KEY, "2026-09-23T06:02:00Z", &dir, None).unwrap();
        assert_eq!(r.outcome, BackupOutcome::Partial, "local-only → Partial");
        assert!(r.local_verified);
        assert_eq!(r.filename, "vidya-saraswati-20260923-0602.vbak");
        assert!(dir.join(&r.filename).exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn run_backup_with_drive_is_verified() {
        let src = source();
        let dir = tmp_dir("run-drive");
        let drive = FakeDrive::new();
        let layout = drive.provision_school(&[("t", "d")]);
        let client = drive.as_actor(PRINCIPAL);
        let target = DriveTarget { client: &client, backups_folder: &layout.backups };
        let r = run_backup(&src, "saraswati", BK_KEY, "2026-09-23T06:02:00Z", &dir, Some(target)).unwrap();
        assert_eq!(r.outcome, BackupOutcome::Verified, "PC + Drive both verified");
        assert_eq!(r.drive, DriveState::Verified);
        // The Drive copy exists under the final name.
        assert!(client.list(&layout.backups).unwrap().iter().any(|f| f.name == r.filename));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn run_backup_drive_unavailable_is_partial_local_intact() {
        let src = source();
        let dir = tmp_dir("run-noupload");
        let drive = FakeDrive::new();
        let layout = drive.provision_school(&[("t", "d")]);
        drive.set_quota_full(true); // Drive fails
        let client = drive.as_actor(PRINCIPAL);
        let target = DriveTarget { client: &client, backups_folder: &layout.backups };
        let r = run_backup(&src, "saraswati", BK_KEY, "2026-09-23T06:02:00Z", &dir, Some(target)).unwrap();
        assert_eq!(r.outcome, BackupOutcome::Partial, "Drive unavailable → Partial");
        assert_eq!(r.drive, DriveState::Unavailable);
        assert!(r.local_verified, "the local copy is still verified + kept");
        assert!(dir.join(&r.filename).exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    // ---- retention application ----

    #[test]
    fn parse_backup_at_reads_the_stamp() {
        assert_eq!(
            parse_backup_at("vidya-saraswati-public-school-20260923-0602.vbak").as_deref(),
            Some("2026-09-23T06:02")
        );
        assert_eq!(parse_backup_at("notes.txt"), None);
        assert_eq!(parse_backup_at("vidya-x-badstamp.vbak"), None);
    }

    #[test]
    fn retain_local_deletes_only_old_files() {
        let dir = tmp_dir("retain");
        // 31 daily files in December → the oldest (Dec 1) falls out of the 30-day tier.
        for d in 1..=31 {
            let name = format!("vidya-x-202612{d:02}-0600.vbak");
            std::fs::write(dir.join(&name), b"x").unwrap();
        }
        let deleted = retain_local(&dir, "2026-12-31").unwrap();
        assert_eq!(deleted, vec!["vidya-x-20261201-0600.vbak".to_string()]);
        assert!(!dir.join("vidya-x-20261201-0600.vbak").exists());
        assert!(dir.join("vidya-x-20261231-0600.vbak").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn retain_drive_deletes_only_old_files() {
        let drive = FakeDrive::new();
        let layout = drive.provision_school(&[("t", "d")]);
        let client = drive.as_actor(PRINCIPAL);
        let props = std::collections::BTreeMap::new();
        for d in 1..=31 {
            let name = format!("vidya-x-202612{d:02}-0600.vbak");
            client.create(&layout.backups, &name, b"x", &props).unwrap();
        }
        let deleted = retain_drive(&client, &layout.backups, "2026-12-31").unwrap();
        assert_eq!(deleted, vec!["vidya-x-20261201-0600.vbak".to_string()]);
        assert!(!client
            .list(&layout.backups)
            .unwrap()
            .iter()
            .any(|f| f.name == "vidya-x-20261201-0600.vbak"));
    }
}
