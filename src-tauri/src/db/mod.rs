//! Encrypted SQLite (SQLCipher) database: open, migrate, and low-level helpers.
//! Repository functions are parameterised only (no string-built SQL with values).

use rusqlite::Connection;
use std::path::Path;

/// Ordered migrations. Each is applied in its own transaction; a failure leaves
/// the previous schema_version intact (prompts/P02 Step 3.2).
pub const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("migrations/0001_init.sql")),
    (2, include_str!("migrations/0002_p03.sql")),
    (3, include_str!("migrations/0003_p05.sql")),
    (4, include_str!("migrations/0004_p08.sql")),
    (5, include_str!("migrations/0005_v2_attendance_pa.sql")),
    (6, include_str!("migrations/0006_v2_modules.sql")),
    (7, include_str!("migrations/0007_v2_drive_accounts.sql")),
    (8, include_str!("migrations/0008_v2_calendar.sql")),
    (9, include_str!("migrations/0009_v2_guardians.sql")),
    (10, include_str!("migrations/0010_v2_ledger.sql")),
    (11, include_str!("migrations/0011_v2_numbering.sql")),
    (12, include_str!("migrations/0012_v2_request_types.sql")),
];

// A per-thread frozen clock for deterministic tests. Compiled ONLY in debug
// builds, so the hook can never reach a release artifact; release `now_iso`
// always reads the real clock. libtest runs each `#[test]` on its own thread,
// so a value set here never leaks between tests.
#[cfg(debug_assertions)]
thread_local! {
    static TEST_NOW: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Freeze (`Some`) or release (`None`) this thread's clock. Debug-only test hook.
#[cfg(debug_assertions)]
pub fn set_test_now(iso: Option<String>) {
    TEST_NOW.with(|c| *c.borrow_mut() = iso);
}

/// Current UTC time as an RFC-3339 string (src-tauri may read the clock).
pub fn now_iso() -> String {
    #[cfg(debug_assertions)]
    if let Some(frozen) = TEST_NOW.with(|c| c.borrow().clone()) {
        return frozen;
    }
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

fn apply_key(conn: &Connection, key_hex: &str) -> rusqlite::Result<()> {
    // SQLCipher: the key MUST be the first statement on the connection.
    conn.execute_batch(&format!("PRAGMA key = \"x'{key_hex}'\";"))
}

/// Open (or create) the encrypted DB file at `path`. Sets the key first, then
/// WAL / foreign_keys / busy_timeout. Does NOT run migrations.
pub fn open_encrypted(path: &Path, key_hex: &str) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    apply_key(&conn, key_hex)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;\
         PRAGMA foreign_keys = ON;\
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(conn)
}

/// Open an in-memory encrypted DB (ephemeral; tests).
pub fn open_in_memory(key_hex: &str) -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    apply_key(&conn, key_hex)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(conn)
}

/// Open an encrypted DB file **read-only** (verification / restore inspection).
/// Sets the key first, then opens read-only — so it never switches the file to
/// WAL or otherwise mutates it (a backup must not change when it is verified).
pub fn open_encrypted_readonly(path: &Path, key_hex: &str) -> rusqlite::Result<Connection> {
    use rusqlite::OpenFlags;
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    apply_key(&conn, key_hex)?;
    Ok(conn)
}

/// True if the connection can actually read (key correct). SQLCipher reports
/// "file is not a database" for a wrong key.
pub fn is_readable(conn: &Connection) -> bool {
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0))
        .is_ok()
}

/// Apply pending migrations; returns the resulting schema version.
pub fn run_migrations(conn: &mut Connection) -> rusqlite::Result<i64> {
    let current: i64 = conn
        .query_row("SELECT COALESCE(MAX(version),0) FROM schema_version", [], |r| r.get(0))
        .unwrap_or(0);
    let mut applied = current;
    for (v, sql) in MIGRATIONS {
        if *v > current {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO schema_version(version, applied_at) VALUES (?1, ?2)",
                rusqlite::params![v, now_iso()],
            )?;
            tx.commit()?;
            applied = *v;
        }
    }
    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 64 hex chars = 32 bytes.
    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const OTHER_KEY: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    fn temp_path(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("vidya-test-{tag}-{}-{nanos}.db", std::process::id()))
    }

    #[test]
    fn migrations_apply_and_are_idempotent() {
        let latest = MIGRATIONS.last().map(|(v, _)| *v).unwrap();
        let mut conn = open_in_memory(KEY).unwrap();
        assert_eq!(run_migrations(&mut conn).unwrap(), latest);
        // second run does nothing (already at the latest version)
        assert_eq!(run_migrations(&mut conn).unwrap(), latest);
        let n: i64 = conn
            .query_row("SELECT count(*) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, MIGRATIONS.len() as i64);
    }

    /// P12 upgrade test (DONE-MEANS #4): a v1 database (P01–P08 migrations only)
    /// with a v1 licence upgrades cleanly and the v2 app opens straight to PIN —
    /// no re-activation, no re-setup — while Home now asks to connect the sync
    /// account. Proves the v2 migrations are additive over real v1 data.
    #[test]
    fn v1_database_upgrades_and_app_starts_without_prompts() {
        let mut conn = open_in_memory(KEY).unwrap();
        // Simulate a v1 install: only migrations 1..=4 (through P08) applied.
        for (v, sql) in MIGRATIONS.iter().filter(|(v, _)| *v <= 4) {
            let tx = conn.transaction().unwrap();
            tx.execute_batch(sql).unwrap();
            tx.execute(
                "INSERT INTO schema_version(version, applied_at) VALUES (?1, 't')",
                rusqlite::params![v],
            )
            .unwrap();
            tx.commit().unwrap();
        }
        // A finished v1 setup: school + perpetual v1 licence (active) + setup done.
        conn.execute(
            "INSERT INTO school(id,name,backup_salt,created_at,updated_at) VALUES ('sch','S',x'00','t','t')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO licence(licence_id,school_id,plan,issued_at,signature,raw_json,status) \
             VALUES ('lic','sch','perpetual','t','sig','{}','active')",
            [],
        )
        .unwrap();
        crate::kv::set(&conn, crate::state::KV_SETUP_STEP, &crate::state::SETUP_STEPS).unwrap();

        // Upgrade: the v2 migrations (5,6,7) apply on top of the v1 data.
        let latest = MIGRATIONS.last().map(|(v, _)| *v).unwrap();
        assert_eq!(run_migrations(&mut conn).unwrap(), latest);

        // v1 data survived; the 0007 objects now exist.
        let name: String = conn.query_row("SELECT name FROM school", [], |r| r.get(0)).unwrap();
        assert_eq!(name, "S");
        let status: String = conn.query_row("SELECT status FROM licence", [], |r| r.get(0)).unwrap();
        assert_eq!(status, "active");
        assert!(conn.query_row("SELECT count(*) FROM drive_account", [], |r| r.get::<_, i64>(0)).is_ok());
        let _pub: Option<Vec<u8>> = conn.query_row("SELECT server_key_pub FROM school", [], |r| r.get(0)).unwrap();

        // The app opens to PIN (Locked) with the licence still active — no prompt.
        let resp = crate::state::compute(&conn, None).unwrap();
        assert_eq!(resp.state, crate::state::AppState::Locked);
        assert_eq!(resp.licence_status.as_deref(), Some("active"));
        // Home now asks to connect the school sync account (no sync row yet).
        assert!(crate::drive_account::needs_sync_account(&conn).unwrap());
    }

    /// P13 Step 2 backfill: existing (v1) students with inline guardian columns
    /// are migrated into the guardian table, and siblings that share the same
    /// (mobile, name) map to ONE guardian row (report counts). Runs migrations
    /// through P13-Step-1 (v8), inserts realistic students, then applies the
    /// guardian migration (v9) on top.
    #[test]
    fn guardian_backfill_dedupes_siblings() {
        let mut conn = open_in_memory(KEY).unwrap();
        // Apply everything up to (but not including) the guardian migration.
        for (v, sql) in MIGRATIONS.iter().filter(|(v, _)| *v <= 8) {
            let tx = conn.transaction().unwrap();
            tx.execute_batch(sql).unwrap();
            tx.execute("INSERT INTO schema_version(version, applied_at) VALUES (?1, 't')", rusqlite::params![v]).unwrap();
            tx.commit().unwrap();
        }
        conn.execute("INSERT INTO school(id,name,backup_salt,created_at,updated_at) VALUES ('sch','S',x'00','t','t')", []).unwrap();
        // Two siblings share the SAME guardian (mobile + name); a third student has
        // a different guardian; a fourth has only a name; a fifth has no guardian.
        let students = [
            ("s1", "Riya",  Some("Ramesh Kumar"), Some("9876543210")),
            ("s2", "Rahul", Some("Ramesh Kumar"), Some("9876543210")),
            ("s3", "Anil",  Some("Suresh Rao"),   Some("9811111111")),
            ("s4", "Zoya",  Some("Farah Khan"),   None),
            ("s5", "Om",    None,                 None),
        ];
        for (id, name, gname, gmob) in students {
            conn.execute(
                "INSERT INTO student(id,name,guardian_name,guardian_mobile,created_at,updated_at) VALUES (?1,?2,?3,?4,'t','t')",
                rusqlite::params![id, name, gname, gmob],
            )
            .unwrap();
        }
        // Apply the guardian migration (v9).
        assert_eq!(run_migrations(&mut conn).unwrap(), MIGRATIONS.last().map(|(v, _)| *v).unwrap());

        // 3 distinct guardians: (Ramesh,9876…) shared by s1+s2, (Suresh,981…), (Farah,none).
        let guardians: i64 = conn.query_row("SELECT COUNT(*) FROM guardian", [], |r| r.get(0)).unwrap();
        assert_eq!(guardians, 3, "siblings share one guardian; s5 has none");
        // s1 and s2 point at the same guardian.
        let g1: String = conn.query_row("SELECT guardian_id FROM student_guardian WHERE student_id='s1'", [], |r| r.get(0)).unwrap();
        let g2: String = conn.query_row("SELECT guardian_id FROM student_guardian WHERE student_id='s2'", [], |r| r.get(0)).unwrap();
        assert_eq!(g1, g2, "same mobile+name → same guardian row");
        // Each backfilled link is primary.
        let primary: i64 = conn.query_row("SELECT COUNT(*) FROM student_guardian WHERE is_primary=1", [], |r| r.get(0)).unwrap();
        assert_eq!(primary, 4, "s1..s4 each get a primary guardian; s5 gets none");
        // s5 has no guardian link.
        let s5: i64 = conn.query_row("SELECT COUNT(*) FROM student_guardian WHERE student_id='s5'", [], |r| r.get(0)).unwrap();
        assert_eq!(s5, 0);
        // The legacy columns are untouched.
        let legacy: String = conn.query_row("SELECT guardian_name FROM student WHERE id='s1'", [], |r| r.get(0)).unwrap();
        assert_eq!(legacy, "Ramesh Kumar");
    }

    /// P13 Step 5: the request-table recreate (0012) keeps existing request rows
    /// and now accepts the new v2 types (leave / attendance_duty / class_notice).
    #[test]
    fn request_type_rebuild_preserves_rows_and_accepts_new_types() {
        let mut conn = open_in_memory(KEY).unwrap();
        for (v, sql) in MIGRATIONS.iter().filter(|(v, _)| *v <= 11) {
            let tx = conn.transaction().unwrap();
            tx.execute_batch(sql).unwrap();
            tx.execute("INSERT INTO schema_version(version, applied_at) VALUES (?1, 't')", rusqlite::params![v]).unwrap();
            tx.commit().unwrap();
        }
        conn.execute("INSERT INTO school(id,name,backup_salt,created_at,updated_at) VALUES ('sch','S',x'00','t','t')", []).unwrap();
        conn.execute("INSERT INTO staff(id,name,role,created_at,updated_at) VALUES ('st1','Meena','teacher','t','t')", []).unwrap();
        // A pre-rebuild request of an existing type (marks_correction).
        conn.execute(
            "INSERT INTO request(id,type,target_table,target_id,base_version,reason,requested_by,created_at,updated_at) \
             VALUES ('rq1','marks_correction','mark_entry','m1',1,'fix mark','st1','t','t')",
            [],
        )
        .unwrap();
        // Before the rebuild the new type is rejected by the old CHECK.
        assert!(conn
            .execute(
                "INSERT INTO request(id,type,target_table,target_id,base_version,reason,requested_by,created_at,updated_at) \
                 VALUES ('rq2','leave','staff','st1',0,'casual leave','st1','t','t')",
                [],
            )
            .is_err());

        // Apply 0012.
        assert_eq!(run_migrations(&mut conn).unwrap(), MIGRATIONS.last().map(|(v, _)| *v).unwrap());

        // The existing request survived.
        let t: String = conn.query_row("SELECT type FROM request WHERE id='rq1'", [], |r| r.get(0)).unwrap();
        assert_eq!(t, "marks_correction");
        // A new-type request now inserts.
        conn.execute(
            "INSERT INTO request(id,type,target_table,target_id,base_version,reason,requested_by,created_at,updated_at) \
             VALUES ('rq2','leave','staff','st1',0,'casual leave','st1','t','t')",
            [],
        )
        .unwrap();
        // But a genuinely unknown type is still rejected.
        assert!(conn
            .execute(
                "INSERT INTO request(id,type,target_table,target_id,base_version,reason,requested_by,created_at,updated_at) \
                 VALUES ('rq3','nope','staff','st1',0,'x','st1','t','t')",
                [],
            )
            .is_err());
    }

    #[test]
    fn database_file_is_unreadable_without_the_key() {
        let path = temp_path("enc");
        {
            let mut conn = open_encrypted(&path, KEY).unwrap();
            run_migrations(&mut conn).unwrap();
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").ok();
        }
        // Reopen with the WRONG key → cannot read (open may error on the first
        // real read, or succeed-but-unreadable; both prove it is not readable).
        match open_encrypted(&path, OTHER_KEY) {
            Ok(conn) => assert!(!is_readable(&conn), "wrong key must NOT read the database"),
            Err(_) => { /* SQLCipher rejected the wrong key on first read */ }
        }
        // Reopen with NO key at all → cannot read.
        {
            let conn = Connection::open(&path).unwrap();
            assert!(!is_readable(&conn), "no key must NOT read the database");
        }
        // Reopen with the RIGHT key → reads fine.
        {
            let conn = open_encrypted(&path, KEY).unwrap();
            assert!(is_readable(&conn), "correct key must read the database");
        }
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
    }

    #[test]
    fn append_only_triggers_raise() {
        let mut conn = open_in_memory(KEY).unwrap();
        run_migrations(&mut conn).unwrap();
        // Isolate the triggers from FK checks for this unit test.
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
        let now = now_iso();

        // audit_log
        conn.execute(
            "INSERT INTO audit_log(seq, at, action, prev_hash, hash) VALUES (1, ?1, 'x', '', 'h')",
            rusqlite::params![now],
        )
        .unwrap();
        assert!(conn
            .execute("UPDATE audit_log SET action='y' WHERE seq=1", [])
            .is_err());
        assert!(conn.execute("DELETE FROM audit_log WHERE seq=1", []).is_err());

        // payment
        conn.execute(
            "INSERT INTO payment(id, receipt_no, student_id, amount_paise, mode, collected_by, collected_at, created_at, updated_at) \
             VALUES ('p1','R-A2-0001','s1', 1000, 'cash', 'st1', ?1, ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        assert!(conn
            .execute("UPDATE payment SET amount_paise=2 WHERE id='p1'", [])
            .is_err());
        assert!(conn.execute("DELETE FROM payment WHERE id='p1'", []).is_err());

        // reversal
        conn.execute(
            "INSERT INTO reversal(id, payment_id, reason, applied_at) VALUES ('r1','p1','dup', ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        assert!(conn
            .execute("UPDATE reversal SET reason='x' WHERE id='r1'", [])
            .is_err());
        assert!(conn.execute("DELETE FROM reversal WHERE id='r1'", []).is_err());
    }

    #[test]
    fn fts_finds_devanagari_names() {
        let mut conn = open_in_memory(KEY).unwrap();
        run_migrations(&mut conn).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
        let now = now_iso();
        conn.execute(
            "INSERT INTO student(id, name, created_at, updated_at) VALUES ('s1', 'रिया वर्मा', ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        let hit: i64 = conn
            .query_row(
                "SELECT count(*) FROM student_fts WHERE student_fts MATCH 'रिया'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hit, 1, "Devanagari name must be FTS-searchable");
    }
}
