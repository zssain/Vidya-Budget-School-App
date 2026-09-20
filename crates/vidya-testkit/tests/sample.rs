//! Sample-school determinism and a settings snapshot (P2.5 Task 8).

use std::collections::BTreeSet;
use std::sync::Arc;

use chrono::{NaiveDate, TimeZone, Utc};
use vidya_core::roles::{Actor, Lang, Origin, Role};
use vidya_db::Db;
use vidya_services::env::{FixedClock, SeededRandom, SeqIds};
use vidya_services::{Mode, Services};
use vidya_testkit::SampleSchool;

fn test_services(seed: u64) -> Services {
    let db = Arc::new(Db::open_in_memory_for_tests().expect("open db"));
    let clock = Arc::new(FixedClock {
        now: Utc.with_ymd_and_hms(2026, 9, 20, 12, 0, 0).unwrap(),
        today: NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
    });
    Services::new(
        db,
        clock,
        Arc::new(SeqIds::new("id-")),
        Arc::new(SeededRandom::new(seed)),
        Mode::Server,
        "VD-DEMO-DEMO-DEMO".to_owned(),
    )
    .expect("services")
}

fn first_student_names(services: &Services) -> Vec<String> {
    services
        .db
        .read(|conn| {
            let mut stmt = conn.prepare("SELECT name FROM students ORDER BY adm_no LIMIT 3")?;
            let names = stmt
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(names)
        })
        .expect("read names")
}

#[test]
fn same_seed_is_reproducible_and_different_seed_differs() {
    let a = test_services(42);
    let logins_a = SampleSchool::build(&a, 42).expect("build a");
    let a2 = test_services(42);
    let logins_a2 = SampleSchool::build(&a2, 42).expect("build a2");
    let b = test_services(99);
    let _ = SampleSchool::build(&b, 99).expect("build b");

    // 17 sections × 8 students.
    assert_eq!(logins_a.student_count, 136);
    assert_eq!(logins_a2.student_count, 136);
    assert_eq!(logins_a.principal, "sunita");

    let names_a = first_student_names(&a);
    let names_a2 = first_student_names(&a2);
    let names_b = first_student_names(&b);
    assert_eq!(names_a, names_a2, "same seed must give the same names");
    assert_ne!(names_a, names_b, "different seed must give different names");
}

#[test]
fn settings_snapshot() {
    let services = test_services(7);
    SampleSchool::build(&services, 7).expect("build");

    let principal = Actor {
        user_id: "sunita".to_owned(),
        role: Role::Principal,
        section_ids: BTreeSet::new(),
        device_id: "VD-DEMO-DEMO-DEMO".to_owned(),
        lang: Lang::En,
        origin: Origin::OfficeComputer,
    };
    let settings = services.settings().get(&principal).expect("settings");
    insta::assert_json_snapshot!(settings);
}
