//! StudentService behaviour tests (P3.2 Task 4). Uses a small empty school so
//! roll numbers start at 1.

use std::collections::BTreeSet;
use std::sync::Arc;

use chrono::{NaiveDate, TimeZone, Utc};
use vidya_core::error::ErrorKind;
use vidya_core::roles::{Actor, Lang, Origin, Role};
use vidya_db::{repo, Db};
use vidya_services::env::{FixedClock, SeededRandom, UuidV7};
use vidya_services::services::students::{StudentDetailDto, StudentFilter, StudentInput, StudentService};
use vidya_services::{Mode, Services};

const HLC: &str = "0000000001000-000001-VD";
const NOW: &str = "2026-09-20T12:00:00Z";

struct School {
    services: Services,
}

fn build_services(db: Arc<Db>, mode: Mode) -> Services {
    let clock = Arc::new(FixedClock {
        now: Utc.with_ymd_and_hms(2026, 9, 20, 12, 0, 0).unwrap(),
        today: NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
    });
    // Real UUID v7 ids so ids never collide across a database reopen.
    Services::new(
        db,
        clock,
        Arc::new(UuidV7),
        Arc::new(SeededRandom::new(1)),
        mode,
        "VD-DEMO-DEMO-DEMO".to_owned(),
    )
    .expect("services")
}

fn seed_minimal(services: &Services) -> (String, String, String) {
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
            repo::fee_plans::upsert(
                tx,
                &repo::fee_plans::FeePlanRow {
                    session_id: "ses1".into(),
                    class_id: "cv".into(),
                    tuition: 2400,
                    exam: 300,
                    other: 400,
                },
                HLC,
            )?;
            Ok(())
        })
        .expect("seed");
    ("va".to_owned(), "vb".to_owned(), "ses1".to_owned())
}

impl School {
    fn new(mode: Mode) -> Self {
        let db = Arc::new(Db::open_in_memory_for_tests().expect("db"));
        let services = build_services(db, mode);
        seed_minimal(&services);
        Self { services }
    }

    fn students(&self) -> StudentService<'_> {
        StudentService::new(&self.services)
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

fn accountant() -> Actor {
    Actor {
        role: Role::Accountant,
        ..principal()
    }
}

fn teacher(sections: &[&str]) -> Actor {
    Actor {
        user_id: "sierra".into(),
        role: Role::Teacher,
        section_ids: sections.iter().map(|s| s.to_string()).collect(),
        device_id: "VD-DEMO-DEMO-DEMO".into(),
        lang: Lang::En,
        origin: Origin::OfficeComputer,
    }
}

fn admit(school: &School, section: &str, name: &str, confirm: bool) -> StudentDetailDto {
    school
        .students()
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
                confirm_duplicate: confirm,
            },
        )
        .expect("admit")
}

fn office(detail: &StudentDetailDto) -> &vidya_services::services::students::StudentOfficeDetailDto {
    match detail {
        StudentDetailDto::Office(d) => d,
        StudentDetailDto::Teacher(_) => panic!("expected office detail"),
    }
}

#[test]
fn admission_numbers_are_sequential_and_survive_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vidya.db");
    let key = zeroize::Zeroizing::new([3u8; 32]);

    {
        let db = Arc::new(Db::open(&path, &key).unwrap());
        let services = build_services(db, Mode::Server);
        seed_minimal(&services);
        let school = School { services };
        assert_eq!(
            office(&admit(&school, "va", "One Kumar", false)).office.adm_no,
            "ADM/0001"
        );
        assert_eq!(
            office(&admit(&school, "va", "Two Kumar", false)).office.adm_no,
            "ADM/0002"
        );
    }
    // Reopen the same encrypted database: admission numbers continue.
    let db = Arc::new(Db::open(&path, &key).unwrap());
    let services = build_services(db, Mode::Server);
    let school = School { services };
    assert_eq!(
        office(&admit(&school, "va", "Three Kumar", false)).office.adm_no,
        "ADM/0003"
    );
}

#[test]
fn roll_numbers_are_not_reused() {
    let school = School::new(Mode::Server);
    let a = admit(&school, "va", "Aaa Kumar", false);
    let b = admit(&school, "va", "Bbb Kumar", false);
    let _c = admit(&school, "va", "Ccc Kumar", false);
    assert_eq!(office(&a).office.roll, 1);
    assert_eq!(office(&b).office.roll, 2);
    assert_eq!(office(&_c).office.roll, 3);

    // Move roll 2 (b) to V-B → next V-B roll (1).
    let b_id = office(&b).office.id.clone();
    let moved = school
        .students()
        .update(
            &principal(),
            vidya_services::services::students::UpdateStudentInput {
                student_id: b_id,
                name: "Bbb Kumar".into(),
                gender: "Male".into(),
                dob: String::new(),
                father: "Raj Kumar".into(),
                mother: String::new(),
                mobile: "9876543210".into(),
                category: "General".into(),
                locality: String::new(),
                section_id: "vb".into(),
                rte: false,
                transport: false,
                concession: 0,
                aadhaar_collected: false,
                apaar_created: false,
            },
        )
        .expect("move");
    assert_eq!(office(&moved).office.roll, 1);
    assert_eq!(office(&moved).office.section_name, "B");

    // The next V-A admission gets roll 4 (2 is not reused).
    let d = admit(&school, "va", "Ddd Kumar", false);
    assert_eq!(office(&d).office.roll, 4);
}

#[test]
fn teacher_list_is_scoped_and_carries_no_fee_keys() {
    let school = School::new(Mode::Server);
    admit(&school, "va", "In Scope", false);
    admit(&school, "vb", "Out Of Scope", false);
    let list = school
        .students()
        .list(&teacher(&["va"]), StudentFilter::default())
        .expect("list");
    assert_eq!(list.items.len(), 1);
    let json = serde_json::to_string(&list).unwrap();
    for forbidden in ["rte", "concession", "\"due\"", "paid", "balance", "category"] {
        assert!(!json.contains(forbidden), "teacher list leaked {forbidden}");
    }
}

#[test]
fn concession_needs_the_right_permission() {
    let school = School::new(Mode::Server);
    let input = |concession| StudentInput {
        name: "Con Kumar".into(),
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
        concession,
        aadhaar_collected: false,
        apaar_created: false,
        confirm_duplicate: false,
    };
    let err = school
        .students()
        .add(&accountant(), input(1000))
        .expect_err("accountant concession");
    assert_eq!(err.kind, ErrorKind::Permission);
    assert!(school.students().add(&principal(), input(1000)).is_ok());
}

#[test]
fn client_mode_add_is_offline() {
    let school = School::new(Mode::Client);
    let err = school
        .students()
        .add(
            &principal(),
            StudentInput {
                name: "Aaa Kumar".into(),
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
                confirm_duplicate: false,
            },
        )
        .expect_err("offline");
    assert_eq!(err.message_key, "students.error.online_only");
}

#[test]
fn duplicate_needs_confirmation() {
    let school = School::new(Mode::Server);
    admit(&school, "va", "Same Name", false);
    let err = school
        .students()
        .add(
            &principal(),
            StudentInput {
                name: "Same Name".into(),
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
                confirm_duplicate: false,
            },
        )
        .expect_err("duplicate");
    assert_eq!(err.kind, ErrorKind::Conflict);
    assert_eq!(err.message_key, "students.possible_duplicate");
    // Confirming lets it through.
    assert!(office(&admit(&school, "va", "Same Name", true))
        .office
        .adm_no
        .starts_with("ADM/"));
}
