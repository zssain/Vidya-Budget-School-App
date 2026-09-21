//! AttendanceService behaviour tests (P3.4 Task 3).

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use chrono::{NaiveDate, TimeZone, Utc};
use vidya_core::error::ErrorKind;
use vidya_core::roles::{Actor, Lang, Origin, Role};
use vidya_db::{repo, Db};
use vidya_services::env::{FixedClock, SeededRandom, UuidV7};
use vidya_services::services::attendance::{AttendanceService, SaveAttendanceInput};
use vidya_services::services::students::{StudentDetailDto, StudentInput, StudentService};
use vidya_services::{Mode, Services};

const HLC: &str = "0000000001000-000001-VD";
const NOW: &str = "2026-09-20T12:00:00Z";
const TODAY: &str = "2026-09-20";
const YESTERDAY: &str = "2026-09-19";

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
    fn attendance(&self) -> AttendanceService<'_> {
        AttendanceService::new(&self.services)
    }
    fn last_log_summary(&self) -> String {
        self.services
            .db
            .read(|c| {
                Ok(c.query_row(
                    "SELECT summary_key FROM change_log ORDER BY seq DESC LIMIT 1",
                    [],
                    |row| row.get::<_, String>(0),
                )?)
            })
            .unwrap()
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

fn admit(school: &School, section: &str, name: &str) -> String {
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
                section_id: section.into(),
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

fn save(section: &str, date: &str, marks: &[(&str, &str)]) -> SaveAttendanceInput {
    SaveAttendanceInput {
        section_id: section.into(),
        date: date.into(),
        marks: marks
            .iter()
            .map(|(id, s)| ((*id).to_owned(), (*s).to_owned()))
            .collect::<HashMap<_, _>>(),
    }
}

#[test]
fn teacher_marks_today_but_not_yesterday_or_other_section() {
    let school = School::new();
    let a = admit(&school, "va", "Aaa Kumar");
    let b = admit(&school, "va", "Bbb Kumar");

    // Today, own section — allowed.
    let sheet = school
        .attendance()
        .save(&teacher(), save("va", TODAY, &[(&a, "P"), (&b, "A")]))
        .expect("today");
    assert_eq!(sheet.saved_by.as_deref(), Some("sierra"));
    assert_eq!(sheet.marks.get(&a).map(String::as_str), Some("P"));

    // Yesterday — edit_past denied for a teacher.
    let err = school
        .attendance()
        .save(&teacher(), save("va", YESTERDAY, &[(&a, "P"), (&b, "P")]))
        .expect_err("yesterday");
    assert_eq!(err.kind, ErrorKind::Permission);

    // Another section — not in the teacher's scope.
    let err = school
        .attendance()
        .save(&teacher(), save("vb", TODAY, &[]))
        .expect_err("other section");
    assert_eq!(err.kind, ErrorKind::Permission);
}

#[test]
fn principal_correction_logs_corrected() {
    let school = School::new();
    let a = admit(&school, "va", "Aaa Kumar");
    school
        .attendance()
        .save(&principal(), save("va", YESTERDAY, &[(&a, "P")]))
        .expect("first save");
    assert_eq!(school.last_log_summary(), "attendance.log.saved");
    school
        .attendance()
        .save(&principal(), save("va", YESTERDAY, &[(&a, "A")]))
        .expect("correction");
    assert_eq!(school.last_log_summary(), "attendance.log.corrected");
}

#[test]
fn future_date_is_refused() {
    let school = School::new();
    let a = admit(&school, "va", "Aaa Kumar");
    for actor in [principal(), teacher()] {
        let err = school
            .attendance()
            .save(&actor, save("va", "2026-09-21", &[(&a, "P")]))
            .expect_err("future");
        assert_eq!(err.message_key, "attendance.error.future");
    }
}

#[test]
fn missing_a_student_names_them() {
    let school = School::new();
    let a = admit(&school, "va", "Aaa Kumar");
    let _b = admit(&school, "va", "Bbb Kumar");
    let err = school
        .attendance()
        .save(&principal(), save("va", TODAY, &[(&a, "P")]))
        .expect_err("missing");
    assert_eq!(err.message_key, "attendance.error.not_marked");
    assert_eq!(err.params.get("names").map(String::as_str), Some("Bbb Kumar"));
}

#[test]
fn a_student_who_left_is_not_in_the_sheet() {
    let school = School::new();
    let a = admit(&school, "va", "Aaa Kumar");
    let b = admit(&school, "va", "Bbb Kumar");
    StudentService::new(&school.services)
        .mark_left(&principal(), &b, YESTERDAY, "moved away")
        .expect("mark left");
    let sheet = school
        .attendance()
        .sheet(&principal(), "va", TODAY)
        .expect("sheet");
    assert_eq!(sheet.students.len(), 1);
    assert_eq!(sheet.students[0].id, a);
}

#[test]
fn register_counts_present_days() {
    let school = School::new();
    let a = admit(&school, "va", "Aaa Kumar");
    let b = admit(&school, "va", "Bbb Kumar");
    school
        .attendance()
        .save(&principal(), save("va", YESTERDAY, &[(&a, "P"), (&b, "A")]))
        .unwrap();
    school
        .attendance()
        .save(&principal(), save("va", TODAY, &[(&a, "P"), (&b, "P")]))
        .unwrap();
    let register = school
        .attendance()
        .register(&principal(), "va", "2026-09")
        .expect("register");
    assert_eq!(register.days, 30);
    let row_a = register.rows.iter().find(|r| r.name == "Aaa Kumar").unwrap();
    let row_b = register.rows.iter().find(|r| r.name == "Bbb Kumar").unwrap();
    assert_eq!(row_a.present, 2);
    assert_eq!(row_b.present, 1);
}
