//! Power-loss / crash durability (prompts/P09 §3): "power loss during a write
//! (kill -9 in tests) → no corruption". WAL + SQLCipher make every transaction
//! atomic; abandoning a connection mid-transaction without committing or running
//! Drop (via `std::mem::forget`) is the in-process equivalent of an abrupt kill.
//! On reopen the committed data must be intact, the half-written row absent, and
//! `PRAGMA integrity_check` must report "ok".

use rusqlite::OptionalExtension;
use vidya_lib::db;

const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("vidya-durability-{}-{}", name, db::now_iso().replace([':', '.'], "-")));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("school.db")
}

#[test]
fn crash_before_commit_leaves_no_partial_write_and_no_corruption() {
    let path = scratch("crash");

    // Session 1 — migrate and commit one row, then close cleanly.
    {
        let mut c = db::open_encrypted(&path, KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        c.execute("CREATE TABLE durab(id TEXT PRIMARY KEY, v INTEGER)", []).unwrap();
        c.execute("INSERT INTO durab(id, v) VALUES ('committed', 1)", []).unwrap();
    }

    // Session 2 — open a write transaction, insert, then abandon the connection
    // WITHOUT committing and WITHOUT running Drop. This is the `kill -9`.
    {
        let c = db::open_encrypted(&path, KEY).unwrap();
        c.execute_batch("BEGIN IMMEDIATE;").unwrap();
        c.execute("INSERT INTO durab(id, v) VALUES ('half-written', 2)", []).unwrap();
        std::mem::forget(c); // no COMMIT, no rollback — abrupt crash
    }

    // Session 3 — reopen from scratch and verify recovery.
    {
        let c = db::open_encrypted(&path, KEY).unwrap();
        let committed: i64 = c
            .query_row("SELECT v FROM durab WHERE id = 'committed'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(committed, 1, "committed data survives the crash");

        let half: Option<i64> = c
            .query_row("SELECT v FROM durab WHERE id = 'half-written'", [], |r| r.get(0))
            .optional()
            .unwrap();
        assert_eq!(half, None, "the uncommitted write left no partial row");

        let integrity: String = c.query_row("PRAGMA integrity_check", [], |r| r.get(0)).unwrap();
        assert_eq!(integrity, "ok", "database is structurally sound after the crash");
    }

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn reopen_after_clean_close_is_intact() {
    // Sanity companion: a normal close-then-reopen round-trips the data and the
    // integrity gate passes (mirrors reliability::integrity_gate's PRAGMA path).
    let path = scratch("clean");
    {
        let mut c = db::open_encrypted(&path, KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        c.execute("CREATE TABLE k(id TEXT PRIMARY KEY, v INTEGER)", []).unwrap();
        c.execute("INSERT INTO k(id, v) VALUES ('a', 42)", []).unwrap();
    }
    {
        let c = db::open_encrypted(&path, KEY).unwrap();
        let v: i64 = c.query_row("SELECT v FROM k WHERE id = 'a'", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 42);
        let integrity: String = c.query_row("PRAGMA integrity_check", [], |r| r.get(0)).unwrap();
        assert_eq!(integrity, "ok");
    }
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
