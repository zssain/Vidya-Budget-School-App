//! The deterministic sample school ("Vaani Public School"), used by tests and
//! debug builds. Given the same `seed`, `SampleSchool::build` produces identical
//! data. It writes through the repositories in one transaction (P2.5 Task 8).

use std::collections::HashMap;

use chrono::Days;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use vidya_core::dates::{default_session_name, session_bounds};
use vidya_db::repo::{
    attendance, classes, enrollments, exams, fee_plans, license, marks, receipts, school, sessions, students,
    subjects, users,
};
use vidya_db::{repo, DbError};
use vidya_services::auth::password;
use vidya_services::{ServiceError, Services};

/// The logins created by [`SampleSchool::build`] (all with password `vidya123`).
#[derive(Debug, Clone)]
pub struct SampleLogins {
    pub principal: String,
    pub accountant: String,
    pub teachers: Vec<String>,
    pub student_count: usize,
    pub password: String,
}

pub struct SampleSchool;

const SUBJECTS: [&str; 5] = ["Hindi", "English", "Mathematics", "Science", "Social Studies"];
const FIRST_M: [&str; 12] = [
    "Aman", "Rohit", "Vikas", "Arjun", "Karan", "Raj", "Sahil", "Nikhil", "Deepak", "Ravi", "Ankit", "Gaurav",
];
const FIRST_F: [&str; 12] = [
    "Priya", "Neha", "Anjali", "Pooja", "Kavya", "Riya", "Sneha", "Divya", "Meena", "Sakshi", "Isha", "Ritu",
];
const SURNAME: [&str; 12] = [
    "Yadav", "Sharma", "Verma", "Gupta", "Singh", "Kumar", "Mishra", "Pandey", "Tiwari", "Joshi", "Chauhan",
    "Saini",
];

struct SectionInfo {
    class_id: String,
    section_id: String,
    label: String,
    sort_order: usize,
}

impl SampleSchool {
    pub fn build(services: &Services, seed: u64) -> Result<SampleLogins, ServiceError> {
        let today = services.clock.today_local();
        let session_name = default_session_name(today);
        let (starts, ends) = session_bounds(&session_name)?;
        let now = services.clock.now_utc().to_rfc3339();
        let hlc = services.next_hlc().to_text();
        let password_hash = password::hash_password(&*services.random, "vidya123")?;

        let logins = services.db.write(|tx| {
            vidya_db::seed_defaults(tx, &now, &hlc)?;

            school::upsert(
                tx,
                &school::SchoolRow {
                    name: "Vaani Public School".into(),
                    address: "Station Road, Vidyanagar".into(),
                    udise: String::new(),
                    board: "State Board".into(),
                    phone: "9000000000".into(),
                },
                &now,
                &hlc,
            )?;

            let session_id = services.ids.new_id();
            sessions::insert(
                tx,
                &sessions::SessionRow {
                    id: session_id.clone(),
                    name: session_name.clone(),
                    starts_on: starts.to_string(),
                    ends_on: ends.to_string(),
                    is_current: true,
                    terms: 3,
                    transport_fee_per_term: 900,
                },
                &now,
                &hlc,
            )?;

            // Classes, sections, fee plans and subjects.
            let class_defs: [(&str, usize); 9] = [
                ("Nursery", 1),
                ("I", 2),
                ("II", 2),
                ("III", 2),
                ("IV", 2),
                ("V", 2),
                ("VI", 2),
                ("VII", 2),
                ("VIII", 2),
            ];
            let mut sections: Vec<SectionInfo> = Vec::new();
            let mut label_to_section: HashMap<String, String> = HashMap::new();
            let mut subjects_by_class: HashMap<String, Vec<String>> = HashMap::new();

            for (order, (class_name, section_count)) in class_defs.iter().enumerate() {
                let class_id = services.ids.new_id();
                classes::insert_class(
                    tx,
                    &classes::ClassRow {
                        id: class_id.clone(),
                        name: (*class_name).into(),
                        sort_order: order as i64,
                        active: true,
                    },
                    &hlc,
                )?;
                fee_plans::upsert(
                    tx,
                    &fee_plans::FeePlanRow {
                        session_id: session_id.clone(),
                        class_id: class_id.clone(),
                        tuition: 2000 + (order as i64) * 100,
                        exam: 300,
                        other: 400,
                    },
                    &hlc,
                )?;
                let mut subject_ids = Vec::new();
                for (sort, subject) in SUBJECTS.iter().enumerate() {
                    let subject_id = services.ids.new_id();
                    subjects::insert(
                        tx,
                        &subjects::SubjectRow {
                            id: subject_id.clone(),
                            class_id: class_id.clone(),
                            name: (*subject).into(),
                            sort_order: sort as i64,
                            active: true,
                        },
                        &hlc,
                    )?;
                    subject_ids.push(subject_id);
                }
                subjects_by_class.insert(class_id.clone(), subject_ids);

                for s in 0..*section_count {
                    let section_name = ["A", "B"][s];
                    let section_id = services.ids.new_id();
                    classes::insert_section(
                        tx,
                        &classes::SectionRow {
                            id: section_id.clone(),
                            class_id: class_id.clone(),
                            name: section_name.into(),
                            active: true,
                        },
                        &hlc,
                    )?;
                    let label = format!("{class_name}-{section_name}");
                    label_to_section.insert(label.clone(), section_id.clone());
                    sections.push(SectionInfo {
                        class_id: class_id.clone(),
                        section_id,
                        label,
                        sort_order: order,
                    });
                }
            }

            // Two exams for the session; remember the Unit Test 1 id for marks.
            let mut unit_test_exam = String::new();
            for (sort, (name, max)) in [("Unit Test 1", 25), ("Half Yearly", 100)].iter().enumerate() {
                let exam_id = services.ids.new_id();
                exams::insert(
                    tx,
                    &exams::ExamRow {
                        id: exam_id.clone(),
                        session_id: session_id.clone(),
                        name: (*name).into(),
                        max_marks: *max,
                        sort_order: sort as i64,
                        active: true,
                    },
                    &hlc,
                )?;
                if *name == "Unit Test 1" {
                    unit_test_exam = exam_id;
                }
            }

            // Staff.
            let make_user = |tx: &rusqlite::Transaction<'_>,
                             username: &str,
                             name: &str,
                             role: &str|
             -> Result<String, DbError> {
                let id = services.ids.new_id();
                users::insert(
                    tx,
                    &users::UserRow {
                        id: id.clone(),
                        username: username.into(),
                        name: name.into(),
                        role: role.into(),
                        mobile: String::new(),
                        password_hash: password_hash.clone(),
                        must_change: false,
                        failed_count: 0,
                        locked: false,
                        locked_until: None,
                        active: true,
                        language: "en".into(),
                        created_at: now.clone(),
                        last_login_at: None,
                        password_changed_at: None,
                    },
                    &hlc,
                )?;
                Ok(id)
            };

            let sunita = make_user(tx, "sunita", "Sunita Mishra", "principal")?;
            let anita = make_user(tx, "anita", "Anita Verma", "accountant")?;
            let sierra = make_user(tx, "sierra", "Sierra D'Souza", "teacher")?;
            let rakesh = make_user(tx, "rakesh", "Rakesh Kumar", "teacher")?;

            let section_ids = |labels: &[&str]| -> Vec<String> {
                labels
                    .iter()
                    .filter_map(|l| label_to_section.get(*l).cloned())
                    .collect()
            };
            users::set_sections(tx, &sierra, &section_ids(&["V-A", "V-B"]))?;
            users::set_sections(tx, &rakesh, &section_ids(&["VI-A", "VI-B", "VII-A"]))?;

            // Students: eight per section, deterministic from the seed.
            let mut rng = StdRng::seed_from_u64(seed);
            let mut student_count = 0usize;
            let mut students_by_section: HashMap<String, Vec<String>> = HashMap::new();
            let mut va_students: Vec<String> = Vec::new();
            let mut va_class_id = String::new();

            for section in &sections {
                for roll in 1..=8i64 {
                    let is_male = rng.random_bool(0.5);
                    let first = if is_male {
                        FIRST_M[rng.random_range(0..FIRST_M.len())]
                    } else {
                        FIRST_F[rng.random_range(0..FIRST_F.len())]
                    };
                    let surname = SURNAME[rng.random_range(0..SURNAME.len())];
                    let father_first = FIRST_M[rng.random_range(0..FIRST_M.len())];
                    let mobile = format!(
                        "{}{:09}",
                        rng.random_range(6..=9u8),
                        rng.random_range(0..1_000_000_000u32)
                    );
                    let adm = repo::meta::next_counter(tx, "adm_no")?;
                    let student_id = services.ids.new_id();
                    students::insert(
                        tx,
                        &students::StudentRow {
                            id: student_id.clone(),
                            adm_no: format!("ADM/{adm:04}"),
                            name: format!("{first} {surname}"),
                            gender: if is_male { "Male".into() } else { "Female".into() },
                            dob: Some(format!("{}-06-15", 2010 + section.sort_order as i32)),
                            father: format!("{father_first} {surname}"),
                            mother: String::new(),
                            mobile,
                            category: ["General", "OBC", "SC", "ST"][rng.random_range(0..4)].into(),
                            locality: String::new(),
                            aadhaar_collected: rng.random_bool(0.7),
                            apaar_created: false,
                            admitted_on: starts.to_string(),
                        },
                        &now,
                        &hlc,
                    )?;
                    enrollments::insert(
                        tx,
                        &enrollments::EnrollmentRow {
                            id: services.ids.new_id(),
                            student_id: student_id.clone(),
                            session_id: session_id.clone(),
                            class_id: section.class_id.clone(),
                            section_id: section.section_id.clone(),
                            roll,
                            rte: rng.random_bool(0.1),
                            transport: rng.random_bool(0.3),
                            concession: 0,
                            status: "active".into(),
                        },
                        &hlc,
                    )?;
                    student_count += 1;
                    students_by_section
                        .entry(section.section_id.clone())
                        .or_default()
                        .push(student_id.clone());
                    if section.label == "V-A" {
                        va_students.push(student_id.clone());
                        va_class_id = section.class_id.clone();
                    }

                    // A cash receipt (from the accountant) for roughly half the students.
                    if rng.random_bool(0.5) {
                        let no = repo::meta::next_counter(tx, "receipt:PC")?;
                        receipts::insert(
                            tx,
                            &receipts::ReceiptRow {
                                id: services.ids.new_id(),
                                receipt_no: format!("PC-{no:04}"),
                                session_id: session_id.clone(),
                                student_id: student_id.clone(),
                                amount: 1000,
                                mode: "Cash".into(),
                                reference: String::new(),
                                note: String::new(),
                                paid_on: starts.to_string(),
                                created_by: anita.clone(),
                                device_code: "PC".into(),
                                balance_after: 0,
                                hlc: hlc.clone(),
                            },
                            &now,
                        )?;
                    }
                }
            }

            // Six working days of attendance before today, for every section.
            for section in &sections {
                let Some(student_ids) = students_by_section.get(&section.section_id) else {
                    continue;
                };
                for day in 1..=6u64 {
                    let date = today
                        .checked_sub_days(Days::new(day))
                        .expect("date in range")
                        .to_string();
                    let day_id = services.ids.new_id();
                    attendance::insert_day(
                        tx,
                        &attendance::AttendanceDayRow {
                            id: day_id.clone(),
                            session_id: session_id.clone(),
                            section_id: section.section_id.clone(),
                            date,
                            saved_by: sunita.clone(),
                            saved_at: now.clone(),
                        },
                        &hlc,
                    )?;
                    for student_id in student_ids {
                        let status = ["P", "P", "P", "A", "L"][rng.random_range(0..5)];
                        attendance::insert_mark(tx, &day_id, student_id, status)?;
                    }
                }
            }

            // Unit Test 1 marks for class V-A.
            if let Some(subject_ids) = subjects_by_class.get(&va_class_id) {
                for student_id in &va_students {
                    for subject_id in subject_ids {
                        marks::upsert(
                            tx,
                            &marks::MarkRow {
                                id: services.ids.new_id(),
                                exam_id: unit_test_exam.clone(),
                                student_id: student_id.clone(),
                                subject_id: subject_id.clone(),
                                value: Some(rng.random_range(0..=25i64)),
                                absent: false,
                                entered_by: sunita.clone(),
                                entered_at: now.clone(),
                            },
                            &now,
                            &hlc,
                        )?;
                    }
                }
            }

            license::upsert(
                tx,
                &license::LicenseRow {
                    school_code: "vaani".into(),
                    activation_code: "DEMO".into(),
                    payload_b64: "DEMO".into(),
                    signature_b64: "DEMO".into(),
                    device_id: "VD-DEMO-DEMO-DEMO".into(),
                    max_users: 40,
                    max_devices: 5,
                    license_type: 0,
                    issued_on: starts.to_string(),
                    activated_at: now.clone(),
                },
            )?;

            Ok::<SampleLogins, DbError>(SampleLogins {
                principal: "sunita".into(),
                accountant: "anita".into(),
                teachers: vec!["sierra".into(), "rakesh".into()],
                student_count,
                password: "vidya123".into(),
            })
        })?;

        Ok(logins)
    }
}
