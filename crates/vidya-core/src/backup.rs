//! backup — pure backup rules (docs/00-SYSTEM-CONTEXT.md §12, prompts/P08 Part B).
//!
//! No IO, no clock reads, no floats. The SQLCipher export, the on-disk verify and
//! the Drive upload live in `src-tauri/src/backup/`; this module holds the pure
//! decisions those steps depend on:
//!
//! * [`select_for_deletion`] — retention (30 daily + 12 monthly per destination);
//! * [`slugify`] + [`backup_filename`] — the `vidya-<slug>-YYYYMMDD-HHMM.vbak` name;
//! * [`backup_outcome`] — the Verified / Partial / Failed status a run gets from
//!   whether the local copy and the Drive copy each verified.

use serde::{Deserialize, Serialize};

/// How many distinct daily backups to keep per destination (§12).
pub const DAILY_KEEP: usize = 30;
/// How many distinct monthly backups to keep per destination (§12).
pub const MONTHLY_KEEP: usize = 12;

/// One backup run, reduced to the fields retention needs.
///
/// `at` is a sortable timestamp string (RFC-3339, e.g. `2026-09-23T06:02:00Z`);
/// its first 10 chars are the day (`YYYY-MM-DD`) and its first 7 the month
/// (`YYYY-MM`). `destination` groups runs so retention runs independently per
/// place a copy lives (e.g. `"local"`, `"drive"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRecord {
    pub id: String,
    pub at: String,
    pub destination: String,
}

impl BackupRecord {
    fn day(&self) -> &str {
        self.at.get(0..10).unwrap_or(&self.at)
    }
    fn month(&self) -> &str {
        self.at.get(0..7).unwrap_or(&self.at)
    }
}

/// Choose which backup runs to delete, applying "keep 30 daily + 12 monthly" per
/// destination (§12). Returns the ids to delete, in the input order.
///
/// A run is **kept** in a destination if it is either:
/// * the newest run of its day, among the [`DAILY_KEEP`] most recent days that
///   have any backup (the daily tier); OR
/// * the newest run of its month, among the [`MONTHLY_KEEP`] most recent months
///   that have any backup (the monthly tier).
///
/// Everything else in that destination is deleted. This is count-based
/// grandfather-father-son retention: the school never loses its most recent 30
/// days or 12 months of history even if backups are sparse. `today` (a
/// `YYYY-MM-DD` string) drops any run dated after today (defensive against a
/// wrong clock) — those are never deleted here, only ignored.
///
/// **[OWNER]** default: count-based (the 30 most recent backup-days / 12 most
/// recent backup-months), not a fixed 30-day / 12-month calendar window.
pub fn select_for_deletion(runs: &[BackupRecord], today: &str) -> Vec<String> {
    // Distinct destinations, first-seen order (only to make the keep-set build
    // deterministic; the returned deletes follow input order regardless).
    let mut destinations: Vec<&str> = Vec::new();
    for r in runs {
        if !destinations.contains(&r.destination.as_str()) {
            destinations.push(&r.destination);
        }
    }

    let mut keep: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for dest in destinations {
        // Runs for this destination, on-or-before today, newest first.
        let mut group: Vec<&BackupRecord> = runs
            .iter()
            .filter(|r| r.destination == dest && r.day() <= today)
            .collect();
        group.sort_by(|a, b| b.at.cmp(&a.at).then_with(|| b.id.cmp(&a.id)));

        keep_tier(&group, DAILY_KEEP, BackupRecord::day, &mut keep);
        keep_tier(&group, MONTHLY_KEEP, BackupRecord::month, &mut keep);
    }

    runs.iter()
        .filter(|r| r.day() <= today && !keep.contains(r.id.as_str()))
        .map(|r| r.id.clone())
        .collect()
}

/// Keep the newest run of each distinct bucket (day or month), for the most
/// recent `limit` buckets. `group` must be sorted newest-first.
fn keep_tier<'a>(
    group: &[&'a BackupRecord],
    limit: usize,
    bucket: fn(&BackupRecord) -> &str,
    keep: &mut std::collections::HashSet<&'a str>,
) {
    let mut seen_buckets: Vec<&str> = Vec::new();
    for r in group {
        let b = bucket(r);
        if seen_buckets.contains(&b) {
            continue; // an earlier (newer) run already represents this bucket
        }
        if seen_buckets.len() >= limit {
            break; // past the most-recent `limit` buckets
        }
        seen_buckets.push(b);
        keep.insert(r.id.as_str());
    }
}

/// A filesystem-safe slug for a school name: lowercase ASCII alphanumerics, with
/// every other run of characters collapsed to a single `-`, trimmed of leading and
/// trailing `-`. Non-ASCII letters (e.g. Devanagari) are dropped, so a school with
/// a purely non-Latin name yields `"school"` as a stable fallback.
///
/// ```
/// use vidya_core::backup::slugify;
/// assert_eq!(slugify("Saraswati Public School"), "saraswati-public-school");
/// assert_eq!(slugify("  St. Mary's  "), "st-mary-s");
/// ```
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "school".to_string()
    } else {
        out
    }
}

/// The backup filename `vidya-<slug>-YYYYMMDD-HHMM.vbak` (§12).
///
/// `at` is an RFC-3339 timestamp; its fixed positions supply the date and time
/// components (this is pure string slicing, not a clock read). A malformed `at`
/// falls back to zeros so a filename is always produced.
///
/// ```
/// use vidya_core::backup::backup_filename;
/// assert_eq!(
///     backup_filename("saraswati-public-school", "2026-09-23T06:02:11Z"),
///     "vidya-saraswati-public-school-20260923-0602.vbak"
/// );
/// ```
pub fn backup_filename(slug: &str, at: &str) -> String {
    let g = |a: usize, b: usize| at.get(a..b).filter(|s| s.bytes().all(|x| x.is_ascii_digit()));
    let y = g(0, 4).unwrap_or("0000");
    let mo = g(5, 7).unwrap_or("00");
    let d = g(8, 10).unwrap_or("00");
    let h = g(11, 13).unwrap_or("00");
    let mi = g(14, 16).unwrap_or("00");
    format!("vidya-{slug}-{y}{mo}{d}-{h}{mi}.vbak")
}

/// Whether the Drive copy of a backup could be verified (§12; Part B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriveState {
    /// Uploaded and the checksum verified.
    Verified,
    /// Drive is configured but the upload/verify failed (offline, quota, …).
    Unavailable,
    /// No Google Drive is set up for this school.
    NotConfigured,
}

/// The status pill a backup run earns (§12; Part B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupOutcome {
    /// Local copy verified AND the Drive copy verified.
    Verified,
    /// Local copy verified, but no verified Drive copy (unavailable / not set up).
    Partial,
    /// The local copy could not be produced or verified — nothing safe was written.
    Failed,
}

/// Decide a run's [`BackupOutcome`] from the local- and Drive-verify results (§12).
///
/// * local not verified → `Failed` (Home never shows a success time — §12, no fake
///   "backed up");
/// * local verified + Drive `Verified` → `Verified` ("this PC and school Drive");
/// * local verified + Drive `Unavailable`/`NotConfigured` → `Partial` (local only).
///
/// **[OWNER]** default: a school with no Drive at all still reads `Partial`
/// (local-only). Confirm whether such a school should instead read `Verified`.
pub fn backup_outcome(local_verified: bool, drive: DriveState) -> BackupOutcome {
    if !local_verified {
        return BackupOutcome::Failed;
    }
    match drive {
        DriveState::Verified => BackupOutcome::Verified,
        DriveState::Unavailable | DriveState::NotConfigured => BackupOutcome::Partial,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: &str, at: &str, dest: &str) -> BackupRecord {
        BackupRecord { id: id.into(), at: at.into(), destination: dest.into() }
    }

    // ---- select_for_deletion ----

    #[test]
    fn keeps_one_per_day_up_to_thirty_days() {
        // 40 daily backups (one per day, valid calendar dates), one destination.
        // Days 1..=31 in July, then Aug 1..=9. Keep the newest 30 days.
        let mut runs = Vec::new();
        for i in 0..40 {
            let (mo, day) = if i < 31 { (7, i + 1) } else { (8, i - 31 + 1) };
            let at = format!("2026-{mo:02}-{day:02}T06:00:00Z");
            runs.push(rec(&format!("r{i}"), &at, "local"));
        }
        let del = select_for_deletion(&runs, "2026-08-09");
        // 40 distinct days → delete the 10 oldest (r0..r9).
        assert_eq!(del.len(), 10);
        for i in 0..10 {
            assert!(del.contains(&format!("r{i}")), "oldest day r{i} should be deleted");
        }
        for i in 10..40 {
            assert!(!del.contains(&format!("r{i}")), "recent day r{i} should be kept");
        }
    }

    #[test]
    fn newest_run_of_the_day_is_kept_older_same_day_deleted() {
        let runs = [
            rec("morning", "2026-09-23T06:00:00Z", "local"),
            rec("evening", "2026-09-23T21:00:00Z", "local"),
        ];
        let del = select_for_deletion(&runs, "2026-09-23");
        // Same day: keep the evening (newest), delete the morning.
        assert_eq!(del, vec!["morning".to_string()]);
    }

    #[test]
    fn monthly_tier_preserves_old_snapshot_beyond_daily_window() {
        // 31 daily backups across December (saturates the 30-day daily tier), plus
        // one old June snapshot far outside the daily window.
        let mut runs = Vec::new();
        for d in 1..=31 {
            runs.push(rec(&format!("d{d}"), &format!("2026-12-{d:02}T06:00:00Z"), "drive"));
        }
        runs.push(rec("jun", "2026-06-30T06:00:00Z", "drive"));
        let del = select_for_deletion(&runs, "2026-12-31");
        // Daily tier keeps the 30 newest days (Dec 2..31); Dec 1 falls out and
        // December's month-representative is Dec 31, so Dec 1 is deleted. June
        // survives only via the monthly tier (2 months present, < 12 kept).
        assert_eq!(del, vec!["d1".to_string()]);
        assert!(!del.contains(&"jun".to_string()), "old monthly snapshot must survive");
    }

    #[test]
    fn monthly_tier_caps_at_twelve_months() {
        let mut runs = Vec::new();
        // Saturate the daily tier: 31 daily backups in 2026-08 (the newest month,
        // month-rank 1). All old snapshot days end up outside the daily tier.
        for d in 1..=31 {
            runs.push(rec(&format!("aug{d}"), &format!("2026-08-{d:02}T06:00:00Z"), "local"));
        }
        // One month-end snapshot per month for the 2nd..15th most-recent months
        // (2026-07 back to 2025-06).
        for rank in 2..=15u32 {
            let idx: i64 = 2026 * 12 + 7 - (rank as i64 - 1); // year*12 + (month-1)
            let (year, month) = (idx / 12, idx % 12 + 1);
            runs.push(rec(&format!("rank{rank}"), &format!("{year}-{month:02}-28T06:00:00Z"), "local"));
        }
        let del = select_for_deletion(&runs, "2026-08-31");
        // The monthly tier keeps the 12 most-recent months (rank 1 = the daily
        // month, ranks 2..12 = old snapshots); ranks 13/14/15 fall off.
        assert!(!del.contains(&"rank2".to_string()), "most recent old month kept");
        assert!(!del.contains(&"rank12".to_string()), "12th month kept");
        assert!(del.contains(&"rank13".to_string()), "13th month dropped");
        assert!(del.contains(&"rank14".to_string()));
        assert!(del.contains(&"rank15".to_string()));
    }

    #[test]
    fn daily_and_monthly_tiers_union() {
        // 30 recent daily backups in Sep 2026 + one old monthly (2026-03).
        let mut runs = Vec::new();
        for d in 1..=30 {
            runs.push(rec(&format!("d{d}"), &format!("2026-09-{d:02}T06:00:00Z"), "local"));
        }
        runs.push(rec("old", "2026-03-15T06:00:00Z", "local"));
        let del = select_for_deletion(&runs, "2026-09-30");
        // 30 daily fill the daily tier; the old March run is kept by the monthly
        // tier (< 12 months present) → nothing deleted.
        assert!(del.is_empty(), "old monthly kept by the monthly tier: {del:?}");
    }

    #[test]
    fn retention_is_per_destination() {
        // Two destinations each with 2 same-month days; each keeps its own tiers.
        let runs = [
            rec("l1", "2026-09-01T06:00:00Z", "local"),
            rec("l2", "2026-09-02T06:00:00Z", "local"),
            rec("d1", "2026-09-01T06:00:00Z", "drive"),
            rec("d2", "2026-09-02T06:00:00Z", "drive"),
        ];
        let del = select_for_deletion(&runs, "2026-09-30");
        // Two days per destination, both within tiers → nothing deleted.
        assert!(del.is_empty());
    }

    #[test]
    fn future_dated_runs_are_ignored_never_deleted() {
        let runs = [
            rec("past", "2026-09-01T06:00:00Z", "local"),
            rec("future", "2099-01-01T06:00:00Z", "local"),
        ];
        let del = select_for_deletion(&runs, "2026-09-23");
        assert!(del.is_empty(), "a future-dated run is ignored, not deleted");
    }

    #[test]
    fn empty_input_deletes_nothing() {
        assert!(select_for_deletion(&[], "2026-09-23").is_empty());
    }

    // ---- slugify ----

    #[test]
    fn slugify_examples() {
        assert_eq!(slugify("Saraswati Public School"), "saraswati-public-school");
        assert_eq!(slugify("  St. Mary's  "), "st-mary-s");
        assert_eq!(slugify("A/B & C"), "a-b-c");
        assert_eq!(slugify("---"), "school");
        assert_eq!(slugify("सरस्वती"), "school"); // no ASCII letters → fallback
        assert_eq!(slugify("Vidya 2026"), "vidya-2026");
    }

    // ---- backup_filename ----

    #[test]
    fn filename_format() {
        assert_eq!(
            backup_filename("saraswati-public-school", "2026-09-23T06:02:11Z"),
            "vidya-saraswati-public-school-20260923-0602.vbak"
        );
    }

    #[test]
    fn filename_falls_back_on_bad_timestamp() {
        assert_eq!(backup_filename("x", "not-a-time"), "vidya-x-00000000-0000.vbak");
    }

    // ---- backup_outcome ----

    #[test]
    fn outcome_matrix() {
        assert_eq!(backup_outcome(true, DriveState::Verified), BackupOutcome::Verified);
        assert_eq!(backup_outcome(true, DriveState::Unavailable), BackupOutcome::Partial);
        assert_eq!(backup_outcome(true, DriveState::NotConfigured), BackupOutcome::Partial);
        assert_eq!(backup_outcome(false, DriveState::Verified), BackupOutcome::Failed);
        assert_eq!(backup_outcome(false, DriveState::Unavailable), BackupOutcome::Failed);
    }
}
