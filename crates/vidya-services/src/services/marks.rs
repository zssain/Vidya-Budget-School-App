//! Marks entry per exam and section, and report-card data. All results, grades
//! and percentages are computed in `vidya-core`; JavaScript never does the maths.
//! A save collects every cell error before touching the database, then replaces
//! the affected cells in one transaction with a single change-log entry.

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use vidya_core::attendance::percent;
use vidya_core::error::ErrorKind;
use vidya_core::marks::{exam_result, grade_for, parse_mark, GradeBand, MarkValue};
use vidya_core::permissions::{self, Action};
use vidya_core::roles::Actor;
use vidya_db::repo::exams::ExamRow;
use vidya_db::repo::marks::MarkRow;
use vidya_db::repo::sessions::SessionRow;
use vidya_db::repo::students::EnrolledStudent;
use vidya_db::repo::subjects::SubjectRow;
use vidya_db::repo::{
    attendance, classes, exams, grade_scale, marks as marks_repo, school, sessions, students, subjects, users,
};

use crate::change_log::{write_entry, ChangeRecord, Op};
use crate::error::ServiceError;
use crate::Services;

// ---------- DTOs ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedDto {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamHeadDto {
    pub id: String,
    pub name: String,
    pub max: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkStudentDto {
    pub id: String,
    pub adm: String,
    pub roll: i64,
    pub name: String,
    pub marks: HashMap<String, String>,
    pub total: String,
    pub grade: String,
    pub percent: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarksSheetDto {
    pub exam: ExamHeadDto,
    pub exams: Vec<NamedDto>,
    pub section_id: String,
    pub subjects: Vec<NamedDto>,
    pub students: Vec<MarkStudentDto>,
    pub saved_by: Option<String>,
    pub saved_at: Option<String>,
    pub editable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchoolLite {
    pub name: String,
    pub session: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportStudentDto {
    pub name: String,
    pub ck: String,
    pub roll: i64,
    pub adm: String,
    pub father: String,
    pub dob: Option<String>,
    pub section_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRowDto {
    pub subject: String,
    pub by_exam: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportTotalDto {
    pub entered: u32,
    pub got: u32,
    pub max: u32,
    pub pct: Option<String>,
    pub grade: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceSummaryDto {
    pub p: i64,
    pub t: i64,
    pub pct: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportCardDto {
    pub school: SchoolLite,
    pub student: ReportStudentDto,
    pub exams: Vec<ExamHeadDto>,
    pub rows: Vec<ReportRowDto>,
    pub totals: Vec<ReportTotalDto>,
    pub attendance: AttendanceSummaryDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkEntry {
    pub student_id: String,
    pub subject_id: String,
    #[serde(default)]
    pub value: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveMarksInput {
    pub exam_id: String,
    pub section_id: String,
    #[serde(default)]
    pub entries: Vec<MarkEntry>,
}

pub struct MarksService<'a> {
    services: &'a Services,
}

fn pct_text(tenths: u32) -> String {
    format!("{}.{}", tenths / 10, tenths % 10)
}

fn scale_of(rows: Vec<grade_scale::GradeBandRow>) -> Vec<GradeBand> {
    rows.into_iter()
        .map(|r| GradeBand {
            grade: r.grade,
            min_percent: r.min_percent.max(0) as u32,
        })
        .collect()
}

fn entry_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

/// Turns a stored mark row into its display string ("AB" or the score).
fn mark_string(row: &MarkRow) -> String {
    if row.absent {
        "AB".to_owned()
    } else {
        row.value.unwrap_or_default().to_string()
    }
}

impl<'a> MarksService<'a> {
    pub fn new(services: &'a Services) -> Self {
        Self { services }
    }

    fn session(&self) -> Result<SessionRow, ServiceError> {
        self.services
            .db
            .read(sessions::current)?
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "session.error.none"))
    }

    fn class_of(&self, section_id: &str) -> Result<String, ServiceError> {
        self.services
            .db
            .read(|c| classes::active_section_class(c, section_id))?
            .ok_or_else(|| ServiceError::validation("students.error.section").field("sectionId"))
    }

    fn section_students(
        &self,
        session: &SessionRow,
        section_id: &str,
    ) -> Result<Vec<EnrolledStudent>, ServiceError> {
        let session_id = session.id.clone();
        let mut students: Vec<EnrolledStudent> = self
            .services
            .db
            .read(|c| students::list_enrolled(c, &session_id, "active"))?
            .into_iter()
            .filter(|e| e.section_id == section_id)
            .collect();
        students.sort_by_key(|e| e.roll);
        Ok(students)
    }

    /// The chosen exam (or the first active one when `exam_id` is blank).
    fn pick_exam(
        &self,
        session: &SessionRow,
        exam_id: &str,
    ) -> Result<(ExamRow, Vec<ExamRow>), ServiceError> {
        let session_id = session.id.clone();
        let all = self.services.db.read(|c| exams::list_active(c, &session_id))?;
        if all.is_empty() {
            return Err(ServiceError::new(ErrorKind::NotFound, "marks.error.no_exam"));
        }
        let exam = if exam_id.trim().is_empty() {
            all[0].clone()
        } else {
            all.iter()
                .find(|e| e.id == exam_id)
                .cloned()
                .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "marks.error.exam_not_found"))?
        };
        Ok((exam, all))
    }

    pub fn sheet(
        &self,
        actor: &Actor,
        exam_id: &str,
        section_id: &str,
    ) -> Result<MarksSheetDto, ServiceError> {
        permissions::authorize_section(actor, Action::MarksView, section_id)?;
        let session = self.session()?;
        let (exam, all_exams) = self.pick_exam(&session, exam_id)?;
        let class_id = self.class_of(section_id)?;
        let subjects = self.services.db.read(|c| subjects::list_active(c, &class_id))?;
        let students = self.section_students(&session, section_id)?;
        let exam_id_owned = exam.id.clone();
        let mark_rows = self
            .services
            .db
            .read(|c| marks_repo::list_for_exam(c, &exam_id_owned))?;
        let scale = scale_of(self.services.db.read(grade_scale::list)?);

        // (student_id, subject_id) -> row, and the newest saver among this section.
        let student_ids: BTreeSet<&str> = students.iter().map(|e| e.student.id.as_str()).collect();
        let mut by_cell: HashMap<(String, String), MarkRow> = HashMap::new();
        let mut newest: Option<(String, String)> = None; // (entered_at, entered_by)
        for row in mark_rows {
            if student_ids.contains(row.student_id.as_str()) {
                if newest.as_ref().is_none_or(|(at, _)| &row.entered_at > at) {
                    newest = Some((row.entered_at.clone(), row.entered_by.clone()));
                }
                by_cell.insert((row.student_id.clone(), row.subject_id.clone()), row);
            }
        }
        let (saved_at, saved_by) = match newest {
            Some((at, by)) => {
                let name = self
                    .services
                    .db
                    .read(|c| users::get_by_id(c, &by))?
                    .map(|u| u.name);
                (Some(at), name)
            }
            None => (None, None),
        };

        let max = exam.max_marks as u16;
        let students_dto = students
            .iter()
            .map(|e| {
                let mut marks = HashMap::new();
                let values: Vec<MarkValue> = subjects
                    .iter()
                    .map(|sub| match by_cell.get(&(e.student.id.clone(), sub.id.clone())) {
                        Some(row) => {
                            marks.insert(sub.id.clone(), mark_string(row));
                            if row.absent {
                                MarkValue::Absent
                            } else {
                                MarkValue::Score(row.value.unwrap_or_default().max(0) as u16)
                            }
                        }
                        None => MarkValue::Blank,
                    })
                    .collect();
                let result = exam_result(&values, max);
                let (total, grade, pct) = if result.entered > 0 {
                    let tenths = result.percent_tenths.unwrap_or(0);
                    (
                        format!("{}/{}", result.got, result.max),
                        grade_for(tenths, &scale),
                        pct_text(tenths),
                    )
                } else {
                    ("—".to_owned(), "—".to_owned(), "—".to_owned())
                };
                MarkStudentDto {
                    id: e.student.id.clone(),
                    adm: e.student.adm_no.clone(),
                    roll: e.roll,
                    name: e.student.name.clone(),
                    marks,
                    total,
                    grade,
                    percent: pct,
                }
            })
            .collect();

        Ok(MarksSheetDto {
            exam: ExamHeadDto {
                id: exam.id,
                name: exam.name,
                max: exam.max_marks,
            },
            exams: all_exams
                .into_iter()
                .map(|e| NamedDto {
                    id: e.id,
                    name: e.name,
                })
                .collect(),
            section_id: section_id.to_owned(),
            subjects: subjects
                .into_iter()
                .map(|s| NamedDto {
                    id: s.id,
                    name: s.name,
                })
                .collect(),
            students: students_dto,
            saved_by,
            saved_at,
            editable: permissions::authorize_section(actor, Action::MarksEnter, section_id).is_ok(),
        })
    }

    pub fn save(&self, actor: &Actor, input: SaveMarksInput) -> Result<MarksSheetDto, ServiceError> {
        permissions::authorize_section(actor, Action::MarksEnter, &input.section_id)?;
        let session = self.session()?;
        let (exam, _) = self.pick_exam(&session, &input.exam_id)?;
        let class_id = self.class_of(&input.section_id)?;
        let subjects: Vec<SubjectRow> = self.services.db.read(|c| subjects::list_active(c, &class_id))?;
        let students = self.section_students(&session, &input.section_id)?;

        let student_names: HashMap<&str, &str> = students
            .iter()
            .map(|e| (e.student.id.as_str(), e.student.name.as_str()))
            .collect();
        let subject_names: HashMap<&str, &str> = subjects
            .iter()
            .map(|s| (s.id.as_str(), s.name.as_str()))
            .collect();
        let max = exam.max_marks as u16;

        // Validate cells; collect *every* error before touching the database.
        let mut labels: Vec<String> = Vec::new();
        let mut bad_cells: Vec<Value> = Vec::new();
        let mut parsed: Vec<(String, String, MarkValue)> = Vec::with_capacity(input.entries.len());
        for entry in &input.entries {
            if !student_names.contains_key(entry.student_id.as_str())
                || !subject_names.contains_key(entry.subject_id.as_str())
            {
                return Err(ServiceError::validation("marks.error.unknown_cell").field("entries"));
            }
            match parse_mark(&entry_string(&entry.value), max) {
                Ok(value) => parsed.push((entry.student_id.clone(), entry.subject_id.clone(), value)),
                Err(_) => {
                    labels.push(format!(
                        "{} – {}",
                        student_names[entry.student_id.as_str()],
                        subject_names[entry.subject_id.as_str()]
                    ));
                    bad_cells.push(json!({ "studentId": entry.student_id, "subjectId": entry.subject_id }));
                }
            }
        }
        if !bad_cells.is_empty() {
            let mut list = labels.iter().take(10).cloned().collect::<Vec<_>>().join(", ");
            if labels.len() > 10 {
                list.push('…');
            }
            return Err(ServiceError::validation("marks.error.cells")
                .field("entries")
                .param("max", max.to_string())
                .param("list", list)
                .param("cells", Value::Array(bad_cells).to_string()));
        }

        let now = self.services.clock.now_utc().to_rfc3339();
        let hlc = self.services.next_hlc().to_text();
        let exam_id = exam.id.clone();
        let section_label = self
            .services
            .db
            .read(|c| classes::section_label(c, &input.section_id))?
            .unwrap_or_default();
        let mut filled = 0_i64;
        let mut payload_entries: Vec<Value> = Vec::with_capacity(parsed.len());

        self.services.db.write(|tx| {
            for (student_id, subject_id, value) in &parsed {
                match value {
                    MarkValue::Blank => {
                        marks_repo::delete(tx, &exam_id, student_id, subject_id)?;
                        payload_entries.push(json!({ "studentId": student_id, "subjectId": subject_id, "value": Value::Null }));
                    }
                    MarkValue::Absent => {
                        marks_repo::upsert(
                            tx,
                            &MarkRow {
                                id: self.services.ids.new_id(),
                                exam_id: exam_id.clone(),
                                student_id: student_id.clone(),
                                subject_id: subject_id.clone(),
                                value: None,
                                absent: true,
                                entered_by: actor.user_id.clone(),
                                entered_at: now.clone(),
                            },
                            &now,
                            &hlc,
                        )?;
                        filled += 1;
                        payload_entries.push(json!({ "studentId": student_id, "subjectId": subject_id, "value": "AB" }));
                    }
                    MarkValue::Score(score) => {
                        marks_repo::upsert(
                            tx,
                            &MarkRow {
                                id: self.services.ids.new_id(),
                                exam_id: exam_id.clone(),
                                student_id: student_id.clone(),
                                subject_id: subject_id.clone(),
                                value: Some(i64::from(*score)),
                                absent: false,
                                entered_by: actor.user_id.clone(),
                                entered_at: now.clone(),
                            },
                            &now,
                            &hlc,
                        )?;
                        filled += 1;
                        payload_entries.push(json!({ "studentId": student_id, "subjectId": subject_id, "value": score }));
                    }
                }
            }
            write_entry(
                self.services,
                tx,
                Some(actor),
                ChangeRecord {
                    kind: "mark",
                    entity: "exam",
                    entity_id: &exam_id,
                    op: Op::ReplaceSet,
                    summary_key: "marks.log.saved",
                    params: json!({ "exam": exam.name, "section": section_label, "count": filled }),
                    payload: json!({ "examId": exam_id, "sectionId": input.section_id, "entries": payload_entries }),
                },
            )?;
            Ok(())
        })?;
        self.sheet(actor, &exam.id, &input.section_id)
    }

    pub fn report_card(&self, actor: &Actor, student_id: &str) -> Result<ReportCardDto, ServiceError> {
        let session = self.session()?;
        let session_id = session.id.clone();
        let enrolled = self
            .services
            .db
            .read(|c| students::enrolled_by_id(c, &session_id, student_id))?
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "students.error.not_found"))?;
        permissions::authorize_section(actor, Action::ReportcardView, &enrolled.section_id)?;
        self.build_report_card(&session, &enrolled)
    }

    pub fn class_report_cards(
        &self,
        actor: &Actor,
        section_id: &str,
    ) -> Result<Vec<ReportCardDto>, ServiceError> {
        permissions::authorize_section(actor, Action::ReportcardPrint, section_id)?;
        let session = self.session()?;
        let students = self.section_students(&session, section_id)?;
        students
            .iter()
            .map(|e| self.build_report_card(&session, e))
            .collect()
    }

    fn build_report_card(
        &self,
        session: &SessionRow,
        enrolled: &EnrolledStudent,
    ) -> Result<ReportCardDto, ServiceError> {
        let session_id = session.id.clone();
        let class_id = enrolled.class_id.clone();
        let exams = self.services.db.read(|c| exams::list_active(c, &session_id))?;
        let subjects = self.services.db.read(|c| subjects::list_active(c, &class_id))?;
        let scale = scale_of(self.services.db.read(grade_scale::list)?);
        let school = self.services.db.read(school::get)?;
        let student_id = enrolled.student.id.clone();

        // Per exam: (subject_id -> MarkRow) for this student.
        let mut per_exam: Vec<HashMap<String, MarkRow>> = Vec::with_capacity(exams.len());
        for exam in &exams {
            let exam_id = exam.id.clone();
            let sid = student_id.clone();
            let rows = self
                .services
                .db
                .read(|c| marks_repo::list_for_exam(c, &exam_id))?;
            let map = rows
                .into_iter()
                .filter(|r| r.student_id == sid)
                .map(|r| (r.subject_id.clone(), r))
                .collect();
            per_exam.push(map);
        }

        let rows = subjects
            .iter()
            .map(|sub| ReportRowDto {
                subject: sub.name.clone(),
                by_exam: per_exam
                    .iter()
                    .map(|map| map.get(&sub.id).map_or_else(|| "—".to_owned(), mark_string))
                    .collect(),
            })
            .collect();

        let totals = exams
            .iter()
            .zip(&per_exam)
            .map(|(exam, map)| {
                let values: Vec<MarkValue> = subjects
                    .iter()
                    .map(|sub| match map.get(&sub.id) {
                        Some(row) if row.absent => MarkValue::Absent,
                        Some(row) => MarkValue::Score(row.value.unwrap_or_default().max(0) as u16),
                        None => MarkValue::Blank,
                    })
                    .collect();
                let r = exam_result(&values, exam.max_marks as u16);
                ReportTotalDto {
                    entered: r.entered,
                    got: r.got,
                    max: r.max,
                    pct: r.percent_tenths.map(pct_text),
                    grade: r.percent_tenths.map(|t| grade_for(t, &scale)).unwrap_or_default(),
                }
            })
            .collect();

        let (present, total) = self
            .services
            .db
            .read(|c| attendance::student_session_counts(c, &session_id, &student_id))?;
        let pct = percent(present.max(0) as u32, total.max(0) as u32).map(i64::from);

        Ok(ReportCardDto {
            school: SchoolLite {
                name: school.as_ref().map(|s| s.name.clone()).unwrap_or_default(),
                session: session.name.clone(),
            },
            student: ReportStudentDto {
                name: enrolled.student.name.clone(),
                ck: format!("{}-{}", enrolled.class_name, enrolled.section_name),
                roll: enrolled.roll,
                adm: enrolled.student.adm_no.clone(),
                father: enrolled.student.father.clone(),
                dob: enrolled.student.dob.clone(),
                section_id: enrolled.section_id.clone(),
            },
            exams: exams
                .into_iter()
                .map(|e| ExamHeadDto {
                    id: e.id,
                    name: e.name,
                    max: e.max_marks,
                })
                .collect(),
            rows,
            totals,
            attendance: AttendanceSummaryDto {
                p: present,
                t: total,
                pct,
            },
        })
    }
}
