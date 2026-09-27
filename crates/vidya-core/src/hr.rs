//! hr — Staff HR rules (P17, §10.5). Pure Rust: no IO, no async, no clock reads
//! (times/dates are passed in), no randomness, no floats.
//!
//! Staff mark their own attendance (check-in/out) on their phones; the Principal
//! sets the HR policy (start time, grace, whether an away check-in is allowed) and
//! approves leave. This module holds the pure decisions those need:
//!   * [`check_in_status`] — Present / Late / AwayPending from the check-in time,
//!     the [`HrSettings`] and the network [`Route`];
//!   * [`clock_warning`] — the "phone clock differs" flag (> 10 min skew);
//!   * leave balances, unpaid spill-over and overlap ([`leave_balance`],
//!     [`split_leave_days`], [`overlaps_any`]);
//!   * [`days_present`] — the salary-day count from HR data (the P15 link).
//!
//! Times are **minutes since midnight** ([`parse_hhmm`]); dates are `YYYY-MM-DD`
//! strings (lexicographic compare, so no parser is needed for ranges). `src-tauri`
//! passes the server's date/time and the settings in.

use crate::errors::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

/// How a check-in reached the school (§10.5). `Lan` = on the school Wi-Fi (the
/// device reached the school server directly) and counts as "at school"; `Drive`
/// (reached only through Google Drive) and `Manual` (a Principal entry) are away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    Lan,
    Drive,
    Manual,
}

impl Route {
    pub fn as_key(self) -> &'static str {
        match self {
            Route::Lan => "lan",
            Route::Drive => "drive",
            Route::Manual => "manual",
        }
    }
    pub fn from_key(s: &str) -> Option<Route> {
        match s {
            "lan" => Some(Route::Lan),
            "drive" => Some(Route::Drive),
            "manual" => Some(Route::Manual),
            _ => None,
        }
    }
}

/// A staff-attendance status for one day (§10.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttnStatus {
    Present,
    Late,
    /// Checked in away from school while away is not allowed — waits for the
    /// Principal to accept or reject.
    AwayPending,
    Absent,
    Leave,
    HalfDay,
}

impl AttnStatus {
    pub fn as_key(self) -> &'static str {
        match self {
            AttnStatus::Present => "present",
            AttnStatus::Late => "late",
            AttnStatus::AwayPending => "away_pending",
            AttnStatus::Absent => "absent",
            AttnStatus::Leave => "leave",
            AttnStatus::HalfDay => "half_day",
        }
    }
}

/// Minutes since midnight for the default school start (09:00, **[OWNER default]**).
pub const DEFAULT_START_MIN: u16 = 9 * 60;
/// The clock-skew flag threshold in minutes (§10.5: "phone clock differs" > 10 min).
pub const CLOCK_SKEW_LIMIT_MIN: i64 = 10;

/// Staff-HR settings (Settings → Staff HR, §10.5). Times are minutes since
/// midnight so the check-in rule stays clockless.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HrSettings {
    /// School start time (default 09:00 = 540). A check-in after start + grace is late.
    pub start_min: u16,
    /// Late grace minutes (default 0).
    pub grace_min: u16,
    /// Allow a check-in away from school (default off → away check-ins wait for
    /// the Principal).
    pub allow_away: bool,
}

impl Default for HrSettings {
    /// **[OWNER defaults]**: start 09:00, grace 0, away check-in not allowed.
    fn default() -> Self {
        HrSettings { start_min: DEFAULT_START_MIN, grace_min: 0, allow_away: false }
    }
}

/// Parse a strict 24-hour `HH:MM` into minutes since midnight (`00:00`..=`23:59`).
pub fn parse_hhmm(s: &str) -> Option<u16> {
    let (h, m) = s.split_once(':')?;
    if h.len() != 2 || m.len() != 2 {
        return None;
    }
    let h: u16 = h.parse().ok()?;
    let m: u16 = m.parse().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
}

/// Format minutes since midnight as `HH:MM` (24-hour).
pub fn fmt_hhmm(min: u16) -> String {
    format!("{:02}:{:02}", (min / 60) % 24, min % 60)
}

/// Validate the HR settings: a valid start time and a sane grace window.
pub fn validate_hr_settings(start_min: u16, grace_min: u16) -> CoreResult<()> {
    if start_min > 23 * 60 + 59 {
        return Err(CoreError::validation("start_time", "range"));
    }
    if grace_min > 180 {
        return Err(CoreError::validation("grace", "range"));
    }
    Ok(())
}

/// The status of a check-in at `check_in_min` (minutes since midnight) given the
/// settings and the route (§10.5):
///   * an away route (anything but LAN) when away is not allowed → `AwayPending`
///     (the Principal must accept it);
///   * otherwise `Late` when after `start + grace`, else `Present`.
pub fn check_in_status(check_in_min: u16, settings: &HrSettings, route: Route) -> AttnStatus {
    if route != Route::Lan && !settings.allow_away {
        return AttnStatus::AwayPending;
    }
    let cutoff = settings.start_min as u32 + settings.grace_min as u32;
    if check_in_min as u32 > cutoff {
        AttnStatus::Late
    } else {
        AttnStatus::Present
    }
}

/// The status an accepted away check-in becomes (§10.5): the Principal accepting
/// an `away_pending` check-in resolves it to Present/Late by the same time rule.
pub fn accepted_away_status(check_in_min: u16, settings: &HrSettings) -> AttnStatus {
    let cutoff = settings.start_min as u32 + settings.grace_min as u32;
    if check_in_min as u32 > cutoff {
        AttnStatus::Late
    } else {
        AttnStatus::Present
    }
}

/// Absolute minutes between the device's claimed time and the server time.
pub fn clock_skew_minutes(device_ms: i64, server_ms: i64) -> i64 {
    (device_ms - server_ms).abs() / 60_000
}

/// The Principal-facing "phone clock differs" flag: the device/server difference
/// exceeds [`CLOCK_SKEW_LIMIT_MIN`] (§10.5).
pub fn clock_warning(device_ms: i64, server_ms: i64) -> bool {
    clock_skew_minutes(device_ms, server_ms) > CLOCK_SKEW_LIMIT_MIN
}

/// Remaining balance for a leave type this session: `quota − approved`. A `None`
/// quota means the type is unlimited (an Unpaid type has no quota) → `None`.
pub fn leave_balance(yearly_quota: Option<u32>, approved_days: u32) -> Option<i64> {
    yearly_quota.map(|q| q as i64 - approved_days as i64)
}

/// A leave request split into paid and unpaid days.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaveSplit {
    pub paid_days: u32,
    pub unpaid_days: u32,
}

/// Split a `requested`-day leave into paid + unpaid (§10.5). An unpaid-type leave
/// is entirely unpaid; a paid type pays up to the remaining balance and spills the
/// rest over to Unpaid (the UI message says so). A `None` balance on a paid type
/// (an unlimited paid type — not a default) pays all days.
pub fn split_leave_days(requested: u32, paid_type: bool, balance: Option<i64>) -> LeaveSplit {
    if !paid_type {
        return LeaveSplit { paid_days: 0, unpaid_days: requested };
    }
    match balance {
        None => LeaveSplit { paid_days: requested, unpaid_days: 0 },
        Some(b) => {
            let avail = b.max(0).min(requested as i64) as u32;
            LeaveSplit { paid_days: avail, unpaid_days: requested - avail }
        }
    }
}

/// Do two inclusive `YYYY-MM-DD` date ranges overlap? (String compare is a valid
/// date order for this fixed format.)
pub fn ranges_overlap(a_from: &str, a_to: &str, b_from: &str, b_to: &str) -> bool {
    a_from <= b_to && b_from <= a_to
}

/// Does `[from, to]` overlap any of the `existing` (from, to) ranges?
pub fn overlaps_any(existing: &[(String, String)], from: &str, to: &str) -> bool {
    existing.iter().any(|(f, t)| ranges_overlap(from, to, f, t))
}

/// Validate a leave request's dates: `from <= to` (both `YYYY-MM-DD`).
pub fn validate_leave_dates(from: &str, to: &str) -> CoreResult<()> {
    if from > to {
        return Err(CoreError::validation("dates", "from_after_to"));
    }
    Ok(())
}

/// Days present for the salary register from HR data (§10.5, the P15 link):
/// `working_days − absent − unpaid_leave`. Paid leave and a not-yet-accepted
/// `away_pending` both count as present (the register flags away_pending
/// separately); only `absent` days and **unpaid**-leave days reduce the count.
pub fn days_present(working_days: u32, absent_days: u32, unpaid_leave_days: u32) -> u32 {
    working_days.saturating_sub(absent_days + unpaid_leave_days)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(start: &str, grace: u16, allow_away: bool) -> HrSettings {
        HrSettings { start_min: parse_hhmm(start).unwrap(), grace_min: grace, allow_away }
    }

    #[test]
    fn parse_and_format_hhmm() {
        assert_eq!(parse_hhmm("09:00"), Some(540));
        assert_eq!(parse_hhmm("08:47"), Some(527));
        assert_eq!(parse_hhmm("23:59"), Some(1439));
        assert_eq!(parse_hhmm("00:00"), Some(0));
        // Strict: no single-digit, no out-of-range, no junk.
        assert_eq!(parse_hhmm("9:00"), None);
        assert_eq!(parse_hhmm("24:00"), None);
        assert_eq!(parse_hhmm("09:60"), None);
        assert_eq!(parse_hhmm(""), None);
        assert_eq!(fmt_hhmm(540), "09:00");
        assert_eq!(fmt_hhmm(527), "08:47");
    }

    #[test]
    fn lan_check_in_before_start_is_present() {
        // Prototype `staffday`: checked in 8:47 on school Wi-Fi before the 9:00 bell.
        let s = settings("09:00", 0, false);
        assert_eq!(check_in_status(parse_hhmm("08:47").unwrap(), &s, Route::Lan), AttnStatus::Present);
    }

    #[test]
    fn lan_check_in_after_start_plus_grace_is_late() {
        let s = settings("09:00", 0, false);
        assert_eq!(check_in_status(parse_hhmm("09:01").unwrap(), &s, Route::Lan), AttnStatus::Late);
        // Exactly at start is on time.
        assert_eq!(check_in_status(parse_hhmm("09:00").unwrap(), &s, Route::Lan), AttnStatus::Present);
        // With a 10-minute grace, 09:10 is still on time, 09:11 is late.
        let g = settings("09:00", 10, false);
        assert_eq!(check_in_status(parse_hhmm("09:10").unwrap(), &g, Route::Lan), AttnStatus::Present);
        assert_eq!(check_in_status(parse_hhmm("09:11").unwrap(), &g, Route::Lan), AttnStatus::Late);
    }

    #[test]
    fn away_route_needs_principal_when_away_not_allowed() {
        // Default: away check-in not allowed → away_pending regardless of time.
        let s = settings("09:00", 0, false);
        assert_eq!(check_in_status(parse_hhmm("08:30").unwrap(), &s, Route::Drive), AttnStatus::AwayPending);
        assert_eq!(check_in_status(parse_hhmm("10:00").unwrap(), &s, Route::Manual), AttnStatus::AwayPending);
        // Allowing away check-ins turns them into the plain time rule.
        let allow = settings("09:00", 0, true);
        assert_eq!(check_in_status(parse_hhmm("08:30").unwrap(), &allow, Route::Drive), AttnStatus::Present);
        assert_eq!(check_in_status(parse_hhmm("10:00").unwrap(), &allow, Route::Drive), AttnStatus::Late);
    }

    #[test]
    fn accepting_an_away_check_in_resolves_by_time() {
        let s = settings("09:00", 0, false);
        assert_eq!(accepted_away_status(parse_hhmm("08:50").unwrap(), &s), AttnStatus::Present);
        assert_eq!(accepted_away_status(parse_hhmm("09:30").unwrap(), &s), AttnStatus::Late);
    }

    #[test]
    fn clock_skew_flag() {
        let base = 1_700_000_000_000i64;
        assert!(!clock_warning(base, base));
        assert!(!clock_warning(base + 10 * 60_000, base)); // exactly 10 min — not over
        assert!(clock_warning(base + 11 * 60_000, base)); // 11 min → flagged
        assert!(clock_warning(base - 15 * 60_000, base)); // behind by 15 min → flagged
        assert_eq!(clock_skew_minutes(base + 11 * 60_000, base), 11);
    }

    #[test]
    fn leave_balance_and_unpaid_spillover() {
        // Casual quota 12, 3 approved → 9 left (matches the prototype "9 of 12").
        assert_eq!(leave_balance(Some(12), 3), Some(9));
        // Unpaid type has no quota → unlimited.
        assert_eq!(leave_balance(None, 5), None);

        // A 2-day paid request within a balance of 9 → all paid.
        assert_eq!(split_leave_days(2, true, Some(9)), LeaveSplit { paid_days: 2, unpaid_days: 0 });
        // A 5-day paid request with only 3 left → 3 paid + 2 unpaid (spill-over).
        assert_eq!(split_leave_days(5, true, Some(3)), LeaveSplit { paid_days: 3, unpaid_days: 2 });
        // No balance left → all unpaid.
        assert_eq!(split_leave_days(2, true, Some(0)), LeaveSplit { paid_days: 0, unpaid_days: 2 });
        assert_eq!(split_leave_days(2, true, Some(-1)), LeaveSplit { paid_days: 0, unpaid_days: 2 });
        // An Unpaid-type request is entirely unpaid regardless of any balance.
        assert_eq!(split_leave_days(3, false, None), LeaveSplit { paid_days: 0, unpaid_days: 3 });
    }

    #[test]
    fn overlap_detection() {
        assert!(ranges_overlap("2026-09-24", "2026-09-25", "2026-09-25", "2026-09-26"));
        assert!(!ranges_overlap("2026-09-24", "2026-09-25", "2026-09-26", "2026-09-27"));
        let existing = vec![("2026-09-24".to_string(), "2026-09-25".to_string())];
        assert!(overlaps_any(&existing, "2026-09-25", "2026-09-28"));
        assert!(!overlaps_any(&existing, "2026-09-26", "2026-09-28"));
        assert!(validate_leave_dates("2026-09-24", "2026-09-25").is_ok());
        assert!(validate_leave_dates("2026-09-25", "2026-09-24").is_err());
    }

    #[test]
    fn salary_days_present_from_hr() {
        // 26 working days, 2 absent, 1 unpaid-leave day → 23 present.
        assert_eq!(days_present(26, 2, 1), 23);
        // Paid leave is not passed as unpaid, so it does not reduce the count.
        assert_eq!(days_present(26, 0, 0), 26);
        // Never underflows.
        assert_eq!(days_present(20, 15, 15), 0);
    }
}
