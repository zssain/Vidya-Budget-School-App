//! numbering — number formats for every counted document (P13, foundation §8.4).
//!
//! Pure. Generalises `receipts.rs`: one place decides how a receipt, voucher,
//! store sale, circular or hall ticket is written. The **receipt format is
//! unchanged** (`R-<series>-<seq>`), asserted by a cross-check test and by the
//! existing `receipts` tests which now delegate here.
//!
//! * Series-based (`<prefix><series>-<seq>`, seq ≥ 4 digits, never truncated):
//!   receipt `R-`, voucher `V-`, store sale `S-`, hall ticket `HT-`. The series
//!   is a device's issue series (e.g. `A2`).
//! * Circular `CIR/<session>/<seq>` (server-only), seq ≥ 3 digits, session like
//!   `2026-27`.

/// A counted document kind. `key()` is the stored `number_series.kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberKind {
    Receipt,
    Voucher,
    StoreSale,
    Circular,
    HallTicket,
}

impl NumberKind {
    /// The stable key stored in `number_series.kind`.
    pub fn key(&self) -> &'static str {
        match self {
            NumberKind::Receipt => "receipt",
            NumberKind::Voucher => "voucher",
            NumberKind::StoreSale => "store",
            NumberKind::Circular => "circular",
            NumberKind::HallTicket => "hall_ticket",
        }
    }

    /// The printed prefix (stored in `number_series.prefix`).
    pub fn prefix(&self) -> &'static str {
        match self {
            NumberKind::Receipt => "R-",
            NumberKind::Voucher => "V-",
            NumberKind::StoreSale => "S-",
            NumberKind::HallTicket => "HT-",
            NumberKind::Circular => "CIR",
        }
    }

    pub fn parse_key(s: &str) -> Option<NumberKind> {
        Some(match s {
            "receipt" => NumberKind::Receipt,
            "voucher" => NumberKind::Voucher,
            "store" => NumberKind::StoreSale,
            "circular" => NumberKind::Circular,
            "hall_ticket" => NumberKind::HallTicket,
            _ => return None,
        })
    }
}

/// Format a document number. `series_or_session` is the device series (e.g. `A2`)
/// for series-based kinds, or the session label (e.g. `2026-27`) for circulars.
/// `seq` is the 1-based sequence within that series.
pub fn format_number(kind: NumberKind, series_or_session: &str, seq: u32) -> String {
    match kind {
        NumberKind::Circular => format!("CIR/{series_or_session}/{seq:03}"),
        _ => format!("{}{}-{:04}", kind.prefix(), series_or_session, seq),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_based_formats() {
        assert_eq!(format_number(NumberKind::Receipt, "A2", 419), "R-A2-0419");
        assert_eq!(format_number(NumberKind::Voucher, "A2", 88), "V-A2-0088");
        assert_eq!(format_number(NumberKind::StoreSale, "A2", 31), "S-A2-0031");
        assert_eq!(format_number(NumberKind::HallTicket, "A2", 4), "HT-A2-0004");
    }

    #[test]
    fn receipt_format_matches_receipts_module_exactly() {
        // The whole point of "keep receipt output identical".
        for (series, last) in [("A2", 418u32), ("A2", 9999), ("B", 0), ("Z9", 99998)] {
            assert_eq!(
                format_number(NumberKind::Receipt, series, last + 1),
                crate::receipts::next_receipt_no(series, last),
            );
        }
    }

    #[test]
    fn seq_grows_past_9999_never_truncates() {
        assert_eq!(format_number(NumberKind::Receipt, "A2", 10000), "R-A2-10000");
        assert_eq!(format_number(NumberKind::Voucher, "A2", 100000), "V-A2-100000");
    }

    #[test]
    fn circular_is_session_scoped_three_digits() {
        assert_eq!(format_number(NumberKind::Circular, "2026-27", 14), "CIR/2026-27/014");
        assert_eq!(format_number(NumberKind::Circular, "2026-27", 1000), "CIR/2026-27/1000");
    }

    #[test]
    fn keys_round_trip() {
        for k in [NumberKind::Receipt, NumberKind::Voucher, NumberKind::StoreSale, NumberKind::Circular, NumberKind::HallTicket] {
            assert_eq!(NumberKind::parse_key(k.key()), Some(k));
        }
        assert_eq!(NumberKind::parse_key("nope"), None);
    }
}
