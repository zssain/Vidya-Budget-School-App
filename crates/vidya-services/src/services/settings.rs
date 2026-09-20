//! Read-only settings snapshot (school, session, classes, fees, subjects,
//! exams, grade scale, app settings). Write methods arrive in P3.6.

use vidya_core::permissions::{self, Action};
use vidya_core::roles::Actor;
use vidya_db::repo::{classes, exams, fee_plans, grade_scale, sessions, subjects};
use vidya_db::{repo, DbError};

use crate::error::ServiceError;
use crate::services::dto::{
    ClassDto, ExamDto, FeePlanDto, GradeBandDto, SchoolHeaderDto, SectionDto, SessionDto, SettingsDto,
    SubjectDto,
};
use crate::Services;

pub struct SettingsService<'a> {
    pub services: &'a Services,
}

/// Raw rows read in one transaction, assembled into DTOs afterwards.
struct SettingsBundle {
    school: Option<repo::school::SchoolRow>,
    session: Option<sessions::SessionRow>,
    fee_plans: Vec<fee_plans::FeePlanRow>,
    exams: Vec<exams::ExamRow>,
    grade_scale: Vec<grade_scale::GradeBandRow>,
    app_settings: std::collections::BTreeMap<String, String>,
    classes: Vec<(
        classes::ClassRow,
        Vec<classes::SectionRow>,
        Vec<subjects::SubjectRow>,
    )>,
}

impl SettingsService<'_> {
    pub fn get(&self, actor: &Actor) -> Result<SettingsDto, ServiceError> {
        permissions::authorize(actor, Action::SettingsView)?;

        let bundle = self.services.db.read(|conn| {
            let school = repo::school::get(conn)?;
            let session = sessions::current(conn)?;
            let session_id = session.as_ref().map(|s| s.id.clone());
            let fee_plans = match &session_id {
                Some(id) => fee_plans::list_for_session(conn, id)?,
                None => Vec::new(),
            };
            let exams = match &session_id {
                Some(id) => exams::list_active(conn, id)?,
                None => Vec::new(),
            };
            let grade_scale = grade_scale::list(conn)?;
            let app_settings = repo::app_settings::all(conn)?;
            let mut class_rows = Vec::new();
            for class in classes::list_active_classes(conn)? {
                let sections = classes::list_active_sections(conn, &class.id)?;
                let subjects = subjects::list_active(conn, &class.id)?;
                class_rows.push((class, sections, subjects));
            }
            Ok::<SettingsBundle, DbError>(SettingsBundle {
                school,
                session,
                fee_plans,
                exams,
                grade_scale,
                app_settings,
                classes: class_rows,
            })
        })?;

        let school = bundle
            .school
            .ok_or_else(|| ServiceError::internal("settings requested before setup"))?;
        let session_name = bundle.session.as_ref().map(|s| s.name.clone());

        let classes = bundle
            .classes
            .into_iter()
            .map(|(class, sections, subjects)| ClassDto {
                fee_plan: bundle
                    .fee_plans
                    .iter()
                    .find(|f| f.class_id == class.id)
                    .map(|f| FeePlanDto {
                        tuition: f.tuition,
                        exam: f.exam,
                        other: f.other,
                    }),
                id: class.id,
                name: class.name,
                sort_order: class.sort_order,
                sections: sections
                    .into_iter()
                    .map(|s| SectionDto {
                        id: s.id,
                        name: s.name,
                    })
                    .collect(),
                subjects: subjects
                    .into_iter()
                    .map(|s| SubjectDto {
                        id: s.id,
                        name: s.name,
                    })
                    .collect(),
            })
            .collect();

        Ok(SettingsDto {
            school: SchoolHeaderDto {
                name: school.name,
                address: school.address,
                udise: school.udise,
                board: school.board,
                phone: school.phone,
                session_name,
            },
            session: bundle.session.map(|s| SessionDto {
                id: s.id,
                name: s.name,
                starts_on: s.starts_on,
                ends_on: s.ends_on,
                terms: s.terms,
                transport_fee_per_term: s.transport_fee_per_term,
            }),
            classes,
            exams: bundle
                .exams
                .into_iter()
                .map(|e| ExamDto {
                    id: e.id,
                    name: e.name,
                    max_marks: e.max_marks,
                })
                .collect(),
            grade_scale: bundle
                .grade_scale
                .into_iter()
                .map(|g| GradeBandDto {
                    grade: g.grade,
                    min_percent: g.min_percent,
                })
                .collect(),
            app_settings: bundle.app_settings,
        })
    }
}
