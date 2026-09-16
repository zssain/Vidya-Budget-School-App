use chrono::{Datelike, NaiveDate};

use crate::error::DomainError;

fn date_error() -> DomainError {
    DomainError::validation("date.error.invalid").field("date")
}

fn session_error() -> DomainError {
    DomainError::validation("session.error.invalid").field("session")
}

/// Parses an exact `YYYY-MM-DD` calendar date.
pub fn parse_date(s: &str) -> Result<NaiveDate, DomainError> {
    if s.len() != 10
        || s.as_bytes()[4] != b'-'
        || s.as_bytes()[7] != b'-'
        || !s
            .bytes()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return Err(date_error());
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| date_error())
}

/// Formats a date as `15 Sep 2026`.
pub fn format_date_en(d: NaiveDate) -> String {
    d.format("%-d %b %Y").to_string()
}

/// Returns the April-to-March session containing the supplied date.
pub fn default_session_name(today: NaiveDate) -> String {
    let start = if today.month() >= 4 {
        today.year()
    } else {
        today.year() - 1
    };
    format!("{start}-{:02}", (start + 1).rem_euclid(100))
}

/// Parses `YYYY-YY` and verifies that the second year follows the first.
pub fn parse_session_name(s: &str) -> Result<(i32, i32), DomainError> {
    if s.len() != 7 || s.as_bytes()[4] != b'-' {
        return Err(session_error());
    }
    let first = s[..4].parse::<i32>().map_err(|_| session_error())?;
    let suffix = s[5..].parse::<i32>().map_err(|_| session_error())?;
    let second = first.checked_add(1).ok_or_else(session_error)?;
    if second.rem_euclid(100) != suffix {
        return Err(session_error());
    }
    Ok((first, second))
}

/// Returns inclusive April-to-March bounds for a valid session name.
pub fn session_bounds(name: &str) -> Result<(NaiveDate, NaiveDate), DomainError> {
    let (first, second) = parse_session_name(name)?;
    let start = NaiveDate::from_ymd_opt(first, 4, 1).ok_or_else(session_error)?;
    let end = NaiveDate::from_ymd_opt(second, 3, 31).ok_or_else(session_error)?;
    Ok((start, end))
}

/// Returns the number of days in a valid month, or zero for an invalid month.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    let Some(first) = NaiveDate::from_ymd_opt(year, month, 1) else {
        return 0;
    };
    let (next_year, next_month) = if month == 12 {
        let Some(next_year) = year.checked_add(1) else {
            return 0;
        };
        (next_year, 1)
    } else {
        (year, month + 1)
    };
    let Some(next) = NaiveDate::from_ymd_opt(next_year, next_month, 1) else {
        return 0;
    };
    (next - first).num_days() as u32
}
