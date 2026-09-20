//! Change-log writer tests: it records clean changes and refuses any payload
//! that would leak a secret.

use std::sync::Arc;

use chrono::{NaiveDate, TimeZone, Utc};
use serde_json::json;
use vidya_core::error::ErrorKind;
use vidya_db::Db;
use vidya_services::change_log::{record, ChangeMeta, ChangeRecord, Op};
use vidya_services::env::{FixedClock, SeededRandom, SeqIds};
use vidya_services::error::ServiceError;
use vidya_services::{Mode, Services};

fn test_services() -> Services {
    let db = Arc::new(Db::open_in_memory_for_tests().expect("open test db"));
    let clock = Arc::new(FixedClock {
        now: Utc.with_ymd_and_hms(2026, 9, 20, 12, 0, 0).unwrap(),
        today: NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
    });
    Services::new(
        db,
        clock,
        Arc::new(SeqIds::new("id-")),
        Arc::new(SeededRandom::new(1)),
        Mode::Server,
        "VD-TEST".to_owned(),
    )
    .expect("build services")
}

/// Runs `record` inside a write transaction and returns its result.
fn record_in_tx(services: &Services, rec: ChangeRecord<'_>) -> Result<ChangeMeta, ServiceError> {
    let mut outcome = None;
    services
        .db
        .write(|tx| {
            outcome = Some(record(services, tx, None, rec));
            Ok(())
        })
        .expect("write transaction");
    outcome.expect("record ran")
}

#[test]
fn records_a_clean_change() {
    let services = test_services();
    let meta = record_in_tx(
        &services,
        ChangeRecord {
            kind: "stu",
            entity: "student",
            entity_id: "s1",
            op: Op::Insert,
            summary_key: "students.log.admitted",
            params: json!({ "name": "Aman" }),
            payload: json!({ "name": "Aman", "father": "Raj" }),
        },
    )
    .expect("clean change recorded");

    assert_eq!(meta.change_id, "VD-TEST:1");
    assert!(meta.seq >= 1);
    assert!(!meta.hlc.is_empty());
}

#[test]
fn refuses_payload_with_password_hash() {
    let services = test_services();
    let error = record_in_tx(
        &services,
        ChangeRecord {
            kind: "user",
            entity: "user",
            entity_id: "u1",
            op: Op::Insert,
            summary_key: "users.log.created",
            params: json!({}),
            // A nested passwordHash must be refused at any depth.
            payload: json!({ "name": "Sierra", "credentials": { "passwordHash": "$argon2id$abc" } }),
        },
    )
    .expect_err("payload with a secret must be refused");

    assert_eq!(error.kind, ErrorKind::Internal);
}
