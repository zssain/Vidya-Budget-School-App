//! upi — UPI deep-link (`upi://pay`) building and VPA validation (P14, §10.1).
//!
//! Pure Rust, no IO. Verified against **NPCI UPI Linking Specifications v1.6
//! (Nov 2017)** — see `docs/phase-14-checks.md`. Vidya's per-student QR is a
//! plain **unsigned** payee link (the spec explicitly allows a static QR carrying
//! just payee address + name); the school pays into its own VPA and records the
//! payment by hand, so no merchant signature (`mode`/`sign`/`orgid`) is involved.
//! Vidya never handles money.
//!
//! Parameters used: `pa` (payee VPA, mandatory), `pn` (payee name, mandatory),
//! `am` (amount, decimal rupees), `cu` (`INR` — the only supported value), `tn`
//! (optional short note). NPCI sets **no** length limit on `tn`/`pn`; the ≤50-char
//! `tn` cap here is Vidya's own conservative choice (shorter URLs scan more
//! reliably and several PSP apps truncate the note), documented in the checks report.

use crate::errors::{CoreError, CoreResult};

/// Vidya's own conservative cap on the transaction-note length (NOT an NPCI limit).
pub const MAX_NOTE_CHARS: usize = 50;

/// Validate a UPI VPA against `^[a-zA-Z0-9.\-_]{2,256}@[a-zA-Z]{2,64}$` (§10.1) and
/// return it trimmed. The local part is 2–256 of `[A-Za-z0-9.\-_]`; the handle
/// (after `@`) is 2–64 ASCII letters. Errors are `Validation{field:"upi_id"}`.
pub fn validate_vpa(s: &str) -> CoreResult<String> {
    let vpa = s.trim();
    // Exactly one '@', splitting local@handle.
    let mut parts = vpa.split('@');
    let local = parts.next().unwrap_or("");
    let handle = match parts.next() {
        Some(h) => h,
        None => return Err(CoreError::validation("upi_id", "pattern")),
    };
    if parts.next().is_some() {
        // More than one '@'.
        return Err(CoreError::validation("upi_id", "pattern"));
    }
    // Local part: 2–256 chars from [A-Za-z0-9 . - _].
    let local_len = local.len();
    if !(2..=256).contains(&local_len) {
        return Err(CoreError::validation("upi_id", "pattern"));
    }
    if !local
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'_')
    {
        return Err(CoreError::validation("upi_id", "pattern"));
    }
    // Handle: 2–64 ASCII letters only.
    let handle_len = handle.len();
    if !(2..=64).contains(&handle_len) {
        return Err(CoreError::validation("upi_id", "pattern"));
    }
    if !handle.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Err(CoreError::validation("upi_id", "pattern"));
    }
    Ok(vpa.to_string())
}

/// Percent-encode `s` for use as a URL query value, encoding every byte that is
/// not RFC 3986 "unreserved" (`A-Za-z0-9 - . _ ~`). A space becomes `%20` (never
/// `+`); non-ASCII is encoded as its UTF-8 bytes. This is the safe encoding for
/// free-text `pn`/`tn` values (spaces, Devanagari/Telugu, punctuation).
fn encode(s: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        let unreserved =
            b.is_ascii_alphanumeric() || b == b'-' || b == b'.' || b == b'_' || b == b'~';
        if unreserved {
            out.push(b as char);
        } else {
            out.push('%');
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0x0f) as usize] as char);
        }
    }
    out
}

/// Format a non-negative paise amount as decimal rupees with exactly two places
/// (NPCI: `am` is "decimal format"), e.g. `310000` → `"3100.00"`, `5` → `"0.05"`.
fn amount_rupees(amount_paise: i64) -> String {
    let p = amount_paise.max(0);
    format!("{}.{:02}", p / 100, p % 100)
}

/// Build a `upi://pay` link for a payee-initiated collection.
///
/// - `vpa` is assumed already validated (only `[A-Za-z0-9.\-_@]`, so it needs no
///   encoding — the `@` is kept literal, as every real UPI QR does).
/// - `name` (payee display name) is percent-encoded.
/// - `amount_paise` > 0 is included as `am` in decimal rupees; ≤ 0 omits `am`
///   (per NPCI, an absent `am` leaves the amount editable in the payer's app).
/// - `note` is trimmed, capped to [`MAX_NOTE_CHARS`], percent-encoded, and omitted
///   when empty.
///
/// Always sets `cu=INR` (the only value NPCI supports).
pub fn upi_uri(vpa: &str, name: &str, amount_paise: i64, note: &str) -> String {
    let mut uri = format!("upi://pay?pa={}&pn={}", vpa, encode(name.trim()));
    if amount_paise > 0 {
        uri.push_str("&am=");
        uri.push_str(&amount_rupees(amount_paise));
    }
    uri.push_str("&cu=INR");
    // Cap the note to MAX_NOTE_CHARS scalar values (Unicode-safe), then encode.
    let note_trimmed: String = note.trim().chars().take(MAX_NOTE_CHARS).collect();
    if !note_trimmed.is_empty() {
        uri.push_str("&tn=");
        uri.push_str(&encode(&note_trimmed));
    }
    uri
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- validate_vpa ------------------------------------------------------

    #[test]
    fn vpa_accepts_common_handles() {
        for good in [
            "school@okhdfcbank",
            "vidya.school@okaxis",
            "9876543210@paytm",
            "a_b-c.d@ybl",
            "AB@upi",
        ] {
            assert!(validate_vpa(good).is_ok(), "{good} should be valid");
        }
    }

    #[test]
    fn vpa_trims_and_returns() {
        assert_eq!(validate_vpa("  school@okhdfcbank  ").unwrap(), "school@okhdfcbank");
    }

    #[test]
    fn vpa_rejects_malformed() {
        for bad in [
            "noathandle",         // no '@'
            "a@b@c",              // two '@'
            "x@okhdfcbank",       // local too short (1)
            "ab@u",               // handle too short (1)
            "ab@bank1",           // digit in handle
            "spaces here@okaxis", // space in local
            "अ@okaxis",           // non-ascii local
            "@okaxis",            // empty local
            "school@",            // empty handle
        ] {
            assert_eq!(
                validate_vpa(bad).unwrap_err().code(),
                "VALIDATION",
                "{bad} should be rejected"
            );
        }
    }

    #[test]
    fn vpa_length_boundaries() {
        // local 2..=256, handle 2..=64
        assert!(validate_vpa(&format!("{}@okaxis", "a".repeat(256))).is_ok());
        assert!(validate_vpa(&format!("{}@okaxis", "a".repeat(257))).is_err());
        assert!(validate_vpa(&format!("ab@{}", "b".repeat(64))).is_ok());
        assert!(validate_vpa(&format!("ab@{}", "b".repeat(65))).is_err());
    }

    // --- upi_uri -----------------------------------------------------------

    #[test]
    fn uri_basic_balance() {
        // ₹3,100.00 balance for a fees note.
        let uri = upi_uri("school@okhdfcbank", "Green Valley School", 310000, "Fees Aarav VII-B");
        assert_eq!(
            uri,
            "upi://pay?pa=school@okhdfcbank&pn=Green%20Valley%20School&am=3100.00&cu=INR&tn=Fees%20Aarav%20VII-B"
        );
    }

    #[test]
    fn uri_amount_formatting() {
        assert!(upi_uri("s@ok", "S", 5, "").contains("&am=0.05&"));
        assert!(upi_uri("s@ok", "S", 100, "").contains("&am=1.00&"));
        assert!(upi_uri("s@ok", "S", 310050, "").contains("&am=3100.50&"));
    }

    #[test]
    fn uri_omits_am_when_not_positive() {
        // No am → the payer app leaves the amount editable (NPCI).
        let uri = upi_uri("s@ok", "S", 0, "Pay school fees");
        assert!(!uri.contains("am="), "{uri}");
        assert!(uri.contains("&cu=INR"));
    }

    #[test]
    fn uri_omits_empty_note() {
        let uri = upi_uri("s@ok", "S", 100, "   ");
        assert!(!uri.contains("tn="), "{uri}");
    }

    #[test]
    fn uri_note_capped_to_50_chars() {
        let long = "x".repeat(80);
        let uri = upi_uri("s@ok", "S", 100, &long);
        // tn value is exactly 50 'x' (ascii → 1 char each, no encoding).
        let tn = uri.split("&tn=").nth(1).unwrap();
        assert_eq!(tn.chars().count(), MAX_NOTE_CHARS);
    }

    #[test]
    fn uri_encodes_unicode_note_and_name() {
        // Devanagari note is UTF-8 percent-encoded; '@' in pa stays literal.
        let uri = upi_uri("school@okaxis", "विद्या", 100, "फीस");
        assert!(uri.starts_with("upi://pay?pa=school@okaxis&pn=%"));
        assert!(uri.contains("&tn=%"));
        assert!(!uri.contains(' '));
    }

    #[test]
    fn uri_encodes_reserved_chars_in_values() {
        // '&' and '=' inside a note must be encoded so they can't break parsing.
        let uri = upi_uri("s@ok", "A & B = C", 100, "a&b=c");
        assert!(uri.contains("pn=A%20%26%20B%20%3D%20C"));
        assert!(uri.contains("tn=a%26b%3Dc"));
    }
}
