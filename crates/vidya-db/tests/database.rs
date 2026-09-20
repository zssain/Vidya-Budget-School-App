use std::fs;

use vidya_db::{Db, DbError};
use zeroize::Zeroizing;

#[test]
fn encrypted_on_disk_and_key_validation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("school.db");
    let key = Zeroizing::new([7u8; 32]);
    let db = Db::open(&path, &key).expect("open encrypted database");
    db.write(|tx| {
        tx.execute("INSERT INTO meta (key, value) VALUES ('probe', 'saved')", [])?;
        Ok(())
    })
    .expect("write a row");
    drop(db);

    let bytes = fs::read(&path).expect("read encrypted database");
    assert_ne!(&bytes[..16], b"SQLite format 3\0");
    let reopened = Db::open(&path, &key).expect("reopen with same key");
    let saved: String = reopened
        .read(|connection| {
            connection
                .query_row("SELECT value FROM meta WHERE key = 'probe'", [], |row| row.get(0))
                .map_err(DbError::from)
        })
        .expect("read saved row");
    assert_eq!(saved, "saved");
    drop(reopened);

    let wrong_key = Zeroizing::new([8u8; 32]);
    assert!(matches!(Db::open(&path, &wrong_key), Err(DbError::WrongKey)));
}

#[test]
fn plaintext_database_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("plain.db");
    let connection = rusqlite::Connection::open(&path).expect("open plaintext SQLite");
    connection
        .execute_batch("CREATE TABLE probe (id INTEGER PRIMARY KEY);")
        .expect("create plaintext table");
    drop(connection);
    assert!(matches!(
        Db::open(&path, &Zeroizing::new([1u8; 32])),
        Err(DbError::NotEncrypted)
    ));
}

#[test]
fn migration_is_idempotent_and_wal_enabled() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("school.db");
    let key = Zeroizing::new([3u8; 32]);
    let first = Db::open(&path, &key).expect("first open");
    drop(first);
    let second = Db::open(&path, &key).expect("second open");
    second
        .read(|connection| {
            let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
            let tables: i64 = connection.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table'",
                [],
                |row| row.get(0),
            )?;
            let journal: String = connection.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
            let foreign_keys: i64 = connection.pragma_query_value(None, "foreign_keys", |row| row.get(0))?;
            let busy_timeout: i64 = connection.pragma_query_value(None, "busy_timeout", |row| row.get(0))?;
            let expected = include_str!("../migrations/0001_init.sql")
                .matches("CREATE TABLE ")
                .count()
                + 1; // sqlite_sequence for the AUTOINCREMENT change log
            assert_eq!(expected, 36);
            assert_eq!(tables, expected as i64);
            assert_eq!(version, 1);
            assert_eq!(journal, "wal");
            assert_eq!(foreign_keys, 1);
            assert_eq!(busy_timeout, 5000);
            Ok(())
        })
        .expect("inspect migration");
}

#[test]
fn newer_version_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("school.db");
    let key = Zeroizing::new([4u8; 32]);
    let db = Db::open(&path, &key).expect("open");
    db.write(|tx| {
        tx.pragma_update(None, "user_version", 99)?;
        Ok(())
    })
    .expect("write newer version");
    drop(db);
    assert!(matches!(
        Db::open(&path, &key),
        Err(DbError::Migration {
            version: 99,
            message
        }) if message == "db.error.newer_version"
    ));
}

#[test]
fn write_error_rolls_back() {
    let db = Db::open_in_memory_for_tests().expect("memory database");
    let result: Result<(), DbError> = db.write(|tx| {
        tx.execute("INSERT INTO meta (key, value) VALUES ('rollback', 'yes')", [])?;
        Err(DbError::Pool("intentional failure".to_owned()))
    });
    assert!(result.is_err());
    db.read(|connection| {
        let count: i64 =
            connection.query_row("SELECT count(*) FROM meta WHERE key = 'rollback'", [], |row| {
                row.get(0)
            })?;
        assert_eq!(count, 0);
        Ok(())
    })
    .expect("read after rollback");
}

#[test]
fn write_panic_rolls_back() {
    let db = Db::open_in_memory_for_tests().expect("memory database");
    let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: Result<(), DbError> = db.write(|tx| {
            tx.execute("INSERT INTO meta (key, value) VALUES ('panic', 'yes')", [])?;
            panic!("intentional test panic");
        });
    }));
    assert!(panic_result.is_err());
    db.read(|connection| {
        let count: i64 =
            connection.query_row("SELECT count(*) FROM meta WHERE key = 'panic'", [], |row| {
                row.get(0)
            })?;
        assert_eq!(count, 0);
        Ok(())
    })
    .expect("read after panic rollback");
}

#[test]
fn defaults_are_idempotent_and_not_seeded_on_open() {
    let db = Db::open_in_memory_for_tests().expect("memory database");
    db.read(|connection| {
        let count: i64 = connection.query_row("SELECT count(*) FROM grade_scale", [], |row| row.get(0))?;
        assert_eq!(count, 0);
        Ok(())
    })
    .expect("read before seed");
    for _ in 0..2 {
        db.write(|tx| vidya_db::seed_defaults(tx, "2026-09-19T00:00:00Z", "hlc"))
            .expect("seed defaults");
    }
    db.read(|connection| {
        let grades: i64 = connection.query_row("SELECT count(*) FROM grade_scale", [], |row| row.get(0))?;
        let counters: i64 = connection.query_row("SELECT count(*) FROM counters", [], |row| row.get(0))?;
        let settings: i64 =
            connection.query_row("SELECT count(*) FROM app_settings", [], |row| row.get(0))?;
        assert_eq!((grades, counters, settings), (5, 3, 8));
        Ok(())
    })
    .expect("read seeded defaults");
}

fn fixture_db() -> Db {
    let db = Db::open_in_memory_for_tests().expect("fixture database");
    db.write(|tx| {
        tx.execute_batch(
            "INSERT INTO academic_sessions (id, name, starts_on, ends_on, is_current, terms, created_at, updated_hlc)
             VALUES ('s1', '2026-27', '2026-04-01', '2027-03-31', 1, 4, 'now', 'hlc');
             INSERT INTO classes (id, name, sort_order, updated_hlc) VALUES ('c1', 'I', 1, 'hlc');
             INSERT INTO sections (id, class_id, name, updated_hlc) VALUES ('sec1', 'c1', 'A', 'hlc');
             INSERT INTO users (id, username, name, role, password_hash, created_at, updated_hlc)
             VALUES ('u1', 'sunita', 'Sunita', 'principal', 'test-hash', 'now', 'hlc');
             INSERT INTO students (id, adm_no, name, gender, father, mobile, category, admitted_on, created_at, updated_at, updated_hlc)
             VALUES ('st1', 'ADM/0001', 'One', 'Female', 'Parent', '9876543210', 'General', '2026-04-01', 'now', 'now', 'hlc');
             INSERT INTO students (id, adm_no, name, gender, father, mobile, category, admitted_on, created_at, updated_at, updated_hlc)
             VALUES ('st2', 'ADM/0002', 'Two', 'Male', 'Parent', '9876543211', 'General', '2026-04-01', 'now', 'now', 'hlc');
             INSERT INTO enrollments (id, student_id, session_id, class_id, section_id, roll, updated_hlc)
             VALUES ('e1', 'st1', 's1', 'c1', 'sec1', 1, 'hlc');
             INSERT INTO subjects (id, class_id, name, sort_order, updated_hlc)
             VALUES ('sub1', 'c1', 'Math', 1, 'hlc');
             INSERT INTO exams (id, session_id, name, max_marks, sort_order, updated_hlc)
             VALUES ('ex1', 's1', 'Annual', 100, 1, 'hlc');",
        )?;
        Ok(())
    })
    .expect("insert fixture rows");
    db
}

fn assert_rejected(db: &Db, statement: &str) {
    let result = db.write(|tx| {
        tx.execute_batch(statement)?;
        Ok(())
    });
    assert!(matches!(result, Err(DbError::Sqlite(_))), "{statement}");
}

#[test]
fn schema_constraints_reject_invalid_rows() {
    let db = fixture_db();
    assert_rejected(
        &db,
        "INSERT INTO academic_sessions (id, name, starts_on, ends_on, is_current, terms, created_at, updated_hlc)
         VALUES ('s2', '2027-28', '2027-04-01', '2028-03-31', 1, 4, 'now', 'hlc');",
    );
    assert_rejected(
        &db,
        "INSERT INTO users (id, username, name, role, password_hash, created_at, updated_hlc)
         VALUES ('u2', 'other', 'Other', 'principal', 'test-hash', 'now', 'hlc');",
    );
    for username in ["Bad Name", "sierra@x"] {
        let result = db.write(|tx| {
            tx.execute(
                "INSERT INTO users (id, username, name, role, password_hash, created_at, updated_hlc)
                 VALUES (?1, ?2, 'Other', 'teacher', 'test-hash', 'now', 'hlc')",
                rusqlite::params![username, username],
            )?;
            Ok(())
        });
        assert!(matches!(result, Err(DbError::Sqlite(_))));
    }
    assert_rejected(
        &db,
        "INSERT INTO students (id, adm_no, name, gender, father, mobile, category, admitted_on, created_at, updated_at, updated_hlc)
         VALUES ('st3', 'ADM/0003', 'Three', 'Male', 'Parent', '98765', 'General', '2026-04-01', 'now', 'now', 'hlc');",
    );
    assert_rejected(
        &db,
        "INSERT INTO school (id, name, udise, board, updated_at, updated_hlc)
         VALUES (1, 'Test School', '123', 'CBSE', 'now', 'hlc');",
    );
    assert_rejected(
        &db,
        "INSERT INTO marks (id, exam_id, student_id, subject_id, value, absent, entered_by, entered_at, updated_hlc)
         VALUES ('m1', 'ex1', 'st1', 'sub1', NULL, 0, 'u1', 'now', 'hlc');",
    );
    assert_rejected(
        &db,
        "INSERT INTO enrollments (id, student_id, session_id, class_id, section_id, roll, updated_hlc)
         VALUES ('e2', 'st2', 's1', 'c1', 'sec1', 1, 'hlc');",
    );
}

#[test]
fn accounting_and_change_log_tables_are_append_only() {
    let db = fixture_db();
    db.write(|tx| {
        tx.execute_batch(
            "INSERT INTO receipts (id, receipt_no, session_id, student_id, amount, mode, paid_on, created_at, created_by, device_code, balance_after, hlc)
             VALUES ('r1', 'PC-0001', 's1', 'st1', 100, 'Cash', '2026-04-01', 'now', 'u1', 'PC', 0, 'hlc');
             INSERT INTO receipt_cancellations (receipt_id, cancelled_by, cancelled_at, reason, hlc)
             VALUES ('r1', 'u1', 'now', 'Mistake', 'hlc');
             INSERT INTO transfer_certificates (id, tc_no, student_id, issued_on, issued_by, data_json, hlc)
             VALUES ('tc1', 'TC-0001', 'st1', '2026-04-01', 'u1', '{}', 'hlc');
             INSERT INTO change_log (change_id, hlc, device_id, kind, entity, entity_id, op, summary_key, at)
             VALUES ('d:1', 'hlc', 'd', 'fee', 'receipt', 'r1', 'append', 'log.receipt', 'now');",
        )?;
        Ok(())
    })
    .expect("insert append-only rows");
    for (table, key, value, message) in [
        ("receipts", "id", "r1", "receipts are append-only"),
        (
            "receipt_cancellations",
            "receipt_id",
            "r1",
            "receipt cancellations are append-only",
        ),
        (
            "transfer_certificates",
            "id",
            "tc1",
            "transfer certificates are append-only",
        ),
        ("change_log", "change_id", "d:1", "change log is append-only"),
    ] {
        for verb in ["UPDATE", "DELETE"] {
            let statement = if verb == "UPDATE" {
                format!("UPDATE {table} SET {key} = {key} WHERE {key} = '{value}'")
            } else {
                format!("DELETE FROM {table} WHERE {key} = '{value}'")
            };
            let result = db.write(|tx| {
                tx.execute_batch(&statement)?;
                Ok(())
            });
            assert!(matches!(result, Err(DbError::Sqlite(error)) if error.to_string().contains(message)));
        }
    }
}
