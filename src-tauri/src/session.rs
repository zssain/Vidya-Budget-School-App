//! New academic session rollover (prompts/P08 Part D). Server-side; the pure
//! rules (promotion ladder, per-student overrides, carry-forward) live in
//! [`vidya_core::session`]. This module commits the rollover in **one
//! transaction** with **one audit entry**:
//!
//! * create the new session (current) + its terms; the old session becomes
//!   read-only and no longer current;
//! * promote every active student one class up the ladder (sections kept), or
//!   repeat / leave per an override; `XII` passes out (status `left`, reason
//!   `Passed out`); create the destination class if it does not exist yet;
//! * carry each student's unpaid balance forward as ONE "Previous balance" due in
//!   the new Term 1, linked to the old dues (which are kept, never deleted);
//! * generate the new session's term-head dues for the enrolled students.
//!
//! Enrollment history is never rewritten (§7): the old session's enrollment rows
//! stay; the new session gets fresh rows.

use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::security::audit;
use vidya_core::fees::{AppliesTo, FeeHead, PeriodSpec, Student as FeeStudent};
use vidya_core::money::Paise;
use vidya_core::session::{
    build_promotion, carry_forward, Outcome, Override, StudentBalance, StudentIn, LADDER,
};
use vidya_core::types::FeeFrequency;

/// A term to create in the new session.
#[derive(Debug, Clone)]
pub struct NewTerm {
    pub id: String,
    pub name: String,
    pub starts_on: String,
    pub ends_on: String,
}

/// Inputs for a rollover commit. Ids for new rows are `<id_prefix><kind><n>` so a
/// commit is deterministic (the command supplies a fresh prefix / UUID).
pub struct RolloverParams {
    pub new_session_id: String,
    pub new_label: String,
    pub new_starts_on: String,
    pub new_ends_on: String,
    pub terms: Vec<NewTerm>,
    /// Per-student overrides; absent students default to [`Override::Promote`].
    pub overrides: HashMap<String, Override>,
    pub now: String,
    pub staff_id: Option<String>,
    pub device_id: Option<String>,
    pub id_prefix: String,
}

/// What a rollover did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RolloverReport {
    pub promoted: u32,
    pub repeated: u32,
    pub passed_out: u32,
    pub left: u32,
    pub unknown: u32,
    pub new_enrollments: u64,
    pub new_dues: u64,
    pub carried_forward: u64,
    pub carried_amount_paise: i64,
    pub classes_created: u64,
}

/// Commit the rollover (Part D). Requires a current session to exist. Everything
/// happens in one transaction; any error rolls the whole thing back.
pub fn rollover_commit(conn: &mut Connection, p: &RolloverParams) -> rusqlite::Result<RolloverReport> {
    let tx = conn.transaction()?;
    let mut report = RolloverReport::default();
    let mut seq = 0u64;
    let mut next_id = |kind: &str| {
        seq += 1;
        format!("{}{}{}", p.id_prefix, kind, seq)
    };

    // 1. New session + terms; retire the old session.
    let old_session: String =
        tx.query_row("SELECT id FROM academic_session WHERE is_current=1 LIMIT 1", [], |r| r.get(0))?;
    tx.execute(
        "INSERT INTO academic_session(id,label,starts_on,ends_on,is_current,read_only) VALUES (?1,?2,?3,?4,1,0)",
        params![p.new_session_id, p.new_label, p.new_starts_on, p.new_ends_on],
    )?;
    tx.execute("UPDATE academic_session SET is_current=0, read_only=1 WHERE id=?1", params![old_session])?;
    for t in &p.terms {
        tx.execute(
            "INSERT INTO term(id,session_id,name,starts_on,ends_on) VALUES (?1,?2,?3,?4,?5)",
            params![t.id, p.new_session_id, t.name, t.starts_on, t.ends_on],
        )?;
    }
    let term1 = p.terms.first().map(|t| t.name.clone()).unwrap_or_else(|| "T1".into());

    // 2. Existing classes keyed by (name, section).
    let mut class_by_key: HashMap<(String, String), String> = {
        let mut s = tx.prepare("SELECT id, name, COALESCE(section,'') FROM class")?;
        let rows = s.query_map([], |r| {
            Ok(((r.get::<_, String>(1)?, r.get::<_, String>(2)?), r.get::<_, String>(0)?))
        })?;
        rows.collect::<rusqlite::Result<_>>()?
    };

    // 3. Active students of the current session, with class + transport.
    let students: Vec<StudentIn> = {
        let mut s = tx.prepare(
            "SELECT s.id, c.name, COALESCE(c.section,''), s.transport \
             FROM student s \
             JOIN enrollment e ON e.student_id=s.id AND e.session_id=?1 AND e.to_date IS NULL \
             JOIN class c ON c.id=e.class_id \
             WHERE s.status='active' ORDER BY s.id",
        )?;
        let rows = s.query_map(params![old_session], |r| {
            let id: String = r.get(0)?;
            let name: String = r.get(1)?;
            let section: String = r.get(2)?;
            Ok(StudentIn {
                student_id: id.clone(),
                class_name: name,
                section: if section.is_empty() { None } else { Some(section) },
                over: Override::Promote,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .map(|mut si| {
                si.over = p.overrides.get(&si.student_id).copied().unwrap_or(Override::Promote);
                si
            })
            .collect()
    };
    // transport flags for dues generation
    let transport: HashMap<String, bool> = {
        let mut s = tx.prepare("SELECT id, transport FROM student")?;
        let rows = s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? != 0)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };

    // 4. Promotion plan.
    let plan = build_promotion(&students);
    let summ = vidya_core::session::summarize(&plan);
    report.promoted = summ.promoted;
    report.repeated = summ.repeated;
    report.passed_out = summ.passed_out;
    report.left = summ.left;
    report.unknown = summ.unknown;

    // Enrolled students collected for dues (student_id, new_class_id).
    let mut enrolled: Vec<(String, String)> = Vec::new();
    let mut roll_by_class: HashMap<String, i64> = HashMap::new();

    for row in &plan {
        match row.outcome {
            Outcome::Promoted | Outcome::Repeated => {
                let name = row.to_name.clone().unwrap_or_default();
                let section = row.to_section.clone();
                let key = (name.clone(), section.clone().unwrap_or_default());
                // Resolve or create the destination class.
                let class_id = match class_by_key.get(&key) {
                    Some(id) => id.clone(),
                    None => {
                        let id = next_id("cls");
                        let display = match &section {
                            Some(s) if !s.is_empty() => format!("{name}-{s}"),
                            _ => name.clone(),
                        };
                        let sort = LADDER.iter().position(|c| *c == name).map(|i| i as i64).unwrap_or(999);
                        tx.execute(
                            "INSERT INTO class(id,name,section,display,sort_order) VALUES (?1,?2,?3,?4,?5)",
                            params![id, name, section, display, sort],
                        )?;
                        class_by_key.insert(key, id.clone());
                        report.classes_created += 1;
                        id
                    }
                };
                let roll = roll_by_class.entry(class_id.clone()).or_insert(0);
                *roll += 1;
                let enr_id = next_id("enr");
                tx.execute(
                    "INSERT INTO enrollment(id,student_id,class_id,session_id,roll_no,from_date,created_at,updated_at,updated_by_staff,updated_by_device,sync_state) \
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?7,?8,?9,'confirmed')",
                    params![enr_id, row.student_id, class_id, p.new_session_id, *roll, p.new_starts_on, p.now, p.staff_id, p.device_id],
                )?;
                report.new_enrollments += 1;
                enrolled.push((row.student_id.clone(), class_id));
            }
            Outcome::PassedOut | Outcome::Left => {
                let reason = row.outcome.left_reason().unwrap_or("Left");
                tx.execute(
                    "UPDATE student SET status='left', left_on=?1, left_reason=?2, updated_at=?3, updated_by_staff=?4, updated_by_device=?5 WHERE id=?6",
                    params![p.new_starts_on, reason, p.now, p.staff_id, p.device_id, row.student_id],
                )?;
            }
            Outcome::Unknown => { /* off-ladder: resolved by the wizard before commit; skipped here */ }
        }
    }

    // 5. Carry-forward: each enrolled student's unpaid balance from BEFORE the
    //    rollover → one "Previous balance" due in the new Term 1 (linked to the old
    //    dues, which are kept). Compute balances first, then insert.
    let enrolled_ids: std::collections::HashSet<&str> = enrolled.iter().map(|(s, _)| s.as_str()).collect();
    let mut per_student: HashMap<String, (i64, Vec<String>)> = HashMap::new();
    {
        let mut s = tx.prepare(
            "SELECT d.student_id, d.id, \
                d.amount_paise - COALESCE((SELECT SUM(a.amount_paise) FROM payment_allocation a WHERE a.fee_due_id=d.id),0) \
             FROM fee_due d WHERE d.cancelled_at IS NULL",
        )?;
        let rows = s.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))
        })?;
        for row in rows {
            let (sid, due_id, bal) = row?;
            if bal > 0 && enrolled_ids.contains(sid.as_str()) {
                let e = per_student.entry(sid).or_insert((0, Vec::new()));
                e.0 += bal;
                e.1.push(due_id);
            }
        }
    }
    let balances: Vec<StudentBalance> = {
        // Deterministic order for stable ids/tests.
        let mut v: Vec<_> = per_student.into_iter().collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v.into_iter()
            .map(|(sid, (bal, dues))| StudentBalance {
                student_id: sid,
                balance_paise: Paise(bal),
                old_due_ids: dues,
            })
            .collect()
    };
    let carry = carry_forward(&balances, &term1);
    if !carry.is_empty() {
        ensure_previous_balance_head(&tx)?;
        for cf in &carry {
            let due_id = next_id("pb");
            let linked = serde_json::to_string(&cf.linked_due_ids).unwrap_or_else(|_| "[]".into());
            tx.execute(
                "INSERT INTO fee_due(id,student_id,fee_head_id,period,amount_paise,carried_from_due_ids,created_at,updated_at,updated_by_staff,updated_by_device,sync_state) \
                 VALUES (?1,?2,'fh-prev-balance',?3,?4,?5,?6,?6,?7,?8,'confirmed')",
                params![due_id, cf.student_id, cf.period, cf.amount_paise.get(), linked, p.now, p.staff_id, p.device_id],
            )?;
            report.carried_forward += 1;
            report.carried_amount_paise += cf.amount_paise.get();
        }
    }

    // 6. New term-head dues for the enrolled students (new session terms).
    let heads: Vec<FeeHead> = {
        let mut s = tx.prepare("SELECT id, amount_paise, frequency, applies_to FROM fee_head WHERE active=1 AND frequency='term'")?;
        let rows = s.query_map([], |r| {
            Ok(FeeHead {
                id: r.get(0)?,
                amount_paise: Paise(r.get::<_, i64>(1)?),
                frequency: FeeFrequency::Term,
                applies_to: parse_applies_to(&r.get::<_, String>(3)?),
            })
        })?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    if !heads.is_empty() {
        let fee_students: Vec<FeeStudent> = enrolled
            .iter()
            .map(|(sid, cid)| FeeStudent {
                id: sid.clone(),
                class_id: cid.clone(),
                transport: *transport.get(sid).unwrap_or(&false),
                admitted_period: term1.clone(),
            })
            .collect();
        let spec = PeriodSpec { terms: p.terms.iter().map(|t| t.name.clone()).collect(), months: vec![] };
        let dues = vidya_core::fees::generate_dues(&heads, &fee_students, &spec, &[]);
        for d in &dues {
            let due_id = next_id("due");
            tx.execute(
                "INSERT INTO fee_due(id,student_id,fee_head_id,period,amount_paise,created_at,updated_at,updated_by_staff,updated_by_device,sync_state) \
                 VALUES (?1,?2,?3,?4,?5,?6,?6,?7,?8,'confirmed')",
                params![due_id, d.student_id, d.fee_head_id, d.period, d.amount_paise.get(), p.now, p.staff_id, p.device_id],
            )?;
            report.new_dues += 1;
        }
    }

    // 7. One audit entry for the whole rollover.
    let after = serde_json::json!({
        "new_session": p.new_session_id,
        "label": p.new_label,
        "promoted": report.promoted,
        "repeated": report.repeated,
        "passed_out": report.passed_out,
        "left": report.left,
        "new_enrollments": report.new_enrollments,
        "carried_forward": report.carried_forward,
        "carried_amount_paise": report.carried_amount_paise,
        "new_dues": report.new_dues,
    })
    .to_string();
    audit::append(
        &tx,
        &audit::AuditEntry {
            at: p.now.clone(),
            staff_id: p.staff_id.clone(),
            device_id: p.device_id.clone(),
            action: "session_rollover".into(),
            table: Some("academic_session".into()),
            record_id: Some(p.new_session_id.clone()),
            after_json: Some(after),
            reason: Some(format!("started session {}", p.new_label)),
            ..Default::default()
        },
    )?;

    tx.commit()?;
    Ok(report)
}

/// Ensure the special "Previous balance" fee head exists (carry-forward dues point
/// at it). Idempotent.
fn ensure_previous_balance_head(tx: &rusqlite::Transaction) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO fee_head(id,name,name_hi,amount_paise,frequency,applies_to,active) \
         SELECT 'fh-prev-balance','Previous balance','पिछला बकाया',0,'once','all',1 \
         WHERE NOT EXISTS (SELECT 1 FROM fee_head WHERE id='fh-prev-balance')",
        [],
    )?;
    Ok(())
}

fn parse_applies_to(s: &str) -> AppliesTo {
    match s {
        "all" => AppliesTo::All,
        "transport" => AppliesTo::Transport,
        other => serde_json::from_str::<Vec<String>>(other).map(AppliesTo::ClassIds).unwrap_or(AppliesTo::All),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn params_for(overrides: HashMap<String, Override>) -> RolloverParams {
        RolloverParams {
            new_session_id: "sess-2728".into(),
            new_label: "2027–28".into(),
            new_starts_on: "2027-04-01".into(),
            new_ends_on: "2028-03-31".into(),
            terms: vec![
                NewTerm { id: "nt1".into(), name: "T1".into(), starts_on: "2027-04-01".into(), ends_on: "2027-09-30".into() },
                NewTerm { id: "nt2".into(), name: "T2".into(), starts_on: "2027-10-01".into(), ends_on: "2028-03-31".into() },
            ],
            overrides,
            now: "2027-04-01T06:00:00Z".into(),
            staff_id: Some("stf-priya".into()),
            device_id: Some("dev-a1".into()),
            id_prefix: "r1-".into(),
        }
    }

    /// A small school: a current session, classes VII-B and XII-A, a few students,
    /// one with an outstanding due, one to be repeated, one XII to pass out.
    fn small_school() -> Connection {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        let now = "2026-09-23T06:00:00Z";
        c.execute("INSERT INTO academic_session(id,label,starts_on,ends_on,is_current,read_only) VALUES ('sess-2627','2026–27','2026-04-01','2027-03-31',1,0)", []).unwrap();
        c.execute("INSERT INTO term(id,session_id,name,starts_on,ends_on) VALUES ('t1','sess-2627','T1','2026-04-01','2026-09-30')", []).unwrap();
        c.execute("INSERT INTO class(id,name,section,display,sort_order) VALUES ('cls-7b','VII','B','VII-B',9),('cls-12a','XII','A','XII-A',14)", []).unwrap();
        c.execute("INSERT INTO fee_head(id,name,amount_paise,frequency,applies_to,active) VALUES ('fh-t1','Term tuition',600000,'term','all',1)", []).unwrap();
        // students: a (VII-B, has ₹3,000 due), b (VII-B, repeat), c (XII-A, passes out)
        for (id, cls) in [("a", "cls-7b"), ("b", "cls-7b"), ("c", "cls-12a")] {
            c.execute("INSERT INTO student(id,name,status,transport,created_at,updated_at,sync_state) VALUES (?1,?2,'active',0,?3,?3,'confirmed')", params![id, format!("Student {id}"), now]).unwrap();
            c.execute("INSERT INTO enrollment(id,student_id,class_id,session_id,roll_no,from_date,created_at,updated_at,sync_state) VALUES (?1,?2,?3,'sess-2627',1,'2026-04-01',?4,?4,'confirmed')", params![format!("e-{id}"), id, cls, now]).unwrap();
        }
        // a owes ₹3,000 (a due of 3000 with no allocation).
        c.execute("INSERT INTO fee_due(id,student_id,fee_head_id,period,amount_paise,created_at,updated_at,sync_state) VALUES ('d-a','a','fh-t1','T1',300000,?1,?1,'confirmed')", params![now]).unwrap();
        c
    }

    #[test]
    fn rollover_promotes_repeats_and_passes_out() {
        let mut c = small_school();
        let mut overrides = HashMap::new();
        overrides.insert("b".to_string(), Override::Repeat);
        let report = rollover_commit(&mut c, &params_for(overrides)).unwrap();

        // a promotes VII→VIII; b repeats (override); c (XII) passes out.
        assert_eq!(report.promoted, 1);
        assert_eq!(report.repeated, 1);
        assert_eq!(report.passed_out, 1);
        assert_eq!(report.new_enrollments, 2, "a + b enrolled; c passed out");

        // New session is current; old is read-only.
        let (cur, ro): (i64, i64) = c
            .query_row("SELECT is_current, read_only FROM academic_session WHERE id='sess-2627'", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!((cur, ro), (0, 1), "old session retired");
        let new_cur: i64 = c.query_row("SELECT is_current FROM academic_session WHERE id='sess-2728'", [], |r| r.get(0)).unwrap();
        assert_eq!(new_cur, 1);

        // a is now enrolled in VIII-B (created on the fly, section kept).
        let (name, section): (String, String) = c
            .query_row(
                "SELECT c.name, COALESCE(c.section,'') FROM enrollment e JOIN class c ON c.id=e.class_id WHERE e.student_id='a' AND e.session_id='sess-2728'",
                [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!((name.as_str(), section.as_str()), ("VIII", "B"));
        assert!(report.classes_created >= 1, "VIII-B was created");

        // c passed out → status left, reason Passed out, no new enrollment.
        let (status, reason): (String, String) = c
            .query_row("SELECT status, COALESCE(left_reason,'') FROM student WHERE id='c'", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!((status.as_str(), reason.as_str()), ("left", "Passed out"));
        let c_new: i64 = c.query_row("SELECT COUNT(*) FROM enrollment WHERE student_id='c' AND session_id='sess-2728'", [], |r| r.get(0)).unwrap();
        assert_eq!(c_new, 0);
    }

    #[test]
    fn rollover_carries_balance_and_generates_new_dues() {
        let mut c = small_school();
        let report = rollover_commit(&mut c, &params_for(HashMap::new())).unwrap();

        // a's ₹3,000 becomes one Previous balance due in the new Term 1, linked to
        // the old due (which is kept).
        assert_eq!(report.carried_forward, 1);
        assert_eq!(report.carried_amount_paise, 300000);
        let (amt, period, linked): (i64, String, String) = c
            .query_row(
                "SELECT amount_paise, period, COALESCE(carried_from_due_ids,'') FROM fee_due WHERE student_id='a' AND fee_head_id='fh-prev-balance'",
                [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap();
        assert_eq!(amt, 300000);
        assert_eq!(period, "T1");
        assert!(linked.contains("d-a"), "links the old due id");
        // The OLD due still exists (never deleted).
        let old_kept: i64 = c.query_row("SELECT COUNT(*) FROM fee_due WHERE id='d-a'", [], |r| r.get(0)).unwrap();
        assert_eq!(old_kept, 1);

        // New term dues: a (VIII-B) + b (VII-B... but c passed out) → 2 students ×
        // 2 terms × 1 term head = 4 new term dues.
        assert_eq!(report.new_dues, 4);

        // One audit entry for the whole rollover; chain still valid.
        let n: i64 = c.query_row("SELECT COUNT(*) FROM audit_log WHERE action='session_rollover'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
        assert_eq!(audit::verify_chain(&c).unwrap(), None);
    }

    #[test]
    fn rollover_scales_to_1500_students() {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        let now = "2026-09-23T06:00:00Z";
        c.execute("INSERT INTO academic_session(id,label,starts_on,ends_on,is_current,read_only) VALUES ('sess-2627','2026–27','2026-04-01','2027-03-31',1,0)", []).unwrap();
        c.execute("INSERT INTO class(id,name,section,display,sort_order) VALUES ('cls-5a','V','A','V-A',7)", []).unwrap();
        c.execute("INSERT INTO fee_head(id,name,amount_paise,frequency,applies_to,active) VALUES ('fh-t1','Term tuition',600000,'term','all',1)", []).unwrap();
        {
            let tx = c.transaction().unwrap();
            for i in 0..1500 {
                let sid = format!("s{i}");
                tx.execute("INSERT INTO student(id,name,status,transport,created_at,updated_at,sync_state) VALUES (?1,?2,'active',0,?3,?3,'confirmed')", params![sid, format!("Student {i}"), now]).unwrap();
                tx.execute("INSERT INTO enrollment(id,student_id,class_id,session_id,roll_no,from_date,created_at,updated_at,sync_state) VALUES (?1,?2,'cls-5a','sess-2627',?3,'2026-04-01',?4,?4,'confirmed')", params![format!("e{i}"), sid, (i as i64)+1, now]).unwrap();
                // Every 3rd student owes ₹1,000.
                if i % 3 == 0 {
                    tx.execute("INSERT INTO fee_due(id,student_id,fee_head_id,period,amount_paise,created_at,updated_at,sync_state) VALUES (?1,?2,'fh-t1','T1',100000,?3,?3,'confirmed')", params![format!("d{i}"), sid, now]).unwrap();
                }
            }
            tx.commit().unwrap();
        }

        let report = rollover_commit(&mut c, &params_for(HashMap::new())).unwrap();
        assert_eq!(report.promoted, 1500, "all 1,500 promote V→VI");
        assert_eq!(report.new_enrollments, 1500);
        assert_eq!(report.carried_forward, 500, "every 3rd student (500) carries a balance");
        assert_eq!(report.carried_amount_paise, 500 * 100000);
        assert_eq!(report.new_dues, 1500 * 2, "1500 students × 2 terms");
        // All 1,500 are in VI-A now.
        let in_vi: i64 = c.query_row(
            "SELECT COUNT(*) FROM enrollment e JOIN class c ON c.id=e.class_id WHERE e.session_id='sess-2728' AND c.name='VI' AND c.section='A'",
            [], |r| r.get(0)).unwrap();
        assert_eq!(in_vi, 1500);
        assert_eq!(audit::verify_chain(&c).unwrap(), None);
    }
}
