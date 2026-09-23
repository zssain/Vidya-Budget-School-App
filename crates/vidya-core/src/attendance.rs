//! attendance — attendance-sheet business rules (prompts/P02 attendance.rs).
//!
//! Pure Rust: no IO, no async, no clock reads (`today` is passed in), no floats.
//! One sheet per class per date. A sheet is a draft until it is submitted; once
//! submitted it is **locked** (§3.6) and changes go through a correction request
//! approved by the Principal (§5, §8.5).
//!
//! Design decisions taken here (owner defaults, see module report):
//! * **Attendance % denominator = P + A + L** (Leave counts in the total), per
//!   §17.9 default "% = P ÷ (P + A + L)". Percentages are integer **tenths of a
//!   percent** (e.g. `914` = 91.4 %), rounded **half-up** with integer math only.
//! * A class with **0 active students cannot submit** — it returns
//!   [`CoreError::IncompleteSheet`] `{ remaining: 0 }` (there is nothing to
//!   submit yet).

use std::collections::BTreeMap;

use time::Date;

use crate::errors::{CoreError, CoreResult};
use crate::types::{Mark, RequestType, Role};

/// A student id. Ids are UUIDv7 text elsewhere in the system (§7); this module
/// only ever compares them, so a `String` alias keeps it decoupled.
pub type StudentId = String;

/// The decision for an edit attempted **after** a sheet has been submitted
/// (i.e. against locked data). Modelled as data so callers (Tauri command, then
/// the school server) can act on it without re-deriving the rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditDecision {
    /// The change is allowed outright, but the audit entry **must** carry a
    /// reason (`audit_reason_required = true`). Only the Principal reaches this.
    Allow { audit_reason_required: bool },
    /// The change is not allowed directly; it must go through a correction
    /// request of the given type (approved by the Principal).
    NeedsRequest(RequestType),
}

/// Result of merging two submissions of the same sheet (§8.5): per student,
/// equal marks merge; different marks become a conflict for that student.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeResult {
    /// Students whose mark is agreed (present in one side, or equal in both).
    pub merged: BTreeMap<StudentId, Mark>,
    /// Students marked differently on the two devices — a conflict each.
    pub conflicts: Vec<StudentId>,
}

/// Validate the sheet's date against `today`: a **future** date is rejected.
///
/// `today` is passed in (vidya-core never reads the clock). A date strictly
/// after `today` → `CoreError::validation("date", "future")`.
pub fn validate_sheet_date(date: Date, today: Date) -> CoreResult<()> {
    if date > today {
        return Err(CoreError::validation("date", "future"));
    }
    Ok(())
}

/// Whether the given actor may edit a **draft** sheet of a class.
///
/// Drafts are editable only by the class teacher of that class or by the
/// Principal (§5). Anyone else → `CoreError::Forbidden`. `is_class_teacher`
/// says whether this teacher is the class teacher *of the sheet's class*.
pub fn can_edit_draft(role: Role, is_class_teacher: bool) -> CoreResult<()> {
    match role {
        Role::Principal => Ok(()),
        Role::Teacher if is_class_teacher => Ok(()),
        Role::Teacher => Err(CoreError::Forbidden {
            reason: "not_class_teacher".into(),
        }),
        Role::Accountant => Err(CoreError::Forbidden {
            reason: "role".into(),
        }),
    }
}

/// Whether the sheet may be **submitted**: every active enrolled student on that
/// date must be marked.
///
/// `active_students` is the roster (active enrolments on the sheet's date);
/// `marks` is what has been entered so far. If any active student is unmarked,
/// returns `CoreError::IncompleteSheet { remaining }` where `remaining` is the
/// count of unmarked active students. A class with **0 active students** cannot
/// submit and returns `IncompleteSheet { remaining: 0 }`.
pub fn can_submit(
    active_students: &[StudentId],
    marks: &BTreeMap<StudentId, Mark>,
) -> CoreResult<()> {
    if active_students.is_empty() {
        // Nothing to submit yet — a distinct, testable case that still surfaces
        // as an incomplete sheet with remaining = 0.
        return Err(CoreError::IncompleteSheet { remaining: 0 });
    }
    let remaining = active_students
        .iter()
        .filter(|s| !marks.contains_key(*s))
        .count() as u32;
    if remaining > 0 {
        return Err(CoreError::IncompleteSheet { remaining });
    }
    Ok(())
}

/// The decision for a change made **after** the sheet was submitted (locked).
///
/// * Teacher → `NeedsRequest(AttendanceCorrection)` (approved by the Principal).
/// * Principal → `Allow { audit_reason_required: true }` (direct edit, always
///   audited with a reason, §5).
/// * Accountant → `NeedsRequest(AttendanceCorrection)`; accountants have no
///   attendance rights, so a direct edit is never allowed. (Permission to touch
///   attendance at all is enforced separately by `permissions`.)
pub fn edit_after_submit(role: Role) -> EditDecision {
    match role {
        Role::Principal => EditDecision::Allow {
            audit_reason_required: true,
        },
        Role::Teacher | Role::Accountant => {
            EditDecision::NeedsRequest(RequestType::AttendanceCorrection)
        }
    }
}

/// Attendance percentage in **tenths of a percent**, rounded **half-up**.
///
/// `= round_half_up(1000 * P / (P + A + L))`. The denominator is `P + A + L`
/// (Leave counts, owner default §17.9). If `P + A + L == 0` → `0`.
///
/// No floats. Half-up on non-negative integers is `(2*num + den) / (2*den)`:
/// here `num = 1000 * P`, `den = total`, so
/// `(2*1000*P + total) / (2*total) = (2000*P + total) / (2*total)`.
/// `u64` intermediates keep `2000 * P` from overflowing for any realistic
/// roster.
///
/// ```
/// use vidya_core::attendance::percent_present;
/// // 612 present of 670 marked → 91.34… % → 913 tenths (half-up).
/// assert_eq!(percent_present(612, 58, 0), 913);
/// assert_eq!(percent_present(0, 0, 0), 0);
/// ```
pub fn percent_present(p: u32, a: u32, l: u32) -> u32 {
    let total = p as u64 + a as u64 + l as u64;
    if total == 0 {
        return 0;
    }
    let num = 2000u64 * p as u64;
    ((num + total) / (2 * total)) as u32
}

/// Merge two submissions of the same sheet, per §8.5.
///
/// Per student: if only one side has a mark, it is taken; if both sides agree,
/// the mark is kept; if they disagree, the student is a **conflict** and is not
/// placed in `merged`. Conflicts are returned sorted (the union of keys is
/// walked in `BTreeMap` order).
pub fn merge_marks(
    a: &BTreeMap<StudentId, Mark>,
    b: &BTreeMap<StudentId, Mark>,
) -> MergeResult {
    let mut merged: BTreeMap<StudentId, Mark> = BTreeMap::new();
    let mut conflicts: Vec<StudentId> = Vec::new();

    // Walk the union of keys in sorted order so output is deterministic.
    let keys: std::collections::BTreeSet<&StudentId> = a.keys().chain(b.keys()).collect();
    for k in keys {
        match (a.get(k), b.get(k)) {
            (Some(ma), Some(mb)) => {
                if ma == mb {
                    merged.insert(k.clone(), *ma);
                } else {
                    conflicts.push(k.clone());
                }
            }
            (Some(m), None) | (None, Some(m)) => {
                merged.insert(k.clone(), *m);
            }
            (None, None) => unreachable!("key came from the union of both maps"),
        }
    }

    MergeResult { merged, conflicts }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    fn sid(s: &str) -> StudentId {
        s.to_string()
    }

    fn marks(pairs: &[(&str, Mark)]) -> BTreeMap<StudentId, Mark> {
        pairs.iter().map(|(s, m)| (sid(s), *m)).collect()
    }

    // ---- dates ------------------------------------------------------------

    #[test]
    fn today_and_past_dates_ok() {
        let today = date!(2026 - 09 - 23);
        assert!(validate_sheet_date(today, today).is_ok());
        assert!(validate_sheet_date(date!(2026 - 09 - 22), today).is_ok());
        assert!(validate_sheet_date(date!(2000 - 01 - 01), today).is_ok());
    }

    #[test]
    fn future_date_rejected() {
        let today = date!(2026 - 09 - 23);
        assert_eq!(
            validate_sheet_date(date!(2026 - 09 - 24), today),
            Err(CoreError::validation("date", "future"))
        );
    }

    // ---- draft edit permission -------------------------------------------

    #[test]
    fn draft_editable_by_class_teacher_and_principal() {
        assert!(can_edit_draft(Role::Principal, false).is_ok());
        assert!(can_edit_draft(Role::Teacher, true).is_ok());
    }

    #[test]
    fn draft_denied_to_others() {
        assert_eq!(
            can_edit_draft(Role::Teacher, false),
            Err(CoreError::Forbidden { reason: "not_class_teacher".into() })
        );
        assert_eq!(
            can_edit_draft(Role::Accountant, false),
            Err(CoreError::Forbidden { reason: "role".into() })
        );
    }

    // ---- submit completeness ---------------------------------------------

    #[test]
    fn submit_complete_sheet_ok() {
        let roster = vec![sid("s1"), sid("s2"), sid("s3")];
        let m = marks(&[("s1", Mark::P), ("s2", Mark::A), ("s3", Mark::L)]);
        assert!(can_submit(&roster, &m).is_ok());
    }

    #[test]
    fn submit_incomplete_reports_remaining() {
        let roster = vec![sid("s1"), sid("s2"), sid("s3"), sid("s4")];
        let m = marks(&[("s1", Mark::P), ("s3", Mark::A)]);
        // s2 and s4 unmarked → remaining = 2.
        assert_eq!(
            can_submit(&roster, &m),
            Err(CoreError::IncompleteSheet { remaining: 2 })
        );
    }

    #[test]
    fn submit_ignores_extra_marks_for_non_roster_students() {
        // A student who left mid-day may still have a stale mark; only the
        // active roster's completeness matters.
        let roster = vec![sid("s1")];
        let m = marks(&[("s1", Mark::P), ("ghost", Mark::A)]);
        assert!(can_submit(&roster, &m).is_ok());
    }

    #[test]
    fn zero_students_cannot_submit() {
        let roster: Vec<StudentId> = vec![];
        let m: BTreeMap<StudentId, Mark> = BTreeMap::new();
        assert_eq!(
            can_submit(&roster, &m),
            Err(CoreError::IncompleteSheet { remaining: 0 })
        );
    }

    // ---- edit after submit -----------------------------------------------

    #[test]
    fn teacher_after_submit_needs_request() {
        assert_eq!(
            edit_after_submit(Role::Teacher),
            EditDecision::NeedsRequest(RequestType::AttendanceCorrection)
        );
    }

    #[test]
    fn principal_after_submit_allows_with_reason() {
        assert_eq!(
            edit_after_submit(Role::Principal),
            EditDecision::Allow { audit_reason_required: true }
        );
    }

    #[test]
    fn accountant_after_submit_needs_request() {
        assert_eq!(
            edit_after_submit(Role::Accountant),
            EditDecision::NeedsRequest(RequestType::AttendanceCorrection)
        );
    }

    // ---- percent (half-up, tenths) ---------------------------------------

    #[test]
    fn percent_zero_when_nothing_marked() {
        assert_eq!(percent_present(0, 0, 0), 0);
    }

    #[test]
    fn percent_all_present_is_1000() {
        assert_eq!(percent_present(30, 0, 0), 1000);
    }

    #[test]
    fn percent_leave_counts_in_denominator() {
        // P=8, A=1, L=1 → 8/10 = 80.0 % = 800 tenths (Leave counts).
        assert_eq!(percent_present(8, 1, 1), 800);
    }

    #[test]
    fn percent_rounds_half_up() {
        // Exact .x5 case: P=1, total=8 → 1000/8 = 125.0 tenths → 125.
        assert_eq!(percent_present(1, 7, 0), 125);
        // Half-up at a tenth boundary: P=3, total=8 → 3000/8 = 375.0 → 375.
        assert_eq!(percent_present(3, 5, 0), 375);
        // A true half at the tenth: P=1, total=16 → 1000/16 = 62.5 tenths →
        // half-up to 63.
        assert_eq!(percent_present(1, 15, 0), 63);
        // Just below a half rounds down: P=1, total=40 → 25.0 tenths → 25.
        assert_eq!(percent_present(1, 39, 0), 25);
    }

    #[test]
    fn percent_typical_case() {
        // 612 present of 670 total → 91.34328… % → 913 tenths (rounds down).
        assert_eq!(percent_present(612, 58, 0), 913);
    }

    // ---- merge (§8.5) -----------------------------------------------------

    #[test]
    fn merge_equal_marks() {
        let a = marks(&[("s1", Mark::P), ("s2", Mark::A)]);
        let b = marks(&[("s1", Mark::P), ("s2", Mark::A)]);
        let r = merge_marks(&a, &b);
        assert_eq!(r.merged, marks(&[("s1", Mark::P), ("s2", Mark::A)]));
        assert!(r.conflicts.is_empty());
    }

    #[test]
    fn merge_disjoint_takes_union() {
        let a = marks(&[("s1", Mark::P)]);
        let b = marks(&[("s2", Mark::L)]);
        let r = merge_marks(&a, &b);
        assert_eq!(r.merged, marks(&[("s1", Mark::P), ("s2", Mark::L)]));
        assert!(r.conflicts.is_empty());
    }

    #[test]
    fn merge_conflicting_marks() {
        let a = marks(&[("s1", Mark::P), ("s2", Mark::A), ("s3", Mark::P)]);
        let b = marks(&[("s1", Mark::A), ("s2", Mark::A), ("s3", Mark::L)]);
        let r = merge_marks(&a, &b);
        // s2 agreed; s1 and s3 conflict and are excluded from merged.
        assert_eq!(r.merged, marks(&[("s2", Mark::A)]));
        assert_eq!(r.conflicts, vec![sid("s1"), sid("s3")]);
    }

    #[test]
    fn merge_conflicts_are_sorted() {
        let a = marks(&[("z", Mark::P), ("a", Mark::P)]);
        let b = marks(&[("z", Mark::A), ("a", Mark::A)]);
        let r = merge_marks(&a, &b);
        assert_eq!(r.conflicts, vec![sid("a"), sid("z")]);
    }
}
