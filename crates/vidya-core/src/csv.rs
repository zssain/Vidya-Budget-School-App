//! csv — a small RFC 4180 CSV writer/reader plus spreadsheet formula-injection
//! guarding (pure, no IO).
//!
//! * [`write_csv`] quotes fields that contain `"`, comma, CR or LF, doubles any
//!   `"`, and joins with CRLF line endings.
//! * [`write_csv_excel`] is [`write_csv`] with a leading UTF-8 BOM so Excel
//!   opens UTF-8 text (e.g. Devanagari) correctly.
//! * [`read_csv`] parses RFC 4180, including quoted fields with embedded commas,
//!   quotes and newlines, stripping a leading BOM if present.
//! * [`escape_formula`] prefixes a `'` to any cell that a spreadsheet might
//!   interpret as a formula.

/// UTF-8 byte-order mark.
const BOM: &str = "\u{feff}";

/// Whether a field must be quoted per RFC 4180.
fn needs_quoting(field: &str) -> bool {
    field.contains('"') || field.contains(',') || field.contains('\r') || field.contains('\n')
}

/// Write one field, quoting and escaping if required.
fn write_field(out: &mut String, field: &str) {
    if needs_quoting(field) {
        out.push('"');
        for ch in field.chars() {
            if ch == '"' {
                out.push('"'); // escape " as ""
            }
            out.push(ch);
        }
        out.push('"');
    } else {
        out.push_str(field);
    }
}

/// Write `rows` as an RFC 4180 CSV string (CRLF line endings).
///
/// A field is quoted when it contains `"`, a comma, CR or LF; embedded `"` are
/// doubled. Every row (including the last) is terminated with `\r\n`.
///
/// ```
/// use vidya_core::csv::write_csv;
/// let s = write_csv(&[vec!["a".into(), "b,c".into()]]);
/// assert_eq!(s, "a,\"b,c\"\r\n");
/// ```
pub fn write_csv(rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    for row in rows {
        for (i, field) in row.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write_field(&mut out, field);
        }
        out.push_str("\r\n");
    }
    out
}

/// Write `rows` as RFC 4180 CSV bytes prefixed with the UTF-8 BOM, so Excel
/// reads the file as UTF-8 (needed for Devanagari and other non-ASCII text).
pub fn write_csv_excel(rows: &[Vec<String>]) -> Vec<u8> {
    let mut out = String::with_capacity(BOM.len());
    out.push_str(BOM);
    out.push_str(&write_csv(rows));
    out.into_bytes()
}

/// Parse an RFC 4180 CSV string into rows of fields.
///
/// Handles quoted fields containing commas, quotes (`""` → `"`) and newlines. A
/// leading UTF-8 BOM is stripped. Both `\r\n` and bare `\n` are accepted as row
/// terminators between records. A trailing newline does not yield an extra empty
/// row; input with no trailing newline still yields its final row.
///
/// ```
/// use vidya_core::csv::read_csv;
/// assert_eq!(read_csv("a,\"b,c\"\r\n"), vec![vec!["a".to_string(), "b,c".to_string()]]);
/// ```
pub fn read_csv(input: &str) -> Vec<Vec<String>> {
    let input = input.strip_prefix(BOM).unwrap_or(input);

    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    // Whether the current record has seen any content (so we know a lone
    // trailing newline is a terminator, not the start of an empty row).
    let mut started = false;

    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_quotes {
            match ch {
                '"' => {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        field.push('"'); // "" → literal "
                    } else {
                        in_quotes = false;
                    }
                }
                _ => field.push(ch),
            }
            continue;
        }

        match ch {
            '"' => {
                in_quotes = true;
                started = true;
            }
            ',' => {
                row.push(std::mem::take(&mut field));
                started = true;
            }
            '\r' => {
                // Swallow the paired \n of a CRLF; end the record.
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                started = false;
            }
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                started = false;
            }
            _ => {
                field.push(ch);
                started = true;
            }
        }
    }

    // Flush a final record that had no trailing newline.
    if started || !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }

    rows
}

/// Guard a cell against spreadsheet formula injection.
///
/// If the cell STARTS WITH `=`, `+`, `-`, `@`, TAB (0x09) or CR (0x0D), prefix a
/// single quote `'`; otherwise return it unchanged.
///
/// ```
/// use vidya_core::csv::escape_formula;
/// assert_eq!(escape_formula("=SUM(A1)"), "'=SUM(A1)");
/// assert_eq!(escape_formula("safe"), "safe");
/// ```
pub fn escape_formula(cell: &str) -> String {
    let dangerous = matches!(
        cell.chars().next(),
        Some('=') | Some('+') | Some('-') | Some('@') | Some('\t') | Some('\r')
    );
    if dangerous {
        let mut s = String::with_capacity(cell.len() + 1);
        s.push('\'');
        s.push_str(cell);
        s
    } else {
        cell.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_fields_unquoted() {
        assert_eq!(write_csv(&[vec!["a".into(), "b".into()]]), "a,b\r\n");
    }

    #[test]
    fn quotes_comma_quote_and_newline() {
        let rows = vec![vec![
            "has,comma".into(),
            "has\"quote".into(),
            "has\nnewline".into(),
        ]];
        let s = write_csv(&rows);
        assert_eq!(s, "\"has,comma\",\"has\"\"quote\",\"has\nnewline\"\r\n");
    }

    #[test]
    fn round_trip_comma_quote_newline() {
        let rows = vec![
            vec!["a,b".into(), "c\"d".into(), "e\nf".into()],
            vec!["plain".into(), "".into(), "trailing space ".into()],
        ];
        let written = write_csv(&rows);
        assert_eq!(read_csv(&written), rows);
    }

    #[test]
    fn round_trip_field_with_crlf_inside() {
        // A quoted field containing a CRLF must survive the round-trip intact.
        let rows = vec![vec!["line1\r\nline2".into(), "x".into()]];
        let written = write_csv(&rows);
        assert_eq!(read_csv(&written), rows);
    }

    #[test]
    fn read_accepts_bare_lf_line_endings() {
        assert_eq!(
            read_csv("a,b\nc,d\n"),
            vec![
                vec!["a".to_string(), "b".to_string()],
                vec!["c".to_string(), "d".to_string()]
            ]
        );
    }

    #[test]
    fn read_final_row_without_trailing_newline() {
        assert_eq!(read_csv("a,b"), vec![vec!["a".to_string(), "b".to_string()]]);
    }

    #[test]
    fn excel_output_has_bom() {
        let bytes = write_csv_excel(&[vec!["a".into()]]);
        assert_eq!(&bytes[0..3], &[0xEF, 0xBB, 0xBF]);
        // The rest is the plain CSV.
        assert_eq!(&bytes[3..], b"a\r\n");
    }

    #[test]
    fn read_strips_leading_bom() {
        let mut input = String::from("\u{feff}");
        input.push_str("a,b\r\n");
        assert_eq!(read_csv(&input), vec![vec!["a".to_string(), "b".to_string()]]);
    }

    #[test]
    fn devanagari_round_trips() {
        // "राधा शर्मा" (a name) with a comma so it also exercises quoting.
        let rows = vec![vec!["राधा, शर्मा".into(), "कक्षा ७".into()]];
        let written = write_csv_excel(&rows);
        let text = String::from_utf8(written).unwrap();
        assert_eq!(read_csv(&text), rows);
    }

    #[test]
    fn escape_formula_dangerous_prefixes() {
        assert_eq!(escape_formula("=SUM(A1)"), "'=SUM(A1)");
        assert_eq!(escape_formula("+1+1"), "'+1+1");
        assert_eq!(escape_formula("-1"), "'-1");
        assert_eq!(escape_formula("@import"), "'@import");
        assert_eq!(escape_formula("\tTAB"), "'\tTAB");
        assert_eq!(escape_formula("\rCR"), "'\rCR");
    }

    #[test]
    fn escape_formula_safe_unchanged() {
        assert_eq!(escape_formula("safe"), "safe");
        assert_eq!(escape_formula("123"), "123");
        assert_eq!(escape_formula(""), "");
        assert_eq!(escape_formula("राधा"), "राधा");
        // A dangerous character NOT at the start is left alone.
        assert_eq!(escape_formula("a=b"), "a=b");
    }
}
