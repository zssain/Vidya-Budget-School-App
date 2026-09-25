//! calendar — repository reads for the school calendar (P13, foundation §8.1).
//!
//! The working-day *rules* live in `vidya_core::calendar`; this module maps
//! `school_week` / `calendar_event` rows to the core types and exposes
//! [`is_working_day`] for attendance and dashboards. Audited **writes** live in
//! `commands::logic` (they go through `with_write`, like every other synced
//! change: row + audit + op in one transaction).

use rusqlite::Connection;
use vidya_core::calendar::{self, CalendarEvent, EventKind, SchoolWeek};

/// Map the stored `kind` string to the core enum (unknown → `Event`, defensive).
pub fn parse_kind(s: &str) -> EventKind {
    match s {
        "holiday" => EventKind::Holiday,
        "exam" => EventKind::Exam,
        _ => EventKind::Event,
    }
}

/// The weekly working pattern from `school_week`, falling back to the default
/// (Mon–Sat working, Sunday off) for any weekday with no row — so an empty table
/// on a fresh install already behaves correctly.
pub fn load_week(conn: &Connection) -> rusqlite::Result<SchoolWeek> {
    let mut week = SchoolWeek::default();
    let mut stmt = conn.prepare("SELECT weekday, is_working FROM school_week")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?;
    for row in rows {
        let (weekday, is_working) = row?;
        if (1..=7).contains(&weekday) {
            week.working[(weekday - 1) as usize] = is_working != 0;
        }
    }
    Ok(week)
}

/// Every calendar event mapped to the core [`CalendarEvent`] used by the
/// working-day rules. Rows whose dates don't parse are skipped defensively (they
/// can never make a day non-working). Dates are absolute, so this is not scoped
/// to a session — a holiday applies on its dates whatever session owns the row.
pub fn load_events(conn: &Connection) -> rusqlite::Result<Vec<CalendarEvent>> {
    let mut stmt =
        conn.prepare("SELECT starts_on, ends_on, kind, is_non_working FROM calendar_event")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, i64>(3)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (starts, ends, kind, non_working) = row?;
        if let (Some(s), Some(e)) = (calendar::parse_date(&starts), calendar::parse_date(&ends)) {
            out.push(CalendarEvent {
                starts_on: s,
                ends_on: e,
                kind: parse_kind(&kind),
                is_non_working: non_working != 0,
            });
        }
    }
    Ok(out)
}

/// Is school open on `date` (a `YYYY-MM-DD` string)? Combines the weekly pattern
/// and non-working events. An unparseable date is treated as a working day
/// (defensive: never hide a required attendance sheet because of a bad string).
pub fn is_working_day(conn: &Connection, date: &str) -> rusqlite::Result<bool> {
    let Some(d) = calendar::parse_date(date) else {
        return Ok(true);
    };
    let week = load_week(conn)?;
    let events = load_events(conn)?;
    Ok(calendar::is_working_day(d, &week, &events))
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
    fn empty_tables_use_the_default_week() {
        let c = fresh();
        // No school_week rows → default (Sunday off).
        assert!(is_working_day(&c, "2026-09-25").unwrap()); // Friday
        assert!(!is_working_day(&c, "2026-09-27").unwrap()); // Sunday
    }

    #[test]
    fn a_stored_weekly_off_makes_that_weekday_non_working() {
        let c = fresh();
        // Make Saturday (ISO 6) a non-working day.
        c.execute(
            "INSERT INTO school_week(id, weekday, is_working, created_at, updated_at) VALUES ('wk-6', 6, 0, 't', 't')",
            [],
        )
        .unwrap();
        assert!(!is_working_day(&c, "2026-09-26").unwrap()); // Saturday now off
        assert!(is_working_day(&c, "2026-09-25").unwrap()); // Friday still working
    }

    #[test]
    fn a_non_working_event_makes_its_range_non_working() {
        let c = fresh();
        c.execute(
            "INSERT INTO calendar_event(id, starts_on, ends_on, kind, title, is_non_working, created_at, updated_at) \
             VALUES ('ev1','2026-10-02','2026-10-02','holiday','Gandhi Jayanti',1,'t','t')",
            [],
        )
        .unwrap();
        assert!(!is_working_day(&c, "2026-10-02").unwrap());
        assert!(is_working_day(&c, "2026-10-01").unwrap());
    }

    #[test]
    fn a_working_event_does_not_change_the_day() {
        let c = fresh();
        c.execute(
            "INSERT INTO calendar_event(id, starts_on, ends_on, kind, title, is_non_working, created_at, updated_at) \
             VALUES ('ev2','2026-09-23','2026-09-25','exam','Unit test',0,'t','t')",
            [],
        )
        .unwrap();
        assert!(is_working_day(&c, "2026-09-24").unwrap()); // still a working day
    }
}
