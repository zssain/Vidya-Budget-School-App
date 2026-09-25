//! calendar — school-calendar rules (P13; 00-SYSTEM-CONTEXT §8.1 / foundation §8.1).
//!
//! Pure Rust: no IO, no async, no clock reads (dates are passed in), no floats.
//! The calendar is two things:
//!   * a **weekly pattern** ([`SchoolWeek`]) — which weekdays are working days
//!     (default Mon–Sat working, Sunday off); and
//!   * dated **events** ([`CalendarEvent`]) — holidays / exams / events, each of
//!     which may or may not be a **non-working** day.
//!
//! [`is_working_day`] answers the one question every module needs: is school open
//! on this date? A non-working event (e.g. a holiday) makes the day non-working;
//! otherwise the weekly pattern decides. Events never make an off weekday working
//! (a Sunday exam is still a non-working day for attendance purposes) — that is a
//! deliberate simplification and matches how attendance/fee due dates use it.
//!
//! Used by: attendance (no sheet is required on a non-working day; dashboards
//! ignore them), fee due dates (only to show a "due on next working day" note —
//! money dates never move), salary days (P15/P17) and leave day counts (P17).

use serde::{Deserialize, Serialize};
use time::{Date, Weekday};

/// The seven-day working pattern. `working[i]` is true when the weekday whose
/// [`weekday_index`] is `i` is a working day (Monday = 0 … Sunday = 6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchoolWeek {
    /// Working flags indexed Monday = 0 … Sunday = 6.
    pub working: [bool; 7],
}

impl Default for SchoolWeek {
    /// Mon–Sat working, Sunday off (§8.1 default).
    fn default() -> Self {
        // Mon Tue Wed Thu Fri Sat Sun
        SchoolWeek { working: [true, true, true, true, true, true, false] }
    }
}

impl SchoolWeek {
    /// True if `weekday` is a working weekday in this pattern (ignores events).
    pub fn is_working_weekday(&self, weekday: Weekday) -> bool {
        self.working[weekday_index(weekday)]
    }

    /// True if at least one weekday is a working day (guards infinite scans).
    pub fn has_any_working_day(&self) -> bool {
        self.working.iter().any(|&w| w)
    }
}

/// Weekday as an array index: Monday = 0 … Sunday = 6 (matches [`SchoolWeek`]).
pub fn weekday_index(w: Weekday) -> usize {
    // `number_days_from_monday` is 0 (Mon) … 6 (Sun).
    w.number_days_from_monday() as usize
}

/// The ISO weekday number 1 (Monday) … 7 (Sunday) — the value stored in the
/// `school_week.weekday` column.
pub fn weekday_iso(w: Weekday) -> u8 {
    w.number_from_monday()
}

/// What a calendar event is (00-SYSTEM-CONTEXT §7a / §8.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Holiday,
    Exam,
    Event,
}

/// A dated calendar event. `starts_on`/`ends_on` are inclusive (a single-day
/// event has `starts_on == ends_on`). Only `is_non_working` events flip a day to
/// non-working; the title/session/circular columns are carried by the row and are
/// irrelevant to the working-day rule, so they are not modelled here.
///
/// Not `Serialize`/`Deserialize`: `time::Date` isn't serde-enabled in this crate
/// and the pure rules never serialize an event — the repository layer builds these
/// from DB rows and its own DTOs carry the string dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarEvent {
    pub starts_on: Date,
    pub ends_on: Date,
    pub kind: EventKind,
    pub is_non_working: bool,
}

impl CalendarEvent {
    /// True if this event covers `date` (inclusive of both ends).
    pub fn covers(&self, date: Date) -> bool {
        self.starts_on <= date && date <= self.ends_on
    }
}

/// Is `date` a working day? A non-working event covering the date makes it
/// non-working; otherwise the weekly pattern decides.
pub fn is_working_day(date: Date, week: &SchoolWeek, events: &[CalendarEvent]) -> bool {
    if events.iter().any(|e| e.is_non_working && e.covers(date)) {
        return false;
    }
    week.is_working_weekday(date.weekday())
}

/// Count the working days in the **inclusive** range `[from, to]`. Returns 0 if
/// `to` precedes `from`.
pub fn working_days(from: Date, to: Date, week: &SchoolWeek, events: &[CalendarEvent]) -> u32 {
    if to < from {
        return 0;
    }
    let mut day = from;
    let mut count = 0u32;
    loop {
        if is_working_day(day, week, events) {
            count += 1;
        }
        if day == to {
            break;
        }
        match day.next_day() {
            Some(next) => day = next,
            None => break, // reached the maximum representable date
        }
    }
    count
}

/// The first working day on or after `date` (returns `date` itself when it is a
/// working day). Used only to show a "due on next working day" note — it never
/// moves a money date. `None` if the week has no working day at all, or no working
/// day is found within a bounded scan (a run of non-working events longer than a
/// year, which never happens in practice).
pub fn next_working_day(date: Date, week: &SchoolWeek, events: &[CalendarEvent]) -> Option<Date> {
    if !week.has_any_working_day() {
        return None;
    }
    let mut day = date;
    // A full year plus slack is far more than any real holiday run.
    for _ in 0..400 {
        if is_working_day(day, week, events) {
            return Some(day);
        }
        day = day.next_day()?;
    }
    None
}

/// Parse a strict `YYYY-MM-DD` date. Convenience for the repository/command
/// boundary; the pure rules above all take a [`Date`] directly.
pub fn parse_date(s: &str) -> Option<Date> {
    let fmt = time::macros::format_description!("[year]-[month]-[day]");
    Date::parse(s, fmt).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn default_week_is_mon_to_sat_working_sunday_off() {
        let w = SchoolWeek::default();
        // 2026-09-21 is a Monday … 2026-09-27 is a Sunday.
        assert!(is_working_day(date!(2026 - 09 - 21), &w, &[])); // Mon
        assert!(is_working_day(date!(2026 - 09 - 22), &w, &[])); // Tue
        assert!(is_working_day(date!(2026 - 09 - 23), &w, &[])); // Wed
        assert!(is_working_day(date!(2026 - 09 - 24), &w, &[])); // Thu
        assert!(is_working_day(date!(2026 - 09 - 25), &w, &[])); // Fri
        assert!(is_working_day(date!(2026 - 09 - 26), &w, &[])); // Sat
        assert!(!is_working_day(date!(2026 - 09 - 27), &w, &[])); // Sun (off)
    }

    #[test]
    fn weekday_index_and_iso_match_time() {
        assert_eq!(weekday_index(date!(2026 - 09 - 21).weekday()), 0); // Mon
        assert_eq!(weekday_index(date!(2026 - 09 - 27).weekday()), 6); // Sun
        assert_eq!(weekday_iso(date!(2026 - 09 - 21).weekday()), 1); // Mon
        assert_eq!(weekday_iso(date!(2026 - 09 - 27).weekday()), 7); // Sun
    }

    #[test]
    fn a_non_working_event_overrides_a_working_weekday() {
        let w = SchoolWeek::default();
        let holiday = CalendarEvent {
            starts_on: date!(2026 - 10 - 02), // Gandhi Jayanti (a Friday)
            ends_on: date!(2026 - 10 - 02),
            kind: EventKind::Holiday,
            is_non_working: true,
        };
        assert!(!is_working_day(date!(2026 - 10 - 02), &w, &[holiday]));
        // The day before and after are still working days.
        assert!(is_working_day(date!(2026 - 10 - 01), &w, &[holiday]));
        assert!(is_working_day(date!(2026 - 10 - 03), &w, &[holiday]));
    }

    #[test]
    fn a_working_event_does_not_change_a_working_day() {
        let w = SchoolWeek::default();
        let exam = CalendarEvent {
            starts_on: date!(2026 - 09 - 23),
            ends_on: date!(2026 - 09 - 25),
            kind: EventKind::Exam,
            is_non_working: false,
        };
        // Exam days that are not marked non-working stay working days.
        assert!(is_working_day(date!(2026 - 09 - 24), &w, &[exam]));
    }

    #[test]
    fn an_event_never_makes_an_off_day_working() {
        let w = SchoolWeek::default();
        let event = CalendarEvent {
            starts_on: date!(2026 - 09 - 27), // a Sunday
            ends_on: date!(2026 - 09 - 27),
            kind: EventKind::Event,
            is_non_working: false, // e.g. a Sunday sports day, but school-week says off
        };
        assert!(!is_working_day(date!(2026 - 09 - 27), &w, &[event]));
    }

    #[test]
    fn multi_day_non_working_event_covers_the_whole_range() {
        let w = SchoolWeek::default();
        let dasara = CalendarEvent {
            starts_on: date!(2026 - 10 - 19),
            ends_on: date!(2026 - 10 - 24),
            kind: EventKind::Holiday,
            is_non_working: true,
        };
        for d in [
            date!(2026 - 10 - 19),
            date!(2026 - 10 - 21),
            date!(2026 - 10 - 24),
        ] {
            assert!(!is_working_day(d, &w, &[dasara]), "{d} should be non-working");
        }
        assert!(is_working_day(date!(2026 - 10 - 26), &w, &[dasara])); // Monday after
    }

    #[test]
    fn working_days_counts_inclusive_range_minus_sundays() {
        let w = SchoolWeek::default();
        // Mon 2026-09-21 .. Sun 2026-09-27 = 6 working days (Sunday off).
        assert_eq!(
            working_days(date!(2026 - 09 - 21), date!(2026 - 09 - 27), &w, &[]),
            6
        );
        // A single working day.
        assert_eq!(
            working_days(date!(2026 - 09 - 21), date!(2026 - 09 - 21), &w, &[]),
            1
        );
        // A single Sunday.
        assert_eq!(
            working_days(date!(2026 - 09 - 27), date!(2026 - 09 - 27), &w, &[]),
            0
        );
        // Reversed range.
        assert_eq!(
            working_days(date!(2026 - 09 - 27), date!(2026 - 09 - 21), &w, &[]),
            0
        );
    }

    #[test]
    fn working_days_subtracts_holidays() {
        let w = SchoolWeek::default();
        let holiday = CalendarEvent {
            starts_on: date!(2026 - 09 - 23),
            ends_on: date!(2026 - 09 - 23),
            kind: EventKind::Holiday,
            is_non_working: true,
        };
        // Mon..Sat = 6 working, minus the Wed holiday = 5.
        assert_eq!(
            working_days(date!(2026 - 09 - 21), date!(2026 - 09 - 26), &w, &[holiday]),
            5
        );
    }

    #[test]
    fn next_working_day_skips_sunday_and_holidays() {
        let w = SchoolWeek::default();
        // From a Sunday → the following Monday.
        assert_eq!(
            next_working_day(date!(2026 - 09 - 27), &w, &[]),
            Some(date!(2026 - 09 - 28))
        );
        // A working day maps to itself.
        assert_eq!(
            next_working_day(date!(2026 - 09 - 21), &w, &[]),
            Some(date!(2026 - 09 - 21))
        );
        // Skip a Monday holiday → Tuesday.
        let holiday = CalendarEvent {
            starts_on: date!(2026 - 09 - 28),
            ends_on: date!(2026 - 09 - 28),
            kind: EventKind::Holiday,
            is_non_working: true,
        };
        assert_eq!(
            next_working_day(date!(2026 - 09 - 27), &w, &[holiday]),
            Some(date!(2026 - 09 - 29))
        );
    }

    #[test]
    fn next_working_day_none_when_no_working_weekday() {
        let none_working = SchoolWeek { working: [false; 7] };
        assert_eq!(next_working_day(date!(2026 - 09 - 21), &none_working, &[]), None);
    }

    #[test]
    fn parse_date_is_strict() {
        assert_eq!(parse_date("2026-09-25"), Some(date!(2026 - 09 - 25)));
        assert_eq!(parse_date("2026-9-25"), None);
        assert_eq!(parse_date("25-09-2026"), None);
        assert_eq!(parse_date(""), None);
        assert_eq!(parse_date("2026-13-01"), None);
    }
}
