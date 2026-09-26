//! ledger — repository for double-entry vouchers (P13, foundation §8.3).
//!
//! vidya-core builds the balanced entries; this module numbers vouchers, posts
//! them (voucher + ledger_entry, append-only) and backfills derived vouchers for
//! existing payments/reversals. The day book / dashboards keep computing money
//! totals from `payment`/`reversal`, so the ledger is purely additive — it never
//! changes a money total.

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use vidya_core::ledger::{self, Entry};
use vidya_core::types::PaymentMode;

/// The date part (`YYYY-MM-DD`) of an ISO timestamp.
fn date_of(iso: &str) -> String {
    iso.get(0..10).unwrap_or(iso).to_string()
}

/// The voucher series for a payment: reuse the series embedded in its receipt
/// number (`R-<series>-…`), else `A1` (the server series).
fn series_for_receipt(receipt_no: &str) -> String {
    vidya_core::receipts::parse_receipt_no(receipt_no).map(|(s, _)| s).unwrap_or_else(|| "A1".to_string())
}

/// Post one balanced voucher and its ledger entries inside an open transaction.
/// `entries` must already be balanced (built by a `vidya_core::ledger` builder).
#[allow(clippy::too_many_arguments)]
pub fn post_voucher(
    tx: &Transaction,
    id: &str,
    voucher_no: &str,
    kind: &str,
    date: &str,
    narration: Option<&str>,
    source_table: &str,
    source_id: &str,
    created_by: Option<&str>,
    device_id: Option<&str>,
    school_id: Option<&str>,
    entries: &[Entry],
    now: &str,
    sync_state: &str,
) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO voucher(id, voucher_no, kind, date, narration, source_table, source_id, created_by, device_id, school_id, created_at, updated_at, sync_state) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?11,?12)",
        params![id, voucher_no, kind, date, narration, source_table, source_id, created_by, device_id, school_id, now, sync_state],
    )?;
    for e in entries {
        tx.execute(
            "INSERT INTO ledger_entry(id, voucher_id, account_id, debit_paise, credit_paise) VALUES (?1,?2,?3,?4,?5)",
            params![format!("le-{}", uuid::Uuid::now_v7()), id, e.account, e.debit_paise, e.credit_paise],
        )?;
    }
    Ok(())
}

/// Voucher id + entries for a payment/reversal already validated as balanced.
/// Returns an error only if the amount is non-positive (never for real data).
fn build(entries: Result<Vec<Entry>, vidya_core::errors::CoreError>) -> rusqlite::Result<Vec<Entry>> {
    entries.map_err(|_| rusqlite::Error::InvalidQuery)
}

/// True if a voucher already exists for `(source_table, source_id)`.
fn has_voucher(conn: &Connection, source_table: &str, source_id: &str) -> rusqlite::Result<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM voucher WHERE source_table=?1 AND source_id=?2 LIMIT 1",
            params![source_table, source_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

/// Create the receipt voucher for a payment inside an open transaction (called by
/// record_payment on the server). No-op if one already exists.
#[allow(clippy::too_many_arguments)]
pub fn post_payment_voucher(
    tx: &Transaction,
    payment_id: &str,
    receipt_no: &str,
    mode: PaymentMode,
    amount_paise: i64,
    date_iso: &str,
    created_by: Option<&str>,
    device_id: Option<&str>,
    school_id: Option<&str>,
    now: &str,
    sync_state: &str,
) -> rusqlite::Result<()> {
    if has_voucher(tx, "payment", payment_id)? {
        return Ok(());
    }
    let series = series_for_receipt(receipt_no);
    let voucher_no = crate::numbering::next_no(tx, vidya_core::numbering::NumberKind::Voucher, &series)?;
    let entries = build(ledger::voucher_for_payment(mode, amount_paise))?;
    let narration = format!("Fee receipt {receipt_no}");
    post_voucher(
        tx, &format!("vch-{}", uuid::Uuid::now_v7()), &voucher_no, "receipt", &date_of(date_iso),
        Some(&narration), "payment", payment_id, created_by, device_id, school_id, &entries, now, sync_state,
    )
}

/// Create the reversal voucher for a reversal row inside an open transaction
/// (called when a payment_reversal request is approved). No-op if one exists.
#[allow(clippy::too_many_arguments)]
pub fn post_reversal_voucher(
    tx: &Transaction,
    reversal_id: &str,
    receipt_no: &str,
    mode: PaymentMode,
    amount_paise: i64,
    date_iso: &str,
    created_by: Option<&str>,
    school_id: Option<&str>,
    now: &str,
    sync_state: &str,
) -> rusqlite::Result<()> {
    if has_voucher(tx, "reversal", reversal_id)? {
        return Ok(());
    }
    let series = series_for_receipt(receipt_no);
    let voucher_no = crate::numbering::next_no(tx, vidya_core::numbering::NumberKind::Voucher, &series)?;
    let entries = build(ledger::voucher_for_reversal(mode, amount_paise))?;
    let narration = format!("Reversal of receipt {receipt_no}");
    post_voucher(
        tx, &format!("vch-{}", uuid::Uuid::now_v7()), &voucher_no, "reversal", &date_of(date_iso),
        Some(&narration), "reversal", reversal_id, created_by, None, school_id, &entries, now, sync_state,
    )
}

fn parse_mode(s: &str) -> PaymentMode {
    match s {
        "upi" => PaymentMode::Upi,
        "cheque" => PaymentMode::Cheque,
        _ => PaymentMode::Cash,
    }
}

/// Backfill derived vouchers for every payment/reversal that has none yet
/// (idempotent). Derived rows only — payments/reversals are never touched, so day
/// and mode money totals are unchanged. Returns how many vouchers were created.
pub fn backfill_vouchers(conn: &mut Connection) -> rusqlite::Result<usize> {
    let school_id: Option<String> = conn.query_row("SELECT id FROM school LIMIT 1", [], |r| r.get(0)).optional()?;
    let now = crate::db::now_iso();
    let mut created = 0usize;

    // (id, amount, mode, receipt_no, collected_at, collected_by)
    type PaymentRow = (String, i64, String, String, Option<String>, Option<String>);
    // (reversal_id, amount, mode, receipt_no, applied_at)
    type ReversalRow = (String, i64, String, String, String);

    // Payments → receipt vouchers.
    let payments: Vec<PaymentRow> = {
        let mut stmt = conn.prepare(
            "SELECT p.id, p.amount_paise, p.mode, p.receipt_no, p.collected_at, p.collected_by \
             FROM payment p WHERE p.sync_state='confirmed' \
               AND NOT EXISTS (SELECT 1 FROM voucher v WHERE v.source_table='payment' AND v.source_id=p.id) \
             ORDER BY p.collected_at, p.receipt_no",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    // Reversals → reversal vouchers (join the original payment for mode + amount).
    let reversals: Vec<ReversalRow> = {
        let mut stmt = conn.prepare(
            "SELECT r.id, p.amount_paise, p.mode, p.receipt_no, r.applied_at \
             FROM reversal r JOIN payment p ON p.id=r.payment_id \
             WHERE NOT EXISTS (SELECT 1 FROM voucher v WHERE v.source_table='reversal' AND v.source_id=r.id) \
             ORDER BY r.applied_at",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };

    // Expenses (P15) → expense vouchers; expense_reversals → reversal vouchers.
    // Voucher series continues from the school's own series ("A1" on the server).
    // (id, category, paid_via, amount, spent_on, created_by)
    type ExpenseRow = (String, String, String, i64, String, Option<String>);
    let expenses: Vec<ExpenseRow> = {
        let mut stmt = conn.prepare(
            "SELECT e.id, e.category_account_id, e.paid_via, e.amount_paise, e.spent_on, e.created_by \
             FROM expense e WHERE e.sync_state='confirmed' \
               AND NOT EXISTS (SELECT 1 FROM voucher v WHERE v.source_table='expense' AND v.source_id=e.id) \
             ORDER BY e.spent_on, e.created_at",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    // (reversal_id, expense category, paid_via, amount, applied_at)
    type ExpRevRow = (String, String, String, i64, String);
    let exp_reversals: Vec<ExpRevRow> = {
        let mut stmt = conn.prepare(
            "SELECT r.id, e.category_account_id, e.paid_via, e.amount_paise, r.applied_at \
             FROM expense_reversal r JOIN expense e ON e.id=r.expense_id \
             WHERE NOT EXISTS (SELECT 1 FROM voucher v WHERE v.source_table='expense_reversal' AND v.source_id=r.id) \
             ORDER BY r.applied_at",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };

    // Staff advances (P15) → advance vouchers (Dr staff advances, Cr cash).
    // (id, amount, given_on, created_by)
    type AdvanceRow = (String, i64, String, Option<String>);
    let advances: Vec<AdvanceRow> = {
        let mut stmt = conn.prepare(
            "SELECT a.id, a.amount_paise, a.given_on, a.created_by FROM staff_advance a \
             WHERE a.sync_state='confirmed' \
               AND NOT EXISTS (SELECT 1 FROM voucher v WHERE v.source_table='staff_advance' AND v.source_id=a.id) \
             ORDER BY a.given_on",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };

    let tx = conn.transaction()?;
    for (aid, amount, given_on, created_by) in advances {
        let vno = crate::numbering::next_no(&tx, vidya_core::numbering::NumberKind::Voucher, "A1")?;
        post_advance_voucher(&tx, &vno, &aid, vidya_core::ledger::CASH, amount, &date_of(&given_on), created_by.as_deref(), None, school_id.as_deref(), &now, "confirmed")?;
        created += 1;
    }
    for (pid, amount, mode, receipt_no, collected_at, collected_by) in payments {
        let date = collected_at.unwrap_or_else(|| now.clone());
        post_payment_voucher(&tx, &pid, &receipt_no, parse_mode(&mode), amount, &date, collected_by.as_deref(), None, school_id.as_deref(), &now, "confirmed")?;
        created += 1;
    }
    for (rid, amount, mode, receipt_no, applied_at) in reversals {
        post_reversal_voucher(&tx, &rid, &receipt_no, parse_mode(&mode), amount, &applied_at, None, school_id.as_deref(), &now, "confirmed")?;
        created += 1;
    }
    for (eid, category, paid_via, amount, spent_on, created_by) in expenses {
        let pv = vidya_core::accounts::PaidVia::parse(&paid_via).unwrap_or(vidya_core::accounts::PaidVia::Cash);
        let vno = crate::numbering::next_no(&tx, vidya_core::numbering::NumberKind::Voucher, "A1")?;
        post_expense_voucher(&tx, &vno, &eid, &category, pv, amount, &spent_on, created_by.as_deref(), None, school_id.as_deref(), &now, "confirmed")?;
        created += 1;
    }
    for (rid, category, paid_via, amount, applied_at) in exp_reversals {
        let pv = vidya_core::accounts::PaidVia::parse(&paid_via).unwrap_or(vidya_core::accounts::PaidVia::Cash);
        let vno = crate::numbering::next_no(&tx, vidya_core::numbering::NumberKind::Voucher, "A1")?;
        post_expense_reversal_voucher(&tx, &vno, &rid, &category, pv, amount, &applied_at, None, school_id.as_deref(), &now, "confirmed")?;
        created += 1;
    }
    tx.commit()?;
    Ok(created)
}

/// Post a voucher with two DYNAMIC ledger accounts (ids from the DB, not the
/// 'static system consts). Used by expenses/salary/store whose category account
/// id comes from a row. `debit_account`/`credit_account` each take `amount_paise`
/// (so the voucher is balanced by construction).
#[allow(clippy::too_many_arguments)]
pub fn post_voucher_dyn(
    tx: &Transaction,
    voucher_no: &str,
    kind: &str,
    date: &str,
    narration: Option<&str>,
    source_table: &str,
    source_id: &str,
    debit_account: &str,
    credit_account: &str,
    amount_paise: i64,
    created_by: Option<&str>,
    device_id: Option<&str>,
    school_id: Option<&str>,
    now: &str,
    sync_state: &str,
) -> rusqlite::Result<String> {
    let vid = format!("vch-{}", uuid::Uuid::now_v7());
    tx.execute(
        "INSERT INTO voucher(id, voucher_no, kind, date, narration, source_table, source_id, created_by, device_id, school_id, created_at, updated_at, sync_state) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?11,?12)",
        params![vid, voucher_no, kind, date, narration, source_table, source_id, created_by, device_id, school_id, now, sync_state],
    )?;
    tx.execute(
        "INSERT INTO ledger_entry(id, voucher_id, account_id, debit_paise, credit_paise) VALUES (?1,?2,?3,?4,0)",
        params![format!("le-{}", uuid::Uuid::now_v7()), vid, debit_account, amount_paise],
    )?;
    tx.execute(
        "INSERT INTO ledger_entry(id, voucher_id, account_id, debit_paise, credit_paise) VALUES (?1,?2,?3,0,?4)",
        params![format!("le-{}", uuid::Uuid::now_v7()), vid, credit_account, amount_paise],
    )?;
    Ok(vid)
}

/// Post the expense voucher (Dr category account, Cr money account for paid_via).
/// Returns the new voucher id. Called by record_expense in the same transaction.
#[allow(clippy::too_many_arguments)]
pub fn post_expense_voucher(
    tx: &Transaction,
    voucher_no: &str,
    expense_id: &str,
    category_account_id: &str,
    paid_via: vidya_core::accounts::PaidVia,
    amount_paise: i64,
    date_iso: &str,
    created_by: Option<&str>,
    device_id: Option<&str>,
    school_id: Option<&str>,
    now: &str,
    sync_state: &str,
) -> rusqlite::Result<String> {
    post_voucher_dyn(
        tx, voucher_no, "expense", &date_of(date_iso), Some("Expense"), "expense", expense_id,
        category_account_id, paid_via.money_account(), amount_paise,
        created_by, device_id, school_id, now, sync_state,
    )
}

/// Post the expense-reversal voucher (Dr money account, Cr category — the exact
/// opposite of the original expense). Principal only.
#[allow(clippy::too_many_arguments)]
pub fn post_expense_reversal_voucher(
    tx: &Transaction,
    voucher_no: &str,
    reversal_id: &str,
    category_account_id: &str,
    paid_via: vidya_core::accounts::PaidVia,
    amount_paise: i64,
    date_iso: &str,
    created_by: Option<&str>,
    school_id: Option<&str>,
    now: &str,
    sync_state: &str,
) -> rusqlite::Result<String> {
    post_voucher_dyn(
        tx, voucher_no, "reversal", &date_of(date_iso), Some("Expense reversal"), "expense_reversal", reversal_id,
        paid_via.money_account(), category_account_id, amount_paise,
        created_by, None, school_id, now, sync_state,
    )
}

/// Post an advance voucher (Dr staff advances, Cr money account). Returns the id.
#[allow(clippy::too_many_arguments)]
pub fn post_advance_voucher(
    tx: &Transaction,
    voucher_no: &str,
    advance_id: &str,
    money_account: &str,
    amount_paise: i64,
    date: &str,
    created_by: Option<&str>,
    device_id: Option<&str>,
    school_id: Option<&str>,
    now: &str,
    sync_state: &str,
) -> rusqlite::Result<String> {
    post_voucher_dyn(
        tx, voucher_no, "advance", date, Some("Staff advance"), "staff_advance", advance_id,
        vidya_core::ledger::STAFF_ADVANCES, money_account, amount_paise,
        created_by, device_id, school_id, now, sync_state,
    )
}

/// Post a salary voucher: Dr Salary expense (earned = net + recovery), Cr Staff
/// advances (recovery), Cr money account (net). Balanced by construction. Returns
/// the voucher id. `earned` must equal `recovery + net`.
#[allow(clippy::too_many_arguments)]
pub fn post_salary_voucher(
    tx: &Transaction,
    voucher_no: &str,
    line_id: &str,
    money_account: &str,
    earned_paise: i64,
    recovery_paise: i64,
    net_paise: i64,
    date: &str,
    created_by: Option<&str>,
    device_id: Option<&str>,
    school_id: Option<&str>,
    now: &str,
    sync_state: &str,
) -> rusqlite::Result<String> {
    let vid = format!("vch-{}", uuid::Uuid::now_v7());
    tx.execute(
        "INSERT INTO voucher(id, voucher_no, kind, date, narration, source_table, source_id, created_by, device_id, school_id, created_at, updated_at, sync_state) \
         VALUES (?1,?2,'salary',?3,'Salary',?4,?5,?6,?7,?8,?9,?9,?10)",
        params![vid, voucher_no, date, "salary_line", line_id, created_by, device_id, school_id, now, sync_state],
    )?;
    let add = |account: &str, debit: i64, credit: i64| -> rusqlite::Result<()> {
        if debit > 0 || credit > 0 {
            tx.execute(
                "INSERT INTO ledger_entry(id, voucher_id, account_id, debit_paise, credit_paise) VALUES (?1,?2,?3,?4,?5)",
                params![format!("le-{}", uuid::Uuid::now_v7()), vid, account, debit, credit],
            )?;
        }
        Ok(())
    };
    add(vidya_core::ledger::SALARY_EXPENSE, earned_paise, 0)?;
    add(vidya_core::ledger::STAFF_ADVANCES, 0, recovery_paise)?;
    add(money_account, 0, net_paise)?;
    Ok(vid)
}

/// Post a store-sale voucher (Dr money account for the mode, Cr Store income).
#[allow(clippy::too_many_arguments)]
pub fn post_store_sale_voucher(
    tx: &Transaction,
    voucher_no: &str,
    sale_id: &str,
    mode: PaymentMode,
    total_paise: i64,
    date_iso: &str,
    created_by: Option<&str>,
    device_id: Option<&str>,
    school_id: Option<&str>,
    now: &str,
    sync_state: &str,
) -> rusqlite::Result<String> {
    post_voucher_dyn(
        tx, voucher_no, "store_sale", &date_of(date_iso), Some("Store sale"), "store_sale", sale_id,
        ledger::money_account_for_mode(mode), vidya_core::ledger::STORE_INCOME, total_paise,
        created_by, device_id, school_id, now, sync_state,
    )
}

/// Total debits minus credits across all ledger entries — must be exactly 0 if
/// every voucher is balanced (a whole-book invariant used by tests).
pub fn ledger_imbalance(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("SELECT COALESCE(SUM(debit_paise - credit_paise),0) FROM ledger_entry", [], |r| r.get(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn seeded() -> Connection {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        let now = time::OffsetDateTime::parse("2026-09-23T09:00:00Z", &time::format_description::well_known::Rfc3339).unwrap();
        crate::seed::seed_demo_school(&mut c, now).unwrap();
        c
    }

    fn day_mode_totals(conn: &Connection) -> Vec<(String, String, i64)> {
        let mut stmt = conn
            .prepare("SELECT substr(collected_at,1,10), mode, SUM(amount_paise) FROM payment GROUP BY 1,2 ORDER BY 1,2")
            .unwrap();
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))).unwrap();
        rows.collect::<rusqlite::Result<_>>().unwrap()
    }

    #[test]
    fn backfill_is_balanced_idempotent_and_preserves_money_totals() {
        let mut c = seeded();
        // The seed inserts payments directly; the seed also backfills, so vouchers
        // already exist. Money totals BEFORE any (re)backfill:
        let before = day_mode_totals(&c);
        let payments: i64 = c.query_row("SELECT COUNT(*) FROM payment", [], |r| r.get(0)).unwrap();
        assert!(payments > 0, "seed has payments");

        // Every CONFIRMED payment has exactly one receipt voucher (on_device
        // payments are not yet real money, so they carry no voucher — P15).
        let confirmed: i64 = c.query_row("SELECT COUNT(*) FROM payment WHERE sync_state='confirmed'", [], |r| r.get(0)).unwrap();
        let vouchers: i64 = c.query_row("SELECT COUNT(*) FROM voucher WHERE source_table='payment'", [], |r| r.get(0)).unwrap();
        assert_eq!(vouchers, confirmed, "one receipt voucher per confirmed payment");
        assert!(confirmed < payments, "the seed has one on_device payment with no voucher");

        // The whole book balances (Σ debit = Σ credit).
        assert_eq!(ledger_imbalance(&c).unwrap(), 0, "ledger balances");

        // Re-running the backfill creates nothing (idempotent).
        assert_eq!(backfill_vouchers(&mut c).unwrap(), 0);

        // Money totals per day+mode are identical (ledger is additive).
        assert_eq!(day_mode_totals(&c), before);
    }

    #[test]
    fn each_voucher_is_internally_balanced() {
        let c = seeded();
        let mut stmt = c
            .prepare("SELECT voucher_id, SUM(debit_paise), SUM(credit_paise) FROM ledger_entry GROUP BY voucher_id")
            .unwrap();
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))).unwrap();
        for row in rows {
            let (vid, d, cr) = row.unwrap();
            assert_eq!(d, cr, "voucher {vid} debits must equal credits");
            assert!(d > 0, "voucher {vid} must be non-empty");
        }
    }

    #[test]
    fn cash_book_money_in_equals_day_book_total_and_book_balances_every_day() {
        // DONE-MEANS (§Step 4/8): for every day in the demo seed, cash-book money
        // in = the day's confirmed receipts (day book total), and Σ debits = Σ
        // credits across the whole book.
        let c = seeded();
        assert_eq!(ledger_imbalance(&c).unwrap(), 0, "the whole book balances");

        // Distinct days that carry a confirmed receipt.
        let days: Vec<String> = {
            let mut s = c.prepare("SELECT DISTINCT substr(collected_at,1,10) FROM payment WHERE sync_state='confirmed' ORDER BY 1").unwrap();
            s.query_map([], |r| r.get::<_, String>(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap()
        };
        assert!(!days.is_empty(), "seed has confirmed receipts");
        for day in days {
            // Day book total = confirmed receipts collected that day.
            let day_total: i64 = c.query_row(
                "SELECT COALESCE(SUM(amount_paise),0) FROM payment WHERE sync_state='confirmed' AND substr(collected_at,1,10)=?1",
                params![day], |r| r.get(0)).unwrap();
            // Cash-book money in = Σ debits to cash/bank/cheques for vouchers dated that day.
            let money_in: i64 = c.query_row(
                "SELECT COALESCE(SUM(le.debit_paise),0) FROM ledger_entry le JOIN voucher v ON v.id=le.voucher_id \
                 WHERE le.account_id IN ('cash','bank','cheques') AND v.date=?1",
                params![day], |r| r.get(0)).unwrap();
            assert_eq!(money_in, day_total, "cash-book money in = day book total for {day}");
        }
    }

    #[test]
    fn vouchers_are_append_only() {
        let c = seeded();
        let vid: String = c.query_row("SELECT id FROM voucher LIMIT 1", [], |r| r.get(0)).unwrap();
        assert!(c.execute("UPDATE voucher SET narration='x' WHERE id=?1", params![vid]).is_err());
        assert!(c.execute("DELETE FROM voucher WHERE id=?1", params![vid]).is_err());
        assert!(c.execute("UPDATE ledger_entry SET debit_paise=0", []).is_err());
    }
}
