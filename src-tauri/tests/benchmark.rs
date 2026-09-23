//! Dashboard aggregate benchmark (prompts/P02 BENCHMARK). Seeds 1,500 students /
//! 60 classes / one term of attendance + payments and times the Principal-Home
//! style aggregates. Ignored by default (heavy seed); run with:
//!   cargo test -p vidya --release --test benchmark -- --ignored --nocapture

use rusqlite::{params, Connection};
use std::time::Instant;
use vidya_lib::db;

const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const DAYS: usize = 20;
const CLASSES: usize = 60;
const PER_CLASS: usize = 25; // 60 * 25 = 1500 students

fn seed(conn: &mut Connection) {
    conn.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
    let tx = conn.transaction().unwrap();
    for c in 0..CLASSES {
        tx.execute(
            "INSERT INTO class(id,name,display,sort_order) VALUES(?1,?2,?3,?4)",
            params![format!("c{c}"), format!("Class {c}"), format!("C-{c}"), c as i64],
        )
        .unwrap();
    }
    {
        let mut st = tx
            .prepare("INSERT INTO student(id,name,transport,status,created_at,updated_at) VALUES(?1,?2,?3,'active','t','t')")
            .unwrap();
        let mut en = tx
            .prepare("INSERT INTO enrollment(id,student_id,class_id,session_id,from_date,created_at,updated_at) VALUES(?1,?2,?3,'sess','2026-04-01','t','t')")
            .unwrap();
        for s in 0..(CLASSES * PER_CLASS) {
            st.execute(params![format!("s{s}"), format!("Student {s}"), (s % 3 == 0) as i64]).unwrap();
            en.execute(params![format!("e{s}"), format!("s{s}"), format!("c{}", s / PER_CLASS)]).unwrap();
        }
    }
    {
        let mut sh = tx
            .prepare("INSERT INTO attendance_sheet(id,class_id,date,status,created_at,updated_at) VALUES(?1,?2,?3,'submitted','t','t')")
            .unwrap();
        let mut mk = tx
            .prepare("INSERT INTO attendance_mark(id,sheet_id,student_id,mark) VALUES(?1,?2,?3,?4)")
            .unwrap();
        for d in 0..DAYS {
            let date = format!("2026-09-{:02}", d + 1);
            for c in 0..CLASSES {
                let sheet = format!("sh_{d}_{c}");
                sh.execute(params![sheet, format!("c{c}"), date]).unwrap();
                for i in 0..PER_CLASS {
                    let sid = c * PER_CLASS + i;
                    let mark = if i % 10 == 0 { "A" } else if i % 15 == 0 { "L" } else { "P" };
                    mk.execute(params![format!("mk_{d}_{c}_{i}"), sheet, format!("s{sid}"), mark]).unwrap();
                }
            }
        }
    }
    {
        let mut pay = tx
            .prepare("INSERT INTO payment(id,receipt_no,student_id,amount_paise,mode,collected_by,collected_at,created_at,updated_at) VALUES(?1,?2,?3,?4,'cash','st1',?5,'t','t')")
            .unwrap();
        for p in 0..(CLASSES * PER_CLASS) {
            pay.execute(params![
                format!("p{p}"),
                format!("R-A1-{p:05}"),
                format!("s{}", p % (CLASSES * PER_CLASS)),
                600000i64,
                format!("2026-09-{:02}T10:00:00Z", (p % DAYS) + 1),
            ])
            .unwrap();
        }
    }
    tx.commit().unwrap();
}

fn bench(conn: &Connection, label: &str, run: impl Fn(&Connection)) {
    // warm once, then measure
    run(conn);
    let t = Instant::now();
    run(conn);
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    println!("  {label:<32} {ms:7.2} ms");
    assert!(ms <= 150.0, "{label} took {ms:.2} ms (> 150 ms budget)");
}

#[test]
#[ignore = "perf benchmark; run with --ignored --release --nocapture"]
fn dashboard_aggregates_under_150ms() {
    let mut conn = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut conn).unwrap();
    let t = Instant::now();
    seed(&mut conn);
    println!(
        "\nseeded {} students / {} classes / {} days attendance / {} payments in {:.0} ms\n",
        CLASSES * PER_CLASS,
        CLASSES,
        DAYS,
        CLASSES * PER_CLASS,
        t.elapsed().as_secs_f64() * 1000.0
    );

    bench(&conn, "active students", |c| {
        let _: i64 = c
            .query_row("SELECT count(*) FROM student WHERE status='active'", [], |r| r.get(0))
            .unwrap();
    });
    bench(&conn, "collected today (sum)", |c| {
        let _: i64 = c
            .query_row(
                "SELECT COALESCE(SUM(amount_paise),0) FROM payment WHERE collected_at LIKE '2026-09-20%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
    });
    bench(&conn, "attendance today by mark", |c| {
        let mut s = c
            .prepare("SELECT mark, count(*) FROM attendance_mark am JOIN attendance_sheet sh ON am.sheet_id=sh.id WHERE sh.date='2026-09-20' GROUP BY mark")
            .unwrap();
        let _ = s.query_map([], |r| r.get::<_, i64>(1)).unwrap().count();
    });
    bench(&conn, "attendance by class today", |c| {
        let mut s = c
            .prepare("SELECT sh.class_id, SUM(mark='P'), COUNT(*) FROM attendance_mark am JOIN attendance_sheet sh ON am.sheet_id=sh.id WHERE sh.date='2026-09-20' GROUP BY sh.class_id")
            .unwrap();
        let _ = s.query_map([], |r| r.get::<_, i64>(0)).unwrap().count();
    });
    bench(&conn, "fee collection last 6 days", |c| {
        let mut s = c
            .prepare("SELECT substr(collected_at,1,10) d, SUM(amount_paise) FROM payment GROUP BY d ORDER BY d DESC LIMIT 6")
            .unwrap();
        let _ = s.query_map([], |r| r.get::<_, i64>(1)).unwrap().count();
    });
}
