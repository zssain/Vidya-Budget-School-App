//! marks — marks-entry business rules (prompts/P02 marks.rs).
//!
//! Pure Rust: no IO, no async, no floats. A mark entry is either a number of
//! marks obtained (`0 ..= max_marks`) **or** `absent`; the two are mutually
//! exclusive. A missing entry is **NULL** (`marks: None, absent: false`) — "not
//! entered" and is **never** treated as zero (§7).
//!
//! The lock is **per exam_subject** (§3.6, §7): a submitted sheet is locked and
//! edits go through a correction request approved by the Principal (§5). A
//! teacher may only touch their own class-subject.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::errors::{CoreError, CoreResult};
use crate::types::{RequestType, Role};

/// A student id (UUIDv7 text elsewhere; a `String` alias keeps this decoupled).
pub type StudentId = String;

/// One student's mark for one exam_subject (`mark_entry`, §7).
///
/// Invariant (enforced by [`validate_entry`]): at most one of `marks`/`absent`
/// is set. `marks = None, absent = false` is **NULL** — not entered, never zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkEntry {
    /// Marks obtained, or `None` for NULL (not entered).
    pub marks: Option<u32>,
    /// Whether the student was absent for this exam_subject.
    pub absent: bool,
}

impl MarkEntry {
    /// The NULL entry: not entered, not absent. Never treated as zero.
    pub const NULL: MarkEntry = MarkEntry { marks: None, absent: false };

    /// An entry with `marks` obtained.
    pub fn obtained(marks: u32) -> MarkEntry {
        MarkEntry { marks: Some(marks), absent: false }
    }

    /// An entry marked absent.
    pub fn absent() -> MarkEntry {
        MarkEntry { marks: None, absent: true }
    }

    /// True for a NULL (not-entered) entry.
    pub fn is_null(&self) -> bool {
        self.marks.is_none() && !self.absent
    }
}

impl Default for MarkEntry {
    fn default() -> Self {
        MarkEntry::NULL
    }
}

/// A single change to one student's entry in a PATCH (see [`apply_patch`]).
///
/// A patch names only the students who change; everyone else is untouched.
/// `Clear` is an **explicit** action that sets the entry back to NULL — it is
/// distinct from simply omitting the student.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "op", content = "value")]
pub enum MarkPatch {
    /// Set the marks obtained.
    Set(u32),
    /// Mark the student absent.
    SetAbsent,
    /// Explicitly clear the entry back to NULL (not entered).
    Clear,
}

/// The decision for an edit attempted against a **submitted (locked)** sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditDecision {
    /// Allowed outright, but the audit entry must carry a reason. Principal only.
    Allow { audit_reason_required: bool },
    /// Must go through a correction request of the given type.
    NeedsRequest(RequestType),
}

/// Validate one entry against `max_marks`.
///
/// * `absent` and `marks` are mutually exclusive → `validation("marks",
///   "absent_and_marks")` if both are set.
/// * `marks` must satisfy `0 <= marks <= max_marks` → `validation("marks",
///   "range")` otherwise.
/// * NULL and plain absent are always valid.
pub fn validate_entry(entry: &MarkEntry, max_marks: u32) -> CoreResult<()> {
    if entry.absent && entry.marks.is_some() {
        return Err(CoreError::validation("marks", "absent_and_marks"));
    }
    if let Some(m) = entry.marks {
        if m > max_marks {
            return Err(CoreError::validation("marks", "range"));
        }
    }
    Ok(())
}

/// Whether a teacher may edit this class-subject's marks at all.
///
/// A teacher may only touch **their own** class-subject (`is_own_class_subject`);
/// otherwise `CoreError::Forbidden`. The Principal may always edit (subject to
/// the lock rules in [`edit_after_submit`]); accountants have no marks rights.
pub fn can_edit(role: Role, is_own_class_subject: bool) -> CoreResult<()> {
    match role {
        Role::Principal => Ok(()),
        Role::Teacher if is_own_class_subject => Ok(()),
        Role::Teacher => Err(CoreError::Forbidden { reason: "not_own_class_subject".into() }),
        Role::Accountant => Err(CoreError::Forbidden { reason: "role".into() }),
    }
}

/// The decision for a change against a **submitted (locked)** marks sheet.
///
/// * Teacher → `NeedsRequest(MarksCorrection)`.
/// * Principal → `Allow { audit_reason_required: true }` (direct edit, audited).
/// * Accountant → `NeedsRequest(MarksCorrection)` (no direct edit ever; the
///   separate permission check keeps them out of marks entirely).
pub fn edit_after_submit(role: Role) -> EditDecision {
    match role {
        Role::Principal => EditDecision::Allow { audit_reason_required: true },
        Role::Teacher | Role::Accountant => {
            EditDecision::NeedsRequest(RequestType::MarksCorrection)
        }
    }
}

/// Apply a PATCH to a sheet: **only** the students listed in `patch` change;
/// every other student's entry is left byte-identical.
///
/// `Set(m)` → marks `m` (not absent); `SetAbsent` → absent; `Clear` → NULL.
/// This does **not** validate ranges — call [`validate_entry`] on each result
/// (or on the patch inputs) before persisting; keeping the two separate lets a
/// caller preview the patched sheet and report every invalid cell at once.
pub fn apply_patch(
    existing: &BTreeMap<StudentId, MarkEntry>,
    patch: &BTreeMap<StudentId, MarkPatch>,
) -> BTreeMap<StudentId, MarkEntry> {
    let mut out = existing.clone();
    for (student, op) in patch {
        let entry = match op {
            MarkPatch::Set(m) => MarkEntry::obtained(*m),
            MarkPatch::SetAbsent => MarkEntry::absent(),
            MarkPatch::Clear => MarkEntry::NULL,
        };
        out.insert(student.clone(), entry);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(s: &str) -> StudentId {
        s.to_string()
    }

    // ---- entry validation -------------------------------------------------

    #[test]
    fn valid_marks_in_range() {
        assert!(validate_entry(&MarkEntry::obtained(0), 100).is_ok());
        assert!(validate_entry(&MarkEntry::obtained(100), 100).is_ok());
        assert!(validate_entry(&MarkEntry::obtained(57), 100).is_ok());
    }

    #[test]
    fn marks_over_max_is_range_error() {
        assert_eq!(
            validate_entry(&MarkEntry::obtained(101), 100),
            Err(CoreError::validation("marks", "range"))
        );
    }

    #[test]
    fn absent_is_valid() {
        assert!(validate_entry(&MarkEntry::absent(), 100).is_ok());
    }

    #[test]
    fn null_is_valid() {
        assert!(validate_entry(&MarkEntry::NULL, 100).is_ok());
        assert!(MarkEntry::NULL.is_null());
    }

    #[test]
    fn absent_and_marks_mutually_exclusive() {
        let bad = MarkEntry { marks: Some(40), absent: true };
        assert_eq!(
            validate_entry(&bad, 100),
            Err(CoreError::validation("marks", "absent_and_marks"))
        );
    }

    // ---- edit permission --------------------------------------------------

    #[test]
    fn teacher_own_subject_can_edit() {
        assert!(can_edit(Role::Teacher, true).is_ok());
    }

    #[test]
    fn teacher_other_subject_forbidden() {
        assert_eq!(
            can_edit(Role::Teacher, false),
            Err(CoreError::Forbidden { reason: "not_own_class_subject".into() })
        );
    }

    #[test]
    fn principal_can_edit_any_subject() {
        assert!(can_edit(Role::Principal, false).is_ok());
    }

    #[test]
    fn accountant_cannot_edit_marks() {
        assert_eq!(
            can_edit(Role::Accountant, true),
            Err(CoreError::Forbidden { reason: "role".into() })
        );
    }

    // ---- edit after submit (lock) ----------------------------------------

    #[test]
    fn teacher_after_submit_needs_request() {
        assert_eq!(
            edit_after_submit(Role::Teacher),
            EditDecision::NeedsRequest(RequestType::MarksCorrection)
        );
    }

    #[test]
    fn principal_after_submit_allows_with_reason() {
        assert_eq!(
            edit_after_submit(Role::Principal),
            EditDecision::Allow { audit_reason_required: true }
        );
    }

    // ---- patch semantics --------------------------------------------------

    /// Build a full 34-student sheet where every student has some entry.
    fn full_sheet_34() -> BTreeMap<StudentId, MarkEntry> {
        (0..34)
            .map(|i| (format!("s{i:02}"), MarkEntry::obtained(i)))
            .collect()
    }

    #[test]
    fn patch_touches_only_listed_students() {
        let existing = full_sheet_34();
        let mut patch: BTreeMap<StudentId, MarkPatch> = BTreeMap::new();
        patch.insert(sid("s05"), MarkPatch::Set(88));
        patch.insert(sid("s10"), MarkPatch::SetAbsent);
        patch.insert(sid("s20"), MarkPatch::Clear);

        let result = apply_patch(&existing, &patch);

        // The 3 touched students changed as instructed.
        assert_eq!(result[&sid("s05")], MarkEntry::obtained(88));
        assert_eq!(result[&sid("s10")], MarkEntry::absent());
        assert_eq!(result[&sid("s20")], MarkEntry::NULL);

        // The other 31 are byte-identical to the original.
        for i in 0..34 {
            let key = format!("s{i:02}");
            if key == "s05" || key == "s10" || key == "s20" {
                continue;
            }
            assert_eq!(result[&key], existing[&key], "{key} must be untouched");
        }
        assert_eq!(result.len(), existing.len());
    }

    #[test]
    fn patch_set_overwrites_absent() {
        let mut existing: BTreeMap<StudentId, MarkEntry> = BTreeMap::new();
        existing.insert(sid("s1"), MarkEntry::absent());
        let mut patch: BTreeMap<StudentId, MarkPatch> = BTreeMap::new();
        patch.insert(sid("s1"), MarkPatch::Set(45));
        let result = apply_patch(&existing, &patch);
        assert_eq!(result[&sid("s1")], MarkEntry::obtained(45));
    }

    #[test]
    fn patch_clear_sets_back_to_null_not_zero() {
        let mut existing: BTreeMap<StudentId, MarkEntry> = BTreeMap::new();
        existing.insert(sid("s1"), MarkEntry::obtained(50));
        let mut patch: BTreeMap<StudentId, MarkPatch> = BTreeMap::new();
        patch.insert(sid("s1"), MarkPatch::Clear);
        let result = apply_patch(&existing, &patch);
        // Clear yields NULL — explicitly NOT marks = 0.
        assert!(result[&sid("s1")].is_null());
        assert_eq!(result[&sid("s1")], MarkEntry::NULL);
        assert_ne!(result[&sid("s1")], MarkEntry::obtained(0));
    }

    #[test]
    fn patch_can_introduce_new_student() {
        let existing: BTreeMap<StudentId, MarkEntry> = BTreeMap::new();
        let mut patch: BTreeMap<StudentId, MarkPatch> = BTreeMap::new();
        patch.insert(sid("s1"), MarkPatch::Set(30));
        let result = apply_patch(&existing, &patch);
        assert_eq!(result[&sid("s1")], MarkEntry::obtained(30));
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn null_entry_never_becomes_zero() {
        // A NULL that is not in the patch stays NULL, never 0.
        let mut existing: BTreeMap<StudentId, MarkEntry> = BTreeMap::new();
        existing.insert(sid("s1"), MarkEntry::NULL);
        existing.insert(sid("s2"), MarkEntry::obtained(10));
        let mut patch: BTreeMap<StudentId, MarkPatch> = BTreeMap::new();
        patch.insert(sid("s2"), MarkPatch::Set(20));
        let result = apply_patch(&existing, &patch);
        assert!(result[&sid("s1")].is_null());
        assert_ne!(result[&sid("s1")], MarkEntry::obtained(0));
    }

    #[test]
    fn patched_entries_still_validate() {
        let existing = full_sheet_34();
        let mut patch: BTreeMap<StudentId, MarkPatch> = BTreeMap::new();
        patch.insert(sid("s00"), MarkPatch::Set(101)); // over max
        let result = apply_patch(&existing, &patch);
        assert_eq!(
            validate_entry(&result[&sid("s00")], 100),
            Err(CoreError::validation("marks", "range"))
        );
    }
}
