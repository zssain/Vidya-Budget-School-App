//! Server-enforced role scopes (prompts/P04 Step 4, docs §8.8). Produces the set
//! of rows (and columns) a device may see, for `/sync/snapshot` and `/sync/pull`.
//!
//! Teacher → school/session/terms, own classes + class-subjects, their students
//! (NO fee tables; guardian address only if class teacher), their sheets, own
//! requests, staff NAMES. Accountant → all students + all fee data, no attendance
//! marks, no marks. Principal → everything.

use std::collections::BTreeSet;

use rusqlite::Connection;
use vidya_core::permissions::Actor;
use vidya_core::types::Role;

use crate::sync::apply::read_row_json;
use crate::sync::protocol::Change;

/// Tables a teacher device must NEVER receive (DONE-MEANS #4).
pub const FEE_TABLES: &[&str] = &["fee_head", "fee_due", "payment", "payment_allocation", "reversal"];
/// Tables an accountant device must never receive (attendance/marks).
pub const MARK_TABLES: &[&str] = &["attendance_sheet", "attendance_mark", "marks_sheet", "mark_entry"];

/// Columns of `staff` a non-Principal device may see (names only, no secrets).
const STAFF_PUBLIC: &[&str] = &["id", "name", "role", "state"];

/// The class ids a teacher actor is scoped to (class-teacher-of ∪ taught classes).
fn teacher_class_ids(conn: &Connection, actor: &Actor) -> rusqlite::Result<BTreeSet<String>> {
    let mut ids: BTreeSet<String> = actor.class_teacher_of.iter().cloned().collect();
    if !actor.class_subjects.is_empty() {
        let placeholders = actor.class_subjects.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("SELECT class_id FROM class_subject WHERE id IN ({placeholders})");
        let mut stmt = conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = actor.class_subjects.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt.query_map(params.as_slice(), |r| r.get::<_, String>(0))?;
        for c in rows {
            ids.insert(c?);
        }
    }
    Ok(ids)
}

/// Can this role receive rows of `table` at all?
pub fn can_see_table(role: Role, table: &str) -> bool {
    match role {
        Role::Principal => true,
        Role::Accountant => !MARK_TABLES.contains(&table),
        Role::Teacher => !FEE_TABLES.contains(&table),
    }
}

fn ids_of(conn: &Connection, sql: &str, args: &[&dyn rusqlite::ToSql]) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(args, |r| r.get::<_, String>(0))?;
    rows.collect()
}

/// Build the full set of rows a device with this actor may see (snapshot).
pub fn snapshot(conn: &Connection, actor: &Actor) -> rusqlite::Result<Vec<Change>> {
    let mut out: Vec<Change> = Vec::new();
    let mut push = |conn: &Connection, table: &str, id: &str, filter: Option<&dyn Fn(&mut serde_json::Value)>| -> rusqlite::Result<()> {
        if let Some(mut row) = read_row_json(conn, table, id)? {
            if let Some(f) = filter {
                f(&mut row);
            }
            out.push(Change { table: table.into(), record_id: id.into(), payload: row, server_seq: 0, hlc: None });
        }
        Ok(())
    };

    // Everyone: school, sessions, terms, subjects.
    for id in ids_of(conn, "SELECT id FROM school", &[])? { push(conn, "school", &id, None)?; }
    for id in ids_of(conn, "SELECT id FROM academic_session", &[])? { push(conn, "academic_session", &id, None)?; }
    for id in ids_of(conn, "SELECT id FROM term", &[])? { push(conn, "term", &id, None)?; }
    for id in ids_of(conn, "SELECT id FROM subject", &[])? { push(conn, "subject", &id, None)?; }

    // Staff — names only for non-Principal.
    let staff_filter = |row: &mut serde_json::Value| {
        if let Some(obj) = row.as_object_mut() {
            obj.retain(|k, _| STAFF_PUBLIC.contains(&k.as_str()));
        }
    };
    for id in ids_of(conn, "SELECT id FROM staff", &[])? {
        let f: Option<&dyn Fn(&mut serde_json::Value)> = if actor.role == Role::Principal { None } else { Some(&staff_filter) };
        push(conn, "staff", &id, f)?;
    }

    match actor.role {
        Role::Principal => {
            for t in ["class", "class_subject", "student", "enrollment", "attendance_sheet", "attendance_mark",
                      "fee_head", "fee_due", "payment", "payment_allocation", "reversal", "request",
                      "marks_sheet", "mark_entry", "exam", "exam_subject"] {
                for id in ids_of(conn, &format!("SELECT id FROM {t}"), &[])? {
                    push(conn, t, &id, None)?;
                }
            }
        }
        Role::Accountant => {
            for t in ["class", "class_subject", "student", "enrollment", "fee_head", "fee_due", "payment", "payment_allocation", "reversal"] {
                for id in ids_of(conn, &format!("SELECT id FROM {t}"), &[])? {
                    push(conn, t, &id, None)?;
                }
            }
            // Own requests only.
            for id in ids_of(conn, "SELECT id FROM request WHERE requested_by=?1", &[&actor.staff_id])? {
                push(conn, "request", &id, None)?;
            }
        }
        Role::Teacher => {
            let classes = teacher_class_ids(conn, actor)?;
            if classes.is_empty() {
                return Ok(out);
            }
            let list: Vec<String> = classes.iter().cloned().collect();
            let ph = list.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let cargs: Vec<&dyn rusqlite::ToSql> = list.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

            for id in ids_of(conn, &format!("SELECT id FROM class WHERE id IN ({ph})"), cargs.as_slice())? { push(conn, "class", &id, None)?; }
            for id in ids_of(conn, "SELECT id FROM class_subject WHERE teacher_id=?1", &[&actor.staff_id])? { push(conn, "class_subject", &id, None)?; }

            // Students enrolled in the teacher's classes; guardian address only for
            // classes where this teacher is the class teacher.
            let ct: BTreeSet<String> = actor.class_teacher_of.iter().cloned().collect();
            let student_rows = {
                let mut stmt = conn.prepare(&format!(
                    "SELECT DISTINCT s.id, e.class_id FROM student s JOIN enrollment e ON e.student_id=s.id AND e.to_date IS NULL WHERE e.class_id IN ({ph})"
                ))?;
                let rows = stmt.query_map(cargs.as_slice(), |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
                rows.collect::<rusqlite::Result<Vec<(String, String)>>>()?
            };
            for (sid, cid) in student_rows {
                let is_ct = ct.contains(&cid);
                let addr_filter = |row: &mut serde_json::Value| {
                    if let Some(obj) = row.as_object_mut() {
                        obj.insert("address".into(), serde_json::Value::Null);
                    }
                };
                let f: Option<&dyn Fn(&mut serde_json::Value)> = if is_ct { None } else { Some(&addr_filter) };
                push(conn, "student", &sid, f)?;
            }
            for id in ids_of(conn, &format!("SELECT id FROM enrollment WHERE class_id IN ({ph}) AND to_date IS NULL"), cargs.as_slice())? { push(conn, "enrollment", &id, None)?; }
            for id in ids_of(conn, &format!("SELECT id FROM attendance_sheet WHERE class_id IN ({ph})"), cargs.as_slice())? { push(conn, "attendance_sheet", &id, None)?; }
            // Marks for those sheets.
            let sheet_ids = ids_of(conn, &format!("SELECT id FROM attendance_sheet WHERE class_id IN ({ph})"), cargs.as_slice())?;
            for sh in &sheet_ids {
                for id in ids_of(conn, "SELECT id FROM attendance_mark WHERE sheet_id=?1", &[sh])? {
                    push(conn, "attendance_mark", &id, None)?;
                }
            }
            for id in ids_of(conn, "SELECT id FROM request WHERE requested_by=?1", &[&actor.staff_id])? { push(conn, "request", &id, None)?; }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use vidya_core::types::StaffState;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn seeded() -> Connection {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        let now = time::OffsetDateTime::parse("2026-09-23T09:00:00Z", &time::format_description::well_known::Rfc3339).unwrap();
        crate::seed::seed_demo_school(&mut c, now).unwrap();
        c
    }

    fn actor(role: Role, id: &str, ct: &[&str], cs: &[&str]) -> Actor {
        Actor {
            staff_id: id.into(), role, state: StaffState::Active,
            class_teacher_of: ct.iter().map(|s| s.to_string()).collect(),
            class_subjects: cs.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn tables(changes: &[Change]) -> BTreeSet<String> {
        changes.iter().map(|c| c.table.clone()).collect()
    }

    #[test]
    fn teacher_gets_no_fee_rows_and_only_their_students() {
        let c = seeded();
        // Meena is class teacher of V-A (cls-5a) in the seed.
        let a = actor(Role::Teacher, "stf-meena", &["cls-5a"], &[]);
        let snap = snapshot(&c, &a).unwrap();
        let ts = tables(&snap);
        // DONE-MEANS #4: no fee tables at all.
        for t in FEE_TABLES {
            assert!(!ts.contains(*t), "teacher must not receive {t}");
        }
        // Students are only V-A's 34; none outside the class.
        let students: Vec<&Change> = snap.iter().filter(|c| c.table == "student").collect();
        assert_eq!(students.len(), 34, "teacher sees exactly their class roster");
        // As class teacher, addresses are visible (not nulled) — spot-check one has the key.
        assert!(students[0].payload.get("address").is_some());
    }

    #[test]
    fn teacher_without_class_teacher_role_hides_address() {
        let c = seeded();
        // A teacher who only teaches VI-B Maths (cs-6b-maths) but isn't the class teacher.
        let a = actor(Role::Teacher, "stf-anita", &[], &["cs-6b-maths"]);
        let snap = snapshot(&c, &a).unwrap();
        let students: Vec<&Change> = snap.iter().filter(|c| c.table == "student").collect();
        assert!(!students.is_empty());
        for s in students {
            assert!(s.payload.get("address").unwrap().is_null(), "guardian address hidden for non-class-teacher");
        }
    }

    #[test]
    fn accountant_sees_fees_but_no_attendance_marks() {
        let c = seeded();
        let a = actor(Role::Accountant, "stf-suresh", &[], &[]);
        let snap = snapshot(&c, &a).unwrap();
        let ts = tables(&snap);
        assert!(ts.contains("payment"), "accountant sees payments");
        assert!(ts.contains("fee_due"), "accountant sees dues");
        for t in MARK_TABLES {
            assert!(!ts.contains(*t), "accountant must not receive {t}");
        }
        // All 670 students visible.
        assert_eq!(snap.iter().filter(|c| c.table == "student").count(), 670);
    }

    #[test]
    fn staff_rows_are_names_only_for_non_principal() {
        let c = seeded();
        let a = actor(Role::Accountant, "stf-suresh", &[], &[]);
        let snap = snapshot(&c, &a).unwrap();
        let staff: Vec<&Change> = snap.iter().filter(|c| c.table == "staff").collect();
        assert!(!staff.is_empty());
        for s in staff {
            assert!(s.payload.get("pin_hash").is_none(), "no pin_hash leaked");
            assert!(s.payload.get("name").is_some());
        }
    }

    #[test]
    fn principal_sees_everything() {
        let c = seeded();
        let a = actor(Role::Principal, "stf-priya", &[], &[]);
        let snap = snapshot(&c, &a).unwrap();
        let ts = tables(&snap);
        assert!(ts.contains("payment") && ts.contains("attendance_mark"));
        // Principal staff rows keep secrets (full row).
        let staff: Vec<&Change> = snap.iter().filter(|c| c.table == "staff").collect();
        assert!(staff.iter().any(|s| s.payload.get("pin_hash").is_some()));
    }
}
