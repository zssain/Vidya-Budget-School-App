//! accounts — school-accounts business rules (P15, §10.3). Pure: no IO, no clock.
//!
//! Expense validation, the opening-balance rule and the cash-in-hand warning.
//! Money is integer **paise** (`i64`). The double-entry voucher builders live in
//! [`crate::ledger`]; posting lives in `src-tauri::ledger`.

use crate::errors::{CoreError, CoreResult};

/// How an expense was paid (§10.3). `upi` and `bank` both settle the bank account;
/// `cash` settles cash in hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaidVia {
    Cash,
    Upi,
    Bank,
}

impl PaidVia {
    pub fn as_str(self) -> &'static str {
        match self {
            PaidVia::Cash => "cash",
            PaidVia::Upi => "upi",
            PaidVia::Bank => "bank",
        }
    }
    pub fn parse(s: &str) -> Option<PaidVia> {
        Some(match s {
            "cash" => PaidVia::Cash,
            "upi" => PaidVia::Upi,
            "bank" => PaidVia::Bank,
            _ => return None,
        })
    }
    /// The money (asset) account this settles: cash → [`crate::ledger::CASH`],
    /// UPI/bank → [`crate::ledger::BANK`].
    pub fn money_account(self) -> &'static str {
        match self {
            PaidVia::Cash => crate::ledger::CASH,
            PaidVia::Upi | PaidVia::Bank => crate::ledger::BANK,
        }
    }
}

/// Validate an expense (§10.3, Step 3). The hard rules only — the cash-in-hand
/// check is a *warning* (see [`cash_would_go_negative`]), never a block.
///
/// * `amount_paise > 0` → else `Validation{amount}`;
/// * the category must be an **active expense account** → else
///   `Validation{category}`.
pub fn validate_expense(amount_paise: i64, category_active_expense: bool) -> CoreResult<()> {
    if amount_paise <= 0 {
        return Err(CoreError::validation("amount", "positive"));
    }
    if !category_active_expense {
        return Err(CoreError::validation("category", "not_active_expense_account"));
    }
    Ok(())
}

/// Whether a **cash** expense would drive cash in hand negative — a WARNING only
/// (**[OWNER default]**: warn, never block). Returns false for non-cash modes.
pub fn cash_would_go_negative(cash_in_hand_paise: i64, paid_via: PaidVia, amount_paise: i64) -> bool {
    matches!(paid_via, PaidVia::Cash) && amount_paise > cash_in_hand_paise
}

/// Validate an opening-balance entry (§10.3, Step 4): both amounts ≥ 0 and at
/// least one is positive (an all-zero opening voucher would be unbalanced).
pub fn validate_opening_balance(cash_paise: i64, bank_paise: i64) -> CoreResult<()> {
    if cash_paise < 0 || bank_paise < 0 {
        return Err(CoreError::validation("opening", "negative"));
    }
    if cash_paise == 0 && bank_paise == 0 {
        return Err(CoreError::validation("opening", "empty"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expense_amount_must_be_positive_and_category_active() {
        assert!(validate_expense(80000, true).is_ok());
        assert_eq!(validate_expense(0, true), Err(CoreError::validation("amount", "positive")));
        assert_eq!(validate_expense(-1, true), Err(CoreError::validation("amount", "positive")));
        assert_eq!(
            validate_expense(80000, false),
            Err(CoreError::validation("category", "not_active_expense_account"))
        );
    }

    #[test]
    fn cash_warning_only_for_cash_and_over_balance() {
        // Cash expense above cash in hand → warning.
        assert!(cash_would_go_negative(50000, PaidVia::Cash, 60000));
        // Cash expense within cash in hand → no warning.
        assert!(!cash_would_go_negative(50000, PaidVia::Cash, 50000));
        // Non-cash never warns (settled from the bank).
        assert!(!cash_would_go_negative(0, PaidVia::Upi, 999999));
        assert!(!cash_would_go_negative(0, PaidVia::Bank, 999999));
    }

    #[test]
    fn paid_via_maps_to_money_account() {
        assert_eq!(PaidVia::Cash.money_account(), crate::ledger::CASH);
        assert_eq!(PaidVia::Upi.money_account(), crate::ledger::BANK);
        assert_eq!(PaidVia::Bank.money_account(), crate::ledger::BANK);
        assert_eq!(PaidVia::parse("bank"), Some(PaidVia::Bank));
        assert_eq!(PaidVia::parse("cheque"), None);
    }

    #[test]
    fn opening_balance_rules() {
        assert!(validate_opening_balance(3_840_000, 0).is_ok());
        assert!(validate_opening_balance(0, 100).is_ok());
        assert_eq!(validate_opening_balance(0, 0), Err(CoreError::validation("opening", "empty")));
        assert_eq!(validate_opening_balance(-1, 0), Err(CoreError::validation("opening", "negative")));
    }
}
