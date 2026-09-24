//! Demo-school seed (prompts/P03 Step 10) — DEBUG BUILDS ONLY.
//!
//! Recreates the mock's world (Saraswati Public School) so every Principal Home
//! number appears from real data. The construction is internally consistent:
//!
//! * 670 active students; 612 marked today across six classes (Nursery 96 · LKG 92
//!   · I-A 94 · II-A 90 · III-A 93 · V-A 91), VII-B (50) pending → overall
//!   559 present / 612 marked = 91.3% (the owner-approved rule; the mock's 91.4%
//!   is a designer-rounded value — see the phase-3 handoff).
//! * Collected today ₹48,500 from 23 confirmed receipts (Cash ₹21,000 + UPI
//!   ₹27,500), plus ₹2,400 waiting (an unsynced payment). Next receipt = R-A2-0419.
//! * Outstanding ₹6,84,200 across exactly 212 students with dues.
//! * Fee chart (last six school days, Sundays skipped): 32,000 · 41,000 · 28,500 ·
//!   55,000 · 46,200 · 48,500 → total ₹2,51,200.
//! * Four pending approvals (marks / payment reversal / attendance / student
//!   details) with the mock's summary lines.
//!
//! Amounts are paise. Ids are deterministic strings (stable for tests). The seed
//! inserts rows directly (no audit entries), so `verify_chain` stays valid (empty
//! chain); real command writes append audit entries.

use rusqlite::{params, Connection};
use time::OffsetDateTime;

use crate::security::pin::hash_pin;

const YMD: &[time::format_description::FormatItem] =
    time::macros::format_description!("[year]-[month]-[day]");

/// A demo PIN set for every seeded staff member so the unlock flow works in the
/// demo. (Never used in production; seed is debug-only.)
pub const DEMO_PIN: &str = "1234";

struct Ctx {
    ts: String,        // full RFC-3339 "now"
    today: time::Date, // date of "now"
    old: String,       // an "admitted long ago" timestamp (outside this week)
}

/// Seed the entire demo school. Assumes a fresh, migrated, empty DB.
pub fn seed_demo_school(conn: &mut Connection, now: OffsetDateTime) -> rusqlite::Result<()> {
    let ctx = Ctx {
        ts: now
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default(),
        today: now.date(),
        old: "2026-04-15T09:00:00Z".to_string(),
    };
    let tx = conn.transaction()?;
    school(&tx, &ctx)?;
    staff(&tx, &ctx)?;
    classes(&tx)?;
    subjects(&tx)?;
    let assigned = students_and_attendance(&tx, &ctx)?;
    fees_and_payments(&tx, &ctx, &assigned)?;
    academics(&tx)?;
    requests(&tx, &ctx)?;
    tx.commit()?;
    Ok(())
}

/// Default grade scale + one demo exam (VI-B Maths) so the exams / marks / report
/// card / grade-scale screens have data. Additive; touches only academics tables.
fn academics(tx: &rusqlite::Transaction) -> rusqlite::Result<()> {
    tx.execute("INSERT INTO grade_scale(id,name,is_default) VALUES ('gs-default','Default',1)", [])?;
    for b in vidya_core::grades::default_scale() {
        tx.execute(
            "INSERT INTO grade_band(id,scale_id,min_pct,max_pct,grade,grade_point) VALUES (?1,'gs-default',?2,?3,?4,?5)",
            params![format!("gb-{}", b.grade), b.min_pct_tenths, b.max_pct_tenths, b.grade, b.grade_point],
        )?;
    }
    tx.execute(
        "INSERT INTO exam(id,session_id,term_id,name,starts_on,ends_on) \
         VALUES ('exam-hy','sess-2627','term-1','Half-Yearly Exam','2026-09-15','2026-09-22')",
        [],
    )?;
    tx.execute(
        "INSERT INTO exam_subject(id,exam_id,class_subject_id,max_marks) VALUES ('es-6b-maths','exam-hy','cs-6b-maths',100)",
        [],
    )?;
    tx.execute(
        "INSERT INTO marks_sheet(id,exam_subject_id,status,created_at,updated_at,sync_state) \
         VALUES ('ms-6b-maths','es-6b-maths','draft','2026-09-23T09:00:00Z','2026-09-23T09:00:00Z','confirmed')",
        [],
    )?;
    let ids: Vec<String> = {
        let mut s = tx.prepare("SELECT student_id FROM enrollment WHERE class_id='cls-6b' AND to_date IS NULL ORDER BY roll_no")?;
        let v: Vec<String> = s.query_map([], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<_>>()?;
        v
    };
    for (i, sid) in ids.iter().enumerate() {
        // Deterministic demo pattern: mostly graded, one absent, one not-entered.
        let (marks, absent): (Option<i64>, i64) = match i % 5 {
            1 => (None, 1),               // absent
            2 => (None, 0),               // NULL (not entered)
            _ => (Some(55 + (i as i64 * 7) % 45), 0),
        };
        tx.execute(
            "INSERT INTO mark_entry(id,sheet_id,student_id,marks,absent) VALUES (?1,'ms-6b-maths',?2,?3,?4)",
            params![format!("me-{sid}"), sid, marks, absent],
        )?;
    }
    Ok(())
}

fn school(tx: &rusqlite::Transaction, c: &Ctx) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO school(id,name,address,board,backup_salt,server_epoch,settings_json,created_at,updated_at,sync_state) \
         VALUES ('sch-demo','Saraswati Public School','12 Temple Road, Pune','CBSE', x'0102030405060708090a0b0c0d0e0f10', 1, '{}', ?1, ?1, 'confirmed')",
        params![c.ts],
    )?;
    tx.execute(
        "INSERT INTO licence(licence_id,school_id,plan,issued_at,signature,raw_json,status,last_check_at) \
         VALUES ('lic-demo','sch-demo','perpetual', ?1, 'demo-sig','{}','active', ?1)",
        params![c.ts],
    )?;
    tx.execute(
        "INSERT INTO academic_session(id,label,starts_on,ends_on,is_current,read_only) \
         VALUES ('sess-2627','2026–27','2026-04-01','2027-03-31',1,0)",
        [],
    )?;
    tx.execute(
        "INSERT INTO term(id,session_id,name,starts_on,ends_on) VALUES \
         ('term-1','sess-2627','Term 1','2026-04-01','2026-09-30'), \
         ('term-2','sess-2627','Term 2','2026-10-01','2027-03-31')",
        [],
    )?;
    Ok(())
}

fn staff(tx: &rusqlite::Transaction, c: &Ctx) -> rusqlite::Result<()> {
    let pin = hash_pin(DEMO_PIN).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let people = [
        ("stf-priya", "Priya Sharma", "principal", "9820011111"),
        ("stf-suresh", "Suresh Patel", "accountant", "9820022222"),
        ("stf-anita", "Anita Rao", "teacher", "9820033333"),
        ("stf-meena", "Meena Iyer", "teacher", "9820044444"),
        ("stf-nair", "R. Nair", "teacher", "9820055555"),
    ];
    for (id, name, role, mobile) in people {
        tx.execute(
            "INSERT INTO staff(id,name,role,mobile,pin_hash,state,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,'active',?6,?6,'confirmed')",
            params![id, name, role, mobile, pin, c.ts],
        )?;
    }
    // The server device (A1) + the accountant's collecting device (A2, series
    // used by the mock receipts) + a phone (A3) for the "waiting" payment.
    tx.execute(
        "INSERT INTO device(id,staff_id,platform,name,token_hash,receipt_series,admission_series,needs_rejoin) VALUES \
         ('dev-a1','stf-priya','macos','School PC','h1','A1','A1',0), \
         ('dev-a2','stf-suresh','windows','Office PC','h2','A2','A2',0), \
         ('dev-a3','stf-suresh','android','Suresh Phone','h3','A3','A3',0)",
        [],
    )?;
    Ok(())
}

fn classes(tx: &rusqlite::Transaction) -> rusqlite::Result<()> {
    // (id, name, section, display, class_teacher, sort_order)
    let rows = [
        ("cls-nur", "Nursery", "", "Nursery", "stf-nair", 0),
        ("cls-lkg", "LKG", "", "LKG", "stf-nair", 1),
        ("cls-1a", "I", "A", "I-A", "stf-anita", 2),
        ("cls-2a", "II", "A", "II-A", "stf-anita", 3),
        ("cls-3a", "III", "A", "III-A", "stf-anita", 4),
        ("cls-5a", "V", "A", "V-A", "stf-meena", 5),
        ("cls-7b", "VII", "B", "VII-B", "stf-meena", 6),
        ("cls-4a", "IV", "A", "IV-A", "stf-nair", 7),
        ("cls-6b", "VI", "B", "VI-B", "stf-anita", 8),
    ];
    for (id, name, section, display, teacher, sort) in rows {
        tx.execute(
            "INSERT INTO class(id,name,section,display,class_teacher_id,sort_order) VALUES (?1,?2,?3,?4,?5,?6)",
            params![id, name, section, display, teacher, sort],
        )?;
    }
    Ok(())
}

fn subjects(tx: &rusqlite::Transaction) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO subject(id,name,name_hi) VALUES ('sub-maths','Maths','गणित')",
        [],
    )?;
    // VI-B Maths, taught by Anita Rao — target of the marks-correction request.
    tx.execute(
        "INSERT INTO class_subject(id,class_id,subject_id,teacher_id) VALUES ('cs-6b-maths','cls-6b','sub-maths','stf-anita')",
        [],
    )?;
    Ok(())
}

/// The V-A roster (34 names), verbatim from the mock (design/screens/Attendance).
const VA_NAMES: [&str; 34] = [
    "Aadhya Sharma", "Aarav Gupta", "Ananya Reddy", "Arjun Yadav", "Diya Patel",
    "Ishaan Khan", "Kabir Joshi", "Meera Nair", "Mohammed Faiz", "Pooja Verma",
    "Rahul Kumar", "Riya Verma", "Saanvi Rao", "Vivaan Singh", "Aditi Mishra",
    "Ayaan Qureshi", "Bhavya Jain", "Dev Malhotra", "Fatima Sheikh", "Gaurav Chauhan",
    "Harini Iyer", "Ira Kapoor", "Karan Mehta", "Lakshmi Pillai", "Manav Tiwari",
    "Nandini Das", "Om Prakash", "Prisha Agarwal", "Reyansh Bose", "Sara Thomas",
    "Tanvi Kulkarni", "Uday Rathore", "Vanya Saxena", "Zoya Ansari",
];

/// Student ids the caller can attach dues to (the general population + Kavya).
struct Assigned {
    /// 211 general students that will each get a due.
    general: Vec<String>,
    /// arbitrary students to attribute today's collections to.
    payers: Vec<String>,
}

/// Create the 670 students + enrollments and today's attendance sheets/marks.
fn students_and_attendance(tx: &rusqlite::Transaction, c: &Ctx) -> rusqlite::Result<Assigned> {
    let today = c.today.format(YMD).unwrap_or_default();
    let mut n = 0usize;
    let mut general: Vec<String> = Vec::new();
    let mut payers: Vec<String> = Vec::new();

    // Helper closures can't borrow tx mutably twice; inline the inserts.
    // (class_id, count, present, absent, submitted, use_roster)
    let plan = [
        ("cls-nur", 25usize, 24u32, 1u32, true, false),
        ("cls-lkg", 150, 138, 12, true, false),
        ("cls-1a", 50, 47, 3, true, false),
        ("cls-2a", 269, 241, 28, true, false),
        ("cls-3a", 84, 78, 6, true, false),
        ("cls-5a", 34, 31, 3, true, true),  // V-A roster, submitted
        ("cls-7b", 50, 0, 0, false, false), // VII-B pending (draft, no marks)
    ];

    for (ci, (class_id, count, present, absent, submitted, roster)) in plan.iter().enumerate() {
        // One attendance sheet per class today.
        let sheet_id = format!("sheet-{class_id}");
        let (status, sub_by, sub_at): (&str, Option<&str>, Option<&str>) = if *submitted {
            ("submitted", Some("stf-meena"), Some(c.ts.as_str()))
        } else {
            ("draft", None, None)
        };
        tx.execute(
            "INSERT INTO attendance_sheet(id,class_id,date,status,submitted_by,submitted_at,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?7,'confirmed')",
            params![sheet_id, class_id, today, status, sub_by, sub_at, c.ts],
        )?;

        // `i` is used for the roll number, mark thresholds and the roster index.
        #[allow(clippy::needless_range_loop)]
        for i in 0..*count {
            let sid = format!("stu-{n}");
            let name = if *roster { VA_NAMES[i].to_string() } else { format!("Student {n}") };
            // Four "admissions this week": the first four students overall.
            let created = if n < 4 { c.ts.clone() } else { c.old.clone() };
            tx.execute(
                "INSERT INTO student(id,name,status,created_at,updated_at,sync_state) VALUES (?1,?2,'active',?3,?3,'confirmed')",
                params![sid, name, created],
            )?;
            tx.execute(
                "INSERT INTO enrollment(id,student_id,class_id,session_id,roll_no,from_date,created_at,updated_at,sync_state) \
                 VALUES (?1,?2,?3,'sess-2627',?4,'2026-04-01',?5,?5,'confirmed')",
                params![format!("enr-{n}"), sid, class_id, (i as i64) + 1, c.ts],
            )?;
            // Marks: first `present` are P, next `absent` are A (submitted classes).
            if *submitted {
                let mark = if (i as u32) < *present {
                    "P"
                } else if (i as u32) < *present + *absent {
                    "A"
                } else {
                    "P"
                };
                tx.execute(
                    "INSERT INTO attendance_mark(id,sheet_id,student_id,mark) VALUES (?1,?2,?3,?4)",
                    params![format!("mk-{n}"), sheet_id, sid, mark],
                )?;
            }
            // Collect ids for dues (skip V-A roster + VII-B; use plain classes).
            if !*roster && ci != 6 && general.len() < 211 {
                general.push(sid.clone());
            }
            if payers.len() < 40 {
                payers.push(sid.clone());
            }
            n += 1;
        }
    }

    // VI-B (5, incl. Kavya Singh) and IV-A (3, incl. Kavya Reddy) — no sheet today.
    let extras = [
        ("cls-6b", 5usize, "stu-kavya-singh", "Kavya Singh", "2023/0287", 14i64),
        ("cls-4a", 3usize, "stu-kavya-reddy", "Kavya Reddy", "2024/0519", 7i64),
    ];
    for (class_id, count, special_id, special_name, special_adm, special_roll) in extras {
        for i in 0..count {
            let (sid, name, adm, roll) = if i == 0 {
                (special_id.to_string(), special_name.to_string(), Some(special_adm), special_roll)
            } else {
                (format!("stu-{n}"), format!("Student {n}"), None, (i as i64) + 1)
            };
            tx.execute(
                "INSERT INTO student(id,name,admission_no,status,created_at,updated_at,sync_state) VALUES (?1,?2,?3,'active',?4,?4,'confirmed')",
                params![sid, name, adm, c.old],
            )?;
            tx.execute(
                "INSERT INTO enrollment(id,student_id,class_id,session_id,roll_no,from_date,created_at,updated_at,sync_state) \
                 VALUES (?1,?2,?3,'sess-2627',?4,'2026-04-01',?5,?5,'confirmed')",
                params![format!("enr-x-{n}"), sid, class_id, roll, c.old],
            )?;
            n += 1;
        }
    }
    // Kavya Mishra (II-A, paid) is one of the first II-A students; rename stu with
    // a known id so search shows all three "Kavya" rows. Reuse an II-A id.
    tx.execute(
        "UPDATE student SET name='Kavya Mishra', admission_no='2025/0142' WHERE id='stu-200'",
        [],
    )?;

    Ok(Assigned { general, payers })
}

fn fees_and_payments(tx: &rusqlite::Transaction, c: &Ctx, a: &Assigned) -> rusqlite::Result<()> {
    // Fee heads.
    tx.execute(
        "INSERT INTO fee_head(id,name,amount_paise,frequency,applies_to,active) VALUES \
         ('fh-t1','Term 1 tuition',600000,'term','all',1), \
         ('fh-t2','Term 2 tuition',600000,'term','all',1), \
         ('fh-exam','Exam fee',50000,'once','all',1), \
         ('fh-transport','Transport',20000,'month','transport',1)",
        [],
    )?;

    // Kavya Singh's dues: T1 6000 (paid), T2 6000 (3600 paid → 2400 due),
    // Exam 500 (due), Transport Sep 200 (due) → outstanding 3100.
    let kavya_dues = [
        ("due-ks-t1", "fh-t1", "T1", 600000i64),
        ("due-ks-t2", "fh-t2", "T2", 600000),
        ("due-ks-exam", "fh-exam", "2026", 50000),
        ("due-ks-tr", "fh-transport", "2026-09", 20000),
    ];
    for (id, head, period, amt) in kavya_dues {
        tx.execute(
            "INSERT INTO fee_due(id,student_id,fee_head_id,period,amount_paise,created_at,updated_at,sync_state) \
             VALUES (?1,'stu-kavya-singh',?2,?3,?4,?5,?5,'confirmed')",
            params![id, head, period, amt, c.old],
        )?;
    }
    // A historical payment for Kavya: ₹9,600 → T1 6000 + T2 3600 (leaves T2 2400).
    tx.execute(
        "INSERT INTO payment(id,receipt_no,student_id,amount_paise,mode,collected_by,collected_at,device_id,confirmed_at,created_at,updated_at,sync_state) \
         VALUES ('pay-ks','R-A2-0300','stu-kavya-singh',960000,'upi','stf-suresh','2026-08-01T10:00:00Z','dev-a2','2026-08-01T10:00:00Z','2026-08-01T10:00:00Z','2026-08-01T10:00:00Z','confirmed')",
        [],
    )?;
    tx.execute(
        "INSERT INTO payment_allocation(id,payment_id,fee_due_id,amount_paise,kind) VALUES \
         ('al-ks-1','pay-ks','due-ks-t1',600000,'due'), \
         ('al-ks-2','pay-ks','due-ks-t2',360000,'due')",
        [],
    )?;

    // 211 general dues summing to 681,100 (210 × 3,200 + 1 × 9,100). With Kavya's
    // 3,100 balance the total outstanding is exactly ₹6,84,200 across 212 students.
    for (i, sid) in a.general.iter().enumerate() {
        let amt: i64 = if i == 0 { 910000 } else { 320000 };
        tx.execute(
            "INSERT INTO fee_due(id,student_id,fee_head_id,period,amount_paise,created_at,updated_at,sync_state) \
             VALUES (?1,?2,'fh-t2','T2',?3,?4,?4,'confirmed')",
            params![format!("due-g-{i}"), sid, amt, c.old],
        )?;
    }

    // --- Payments driving the fee chart + "collected today" ---
    // Six school days ending today (Sundays skipped): the 5 prior days get one
    // payment each; today gets the 23 confirmed receipts.
    let days = crate::dash::fee_collection_days_dates(c.today);
    let prior_amounts = [3_200_000i64, 4_100_000, 2_850_000, 5_500_000, 4_620_000];
    // days[0..5] are the five prior school days (oldest→newest); days[5] is today.
    for (i, date) in days.iter().take(5).enumerate() {
        let ds = date.format(YMD).unwrap_or_default();
        let sid = &a.payers[i % a.payers.len()];
        tx.execute(
            "INSERT INTO payment(id,receipt_no,student_id,amount_paise,mode,collected_by,collected_at,device_id,confirmed_at,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,'cash','stf-suresh',?5,'dev-a2',?5,?5,?5,'confirmed')",
            params![
                format!("pay-prior-{i}"),
                format!("R-A2-{:04}", 391 + i),
                sid,
                prior_amounts[i],
                format!("{ds}T10:00:00Z")
            ],
        )?;
    }
    // Today: 23 confirmed receipts A2-0396..0418 = Cash ₹21,000 (10×2,100) +
    // UPI ₹27,500 (12×2,100 + 1×2,300).
    let today_ts = format!("{}T11:00:00Z", c.today.format(YMD).unwrap_or_default());
    for i in 0..23usize {
        let (amt, mode): (i64, &str) = if i < 10 {
            (210_000, "cash")
        } else if i < 22 {
            (210_000, "upi")
        } else {
            (230_000, "upi")
        };
        let seq = 396 + i;
        let sid = &a.payers[i % a.payers.len()];
        tx.execute(
            "INSERT INTO payment(id,receipt_no,student_id,amount_paise,mode,collected_by,collected_at,device_id,confirmed_at,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,?5,'stf-suresh',?6,'dev-a2',?6,?6,?6,'confirmed')",
            params![format!("pay-today-{i}"), format!("R-A2-{seq:04}"), sid, amt, mode, today_ts],
        )?;
    }
    // ₹2,400 waiting for server (unconfirmed, different series so A2 max stays 0418).
    tx.execute(
        "INSERT INTO payment(id,receipt_no,student_id,amount_paise,mode,collected_by,collected_at,device_id,created_at,updated_at,sync_state) \
         VALUES ('pay-wait','R-A3-0001',?1,240000,'cash','stf-suresh',?2,'dev-a3',?2,?2,'on_device')",
        params![a.payers[0], today_ts],
    )?;
    Ok(())
}

fn requests(tx: &rusqlite::Transaction, c: &Ctx) -> rusqlite::Result<()> {
    // created_at offsets: 2h, 5h, 1 day, 2 days ago.
    let h2 = (c.ts.clone(), "2 hours");
    let two_hours = shift_hours(&c.ts, -2);
    let five_hours = shift_hours(&c.ts, -5);
    let one_day = shift_hours(&c.ts, -24);
    let two_days = shift_hours(&c.ts, -48);
    let _ = h2;

    // (id, type, target_table, target_id, requested_by, created_at, summary, before, after_extra)
    let reqs = [
        (
            "req-marks", "marks_correction", "mark_entry", "me-ks-maths", "stf-anita", &two_hours,
            "Half Yearly · VI-B Maths · Kavya Singh 62 → 72", "62", "72",
        ),
        (
            "req-reversal", "payment_reversal", "payment", "pay-dup-0418", "stf-suresh", &five_hours,
            "Receipt R-A2-0418 · ₹1,500 · entered twice", "₹1,500", "reversed",
        ),
        (
            "req-attend", "attendance_correction", "attendance_mark", "mk-rahul", "stf-meena", &one_day,
            "V-A · 19 Sep · Rahul Kumar: Absent → Present", "A", "P",
        ),
        (
            "req-details", "student_details", "student", "stu-aarav", "stf-suresh", &two_days,
            "Aarav Gupta · father’s mobile number", "98200 00000", "98200 12345",
        ),
    ];
    for (id, ty, tbl, tid, by, created, summary, before, after) in reqs {
        let before_json = serde_json::json!({ "value": before }).to_string();
        let after_json = serde_json::json!({ "value": after, "summary": summary }).to_string();
        tx.execute(
            "INSERT INTO request(id,type,target_table,target_id,base_version,before_json,after_json,reason,requested_by,revision,status,apply_state,created_at,updated_at,sync_state) \
             VALUES (?1,?2,?3,?4,1,?5,?6,'Correction requested',?7,1,'pending','not_applied',?8,?8,'confirmed')",
            params![id, ty, tbl, tid, before_json, after_json, by, created],
        )?;
    }
    Ok(())
}

/// Shift an RFC-3339 timestamp by whole hours (for request ages).
fn shift_hours(ts: &str, hours: i64) -> String {
    let fmt = &time::format_description::well_known::Rfc3339;
    match OffsetDateTime::parse(ts, fmt) {
        Ok(t) => (t + time::Duration::hours(hours)).format(fmt).unwrap_or_else(|_| ts.to_string()),
        Err(_) => ts.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dash;
    use crate::db;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn seeded() -> Connection {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        let now = OffsetDateTime::parse("2026-09-23T09:00:00Z", &time::format_description::well_known::Rfc3339).unwrap();
        seed_demo_school(&mut c, now).unwrap();
        c
    }

    #[test]
    fn demo_reproduces_principal_home_numbers() {
        let c = seeded();
        let d = dash::principal_dashboard(&c, "2026-09-23").unwrap();

        // Stat 1 — attendance today (owner decision: true 91.3%, not the mock's 91.4%).
        assert_eq!(d.attendance_pct_tenths, 913, "attendance 91.3%");
        assert_eq!(d.attendance_marked, 612, "612 marked");
        assert_eq!(d.attendance_total, 670, "670 total");
        assert_eq!(
            d.attendance_pending,
            vec![dash::PendingClass { class: "VII-B".to_string(), teacher: Some("Meena Iyer".to_string()) }],
            "VII-B pending, class teacher Meena Iyer"
        );

        // Attendance by class rows (exact mock values).
        let classes: Vec<(String, Option<i64>)> =
            d.classes.iter().map(|c| (c.name.clone(), c.pct)).collect();
        assert_eq!(
            classes,
            vec![
                ("Nursery".into(), Some(96)),
                ("LKG".into(), Some(92)),
                ("I-A".into(), Some(94)),
                ("II-A".into(), Some(90)),
                ("III-A".into(), Some(93)),
                ("V-A".into(), Some(91)),
                ("VII-B".into(), None),
            ]
        );

        // Stat 2 — collected today.
        assert_eq!(d.collected_today_paise, 4_850_000, "₹48,500 collected");
        assert_eq!(d.receipts_today, 23, "23 receipts");
        assert_eq!(d.waiting_paise, 240_000, "₹2,400 waiting");

        // Stat 3 — outstanding.
        assert_eq!(d.outstanding_paise, 68_420_000, "₹6,84,200 outstanding");
        assert_eq!(d.students_with_dues, 212, "212 with dues");

        // Stat 4 — students.
        assert_eq!(d.active_students, 670, "670 active");
        assert_eq!(d.admissions_this_week, 4, "4 admissions this week");

        // Fee chart (last six school days) + total ₹2,51,200.
        let vals: Vec<i64> = d.fee_days.iter().map(|x| x.value_paise).collect();
        assert_eq!(vals, vec![3_200_000, 4_100_000, 2_850_000, 5_500_000, 4_620_000, 4_850_000]);
        assert_eq!(d.fee_total_paise, 25_120_000, "₹2,51,200 total");
        assert_eq!(d.fee_days.last().unwrap().day, "Today");

        // Approvals — 4 pending, most-recent first, with the mock summaries.
        assert_eq!(d.approvals_total, 4);
        assert_eq!(d.approvals.len(), 4);
        assert_eq!(d.approvals[0].what, "Half Yearly · VI-B Maths · Kavya Singh 62 → 72");
        assert_eq!(d.approvals[0].who, "Anita Rao, Teacher");
    }

    #[test]
    fn cash_upi_split_matches_mock() {
        let c = seeded();
        let cash: i64 = c
            .query_row(
                "SELECT COALESCE(SUM(amount_paise),0) FROM payment WHERE date(collected_at)='2026-09-23' AND mode='cash' AND sync_state='confirmed'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let upi: i64 = c
            .query_row(
                "SELECT COALESCE(SUM(amount_paise),0) FROM payment WHERE date(collected_at)='2026-09-23' AND mode='upi' AND sync_state='confirmed'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cash, 2_100_000, "Cash ₹21,000");
        assert_eq!(upi, 2_750_000, "UPI ₹27,500");
    }

    #[test]
    fn next_receipt_is_r_a2_0419() {
        let c = seeded();
        let max_seq: i64 = c
            .query_row(
                "SELECT MAX(CAST(substr(receipt_no,6) AS INTEGER)) FROM payment WHERE receipt_no LIKE 'R-A2-%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(max_seq, 418, "next A2 receipt is R-A2-0419");
    }
}
