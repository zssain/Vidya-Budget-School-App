//! salary — salary-register rules (P15 Step 5, §10.3). Pure: no IO, no clock.
//!
//! Money is integer **paise** (`i64`). Working days come from the school calendar
//! (`crate::calendar::working_days`); this module turns them, days present and the
//! monthly salary into an unpaid-leave deduction, an advance recovery and a net.
//!
//! **[OWNER default]** deduction = monthly ÷ working days × unpaid days, rounded
//! **half-up to the rupee** (§10.3). The prototype's R. Nair case is the anchor:
//! ₹17,000 monthly, 26 working days, 2 unpaid → ₹1,308 deducted.

use crate::errors::{CoreError, CoreResult};

/// Unpaid-leave deduction (paise), rounded half-up to the whole rupee
/// (**[OWNER default]**). `monthly ÷ working_days × unpaid_days`. Returns 0 when
/// there are no unpaid days or no working days.
pub fn unpaid_leave_deduction(monthly_paise: i64, working_days: u32, unpaid_days: u32) -> i64 {
    if working_days == 0 || unpaid_days == 0 || monthly_paise <= 0 {
        return 0;
    }
    let num = monthly_paise as i128 * unpaid_days as i128; // exact deduction in paise = num / den
    let den = working_days as i128;
    // Round the paise value to the nearest rupee (100 paise), half-up:
    //   rupees = floor( (num/den)/100 + 1/2 ) = floor( (num + 50·den) / (100·den) )
    let rupees = (num + 50 * den) / (100 * den);
    (rupees * 100) as i64
}

/// The inputs to one staff member's salary line for a month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SalaryInputs {
    pub monthly_paise: i64,
    /// Working days in the month (from the calendar).
    pub working_days: u32,
    /// Days present (from staff attendance + approved leave once HR exists; until
    /// then entered by the Principal). Clamped to `0..=working_days`.
    pub days_present: u32,
    /// Advance recovery requested this month (usually `recover_per_month`).
    pub advance_recovery_paise: i64,
    /// Advance still outstanding for this staff member (amount − recovered).
    pub remaining_advance_paise: i64,
}

/// A computed salary line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SalaryLine {
    pub working_days: u32,
    pub days_present: u32,
    pub unpaid_leave_days: u32,
    pub deduction_paise: i64,
    pub advance_recovery_paise: i64,
    pub net_paise: i64,
}

/// Compute one staff member's salary line (§10.3):
/// * unpaid days = working_days − min(days_present, working_days);
/// * deduction = [`unpaid_leave_deduction`];
/// * advance recovery ≤ remaining advance AND ≤ (monthly − deduction) so net ≥ 0;
/// * net = monthly − deduction − advance recovery (≥ 0).
pub fn compute_salary_line(inp: &SalaryInputs) -> SalaryLine {
    let working_days = inp.working_days;
    let days_present = inp.days_present.min(working_days);
    let unpaid = working_days - days_present;
    let deduction = unpaid_leave_deduction(inp.monthly_paise, working_days, unpaid);
    let earned = (inp.monthly_paise - deduction).max(0);
    let recovery = inp
        .advance_recovery_paise
        .max(0)
        .min(inp.remaining_advance_paise.max(0))
        .min(earned);
    let net = (earned - recovery).max(0);
    SalaryLine {
        working_days,
        days_present,
        unpaid_leave_days: unpaid,
        deduction_paise: deduction,
        advance_recovery_paise: recovery,
        net_paise: net,
    }
}

/// Validate a monthly salary structure amount (> 0).
pub fn validate_monthly(monthly_paise: i64) -> CoreResult<()> {
    if monthly_paise <= 0 {
        return Err(CoreError::validation("monthly", "positive"));
    }
    Ok(())
}

/// Validate a new advance: amount > 0 and the per-month recovery is within
/// `0..=amount`.
pub fn validate_advance(amount_paise: i64, recover_per_month_paise: i64) -> CoreResult<()> {
    if amount_paise <= 0 {
        return Err(CoreError::validation("advance", "positive"));
    }
    if recover_per_month_paise < 0 || recover_per_month_paise > amount_paise {
        return Err(CoreError::validation("advance_recovery", "range"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r_nair_case_deducts_1308() {
        // Prototype anchor: ₹17,000, 26 working days, 2 unpaid → ₹1,308.
        assert_eq!(unpaid_leave_deduction(1_700_000, 26, 2), 130_800);
    }

    #[test]
    fn deduction_rounds_half_up_to_the_rupee() {
        // No unpaid / no working days → 0.
        assert_eq!(unpaid_leave_deduction(1_700_000, 26, 0), 0);
        assert_eq!(unpaid_leave_deduction(1_700_000, 0, 2), 0);
        // Exact division: ₹26,000 / 26 × 1 = ₹1,000.
        assert_eq!(unpaid_leave_deduction(2_600_000, 26, 1), 100_000);
        // Half rounds up: ₹15,000 / 30 × 1 = ₹500.00 exactly.
        assert_eq!(unpaid_leave_deduction(1_500_000, 30, 1), 50_000);
        // .50 rupee rounds up: 100 paise? ₹100 / 4 × 1 = ₹25.00.
        assert_eq!(unpaid_leave_deduction(10_000, 4, 1), 2_500);
        // Rounding: ₹1000 /3 ×1 = 333.33 → ₹333.
        assert_eq!(unpaid_leave_deduction(100_000, 3, 1), 33_300);
    }

    #[test]
    fn compute_line_full_attendance_no_deduction() {
        let line = compute_salary_line(&SalaryInputs {
            monthly_paise: 2_000_000,
            working_days: 26,
            days_present: 26,
            advance_recovery_paise: 0,
            remaining_advance_paise: 0,
        });
        assert_eq!(line.unpaid_leave_days, 0);
        assert_eq!(line.deduction_paise, 0);
        assert_eq!(line.net_paise, 2_000_000);
    }

    #[test]
    fn compute_line_r_nair_net() {
        // ₹17,000, 24 present of 26 → 2 unpaid → deduct ₹1,308 → net ₹15,692.
        let line = compute_salary_line(&SalaryInputs {
            monthly_paise: 1_700_000,
            working_days: 26,
            days_present: 24,
            advance_recovery_paise: 0,
            remaining_advance_paise: 0,
        });
        assert_eq!(line.unpaid_leave_days, 2);
        assert_eq!(line.deduction_paise, 130_800);
        assert_eq!(line.net_paise, 1_569_200);
    }

    #[test]
    fn advance_recovery_capped_and_net_never_negative() {
        // Meena: ₹18,000, full month, recover ₹2,000 of a ₹2,000 advance → net ₹16,000.
        let line = compute_salary_line(&SalaryInputs {
            monthly_paise: 1_800_000,
            working_days: 26,
            days_present: 26,
            advance_recovery_paise: 200_000,
            remaining_advance_paise: 200_000,
        });
        assert_eq!(line.advance_recovery_paise, 200_000);
        assert_eq!(line.net_paise, 1_600_000);

        // Recovery requested beyond the remaining advance is capped.
        let line = compute_salary_line(&SalaryInputs {
            monthly_paise: 1_800_000,
            working_days: 26,
            days_present: 26,
            advance_recovery_paise: 500_000,
            remaining_advance_paise: 100_000,
        });
        assert_eq!(line.advance_recovery_paise, 100_000);

        // Recovery cannot exceed earned (net ≥ 0).
        let line = compute_salary_line(&SalaryInputs {
            monthly_paise: 100_000,
            working_days: 26,
            days_present: 26,
            advance_recovery_paise: 500_000,
            remaining_advance_paise: 500_000,
        });
        assert_eq!(line.net_paise, 0);
        assert_eq!(line.advance_recovery_paise, 100_000);
    }

    #[test]
    fn validators() {
        assert!(validate_monthly(1).is_ok());
        assert!(validate_monthly(0).is_err());
        assert!(validate_advance(100_000, 20_000).is_ok());
        assert!(validate_advance(0, 0).is_err());
        assert!(validate_advance(100_000, 200_000).is_err());
    }
}
