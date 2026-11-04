//! Backup → restore drill onto a clean "replacement PC" (the owner's pre-sale check,
//! docs/00-SYSTEM-CONTEXT.md §12, automated so it can run again).
//!
//! PC A builds a synthetic school (> 1,000 students across many classes, teachers,
//! an accountant, historical attendance sheets, marks, fee dues, payments with
//! allocations + vouchers/ledger, a reversal, requests, devices) through the REAL
//! command-logic layer, then takes a backup through the REAL backup path
//! (`backup::schedule::derive_and_cache` + `run_and_record`: encrypted `.vbak` +
//! `.vbak.meta`, keyed from the recovery key). PC B is a brand-new data dir with no
//! DB: the drill restores through the REAL restore path the `restore_summary` /
//! `restore_install` commands use (`summary_with_recovery` → `backup_key_from_meta`
//! → `install_then`), then verifies the restored school and runs the app's normal
//! boot + reads/writes on it.
//!
//! Also: a wrong recovery key fails without touching anything, and an interrupted
//! install (failed commit / crash mid-swap / stale staged file) leaves the previous
//! live DB usable.
//!
//! Run: `cargo test -p vidya --test restore_drill` (≈ a minute in debug).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use rusqlite::{params, types::ValueRef, Connection};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use vidya_lib::backup::{self, restore, schedule};
use vidya_lib::commands::logic::*;
use vidya_lib::state::{self, AppState, SessionStaff};
use vidya_lib::write::DeviceMode;
use vidya_lib::{db, ledger, roles, security};

const KEY_A: &str = "a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1";
const KEY_B: &str = "b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2";
const KEY_OLD: &str = "c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3";
/// A fake recovery key (format of `security::recovery`), test-only.
const RECOVERY: &str = "QWERT-YUPAS-DFGHJ-KLZXC-VBNMQ-23456";
const WRONG_RECOVERY: &str = "ZZZZZ-ZZZZZ-ZZZZZ-ZZZZZ-ZZZZZ-77777";

const NEW_CLASSES: usize = 8;
const PER_CLASS: usize = 50; // 8 × 50 = 400 new + 670 seeded = 1,070 students
const HISTORY_DAYS: [&str; 6] = ["2026-08-10", "2026-08-11", "2026-08-12", "2026-08-13", "2026-08-14", "2026-08-17"];

fn sess(id: &str, role: &str) -> SessionStaff {
    SessionStaff { id: id.into(), name: id.into(), role: role.into() }
}
fn principal() -> SessionStaff {
    sess("stf-priya", "principal")
}
fn accountant() -> SessionStaff {
    sess("stf-suresh", "accountant")
}

fn tmp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let d = std::env::temp_dir().join(format!("vidya-drill-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

// ---- the synthetic school (PC A) ---------------------------------------------

/// Seed the demo school, then grow it through the real command logic.
fn build_school(conn: &mut Connection) {
    let now = OffsetDateTime::parse("2026-09-23T09:00:00Z", &time::format_description::well_known::Rfc3339).unwrap();
    vidya_lib::seed::seed_demo_school(conn, now).unwrap();
    roles::seed_role_permissions(conn).unwrap();
    // The setup wizard finished on PC A (the debug seed skips the wizard).
    vidya_lib::kv::set(conn, state::KV_SETUP_STEP, &state::SETUP_STEPS).unwrap();
    // A real per-school KDF salt (setup generates one; the seed's is a fixed demo).
    conn.execute("UPDATE school SET backup_salt = x'5a17c0ffee5a17c0ffee5a17c0ffee00'", []).unwrap();

    // More classes (class teachers = the seeded teachers).
    let teachers = ["stf-anita", "stf-meena", "stf-nair"];
    for c in 0..NEW_CLASSES {
        conn.execute(
            "INSERT INTO class(id,name,section,display,class_teacher_id,sort_order) VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                format!("cls-x{c}"),
                format!("{}", 8 + c / 2),
                if c % 2 == 0 { "A" } else { "B" },
                format!("{}-{}", 8 + c / 2, if c % 2 == 0 { "A" } else { "B" }),
                teachers[c % teachers.len()],
                20 + c as i64
            ],
        )
        .unwrap();
    }

    // Admissions in early August (real create_student_logic: admission no., enrolment,
    // fee dues, audit entries).
    db::set_test_now(Some("2026-08-03T09:00:00Z".into()));
    let mut new_students: Vec<(String, String)> = Vec::new(); // (student, class)
    for c in 0..NEW_CLASSES {
        for i in 0..PER_CLASS {
            let s = create_student_logic(
                conn,
                &principal(),
                Some("dev-a1"),
                DeviceMode::Server,
                "2026-08-03",
                &NewStudentInput {
                    name: format!("Drill Student {c}-{i}"),
                    class_id: format!("cls-x{c}"),
                    roll_no: Some(i as i64 + 1),
                    guardian_name: Some(format!("Guardian {c}-{i}")),
                    guardian_mobile: Some(format!("98{:08}", c * 1000 + i)),
                    dob: None,
                    gender: Some(if i % 2 == 0 { "F" } else { "M" }.into()),
                    address: None,
                    transport: Some(i % 4 == 0),
                    rte: Some(false),
                    category: None,
                    aadhaar_status: None,
                },
            )
            .unwrap_or_else(|e| panic!("create student {c}-{i}: {e:?}"));
            new_students.push((s.id, format!("cls-x{c}")));
        }
    }

    // Historical attendance sheets for every new class (real submit path).
    for day in HISTORY_DAYS {
        db::set_test_now(Some(format!("{day}T09:30:00Z")));
        for c in 0..NEW_CLASSES {
            let class = format!("cls-x{c}");
            let marks: Vec<MarkInput> = new_students
                .iter()
                .filter(|(_, cl)| *cl == class)
                .enumerate()
                .map(|(i, (sid, _))| MarkInput {
                    student_id: sid.clone(),
                    mark: if i % 11 == 0 || i % 17 == 0 { "A" } else { "P" }.into(),
                })
                .collect();
            submit_attendance_logic(conn, &principal(), &class, day, &marks)
                .unwrap_or_else(|e| panic!("attendance {class} {day}: {e:?}"));
        }
    }

    // Marks: the Half-Yearly maths sheet of VI-B (seeded draft) submitted for real.
    db::set_test_now(Some("2026-09-22T12:00:00Z".into()));
    let six_b: Vec<String> = {
        let mut s = conn
            .prepare("SELECT student_id FROM enrollment WHERE class_id='cls-6b' AND to_date IS NULL ORDER BY roll_no")
            .unwrap();
        let v = s.query_map([], |r| r.get::<_, String>(0)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
        v
    };
    let entries: Vec<MarkEntryInput> = six_b
        .iter()
        .enumerate()
        .map(|(i, sid)| MarkEntryInput { student_id: sid.clone(), marks: Some(40 + (i as i64 * 13) % 60), absent: false })
        .collect();
    submit_marks_logic(conn, &principal(), DeviceMode::Server, "es-6b-maths", &entries).unwrap();

    // Payments by the accountant (real record_payment: receipt no., allocations
    // oldest-first, receipt voucher + ledger entries, audit).
    db::set_test_now(Some("2026-09-23T11:30:00Z".into()));
    let mut paid = Vec::new();
    for (n, (sid, _)) in new_students.iter().enumerate().filter(|(n, _)| n % 3 == 0) {
        let (mode, reference) = match n % 4 {
            0 => ("upi", Some(format!("4265{:08}", n))),
            _ => ("cash", None),
        };
        let p = record_payment_logic(
            conn,
            &accountant(),
            Some("dev-a2"),
            DeviceMode::Server,
            &PaymentInput { student_id: sid.clone(), amount_paise: 250_000 + (n as i64 % 7) * 50_000, mode: mode.into(), reference },
        )
        .unwrap_or_else(|e| panic!("payment for {sid}: {e:?}"));
        paid.push(p.id);
    }
    assert!(paid.len() > 100);

    // A reversal (Principal direct reversal: request + decision + reversal row +
    // reversal voucher).
    reverse_payment_logic(conn, &principal(), DeviceMode::Server, &paid[0], "Cheque bounced (drill)").unwrap();

    // Pending requests from staff.
    create_request_logic(
        conn,
        &accountant(),
        &RequestInput {
            kind: "payment_reversal".into(),
            target_table: "payment".into(),
            target_id: paid[1].clone(),
            base_version: 0,
            reason: "Wrong student (drill)".into(),
            before_json: None,
            after_json: Some(serde_json::json!({ "summary": "Reverse a drill payment" }).to_string()),
        },
    )
    .unwrap();

    // Extra devices (phones + another office PC), none flagged.
    conn.execute(
        "INSERT INTO device(id,staff_id,platform,name,token_hash,receipt_series,admission_series,needs_rejoin) VALUES \
         ('dev-a4','stf-anita','android','Anita Phone','h4','A4','A4',0), \
         ('dev-a5','stf-meena','web','Meena iPhone (web app)','h5','A5','A5',0), \
         ('dev-a6','stf-suresh','windows','Front desk PC','h6','A6','A6',0)",
        [],
    )
    .unwrap();
    db::set_test_now(None);
}

// ---- a comparable snapshot of a whole database ---------------------------------

#[derive(Debug, Clone, PartialEq)]
struct Snapshot {
    /// table → (row count, sha256 of the sorted rows)
    tables: BTreeMap<String, (i64, String)>,
    /// account → (Σ debit, Σ credit)
    trial_balance: BTreeMap<String, (i64, i64)>,
    chain_head: String,
}

fn tables(conn: &Connection) -> Vec<String> {
    let mut s = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' \
             AND name NOT LIKE '%\\_fts%' ESCAPE '\\' ORDER BY name",
        )
        .unwrap();
    let v = s.query_map([], |r| r.get::<_, String>(0)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
    v
}

fn table_digest(conn: &Connection, t: &str) -> (i64, String) {
    let mut stmt = conn.prepare(&format!("SELECT * FROM \"{t}\"")).unwrap();
    let ncol = stmt.column_count();
    let mut rows: Vec<String> = stmt
        .query_map([], |r| {
            let mut line = String::new();
            for i in 0..ncol {
                match r.get_ref(i)? {
                    ValueRef::Null => line.push('N'),
                    ValueRef::Integer(v) => line.push_str(&format!("i{v}")),
                    ValueRef::Real(v) => line.push_str(&format!("r{v}")),
                    ValueRef::Text(v) => line.push_str(&format!("t{}", String::from_utf8_lossy(v))),
                    ValueRef::Blob(v) => line.push_str(&format!("b{v:?}")),
                }
                line.push('\u{1f}');
            }
            Ok(line)
        })
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    rows.sort();
    let mut h = Sha256::new();
    for r in &rows {
        h.update(r.as_bytes());
        h.update(b"\n");
    }
    (rows.len() as i64, format!("{:x}", h.finalize()))
}

fn snapshot(conn: &Connection) -> Snapshot {
    let tables = tables(conn).into_iter().map(|t| {
        let d = table_digest(conn, &t);
        (t, d)
    });
    let mut tb = BTreeMap::new();
    {
        let mut s = conn
            .prepare("SELECT account_id, SUM(debit_paise), SUM(credit_paise) FROM ledger_entry GROUP BY account_id")
            .unwrap();
        for row in s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))).unwrap() {
            let (a, d, c) = row.unwrap();
            tb.insert(a, (d, c));
        }
    }
    Snapshot { tables: tables.collect(), trial_balance: tb, chain_head: security::audit::head_hash(conn).unwrap() }
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap()
}

/// Every payment's allocations sum to its amount (allocated payments), and every
/// confirmed payment has exactly one receipt voucher; the book balances.
fn assert_money_invariants(conn: &Connection) {
    let bad_alloc = count(
        conn,
        "SELECT COUNT(*) FROM payment p JOIN (SELECT payment_id, SUM(amount_paise) s FROM payment_allocation GROUP BY payment_id) a \
         ON a.payment_id = p.id WHERE a.s != p.amount_paise",
    );
    assert_eq!(bad_alloc, 0, "a payment's allocations do not sum to its amount");
    // (the demo seed's `pay-prior-*` / `pay-today-*` rows are inserted without
    // allocations; every payment recorded through record_payment carries them)
    let unallocated_confirmed = count(
        conn,
        "SELECT COUNT(*) FROM payment p WHERE p.sync_state='confirmed' AND p.id NOT LIKE 'pay-prior-%' \
         AND p.id NOT LIKE 'pay-today-%' AND NOT EXISTS (SELECT 1 FROM payment_allocation a WHERE a.payment_id=p.id)",
    );
    let recorded = count(conn, "SELECT COUNT(*) FROM payment WHERE id LIKE 'pay-0%' OR id LIKE 'pay-1%'");
    assert!(recorded > 100, "real record_payment rows present ({recorded})");
    assert_eq!(unallocated_confirmed, 0, "every recorded payment carries allocations");
    let confirmed = count(conn, "SELECT COUNT(*) FROM payment WHERE sync_state='confirmed'");
    let vouchers = count(conn, "SELECT COUNT(*) FROM voucher WHERE source_table='payment'");
    assert_eq!(vouchers, confirmed, "one receipt voucher per confirmed payment");
    assert_eq!(ledger::ledger_imbalance(conn).unwrap(), 0, "Σ debits = Σ credits");
    let unbalanced_vouchers = count(
        conn,
        "SELECT COUNT(*) FROM (SELECT voucher_id FROM ledger_entry GROUP BY voucher_id HAVING SUM(debit_paise) != SUM(credit_paise))",
    );
    assert_eq!(unbalanced_vouchers, 0, "every voucher balances");
}

// ---- the fixture: PC A's school + its backup (built once, shared) --------------

struct Fixture {
    vbak: PathBuf,
    /// Snapshot of PC A taken right before the backup ran.
    before: Snapshot,
    students: i64,
    payments: i64,
    last_receipt: String,
    devices: i64,
    epoch: i64,
    school_id: String,
}

fn fixture() -> &'static Fixture {
    static F: OnceLock<Fixture> = OnceLock::new();
    F.get_or_init(|| {
        let pc_a = tmp_dir("pc-a");
        let live = pc_a.join("vidya.db");
        let mut conn = db::open_encrypted(&live, KEY_A).unwrap();
        db::run_migrations(&mut conn).unwrap();
        build_school(&mut conn);

        let students = count(&conn, "SELECT COUNT(*) FROM student");
        assert!(students >= 1000, "synthetic school has {students} students");
        assert!(count(&conn, "SELECT COUNT(*) FROM class") >= 15);
        assert!(count(&conn, "SELECT COUNT(*) FROM reversal") >= 1);
        assert!(count(&conn, "SELECT COUNT(*) FROM request WHERE status='pending'") >= 1);
        assert_eq!(security::audit::verify_chain(&conn).unwrap(), None, "PC A chain valid");
        assert!(count(&conn, "SELECT COUNT(*) FROM audit_log") > 400, "real writes appended audit entries");
        assert_money_invariants(&conn);

        let before = snapshot(&conn);
        let payments = count(&conn, "SELECT COUNT(*) FROM payment");
        let last_receipt: String =
            conn.query_row("SELECT receipt_no FROM payment ORDER BY (sync_state='confirmed') DESC, collected_at DESC, receipt_no DESC LIMIT 1", [], |r| r.get(0)).unwrap();
        assert!(last_receipt.starts_with("R-A2-"), "last receipt is the latest collected one: {last_receipt}");
        let devices = count(&conn, "SELECT COUNT(*) FROM device");
        let epoch = count(&conn, "SELECT server_epoch FROM school");
        let school_id: String = conn.query_row("SELECT id FROM school", [], |r| r.get(0)).unwrap();

        // The REAL backup path: "Turn on backups" derives the backup key from the
        // recovery key + the school salt and caches it; run_and_record exports,
        // verifies, writes the .meta sidecar and records the run.
        let key_hex = schedule::derive_and_cache(&conn, &pc_a, RECOVERY).unwrap();
        schedule::run_and_record(&mut conn, &pc_a, &key_hex).unwrap();
        let status: String =
            conn.query_row("SELECT status FROM backup_run ORDER BY started_at DESC LIMIT 1", [], |r| r.get(0)).unwrap();
        assert_ne!(status, "failed", "the backup verified locally");
        let backups: Vec<PathBuf> = std::fs::read_dir(pc_a.join("backups"))
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "vbak"))
            .collect();
        assert_eq!(backups.len(), 1, "one .vbak written: {backups:?}");
        let vbak = backups[0].clone();
        assert!(backup::meta_path(&vbak).exists(), ".vbak.meta sidecar written");
        // The .vbak is encrypted: not a plaintext SQLite file, unreadable without the key.
        let head = std::fs::read(&vbak).unwrap();
        assert_ne!(&head[..15], b"SQLite format 3", "backup is encrypted");
        drop(conn);

        Fixture { vbak, before, students, payments, last_receipt, devices, epoch, school_id }
    })
}

/// Copy the backup + meta to a "USB stick" dir, as the owner would carry them.
fn carry_backup(f: &Fixture, to: &Path) -> PathBuf {
    std::fs::create_dir_all(to).unwrap();
    let dest = to.join(f.vbak.file_name().unwrap());
    std::fs::copy(&f.vbak, &dest).unwrap();
    std::fs::copy(backup::meta_path(&f.vbak), backup::meta_path(&dest)).unwrap();
    dest
}

fn dir_listing(dir: &Path) -> Vec<(String, u64, Vec<u8>)> {
    let mut v: Vec<_> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| {
                    let p = e.path();
                    let bytes = if p.is_file() { std::fs::read(&p).unwrap() } else { vec![] };
                    (e.file_name().to_string_lossy().to_string(), bytes.len() as u64, Sha256::digest(&bytes).to_vec())
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

// ---- the drill ----------------------------------------------------------------

#[test]
fn restore_drill_onto_a_clean_replacement_pc() {
    let f = fixture();
    let usb = tmp_dir("usb");
    let vbak = carry_backup(f, &usb);

    // PC B: a brand-new data dir that does not exist yet (no prior DB, no key).
    let pc_b = tmp_dir("pc-b-parent").join("Vidya");
    assert!(!pc_b.exists());
    let live = pc_b.join("vidya.db");

    // restore_summary: recovery key + .meta salt → summary; validate + staleness.
    let (summary, key_hex) = restore::summary_with_recovery(&vbak, RECOVERY).unwrap();
    assert_eq!(summary.school_name, "Saraswati Public School");
    assert_eq!(summary.students as i64, f.students);
    assert_eq!(summary.payments as i64, f.payments);
    assert_eq!(summary.last_receipt_no.as_deref(), Some(f.last_receipt.as_str()));
    assert!(summary.chain_ok);
    assert!(vidya_core::restore::validate_restore(&summary).is_ok());
    assert_eq!(restore::backup_key_from_meta(&vbak, RECOVERY).unwrap(), key_hex);

    // restore_install: a fresh DB key; the key store write is the commit step.
    let stored_key: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
    let report = restore::install_then(&vbak, &key_hex, &live, KEY_B, "2026-09-24T08:00:00Z", || {
        *stored_key.borrow_mut() = Some(KEY_B.into());
        Ok(())
    })
    .unwrap();
    assert_eq!(stored_key.borrow().as_deref(), Some(KEY_B));
    assert!(report.safety_copy.is_none(), "no previous DB on a clean PC");
    assert_eq!(report.new_epoch, f.epoch + 1, "server epoch bumped");
    assert_eq!(report.devices_flagged as i64, f.devices, "every device flagged");
    let stray: Vec<String> = std::fs::read_dir(&pc_b)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n != "vidya.db")
        .collect();
    assert!(stray.is_empty(), "only the live DB is left on PC B: {stray:?}");
    // The .vbak was only read.
    assert_eq!(std::fs::read(&vbak).unwrap(), std::fs::read(&f.vbak).unwrap());

    // ---- verify the restored DB (opened as the app does at boot) ----------------
    assert!(!restore::recover_interrupted_install(&live), "nothing to repair after a clean install");
    let mut conn = db::open_encrypted(&live, KEY_B).unwrap();
    assert!(db::open_encrypted(&live, KEY_A).map(|c| !db::is_readable(&c)).unwrap_or(true), "old PC's key does not open it");
    db::run_migrations(&mut conn).unwrap();
    ledger::backfill_vouchers(&mut conn).unwrap();
    roles::seed_role_permissions(&mut conn).unwrap();
    let integrity: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0)).unwrap();
    assert_eq!(integrity, "ok");

    let after = snapshot(&conn);
    // Per-table: same row counts everywhere except audit_log (+1 restore entry); same
    // content everywhere except the rows the fencing deliberately changes.
    let mutated = ["audit_log", "school", "device", "data_revision", "role_permission"];
    assert_eq!(
        after.tables.keys().collect::<Vec<_>>(),
        f.before.tables.keys().collect::<Vec<_>>(),
        "same tables"
    );
    for (t, (n_before, d_before)) in &f.before.tables {
        let (n_after, d_after) = &after.tables[t];
        if t == "audit_log" {
            assert_eq!(*n_after, n_before + 1, "audit_log gains exactly the restore entry");
        } else if t == "backup_run" {
            assert_eq!(n_after, n_before, "backup_run as of the backup");
        } else {
            assert_eq!(n_after, n_before, "row count of {t}");
        }
        if !mutated.contains(&t.as_str()) {
            assert_eq!(d_after, d_before, "content of {t} identical");
        }
    }
    assert!(f.before.tables["student"].0 >= 1000);
    assert!(f.before.tables["attendance_mark"].0 >= (NEW_CLASSES * PER_CLASS * HISTORY_DAYS.len()) as i64);

    // Ledger: identical trial balance, balanced book, allocations intact.
    assert!(!after.trial_balance.is_empty());
    assert_eq!(after.trial_balance, f.before.trial_balance, "trial balance identical");
    assert_money_invariants(&conn);

    // Audit chain verifies (the verify_audit_chain command's logic) and ends with the
    // restore entry.
    assert_eq!(security::audit::verify_chain(&conn).unwrap(), None, "audit chain verifies");
    assert_ne!(after.chain_head, f.before.chain_head);
    let last_action: String =
        conn.query_row("SELECT action FROM audit_log ORDER BY seq DESC LIMIT 1", [], |r| r.get(0)).unwrap();
    assert_eq!(last_action, "restore");

    // Fencing per §8.9/§12: epoch + 1, every device needs to rejoin.
    assert_eq!(count(&conn, "SELECT server_epoch FROM school"), f.epoch + 1);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM device WHERE needs_rejoin=1"), f.devices);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM device WHERE needs_rejoin=0"), 0);
    let school_id: String = conn.query_row("SELECT id FROM school", [], |r| r.get(0)).unwrap();
    assert_eq!(school_id, f.school_id, "same school identity");
    assert!(schedule::read_cached_key(&pc_b).is_none(), "backup key is never carried over");

    // ---- the app works on the restored DB: normal reads + writes --------------
    // App state: school + licence present, setup complete → locked; PIN unlock works.
    // (§12: restore → Principal unlocks → Home; NOT the "no longer the server" screen)
    assert_eq!(state::compute(&conn, None).unwrap().state, AppState::Locked);
    let staff = unlock_logic(&mut conn, "stf-priya", vidya_lib::seed::DEMO_PIN).unwrap();
    assert_eq!(staff.role, "principal");

    db::set_test_now(Some("2026-09-24T10:00:00Z".into()));
    let page = list_students_page_logic(
        &mut conn,
        &StudentQuery { class_id: None, section: None, status: Some("active".into()), query: None, limit: 50, offset: 0 },
    )
    .unwrap();
    assert_eq!(page.total, count(&conn, "SELECT COUNT(*) FROM student WHERE status='active'"));
    assert_eq!(page.rows.len(), 50);
    let dues = list_dues_logic(&mut conn, &principal(), None).unwrap();
    assert!(dues.strip.total_due_paise > 0 && !dues.rows.is_empty());
    let kavya = list_fee_dues_logic(&mut conn, "stu-kavya-singh").unwrap();
    let sheet = get_attendance_sheet_logic(&mut conn, "cls-x0", HISTORY_DAYS[0]).unwrap();
    assert_eq!(sheet.status, "submitted");
    let day_book = day_book_logic(&mut conn, &principal(), "2026-09-23").unwrap();
    assert!(day_book.reversals_paise > 0, "the reversal survived");
    profit_summary_logic(&mut conn, &principal()).unwrap();

    // Writes: a new payment continues the receipt series; a new admission works.
    let p = record_payment_logic(
        &mut conn,
        &accountant(),
        Some("dev-a2"),
        DeviceMode::Server,
        &PaymentInput { student_id: "stu-kavya-singh".into(), amount_paise: kavya.total_due_paise, mode: "cash".into(), reference: None },
    )
    .unwrap();
    assert!(p.confirmed);
    assert!(p.receipt_no > f.last_receipt, "receipt numbering continues: {} > {}", p.receipt_no, f.last_receipt);
    assert_eq!(list_fee_dues_logic(&mut conn, "stu-kavya-singh").unwrap().total_due_paise, 0);
    create_student_logic(
        &mut conn,
        &principal(),
        Some("dev-a1"),
        DeviceMode::Server,
        "2026-09-24",
        &NewStudentInput {
            name: "After Restore".into(),
            class_id: "cls-x1".into(),
            roll_no: None,
            guardian_name: None,
            guardian_mobile: None,
            dob: None,
            gender: None,
            address: None,
            transport: None,
            rte: None,
            category: None,
            aadhaar_status: None,
        },
    )
    .unwrap();
    db::set_test_now(None);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM student"), f.students + 1);
    assert_money_invariants(&conn);
    assert_eq!(security::audit::verify_chain(&conn).unwrap(), None, "chain still verifies after new writes");

    // And PC B can itself be backed up again (the cycle closes).
    let key_b = schedule::derive_and_cache(&conn, &pc_b, RECOVERY).unwrap();
    assert_eq!(key_b, key_hex, "same recovery key + salt → same backup key on the new PC");
    schedule::run_and_record(&mut conn, &pc_b, &key_b).unwrap();
    let status: String =
        conn.query_row("SELECT status FROM backup_run ORDER BY started_at DESC LIMIT 1", [], |r| r.get(0)).unwrap();
    assert_ne!(status, "failed");
}

#[test]
fn wrong_recovery_key_fails_without_touching_anything() {
    let f = fixture();
    let usb = tmp_dir("usb-wrong");
    let vbak = carry_backup(f, &usb);

    // A PC that already has a live DB (e.g. retrying a restore).
    let pc = tmp_dir("pc-wrong");
    let live = pc.join("vidya.db");
    {
        let mut c = db::open_encrypted(&live, KEY_OLD).unwrap();
        db::run_migrations(&mut c).unwrap();
        c.execute("INSERT INTO kv(key,value) VALUES ('marker','\"old\"')", []).ok();
        c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();
    }
    let pc_before = dir_listing(&pc);
    let usb_before = dir_listing(&usb);

    // Summary: unreadable.
    assert!(matches!(
        restore::summary_with_recovery(&vbak, WRONG_RECOVERY),
        Err(backup::BackupError::Unreadable(_))
    ));
    // Install with the key the wrong recovery key derives: refused before staging.
    let wrong_hex = restore::backup_key_from_meta(&vbak, WRONG_RECOVERY).unwrap();
    let committed = std::cell::Cell::new(false);
    let err = restore::install_then(&vbak, &wrong_hex, &live, KEY_B, "2026-09-24T08:00:00Z", || {
        committed.set(true);
        Ok(())
    })
    .unwrap_err();
    assert!(matches!(err, backup::BackupError::Unreadable(_)), "{err:?}");
    assert!(!committed.get(), "no key was stored");
    assert_eq!(dir_listing(&pc), pc_before, "PC data dir untouched");
    assert_eq!(dir_listing(&usb), usb_before, "backup + meta untouched");

    // A wrong key into a fresh PC creates nothing either.
    let fresh = tmp_dir("pc-wrong-fresh-parent").join("Vidya");
    assert!(restore::install(&vbak, &wrong_hex, &fresh.join("vidya.db"), KEY_B, "2026-09-24T08:00:00Z").is_err());
    assert!(!fresh.exists(), "no data dir created for a wrong key");

    // The previous live DB still opens and works.
    let c = db::open_encrypted(&live, KEY_OLD).unwrap();
    assert!(db::is_readable(&c));
}

/// A replacement PC that already runs a (different) live school.
fn make_previous_live(pc: &Path) -> PathBuf {
    let live = pc.join("vidya.db");
    let mut c = db::open_encrypted(&live, KEY_OLD).unwrap();
    db::run_migrations(&mut c).unwrap();
    c.execute(
        "INSERT INTO school(id,name,backup_salt,server_epoch,created_at,updated_at) VALUES ('sch-prev','Previous Live School', x'00', 7, 't', 't')",
        [],
    )
    .unwrap();
    c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();
    live
}

fn previous_live_usable(live: &Path) {
    let mut c = db::open_encrypted(live, KEY_OLD).expect("previous live DB opens with its own key");
    assert!(db::is_readable(&c), "previous live DB readable");
    db::run_migrations(&mut c).unwrap();
    let (name, epoch): (String, i64) =
        c.query_row("SELECT name, server_epoch FROM school", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
    assert_eq!((name.as_str(), epoch), ("Previous Live School", 7), "untouched previous school");
    // …and still takes writes.
    c.execute("UPDATE school SET address='still works'", []).unwrap();
}

#[test]
fn interrupted_install_leaves_the_previous_live_db_usable() {
    let f = fixture();
    let usb = tmp_dir("usb-int");
    let vbak = carry_backup(f, &usb);
    let key_hex = restore::backup_key_from_meta(&vbak, RECOVERY).unwrap();

    // 1. Stale staged file from an earlier crashed attempt: the app's boot repair
    //    drops it and the live DB is untouched; a new install cleans it up too.
    let pc = tmp_dir("pc-int-stale");
    let live = make_previous_live(&pc);
    let staged = pc.join(".vidya.db.restore-staged");
    std::fs::write(&staged, b"half-written from a crash").unwrap();
    std::fs::write(pc.join(".vidya.db.restore-staged-wal"), b"junk").unwrap();
    assert!(!restore::recover_interrupted_install(&live));
    assert!(!staged.exists() && !pc.join(".vidya.db.restore-staged-wal").exists());
    previous_live_usable(&live);

    // 2. The install fails at its commit step (key store refuses the new key):
    //    rolled back — the previous live DB is back, under its own key.
    let err = restore::install_then(&vbak, &key_hex, &live, KEY_B, "2026-09-24T08:00:00Z", || {
        Err("keychain unavailable (drill)".into())
    })
    .unwrap_err();
    assert!(matches!(err, backup::BackupError::Io(_)), "{err:?}");
    previous_live_usable(&live);
    let leftovers: Vec<String> = std::fs::read_dir(&pc)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.contains("restore"))
        .collect();
    assert!(leftovers.is_empty(), "nothing half-done left: {leftovers:?}");

    // 3. Power cut mid-swap: the live DB had been moved aside as the safety copy and
    //    the staged DB was never renamed in. The next boot puts the previous DB back.
    let pc3 = tmp_dir("pc-int-crash");
    let live3 = make_previous_live(&pc3);
    std::fs::rename(&live3, pc3.join("vidya.db.pre-restore-20260924080000")).unwrap();
    std::fs::write(pc3.join(".vidya.db.restore-staged"), b"staged under a key that was never stored").unwrap();
    assert!(!live3.exists());
    assert!(restore::recover_interrupted_install(&live3), "boot repair restores the live DB");
    previous_live_usable(&live3);

    // 4. After all that, a real install over the previous school still succeeds and
    //    keeps the previous DB as a safety copy.
    let report = restore::install(&vbak, &key_hex, &live, KEY_B, "2026-09-24T09:00:00Z").unwrap();
    let safety = report.safety_copy.expect("previous DB kept as a safety copy");
    previous_live_usable(&safety);
    let c = db::open_encrypted(&live, KEY_B).unwrap();
    assert_eq!(count(&c, "SELECT COUNT(*) FROM student"), f.students);
    assert_eq!(security::audit::verify_chain(&c).unwrap(), None);
}
