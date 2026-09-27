//! Phase 18 — v1 → v2 upgrade & migration safety (prompts/P18 WORK 2, DONE-MEANS 1).
//!
//! Proves that a real v1.x database — demo-style seed + one term of activity +
//! legacy `L` attendance marks + a v1 licence + an applied payment reversal —
//! survives the v2 migration chain with:
//!   * every money total identical to the paise (per day and per mode),
//!   * the hash-chained audit log still verifying and its head unchanged,
//!   * legacy `L` marks preserved (shown as "Leave (old)" in the UI, P11),
//!   * guardians backfilled with siblings de-duplicated,
//!   * the licence still active (no re-activation prompt),
//!   * balanced vouchers after the idempotent ledger backfill.
//!
//! It also exercises the P13-flagged production caveat: migration 0012 recreates
//! `request` with `DROP TABLE request`, which — on a DB that holds an applied
//! `reversal.request_id → request(id)` link — would fail with foreign keys on.
//! The runner now defers FK checks to COMMIT (db::run_migrations), so the upgrade
//! succeeds; this test would fail without that fix.
//!
//! There is no frozen v1 *binary* in the repo (no v1 release was ever tagged), so
//! the "frozen v1 database" is reconstructed from the last v1 schema — migrations
//! 0001–0004, which are exactly the v1 release schema — and realistic v1 rows.

use rusqlite::Connection;
use vidya_lib::security::audit::{self, AuditEntry};
use vidya_lib::{db, ledger};

const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// Highest v1 migration (through P08). Everything above it is a v2 migration.
const V1_TOP: i64 = 4;

fn temp_path(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("vidya-upgrade-{tag}-{}-{nanos}.db", std::process::id()))
}

/// Apply migrations `1..=through` (each in its own transaction), recording the
/// version — the same shape as the real runner, so a partial run is realistic.
fn apply_through(conn: &mut Connection, through: i64) {
    let current: i64 = conn
        .query_row("SELECT COALESCE(MAX(version),0) FROM schema_version", [], |r| r.get(0))
        .unwrap_or(0);
    conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
    for (v, sql) in db::MIGRATIONS.iter().filter(|(v, _)| *v > current && *v <= through) {
        let tx = conn.transaction().unwrap();
        tx.execute_batch(sql).unwrap();
        tx.execute(
            "INSERT INTO schema_version(version, applied_at) VALUES (?1, 't')",
            rusqlite::params![v],
        )
        .unwrap();
        tx.commit().unwrap();
    }
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
}

/// Build a realistic *finished* v1 install: school + perpetual v1 licence + a
/// term of classes, staff, students (with inline guardian columns incl.
/// siblings), attendance including legacy `L`, confirmed payments across days
/// and modes, and an applied reversal linked to its approval request.
fn seed_v1(conn: &mut Connection) {
    apply_through(conn, V1_TOP);

    // School + a v1 licence (perpetual, active) + a completed setup.
    conn.execute(
        "INSERT INTO school(id,name,backup_salt,created_at,updated_at) VALUES ('sch','Saraswati Public School',x'00','t','t')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO licence(licence_id,school_id,plan,issued_at,signature,raw_json,status) \
         VALUES ('lic','sch','perpetual','2026-04-01','sig','{}','active')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO academic_session(id,label,starts_on,ends_on,is_current) VALUES ('ses','2026–27','2026-04-01','2027-03-31',1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO term(id,session_id,name,starts_on,ends_on) VALUES ('t1','ses','Term 1','2026-04-01','2026-09-30')",
        [],
    )
    .unwrap();

    // Staff: principal + accountant + a teacher (the teacher carries a
    // google_email — the deprecated column P18 removes in a later step).
    conn.execute("INSERT INTO staff(id,name,role,state,created_at,updated_at) VALUES ('stp','Rajesh Kumar','principal','active','t','t')", []).unwrap();
    conn.execute("INSERT INTO staff(id,name,role,state,created_at,updated_at) VALUES ('sta','Anita Sharma','accountant','active','t','t')", []).unwrap();
    conn.execute("INSERT INTO staff(id,name,role,google_email,state,created_at,updated_at) VALUES ('stt','Meena Iyer','teacher','meena@example.com','active','t','t')", []).unwrap();

    // Two classes.
    conn.execute("INSERT INTO class(id,name,section,display,class_teacher_id,sort_order) VALUES ('cA','V','A','V-A','stt',1)", []).unwrap();
    conn.execute("INSERT INTO class(id,name,section,display,sort_order) VALUES ('cB','VII','B','VII-B',2)", []).unwrap();

    // Students with inline guardian columns: s1+s2 siblings (same guardian),
    // s3 distinct guardian, s4 name-only, s5 no guardian.
    let students = [
        ("s1", "Riya Kumar", Some("Ramesh Kumar"), Some("9876543210")),
        ("s2", "Rahul Kumar", Some("Ramesh Kumar"), Some("9876543210")),
        ("s3", "Anil Rao", Some("Suresh Rao"), Some("9811111111")),
        ("s4", "Zoya Khan", Some("Farah Khan"), None),
        ("s5", "Om Verma", None, None),
    ];
    for (id, name, gname, gmob) in students {
        conn.execute(
            "INSERT INTO student(id,admission_no,name,guardian_name,guardian_mobile,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,'t','t','confirmed')",
            rusqlite::params![id, format!("2026/{}", &id[1..]), name, gname, gmob],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO enrollment(id,student_id,class_id,session_id,roll_no,from_date,created_at,updated_at) \
             VALUES (?1,?2,'cA','ses',1,'2026-04-01','t','t')",
            rusqlite::params![format!("e{id}"), id],
        )
        .unwrap();
    }

    // A fee head + a due for two students (so allocations are realistic).
    conn.execute("INSERT INTO fee_head(id,name,amount_paise,frequency) VALUES ('fh','Term 1 Fee',310000,'term')", []).unwrap();
    for id in ["s1", "s3"] {
        conn.execute(
            "INSERT INTO fee_due(id,student_id,fee_head_id,period,amount_paise,created_at,updated_at,sync_state) \
             VALUES (?1,?2,'fh','Term 1',310000,'t','t','confirmed')",
            rusqlite::params![format!("d{id}"), id],
        )
        .unwrap();
    }

    // A term of attendance: three submitted sheets with P/A and some legacy L.
    let days = ["2026-07-01", "2026-07-02", "2026-07-03"];
    for (di, date) in days.iter().enumerate() {
        let sheet = format!("sh{di}");
        conn.execute(
            "INSERT INTO attendance_sheet(id,class_id,date,status,submitted_by,submitted_at,created_at,updated_at,sync_state) \
             VALUES (?1,'cA',?2,'submitted','stt','t','t','t','confirmed')",
            rusqlite::params![sheet, date],
        )
        .unwrap();
        // s1 P, s2 A, s3 L (legacy!), s4 P, s5 P.
        let marks = [("s1", "P"), ("s2", "A"), ("s3", "L"), ("s4", "P"), ("s5", "P")];
        for (sid, mark) in marks {
            conn.execute(
                "INSERT INTO attendance_mark(id,sheet_id,student_id,mark) VALUES (?1,?2,?3,?4)",
                rusqlite::params![format!("m{di}{sid}"), sheet, sid, mark],
            )
            .unwrap();
        }
    }

    // Confirmed payments across two days and two modes (append-only table).
    // Day 1: cash 100000 + upi 50000 ; Day 2: cash 60000 + upi 200000.
    let payments = [
        ("p1", "R-A2-0001", "s1", 100000i64, "cash", "2026-07-01T10:00:00Z"),
        ("p2", "R-A2-0002", "s2", 50000, "upi", "2026-07-01T11:00:00Z"),
        ("p3", "R-A2-0003", "s3", 60000, "cash", "2026-07-02T10:00:00Z"),
        ("p4", "R-A2-0004", "s4", 200000, "upi", "2026-07-02T12:00:00Z"),
    ];
    for (id, rno, sid, amt, mode, at) in payments {
        conn.execute(
            "INSERT INTO payment(id,receipt_no,student_id,amount_paise,mode,collected_by,collected_at,confirmed_at,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,'sta',?6,?6,?6,?6,'confirmed')",
            rusqlite::params![id, rno, sid, amt, mode, at],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO payment_allocation(id,payment_id,amount_paise,kind) VALUES (?1,?2,?3,'due')",
            rusqlite::params![format!("al{id}"), id, amt],
        )
        .unwrap();
    }

    // An approved payment-reversal REQUEST + the applied reversal that links to
    // it — the exact shape that makes 0012's `DROP TABLE request` blow up on FK.
    conn.execute(
        "INSERT INTO request(id,type,target_table,target_id,base_version,reason,requested_by,status,decided_by,decided_at,apply_state,applied_at,created_at,updated_at) \
         VALUES ('rqrev','payment_reversal','payment','p4',1,'wrong student','sta','approved','stp','t','applied','t','t','t')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO reversal(id,payment_id,reason,request_id,approved_by,applied_at) \
         VALUES ('rev1','p4','wrong student','rqrev','stp','2026-07-02T13:00:00Z')",
        [],
    )
    .unwrap();

    // A short, valid audit chain over the money writes.
    {
        let tx = conn.transaction().unwrap();
        for (action, table, rec) in [
            ("create_school", "school", "sch"),
            ("record_payment", "payment", "p1"),
            ("record_payment", "payment", "p4"),
            ("apply_reversal", "reversal", "rev1"),
        ] {
            audit::append(
                &tx,
                &AuditEntry {
                    at: "2026-07-02T13:00:00Z".into(),
                    staff_id: Some("stp".into()),
                    action: action.into(),
                    table: Some(table.into()),
                    record_id: Some(rec.into()),
                    ..Default::default()
                },
            )
            .unwrap();
        }
        tx.commit().unwrap();
    }
}

/// Money total per (calendar day, mode) — the invariant that must not shift by a
/// single paisa across the upgrade.
fn day_mode_totals(conn: &Connection) -> Vec<(String, String, i64)> {
    let mut stmt = conn
        .prepare("SELECT substr(collected_at,1,10), mode, SUM(amount_paise) FROM payment GROUP BY 1,2 ORDER BY 1,2")
        .unwrap();
    let rows = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap();
    rows.collect::<rusqlite::Result<_>>().unwrap()
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap()
}

#[test]
fn v1_database_upgrades_with_money_audit_and_history_intact() {
    let mut conn = db::open_in_memory(KEY).unwrap();
    seed_v1(&mut conn);

    // ---- capture pre-upgrade invariants ------------------------------------
    let pre_students = count(&conn, "SELECT COUNT(*) FROM student");
    let pre_payments = count(&conn, "SELECT COUNT(*) FROM payment");
    let pre_marks = count(&conn, "SELECT COUNT(*) FROM attendance_mark");
    let pre_legacy_l = count(&conn, "SELECT COUNT(*) FROM attendance_mark WHERE mark='L'");
    let pre_receipt_total = count(&conn, "SELECT COALESCE(SUM(amount_paise),0) FROM payment");
    let pre_totals = day_mode_totals(&conn);
    let pre_audit_head = audit::head_hash(&conn).unwrap();
    assert!(audit::verify_chain(&conn).unwrap().is_none(), "v1 audit chain must verify before upgrade");
    assert_eq!(pre_legacy_l, 3, "three legacy L marks seeded");

    // ---- run the v2 migration chain (5..=latest) ---------------------------
    // Without the FK-defer runner fix this panics at 0012 (reversal→request).
    let latest = db::MIGRATIONS.last().map(|(v, _)| *v).unwrap();
    assert_eq!(db::run_migrations(&mut conn).unwrap(), latest, "upgrade reaches the latest schema");

    // ---- money is identical to the paise -----------------------------------
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM payment"), pre_payments, "no payment added or lost");
    assert_eq!(count(&conn, "SELECT COALESCE(SUM(amount_paise),0) FROM payment"), pre_receipt_total);
    assert_eq!(day_mode_totals(&conn), pre_totals, "per-day, per-mode totals unchanged to the paisa");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM reversal"), 1, "the applied reversal survived");

    // ---- audit chain still verifies and its head is unchanged --------------
    assert!(audit::verify_chain(&conn).unwrap().is_none(), "audit chain still verifies after upgrade");
    assert_eq!(audit::head_hash(&conn).unwrap(), pre_audit_head, "migrations never touch the audit log");

    // ---- attendance history preserved (legacy L intact) --------------------
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM attendance_mark"), pre_marks);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM attendance_mark WHERE mark='L'"), pre_legacy_l, "legacy L rows kept for history");

    // ---- guardians backfilled, siblings de-duplicated ----------------------
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM student"), pre_students);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM guardian"), 3, "siblings share one guardian; s5 has none");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM student_guardian WHERE is_primary=1"), 4, "s1..s4 each get a primary guardian");
    let g1: String = conn.query_row("SELECT guardian_id FROM student_guardian WHERE student_id='s1'", [], |r| r.get(0)).unwrap();
    let g2: String = conn.query_row("SELECT guardian_id FROM student_guardian WHERE student_id='s2'", [], |r| r.get(0)).unwrap();
    assert_eq!(g1, g2, "siblings map to the same guardian row");

    // ---- licence still active: the app opens without a re-activation prompt -
    let status: String = conn.query_row("SELECT status FROM licence", [], |r| r.get(0)).unwrap();
    assert_eq!(status, "active");

    // ---- ledger backfill is balanced and covers every confirmed payment ----
    let created = ledger::backfill_vouchers(&mut conn).unwrap();
    assert!(created > 0, "backfill posts vouchers for the confirmed money");
    assert_eq!(ledger::ledger_imbalance(&conn).unwrap(), 0, "every voucher balances (Σdebit = Σcredit)");
    let confirmed_payments = count(&conn, "SELECT COUNT(*) FROM payment WHERE sync_state='confirmed'");
    let receipt_vouchers = count(&conn, "SELECT COUNT(*) FROM voucher WHERE source_table='payment'");
    assert_eq!(receipt_vouchers, confirmed_payments, "one receipt voucher per confirmed payment");
    // Idempotent: a second backfill creates nothing and stays balanced.
    assert_eq!(ledger::backfill_vouchers(&mut conn).unwrap(), 0, "backfill is idempotent");
    assert_eq!(ledger::ledger_imbalance(&conn).unwrap(), 0);
}

#[test]
fn interrupted_upgrade_resumes_from_the_last_committed_migration() {
    // A real file DB so state survives "closing" the connection (a kill).
    let path = temp_path("resume");
    {
        let mut conn = db::open_encrypted(&path, KEY).unwrap();
        seed_v1(&mut conn); // v1 (through migration 4)
                            // Partial upgrade: got as far as migration 15, then the process died.
        apply_through(&mut conn, 15);
        assert_eq!(count(&conn, "SELECT MAX(version) FROM schema_version"), 15);
    } // connection dropped == app killed

    // Next start: the runner resumes 16..=latest and reaches the top cleanly.
    {
        let mut conn = db::open_encrypted(&path, KEY).unwrap();
        let latest = db::MIGRATIONS.last().map(|(v, _)| *v).unwrap();
        assert_eq!(db::run_migrations(&mut conn).unwrap(), latest, "resume reaches latest");
        // The v1 data is still there and the reversal→request rebuild succeeded.
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM payment"), 4);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM reversal"), 1);
        assert!(audit::verify_chain(&conn).unwrap().is_none());
    }

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("db-wal"));
    let _ = std::fs::remove_file(path.with_extension("db-shm"));
}

#[test]
fn a_failed_migration_rolls_back_and_leaves_the_schema_version_intact() {
    // Each migration runs in its own transaction, so a crash *inside* one must
    // leave schema_version at the previous value (nothing half-applied).
    let path = temp_path("rollback");
    {
        let mut conn = db::open_encrypted(&path, KEY).unwrap();
        seed_v1(&mut conn);
        apply_through(&mut conn, 10);
        assert_eq!(count(&conn, "SELECT MAX(version) FROM schema_version"), 10);

        // Simulate a migration that starts, makes a change, then the process is
        // killed before COMMIT: open a transaction, create a table, drop the
        // connection WITHOUT committing.
        conn.execute_batch("BEGIN; CREATE TABLE half_applied(x); ").unwrap();
    } // dropped mid-transaction → SQLite rolls the transaction back

    {
        let mut conn = db::open_encrypted(&path, KEY).unwrap();
        // The half-applied table is gone and the version is still 10.
        assert_eq!(count(&conn, "SELECT MAX(version) FROM schema_version"), 10, "no partial migration recorded");
        let leftover: i64 = conn
            .query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='half_applied'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(leftover, 0, "the uncommitted change rolled back cleanly");
        // And a fresh run completes the upgrade.
        let latest = db::MIGRATIONS.last().map(|(v, _)| *v).unwrap();
        assert_eq!(db::run_migrations(&mut conn).unwrap(), latest);
    }

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("db-wal"));
    let _ = std::fs::remove_file(path.with_extension("db-shm"));
}
