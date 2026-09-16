use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::error::DomainError;

/// A mark cell before it is persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkValue {
    Blank,
    Absent,
    Score(u16),
}

/// Aggregate totals for one student's exam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExamResult {
    pub got: u32,
    pub max: u32,
    pub entered: u32,
    pub percent_tenths: Option<u32>,
}

/// One descending band in a grade scale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GradeBand {
    pub grade: String,
    pub min_percent: u32,
}

/// Parses a blank, `AB`, or integer score bounded by `max`.
pub fn parse_mark(input: &str, max: u16) -> Result<MarkValue, DomainError> {
    let value = input.trim();
    if value.is_empty() {
        return Ok(MarkValue::Blank);
    }
    if value.eq_ignore_ascii_case("ab") {
        return Ok(MarkValue::Absent);
    }
    match value.parse::<u16>() {
        Ok(score) if score <= max => Ok(MarkValue::Score(score)),
        _ => Err(DomainError::validation("marks.error.invalid")
            .field("mark")
            .param("max", max.to_string())),
    }
}

/// Calculates entered subjects, totals and half-up percentage tenths.
pub fn exam_result(values: &[MarkValue], max_per_subject: u16) -> ExamResult {
    let mut got = 0_u32;
    let mut max = 0_u32;
    let mut entered = 0_u32;
    for value in values {
        match value {
            MarkValue::Blank => {}
            MarkValue::Absent => {
                entered = entered.saturating_add(1);
                max = max.saturating_add(u32::from(max_per_subject));
            }
            MarkValue::Score(score) => {
                entered = entered.saturating_add(1);
                got = got.saturating_add(u32::from(*score));
                max = max.saturating_add(u32::from(max_per_subject));
            }
        }
    }
    let percent_tenths = (max > 0).then(|| {
        let numerator = u64::from(got) * 1_000 + u64::from(max) / 2;
        (numerator / u64::from(max)) as u32
    });
    ExamResult {
        got,
        max,
        entered,
        percent_tenths,
    }
}

/// Returns the first matching grade from a descending validated scale.
pub fn grade_for(percent_tenths: u32, scale: &[GradeBand]) -> String {
    scale
        .iter()
        .find(|band| percent_tenths >= band.min_percent.saturating_mul(10))
        .map(|band| band.grade.clone())
        .unwrap_or_default()
}

/// Requires unique grade names, strictly decreasing minima and a final zero band.
pub fn validate_grade_scale(scale: &[GradeBand]) -> Result<(), DomainError> {
    let distinct = scale
        .iter()
        .map(|band| &band.grade)
        .collect::<BTreeSet<_>>()
        .len()
        == scale.len();
    let decreasing = scale
        .windows(2)
        .all(|pair| pair[0].min_percent > pair[1].min_percent);
    let ends_at_zero = scale.last().is_some_and(|band| band.min_percent == 0);
    let valid_range = scale
        .iter()
        .all(|band| !band.grade.trim().is_empty() && band.min_percent <= 100);
    if distinct && decreasing && ends_at_zero && valid_range {
        Ok(())
    } else {
        Err(DomainError::validation("marks.error.grade_scale").field("grade_scale"))
    }
}
