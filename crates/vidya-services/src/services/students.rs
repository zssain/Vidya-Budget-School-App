//! Students and admissions. Role-shaped DTOs make it impossible for a teacher
//! response to carry fee or category data. Every write authorizes, validates,
//! runs one transaction and appends a change-log entry.

use std::collections::HashMap;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use vidya_core::dates::parse_date;
use vidya_core::error::ErrorKind;
use vidya_core::fees::{self, FeeInputs, FeePlan, FeeState};
use vidya_core::permissions::{self, Action, Scope};
use vidya_core::roles::{Actor, Role};
use vidya_core::validation::{validate_mobile, validate_person_name, validate_reason};
use vidya_db::repo::sessions::SessionRow;
use vidya_db::repo::students::{EnrolledStudent, StudentRow};
use vidya_db::repo::{classes, enrollments, fee_plans, meta, receipts, sessions, students};
use vidya_db::DbError;

use crate::change_log::{write_entry, ChangeRecord, Op};
use crate::error::ServiceError;
use crate::{Mode, Services};

// ---------- DTOs ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentTeacherDto {
    pub id: String,
    pub adm_no: String,
    pub name: String,
    pub gender: String,
    pub dob: Option<String>,
    pub father: String,
    pub mother: String,
    pub mobile: String,
    pub locality: String,
    pub class_name: String,
    pub section_name: String,
    pub section_id: String,
    pub roll: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeSummaryDto {
    pub due: i64,
    pub paid: i64,
    pub balance: i64,
    pub state: FeeState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentOfficeDto {
    // Everything a teacher sees, plus office-only fields.
    pub id: String,
    pub adm_no: String,
    pub name: String,
    pub gender: String,
    pub dob: Option<String>,
    pub father: String,
    pub mother: String,
    pub mobile: String,
    pub locality: String,
    pub class_name: String,
    pub section_name: String,
    pub section_id: String,
    pub roll: i64,
    pub status: String,
    pub category: String,
    pub rte: bool,
    pub transport: bool,
    pub aadhaar_collected: bool,
    pub apaar_created: bool,
    pub admitted_on: String,
    pub concession: i64,
    pub fee: FeeSummaryDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptRowDto {
    pub id: String,
    pub receipt_no: String,
    pub paid_on: String,
    pub amount: i64,
    pub mode: String,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentOfficeDetailDto {
    #[serde(flatten)]
    pub office: StudentOfficeDto,
    pub receipts: Vec<ReceiptRowDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "shape", rename_all = "camelCase")]
pub enum StudentListItemDto {
    Teacher(StudentTeacherDto),
    Office(StudentOfficeDto),
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "shape", rename_all = "camelCase")]
pub enum StudentDetailDto {
    Teacher(Box<StudentTeacherDto>),
    Office(Box<StudentOfficeDetailDto>),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentListDto {
    pub items: Vec<StudentListItemDto>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentFilter {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub section_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentInput {
    pub name: String,
    pub gender: String,
    #[serde(default)]
    pub dob: String,
    pub father: String,
    #[serde(default)]
    pub mother: String,
    pub mobile: String,
    pub category: String,
    #[serde(default)]
    pub locality: String,
    pub section_id: String,
    #[serde(default)]
    pub rte: bool,
    #[serde(default)]
    pub transport: bool,
    #[serde(default)]
    pub concession: i64,
    #[serde(default)]
    pub aadhaar_collected: bool,
    #[serde(default)]
    pub apaar_created: bool,
    #[serde(default)]
    pub confirm_duplicate: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStudentInput {
    pub student_id: String,
    pub name: String,
    pub gender: String,
    #[serde(default)]
    pub dob: String,
    pub father: String,
    #[serde(default)]
    pub mother: String,
    pub mobile: String,
    pub category: String,
    #[serde(default)]
    pub locality: String,
    pub section_id: String,
    #[serde(default)]
    pub rte: bool,
    #[serde(default)]
    pub transport: bool,
    #[serde(default)]
    pub concession: i64,
    #[serde(default)]
    pub aadhaar_collected: bool,
    #[serde(default)]
    pub apaar_created: bool,
}

pub struct StudentService<'a> {
    services: &'a Services,
}

fn student_event<'a>(
    student_id: &'a str,
    summary_key: &'a str,
    params: serde_json::Value,
) -> ChangeRecord<'a> {
    ChangeRecord {
        kind: "stu",
        entity: "student",
        entity_id: student_id,
        op: Op::Insert,
        summary_key,
        params,
        payload: serde_json::json!({}),
    }
}

fn teacher_dto(e: &EnrolledStudent) -> StudentTeacherDto {
    StudentTeacherDto {
        id: e.student.id.clone(),
        adm_no: e.student.adm_no.clone(),
        name: e.student.name.clone(),
        gender: e.student.gender.clone(),
        dob: e.student.dob.clone(),
        father: e.student.father.clone(),
        mother: e.student.mother.clone(),
        mobile: e.student.mobile.clone(),
        locality: e.student.locality.clone(),
        class_name: e.class_name.clone(),
        section_name: e.section_name.clone(),
        section_id: e.section_id.clone(),
        roll: e.roll,
        status: e.status.clone(),
    }
}

fn fee_summary(
    conn: &Connection,
    session: &SessionRow,
    plan: Option<&fee_plans::FeePlanRow>,
    e: &EnrolledStudent,
) -> Result<FeeSummaryDto, DbError> {
    let plan = plan.map_or(
        FeePlan {
            tuition: 0,
            exam: 0,
            other: 0,
        },
        |p| FeePlan {
            tuition: p.tuition,
            exam: p.exam,
            other: p.other,
        },
    );
    let inputs = FeeInputs {
        plan,
        terms: session.terms as u32,
        transport_fee_per_term: session.transport_fee_per_term,
        transport: e.transport,
        rte: e.rte,
        concession: e.concession,
    };
    let due = fees::session_due(&inputs);
    let paid = receipts::sum_paid(conn, &session.id, &e.student.id)?;
    Ok(FeeSummaryDto {
        due,
        paid,
        balance: fees::balance(due, paid),
        state: fees::fee_state(due, paid, e.rte),
    })
}

fn office_dto(
    conn: &Connection,
    session: &SessionRow,
    plans: &HashMap<String, fee_plans::FeePlanRow>,
    e: &EnrolledStudent,
) -> Result<StudentOfficeDto, DbError> {
    let t = teacher_dto(e);
    Ok(StudentOfficeDto {
        id: t.id,
        adm_no: t.adm_no,
        name: t.name,
        gender: t.gender,
        dob: t.dob,
        father: t.father,
        mother: t.mother,
        mobile: t.mobile,
        locality: t.locality,
        class_name: t.class_name,
        section_name: t.section_name,
        section_id: t.section_id,
        roll: t.roll,
        status: t.status,
        category: e.student.category.clone(),
        rte: e.rte,
        transport: e.transport,
        aadhaar_collected: e.student.aadhaar_collected,
        apaar_created: e.student.apaar_created,
        admitted_on: e.student.admitted_on.clone(),
        concession: e.concession,
        fee: fee_summary(conn, session, plans.get(&e.class_id), e)?,
    })
}

impl<'a> StudentService<'a> {
    pub fn new(services: &'a Services) -> Self {
        Self { services }
    }

    fn is_office(actor: &Actor) -> bool {
        actor.role != Role::Teacher
    }

    fn session(&self) -> Result<SessionRow, ServiceError> {
        self.services
            .db
            .read(sessions::current)?
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "session.error.none"))
    }

    fn plan_map(
        conn: &Connection,
        session_id: &str,
    ) -> Result<HashMap<String, fee_plans::FeePlanRow>, DbError> {
        Ok(fee_plans::list_for_session(conn, session_id)?
            .into_iter()
            .map(|p| (p.class_id.clone(), p))
            .collect())
    }

    pub fn list(&self, actor: &Actor, filter: StudentFilter) -> Result<StudentListDto, ServiceError> {
        let scope = permissions::scope(actor, Action::StudentsView)?;
        if let (Some(section), Scope::Sections(allowed)) = (&filter.section_id, &scope) {
            if !allowed.contains(section) {
                return Err(ServiceError::new(ErrorKind::Permission, "permission.denied"));
            }
        }
        let status = match filter.status.as_deref() {
            Some("all") => "",
            Some(s) => s,
            None => "active",
        };
        let session = self.session()?;
        let session_id = session.id.clone();
        let all = self
            .services
            .db
            .read(|c| students::list_enrolled(c, &session_id, status))?;

        let q = filter
            .q
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase);
        let mut rows: Vec<EnrolledStudent> = all
            .into_iter()
            .filter(|e| match &scope {
                Scope::All => true,
                Scope::Sections(secs) => secs.contains(&e.section_id),
            })
            .filter(|e| filter.section_id.as_ref().is_none_or(|s| &e.section_id == s))
            .filter(|e| match &q {
                None => true,
                Some(q) => {
                    e.student.name.to_lowercase().contains(q)
                        || e.student.adm_no.to_lowercase().contains(q)
                        || e.student.father.to_lowercase().contains(q)
                        || e.student.mobile.contains(q)
                        || e.roll.to_string() == *q
                }
            })
            .collect();

        let truncated = rows.len() > 500;
        rows.truncate(500);

        let items = if Self::is_office(actor) {
            self.services.db.read(|c| {
                let plans = Self::plan_map(c, &session_id)?;
                rows.iter()
                    .map(|e| office_dto(c, &session, &plans, e).map(StudentListItemDto::Office))
                    .collect::<Result<Vec<_>, DbError>>()
            })?
        } else {
            rows.iter()
                .map(|e| StudentListItemDto::Teacher(teacher_dto(e)))
                .collect()
        };
        Ok(StudentListDto { items, truncated })
    }

    fn ensure_in_scope(actor: &Actor, section_id: &str) -> Result<(), ServiceError> {
        permissions::authorize_section(actor, Action::StudentsView, section_id).map_err(Into::into)
    }

    pub fn get(&self, actor: &Actor, student_id: &str) -> Result<StudentDetailDto, ServiceError> {
        // Access is decided by the student's section (`ensure_in_scope` below).
        let session = self.session()?;
        let session_id = session.id.clone();
        let enrolled = self
            .services
            .db
            .read(|c| students::enrolled_by_id(c, &session_id, student_id))?
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "students.error.not_found"))?;
        Self::ensure_in_scope(actor, &enrolled.section_id)?;

        if Self::is_office(actor) {
            let detail = self.services.db.read(|c| {
                let plans = Self::plan_map(c, &session_id)?;
                let office = office_dto(c, &session, &plans, &enrolled)?;
                let receipts = receipts::list_for_student(c, &session_id, student_id)?
                    .into_iter()
                    .map(|r| ReceiptRowDto {
                        id: r.id,
                        receipt_no: r.receipt_no,
                        paid_on: r.paid_on,
                        amount: r.amount,
                        mode: r.mode,
                        cancelled: r.cancelled,
                    })
                    .collect();
                Ok(StudentOfficeDetailDto { office, receipts })
            })?;
            Ok(StudentDetailDto::Office(Box::new(detail)))
        } else {
            Ok(StudentDetailDto::Teacher(Box::new(teacher_dto(&enrolled))))
        }
    }

    pub fn add(&self, actor: &Actor, input: StudentInput) -> Result<StudentDetailDto, ServiceError> {
        permissions::authorize(actor, Action::StudentsAdd)?;
        if self.services.mode == Mode::Client {
            return Err(ServiceError::new(
                ErrorKind::Offline,
                "students.error.online_only",
            ));
        }
        let name = validate_person_name(&input.name, "name")?;
        let father = validate_person_name(&input.father, "father")?;
        let mother = if input.mother.trim().is_empty() {
            String::new()
        } else {
            validate_person_name(&input.mother, "mother")?
        };
        let mobile = validate_mobile(&input.mobile)?;
        let gender = validate_gender(&input.gender)?;
        let category = validate_category(&input.category)?;
        let dob = self.validate_dob(&input.dob)?;
        if input.concession != 0 {
            permissions::authorize(actor, Action::StudentsSetConcession)?;
        }

        let session = self.session()?;
        let class_id = self
            .services
            .db
            .read(|c| classes::active_section_class(c, &input.section_id))?
            .ok_or_else(|| ServiceError::validation("students.error.section").field("sectionId"))?;

        if !input.confirm_duplicate {
            if let Some(dup) = self
                .services
                .db
                .read(|c| students::find_active_duplicate(c, &session.id, &name, &father))?
            {
                return Err(
                    ServiceError::new(ErrorKind::Conflict, "students.possible_duplicate")
                        .param("name", dup.student.name.clone())
                        .param("class", format!("{}-{}", dup.class_name, dup.section_name))
                        .param("admNo", dup.student.adm_no),
                );
            }
        }

        let id = self.services.ids.new_id();
        let now = self.services.clock.now_utc().to_rfc3339();
        let today = self.services.clock.today_local().to_string();
        let hlc = self.services.next_hlc().to_text();
        let session_id = session.id.clone();
        self.services.db.write(|tx| {
            let adm = meta::next_counter(tx, "adm_no")?;
            let adm_no = format!("ADM/{adm:04}");
            students::insert(
                tx,
                &StudentRow {
                    id: id.clone(),
                    adm_no,
                    name: name.clone(),
                    gender: gender.clone(),
                    dob: dob.clone(),
                    father: father.clone(),
                    mother: mother.clone(),
                    mobile: mobile.clone(),
                    category: category.clone(),
                    locality: input.locality.clone(),
                    aadhaar_collected: input.aadhaar_collected,
                    apaar_created: input.apaar_created,
                    admitted_on: today.clone(),
                },
                &now,
                &hlc,
            )?;
            let roll = enrollments::max_roll(tx, &session_id, &input.section_id)? + 1;
            enrollments::insert(
                tx,
                &enrollments::EnrollmentRow {
                    id: self.services.ids.new_id(),
                    student_id: id.clone(),
                    session_id: session_id.clone(),
                    class_id: class_id.clone(),
                    section_id: input.section_id.clone(),
                    roll,
                    rte: input.rte,
                    transport: input.transport,
                    concession: input.concession,
                    status: "active".to_owned(),
                },
                &hlc,
            )?;
            write_entry(
                self.services,
                tx,
                Some(actor),
                student_event(&id, "students.log.admitted", serde_json::json!({ "name": name })),
            )?;
            Ok(())
        })?;
        self.get(actor, &id)
    }

    pub fn update(&self, actor: &Actor, input: UpdateStudentInput) -> Result<StudentDetailDto, ServiceError> {
        permissions::authorize(actor, Action::StudentsEdit)?;
        let session = self.session()?;
        let session_id = session.id.clone();
        let enrolled = self
            .services
            .db
            .read(|c| students::enrolled_by_id(c, &session_id, &input.student_id))?
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "students.error.not_found"))?;
        Self::ensure_in_scope(actor, &enrolled.section_id)?;
        if input.concession != enrolled.concession {
            permissions::authorize(actor, Action::StudentsSetConcession)?;
        }
        let name = validate_person_name(&input.name, "name")?;
        let father = validate_person_name(&input.father, "father")?;
        let mother = if input.mother.trim().is_empty() {
            String::new()
        } else {
            validate_person_name(&input.mother, "mother")?
        };
        let mobile = validate_mobile(&input.mobile)?;
        let gender = validate_gender(&input.gender)?;
        let category = validate_category(&input.category)?;
        let dob = self.validate_dob(&input.dob)?;
        let new_class = self
            .services
            .db
            .read(|c| classes::active_section_class(c, &input.section_id))?
            .ok_or_else(|| ServiceError::validation("students.error.section").field("sectionId"))?;

        let now = self.services.clock.now_utc().to_rfc3339();
        let hlc = self.services.next_hlc().to_text();
        let moved = input.section_id != enrolled.section_id;
        let student_id = input.student_id.clone();
        self.services.db.write(|tx| {
            students::update(
                tx,
                &StudentRow {
                    id: student_id.clone(),
                    adm_no: enrolled.student.adm_no.clone(),
                    name: name.clone(),
                    gender,
                    dob,
                    father,
                    mother,
                    mobile,
                    category,
                    locality: input.locality.clone(),
                    aadhaar_collected: input.aadhaar_collected,
                    apaar_created: input.apaar_created,
                    admitted_on: enrolled.student.admitted_on.clone(),
                },
                &now,
                &hlc,
            )?;
            let roll = if moved {
                enrollments::max_roll(tx, &session_id, &input.section_id)? + 1
            } else {
                enrolled.roll
            };
            enrollments::update(
                tx,
                &enrolled.enrollment_id,
                &new_class,
                &input.section_id,
                roll,
                input.rte,
                input.transport,
                input.concession,
                &hlc,
            )?;
            write_entry(
                self.services,
                tx,
                Some(actor),
                student_event(
                    &student_id,
                    "students.log.updated",
                    serde_json::json!({ "name": name }),
                ),
            )?;
            Ok(())
        })?;
        self.get(actor, &input.student_id)
    }

    pub fn mark_left(
        &self,
        actor: &Actor,
        student_id: &str,
        left_on: &str,
        reason: &str,
    ) -> Result<StudentDetailDto, ServiceError> {
        permissions::authorize(actor, Action::StudentsMarkLeft)?;
        let reason = validate_reason(reason, 2)?;
        let left = parse_date(left_on)?;
        if left > self.services.clock.today_local() {
            return Err(ServiceError::validation("students.error.left_future").field("leftOn"));
        }
        let session = self.session()?;
        let session_id = session.id.clone();
        let enrolled = self
            .services
            .db
            .read(|c| students::enrolled_by_id(c, &session_id, student_id))?
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "students.error.not_found"))?;
        Self::ensure_in_scope(actor, &enrolled.section_id)?;
        let hlc = self.services.next_hlc().to_text();
        let sid = student_id.to_owned();
        self.services.db.write(|tx| {
            enrollments::set_status(tx, &enrolled.enrollment_id, "left", Some(left_on), &reason, &hlc)?;
            write_entry(
                self.services,
                tx,
                Some(actor),
                student_event(&sid, "students.log.left", serde_json::json!({})),
            )?;
            Ok(())
        })?;
        self.get(actor, student_id)
    }

    fn validate_dob(&self, dob: &str) -> Result<Option<String>, ServiceError> {
        if dob.trim().is_empty() {
            return Ok(None);
        }
        let parsed = parse_date(dob)?;
        if parsed > self.services.clock.today_local() {
            return Err(ServiceError::validation("students.error.dob_future").field("dob"));
        }
        Ok(Some(dob.to_owned()))
    }
}

fn validate_gender(gender: &str) -> Result<String, ServiceError> {
    match gender {
        "Male" | "Female" | "Other" => Ok(gender.to_owned()),
        _ => Err(ServiceError::validation("students.error.gender").field("gender")),
    }
}

fn validate_category(category: &str) -> Result<String, ServiceError> {
    match category {
        "General" | "OBC" | "SC" | "ST" => Ok(category.to_owned()),
        _ => Err(ServiceError::validation("students.error.category").field("category")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teacher_dto_has_no_fee_or_category_keys() {
        let dto = StudentTeacherDto {
            id: "s1".into(),
            adm_no: "ADM/0001".into(),
            name: "Aman".into(),
            gender: "Male".into(),
            dob: None,
            father: "Raj".into(),
            mother: String::new(),
            mobile: "9876543210".into(),
            locality: String::new(),
            class_name: "V".into(),
            section_name: "A".into(),
            section_id: "sec".into(),
            roll: 1,
            status: "active".into(),
        };
        let json = serde_json::to_string(&dto).unwrap();
        for forbidden in ["rte", "concession", "\"due\"", "paid", "balance", "category"] {
            assert!(
                !json.contains(forbidden),
                "teacher DTO leaked {forbidden}: {json}"
            );
        }
    }
}
