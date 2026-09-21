//! FeeService behaviour tests (P3.3 Task 5). Real enrollments through the
//! student service, then collection, receipt series, cancellation and day book.

use std::collections::BTreeSet;
use std::sync::Arc;

use chrono::{NaiveDate, TimeZone, Utc};
use vidya_core::error::ErrorKind;
use vidya_core::roles::{Actor, Lang, Origin, Role};
use vidya_db::{repo, Db};
use vidya_services::env::{FixedClock, SeededRandom, UuidV7};
use vidya_services::services::fees::{CollectInput, FeeService};
use vidya_services::services::students::{StudentDetailDto, StudentInput, StudentService};
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

fn seed(services: &Services, terms: i64) {
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
                    terms,
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
            // A principal user so receipts (created_by) and cancellations
            // (cancelled_by) satisfy their foreign keys.
            repo::users::insert(
                tx,
                &repo::users::UserRow {
                    id: "sunita".into(),
                    username: "sunita".into(),
                    name: "Sunita".into(),
                    role: "principal".into(),
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
                },
                HLC,
            )?;
            Ok(())
        })
        .expect("seed");
}

impl School {
    fn new(terms: i64) -> Self {
        let db = Arc::new(Db::open_in_memory_for_tests().expect("db"));
        let services = build_services(db, Mode::Server);
        seed(&services, terms);
        Self { services }
    }

    fn students(&self) -> StudentService<'_> {
        StudentService::new(&self.services)
    }

    fn fees(&self) -> FeeService<'_> {
        FeeService::new(&self.services)
    }

    fn set_device_code(&self, code: &str) {
        self.services
            .db
            .write(|tx| repo::meta::set(tx, "device_code", code))
            .expect("device code");
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

fn admit(school: &School, rte: bool, transport: bool, concession: i64) -> String {
    let detail = school
        .students()
        .add(
            &principal(),
            StudentInput {
                name: "Aman Kumar".into(),
                gender: "Male".into(),
                dob: String::new(),
                father: "Raj Kumar".into(),
                mother: String::new(),
                mobile: "9876543210".into(),
                category: "General".into(),
                locality: String::new(),
                section_id: "va".into(),
                rte,
                transport,
                concession,
                aadhaar_collected: false,
                apaar_created: false,
                confirm_duplicate: true,
            },
        )
        .expect("admit");
    match detail {
        StudentDetailDto::Office(d) => d.office.id,
        StudentDetailDto::Teacher(_) => panic!("expected office"),
    }
}

fn collect(student_id: &str, amount: i64, mode: &str, reference: &str) -> CollectInput {
    CollectInput {
        student_id: student_id.to_owned(),
        amount,
        mode: mode.to_owned(),
        reference: reference.to_owned(),
        note: String::new(),
    }
}

#[test]
fn due_vectors_match_core_maths() {
    let school = School::new(3);
    // Per term = 2400 + 300 + 400 = 3100; three terms.
    let plain = admit(&school, false, false, 0);
    assert_eq!(school.fees().account(&principal(), &plain).unwrap().due, 9300);

    let bus = {
        // A second student in section B so admission is unique.
        let d = school
            .students()
            .add(
                &principal(),
                StudentInput {
                    name: "Bus Rider".into(),
                    gender: "Male".into(),
                    dob: String::new(),
                    father: "Raj Kumar".into(),
                    mother: String::new(),
                    mobile: "9876543210".into(),
                    category: "General".into(),
                    locality: String::new(),
                    section_id: "vb".into(),
                    rte: false,
                    transport: true,
                    concession: 0,
                    aadhaar_collected: false,
                    apaar_created: false,
                    confirm_duplicate: true,
                },
            )
            .unwrap();
        match d {
            StudentDetailDto::Office(d) => d.office.id,
            StudentDetailDto::Teacher(_) => unreachable!(),
        }
    };
    // (3100 + 900) * 3 = 12000.
    assert_eq!(school.fees().account(&principal(), &bus).unwrap().due, 12000);

    let concession = admit(&school, false, false, 1000);
    assert_eq!(
        school.fees().account(&principal(), &concession).unwrap().due,
        8300
    );

    let rte = admit(&school, true, false, 0);
    let acct = school.fees().account(&principal(), &rte).unwrap();
    assert_eq!(acct.due, 0);
    assert_eq!(acct.balance, 0);
    assert!(acct.rte);
}

#[test]
fn twelve_terms_multiply() {
    let school = School::new(12);
    let s = admit(&school, false, false, 0);
    assert_eq!(school.fees().account(&principal(), &s).unwrap().due, 37200);
}

#[test]
fn rte_collection_is_refused() {
    let school = School::new(3);
    let rte = admit(&school, true, false, 0);
    let err = school
        .fees()
        .collect(&principal(), collect(&rte, 100, "Cash", ""))
        .expect_err("rte");
    assert_eq!(err.message_key, "fees.error.rte");
}

#[test]
fn overpay_and_zero_balance_are_refused() {
    let school = School::new(3);
    let s = admit(&school, false, false, 0);
    // More than the balance.
    let err = school
        .fees()
        .collect(&principal(), collect(&s, 10000, "Cash", ""))
        .expect_err("over");
    assert_eq!(err.message_key, "fees.error.over_balance");
    assert_eq!(err.field.as_deref(), Some("amount"));
    // Pay it off, then a further payment is refused.
    school
        .fees()
        .collect(&principal(), collect(&s, 9300, "Cash", ""))
        .expect("full");
    let err = school
        .fees()
        .collect(&principal(), collect(&s, 100, "Cash", ""))
        .expect_err("no balance");
    assert_eq!(err.message_key, "fees.error.no_balance");
}

#[test]
fn receipt_series_is_per_device_and_continues() {
    let school = School::new(3);
    let s = admit(&school, false, false, 0);
    assert_eq!(
        school
            .fees()
            .collect(&principal(), collect(&s, 500, "Cash", ""))
            .unwrap()
            .no,
        "PC-0001"
    );
    assert_eq!(
        school
            .fees()
            .collect(&principal(), collect(&s, 300, "Cash", ""))
            .unwrap()
            .no,
        "PC-0002"
    );
    // Change the device code: the new series starts at 0001.
    school.set_device_code("T1");
    assert_eq!(
        school
            .fees()
            .collect(&principal(), collect(&s, 100, "Cash", ""))
            .unwrap()
            .no,
        "T1-0001"
    );
    // Switching back continues the PC series where it left off.
    school.set_device_code("PC");
    assert_eq!(
        school
            .fees()
            .collect(&principal(), collect(&s, 100, "Cash", ""))
            .unwrap()
            .no,
        "PC-0003"
    );
}

#[test]
fn receipt_series_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vidya.db");
    let key = zeroize::Zeroizing::new([5u8; 32]);
    let student_id;
    {
        let db = Arc::new(Db::open(&path, &key).unwrap());
        let services = build_services(db, Mode::Server);
        seed(&services, 3);
        let school = School { services };
        student_id = admit(&school, false, false, 0);
        assert_eq!(
            school
                .fees()
                .collect(&principal(), collect(&student_id, 500, "Cash", ""))
                .unwrap()
                .no,
            "PC-0001"
        );
    }
    let db = Arc::new(Db::open(&path, &key).unwrap());
    let services = build_services(db, Mode::Server);
    let school = School { services };
    assert_eq!(
        school
            .fees()
            .collect(&principal(), collect(&student_id, 500, "Cash", ""))
            .unwrap()
            .no,
        "PC-0002"
    );
}

#[test]
fn upi_reference_may_not_be_reused_while_active() {
    let school = School::new(3);
    let s = admit(&school, false, false, 0);
    // A malformed UPI id is refused.
    let err = school
        .fees()
        .collect(&principal(), collect(&s, 100, "UPI", "123"))
        .expect_err("bad upi");
    assert_eq!(err.field.as_deref(), Some("reference"));
    // A valid 12-digit id goes through.
    let first = school
        .fees()
        .collect(&principal(), collect(&s, 100, "UPI", "123456789012"))
        .expect("upi ok");
    // The same id on a second receipt is refused while the first stands.
    let err = school
        .fees()
        .collect(&principal(), collect(&s, 100, "UPI", "123456789012"))
        .expect_err("reused upi");
    assert_eq!(err.message_key, "fees.error.upi_used");
    // After cancelling the first, the id is free again.
    school
        .fees()
        .cancel(&principal(), &first.id, "Wrong entry")
        .expect("cancel");
    assert!(school
        .fees()
        .collect(&principal(), collect(&s, 100, "UPI", "123456789012"))
        .is_ok());
}

#[test]
fn receipts_are_append_only() {
    let school = School::new(3);
    let s = admit(&school, false, false, 0);
    school
        .fees()
        .collect(&principal(), collect(&s, 500, "Cash", ""))
        .unwrap();
    let res = school.services.db.write(|tx| {
        tx.execute("UPDATE receipts SET amount = 1", [])
            .map_err(Into::into)
    });
    assert!(res.is_err(), "trigger should block receipt UPDATE");
}

#[test]
fn cancel_is_principal_only_and_restores_balance() {
    let school = School::new(3);
    let s = admit(&school, false, false, 0);
    let receipt = school
        .fees()
        .collect(&principal(), collect(&s, 500, "Cash", ""))
        .unwrap();
    assert_eq!(school.fees().account(&principal(), &s).unwrap().balance, 8800);

    let err = school
        .fees()
        .cancel(&accountant(), &receipt.id, "Mistaken entry")
        .expect_err("accountant cannot cancel");
    assert_eq!(err.kind, ErrorKind::Permission);

    // Too-short reason is refused.
    let err = school
        .fees()
        .cancel(&principal(), &receipt.id, "no")
        .expect_err("short reason");
    assert_eq!(err.message_key, "fees.error.cancel_reason");

    let cancelled = school
        .fees()
        .cancel(&principal(), &receipt.id, "Duplicate receipt")
        .expect("cancel");
    assert!(cancelled.cancelled.is_some());
    // Balance is restored (paid excludes the cancelled receipt).
    assert_eq!(school.fees().account(&principal(), &s).unwrap().balance, 9300);
    // A second cancel is a conflict.
    let err = school
        .fees()
        .cancel(&principal(), &receipt.id, "Again please")
        .expect_err("already");
    assert_eq!(err.message_key, "fees.error.already_cancelled");
}

#[test]
fn day_book_totals_per_mode_and_flags_cancelled() {
    let school = School::new(3);
    let s = admit(&school, false, false, 0);
    school
        .fees()
        .collect(&principal(), collect(&s, 500, "Cash", ""))
        .unwrap();
    school
        .fees()
        .collect(&principal(), collect(&s, 300, "UPI", "123456789012"))
        .unwrap();
    let cheque = school
        .fees()
        .collect(&principal(), collect(&s, 200, "Cheque", "CHQ 12345 SBI"))
        .unwrap();
    school
        .fees()
        .cancel(&principal(), &cheque.id, "Bounced cheque")
        .unwrap();

    let book = school.fees().day_book(&principal(), "2026-09-20").unwrap();
    let by = |m: &str| book.modes.iter().find(|x| x.mode == m).unwrap().clone();
    assert_eq!(by("Cash").total, 500);
    assert_eq!(by("UPI").total, 300);
    assert_eq!(by("Cheque").total, 0); // the only cheque was cancelled
    assert_eq!(book.total, 800);
    assert_eq!(book.count, 2);
    assert_eq!(book.cancelled_count, 1);
    assert_eq!(book.receipts.len(), 3);
}

#[test]
fn future_day_book_is_refused() {
    let school = School::new(3);
    let err = school
        .fees()
        .day_book(&principal(), "2027-01-01")
        .expect_err("future");
    assert_eq!(err.field.as_deref(), Some("date"));
}

#[test]
fn alerts_are_role_filtered_and_resolvable() {
    let school = School::new(3);
    let s = admit(&school, false, false, 0);
    // Seed one overpayment alert and one backup alert directly (overpayment
    // otherwise only arises from a concession change or a sync merge).
    let overpay_id = school.services.ids.new_id();
    let backup_id = school.services.ids.new_id();
    school
        .services
        .db
        .write(|tx| {
            repo::alerts::insert(
                tx,
                &repo::alerts::AlertRow {
                    id: overpay_id.clone(),
                    kind: "overpayment".into(),
                    entity: "student".into(),
                    entity_id: s.clone(),
                    message_key: "alerts.overpayment".into(),
                    params_json: "{}".into(),
                    created_at: NOW.into(),
                    resolved_at: None,
                },
            )?;
            repo::alerts::insert(
                tx,
                &repo::alerts::AlertRow {
                    id: backup_id.clone(),
                    kind: "backup_overdue".into(),
                    entity: "backup".into(),
                    entity_id: "x".into(),
                    message_key: "alerts.backup".into(),
                    params_json: "{}".into(),
                    created_at: NOW.into(),
                    resolved_at: None,
                },
            )?;
            Ok(())
        })
        .unwrap();

    // The principal sees both; the accountant sees overpayment only.
    assert_eq!(school.fees().list_alerts(&principal()).unwrap().len(), 2);
    let acc = school.fees().list_alerts(&accountant()).unwrap();
    assert_eq!(acc.len(), 1);
    assert_eq!(acc[0].kind, "overpayment");

    // Resolving removes it from the list; resolving again is not found.
    school.fees().resolve_alert(&principal(), &overpay_id).unwrap();
    assert_eq!(school.fees().list_alerts(&principal()).unwrap().len(), 1);
    let err = school
        .fees()
        .resolve_alert(&principal(), &overpay_id)
        .expect_err("already resolved");
    assert_eq!(err.kind, ErrorKind::NotFound);
}
