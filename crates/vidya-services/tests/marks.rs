//! MarksService behaviour tests (P3.5 Task 3).

use std::collections::BTreeSet;
use std::sync::Arc;

use chrono::{NaiveDate, TimeZone, Utc};
use serde_json::{json, Value};
use vidya_core::error::ErrorKind;
use vidya_core::roles::{Actor, Lang, Origin, Role};
use vidya_db::{repo, Db};
use vidya_services::env::{FixedClock, SeededRandom, UuidV7};
use vidya_services::services::marks::{MarkEntry, MarksService, SaveMarksInput};
use vidya_services::services::students::{StudentDetailDto, StudentInput, StudentService};
use vidya_services::{Mode, Services};

const HLC: &str = "0000000001000-000001-VD";
const NOW: &str = "2026-09-20T12:00:00Z";
const SUBJECTS: [(&str, &str); 5] = [
    ("s_hi", "Hindi"),
    ("s_en", "English"),
    ("s_ma", "Maths"),
    ("s_sc", "Science"),
    ("s_so", "Social"),
];

struct School {
    services: Services,
}

fn build_services(db: Arc<Db>) -> Services {
    let clock = Arc::new(FixedClock {
        now: Utc.with_ymd_and_hms(2026, 9, 20, 12, 0, 0).unwrap(),
        today: NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
    });
    Services::new(
        db,
        clock,
        Arc::new(UuidV7),
        Arc::new(SeededRandom::new(1)),
        Mode::Server,
        "VD-DEMO-DEMO-DEMO".to_owned(),
    )
    .expect("services")
}

fn user_row(id: &str, role: &str) -> repo::users::UserRow {
    repo::users::UserRow {
        id: id.into(),
        username: id.into(),
        name: id.into(),
        role: role.into(),
        mobile: "9876500000".into(),
        password_hash: "x".into(),
        must_change: false,
        failed_count: 0,
        locked: false,
        locked_until: None,
        active: true,
        language: "en".into(),
        created_at: NOW.into(),
        last_login_at: None,
        password_changed_at: None,
    }
}

fn seed(services: &Services) {
    services
        .db
        .write(|tx| {
            vidya_db::seed_defaults(tx, NOW, HLC)?;
            repo::school::upsert(
                tx,
                &repo::school::SchoolRow {
                    name: "Vaani".into(),
                    address: String::new(),
                    udise: String::new(),
                    board: "State Board".into(),
                    phone: String::new(),
                },
                NOW,
                HLC,
            )?;
            repo::sessions::insert(
                tx,
                &repo::sessions::SessionRow {
                    id: "ses1".into(),
                    name: "2026-27".into(),
                    starts_on: "2026-04-01".into(),
                    ends_on: "2027-03-31".into(),
                    is_current: true,
                    terms: 3,
                    transport_fee_per_term: 900,
                },
                NOW,
                HLC,
            )?;
            repo::classes::insert_class(
                tx,
                &repo::classes::ClassRow {
                    id: "cv".into(),
                    name: "V".into(),
                    sort_order: 5,
                    active: true,
                },
                HLC,
            )?;
            for (id, name) in [("va", "A"), ("vb", "B")] {
                repo::classes::insert_section(
                    tx,
                    &repo::classes::SectionRow {
                        id: id.into(),
                        class_id: "cv".into(),
                        name: name.into(),
                        active: true,
                    },
                    HLC,
                )?;
            }
            for (i, (id, name)) in SUBJECTS.iter().enumerate() {
                repo::subjects::insert(
                    tx,
                    &repo::subjects::SubjectRow {
                        id: (*id).into(),
                        class_id: "cv".into(),
                        name: (*name).into(),
                        sort_order: i as i64,
                        active: true,
                    },
                    HLC,
                )?;
            }
            repo::exams::insert(
                tx,
                &repo::exams::ExamRow {
                    id: "ex1".into(),
                    session_id: "ses1".into(),
                    name: "Half Yearly".into(),
                    max_marks: 25,
                    sort_order: 0,
                    active: true,
                },
                HLC,
            )?;
            repo::users::insert(tx, &user_row("sunita", "principal"), HLC)?;
            repo::users::insert(tx, &user_row("sierra", "teacher"), HLC)?;
            Ok(())
        })
        .expect("seed");
}

impl School {
    fn new() -> Self {
        let db = Arc::new(Db::open_in_memory_for_tests().expect("db"));
        let services = build_services(db);
        seed(&services);
        Self { services }
    }
    fn marks(&self) -> MarksService<'_> {
        MarksService::new(&self.services)
    }
    fn mark_count(&self) -> usize {
        self.services
            .db
            .read(|c| repo::marks::list_for_exam(c, "ex1"))
            .unwrap()
            .len()
    }
}

fn principal() -> Actor {
    Actor {
        user_id: "sunita".into(),
        role: Role::Principal,
        section_ids: BTreeSet::new(),
        device_id: "VD-DEMO-DEMO-DEMO".into(),
        lang: Lang::En,
        origin: Origin::OfficeComputer,
    }
}

fn teacher() -> Actor {
    Actor {
        user_id: "sierra".into(),
        role: Role::Teacher,
        section_ids: BTreeSet::from(["va".to_owned()]),
        device_id: "VD-DEMO-DEMO-DEMO".into(),
        lang: Lang::En,
        origin: Origin::OfficeComputer,
    }
}

fn admit(school: &School, name: &str) -> String {
    let detail = StudentService::new(&school.services)
        .add(
            &principal(),
            StudentInput {
                name: name.into(),
                gender: "Male".into(),
                dob: String::new(),
                father: "Raj Kumar".into(),
                mother: String::new(),
                mobile: "9876543210".into(),
                category: "General".into(),
                locality: String::new(),
                section_id: "va".into(),
                rte: false,
                transport: false,
                concession: 0,
                aadhaar_collected: false,
                apaar_created: false,
                confirm_duplicate: true,
            },
        )
        .expect("admit");
    match detail {
        StudentDetailDto::Office(d) => d.office.id,
        StudentDetailDto::Teacher(_) => unreachable!(),
    }
}

fn entries(student: &str, cells: &[(&str, Value)]) -> SaveMarksInput {
    SaveMarksInput {
        exam_id: "ex1".into(),
        section_id: "va".into(),
        entries: cells
            .iter()
            .map(|(sub, value)| MarkEntry {
                student_id: student.to_owned(),
                subject_id: (*sub).to_owned(),
                value: value.clone(),
            })
            .collect(),
    }
}

#[test]
fn a_value_above_max_lists_the_cell_and_saves_nothing() {
    let school = School::new();
    let a = admit(&school, "Aman Yadav");
    let err = school
        .marks()
        .save(
            &principal(),
            entries(&a, &[("s_hi", json!(30)), ("s_en", json!(10))]),
        )
        .expect_err("above max");
    assert_eq!(err.message_key, "marks.error.cells");
    assert!(err.params.get("list").unwrap().contains("Aman Yadav – Hindi"));
    let cells = err.params.get("cells").unwrap();
    assert!(cells.contains("s_hi"));
    // The whole request is rejected: nothing was written.
    assert_eq!(school.mark_count(), 0);
}

#[test]
fn absent_counts_in_max_and_blank_is_excluded() {
    let school = School::new();
    let a = admit(&school, "Aman Yadav");
    let sheet = school
        .marks()
        .save(
            &principal(),
            entries(
                &a,
                &[("s_hi", json!(20)), ("s_en", json!("AB")), ("s_ma", json!(null))],
            ),
        )
        .expect("save");
    let student = sheet.students.iter().find(|s| s.id == a).unwrap();
    // Hindi 20 + English AB (0/25) counted, Maths blank excluded → 20/50.
    assert_eq!(student.total, "20/50");
}

#[test]
fn clearing_a_value_deletes_the_row() {
    let school = School::new();
    let a = admit(&school, "Aman Yadav");
    school
        .marks()
        .save(&principal(), entries(&a, &[("s_hi", json!(20))]))
        .expect("save");
    assert_eq!(school.mark_count(), 1);
    school
        .marks()
        .save(&principal(), entries(&a, &[("s_hi", json!(null))]))
        .expect("clear");
    assert_eq!(school.mark_count(), 0);
}

#[test]
fn report_card_percent_and_grade() {
    let school = School::new();
    let a = admit(&school, "Aman Yadav");
    // 20 + 15 + 18 + 12 + 12 = 77 out of 125 → 61.6% → C on the default scale.
    school
        .marks()
        .save(
            &principal(),
            entries(
                &a,
                &[
                    ("s_hi", json!(20)),
                    ("s_en", json!(15)),
                    ("s_ma", json!(18)),
                    ("s_sc", json!(12)),
                    ("s_so", json!(12)),
                ],
            ),
        )
        .expect("save");
    let card = school.marks().report_card(&principal(), &a).expect("report");
    let total = &card.totals[0];
    assert_eq!(total.got, 77);
    assert_eq!(total.max, 125);
    assert_eq!(total.pct.as_deref(), Some("61.6"));
    assert_eq!(total.grade, "C");
}

#[test]
fn teacher_other_section_is_denied() {
    let school = School::new();
    let _a = admit(&school, "Aman Yadav");
    let err = school
        .marks()
        .sheet(&teacher(), "ex1", "vb")
        .expect_err("other section");
    assert_eq!(err.kind, ErrorKind::Permission);
}
