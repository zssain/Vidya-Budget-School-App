//! grades — grade bands and report-card grading (prompts/P02 grades.rs).
//!
//! Pure Rust: no IO, no async, no floats. Percentages are integer **tenths of a
//! percent** (e.g. `915` = 91.5 %), rounded **half-up** with integer math only.
//!
//! There is **no pass / fail / promotion** logic here (§ deliberately omitted).
//! A subject the student was **absent** for is graded `"AB"` and is **excluded**
//! from the totals and the maximum. If **any** subject mark is NULL (not
//! entered) the overall result is `Incomplete` with no grade.
//!
//! The default scale below is an **owner default** (§17.4) and is editable in
//! Settings; see the module report.

use crate::marks::MarkEntry;

/// A grade band over a percentage range, expressed in **tenths of a percent**.
///
/// The range is inclusive on both ends: a percentage `p` (tenths) is in the
/// band iff `min_pct_tenths <= p <= max_pct_tenths`. `grade_point` is `None`
/// for bands (e.g. the fail-adjacent "E") that carry no grade point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradeBand {
    /// Inclusive lower bound, tenths of a percent (e.g. `910` = 91.0 %).
    pub min_pct_tenths: u32,
    /// Inclusive upper bound, tenths of a percent (e.g. `1000` = 100.0 %).
    pub max_pct_tenths: u32,
    /// The grade label (e.g. `"A1"`).
    pub grade: String,
    /// The grade point, if the band carries one.
    pub grade_point: Option<u32>,
}

/// The grading result for a single subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubjectResult {
    /// A graded subject: its percentage (tenths) and grade label.
    Graded { pct_tenths: u32, grade: String },
    /// The student was absent — graded `"AB"`, excluded from totals.
    Absent,
    /// The mark was NULL (not entered).
    Incomplete,
}

/// The aggregate result for a whole report card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportResult {
    /// Every entered subject graded: overall percentage (tenths) and grade.
    /// Absent subjects are excluded from the totals; if `grade` is `None` the
    /// overall percentage fell outside every band.
    Graded { pct_tenths: u32, grade: Option<String> },
    /// At least one subject mark was NULL → the card is incomplete.
    Incomplete,
}

/// The percentage (tenths) a subject scored: `round_half_up(1000 * obtained /
/// max)`.
///
/// No floats. Half-up on non-negative integers is `(2*num + den) / (2*den)`
/// with `num = 1000 * obtained`, `den = max`:
/// `(2000 * obtained + max) / (2 * max)`. `u64` intermediates avoid overflow.
/// `max == 0` → `0` (a zero-mark paper cannot be a percentage).
///
/// ```
/// use vidya_core::grades::subject_percent;
/// assert_eq!(subject_percent(915, 1000), 915); // 91.5 % held exactly
/// assert_eq!(subject_percent(1, 3), 333);      // 33.33.. % → 333 tenths
/// ```
pub fn subject_percent(obtained: u32, max: u32) -> u32 {
    if max == 0 {
        return 0;
    }
    let num = 2000u64 * obtained as u64;
    let den = 2u64 * max as u64;
    ((num + max as u64) / den) as u32
}

/// The band containing `pct_tenths`, or `None` if it falls outside every band.
///
/// Bands are inclusive on both ends (see [`GradeBand`]). The default scale's
/// bands are contiguous, so a percentage maps to exactly one band.
pub fn grade_for(pct_tenths: u32, scale: &[GradeBand]) -> Option<&GradeBand> {
    scale
        .iter()
        .find(|b| pct_tenths >= b.min_pct_tenths && pct_tenths <= b.max_pct_tenths)
}

/// The owner-default CBSE-style grade scale (§17.4), in **tenths of a percent**.
///
/// | Grade | Range (%) | Range (tenths) | Grade point |
/// |-------|-----------|----------------|-------------|
/// | A1 | 91–100 | 910..=1000 | 10 |
/// | A2 | 81–90  | 810..=909  | 9  |
/// | B1 | 71–80  | 710..=809  | 8  |
/// | B2 | 61–70  | 610..=709  | 7  |
/// | C1 | 51–60  | 510..=609  | 6  |
/// | C2 | 41–50  | 410..=509  | 5  |
/// | D  | 33–40  | 330..=409  | 4  |
/// | E  | 0–32   | 0..=329    | None |
///
/// Boundaries are contiguous and inclusive: 33.0 % (`330`) is D, 32.9 % (`329`)
/// is E; 91.0 % (`910`) is A1, 90.9 % (`909`) is A2. This is an owner default
/// and is editable in Settings.
pub fn default_scale() -> Vec<GradeBand> {
    fn band(min: u32, max: u32, grade: &str, gp: Option<u32>) -> GradeBand {
        GradeBand { min_pct_tenths: min, max_pct_tenths: max, grade: grade.to_string(), grade_point: gp }
    }
    vec![
        band(910, 1000, "A1", Some(10)),
        band(810, 909, "A2", Some(9)),
        band(710, 809, "B1", Some(8)),
        band(610, 709, "B2", Some(7)),
        band(510, 609, "C1", Some(6)),
        band(410, 509, "C2", Some(5)),
        band(330, 409, "D", Some(4)),
        band(0, 329, "E", None),
    ]
}

/// Grade a single subject's [`MarkEntry`] against `max_marks` and `scale`.
///
/// * Absent → [`SubjectResult::Absent`] (graded "AB", excluded from totals).
/// * NULL → [`SubjectResult::Incomplete`].
/// * A number → [`SubjectResult::Graded`] with its percentage (tenths) and the
///   band's grade (empty string if it falls outside every band — the default
///   scale covers 0..=1000 so this cannot happen with it).
pub fn grade_subject(entry: &MarkEntry, max_marks: u32, scale: &[GradeBand]) -> SubjectResult {
    if entry.absent {
        return SubjectResult::Absent;
    }
    match entry.marks {
        None => SubjectResult::Incomplete,
        Some(obtained) => {
            let pct = subject_percent(obtained, max_marks);
            let grade = grade_for(pct, scale).map(|b| b.grade.clone()).unwrap_or_default();
            SubjectResult::Graded { pct_tenths: pct, grade }
        }
    }
}

/// One subject line for a report card: an entry paired with its max marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubjectMark {
    /// The student's entry for this subject.
    pub entry: MarkEntry,
    /// The subject's maximum marks.
    pub max_marks: u32,
}

/// Aggregate a report card across subjects.
///
/// * If **any** subject is NULL → [`ReportResult::Incomplete`] (no grade).
/// * Absent subjects are **excluded** from both the obtained total and the max
///   total (they do not count against the student).
/// * Otherwise the overall percentage is `round_half_up(1000 * sum_obtained /
///   sum_max)` over the non-absent, non-null subjects, graded via `scale`.
/// * With **no** counting subjects (every subject absent, or an empty card) the
///   overall percentage is `0` and its grade comes from the scale for `0`.
pub fn grade_report(subjects: &[SubjectMark], scale: &[GradeBand]) -> ReportResult {
    let mut sum_obtained: u64 = 0;
    let mut sum_max: u64 = 0;
    for s in subjects {
        if s.entry.absent {
            continue; // excluded from totals
        }
        match s.entry.marks {
            None => return ReportResult::Incomplete,
            Some(m) => {
                sum_obtained += m as u64;
                sum_max += s.max_marks as u64;
            }
        }
    }
    let pct = subject_percent_u64(sum_obtained, sum_max);
    let grade = grade_for(pct, scale).map(|b| b.grade.clone());
    ReportResult::Graded { pct_tenths: pct, grade }
}

/// `subject_percent` on `u64` totals (report-level aggregation). Half-up,
/// tenths; `max == 0` → `0`.
fn subject_percent_u64(obtained: u64, max: u64) -> u32 {
    if max == 0 {
        return 0;
    }
    let num = 2000u64 * obtained;
    let den = 2u64 * max;
    ((num + max) / den) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scale() -> Vec<GradeBand> {
        default_scale()
    }

    fn grade_at(pct_tenths: u32) -> Option<String> {
        grade_for(pct_tenths, &scale()).map(|b| b.grade.clone())
    }

    // ---- subject_percent (half-up, tenths) --------------------------------

    #[test]
    fn subject_percent_exact() {
        assert_eq!(subject_percent(50, 100), 500); // 50.0 %
        assert_eq!(subject_percent(100, 100), 1000); // 100.0 %
        assert_eq!(subject_percent(0, 100), 0); // 0.0 %
    }

    #[test]
    fn subject_percent_max_zero_is_zero() {
        assert_eq!(subject_percent(0, 0), 0);
        assert_eq!(subject_percent(5, 0), 0);
    }

    #[test]
    fn subject_percent_rounds_half_up() {
        // 1/3 = 33.33.. % → 333 tenths (rounds down).
        assert_eq!(subject_percent(1, 3), 333);
        // 2/3 = 66.66.. % → 667 tenths (rounds up).
        assert_eq!(subject_percent(2, 3), 667);
        // A true half at the tenth: 1/16 = 6.25 % = 62.5 tenths → half-up 63.
        assert_eq!(subject_percent(1, 16), 63);
        // 3/16 = 18.75 % = 187.5 tenths → half-up 188.
        assert_eq!(subject_percent(3, 16), 188);
    }

    // ---- default scale band boundaries -----------------------------------

    #[test]
    fn default_scale_has_expected_bands() {
        let s = default_scale();
        assert_eq!(s.len(), 8);
        assert_eq!(s[0].grade, "A1");
        assert_eq!(s[0].grade_point, Some(10));
        assert_eq!(s[7].grade, "E");
        assert_eq!(s[7].grade_point, None);
    }

    #[test]
    fn boundary_32_vs_33() {
        // 32.9 % (329) is E; 33.0 % (330) is D.
        assert_eq!(grade_at(329).as_deref(), Some("E"));
        assert_eq!(grade_at(330).as_deref(), Some("D"));
    }

    #[test]
    fn boundary_40_vs_41() {
        assert_eq!(grade_at(409).as_deref(), Some("D"));
        assert_eq!(grade_at(410).as_deref(), Some("C2"));
    }

    #[test]
    fn boundary_90_vs_91() {
        // 90.9 % (909) is A2; 91.0 % (910) is A1.
        assert_eq!(grade_at(909).as_deref(), Some("A2"));
        assert_eq!(grade_at(910).as_deref(), Some("A1"));
    }

    #[test]
    fn boundary_extremes() {
        assert_eq!(grade_at(0).as_deref(), Some("E"));
        assert_eq!(grade_at(1000).as_deref(), Some("A1"));
    }

    #[test]
    fn default_scale_is_contiguous_and_covers_0_to_1000() {
        let s = default_scale();
        // Every tenth from 0..=1000 maps to exactly one band.
        for p in 0..=1000u32 {
            let matches: Vec<&GradeBand> =
                s.iter().filter(|b| p >= b.min_pct_tenths && p <= b.max_pct_tenths).collect();
            assert_eq!(matches.len(), 1, "pct {p} must map to exactly one band");
        }
    }

    // ---- grade_subject ----------------------------------------------------

    #[test]
    fn grade_subject_number() {
        // 91/100 = 91.0 % → A1.
        assert_eq!(
            grade_subject(&MarkEntry::obtained(91), 100, &scale()),
            SubjectResult::Graded { pct_tenths: 910, grade: "A1".into() }
        );
    }

    #[test]
    fn grade_subject_absent() {
        assert_eq!(grade_subject(&MarkEntry::absent(), 100, &scale()), SubjectResult::Absent);
    }

    #[test]
    fn grade_subject_null_incomplete() {
        assert_eq!(grade_subject(&MarkEntry::NULL, 100, &scale()), SubjectResult::Incomplete);
    }

    // ---- grade_report -----------------------------------------------------

    fn sm(entry: MarkEntry, max: u32) -> SubjectMark {
        SubjectMark { entry, max_marks: max }
    }

    #[test]
    fn report_all_graded() {
        // 90 + 80 + 70 of 100 each = 240/300 = 80.0 % → 800 tenths → B1
        // (B1 is 71–80, i.e. 710..=809).
        let subjects = [
            sm(MarkEntry::obtained(90), 100),
            sm(MarkEntry::obtained(80), 100),
            sm(MarkEntry::obtained(70), 100),
        ];
        assert_eq!(
            grade_report(&subjects, &scale()),
            ReportResult::Graded { pct_tenths: 800, grade: Some("B1".into()) }
        );
    }

    #[test]
    fn report_absent_excluded_from_totals() {
        // Two subjects count: 90/100 and 80/100 = 170/200 = 85.0 % → A2.
        // The absent third subject (max 50) is excluded entirely; had it been
        // counted as 0/50 the total would be 170/250 = 68.0 % (B2).
        let subjects = [
            sm(MarkEntry::obtained(90), 100),
            sm(MarkEntry::obtained(80), 100),
            sm(MarkEntry::absent(), 50),
        ];
        assert_eq!(
            grade_report(&subjects, &scale()),
            ReportResult::Graded { pct_tenths: 850, grade: Some("A2".into()) }
        );
    }

    #[test]
    fn report_incomplete_when_any_null() {
        let subjects = [
            sm(MarkEntry::obtained(90), 100),
            sm(MarkEntry::NULL, 100),
            sm(MarkEntry::obtained(70), 100),
        ];
        assert_eq!(grade_report(&subjects, &scale()), ReportResult::Incomplete);
    }

    #[test]
    fn report_incomplete_beats_absent() {
        // A NULL anywhere makes the whole card Incomplete even with absents.
        let subjects = [sm(MarkEntry::absent(), 100), sm(MarkEntry::NULL, 100)];
        assert_eq!(grade_report(&subjects, &scale()), ReportResult::Incomplete);
    }

    #[test]
    fn report_all_absent_is_zero_percent() {
        // No counting subjects → 0 % → E (grade point None).
        let subjects = [sm(MarkEntry::absent(), 100), sm(MarkEntry::absent(), 100)];
        assert_eq!(
            grade_report(&subjects, &scale()),
            ReportResult::Graded { pct_tenths: 0, grade: Some("E".into()) }
        );
    }

    #[test]
    fn report_half_up_rounding() {
        // 1/16 across one subject = 6.25 % = 62.5 tenths → half-up 63 → E.
        let subjects = [sm(MarkEntry::obtained(1), 16)];
        assert_eq!(
            grade_report(&subjects, &scale()),
            ReportResult::Graded { pct_tenths: 63, grade: Some("E".into()) }
        );
    }

    #[test]
    fn report_mixed_max_marks_totals() {
        // 45/50 + 90/100 = 135/150 = 90.0 % → 900 tenths → A2.
        let subjects = [sm(MarkEntry::obtained(45), 50), sm(MarkEntry::obtained(90), 100)];
        assert_eq!(
            grade_report(&subjects, &scale()),
            ReportResult::Graded { pct_tenths: 900, grade: Some("A2".into()) }
        );
    }
}
