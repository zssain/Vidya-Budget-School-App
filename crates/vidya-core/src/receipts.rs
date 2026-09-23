//! receipts — receipt-number formatting and parsing (pure, no IO).
//!
//! A receipt number is `R-<series>-<seq>` where `<seq>` is zero-padded to a
//! minimum of 4 digits and grows to 5+ digits once it passes 9999 (never
//! truncated). Numbers are never reused: the caller stores `last_seq` and this
//! module returns `last_seq + 1`.

/// The next receipt number for `series`, given the last-issued sequence.
///
/// Returns `R-<series>-<seq>` with `seq = last_seq + 1`, zero-padded to at
/// least 4 digits.
///
/// ```
/// use vidya_core::receipts::next_receipt_no;
/// assert_eq!(next_receipt_no("A2", 418), "R-A2-0419");
/// assert_eq!(next_receipt_no("A2", 9999), "R-A2-10000");
/// ```
pub fn next_receipt_no(series: &str, last_seq: u32) -> String {
    let seq = last_seq + 1;
    format!("R-{series}-{seq:04}")
}

/// Parse a receipt number back into `(series, seq)`.
///
/// Returns `None` if the string is not of the form `R-<series>-<seq>` with a
/// non-empty series and an all-digit sequence.
///
/// ```
/// use vidya_core::receipts::parse_receipt_no;
/// assert_eq!(parse_receipt_no("R-A2-0419"), Some(("A2".to_string(), 419)));
/// ```
pub fn parse_receipt_no(s: &str) -> Option<(String, u32)> {
    // Must start with the `R-` prefix.
    let rest = s.strip_prefix("R-")?;
    // The sequence is the final `-`-separated segment; the series is everything
    // before it (which may itself contain `-`, though callers don't use that).
    let split = rest.rfind('-')?;
    let series = &rest[..split];
    let seq_str = &rest[split + 1..];
    if series.is_empty() || seq_str.is_empty() {
        return None;
    }
    // All-ASCII-digits only (reject "+1", " 1", etc.).
    if !seq_str.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let seq: u32 = seq_str.parse().ok()?;
    Some((series.to_string(), seq))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_pads_to_four_digits() {
        assert_eq!(next_receipt_no("A2", 418), "R-A2-0419");
        assert_eq!(next_receipt_no("A2", 0), "R-A2-0001");
        assert_eq!(next_receipt_no("A2", 8), "R-A2-0009");
        assert_eq!(next_receipt_no("B", 98), "R-B-0099");
    }

    #[test]
    fn next_grows_past_9999_never_truncates() {
        assert_eq!(next_receipt_no("A2", 9999), "R-A2-10000");
        assert_eq!(next_receipt_no("A2", 10000), "R-A2-10001");
        assert_eq!(next_receipt_no("A2", 99999), "R-A2-100000");
    }

    #[test]
    fn parse_basic() {
        assert_eq!(parse_receipt_no("R-A2-0419"), Some(("A2".to_string(), 419)));
        assert_eq!(parse_receipt_no("R-A2-0001"), Some(("A2".to_string(), 1)));
        assert_eq!(parse_receipt_no("R-B-0099"), Some(("B".to_string(), 99)));
    }

    #[test]
    fn parse_five_plus_digit_seq() {
        assert_eq!(parse_receipt_no("R-A2-10000"), Some(("A2".to_string(), 10000)));
        assert_eq!(parse_receipt_no("R-A2-100000"), Some(("A2".to_string(), 100000)));
    }

    #[test]
    fn parse_rejects_bad_input() {
        assert_eq!(parse_receipt_no(""), None);
        assert_eq!(parse_receipt_no("R-A2-"), None);
        assert_eq!(parse_receipt_no("R--0419"), None);
        assert_eq!(parse_receipt_no("A2-0419"), None); // missing R- prefix
        assert_eq!(parse_receipt_no("R-A2-abcd"), None);
        assert_eq!(parse_receipt_no("R-A2-04x9"), None);
        assert_eq!(parse_receipt_no("R-A2"), None); // no seq segment
    }

    #[test]
    fn round_trips() {
        for (series, last_seq, expect_seq) in
            [("A2", 418u32, 419u32), ("A2", 9999, 10000), ("A2", 0, 1), ("Z9", 99998, 99999)]
        {
            let s = next_receipt_no(series, last_seq);
            let (parsed_series, parsed_seq) = parse_receipt_no(&s).unwrap();
            assert_eq!(parsed_series, series);
            assert_eq!(parsed_seq, expect_seq);
        }
    }

    #[test]
    fn seq_zero_pads_to_0001() {
        // last_seq = 0 → seq 1 → "0001".
        let s = next_receipt_no("A2", 0);
        assert_eq!(s, "R-A2-0001");
        assert_eq!(parse_receipt_no(&s), Some(("A2".to_string(), 1)));
    }
}
