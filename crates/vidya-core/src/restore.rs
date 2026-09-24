//! restore — pure restore-validation rules (docs/00-SYSTEM-CONTEXT.md §12,
//! prompts/P08 Part C). No IO, no clock, no floats.
//!
//! The actual restore (open the `.vbak` with the backup key, `integrity_check`,
//! row counts, verify the audit chain, stage into a temp DB and swap atomically,
//! bump the epoch) lives in `src-tauri/src/backup/restore.rs`. This module holds
//! the pure decisions around it:
//!
//! * [`validate_restore`] — the summary must pass its checks before installing;
//! * [`staleness_warning`] — warn when the chosen backup is older than a day;
//! * [`recovery_lockout_seconds`] — the recovery-key retry backoff (3 wrong → 30 s);
//! * [`days_between`] — pure calendar-day arithmetic used above (and reusable).

use crate::errors::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

/// The summary shown before a restore is installed (§12): school, backup date,
/// counts, last receipt number, and whether the backup's audit chain verified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreSummary {
    pub school_name: String,
    /// The backup's own date, `YYYY-MM-DD`.
    pub backup_date: String,
    pub students: u64,
    pub payments: u64,
    /// The highest receipt number recorded in the backup, if any.
    pub last_receipt_no: Option<String>,
    /// Whether the backup's audit chain verified end-to-end.
    pub chain_ok: bool,
}

/// Validate a restore summary before installing it (§12). A restore is refused if
/// the backup's audit chain does not verify — a tampered/corrupt backup must never
/// silently become the live record (§9, honest status).
///
/// Returns `Err(Validation{ field: "audit_chain", rule: "broken" })` on a bad
/// chain; `Ok(())` otherwise. (An empty backup — 0 students — is allowed: a brand
/// new school can be legitimately restored.)
pub fn validate_restore(summary: &RestoreSummary) -> CoreResult<()> {
    if !summary.chain_ok {
        return Err(CoreError::validation("audit_chain", "broken"));
    }
    Ok(())
}

/// A staleness warning for a chosen backup (§12): "Receipts after `<date>`
/// recorded on this computer will be missing."
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StalenessWarning {
    /// The backup date to show in the message (`YYYY-MM-DD`).
    pub backup_date: String,
    /// Whole calendar days between the backup and today.
    pub days_old: i64,
}

/// Warn when the chosen backup is **older than one day** (§12). Returns the
/// warning (with the backup date + age in days) when `today` is at least one
/// calendar day after `backup_date`; `None` when the backup is from today (or
/// dated in the future, which is never "stale").
pub fn staleness_warning(backup_date: &str, today: &str) -> Option<StalenessWarning> {
    let days = days_between(backup_date, today)?;
    if days >= 1 {
        Some(StalenessWarning { backup_date: backup_date.to_string(), days_old: days })
    } else {
        None
    }
}

/// The recovery-key retry backoff on the restore screen (§Part C: "3 wrong → 30 s
/// wait"). `None` before the third wrong attempt; then a 30-second wait on the
/// third and each subsequent wrong attempt. Mirrors [`crate`]'s PIN backoff shape
/// (the PIN lives in `src-tauri`), but with the recovery-key threshold + fixed
/// wait the prompt specifies.
pub fn recovery_lockout_seconds(fail_count: u32) -> Option<u64> {
    if fail_count < 3 {
        None
    } else {
        Some(30)
    }
}

/// Tries remaining before the recovery-key input imposes its 30-second wait.
pub fn recovery_tries_remaining(fail_count: u32) -> u32 {
    3u32.saturating_sub(fail_count)
}

/// Whole calendar days from `from` to `to`, each a `YYYY-MM-DD` string
/// (`to - from`; negative if `to` precedes `from`). `None` if either date does
/// not parse. Uses Howard Hinnant's `days_from_civil` — pure integer math, no
/// clock, no floats, correct across month/year boundaries.
pub fn days_between(from: &str, to: &str) -> Option<i64> {
    Some(days_from_civil(parse_ymd(to)?) - days_from_civil(parse_ymd(from)?))
}

/// Parse a strict `YYYY-MM-DD` date into `(year, month, day)`. Rejects any other
/// shape, non-digits, or an out-of-range month/day.
fn parse_ymd(s: &str) -> Option<(i64, u32, u32)> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    if !b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit()) {
        return None;
    }
    let y: i64 = s[0..4].parse().ok()?;
    let m: u32 = s[5..7].parse().ok()?;
    let d: u32 = s[8..10].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

/// Days since the Unix epoch (1970-01-01) for a proleptic-Gregorian date.
/// Howard Hinnant, *chrono-Compatible Low-Level Date Algorithms*.
fn days_from_civil((y, m, d): (i64, u32, u32)) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let m = m as i64;
    let d = d as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146097 + doe - 719468
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(chain_ok: bool) -> RestoreSummary {
        RestoreSummary {
            school_name: "Saraswati Public School".into(),
            backup_date: "2026-09-23".into(),
            students: 670,
            payments: 40,
            last_receipt_no: Some("R-A2-0418".into()),
            chain_ok,
        }
    }

    // ---- validate_restore ----

    #[test]
    fn good_chain_restores() {
        assert!(validate_restore(&summary(true)).is_ok());
    }

    #[test]
    fn broken_chain_is_refused() {
        assert_eq!(
            validate_restore(&summary(false)),
            Err(CoreError::validation("audit_chain", "broken"))
        );
    }

    #[test]
    fn empty_but_valid_backup_is_allowed() {
        let mut s = summary(true);
        s.students = 0;
        s.payments = 0;
        s.last_receipt_no = None;
        assert!(validate_restore(&s).is_ok());
    }

    // ---- staleness_warning ----

    #[test]
    fn same_day_is_not_stale() {
        assert_eq!(staleness_warning("2026-09-23", "2026-09-23"), None);
    }

    #[test]
    fn yesterday_is_stale_one_day() {
        let w = staleness_warning("2026-09-22", "2026-09-23").unwrap();
        assert_eq!(w.backup_date, "2026-09-22");
        assert_eq!(w.days_old, 1);
    }

    #[test]
    fn across_month_boundary() {
        // 2026-08-31 → 2026-09-02 is 2 days.
        let w = staleness_warning("2026-08-31", "2026-09-02").unwrap();
        assert_eq!(w.days_old, 2);
    }

    #[test]
    fn future_backup_is_not_stale() {
        assert_eq!(staleness_warning("2026-09-24", "2026-09-23"), None);
    }

    #[test]
    fn bad_date_yields_no_warning() {
        assert_eq!(staleness_warning("not-a-date", "2026-09-23"), None);
    }

    // ---- recovery backoff ----

    #[test]
    fn recovery_backoff_curve() {
        assert_eq!(recovery_lockout_seconds(0), None);
        assert_eq!(recovery_lockout_seconds(1), None);
        assert_eq!(recovery_lockout_seconds(2), None);
        assert_eq!(recovery_lockout_seconds(3), Some(30));
        assert_eq!(recovery_lockout_seconds(4), Some(30));
        assert_eq!(recovery_lockout_seconds(100), Some(30));
    }

    #[test]
    fn recovery_tries_remaining_counts_down() {
        assert_eq!(recovery_tries_remaining(0), 3);
        assert_eq!(recovery_tries_remaining(2), 1);
        assert_eq!(recovery_tries_remaining(3), 0);
        assert_eq!(recovery_tries_remaining(9), 0);
    }

    // ---- days_between ----

    #[test]
    fn days_between_basics() {
        assert_eq!(days_between("2026-09-23", "2026-09-23"), Some(0));
        assert_eq!(days_between("2026-09-23", "2026-09-24"), Some(1));
        assert_eq!(days_between("2026-09-24", "2026-09-23"), Some(-1));
        // A full non-leap year.
        assert_eq!(days_between("2025-01-01", "2026-01-01"), Some(365));
        // 2024 is a leap year.
        assert_eq!(days_between("2024-01-01", "2025-01-01"), Some(366));
    }

    #[test]
    fn days_between_rejects_bad_dates() {
        assert_eq!(days_between("2026-13-01", "2026-09-23"), None);
        assert_eq!(days_between("2026-09-23", "2026-09-32"), None);
        assert_eq!(days_between("2026/09/23", "2026-09-23"), None);
        assert_eq!(days_between("", "2026-09-23"), None);
    }

    #[test]
    fn days_from_civil_epoch_anchor() {
        // 1970-01-01 is day 0 by definition.
        assert_eq!(days_from_civil((1970, 1, 1)), 0);
        assert_eq!(days_from_civil((1969, 12, 31)), -1);
    }
}
