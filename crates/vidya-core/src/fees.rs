//! fees — dues generation, payment allocation and payment validation (pure).
//!
//! Money is integer **paise** everywhere (`crate::money::Paise`). No floats, no
//! IO, no clock reads (times are passed in as epoch-ms `i64`).
//!
//! The rules follow `docs/00-SYSTEM-CONTEXT.md` §7 (fee_head / fee_due / payment
//! / payment_allocation) and §8.6 (money in sync — money received is never
//! rejected on the server; over-payment becomes `advance_credit` plus a flag,
//! and possible duplicates are flagged, never dropped).

use crate::errors::{CoreError, CoreResult};
use crate::money::Paise;
use crate::types::{FeeFrequency, PaymentMode};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Which students a fee head applies to (§7 `fee_head.applies_to`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppliesTo {
    /// Every active student.
    All,
    /// Only students who use transport (`student.transport == true`).
    Transport,
    /// Only students currently in one of these classes.
    ClassIds(Vec<String>),
}

/// A fee head definition (§7 `fee_head`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeHead {
    pub id: String,
    pub amount_paise: Paise,
    pub frequency: FeeFrequency,
    pub applies_to: AppliesTo,
    /// Optional instalment plan (§10.2, P15). When present, dues are generated per
    /// instalment (each carrying its `no` and `due_date`) instead of once per
    /// period. `None` keeps the pre-P15 per-period behaviour (one due, instalment
    /// 1, no due date).
    #[serde(default)]
    pub instalments: Option<InstalmentPlan>,
}

/// One instalment of a fee head's plan (§10.2). `no` is 1-based and contiguous;
/// `amount_paise` is this instalment's charge; `due_date` is `YYYY-MM-DD`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instalment {
    pub no: u32,
    pub amount_paise: Paise,
    pub due_date: String,
}

/// A fee head's instalment plan, stored as `fee_head.instalments_json` (§10.2).
///
/// * `List` — an explicit set of instalments (their amounts must sum to the head's
///   total; used by term / once / custom heads, e.g. Tuition 3× ₹4,000).
/// * `Monthly` — the same amount due on `day_of_month` of every session month
///   (used by `month`-frequency heads such as Transport; the prototype shows
///   "Every month, due on the 10th").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstalmentPlan {
    List { instalments: Vec<Instalment> },
    Monthly { monthly_amount_paise: Paise, day_of_month: u8 },
}

/// A student, reduced to the fields dues generation needs.
///
/// `admitted_period` is the period the once-heads are charged in (typically the
/// admission month or term), so a once head lands exactly once per student.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Student {
    pub id: String,
    pub class_id: String,
    pub transport: bool,
    pub admitted_period: String,
}

/// A `fee_due` row to insert (§7 `fee_due`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeDue {
    pub student_id: String,
    pub fee_head_id: String,
    pub period: String,
    pub amount_paise: Paise,
    /// 1-based instalment number (§10.2). Plan-less heads use `1`.
    pub instalment_no: u32,
    /// Due date `YYYY-MM-DD`, or `None` for plan-less heads (the caller supplies a
    /// default, e.g. term start, in the migration).
    pub due_date: Option<String>,
}

/// Which periods to generate for, in this run.
///
/// `terms` and `months` are the identifiers used as the due `period` for term
/// and monthly heads respectively (e.g. `"T1"` / `"2026-07"`). Once heads
/// ignore this and use each student's `admitted_period`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodSpec {
    pub terms: Vec<String>,
    pub months: Vec<String>,
}

/// Generate the `fee_due` rows to insert.
///
/// * term heads → once per term in `spec.terms`, for every matching student;
/// * month heads → once per month in `spec.months`, for every matching student;
/// * once heads → once per student, in that student's `admitted_period`.
///
/// A head "matches" a student per its `applies_to`: `All` → everyone;
/// `Transport` → only `student.transport`; `ClassIds(ids)` → only students whose
/// `class_id` is in `ids`.
///
/// `existing_allocated` is the set of `(student_id, fee_head_id, period)` that
/// already carry at least one allocation. Such dues are NEVER regenerated —
/// money has already been applied to them (§8.6, append-only payments).
pub fn generate_dues(
    heads: &[FeeHead],
    students: &[Student],
    spec: &PeriodSpec,
    existing_allocated: &[(String, String, String)],
) -> Vec<FeeDue> {
    let allocated: HashSet<(&str, &str, &str)> = existing_allocated
        .iter()
        .map(|(s, h, p)| (s.as_str(), h.as_str(), p.as_str()))
        .collect();

    let mut out = Vec::new();
    // A due that already has an allocation is NEVER regenerated (money applied).
    let push = |out: &mut Vec<FeeDue>, head: &FeeHead, student: &Student, period: &str, amount: Paise, no: u32, due: Option<String>| {
        let key = (student.id.as_str(), head.id.as_str(), period);
        if allocated.contains(&key) {
            return;
        }
        out.push(FeeDue {
            student_id: student.id.clone(),
            fee_head_id: head.id.clone(),
            period: period.to_string(),
            amount_paise: amount,
            instalment_no: no,
            due_date: due,
        });
    };

    for head in heads {
        for student in students {
            if !head_matches(head, student) {
                continue;
            }
            match &head.instalments {
                // Explicit instalment list: one due per instalment, keyed by its
                // due date (§10.2). Ignores `spec` — the plan carries its own dates.
                Some(InstalmentPlan::List { instalments }) => {
                    for inst in instalments {
                        push(&mut out, head, student, &inst.due_date, inst.amount_paise, inst.no, Some(inst.due_date.clone()));
                    }
                }
                // Monthly rule: one due per session month, due on `day_of_month`.
                Some(InstalmentPlan::Monthly { monthly_amount_paise, day_of_month }) => {
                    for (i, month) in spec.months.iter().enumerate() {
                        let due = format!("{month}-{day_of_month:02}");
                        push(&mut out, head, student, month, *monthly_amount_paise, (i + 1) as u32, Some(due));
                    }
                }
                // Plan-less heads keep the pre-P15 per-period behaviour.
                None => match head.frequency {
                    FeeFrequency::Term => {
                        for period in &spec.terms {
                            push(&mut out, head, student, period, head.amount_paise, 1, None);
                        }
                    }
                    FeeFrequency::Month => {
                        for period in &spec.months {
                            push(&mut out, head, student, period, head.amount_paise, 1, None);
                        }
                    }
                    FeeFrequency::Once => {
                        push(&mut out, head, student, &student.admitted_period, head.amount_paise, 1, None);
                    }
                },
            }
        }
    }
    out
}

/// Validate a fee head's instalment plan (§10.2, P15).
///
/// * `List` — non-empty; `no` values are exactly `1..=len` (each once); every
///   amount is positive; the amounts sum to `head_total_paise` (the head's total
///   amount); every `due_date` parses and falls within `[session_start,
///   session_end]` inclusive.
/// * `Monthly` — the monthly amount is positive and `day_of_month` is `1..=28`
///   (safe in every month, no skipped months).
///
/// Errors are `Validation{ field: "instalments", rule }` where `rule` is one of
/// `empty`, `no`, `amount`, `sum`, `due_date`, `day_of_month`.
pub fn validate_instalment_plan(
    plan: &InstalmentPlan,
    head_total_paise: Paise,
    session_start: &str,
    session_end: &str,
) -> CoreResult<()> {
    let inst_err = |rule: &str| CoreError::validation("instalments", rule);
    match plan {
        InstalmentPlan::List { instalments } => {
            if instalments.is_empty() {
                return Err(inst_err("empty"));
            }
            let start = crate::calendar::parse_date(session_start).ok_or_else(|| inst_err("due_date"))?;
            let end = crate::calendar::parse_date(session_end).ok_or_else(|| inst_err("due_date"))?;
            let mut nos: Vec<u32> = instalments.iter().map(|i| i.no).collect();
            nos.sort_unstable();
            let expected: Vec<u32> = (1..=instalments.len() as u32).collect();
            if nos != expected {
                return Err(inst_err("no"));
            }
            let mut sum = 0i64;
            for inst in instalments {
                if !inst.amount_paise.is_positive() {
                    return Err(inst_err("amount"));
                }
                sum += inst.amount_paise.get();
                let d = crate::calendar::parse_date(&inst.due_date).ok_or_else(|| inst_err("due_date"))?;
                if d < start || d > end {
                    return Err(inst_err("due_date"));
                }
            }
            if sum != head_total_paise.get() {
                return Err(inst_err("sum"));
            }
            Ok(())
        }
        InstalmentPlan::Monthly { monthly_amount_paise, day_of_month } => {
            if !monthly_amount_paise.is_positive() {
                return Err(inst_err("amount"));
            }
            if !(1..=28).contains(day_of_month) {
                return Err(inst_err("day_of_month"));
            }
            Ok(())
        }
    }
}

/// Whether `head.applies_to` covers `student`.
fn head_matches(head: &FeeHead, student: &Student) -> bool {
    match &head.applies_to {
        AppliesTo::All => true,
        AppliesTo::Transport => student.transport,
        AppliesTo::ClassIds(ids) => ids.iter().any(|c| c == &student.class_id),
    }
}

/// An existing `fee_due` as the plan-change preview needs it (§10.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExistingDue {
    pub id: String,
    pub instalment_no: u32,
    pub amount_paise: Paise,
    /// True if any payment has been allocated to this due (paid or partly paid).
    pub paid: bool,
}

/// One unpaid instalment due whose amount changes under a new plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanChangeLine {
    pub due_id: String,
    pub instalment_no: u32,
    pub old_amount: Paise,
    pub new_amount: Paise,
}

/// What a plan change would do to the UNPAID dues of a head (§10.2). Paid dues
/// (those with any allocation) are NEVER changed and never appear here — the
/// change-preview dialog in `feesadmin` shows only affected unpaid instalments.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanChangePreview {
    /// Unpaid dues whose amount differs under the new plan.
    pub changed: Vec<PlanChangeLine>,
    /// New instalment numbers that have no existing unpaid due (will be added).
    pub added: Vec<u32>,
    /// Unpaid due ids whose instalment number is not in the new plan (removed).
    pub removed: Vec<String>,
    /// Net change in total dues once applied (paise). Paid dues excluded.
    pub delta_paise: i64,
}

/// Preview a fee head's instalment-plan change against its existing dues (§10.2).
///
/// Compares `new_instalments` (the proposed plan) with the head's `existing` dues,
/// matched by `instalment_no`, and reports only the UNPAID dues affected. Paid
/// dues are left untouched (never changed, never removed). At most one due per
/// instalment number is assumed (as generation guarantees).
pub fn preview_plan_change(existing: &[ExistingDue], new_instalments: &[Instalment]) -> PlanChangePreview {
    use std::collections::{HashMap, HashSet};
    let paid_nos: HashSet<u32> = existing.iter().filter(|d| d.paid).map(|d| d.instalment_no).collect();
    let unpaid_by_no: HashMap<u32, &ExistingDue> =
        existing.iter().filter(|d| !d.paid).map(|d| (d.instalment_no, d)).collect();
    let new_nos: HashSet<u32> = new_instalments.iter().map(|i| i.no).collect();

    let mut preview = PlanChangePreview::default();
    for inst in new_instalments {
        if paid_nos.contains(&inst.no) {
            continue; // a paid instalment is never changed
        }
        match unpaid_by_no.get(&inst.no) {
            Some(due) if due.amount_paise != inst.amount_paise => {
                preview.delta_paise += inst.amount_paise.get() - due.amount_paise.get();
                preview.changed.push(PlanChangeLine {
                    due_id: due.id.clone(),
                    instalment_no: inst.no,
                    old_amount: due.amount_paise,
                    new_amount: inst.amount_paise,
                });
            }
            Some(_) => {} // unchanged amount
            None => {
                preview.delta_paise += inst.amount_paise.get();
                preview.added.push(inst.no);
            }
        }
    }
    for due in existing.iter().filter(|d| !d.paid) {
        if !new_nos.contains(&due.instalment_no) {
            preview.delta_paise -= due.amount_paise.get();
            preview.removed.push(due.id.clone());
        }
    }
    preview
}

/// A due to pay against, oldest first (caller supplies the ordering — P15 orders
/// by `due_date` so allocation still consumes the oldest instalment first).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Due {
    pub id: String,
    /// Outstanding balance on this due (amount − already-allocated), in paise.
    pub balance: Paise,
}

/// What an allocation covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllocKind {
    /// Applied to a specific `fee_due`.
    Due,
    /// Excess kept as credit against future dues.
    AdvanceCredit,
}

/// One line of an allocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alloc {
    /// The `fee_due` this covers, or `None` for `advance_credit`.
    pub due_id: Option<String>,
    pub amount: Paise,
    pub kind: AllocKind,
}

/// The result of allocating a payment across dues.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Allocation {
    pub allocations: Vec<Alloc>,
}

/// Allocate `payment` across `dues_oldest_first`, oldest due first.
///
/// Each due is paid (fully or partly) in order; any amount beyond every due's
/// balance becomes a single `advance_credit` line (`due_id = None`). Dues with a
/// zero balance produce no line. The amounts always sum to `payment`.
pub fn allocate(payment: Paise, dues_oldest_first: &[Due]) -> Allocation {
    let mut remaining = payment.get();
    let mut allocations = Vec::new();

    for due in dues_oldest_first {
        if remaining <= 0 {
            break;
        }
        let bal = due.balance.get();
        if bal <= 0 {
            continue;
        }
        let take = remaining.min(bal);
        allocations.push(Alloc {
            due_id: Some(due.id.clone()),
            amount: Paise(take),
            kind: AllocKind::Due,
        });
        remaining -= take;
    }

    if remaining > 0 {
        allocations.push(Alloc {
            due_id: None,
            amount: Paise(remaining),
            kind: AllocKind::AdvanceCredit,
        });
    }

    Allocation { allocations }
}

/// Validate a fee collection on the **collecting device** (§8.6, first bullet —
/// the ONLY place this limit applies).
///
/// * `total_due_now == 0` → `NO_DUES`;
/// * `amount > total_due_now` → `AMOUNT_EXCEEDS_DUE`;
/// * `amount <= 0` → `VALIDATION{amount}`;
/// * `0 < amount == total_due_now` is allowed (settles the balance to zero).
pub fn validate_collection(amount: Paise, total_due_now: Paise) -> CoreResult<()> {
    if total_due_now.is_zero() {
        return Err(CoreError::NoDues);
    }
    if amount > total_due_now {
        return Err(CoreError::AmountExceedsDue);
    }
    if !amount.is_positive() {
        return Err(CoreError::validation("amount", "positive"));
    }
    Ok(())
}

/// The result of applying a synced payment on the server (§8.6, second bullet).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncedResult {
    pub allocations: Vec<Alloc>,
    /// Set to the excess amount when the payment exceeded the total due; that
    /// excess is also present as an `advance_credit` allocation. Raises the
    /// caller's `review_flag excess_payment`.
    pub excess_payment: Option<Paise>,
}

/// Apply a payment on the **server**. ALWAYS accepts — money received is never
/// rejected (§8.6). Returns allocations (oldest due first) and, when the payment
/// exceeds the total due, the excess amount (also allocated as `advance_credit`).
pub fn apply_synced_payment(amount: Paise, dues_now: &[Due]) -> SyncedResult {
    let allocation = allocate(amount, dues_now);
    let excess_payment = allocation
        .allocations
        .iter()
        .find(|a| a.kind == AllocKind::AdvanceCredit)
        .map(|a| a.amount);
    SyncedResult { allocations: allocation.allocations, excess_payment }
}

/// Validate a payment reference for its mode (§7).
///
/// * `Upi` → 6–30 characters, all ASCII alphanumeric, REQUIRED;
/// * `Cheque` → non-empty, at most 60 characters, REQUIRED;
/// * `Cash` → the reference is ignored; always `Ok` (any value, empty or not).
pub fn validate_reference(mode: PaymentMode, reference: &str) -> CoreResult<()> {
    match mode {
        PaymentMode::Cash => Ok(()),
        PaymentMode::Upi => {
            let len = reference.chars().count();
            if !(6..=30).contains(&len) || !reference.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Err(CoreError::validation("reference", "upi"));
            }
            Ok(())
        }
        PaymentMode::Cheque => {
            let len = reference.chars().count();
            if len == 0 || len > 60 {
                return Err(CoreError::validation("reference", "cheque"));
            }
            Ok(())
        }
    }
}

/// A payment reduced to the fields duplicate detection needs (§8.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentLite {
    pub student_id: String,
    pub amount: Paise,
    pub mode: PaymentMode,
    pub reference: String,
    pub device_id: String,
    /// When the payment was collected, epoch milliseconds.
    pub collected_at_ms: i64,
}

/// The kind of duplicate flag raised (§7 `review_flag`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateKind {
    PossibleDuplicate,
}

/// Ten minutes in milliseconds.
const DUPLICATE_WINDOW_MS: i64 = 600_000;

/// Flag `candidate` as a possible duplicate of any existing payment (§8.6).
///
/// A flag (never a rejection) is raised when, for some existing payment with the
/// same student and the same amount, either:
/// * the UPI/cheque reference is the same and non-empty; OR
/// * it was collected within 10 minutes on a **different** device.
///
/// The caller keeps BOTH payments — this only surfaces a review flag.
pub fn duplicate_flag(candidate: &PaymentLite, existing: &[PaymentLite]) -> Option<DuplicateKind> {
    for other in existing {
        if other.student_id != candidate.student_id || other.amount != candidate.amount {
            continue;
        }

        // Same non-empty UPI/cheque reference → duplicate.
        let ref_dup = matches!(candidate.mode, PaymentMode::Upi | PaymentMode::Cheque)
            && candidate.mode == other.mode
            && !candidate.reference.is_empty()
            && candidate.reference == other.reference;

        // Same amount within 10 minutes on a different device → duplicate.
        let window_dup = candidate.device_id != other.device_id
            && (candidate.collected_at_ms - other.collected_at_ms).abs() <= DUPLICATE_WINDOW_MS;

        if ref_dup || window_dup {
            return Some(DuplicateKind::PossibleDuplicate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(id: &str, amount: i64, freq: FeeFrequency, applies_to: AppliesTo) -> FeeHead {
        FeeHead { id: id.into(), amount_paise: Paise(amount), frequency: freq, applies_to, instalments: None }
    }

    fn inst(no: u32, amount: i64, due: &str) -> Instalment {
        Instalment { no, amount_paise: Paise(amount), due_date: due.into() }
    }

    fn student(id: &str, class_id: &str, transport: bool, admitted: &str) -> Student {
        Student {
            id: id.into(),
            class_id: class_id.into(),
            transport,
            admitted_period: admitted.into(),
        }
    }

    fn due(id: &str, balance: i64) -> Due {
        Due { id: id.into(), balance: Paise(balance) }
    }

    // ---- validate_collection ----

    #[test]
    fn amount_equals_due_allowed_balance_zero() {
        // amount == due settles the balance to 0 and is allowed.
        assert!(validate_collection(Paise(50000), Paise(50000)).is_ok());
    }

    #[test]
    fn no_dues_rejected_on_collecting_device() {
        assert_eq!(validate_collection(Paise(50000), Paise::ZERO), Err(CoreError::NoDues));
    }

    #[test]
    fn over_due_on_collecting_device_rejected() {
        assert_eq!(
            validate_collection(Paise(60000), Paise(50000)),
            Err(CoreError::AmountExceedsDue)
        );
    }

    #[test]
    fn collection_rejects_zero_or_negative_amount() {
        assert_eq!(
            validate_collection(Paise::ZERO, Paise(50000)),
            Err(CoreError::validation("amount", "positive"))
        );
        assert_eq!(
            validate_collection(Paise(-1), Paise(50000)),
            Err(CoreError::validation("amount", "positive"))
        );
    }

    // ---- apply_synced_payment ----

    #[test]
    fn synced_payment_over_due_accepted_with_excess_flag_and_advance_credit() {
        let dues = [due("d1", 30000), due("d2", 20000)];
        let result = apply_synced_payment(Paise(70000), &dues);
        // Full dues covered, then the excess as advance_credit.
        assert_eq!(result.excess_payment, Some(Paise(20000)));
        assert_eq!(
            result.allocations,
            vec![
                Alloc { due_id: Some("d1".into()), amount: Paise(30000), kind: AllocKind::Due },
                Alloc { due_id: Some("d2".into()), amount: Paise(20000), kind: AllocKind::Due },
                Alloc { due_id: None, amount: Paise(20000), kind: AllocKind::AdvanceCredit },
            ]
        );
    }

    #[test]
    fn synced_payment_within_due_has_no_excess_flag() {
        let dues = [due("d1", 30000), due("d2", 20000)];
        let result = apply_synced_payment(Paise(40000), &dues);
        assert_eq!(result.excess_payment, None);
        assert_eq!(
            result.allocations,
            vec![
                Alloc { due_id: Some("d1".into()), amount: Paise(30000), kind: AllocKind::Due },
                Alloc { due_id: Some("d2".into()), amount: Paise(10000), kind: AllocKind::Due },
            ]
        );
    }

    #[test]
    fn synced_payment_with_no_dues_is_all_advance_credit() {
        let result = apply_synced_payment(Paise(50000), &[]);
        assert_eq!(result.excess_payment, Some(Paise(50000)));
        assert_eq!(
            result.allocations,
            vec![Alloc { due_id: None, amount: Paise(50000), kind: AllocKind::AdvanceCredit }]
        );
    }

    // ---- allocate ----

    #[test]
    fn allocation_oldest_first_with_partial_and_excess() {
        let dues = [due("d1", 30000), due("d2", 20000), due("d3", 10000)];
        // 45000: fully pays d1 (30000), partly pays d2 (15000), nothing to d3.
        let result = allocate(Paise(45000), &dues);
        assert_eq!(
            result.allocations,
            vec![
                Alloc { due_id: Some("d1".into()), amount: Paise(30000), kind: AllocKind::Due },
                Alloc { due_id: Some("d2".into()), amount: Paise(15000), kind: AllocKind::Due },
            ]
        );

        // 75000: pays all three (60000) and 15000 becomes advance_credit.
        let result = allocate(Paise(75000), &dues);
        assert_eq!(
            result.allocations,
            vec![
                Alloc { due_id: Some("d1".into()), amount: Paise(30000), kind: AllocKind::Due },
                Alloc { due_id: Some("d2".into()), amount: Paise(20000), kind: AllocKind::Due },
                Alloc { due_id: Some("d3".into()), amount: Paise(10000), kind: AllocKind::Due },
                Alloc { due_id: None, amount: Paise(15000), kind: AllocKind::AdvanceCredit },
            ]
        );
    }

    #[test]
    fn allocate_skips_zero_balance_dues() {
        let dues = [due("d0", 0), due("d1", 10000)];
        let result = allocate(Paise(10000), &dues);
        assert_eq!(
            result.allocations,
            vec![Alloc { due_id: Some("d1".into()), amount: Paise(10000), kind: AllocKind::Due }]
        );
    }

    // ---- validate_reference ----

    #[test]
    fn upi_reference_rules() {
        assert!(validate_reference(PaymentMode::Upi, "abc123").is_ok()); // 6 chars
        assert!(validate_reference(PaymentMode::Upi, "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5").is_ok()); // 30
        assert!(validate_reference(PaymentMode::Upi, "abc12").is_err()); // too short
        assert!(validate_reference(PaymentMode::Upi, &"a".repeat(31)).is_err()); // too long
        assert!(validate_reference(PaymentMode::Upi, "abc-12").is_err()); // non-alphanumeric
        assert!(validate_reference(PaymentMode::Upi, "").is_err()); // empty
    }

    #[test]
    fn cheque_reference_rules() {
        assert!(validate_reference(PaymentMode::Cheque, "123456").is_ok());
        assert!(validate_reference(PaymentMode::Cheque, "cheque no. 42 / drawn on SBI").is_ok());
        assert!(validate_reference(PaymentMode::Cheque, &"x".repeat(60)).is_ok());
        assert!(validate_reference(PaymentMode::Cheque, "").is_err()); // empty
        assert!(validate_reference(PaymentMode::Cheque, &"x".repeat(61)).is_err()); // too long
    }

    #[test]
    fn cash_reference_ignored() {
        assert!(validate_reference(PaymentMode::Cash, "").is_ok());
        assert!(validate_reference(PaymentMode::Cash, "anything at all").is_ok());
    }

    // ---- duplicate_flag ----

    #[test]
    fn duplicate_upi_reference_flagged_both_kept() {
        let existing = [PaymentLite {
            student_id: "s1".into(),
            amount: Paise(50000),
            mode: PaymentMode::Upi,
            reference: "TXN12345".into(),
            device_id: "A2".into(),
            collected_at_ms: 1_000,
        }];
        let candidate = PaymentLite {
            student_id: "s1".into(),
            amount: Paise(50000),
            mode: PaymentMode::Upi,
            reference: "TXN12345".into(),
            device_id: "A3".into(), // even a different device
            collected_at_ms: 9_999_999, // and far apart in time
        };
        // Reference-based duplicate → flag; the flag never rejects, caller keeps both.
        assert_eq!(duplicate_flag(&candidate, &existing), Some(DuplicateKind::PossibleDuplicate));
    }

    #[test]
    fn duplicate_within_10_min_on_different_device_flagged() {
        let existing = [PaymentLite {
            student_id: "s1".into(),
            amount: Paise(50000),
            mode: PaymentMode::Cash,
            reference: String::new(),
            device_id: "A2".into(),
            collected_at_ms: 1_000_000,
        }];
        let candidate = PaymentLite {
            student_id: "s1".into(),
            amount: Paise(50000),
            mode: PaymentMode::Cash,
            reference: String::new(),
            device_id: "A3".into(),
            collected_at_ms: 1_000_000 + 600_000, // exactly 10 min later
        };
        assert_eq!(duplicate_flag(&candidate, &existing), Some(DuplicateKind::PossibleDuplicate));
    }

    #[test]
    fn not_a_duplicate_same_device_or_outside_window() {
        let base = PaymentLite {
            student_id: "s1".into(),
            amount: Paise(50000),
            mode: PaymentMode::Cash,
            reference: String::new(),
            device_id: "A2".into(),
            collected_at_ms: 1_000_000,
        };
        let existing = [base.clone()];
        // Same device → not a window duplicate.
        let same_device = PaymentLite {
            device_id: "A2".into(),
            collected_at_ms: 1_000_000 + 60_000,
            ..base.clone()
        };
        assert_eq!(duplicate_flag(&same_device, &existing), None);
        // Different device but just past 10 min → not a duplicate.
        let outside = PaymentLite {
            device_id: "A3".into(),
            collected_at_ms: 1_000_000 + 600_001,
            ..base.clone()
        };
        assert_eq!(duplicate_flag(&outside, &existing), None);
        // Different amount → not a duplicate.
        let diff_amount =
            PaymentLite { device_id: "A3".into(), amount: Paise(40000), ..base };
        assert_eq!(duplicate_flag(&diff_amount, &existing), None);
    }

    #[test]
    fn empty_reference_not_treated_as_duplicate_reference() {
        // Two cash payments with empty references, different device but far apart:
        // no reference match and outside the window → no flag.
        let existing = [PaymentLite {
            student_id: "s1".into(),
            amount: Paise(50000),
            mode: PaymentMode::Cash,
            reference: String::new(),
            device_id: "A2".into(),
            collected_at_ms: 1_000,
        }];
        let candidate = PaymentLite {
            student_id: "s1".into(),
            amount: Paise(50000),
            mode: PaymentMode::Cash,
            reference: String::new(),
            device_id: "A3".into(),
            collected_at_ms: 10_000_000,
        };
        assert_eq!(duplicate_flag(&candidate, &existing), None);
    }

    // ---- generate_dues ----

    #[test]
    fn transport_head_only_for_transport_students() {
        let heads = [head("transport", 80000, FeeFrequency::Month, AppliesTo::Transport)];
        let students = [
            student("s1", "c1", true, "2026-07"),
            student("s2", "c1", false, "2026-07"),
        ];
        let spec = PeriodSpec { terms: vec![], months: vec!["2026-07".into()] };
        let dues = generate_dues(&heads, &students, &spec, &[]);
        assert_eq!(dues.len(), 1);
        assert_eq!(dues[0].student_id, "s1");
        assert_eq!(dues[0].amount_paise, Paise(80000));
        assert_eq!(dues[0].period, "2026-07");
    }

    #[test]
    fn class_ids_head_only_for_those_classes() {
        let heads = [head(
            "lab",
            50000,
            FeeFrequency::Term,
            AppliesTo::ClassIds(vec!["c1".into(), "c2".into()]),
        )];
        let students = [
            student("s1", "c1", false, "T1"),
            student("s2", "c3", false, "T1"),
            student("s3", "c2", false, "T1"),
        ];
        let spec = PeriodSpec { terms: vec!["T1".into()], months: vec![] };
        let dues = generate_dues(&heads, &students, &spec, &[]);
        let ids: Vec<&str> = dues.iter().map(|d| d.student_id.as_str()).collect();
        assert_eq!(ids, vec!["s1", "s3"]);
    }

    #[test]
    fn term_and_month_and_once_heads_generate_correctly() {
        let heads = [
            head("tuition_term", 300000, FeeFrequency::Term, AppliesTo::All),
            head("monthly", 100000, FeeFrequency::Month, AppliesTo::All),
            head("admission", 500000, FeeFrequency::Once, AppliesTo::All),
        ];
        let students = [student("s1", "c1", false, "2026-07")];
        let spec = PeriodSpec {
            terms: vec!["T1".into(), "T2".into()],
            months: vec!["2026-07".into(), "2026-08".into()],
        };
        let dues = generate_dues(&heads, &students, &spec, &[]);
        // 2 terms + 2 months + 1 once = 5.
        assert_eq!(dues.len(), 5);
        // The once head lands in the admitted period.
        let once: Vec<&FeeDue> =
            dues.iter().filter(|d| d.fee_head_id == "admission").collect();
        assert_eq!(once.len(), 1);
        assert_eq!(once[0].period, "2026-07");
    }

    #[test]
    fn never_regenerate_due_with_allocation() {
        let heads = [head("monthly", 100000, FeeFrequency::Month, AppliesTo::All)];
        let students = [student("s1", "c1", false, "2026-07")];
        let spec = PeriodSpec {
            terms: vec![],
            months: vec!["2026-07".into(), "2026-08".into()],
        };
        // July already has an allocation → skip it; August is regenerated.
        let existing = [("s1".to_string(), "monthly".to_string(), "2026-07".to_string())];
        let dues = generate_dues(&heads, &students, &spec, &existing);
        assert_eq!(dues.len(), 1);
        assert_eq!(dues[0].period, "2026-08");
    }

    // ---- instalments (P15) ----

    #[test]
    fn instalment_list_generates_one_due_per_instalment() {
        let mut h = head("tuition", 1_200_000, FeeFrequency::Term, AppliesTo::All);
        h.instalments = Some(InstalmentPlan::List {
            instalments: vec![
                inst(1, 400_000, "2026-04-15"),
                inst(2, 400_000, "2026-08-15"),
                inst(3, 400_000, "2026-12-15"),
            ],
        });
        let students = [student("s1", "c1", false, "2026-04")];
        let spec = PeriodSpec { terms: vec!["T1".into()], months: vec![] };
        let dues = generate_dues(&[h], &students, &spec, &[]);
        assert_eq!(dues.len(), 3);
        assert_eq!(dues[1].instalment_no, 2);
        assert_eq!(dues[1].amount_paise, Paise(400_000));
        assert_eq!(dues[1].due_date.as_deref(), Some("2026-08-15"));
        // The due date is the dedup period, so a paid instalment is not regenerated.
        let existing = [("s1".to_string(), "tuition".to_string(), "2026-04-15".to_string())];
        let dues2 = generate_dues(
            &[FeeHead {
                id: "tuition".into(),
                amount_paise: Paise(1_200_000),
                frequency: FeeFrequency::Term,
                applies_to: AppliesTo::All,
                instalments: Some(InstalmentPlan::List {
                    instalments: vec![inst(1, 400_000, "2026-04-15"), inst(2, 400_000, "2026-08-15")],
                }),
            }],
            &students,
            &spec,
            &existing,
        );
        assert_eq!(dues2.len(), 1);
        assert_eq!(dues2[0].instalment_no, 2);
    }

    #[test]
    fn monthly_rule_generates_monthly_dues_with_due_dates() {
        let mut h = head("transport", 20000, FeeFrequency::Month, AppliesTo::Transport);
        h.instalments = Some(InstalmentPlan::Monthly { monthly_amount_paise: Paise(20000), day_of_month: 10 });
        let students = [student("s1", "c1", true, "2026-07"), student("s2", "c1", false, "2026-07")];
        let spec = PeriodSpec { terms: vec![], months: vec!["2026-07".into(), "2026-08".into()] };
        let dues = generate_dues(&[h], &students, &spec, &[]);
        // Only the transport student, one due per month.
        assert_eq!(dues.len(), 2);
        assert_eq!(dues[0].due_date.as_deref(), Some("2026-07-10"));
        assert_eq!(dues[0].instalment_no, 1);
        assert_eq!(dues[1].due_date.as_deref(), Some("2026-08-10"));
        assert_eq!(dues[1].instalment_no, 2);
    }

    #[test]
    fn validate_instalment_plan_sum_must_match_head_total() {
        let ok = InstalmentPlan::List {
            instalments: vec![inst(1, 400_000, "2026-04-15"), inst(2, 400_000, "2026-08-15"), inst(3, 400_000, "2026-12-15")],
        };
        assert!(validate_instalment_plan(&ok, Paise(1_200_000), "2026-04-01", "2027-03-31").is_ok());
        // Sum 1,150,000 ≠ 1,200,000 → VALIDATION{instalments, sum}.
        let bad = InstalmentPlan::List {
            instalments: vec![inst(1, 400_000, "2026-04-15"), inst(2, 350_000, "2026-08-15"), inst(3, 400_000, "2026-12-15")],
        };
        assert_eq!(
            validate_instalment_plan(&bad, Paise(1_200_000), "2026-04-01", "2027-03-31"),
            Err(CoreError::validation("instalments", "sum"))
        );
    }

    #[test]
    fn validate_instalment_plan_dates_within_session_and_contiguous_nos() {
        // A due date outside the session is rejected.
        let out_of_range = InstalmentPlan::List { instalments: vec![inst(1, 100_000, "2025-01-01")] };
        assert_eq!(
            validate_instalment_plan(&out_of_range, Paise(100_000), "2026-04-01", "2027-03-31"),
            Err(CoreError::validation("instalments", "due_date"))
        );
        // Non-contiguous instalment numbers are rejected.
        let bad_nos = InstalmentPlan::List {
            instalments: vec![inst(1, 50_000, "2026-05-01"), inst(3, 50_000, "2026-09-01")],
        };
        assert_eq!(
            validate_instalment_plan(&bad_nos, Paise(100_000), "2026-04-01", "2027-03-31"),
            Err(CoreError::validation("instalments", "no"))
        );
        // Empty list rejected.
        assert_eq!(
            validate_instalment_plan(&InstalmentPlan::List { instalments: vec![] }, Paise(0), "2026-04-01", "2027-03-31"),
            Err(CoreError::validation("instalments", "empty"))
        );
    }

    #[test]
    fn validate_monthly_plan_rules() {
        let ok = InstalmentPlan::Monthly { monthly_amount_paise: Paise(20000), day_of_month: 10 };
        assert!(validate_instalment_plan(&ok, Paise(20000), "2026-04-01", "2027-03-31").is_ok());
        // day_of_month must be 1..=28.
        let bad_day = InstalmentPlan::Monthly { monthly_amount_paise: Paise(20000), day_of_month: 31 };
        assert_eq!(
            validate_instalment_plan(&bad_day, Paise(20000), "2026-04-01", "2027-03-31"),
            Err(CoreError::validation("instalments", "day_of_month"))
        );
        // Non-positive amount rejected.
        let bad_amt = InstalmentPlan::Monthly { monthly_amount_paise: Paise(0), day_of_month: 10 };
        assert_eq!(
            validate_instalment_plan(&bad_amt, Paise(0), "2026-04-01", "2027-03-31"),
            Err(CoreError::validation("instalments", "amount"))
        );
    }

    #[test]
    fn preview_plan_change_lists_only_unpaid_dues() {
        let existing = [
            ExistingDue { id: "d1".into(), instalment_no: 1, amount_paise: Paise(400_000), paid: true },
            ExistingDue { id: "d2".into(), instalment_no: 2, amount_paise: Paise(400_000), paid: false },
            ExistingDue { id: "d3".into(), instalment_no: 3, amount_paise: Paise(400_000), paid: false },
        ];
        // New plan: inst1 changes (but paid → untouched), inst2 changes to 500k,
        // inst3 dropped, inst4 added.
        let new_plan = [inst(1, 500_000, "2026-04-15"), inst(2, 500_000, "2026-08-15"), inst(4, 300_000, "2027-01-15")];
        let p = preview_plan_change(&existing, &new_plan);
        // d1 is paid → never listed.
        assert!(p.changed.iter().all(|l| l.due_id != "d1"));
        // d2 changed 400k → 500k.
        assert_eq!(p.changed.len(), 1);
        assert_eq!(p.changed[0].due_id, "d2");
        assert_eq!(p.changed[0].new_amount, Paise(500_000));
        // inst4 added; d3 removed.
        assert_eq!(p.added, vec![4]);
        assert_eq!(p.removed, vec!["d3".to_string()]);
        // delta = (+100k for d2) + (+300k added) - (400k removed d3) = 0.
        assert_eq!(p.delta_paise, 0);
    }
}
