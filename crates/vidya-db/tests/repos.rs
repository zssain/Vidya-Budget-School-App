//! Inserts one row per repository group into an encrypted in-memory database
//! and reads it back, exercising every repo module (P2.5 Task 5).

use vidya_db::repo::{
    app_settings, attendance, cancellations, classes, enrollments, exams, fee_plans, grade_scale, license,
    marks, meta, receipts, school, sessions, students, subjects, users,
};
use vidya_db::Db;

const HLC: &str = "0000000001000-000001-VD";
const NOW: &str = "2026-09-20T12:00:00Z";
const DATE: &str = "2026-09-20";

#[test]
fn insert_and_read_back_each_table() {
    let db = Db::open_in_memory_for_tests().expect("open db");

    db.write(|tx| {
        vidya_db::seed_defaults(tx, NOW, HLC)?;

        meta::set(tx, "device_id", "VD-TEST")?;
        assert_eq!(meta::next_counter(tx, "receipt:PC")?, 1);

        school::upsert(
            tx,
            &school::SchoolRow {
                name: "Vaani Public School".into(),
                address: "Main Road".into(),
                udise: String::new(),
                board: "State Board".into(),
                phone: String::new(),
            },
            NOW,
            HLC,
        )?;

        sessions::insert(
            tx,
            &sessions::SessionRow {
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

        classes::insert_class(
            tx,
            &classes::ClassRow {
                id: "c5".into(),
                name: "V".into(),
                sort_order: 5,
                active: true,
            },
            HLC,
        )?;
        classes::insert_section(
            tx,
            &classes::SectionRow {
                id: "sec5a".into(),
                class_id: "c5".into(),
                name: "A".into(),
                active: true,
            },
            HLC,
        )?;

        fee_plans::upsert(
            tx,
            &fee_plans::FeePlanRow {
                session_id: "ses1".into(),
                class_id: "c5".into(),
                tuition: 2400,
                exam: 300,
                other: 400,
            },
            HLC,
        )?;

        subjects::insert(
            tx,
            &subjects::SubjectRow {
                id: "sub1".into(),
                class_id: "c5".into(),
                name: "Hindi".into(),
                sort_order: 1,
                active: true,
            },
            HLC,
        )?;

        exams::insert(
            tx,
            &exams::ExamRow {
                id: "ex1".into(),
                session_id: "ses1".into(),
                name: "Unit Test 1".into(),
                max_marks: 25,
                sort_order: 1,
                active: true,
            },
            HLC,
        )?;

        users::insert(
            tx,
            &users::UserRow {
                id: "u1".into(),
                username: "sierra".into(),
                name: "Sierra D'Souza".into(),
                role: "teacher".into(),
                mobile: String::new(),
                password_hash: "$argon2id$dummy".into(),
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
        users::set_sections(tx, "u1", &["sec5a".into()])?;

        students::insert(
            tx,
            &students::StudentRow {
                id: "st1".into(),
                adm_no: "ADM/0001".into(),
                name: "Aman Yadav".into(),
                gender: "Male".into(),
                dob: Some("2015-05-01".into()),
                father: "Raj Yadav".into(),
                mother: String::new(),
                mobile: "9876543210".into(),
                category: "General".into(),
                locality: String::new(),
                aadhaar_collected: false,
                apaar_created: false,
                admitted_on: DATE.into(),
            },
            NOW,
            HLC,
        )?;

        enrollments::insert(
            tx,
            &enrollments::EnrollmentRow {
                id: "en1".into(),
                student_id: "st1".into(),
                session_id: "ses1".into(),
                class_id: "c5".into(),
                section_id: "sec5a".into(),
                roll: 1,
                rte: false,
                transport: false,
                concession: 0,
                status: "active".into(),
            },
            HLC,
        )?;

        receipts::insert(
            tx,
            &receipts::ReceiptRow {
                id: "r1".into(),
                receipt_no: "PC-0001".into(),
                session_id: "ses1".into(),
                student_id: "st1".into(),
                amount: 500,
                mode: "Cash".into(),
                reference: String::new(),
                note: String::new(),
                paid_on: DATE.into(),
                created_by: "u1".into(),
                device_code: "PC".into(),
                balance_after: 11500,
                hlc: HLC.into(),
            },
            NOW,
        )?;
        cancellations::insert(tx, "r1", "u1", NOW, "duplicate", HLC)?;

        attendance::insert_day(
            tx,
            &attendance::AttendanceDayRow {
                id: "ad1".into(),
                session_id: "ses1".into(),
                section_id: "sec5a".into(),
                date: DATE.into(),
                saved_by: "u1".into(),
                saved_at: NOW.into(),
            },
            HLC,
        )?;
        attendance::insert_mark(tx, "ad1", "st1", "P")?;

        marks::upsert(
            tx,
            &marks::MarkRow {
                id: "m1".into(),
                exam_id: "ex1".into(),
                student_id: "st1".into(),
                subject_id: "sub1".into(),
                value: Some(20),
                absent: false,
                entered_by: "u1".into(),
                entered_at: NOW.into(),
            },
            NOW,
            HLC,
        )?;

        license::upsert(
            tx,
            &license::LicenseRow {
                school_code: "vaani".into(),
                activation_code: "DEMO".into(),
                payload_b64: "AA".into(),
                signature_b64: "AA".into(),
                device_id: "VD-DEMO-DEMO-DEMO".into(),
                max_users: 40,
                max_devices: 5,
                license_type: 0,
                issued_on: DATE.into(),
                activated_at: NOW.into(),
            },
        )?;

        Ok(())
    })
    .expect("all inserts");

    db.read(|conn| {
        assert_eq!(school::get(conn)?.expect("school").name, "Vaani Public School");
        assert_eq!(sessions::current(conn)?.expect("session").name, "2026-27");
        assert_eq!(classes::list_active_classes(conn)?.len(), 1);
        assert_eq!(classes::list_active_sections(conn, "c5")?.len(), 1);
        assert_eq!(fee_plans::list_for_session(conn, "ses1")?.len(), 1);
        assert_eq!(subjects::list_active(conn, "c5")?.len(), 1);
        assert_eq!(exams::list_active(conn, "ses1")?.len(), 1);
        assert_eq!(grade_scale::list(conn)?.len(), 5);
        assert!(app_settings::all(conn)?.contains_key("session_timeout_minutes"));
        assert_eq!(
            users::get_by_username(conn, "sierra")?.expect("user").name,
            "Sierra D'Souza"
        );
        assert_eq!(users::sections_for(conn, "u1")?.len(), 1);
        assert_eq!(students::get(conn, "st1")?.expect("student").adm_no, "ADM/0001");
        assert_eq!(
            enrollments::get_for_student(conn, "st1", "ses1")?
                .expect("enrollment")
                .roll,
            1
        );
        assert_eq!(
            enrollments::list_active_in_section(conn, "ses1", "sec5a")?.len(),
            1
        );
        // The only receipt is cancelled, so the paid sum is 0.
        assert_eq!(receipts::sum_paid(conn, "ses1", "st1")?, 0);
        assert!(cancellations::exists(conn, "r1")?);
        assert_eq!(
            attendance::get_day(conn, "sec5a", DATE)?.expect("day").saved_by,
            "u1"
        );
        assert_eq!(marks::list_for_exam(conn, "ex1")?.len(), 1);
        assert_eq!(license::get(conn)?.expect("license").school_code, "vaani");
        assert_eq!(meta::get(conn, "device_id")?.as_deref(), Some("VD-TEST"));
        Ok(())
    })
    .expect("all reads");
}
