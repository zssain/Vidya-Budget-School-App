//! timetable — clash detection and free-teacher search for the weekly timetable
//! (P16, 00-SYSTEM-CONTEXT §10.4 / §14 Classroom).
//!
//! Pure Rust: no IO, no async, no clock, no randomness. The database side
//! (`src-tauri`) loads the slots, the canonical class-subject → teacher
//! assignments and the teachers on leave, and passes them in; this module only
//! *decides*. Weekdays are ISO (Monday = 1 … Sunday = 7), matching
//! `vidya_core::calendar::weekday_iso` and `school_week.weekday`.
//!
//! The three clashes core rejects (00-SYSTEM-CONTEXT §10.4 "core rejects
//! clashes"): a teacher double-booked, a class double-booked, and a teacher put
//! in a slot for a class-subject they are not assigned to teach.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// One timetable slot: on `weekday`, in `period_no`, class `class_id` studies
/// `class_subject_id` taught by `teacher_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    pub id: String,
    pub class_id: String,
    /// ISO weekday, Monday = 1 … Saturday = 6 (Sunday = 7 is rarely used).
    pub weekday: u8,
    pub period_no: u8,
    pub class_subject_id: String,
    pub teacher_id: String,
}

/// A clash found in a set of slots. Reported with enough context for an inline
/// UI error on the slot editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Clash {
    /// The same teacher is placed in two classes at the same weekday + period.
    TeacherDoubleBooked { weekday: u8, period_no: u8, teacher_id: String, slot_ids: [String; 2] },
    /// The same class is placed in two subjects at the same weekday + period.
    ClassDoubleBooked { weekday: u8, period_no: u8, class_id: String, slot_ids: [String; 2] },
    /// The slot's teacher is not the teacher assigned to that class-subject.
    TeacherNotAssigned { slot_id: String, class_subject_id: String, teacher_id: String },
}

/// Every clash in `slots`, given the canonical `class_subject_id → teacher_id`
/// assignments (from `class_subject.teacher_id`). Deterministic: the result order
/// follows the order of `slots`. An empty result means the timetable is valid.
pub fn clashes(slots: &[Slot], assigned: &BTreeMap<String, String>) -> Vec<Clash> {
    let mut out = Vec::new();
    // First seen (weekday, period, teacher) / (weekday, period, class) → slot id.
    let mut seen_teacher: BTreeMap<(u8, u8, String), String> = BTreeMap::new();
    let mut seen_class: BTreeMap<(u8, u8, String), String> = BTreeMap::new();

    for s in slots {
        // The slot's teacher must match the class-subject's assigned teacher.
        match assigned.get(&s.class_subject_id) {
            Some(t) if *t == s.teacher_id => {}
            _ => out.push(Clash::TeacherNotAssigned {
                slot_id: s.id.clone(),
                class_subject_id: s.class_subject_id.clone(),
                teacher_id: s.teacher_id.clone(),
            }),
        }

        let tkey = (s.weekday, s.period_no, s.teacher_id.clone());
        if let Some(prev) = seen_teacher.get(&tkey) {
            out.push(Clash::TeacherDoubleBooked {
                weekday: s.weekday,
                period_no: s.period_no,
                teacher_id: s.teacher_id.clone(),
                slot_ids: [prev.clone(), s.id.clone()],
            });
        } else {
            seen_teacher.insert(tkey, s.id.clone());
        }

        let ckey = (s.weekday, s.period_no, s.class_id.clone());
        if let Some(prev) = seen_class.get(&ckey) {
            out.push(Clash::ClassDoubleBooked {
                weekday: s.weekday,
                period_no: s.period_no,
                class_id: s.class_id.clone(),
                slot_ids: [prev.clone(), s.id.clone()],
            });
        } else {
            seen_class.insert(ckey, s.id.clone());
        }
    }
    out
}

/// True if adding/keeping `candidate` among `others` (all *other* slots in the
/// timetable) introduces any clash. Used by the slot editor to validate one edit
/// without re-checking the whole week. `assigned` is the canonical map.
pub fn slot_is_valid(
    candidate: &Slot,
    others: &[Slot],
    assigned: &BTreeMap<String, String>,
) -> Result<(), Vec<Clash>> {
    let mut all: Vec<Slot> = Vec::with_capacity(others.len() + 1);
    // Put the candidate first so any clash names the candidate as the earlier id
    // where relevant; the caller only needs the presence + reasons.
    all.push(candidate.clone());
    all.extend(others.iter().cloned());
    let found: Vec<Clash> = clashes(&all, assigned)
        .into_iter()
        .filter(|c| involves(c, &candidate.id))
        .collect();
    if found.is_empty() {
        Ok(())
    } else {
        Err(found)
    }
}

/// Whether a clash involves the given slot id.
fn involves(clash: &Clash, slot_id: &str) -> bool {
    match clash {
        Clash::TeacherDoubleBooked { slot_ids, .. } | Clash::ClassDoubleBooked { slot_ids, .. } => {
            slot_ids.iter().any(|s| s == slot_id)
        }
        Clash::TeacherNotAssigned { slot_id: s, .. } => s == slot_id,
    }
}

/// A teacher's slots on one weekday, sorted by period (their "day"). The date →
/// weekday conversion happens in the caller (school calendar); this stays
/// clockless.
pub fn teacher_day(teacher_id: &str, weekday: u8, slots: &[Slot]) -> Vec<Slot> {
    let mut day: Vec<Slot> = slots
        .iter()
        .filter(|s| s.teacher_id == teacher_id && s.weekday == weekday)
        .cloned()
        .collect();
    day.sort_by_key(|s| s.period_no);
    day
}

/// Teachers free in `period_no` on `weekday`: candidates who are not already
/// teaching a slot in that period and are not on leave that day. Order follows
/// `teachers`. Used to fill a substitute's cover (P16 Step 2).
pub fn free_teachers(
    weekday: u8,
    period_no: u8,
    slots: &[Slot],
    teachers: &[String],
    on_leave: &BTreeSet<String>,
) -> Vec<String> {
    let busy: BTreeSet<&str> = slots
        .iter()
        .filter(|s| s.weekday == weekday && s.period_no == period_no)
        .map(|s| s.teacher_id.as_str())
        .collect();
    teachers
        .iter()
        .filter(|t| !busy.contains(t.as_str()) && !on_leave.contains(*t))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(id: &str, class: &str, wd: u8, p: u8, cs: &str, teacher: &str) -> Slot {
        Slot {
            id: id.into(),
            class_id: class.into(),
            weekday: wd,
            period_no: p,
            class_subject_id: cs.into(),
            teacher_id: teacher.into(),
        }
    }

    fn assigned(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(cs, t)| (cs.to_string(), t.to_string())).collect()
    }

    #[test]
    fn a_valid_week_has_no_clashes() {
        let slots = vec![
            slot("s1", "cls-5a", 1, 1, "cs-5a-eng", "t-meena"),
            slot("s2", "cls-6b", 1, 1, "cs-6b-mat", "t-anita"),
            slot("s3", "cls-5a", 1, 2, "cs-5a-eng", "t-meena"),
        ];
        let asg = assigned(&[("cs-5a-eng", "t-meena"), ("cs-6b-mat", "t-anita")]);
        assert!(clashes(&slots, &asg).is_empty());
    }

    #[test]
    fn teacher_double_booked_is_flagged() {
        // t-meena teaches two classes in weekday 1 period 1.
        let slots = vec![
            slot("s1", "cls-5a", 1, 1, "cs-5a-eng", "t-meena"),
            slot("s2", "cls-6b", 1, 1, "cs-6b-eng", "t-meena"),
        ];
        let asg = assigned(&[("cs-5a-eng", "t-meena"), ("cs-6b-eng", "t-meena")]);
        let cs = clashes(&slots, &asg);
        assert_eq!(
            cs,
            vec![Clash::TeacherDoubleBooked {
                weekday: 1,
                period_no: 1,
                teacher_id: "t-meena".into(),
                slot_ids: ["s1".into(), "s2".into()],
            }]
        );
    }

    #[test]
    fn class_double_booked_is_flagged() {
        // cls-5a has two subjects in weekday 2 period 3 (different teachers).
        let slots = vec![
            slot("s1", "cls-5a", 2, 3, "cs-5a-eng", "t-meena"),
            slot("s2", "cls-5a", 2, 3, "cs-5a-mat", "t-anita"),
        ];
        let asg = assigned(&[("cs-5a-eng", "t-meena"), ("cs-5a-mat", "t-anita")]);
        let cs = clashes(&slots, &asg);
        assert_eq!(
            cs,
            vec![Clash::ClassDoubleBooked {
                weekday: 2,
                period_no: 3,
                class_id: "cls-5a".into(),
                slot_ids: ["s1".into(), "s2".into()],
            }]
        );
    }

    #[test]
    fn teacher_not_assigned_to_the_class_subject_is_flagged() {
        // s1 puts t-nair on cs-5a-eng, which is assigned to t-meena.
        let slots = vec![slot("s1", "cls-5a", 1, 1, "cs-5a-eng", "t-nair")];
        let asg = assigned(&[("cs-5a-eng", "t-meena")]);
        let cs = clashes(&slots, &asg);
        assert_eq!(
            cs,
            vec![Clash::TeacherNotAssigned {
                slot_id: "s1".into(),
                class_subject_id: "cs-5a-eng".into(),
                teacher_id: "t-nair".into(),
            }]
        );
    }

    #[test]
    fn slot_is_valid_checks_one_edit_against_the_rest() {
        let others = vec![slot("s1", "cls-5a", 1, 1, "cs-5a-eng", "t-meena")];
        let asg = assigned(&[("cs-5a-eng", "t-meena"), ("cs-6b-eng", "t-meena")]);
        // A new slot that double-books t-meena in weekday 1 period 1 is rejected.
        let bad = slot("s2", "cls-6b", 1, 1, "cs-6b-eng", "t-meena");
        assert!(slot_is_valid(&bad, &others, &asg).is_err());
        // A slot in a different period is fine.
        let good = slot("s3", "cls-6b", 1, 2, "cs-6b-eng", "t-meena");
        assert!(slot_is_valid(&good, &others, &asg).is_ok());
    }

    #[test]
    fn teacher_day_is_sorted_by_period() {
        let slots = vec![
            slot("s1", "cls-5a", 3, 4, "cs-5a-eng", "t-meena"),
            slot("s2", "cls-5a", 3, 1, "cs-5a-eng", "t-meena"),
            slot("s3", "cls-6b", 3, 2, "cs-6b-eng", "t-meena"),
            slot("s4", "cls-5a", 2, 1, "cs-5a-eng", "t-meena"), // other weekday
        ];
        let day = teacher_day("t-meena", 3, &slots);
        assert_eq!(day.iter().map(|s| s.period_no).collect::<Vec<_>>(), vec![1, 2, 4]);
    }

    #[test]
    fn free_teachers_excludes_the_busy_and_the_on_leave() {
        let slots = vec![
            slot("s1", "cls-5a", 3, 2, "cs-5a-eng", "t-meena"),
            slot("s2", "cls-6b", 3, 2, "cs-6b-mat", "t-anita"),
        ];
        let teachers = vec!["t-meena".to_string(), "t-anita".to_string(), "t-nair".to_string()];
        let on_leave: BTreeSet<String> = ["t-nair".to_string()].into_iter().collect();
        // Period 2: meena & anita are busy, nair is on leave → nobody free.
        assert!(free_teachers(3, 2, &slots, &teachers, &on_leave).is_empty());
        // Period 4: nobody teaches; only nair is on leave → meena & anita free.
        assert_eq!(
            free_teachers(3, 4, &slots, &teachers, &on_leave),
            vec!["t-meena".to_string(), "t-anita".to_string()]
        );
    }
}
