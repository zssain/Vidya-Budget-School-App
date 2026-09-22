//! Read-only settings snapshot (school, session, classes, fees, subjects,
//! exams, grade scale, app settings). Write methods arrive in P3.6.

use serde::Deserialize;
use serde_json::json;

use vidya_core::error::ErrorKind;
use vidya_core::marks::{validate_grade_scale, GradeBand};
use vidya_core::money::parse_rupees;
use vidya_core::permissions::{self, Action};
use vidya_core::roles::Actor;
use vidya_core::validation::{validate_class_name, validate_receipt_prefix, validate_udise};
use vidya_db::repo::classes::{ClassRow, SectionRow};
use vidya_db::repo::exams::ExamRow;
use vidya_db::repo::fee_plans::FeePlanRow;
use vidya_db::repo::school::SchoolRow;
use vidya_db::repo::subjects::SubjectRow;
use vidya_db::repo::{classes, exams, fee_plans, grade_scale, marks, meta, receipts, school, sessions, subjects};
use vidya_db::{repo, DbError};

use crate::change_log::{write_entry, ChangeRecord, Op};
use crate::error::ServiceError;
use crate::services::dto::{
    ClassDto, ExamDto, FeePlanDto, GradeBandDto, SchoolHeaderDto, SectionDto, SessionDto, SettingsDto,
    SubjectDto,
};
use crate::Services;

const BOARDS: [&str; 5] = ["State Board", "CBSE", "ICSE", "U.P. Board", "Other"];

// ---------- write inputs ----------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchoolInput {
    pub name: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub udise: String,
    #[serde(default)]
    pub board: String,
    #[serde(default)]
    pub phone: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassInput {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    pub sections: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassesInput {
    pub classes: Vec<ClassInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeePlanEntry {
    pub class_id: String,
    #[serde(default)]
    pub tuition: String,
    #[serde(default)]
    pub exam: String,
    #[serde(default)]
    pub other: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeePlanInput {
    pub plans: Vec<FeePlanEntry>,
    pub terms: i64,
    #[serde(default)]
    pub transport_fee_per_term: String,
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubjectsInput {
    pub class_id: String,
    pub subjects: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamInput {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    pub max: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamsInput {
    pub exams: Vec<ExamInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradeBandInput {
    pub grade: String,
    pub min_percent: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradeScaleInput {
    pub bands: Vec<GradeBandInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsInput {
    pub session_timeout_minutes: i64,
    #[serde(default)]
    pub receipt_paper: String,
    #[serde(default)]
    pub print_language: String,
    #[serde(default)]
    pub tray: bool,
    #[serde(default)]
    pub start_at_login: bool,
    #[serde(default)]
    pub keep_awake: bool,
    #[serde(default)]
    pub school_start: String,
    #[serde(default)]
    pub school_end: String,
}

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

    fn stamp(&self) -> (String, String) {
        (
            self.services.clock.now_utc().to_rfc3339(),
            self.services.next_hlc().to_text(),
        )
    }

    fn current_session(&self) -> Result<sessions::SessionRow, ServiceError> {
        self.services
            .db
            .read(sessions::current)?
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "session.error.none"))
    }

    fn settings_log(
        &self,
        tx: &rusqlite::Transaction<'_>,
        actor: &Actor,
        area: &str,
        summary_key: &str,
    ) -> Result<(), DbError> {
        write_entry(
            self.services,
            tx,
            Some(actor),
            ChangeRecord {
                kind: "settings",
                entity: "settings",
                entity_id: area,
                op: Op::UpdateFields,
                summary_key,
                params: json!({}),
                payload: json!({}),
            },
        )?;
        Ok(())
    }

    fn ensure_section_removable(&self, section: &SectionRow) -> Result<(), ServiceError> {
        let count = self
            .services
            .db
            .read(|c| classes::active_enrollments_in_section(c, &section.id))?;
        if count > 0 {
            return Err(ServiceError::new(ErrorKind::Conflict, "settings.error.section_has_students")
                .param("count", count.to_string()));
        }
        let teachers = self
            .services
            .db
            .read(|c| classes::teacher_names_for_section(c, &section.id))?;
        if !teachers.is_empty() {
            return Err(ServiceError::new(ErrorKind::Conflict, "settings.error.section_has_teacher")
                .param("names", teachers.join(", ")));
        }
        Ok(())
    }

    pub fn save_school(&self, actor: &Actor, input: SchoolInput) -> Result<SettingsDto, ServiceError> {
        permissions::authorize(actor, Action::SettingsEdit)?;
        let name = input.name.trim().to_owned();
        if name.is_empty() {
            return Err(ServiceError::validation("settings.error.school_name").field("name"));
        }
        let udise = validate_udise(&input.udise)?;
        let board = if input.board.trim().is_empty() {
            "State Board".to_owned()
        } else if BOARDS.contains(&input.board.as_str()) {
            input.board.clone()
        } else {
            return Err(ServiceError::validation("settings.error.board").field("board"));
        };
        let (now, hlc) = self.stamp();
        self.services.db.write(|tx| {
            school::upsert(
                tx,
                &SchoolRow {
                    name: name.clone(),
                    address: input.address.clone(),
                    udise: udise.clone(),
                    board: board.clone(),
                    phone: input.phone.clone(),
                },
                &now,
                &hlc,
            )?;
            self.settings_log(tx, actor, "school", "settings.log.school")?;
            Ok(())
        })?;
        self.get(actor)
    }

    pub fn save_classes(&self, actor: &Actor, input: ClassesInput) -> Result<SettingsDto, ServiceError> {
        permissions::authorize(actor, Action::SettingsEdit)?;
        let mut seen = std::collections::BTreeSet::new();
        for c in &input.classes {
            let name = validate_class_name(&c.name)?;
            if !seen.insert(name.to_lowercase()) {
                return Err(ServiceError::validation("settings.error.class_dup").param("name", name));
            }
            if !(1..=6).contains(&c.sections) {
                return Err(ServiceError::validation("settings.error.section_count"));
            }
        }
        let existing = self.services.db.read(classes::list_all_classes)?;
        let input_ids: std::collections::BTreeSet<&str> =
            input.classes.iter().filter_map(|c| c.id.as_deref()).collect();

        // Pre-check every section that would be removed, before writing anything.
        for cls in existing.iter().filter(|c| c.active && !input_ids.contains(c.id.as_str())) {
            for sec in self.services.db.read(|con| classes::list_active_sections(con, &cls.id))? {
                self.ensure_section_removable(&sec)?;
            }
        }
        for c in &input.classes {
            if let Some(id) = &c.id {
                let desired = section_names(c.sections);
                for sec in self.services.db.read(|con| classes::list_active_sections(con, id))? {
                    if !desired.contains(&sec.name) {
                        self.ensure_section_removable(&sec)?;
                    }
                }
            }
        }

        let (_now, hlc) = self.stamp();
        self.services.db.write(|tx| {
            for cls in existing.iter().filter(|c| c.active && !input_ids.contains(c.id.as_str())) {
                for sec in classes::list_active_sections(tx, &cls.id)? {
                    classes::set_section_active(tx, &sec.id, false, &hlc)?;
                }
                classes::set_class_active(tx, &cls.id, false, &hlc)?;
            }
            for (i, c) in input.classes.iter().enumerate() {
                let sort = i as i64;
                let name = c.name.trim();
                let class_id = match &c.id {
                    Some(id) => {
                        classes::update_class(tx, id, name, sort, &hlc)?;
                        classes::set_class_active(tx, id, true, &hlc)?;
                        id.clone()
                    }
                    None => {
                        let id = self.services.ids.new_id();
                        classes::insert_class(
                            tx,
                            &ClassRow {
                                id: id.clone(),
                                name: name.to_owned(),
                                sort_order: sort,
                                active: true,
                            },
                            &hlc,
                        )?;
                        id
                    }
                };
                let desired = section_names(c.sections);
                let all_sections = classes::list_all_sections(tx, &class_id)?;
                for sec in &all_sections {
                    if sec.active && !desired.contains(&sec.name) {
                        classes::set_section_active(tx, &sec.id, false, &hlc)?;
                    }
                }
                for name in &desired {
                    match all_sections.iter().find(|s| &s.name == name) {
                        Some(sec) if !sec.active => classes::set_section_active(tx, &sec.id, true, &hlc)?,
                        Some(_) => {}
                        None => classes::insert_section(
                            tx,
                            &SectionRow {
                                id: self.services.ids.new_id(),
                                class_id: class_id.clone(),
                                name: name.clone(),
                                active: true,
                            },
                            &hlc,
                        )?,
                    }
                }
            }
            self.settings_log(tx, actor, "classes", "settings.log.classes")?;
            Ok(())
        })?;
        self.get(actor)
    }

    pub fn save_fee_plan(&self, actor: &Actor, input: FeePlanInput) -> Result<SettingsDto, ServiceError> {
        permissions::authorize(actor, Action::SettingsEdit)?;
        if !matches!(input.terms, 1 | 2 | 3 | 4 | 12) {
            return Err(ServiceError::validation("settings.error.terms").field("terms"));
        }
        let transport = rupees(&input.transport_fee_per_term)?;
        let mut parsed = Vec::with_capacity(input.plans.len());
        for p in &input.plans {
            parsed.push((p.class_id.clone(), rupees(&p.tuition)?, rupees(&p.exam)?, rupees(&p.other)?));
        }
        let session = self.current_session()?;
        if input.terms != session.terms {
            let receipts = self.services.db.read(|c| receipts::count_for_session(c, &session.id))?;
            if receipts > 0 && !input.confirm {
                return Err(ServiceError::new(ErrorKind::Conflict, "settings.error.terms_locked"));
            }
        }
        let (_now, hlc) = self.stamp();
        let session_id = session.id.clone();
        self.services.db.write(|tx| {
            sessions::update_terms_transport(tx, &session_id, input.terms, transport, &hlc)?;
            for (class_id, tuition, exam, other) in &parsed {
                fee_plans::upsert(
                    tx,
                    &FeePlanRow {
                        session_id: session_id.clone(),
                        class_id: class_id.clone(),
                        tuition: *tuition,
                        exam: *exam,
                        other: *other,
                    },
                    &hlc,
                )?;
            }
            self.settings_log(tx, actor, "fees", "settings.log.fees")?;
            Ok(())
        })?;
        self.get(actor)
    }

    pub fn save_subjects(&self, actor: &Actor, input: SubjectsInput) -> Result<SettingsDto, ServiceError> {
        permissions::authorize(actor, Action::SettingsEdit)?;
        let mut names = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for s in &input.subjects {
            let n = s.trim();
            if n.is_empty() {
                continue;
            }
            if !seen.insert(n.to_lowercase()) {
                return Err(ServiceError::validation("settings.error.subject_dup").param("name", n.to_owned()));
            }
            names.push(n.to_owned());
        }
        if names.is_empty() {
            return Err(ServiceError::validation("settings.error.subject_min"));
        }
        let class_id = input.class_id.clone();
        let (_now, hlc) = self.stamp();
        self.services.db.write(|tx| {
            let existing = subjects::list_all(tx, &class_id)?;
            for sub in &existing {
                if sub.active && !names.iter().any(|n| n.eq_ignore_ascii_case(&sub.name)) {
                    subjects::update(tx, &sub.id, &sub.name, sub.sort_order, false, &hlc)?;
                }
            }
            for (i, name) in names.iter().enumerate() {
                match existing.iter().find(|s| s.name.eq_ignore_ascii_case(name)) {
                    Some(sub) => subjects::update(tx, &sub.id, name, i as i64, true, &hlc)?,
                    None => subjects::insert(
                        tx,
                        &SubjectRow {
                            id: self.services.ids.new_id(),
                            class_id: class_id.clone(),
                            name: name.clone(),
                            sort_order: i as i64,
                            active: true,
                        },
                        &hlc,
                    )?,
                }
            }
            self.settings_log(tx, actor, "subjects", "settings.log.subjects")?;
            Ok(())
        })?;
        self.get(actor)
    }

    pub fn save_exams(&self, actor: &Actor, input: ExamsInput) -> Result<SettingsDto, ServiceError> {
        permissions::authorize(actor, Action::SettingsEdit)?;
        if input.exams.is_empty() {
            return Err(ServiceError::validation("settings.error.exam_min"));
        }
        let mut seen = std::collections::BTreeSet::new();
        for e in &input.exams {
            let n = e.name.trim();
            if n.is_empty() {
                return Err(ServiceError::validation("settings.error.exam_name"));
            }
            if !seen.insert(n.to_lowercase()) {
                return Err(ServiceError::validation("settings.error.exam_dup").param("name", n.to_owned()));
            }
            if !(1..=500).contains(&e.max) {
                return Err(ServiceError::validation("settings.error.exam_max").param("name", n.to_owned()));
            }
        }
        let session = self.current_session()?;
        let existing = self.services.db.read(|c| exams::list_all(c, &session.id))?;
        for e in &input.exams {
            if let (Some(id), Some(prev)) = (&e.id, existing.iter().find(|x| Some(&x.id) == e.id.as_ref())) {
                if e.max < prev.max_marks {
                    let over = self.services.db.read(|c| marks::over_max(c, id, e.max))?;
                    if !over.is_empty() {
                        let list = over
                            .iter()
                            .take(10)
                            .map(|(s, sub)| format!("{s} – {sub}"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        return Err(ServiceError::validation("settings.error.exam_lower")
                            .param("name", prev.name.clone())
                            .param("list", list));
                    }
                }
            }
        }
        let input_ids: std::collections::BTreeSet<&str> =
            input.exams.iter().filter_map(|e| e.id.as_deref()).collect();
        let (_now, hlc) = self.stamp();
        let session_id = session.id.clone();
        self.services.db.write(|tx| {
            for ex in existing.iter().filter(|x| x.active && !input_ids.contains(x.id.as_str())) {
                exams::update(tx, &ex.id, &ex.name, ex.max_marks, ex.sort_order, false, &hlc)?;
            }
            for (i, e) in input.exams.iter().enumerate() {
                let sort = i as i64;
                let name = e.name.trim();
                match &e.id {
                    Some(id) => exams::update(tx, id, name, e.max, sort, true, &hlc)?,
                    None => exams::insert(
                        tx,
                        &ExamRow {
                            id: self.services.ids.new_id(),
                            session_id: session_id.clone(),
                            name: name.to_owned(),
                            max_marks: e.max,
                            sort_order: sort,
                            active: true,
                        },
                        &hlc,
                    )?,
                }
            }
            self.settings_log(tx, actor, "exams", "settings.log.exams")?;
            Ok(())
        })?;
        self.get(actor)
    }

    pub fn save_grade_scale(&self, actor: &Actor, input: GradeScaleInput) -> Result<SettingsDto, ServiceError> {
        permissions::authorize(actor, Action::SettingsEdit)?;
        let bands: Vec<GradeBand> = input
            .bands
            .iter()
            .map(|b| GradeBand {
                grade: b.grade.trim().to_owned(),
                min_percent: b.min_percent.max(0) as u32,
            })
            .collect();
        validate_grade_scale(&bands)?;
        let rows: Vec<(String, i64)> = bands
            .iter()
            .map(|b| (b.grade.clone(), i64::from(b.min_percent)))
            .collect();
        let (_now, hlc) = self.stamp();
        self.services.db.write(|tx| {
            grade_scale::replace_all(tx, &rows, &hlc)?;
            self.settings_log(tx, actor, "grades", "settings.log.grades")?;
            Ok(())
        })?;
        self.get(actor)
    }

    pub fn save_app_settings(&self, actor: &Actor, input: AppSettingsInput) -> Result<SettingsDto, ServiceError> {
        permissions::authorize(actor, Action::SettingsEdit)?;
        if !(5..=120).contains(&input.session_timeout_minutes) {
            return Err(ServiceError::validation("settings.error.timeout").field("sessionTimeoutMinutes"));
        }
        validate_hhmm(&input.school_start)?;
        validate_hhmm(&input.school_end)?;
        self.services.db.write(|tx| {
            let bool_str = |b: bool| if b { "1" } else { "0" };
            repo::app_settings::set(tx, "session_timeout_minutes", &input.session_timeout_minutes.to_string())?;
            repo::app_settings::set(tx, "receipt_paper", &input.receipt_paper)?;
            repo::app_settings::set(tx, "print_language", &input.print_language)?;
            repo::app_settings::set(tx, "tray", bool_str(input.tray))?;
            repo::app_settings::set(tx, "start_at_login", bool_str(input.start_at_login))?;
            repo::app_settings::set(tx, "keep_awake", bool_str(input.keep_awake))?;
            repo::app_settings::set(tx, "school_start", &input.school_start)?;
            repo::app_settings::set(tx, "school_end", &input.school_end)?;
            self.settings_log(tx, actor, "app", "settings.log.app")?;
            Ok(())
        })?;
        self.get(actor)
    }

    pub fn save_device_code(&self, actor: &Actor, code: &str) -> Result<SettingsDto, ServiceError> {
        permissions::authorize(actor, Action::SettingsEdit)?;
        let code = validate_receipt_prefix(code)?;
        self.services.db.write(|tx| {
            meta::set(tx, "device_code", &code)?;
            self.settings_log(tx, actor, "device", "settings.log.device")?;
            Ok(())
        })?;
        self.get(actor)
    }
}

fn section_names(count: i64) -> Vec<String> {
    (0..count.max(0)).map(|i| ((b'A' + i as u8) as char).to_string()).collect()
}

fn rupees(input: &str) -> Result<i64, ServiceError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(0);
    }
    Ok(parse_rupees(trimmed)?.0)
}

fn validate_hhmm(value: &str) -> Result<(), ServiceError> {
    let v = value.trim();
    if v.is_empty() {
        return Ok(());
    }
    let ok = matches!(v.split_once(':'), Some((h, m))
        if h.len() == 2
            && m.len() == 2
            && h.parse::<u32>().is_ok_and(|h| h < 24)
            && m.parse::<u32>().is_ok_and(|m| m < 60));
    if ok {
        Ok(())
    } else {
        Err(ServiceError::validation("settings.error.school_hours"))
    }
}
