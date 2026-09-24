//! session — new-session rollover rules (prompts/P08 Part D). Pure: no IO, no
//! clock, no floats.
//!
//! The rollover promotes every student one class up the ladder
//! `Nursery → LKG → UKG → I → … → XII → "Passed out"`, keeping the section, with
//! per-student overrides (repeat the class, or leave). Unpaid balances carry
//! forward as a single "Previous balance" due in the new Term 1, linked to — and
//! never replacing — the old dues. The commit itself (one transaction + one audit
//! entry, old session set read-only) lives in `src-tauri`.

use crate::money::Paise;
use serde::{Deserialize, Serialize};

/// The promotion ladder by `class.name` (§7 `class.name`: `Nursery`, `LKG`,
/// `UKG`, then Roman `I`..`XII`). Sections are not part of the ladder — they are
/// carried across unchanged.
pub const LADDER: &[&str] = &[
    "Nursery", "LKG", "UKG", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI",
    "XII",
];

/// Where a class promotes to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Promotion {
    /// Promotes to this class name (same section kept by the caller).
    To { name: String },
    /// The top of the ladder (`XII`) — the student passes out.
    PassedOut,
    /// The class name is not on the ladder — the caller must decide (never
    /// auto-promoted).
    Unknown,
}

/// The next class up the ladder for `name` (§Part D).
///
/// ```
/// use vidya_core::session::{next_class, Promotion};
/// assert_eq!(next_class("Nursery"), Promotion::To { name: "LKG".into() });
/// assert_eq!(next_class("XII"), Promotion::PassedOut);
/// assert_eq!(next_class("Playgroup"), Promotion::Unknown);
/// ```
pub fn next_class(name: &str) -> Promotion {
    match LADDER.iter().position(|c| *c == name) {
        Some(i) if i + 1 < LADDER.len() => Promotion::To { name: LADDER[i + 1].to_string() },
        Some(_) => Promotion::PassedOut, // last rung = XII
        None => Promotion::Unknown,
    }
}

/// The per-student choice in the wizard preview (Part D: "per-student override
/// (repeat class / leave)").
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Override {
    /// Default: promote one class up the ladder.
    #[default]
    Promote,
    /// Keep the student in the same class next session.
    Repeat,
    /// The student leaves; no enrollment in the new session.
    Leave,
}

/// A student entering the rollover, reduced to the fields promotion needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudentIn {
    pub student_id: String,
    /// The student's current `class.name` (e.g. `"VII"`).
    pub class_name: String,
    /// The current section, kept across the rollover (e.g. `Some("B")`).
    pub section: Option<String>,
    #[serde(default)]
    pub over: Override,
}

/// What the rollover does to a student.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Enrolled in the next class up (same section).
    Promoted,
    /// Re-enrolled in the same class (repeat).
    Repeated,
    /// Reached the top of the ladder — status `left`, reason `Passed out`.
    PassedOut,
    /// Left by override — status `left`.
    Left,
    /// The class name is off the ladder — needs a manual decision; not enrolled.
    Unknown,
}

impl Outcome {
    /// The stable reason recorded on `student.left_reason` for the two leaving
    /// outcomes (English; the UI/report language translates for display). `None`
    /// for outcomes that keep the student enrolled.
    pub fn left_reason(self) -> Option<&'static str> {
        match self {
            Outcome::PassedOut => Some("Passed out"),
            Outcome::Left => Some("Left"),
            _ => None,
        }
    }
}

/// One row of the rollover preview / commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionRow {
    pub student_id: String,
    pub outcome: Outcome,
    /// The destination `class.name` when the student stays enrolled (promote or
    /// repeat); `None` when they leave / pass out / are unknown.
    pub to_name: Option<String>,
    /// The destination section (unchanged from the source), when enrolled.
    pub to_section: Option<String>,
}

/// Build the rollover plan for every student (Part D preview table).
///
/// * [`Override::Leave`] → [`Outcome::Left`] (no new enrollment);
/// * [`Override::Repeat`] → [`Outcome::Repeated`] in the same class + section;
/// * [`Override::Promote`] (default) → the ladder:
///   * a next class → [`Outcome::Promoted`] into it, same section;
///   * top of the ladder → [`Outcome::PassedOut`] (status `left`, reason
///     `Passed out`);
///   * an off-ladder class → [`Outcome::Unknown`] (not enrolled; the wizard flags
///     it for a manual choice).
pub fn build_promotion(students: &[StudentIn]) -> Vec<PromotionRow> {
    students.iter().map(promote_one).collect()
}

fn promote_one(s: &StudentIn) -> PromotionRow {
    let row = |outcome: Outcome, to_name: Option<String>| PromotionRow {
        student_id: s.student_id.clone(),
        outcome,
        to_name: to_name.clone(),
        to_section: to_name.as_ref().and(s.section.clone()),
    };
    match s.over {
        Override::Leave => row(Outcome::Left, None),
        Override::Repeat => row(Outcome::Repeated, Some(s.class_name.clone())),
        Override::Promote => match next_class(&s.class_name) {
            Promotion::To { name } => row(Outcome::Promoted, Some(name)),
            Promotion::PassedOut => row(Outcome::PassedOut, None),
            Promotion::Unknown => row(Outcome::Unknown, None),
        },
    }
}

/// Counts for the wizard summary line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionSummary {
    pub promoted: u32,
    pub repeated: u32,
    pub passed_out: u32,
    pub left: u32,
    pub unknown: u32,
}

/// Tally a plan by outcome.
pub fn summarize(rows: &[PromotionRow]) -> PromotionSummary {
    let mut s = PromotionSummary::default();
    for r in rows {
        match r.outcome {
            Outcome::Promoted => s.promoted += 1,
            Outcome::Repeated => s.repeated += 1,
            Outcome::PassedOut => s.passed_out += 1,
            Outcome::Left => s.left += 1,
            Outcome::Unknown => s.unknown += 1,
        }
    }
    s
}

// ---- carry-forward of unpaid balances -----------------------------------

/// A student's outstanding balance from the closing session, with the ids of the
/// dues that make it up (so the carried-forward due can link back to them).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudentBalance {
    pub student_id: String,
    /// Total outstanding across the old session's unpaid/partly-paid dues, paise.
    pub balance_paise: Paise,
    /// The old `fee_due` ids this balance came from (kept for the link; never
    /// deleted).
    pub old_due_ids: Vec<String>,
}

/// A single "Previous balance" due to insert in the new session's Term 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CarryForwardDue {
    pub student_id: String,
    pub amount_paise: Paise,
    /// The new Term 1 period id/name (passed in by the caller).
    pub period: String,
    /// The old dues this carried-forward balance corresponds to (never deleted).
    pub linked_due_ids: Vec<String>,
}

/// Compute the carry-forward dues for the new session (Part D: "unpaid dues → one
/// 'Previous balance' due per student in the new Term 1 (linked to old dues, never
/// deleting them)").
///
/// One due per student **with a positive balance**, for `new_term1_period`.
/// Students who owe nothing (balance ≤ 0) get no row. Old dues are referenced by
/// id only — the caller keeps them intact.
pub fn carry_forward(balances: &[StudentBalance], new_term1_period: &str) -> Vec<CarryForwardDue> {
    balances
        .iter()
        .filter(|b| b.balance_paise.is_positive())
        .map(|b| CarryForwardDue {
            student_id: b.student_id.clone(),
            amount_paise: b.balance_paise,
            period: new_term1_period.to_string(),
            linked_due_ids: b.old_due_ids.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(id: &str, class: &str, section: Option<&str>, over: Override) -> StudentIn {
        StudentIn {
            student_id: id.into(),
            class_name: class.into(),
            section: section.map(|x| x.into()),
            over,
        }
    }

    // ---- next_class ----

    #[test]
    fn ladder_full_walk() {
        assert_eq!(next_class("Nursery"), Promotion::To { name: "LKG".into() });
        assert_eq!(next_class("LKG"), Promotion::To { name: "UKG".into() });
        assert_eq!(next_class("UKG"), Promotion::To { name: "I".into() });
        assert_eq!(next_class("I"), Promotion::To { name: "II".into() });
        assert_eq!(next_class("XI"), Promotion::To { name: "XII".into() });
        assert_eq!(next_class("XII"), Promotion::PassedOut);
    }

    #[test]
    fn every_rung_but_the_last_promotes_to_the_next_rung() {
        for w in LADDER.windows(2) {
            assert_eq!(next_class(w[0]), Promotion::To { name: w[1].to_string() });
        }
    }

    #[test]
    fn off_ladder_is_unknown() {
        assert_eq!(next_class("Playgroup"), Promotion::Unknown);
        assert_eq!(next_class("XIII"), Promotion::Unknown);
        assert_eq!(next_class(""), Promotion::Unknown);
    }

    // ---- build_promotion ----

    #[test]
    fn promote_keeps_section() {
        let rows = build_promotion(&[s("a", "VII", Some("B"), Override::Promote)]);
        assert_eq!(rows[0].outcome, Outcome::Promoted);
        assert_eq!(rows[0].to_name.as_deref(), Some("VIII"));
        assert_eq!(rows[0].to_section.as_deref(), Some("B"));
    }

    #[test]
    fn xii_passes_out_with_reason() {
        let rows = build_promotion(&[s("a", "XII", Some("A"), Override::Promote)]);
        assert_eq!(rows[0].outcome, Outcome::PassedOut);
        assert_eq!(rows[0].to_name, None);
        assert_eq!(rows[0].to_section, None);
        assert_eq!(rows[0].outcome.left_reason(), Some("Passed out"));
    }

    #[test]
    fn repeat_keeps_same_class_and_section() {
        let rows = build_promotion(&[s("a", "V", Some("A"), Override::Repeat)]);
        assert_eq!(rows[0].outcome, Outcome::Repeated);
        assert_eq!(rows[0].to_name.as_deref(), Some("V"));
        assert_eq!(rows[0].to_section.as_deref(), Some("A"));
        assert_eq!(rows[0].outcome.left_reason(), None);
    }

    #[test]
    fn leave_produces_no_enrollment() {
        let rows = build_promotion(&[s("a", "III", Some("A"), Override::Leave)]);
        assert_eq!(rows[0].outcome, Outcome::Left);
        assert_eq!(rows[0].to_name, None);
        assert_eq!(rows[0].outcome.left_reason(), Some("Left"));
    }

    #[test]
    fn unknown_class_is_not_auto_promoted() {
        let rows = build_promotion(&[s("a", "Playgroup", None, Override::Promote)]);
        assert_eq!(rows[0].outcome, Outcome::Unknown);
        assert_eq!(rows[0].to_name, None);
    }

    #[test]
    fn nursery_promotes_to_lkg_no_section() {
        let rows = build_promotion(&[s("a", "Nursery", None, Override::Promote)]);
        assert_eq!(rows[0].outcome, Outcome::Promoted);
        assert_eq!(rows[0].to_name.as_deref(), Some("LKG"));
        assert_eq!(rows[0].to_section, None);
    }

    #[test]
    fn summary_tallies_every_outcome() {
        let rows = build_promotion(&[
            s("a", "I", Some("A"), Override::Promote),
            s("b", "II", Some("A"), Override::Repeat),
            s("c", "XII", Some("A"), Override::Promote),
            s("d", "III", Some("A"), Override::Leave),
            s("e", "Playgroup", None, Override::Promote),
        ]);
        let sum = summarize(&rows);
        assert_eq!(
            sum,
            PromotionSummary { promoted: 1, repeated: 1, passed_out: 1, left: 1, unknown: 1 }
        );
    }

    // ---- carry_forward ----

    #[test]
    fn carry_forward_one_due_per_positive_balance() {
        let balances = [
            StudentBalance {
                student_id: "a".into(),
                balance_paise: Paise(310000),
                old_due_ids: vec!["d1".into(), "d2".into()],
            },
            StudentBalance {
                student_id: "b".into(),
                balance_paise: Paise::ZERO,
                old_due_ids: vec!["d3".into()],
            },
        ];
        let cf = carry_forward(&balances, "T1");
        assert_eq!(cf.len(), 1, "only the student who owes money carries forward");
        assert_eq!(cf[0].student_id, "a");
        assert_eq!(cf[0].amount_paise, Paise(310000));
        assert_eq!(cf[0].period, "T1");
        assert_eq!(cf[0].linked_due_ids, vec!["d1".to_string(), "d2".to_string()]);
    }

    #[test]
    fn carry_forward_skips_zero_and_negative() {
        let balances = [
            StudentBalance {
                student_id: "a".into(),
                balance_paise: Paise::ZERO,
                old_due_ids: vec![],
            },
            StudentBalance {
                student_id: "b".into(),
                balance_paise: Paise(-500),
                old_due_ids: vec![],
            },
        ];
        assert!(carry_forward(&balances, "T1").is_empty());
    }
}
