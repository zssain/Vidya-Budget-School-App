use serde::{Deserialize, Serialize};

use crate::{
    error::DomainError,
    money::{format_inr, Rupees},
    validation::{validate_cheque_reference, validate_upi_reference},
};

/// Per-term fee components for a class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeePlan {
    pub tuition: i64,
    pub exam: i64,
    pub other: i64,
}

/// Inputs used to calculate one student's session fee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeInputs {
    pub plan: FeePlan,
    pub terms: u32,
    pub transport_fee_per_term: i64,
    pub transport: bool,
    pub rte: bool,
    pub concession: i64,
}

/// Fee status shown for an enrollment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeeState {
    Rte,
    Paid,
    Part,
    Due,
}

/// A receipt payment method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayMode {
    Cash,
    #[serde(rename = "UPI")]
    Upi,
    Cheque,
}

/// Values required to validate a proposed payment.
#[derive(Debug, Clone, Copy)]
pub struct PaymentCheck<'a> {
    pub amount: i64,
    pub balance: i64,
    pub mode: PayMode,
    pub reference: &'a str,
    pub upi_reference_already_used: bool,
    pub rte: bool,
    pub student_active: bool,
}

/// Adds a fee plan's three per-term components.
pub fn term_fee(plan: &FeePlan) -> i64 {
    plan.tuition.saturating_add(plan.exam).saturating_add(plan.other)
}

/// Calculates the session amount due, never returning less than zero.
pub fn session_due(inputs: &FeeInputs) -> i64 {
    if inputs.rte {
        return 0;
    }
    let per_term = one_term_amount(inputs).max(0);
    per_term
        .saturating_mul(i64::from(inputs.terms))
        .saturating_sub(inputs.concession.max(0))
        .max(0)
}

/// Calculates one term of fees including transport when selected.
pub fn one_term_amount(inputs: &FeeInputs) -> i64 {
    let transport = if inputs.transport {
        inputs.transport_fee_per_term
    } else {
        0
    };
    term_fee(&inputs.plan).saturating_add(transport)
}

/// Derives the enrollment fee state from due, paid and RTE status.
pub fn fee_state(due: i64, paid: i64, rte: bool) -> FeeState {
    if rte {
        FeeState::Rte
    } else if balance(due, paid) == 0 {
        FeeState::Paid
    } else if paid > 0 {
        FeeState::Part
    } else {
        FeeState::Due
    }
}

/// Calculates the unpaid balance, never returning less than zero.
pub fn balance(due: i64, paid: i64) -> i64 {
    due.saturating_sub(paid).max(0)
}

/// Validates payment rules in their required user-facing priority order.
pub fn validate_payment(payment: &PaymentCheck<'_>) -> Result<(), DomainError> {
    if !payment.student_active {
        return Err(DomainError::validation("fees.error.student_not_active"));
    }
    if payment.rte {
        return Err(DomainError::validation("fees.error.rte"));
    }
    if payment.amount <= 0 {
        return Err(DomainError::validation("fees.error.amount").field("amount"));
    }
    if payment.balance <= 0 {
        return Err(DomainError::validation("fees.error.no_balance"));
    }
    if payment.amount > payment.balance {
        return Err(DomainError::validation("fees.error.over_balance")
            .field("amount")
            .param("balance", format_inr(Rupees(payment.balance))));
    }
    match payment.mode {
        PayMode::Cash => {}
        PayMode::Upi => {
            validate_upi_reference(payment.reference)?;
            if payment.upi_reference_already_used {
                return Err(DomainError::validation("fees.error.upi_used").field("reference"));
            }
        }
        PayMode::Cheque => {
            validate_cheque_reference(payment.reference)?;
        }
    }
    Ok(())
}

/// Formats a device-prefixed receipt counter with at least four digits.
pub fn receipt_number(prefix: &str, counter: i64) -> String {
    format!("{prefix}-{counter:04}")
}
