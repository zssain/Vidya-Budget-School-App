//! Reliability (prompts/P09 §3): rotated file logs, a panic hook that records
//! the crash for a calm "Restart Vidya" screen on next launch, and a startup
//! integrity gate (SQLite `PRAGMA integrity_check` + audit-chain check) whose
//! failure must block writes and guide the Principal to Recover.
//!
//! Log policy: one file per UTC day at `<app_data>/logs/vidya-YYYY-MM-DD.log`,
//! kept 14 days, each ≤ 10 MB (older or oversize files are pruned on startup).
//! No `tracing-appender` (not on the §13 allow-list) — rotation is plain `std::fs`.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;

use crate::db::now_iso;

const KEEP_DAYS: i64 = 14;
const MAX_BYTES: u64 = 10 * 1024 * 1024; // 10 MB per file
const PREFIX: &str = "vidya-";
const CRASH_MARKER: &str = "last-crash";

/// `<app_data>/logs`.
pub fn logs_dir(app_data: &Path) -> PathBuf {
    app_data.join("logs")
}

fn day_of(iso: &str) -> String {
    iso.get(0..10).unwrap_or("0000-00-00").to_string()
}

/// `today` minus `days`, as `YYYY-MM-DD` (empty on parse failure).
fn minus_days(today: &str, days: i64) -> String {
    use time::macros::format_description;
    let fmt = format_description!("[year]-[month]-[day]");
    time::Date::parse(today, &fmt)
        .ok()
        .and_then(|d| d.checked_sub(time::Duration::days(days)))
        .and_then(|d| d.format(&fmt).ok())
        .unwrap_or_default()
}

/// Remove rotated logs older than `keep_days` (by the date in the filename) or
/// larger than `max_bytes`. `today` is `YYYY-MM-DD`. Returns files removed.
pub fn prune(dir: &Path, keep_days: i64, max_bytes: u64, today: &str) -> usize {
    let cutoff = minus_days(today, keep_days);
    let mut removed = 0;
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let day = match name.strip_prefix(PREFIX).and_then(|s| s.strip_suffix(".log")) {
            Some(d) => d,
            None => continue,
        };
        let too_old = !cutoff.is_empty() && day < cutoff.as_str();
        let too_big = entry.metadata().map(|m| m.len() > max_bytes).unwrap_or(false);
        if (too_old || too_big) && fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Append one line to today's log file (best-effort).
pub fn append_line(dir: &Path, line: &str) {
    let _ = fs::create_dir_all(dir);
    let path = dir.join(format!("{PREFIX}{}.log", day_of(&now_iso())));
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}

/// Record a crash so the next launch can show the calm "Restart Vidya" screen.
fn mark_crash(dir: &Path) {
    let _ = fs::create_dir_all(dir);
    let _ = fs::write(dir.join(CRASH_MARKER), now_iso());
}

/// True if the previous run crashed; clears the marker (one-shot). The frontend
/// calls this on startup to decide whether to show the calm restart screen.
pub fn take_last_crash(app_data: &Path) -> bool {
    let marker = logs_dir(app_data).join(CRASH_MARKER);
    if marker.exists() {
        let _ = fs::remove_file(&marker);
        true
    } else {
        false
    }
}

/// Install a panic hook that writes the panic to the rotated log + a crash
/// marker, then chains to the previous hook. (Release uses panic=abort, so this
/// runs once and the calm screen appears on the next launch.)
pub fn install_panic_hook(dir: PathBuf) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".into());
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        append_line(&dir, &format!("[{}] PANIC at {loc}: {msg}", now_iso()));
        mark_crash(&dir);
        previous(info);
    }));
}

/// Set up rotated file logging + the panic hook. Best-effort; never fails the app.
pub fn init(app_data: &Path) {
    let dir = logs_dir(app_data);
    let _ = fs::create_dir_all(&dir);
    prune(&dir, KEEP_DAYS, MAX_BYTES, &day_of(&now_iso()));
    install_panic_hook(dir.clone());
    let make = DailyLog { dir: Mutex::new(dir) };
    let _ = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .with_writer(make)
        .try_init();
}

/// Outcome of the startup integrity gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Integrity {
    /// DB structurally sound and the audit chain verifies.
    Ok,
    /// SQLite `PRAGMA integrity_check` reported corruption.
    DbCorrupt(String),
    /// The audit hash-chain is broken at the given `server_seq`.
    AuditBroken(i64),
}

impl Integrity {
    pub fn is_ok(&self) -> bool {
        matches!(self, Integrity::Ok)
    }
}

/// Run the daily gate (§3): `PRAGMA integrity_check` + audit-chain check. A
/// non-`Ok` result must block writes and route the Principal to Recover.
pub fn integrity_gate(conn: &Connection) -> rusqlite::Result<Integrity> {
    let check: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if check != "ok" {
        return Ok(Integrity::DbCorrupt(check));
    }
    if let Some(bad_seq) = crate::security::audit::verify_chain(conn)? {
        return Ok(Integrity::AuditBroken(bad_seq));
    }
    Ok(Integrity::Ok)
}

/// A `MakeWriter` that appends each log event to today's file, reopening on the
/// UTC day boundary (so midnight rolls over with no long-lived handle).
struct DailyLog {
    dir: Mutex<PathBuf>,
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for DailyLog {
    type Writer = DayFile;
    fn make_writer(&'a self) -> Self::Writer {
        let dir = self.dir.lock().map(|g| g.clone()).unwrap_or_default();
        DayFile {
            path: dir.join(format!("{PREFIX}{}.log", day_of(&now_iso()))),
        }
    }
}

struct DayFile {
    path: PathBuf,
}

impl Write for DayFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut f = OpenOptions::new().create(true).append(true).open(&self.path)?;
        f.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let p = std::env::temp_dir().join(format!("vidya-reliab-{}", crate::db::now_iso().replace([':', '.'], "-")));
        let _ = fs::create_dir_all(&p);
        p
    }

    #[test]
    fn minus_days_walks_back_across_months() {
        assert_eq!(minus_days("2026-09-24", 14), "2026-09-10");
        assert_eq!(minus_days("2026-09-05", 14), "2026-08-22");
    }

    #[test]
    fn prune_removes_old_and_oversize_logs() {
        let dir = tmp();
        // Old (should go), recent (should stay), oversize-recent (should go).
        fs::write(dir.join("vidya-2026-09-01.log"), b"old").unwrap();
        fs::write(dir.join("vidya-2026-09-20.log"), b"recent").unwrap();
        fs::write(dir.join("vidya-2026-09-23.log"), vec![0u8; (MAX_BYTES + 1) as usize]).unwrap();
        fs::write(dir.join("not-a-log.txt"), b"keep").unwrap();

        let removed = prune(&dir, KEEP_DAYS, MAX_BYTES, "2026-09-24");
        assert_eq!(removed, 2);
        assert!(!dir.join("vidya-2026-09-01.log").exists(), "old pruned");
        assert!(dir.join("vidya-2026-09-20.log").exists(), "recent kept");
        assert!(!dir.join("vidya-2026-09-23.log").exists(), "oversize pruned");
        assert!(dir.join("not-a-log.txt").exists(), "non-log untouched");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn crash_marker_is_written_then_cleared_once() {
        let app_data = tmp();
        let dir = logs_dir(&app_data);
        let _ = fs::create_dir_all(&dir);
        // Exercise the writer + marker directly (installing a global panic hook
        // in a test would fight the harness's own hook).
        append_line(&dir, "[t] PANIC at x:1: boom");
        mark_crash(&dir);
        let day = day_of(&now_iso());
        let log = fs::read_to_string(dir.join(format!("vidya-{day}.log"))).unwrap();
        assert!(log.contains("PANIC at x:1: boom"));
        assert!(dir.join(CRASH_MARKER).exists());
        // One-shot: first read is true and clears it, second read is false.
        assert!(take_last_crash(&app_data), "crash detected on next launch");
        assert!(!take_last_crash(&app_data), "marker cleared after reading");
        let _ = fs::remove_dir_all(&app_data);
    }
}
