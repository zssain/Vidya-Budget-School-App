//! numbering — the counter side of the numbering engine (P13, foundation §8.4).
//!
//! The format lives in `vidya_core::numbering`; this module manages
//! `number_series.last_seq` and, on first use of a (kind, series), seeds the
//! counter from any pre-engine rows (payments/vouchers created before the engine)
//! so the sequence continues instead of restarting. Call [`next_no`] inside the
//! surrounding write transaction so reserving a number is atomic with the write.

use rusqlite::{params, Connection, OptionalExtension};
use vidya_core::numbering::{format_number, NumberKind};

/// The highest sequence already issued for (kind, series) among pre-engine rows.
/// 0 for kinds with no prior history (store sale / circular / hall ticket).
fn seed_from_existing(conn: &Connection, kind: NumberKind, series: &str) -> rusqlite::Result<i64> {
    let (sql, like) = match kind {
        NumberKind::Receipt => (
            "SELECT COALESCE(MAX(CAST(substr(receipt_no, length(?1)+4) AS INTEGER)),0) FROM payment WHERE receipt_no LIKE ?2",
            format!("R-{series}-%"),
        ),
        NumberKind::Voucher => (
            "SELECT COALESCE(MAX(CAST(substr(voucher_no, length(?1)+4) AS INTEGER)),0) FROM voucher WHERE voucher_no LIKE ?2",
            format!("V-{series}-%"),
        ),
        _ => return Ok(0),
    };
    Ok(conn.query_row(sql, params![series, like], |r| r.get(0)).optional()?.unwrap_or(0))
}

/// Reserve and return the next number for (kind, series). Increments the counter
/// atomically; seeds it from existing rows on first use.
pub fn next_no(conn: &Connection, kind: NumberKind, series: &str) -> rusqlite::Result<String> {
    let exists = conn
        .query_row("SELECT 1 FROM number_series WHERE kind=?1 AND series=?2", params![kind.key(), series], |_| Ok(()))
        .optional()?
        .is_some();
    if !exists {
        let seed = seed_from_existing(conn, kind, series)?;
        conn.execute(
            "INSERT INTO number_series(kind, series, prefix, last_seq) VALUES (?1,?2,?3,?4)",
            params![kind.key(), series, kind.prefix(), seed],
        )?;
    }
    conn.execute("UPDATE number_series SET last_seq = last_seq + 1 WHERE kind=?1 AND series=?2", params![kind.key(), series])?;
    let seq: i64 = conn.query_row("SELECT last_seq FROM number_series WHERE kind=?1 AND series=?2", params![kind.key(), series], |r| r.get(0))?;
    Ok(format_number(kind, series, seq as u32))
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
    fn counts_up_per_series_and_kind() {
        let c = fresh();
        assert_eq!(next_no(&c, NumberKind::StoreSale, "A2").unwrap(), "S-A2-0001");
        assert_eq!(next_no(&c, NumberKind::StoreSale, "A2").unwrap(), "S-A2-0002");
        // Different series is a separate counter.
        assert_eq!(next_no(&c, NumberKind::StoreSale, "A3").unwrap(), "S-A3-0001");
        // Different kind is a separate counter.
        assert_eq!(next_no(&c, NumberKind::HallTicket, "A2").unwrap(), "HT-A2-0001");
        // Circular counts by session, three digits.
        assert_eq!(next_no(&c, NumberKind::Circular, "2026-27").unwrap(), "CIR/2026-27/001");
    }

    #[test]
    fn receipt_counter_continues_from_existing_payments() {
        let c = fresh();
        // A pre-engine payment R-A2-0418 exists (like the demo seed).
        c.execute("PRAGMA foreign_keys=OFF", []).unwrap();
        c.execute(
            "INSERT INTO payment(id,receipt_no,student_id,amount_paise,mode,collected_by,collected_at,created_at,updated_at) \
             VALUES ('p1','R-A2-0418','s1',1000,'cash','st1','t','t','t')",
            [],
        )
        .unwrap();
        // The engine continues from 418 → 419 (output identical to the old scheme).
        assert_eq!(next_no(&c, NumberKind::Receipt, "A2").unwrap(), "R-A2-0419");
    }
}
