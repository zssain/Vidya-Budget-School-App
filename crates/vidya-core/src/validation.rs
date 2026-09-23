//! Field validators (prompts/P02 validation.rs).
//!
//! Pure Rust, no regex crate — patterns are checked by hand over Unicode scalar
//! values. All failures are [`CoreError::Validation`] carrying the field name.

use crate::errors::{CoreError, CoreResult};

/// EN DASH (U+2013) — the separator used in academic session labels.
const EN_DASH: char = '\u{2013}';

/// Trim surrounding whitespace and require 1–120 characters (counted as Unicode
/// scalar values, so Devanagari and any other script count one-per-character,
/// NOT per byte). Returns the trimmed name.
///
/// Empty (after trim) or longer than 120 chars → validation error on `"name"`.
pub fn validate_name(s: &str) -> CoreResult<String> {
    let trimmed = s.trim();
    let len = trimmed.chars().count();
    if len == 0 {
        return Err(CoreError::validation("name", "required"));
    }
    if len > 120 {
        return Err(CoreError::validation("name", "too_long"));
    }
    Ok(trimmed.to_string())
}

/// Indian mobile: exactly `^[6-9]\d{9}$` — 10 ASCII digits, first digit 6–9.
///
/// No leading/trailing whitespace is tolerated (the caller trims if it wants).
pub fn validate_mobile(s: &str) -> CoreResult<()> {
    let bytes = s.as_bytes();
    if bytes.len() != 10 {
        return Err(CoreError::validation("mobile", "pattern"));
    }
    // First digit must be 6–9.
    match bytes[0] {
        b'6'..=b'9' => {}
        _ => return Err(CoreError::validation("mobile", "pattern")),
    }
    // Remaining nine must all be ASCII digits.
    for &b in &bytes[1..] {
        if !b.is_ascii_digit() {
            return Err(CoreError::validation("mobile", "pattern"));
        }
    }
    Ok(())
}

/// Academic session label: `"YYYY–YY"` with an EN DASH (U+2013) separator, where
/// the two-digit second part equals `(first_year + 1) mod 100`, zero-padded
/// (e.g. `"2026–27"`, `"2099–00"`).
///
/// A hyphen-minus separator, a wrong second year, or malformed digits → error.
pub fn validate_session_label(s: &str) -> CoreResult<()> {
    // Split on the EN DASH exactly once.
    let mut parts = s.split(EN_DASH);
    let first = parts.next().unwrap_or("");
    let second = match parts.next() {
        Some(v) => v,
        None => return Err(CoreError::validation("session_label", "pattern")),
    };
    // No third segment allowed (no stray extra dashes).
    if parts.next().is_some() {
        return Err(CoreError::validation("session_label", "pattern"));
    }
    // First part: exactly 4 ASCII digits.
    if first.len() != 4 || !first.bytes().all(|b| b.is_ascii_digit()) {
        return Err(CoreError::validation("session_label", "pattern"));
    }
    // Second part: exactly 2 ASCII digits.
    if second.len() != 2 || !second.bytes().all(|b| b.is_ascii_digit()) {
        return Err(CoreError::validation("session_label", "pattern"));
    }
    // Digits are ASCII → these parses cannot fail.
    let first_year: u32 = first.parse().expect("4 ascii digits");
    let second_year: u32 = second.parse().expect("2 ascii digits");
    let expected = (first_year + 1) % 100;
    if second_year != expected {
        return Err(CoreError::validation("session_label", "second_year"));
    }
    Ok(())
}

/// A date must fall within `[starts_on, ends_on]` inclusive.
pub fn date_in_session(
    d: time::Date,
    starts_on: time::Date,
    ends_on: time::Date,
) -> CoreResult<()> {
    if d < starts_on || d > ends_on {
        return Err(CoreError::validation("date", "out_of_session"));
    }
    Ok(())
}

/// Local unlock PIN: 4–6 ASCII digits.
pub fn validate_pin(s: &str) -> CoreResult<()> {
    let len = s.len();
    if !(4..=6).contains(&len) {
        return Err(CoreError::validation("pin", "length"));
    }
    if !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(CoreError::validation("pin", "digits"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    // --- validate_name -----------------------------------------------------

    #[test]
    fn name_trims_and_returns() {
        assert_eq!(validate_name("  Asha Rao  ").unwrap(), "Asha Rao");
    }

    #[test]
    fn name_allows_devanagari() {
        // "प्रिया शर्मा" — Devanagari counts by scalar value, well under 120.
        let out = validate_name("प्रिया शर्मा").unwrap();
        assert_eq!(out, "प्रिया शर्मा");
    }

    #[test]
    fn name_empty_is_rejected() {
        // 0 chars after trim.
        assert!(validate_name("").is_err());
        assert!(validate_name("     ").is_err());
    }

    #[test]
    fn name_boundary_lengths() {
        // 1 char ok, 120 chars ok, 121 chars rejected.
        assert!(validate_name("A").is_ok());
        let n120 = "अ".repeat(120);
        assert_eq!(n120.chars().count(), 120);
        assert!(validate_name(&n120).is_ok());
        let n121 = "अ".repeat(121);
        assert_eq!(n121.chars().count(), 121);
        assert!(validate_name(&n121).is_err());
    }

    #[test]
    fn name_120_multibyte_not_counted_by_bytes() {
        // 120 Devanagari chars are > 120 bytes; must still pass (char count, not bytes).
        let n120 = "क".repeat(120);
        assert_eq!(n120.chars().count(), 120);
        assert!(n120.len() > 120); // more than 120 bytes
        assert!(validate_name(&n120).is_ok());
    }

    // --- validate_mobile ---------------------------------------------------

    #[test]
    fn mobile_valid() {
        assert!(validate_mobile("9876543210").is_ok());
        assert!(validate_mobile("6000000000").is_ok());
    }

    #[test]
    fn mobile_leading_digit_below_6_rejected() {
        assert!(validate_mobile("5876543210").is_err());
        assert!(validate_mobile("0123456789").is_err());
    }

    #[test]
    fn mobile_wrong_length_rejected() {
        assert!(validate_mobile("987654321").is_err()); // 9 digits
        assert!(validate_mobile("98765432101").is_err()); // 11 digits
    }

    #[test]
    fn mobile_non_digits_rejected() {
        assert!(validate_mobile("98765abcde").is_err());
        assert!(validate_mobile("+91987654321").is_err());
    }

    // --- validate_session_label -------------------------------------------

    #[test]
    fn session_label_en_dash_valid() {
        assert!(validate_session_label("2026\u{2013}27").is_ok());
        assert!(validate_session_label("2099\u{2013}00").is_ok()); // wrap-around
    }

    #[test]
    fn session_label_hyphen_minus_rejected() {
        // ASCII hyphen-minus instead of EN DASH.
        assert!(validate_session_label("2026-27").is_err());
    }

    #[test]
    fn session_label_wrong_second_year_rejected() {
        assert!(validate_session_label("2026\u{2013}28").is_err());
        assert!(validate_session_label("2026\u{2013}26").is_err());
    }

    #[test]
    fn session_label_malformed_rejected() {
        assert!(validate_session_label("26\u{2013}27").is_err()); // 2-digit first
        assert!(validate_session_label("2026\u{2013}277").is_err()); // 3-digit second
        assert!(validate_session_label("2026").is_err()); // no dash
        assert!(validate_session_label("2026\u{2013}27\u{2013}28").is_err()); // extra dash
    }

    // --- date_in_session ---------------------------------------------------

    #[test]
    fn date_in_session_inclusive() {
        let start = date!(2026 - 04 - 01);
        let end = date!(2027 - 03 - 31);
        assert!(date_in_session(start, start, end).is_ok()); // start boundary
        assert!(date_in_session(end, start, end).is_ok()); // end boundary
        assert!(date_in_session(date!(2026 - 09 - 23), start, end).is_ok());
    }

    #[test]
    fn date_out_of_session_rejected() {
        let start = date!(2026 - 04 - 01);
        let end = date!(2027 - 03 - 31);
        assert!(date_in_session(date!(2026 - 03 - 31), start, end).is_err());
        assert!(date_in_session(date!(2027 - 04 - 01), start, end).is_err());
    }

    // --- validate_pin ------------------------------------------------------

    #[test]
    fn pin_length_boundaries() {
        assert!(validate_pin("123").is_err()); // 3 digits
        assert!(validate_pin("1234").is_ok()); // 4 digits
        assert!(validate_pin("123456").is_ok()); // 6 digits
        assert!(validate_pin("1234567").is_err()); // 7 digits
    }

    #[test]
    fn pin_non_digits_rejected() {
        assert!(validate_pin("12a4").is_err());
        assert!(validate_pin("१२३४").is_err()); // Devanagari digits are not ASCII
    }
}
