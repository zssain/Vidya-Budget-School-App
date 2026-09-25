//! ledger — double-entry vouchers (P13, foundation §8.3). Pure: no IO, no clock.
//!
//! Every money movement writes one **balanced** voucher (sum of debits = sum of
//! credits) made of [`Entry`] rows against system [accounts](system_accounts).
//! This module builds the entries and checks the balance ([`LEDGER_UNBALANCED`]);
//! the tables, numbering and posting live in `src-tauri::ledger`.
//!
//! Convention (Indian school books): assets & expenses are debit-normal; income &
//! liabilities/equity are credit-normal. A fee **receipt** debits the money
//! account (cash/bank/cheques) and credits Fee income. A **reversal** is the exact
//! opposite. An **expense** debits the expense account and credits the money paid.
//!
//! [`LEDGER_UNBALANCED`]: crate::errors::CoreError::LedgerUnbalanced

use crate::errors::{CoreError, CoreResult};
use crate::types::PaymentMode;

// ---- system account ids (also the `ledger_account.id`) ----------------------
pub const CASH: &str = "cash";
pub const BANK: &str = "bank"; // UPI / Bank
pub const CHEQUES: &str = "cheques"; // Cheques to deposit
pub const FEE_INCOME: &str = "fee_income";
pub const STORE_INCOME: &str = "store_income";
pub const STAFF_ADVANCES: &str = "staff_advances";
pub const SALARY_EXPENSE: &str = "salary_expense";
pub const ELECTRICITY: &str = "electricity";
pub const RENT: &str = "rent";
pub const REPAIRS: &str = "repairs"; // Repairs & maintenance
pub const STATIONERY: &str = "stationery";
pub const OTHER_EXPENSE: &str = "other_expense";
pub const OPENING_EQUITY: &str = "opening_equity";

/// The five account natures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountKind {
    Asset,
    Liability,
    Income,
    Expense,
}

impl AccountKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountKind::Asset => "asset",
            AccountKind::Liability => "liability",
            AccountKind::Income => "income",
            AccountKind::Expense => "expense",
        }
    }
}

/// A seed row for a system account: `(id, code, name_en, kind)`. Localised names
/// (`name_hi`, `name_te`) are filled by the migration / Telugu step; this list is
/// the single source of truth for ids, codes and kinds.
pub struct SystemAccount {
    pub id: &'static str,
    pub code: &'static str,
    pub name: &'static str,
    pub kind: AccountKind,
}

/// The system chart of accounts seeded in every school (§8.3).
pub fn system_accounts() -> Vec<SystemAccount> {
    use AccountKind::*;
    vec![
        SystemAccount { id: CASH, code: "1001", name: "Cash in hand", kind: Asset },
        SystemAccount { id: BANK, code: "1002", name: "UPI / Bank", kind: Asset },
        SystemAccount { id: CHEQUES, code: "1003", name: "Cheques to deposit", kind: Asset },
        SystemAccount { id: STAFF_ADVANCES, code: "1004", name: "Staff advances", kind: Asset },
        SystemAccount { id: FEE_INCOME, code: "4001", name: "Fee income", kind: Income },
        SystemAccount { id: STORE_INCOME, code: "4002", name: "Store income", kind: Income },
        SystemAccount { id: SALARY_EXPENSE, code: "5001", name: "Salary expense", kind: Expense },
        SystemAccount { id: ELECTRICITY, code: "5002", name: "Electricity", kind: Expense },
        SystemAccount { id: RENT, code: "5003", name: "Rent", kind: Expense },
        SystemAccount { id: REPAIRS, code: "5004", name: "Repairs & maintenance", kind: Expense },
        SystemAccount { id: STATIONERY, code: "5005", name: "Stationery", kind: Expense },
        SystemAccount { id: OTHER_EXPENSE, code: "5099", name: "Other expense", kind: Expense },
        SystemAccount { id: OPENING_EQUITY, code: "3001", name: "Opening balance equity", kind: Liability },
    ]
}

/// One line of a voucher: exactly one of `debit_paise` / `credit_paise` is > 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The account id (a `system_accounts` id such as [`CASH`]).
    pub account: &'static str,
    pub debit_paise: i64,
    pub credit_paise: i64,
}

impl Entry {
    pub fn debit(account: &'static str, paise: i64) -> Entry {
        Entry { account, debit_paise: paise, credit_paise: 0 }
    }
    pub fn credit(account: &'static str, paise: i64) -> Entry {
        Entry { account, debit_paise: 0, credit_paise: paise }
    }
}

/// The money (asset) account for a payment mode.
pub fn money_account_for_mode(mode: PaymentMode) -> &'static str {
    match mode {
        PaymentMode::Cash => CASH,
        PaymentMode::Upi => BANK,
        PaymentMode::Cheque => CHEQUES,
    }
}

/// A voucher is balanced iff total debits = total credits and the total is > 0.
/// Otherwise [`CoreError::LedgerUnbalanced`].
pub fn validate_balanced(entries: &[Entry]) -> CoreResult<()> {
    let debits: i64 = entries.iter().map(|e| e.debit_paise).sum();
    let credits: i64 = entries.iter().map(|e| e.credit_paise).sum();
    if debits != credits || debits <= 0 {
        return Err(CoreError::LedgerUnbalanced);
    }
    // Each entry is one-sided and non-negative.
    for e in entries {
        if e.debit_paise < 0 || e.credit_paise < 0 || (e.debit_paise > 0 && e.credit_paise > 0) {
            return Err(CoreError::LedgerUnbalanced);
        }
    }
    Ok(())
}

/// Entries for a fee **receipt**: Dr money account, Cr Fee income.
pub fn voucher_for_payment(mode: PaymentMode, amount_paise: i64) -> CoreResult<Vec<Entry>> {
    let entries = vec![
        Entry::debit(money_account_for_mode(mode), amount_paise),
        Entry::credit(FEE_INCOME, amount_paise),
    ];
    validate_balanced(&entries)?;
    Ok(entries)
}

/// Entries for a **reversal** of a fee receipt: Dr Fee income, Cr money account
/// (the exact opposite of the original receipt).
pub fn voucher_for_reversal(mode: PaymentMode, amount_paise: i64) -> CoreResult<Vec<Entry>> {
    let entries = vec![
        Entry::debit(FEE_INCOME, amount_paise),
        Entry::credit(money_account_for_mode(mode), amount_paise),
    ];
    validate_balanced(&entries)?;
    Ok(entries)
}

/// Entries for an **expense**: Dr expense account, Cr money account (P15 uses this).
pub fn voucher_for_expense(expense_account: &'static str, mode: PaymentMode, amount_paise: i64) -> CoreResult<Vec<Entry>> {
    let entries = vec![
        Entry::debit(expense_account, amount_paise),
        Entry::credit(money_account_for_mode(mode), amount_paise),
    ];
    validate_balanced(&entries)?;
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_debits_money_credits_fee_income() {
        let e = voucher_for_payment(PaymentMode::Cash, 50000).unwrap();
        assert_eq!(e, vec![Entry::debit(CASH, 50000), Entry::credit(FEE_INCOME, 50000)]);
        assert_eq!(money_account_for_mode(PaymentMode::Upi), BANK);
        assert_eq!(money_account_for_mode(PaymentMode::Cheque), CHEQUES);
    }

    #[test]
    fn reversal_is_the_exact_opposite() {
        let pay = voucher_for_payment(PaymentMode::Upi, 12345).unwrap();
        let rev = voucher_for_reversal(PaymentMode::Upi, 12345).unwrap();
        assert_eq!(rev[0], Entry::debit(FEE_INCOME, 12345));
        assert_eq!(rev[1], Entry::credit(BANK, 12345));
        // Net of the pair is zero on every account.
        for acc in [BANK, FEE_INCOME] {
            let net: i64 = pay.iter().chain(rev.iter()).filter(|e| e.account == acc).map(|e| e.debit_paise - e.credit_paise).sum();
            assert_eq!(net, 0, "receipt+reversal nets to zero on {acc}");
        }
    }

    #[test]
    fn expense_debits_expense_credits_money() {
        let e = voucher_for_expense(ELECTRICITY, PaymentMode::Cash, 80000).unwrap();
        assert_eq!(e, vec![Entry::debit(ELECTRICITY, 80000), Entry::credit(CASH, 80000)]);
    }

    #[test]
    fn unbalanced_or_nonpositive_rejected() {
        assert_eq!(validate_balanced(&[Entry::debit(CASH, 100), Entry::credit(FEE_INCOME, 90)]).unwrap_err().code(), "LEDGER_UNBALANCED");
        assert_eq!(validate_balanced(&[]).unwrap_err().code(), "LEDGER_UNBALANCED");
        assert_eq!(voucher_for_payment(PaymentMode::Cash, 0).unwrap_err().code(), "LEDGER_UNBALANCED");
        // A single entry both-sided is invalid.
        assert!(validate_balanced(&[Entry { account: CASH, debit_paise: 100, credit_paise: 100 }]).is_err());
    }

    #[test]
    fn system_accounts_are_unique_and_complete() {
        let accts = system_accounts();
        assert_eq!(accts.len(), 13);
        let mut ids: Vec<&str> = accts.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 13, "no duplicate account ids");
        // Every account the builders reference is seeded.
        for acc in [CASH, BANK, CHEQUES, FEE_INCOME, ELECTRICITY] {
            assert!(accts.iter().any(|a| a.id == acc), "{acc} must be seeded");
        }
    }
}
