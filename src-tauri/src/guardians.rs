//! guardians — repository for the v2 guardian tables (P13, foundation §8.2).
//!
//! The `0009` migration backfills existing (v1) students. New students created in
//! v2 (create_student, CSV import, the demo seed) call [`ensure_primary_guardian`]
//! here, which uses the SAME find-or-create-by-(mobile, name) rule so siblings
//! share one guardian row. The old `student.guardian_*` columns are kept in sync
//! for read-only compatibility until Phase 18 drops them.

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

/// One guardian linked to a student, for the profile UI.
#[derive(Debug, Clone, Serialize)]
pub struct GuardianDto {
    pub id: String,
    pub name: String,
    pub relation: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub language: String,
    pub whatsapp_ok: bool,
    pub is_primary: bool,
}

/// Find-or-create the student's **primary** guardian by exact (mobile, name), then
/// link the student to it (is_primary = 1). Runs inside an existing transaction
/// (called from create_student's `with_write`, like enrollment/dues). Returns the
/// guardian id, or `None` if there is no guardian data at all.
pub fn ensure_primary_guardian(
    tx: &rusqlite::Transaction,
    student_id: &str,
    name: Option<&str>,
    mobile: Option<&str>,
    school_id: Option<&str>,
    now: &str,
    sync_state: &str,
) -> rusqlite::Result<Option<String>> {
    let name = name.unwrap_or("").trim();
    let mob = mobile.unwrap_or("").trim();
    if name.is_empty() && mob.is_empty() {
        return Ok(None);
    }
    // Find an existing guardian with the same (mobile, name).
    let existing: Option<String> = tx
        .query_row(
            "SELECT id FROM guardian WHERE COALESCE(mobile,'')=?1 AND name=?2 LIMIT 1",
            rusqlite::params![mob, name],
            |r| r.get(0),
        )
        .optional()?;
    let gid = match existing {
        Some(id) => id,
        None => {
            let id = format!("grd-{}", uuid::Uuid::now_v7());
            tx.execute(
                "INSERT INTO guardian(id,name,mobile,language,whatsapp_ok,school_id,created_at,updated_at,sync_state) \
                 VALUES (?1,?2,NULLIF(?3,''),'en',1,?4,?5,?5,?6)",
                rusqlite::params![id, name, mob, school_id, now, sync_state],
            )?;
            id
        }
    };
    tx.execute(
        "INSERT OR IGNORE INTO student_guardian(id,student_id,guardian_id,is_primary,school_id,created_at,updated_at,sync_state) \
         VALUES (?1,?2,?3,1,?4,?5,?5,?6)",
        rusqlite::params![format!("sg-{}", uuid::Uuid::now_v7()), student_id, gid, school_id, now, sync_state],
    )?;
    Ok(Some(gid))
}

/// Guardians linked to a student, primary first.
pub fn list_for_student(conn: &Connection, student_id: &str) -> rusqlite::Result<Vec<GuardianDto>> {
    let mut stmt = conn.prepare(
        "SELECT g.id, g.name, g.relation, g.mobile, g.email, g.language, g.whatsapp_ok, sg.is_primary \
         FROM student_guardian sg JOIN guardian g ON g.id=sg.guardian_id \
         WHERE sg.student_id=?1 ORDER BY sg.is_primary DESC, g.name",
    )?;
    let rows = stmt.query_map([student_id], |r| {
        Ok(GuardianDto {
            id: r.get(0)?,
            name: r.get(1)?,
            relation: r.get(2)?,
            mobile: r.get(3)?,
            email: r.get(4)?,
            language: r.get(5)?,
            whatsapp_ok: r.get::<_, i64>(6)? != 0,
            is_primary: r.get::<_, i64>(7)? != 0,
        })
    })?;
    rows.collect()
}

/// How many guardians a student has (for the ≤ 2 limit).
pub fn count_for_student(conn: &Connection, student_id: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM student_guardian WHERE student_id=?1",
        [student_id],
        |r| r.get(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn fresh() -> Connection {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        c.execute("INSERT INTO school(id,name,backup_salt,created_at,updated_at) VALUES ('sch','S',x'00','t','t')", []).unwrap();
        c
    }

    fn add_student(conn: &Connection, id: &str, name: &str) {
        conn.execute(
            "INSERT INTO student(id,name,created_at,updated_at) VALUES (?1,?2,'t','t')",
            rusqlite::params![id, name],
        )
        .unwrap();
    }

    #[test]
    fn siblings_share_one_guardian_row() {
        let mut c = fresh();
        add_student(&c, "s1", "Riya");
        add_student(&c, "s2", "Rahul");
        let tx = c.transaction().unwrap();
        let g1 = ensure_primary_guardian(&tx, "s1", Some("Ramesh Kumar"), Some("9876543210"), Some("sch"), "t", "confirmed").unwrap();
        let g2 = ensure_primary_guardian(&tx, "s2", Some("Ramesh Kumar"), Some("9876543210"), Some("sch"), "t", "confirmed").unwrap();
        tx.commit().unwrap();
        assert_eq!(g1, g2, "same mobile+name → same guardian row");
        let count: i64 = c.query_row("SELECT COUNT(*) FROM guardian", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
        assert_eq!(list_for_student(&c, "s1").unwrap().len(), 1);
        assert!(list_for_student(&c, "s1").unwrap()[0].is_primary);
    }

    #[test]
    fn no_guardian_data_creates_nothing() {
        let mut c = fresh();
        add_student(&c, "s1", "Riya");
        let tx = c.transaction().unwrap();
        let g = ensure_primary_guardian(&tx, "s1", None, None, Some("sch"), "t", "confirmed").unwrap();
        tx.commit().unwrap();
        assert_eq!(g, None);
        assert_eq!(c.query_row("SELECT COUNT(*) FROM guardian", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
    }
}
