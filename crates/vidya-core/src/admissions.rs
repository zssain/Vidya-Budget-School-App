//! admissions — provisional and admission number formatting/validation (pure).
//!
//! * Provisional numbers: `P-<series>-<seq>`, `seq` zero-padded to a minimum of
//!   4 digits, growing past 9999 (never truncated).
//! * Admission numbers: `YYYY/NNNN` — a 4-digit year, a slash, then a sequence
//!   of at least 4 digits.

use crate::errors::{CoreError, CoreResult};

/// A provisional number `P-<series>-<seq>` with `seq` zero-padded to at least
/// 4 digits.
///
/// ```
/// use vidya_core::admissions::provisional_no;
/// assert_eq!(provisional_no("A2", 3), "P-A2-0003");
/// ```
pub fn provisional_no(series: &str, seq: u32) -> String {
    format!("P-{series}-{seq:04}")
}

/// The next admission number for `year`, given the last-issued sequence.
///
/// Returns `YYYY/NNNN` with `seq = last + 1`, zero-padded to at least 4 digits.
///
/// ```
/// use vidya_core::admissions::next_admission_no;
/// assert_eq!(next_admission_no(2026, 141), "2026/0142");
/// ```
pub fn next_admission_no(year: i32, last: u32) -> String {
    let seq = last + 1;
    format!("{year:04}/{seq:04}")
}

/// Validate an admission number against `^\d{4}/\d{4,}$` (4-digit year, slash,
/// then a sequence of 4 or more digits). Parsed by hand — no `regex` crate.
///
/// Returns `CoreError::validation("admission_no", "format")` on any mismatch.
pub fn validate_admission_no(s: &str) -> CoreResult<()> {
    let err = || CoreError::validation("admission_no", "format");

    // Exactly one slash, splitting year and sequence.
    let (year, seq) = s.split_once('/').ok_or_else(err)?;

    // No further slashes allowed anywhere.
    if seq.contains('/') {
        return Err(err());
    }

    // Year: exactly 4 ASCII digits.
    if year.len() != 4 || !year.bytes().all(|b| b.is_ascii_digit()) {
        return Err(err());
    }

    // Sequence: at least 4 ASCII digits.
    if seq.len() < 4 || !seq.bytes().all(|b| b.is_ascii_digit()) {
        return Err(err());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provisional_pads_to_four() {
        assert_eq!(provisional_no("A2", 3), "P-A2-0003");
        assert_eq!(provisional_no("A2", 0), "P-A2-0000");
        assert_eq!(provisional_no("B", 99), "P-B-0099");
    }

    #[test]
    fn provisional_grows_past_9999() {
        assert_eq!(provisional_no("A2", 9999), "P-A2-9999");
        assert_eq!(provisional_no("A2", 10000), "P-A2-10000");
        assert_eq!(provisional_no("A2", 100000), "P-A2-100000");
    }

    #[test]
    fn next_admission_basic() {
        assert_eq!(next_admission_no(2026, 141), "2026/0142");
        assert_eq!(next_admission_no(2026, 0), "2026/0001");
        assert_eq!(next_admission_no(2026, 8), "2026/0009");
    }

    #[test]
    fn next_admission_grows_past_9999() {
        assert_eq!(next_admission_no(2026, 9999), "2026/10000");
        assert_eq!(next_admission_no(2026, 99999), "2026/100000");
    }

    #[test]
    fn validate_accepts_well_formed() {
        assert!(validate_admission_no("2026/0142").is_ok());
        assert!(validate_admission_no("2026/0001").is_ok());
        assert!(validate_admission_no("1999/9999").is_ok());
        assert!(validate_admission_no("2026/10000").is_ok()); // 5-digit seq
    }

    #[test]
    fn validate_rejects_malformed() {
        let want = CoreError::validation("admission_no", "format");
        assert_eq!(validate_admission_no("2026/12"), Err(want.clone())); // seq too short
        assert_eq!(validate_admission_no("abc"), Err(want.clone())); // no slash
        assert_eq!(validate_admission_no("202/0142"), Err(want.clone())); // year too short
        assert_eq!(validate_admission_no("20268/0142"), Err(want.clone())); // year too long
        assert_eq!(validate_admission_no("2026/abcd"), Err(want.clone())); // non-digit seq
        assert_eq!(validate_admission_no("20a6/0142"), Err(want.clone())); // non-digit year
        assert_eq!(validate_admission_no("2026-0142"), Err(want.clone())); // wrong separator
        assert_eq!(validate_admission_no("/0142"), Err(want.clone())); // empty year
        assert_eq!(validate_admission_no("2026/"), Err(want.clone())); // empty seq
        assert_eq!(validate_admission_no("2026/01/42"), Err(want.clone())); // two slashes
        assert_eq!(validate_admission_no(""), Err(want));
    }

    #[test]
    fn next_admission_round_trips_through_validate() {
        for (year, last) in [(2026, 141), (2026, 9999), (1999, 0)] {
            let s = next_admission_no(year, last);
            assert!(validate_admission_no(&s).is_ok(), "{s} should validate");
        }
    }

    #[test]
    fn provisional_and_next_are_distinct_shapes() {
        // Provisional uses P- prefix and hyphens; admission uses YYYY/NNNN.
        let p = provisional_no("A2", 3);
        assert!(p.starts_with("P-"));
        assert!(validate_admission_no(&p).is_err());
        let a = next_admission_no(2026, 2);
        assert!(!a.starts_with("P-"));
        assert!(validate_admission_no(&a).is_ok());
    }
}
