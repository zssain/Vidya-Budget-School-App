//! Append-only, hash-chained audit log (docs/00-SYSTEM-CONTEXT.md §9,
//! prompts/P02 Step 4). `hash = SHA-256(prev_hash ‖ canonical_json(entry))`
//! where canonical JSON has sorted keys and no whitespace.

use rusqlite::{Connection, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct AuditEntry {
    pub at: String,
    pub staff_id: Option<String>,
    pub device_id: Option<String>,
    pub action: String,
    pub table: Option<String>,
    pub record_id: Option<String>,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    pub reason: Option<String>,
    pub op_id: Option<String>,
}

impl AuditEntry {
    /// Canonical JSON: BTreeMap → sorted keys, compact (no whitespace).
    fn canonical(&self) -> String {
        let mut m: BTreeMap<&str, serde_json::Value> = BTreeMap::new();
        m.insert("at", json_str(&self.at));
        m.insert("staff_id", json_opt(&self.staff_id));
        m.insert("device_id", json_opt(&self.device_id));
        m.insert("action", json_str(&self.action));
        m.insert("table", json_opt(&self.table));
        m.insert("record_id", json_opt(&self.record_id));
        m.insert("before_json", json_opt(&self.before_json));
        m.insert("after_json", json_opt(&self.after_json));
        m.insert("reason", json_opt(&self.reason));
        m.insert("op_id", json_opt(&self.op_id));
        serde_json::to_string(&m).expect("canonical json")
    }
}

fn json_str(s: &str) -> serde_json::Value {
    serde_json::Value::String(s.to_string())
}
fn json_opt(s: &Option<String>) -> serde_json::Value {
    match s {
        Some(v) => serde_json::Value::String(v.clone()),
        None => serde_json::Value::Null,
    }
}

fn chain_hash(prev_hash: &str, canonical: &str) -> String {
    let mut h = Sha256::new();
    h.update(prev_hash.as_bytes());
    h.update(canonical.as_bytes());
    hex(&h.finalize())
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// The current chain head hash (empty string for an empty log).
pub fn head_hash(conn: &Connection) -> rusqlite::Result<String> {
    Ok(conn
        .query_row("SELECT hash FROM audit_log ORDER BY seq DESC LIMIT 1", [], |r| r.get::<_, String>(0))
        .optional()?
        .unwrap_or_default())
}

/// Append an entry inside an open transaction. Returns (seq, hash).
pub fn append(tx: &Transaction, entry: &AuditEntry) -> rusqlite::Result<(i64, String)> {
    let prev = tx
        .query_row("SELECT hash FROM audit_log ORDER BY seq DESC LIMIT 1", [], |r| r.get::<_, String>(0))
        .optional()?
        .unwrap_or_default();
    let hash = chain_hash(&prev, &entry.canonical());
    tx.execute(
        "INSERT INTO audit_log \
         (at, staff_id, device_id, action, \"table\", record_id, before_json, after_json, reason, op_id, prev_hash, hash) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        rusqlite::params![
            entry.at, entry.staff_id, entry.device_id, entry.action, entry.table,
            entry.record_id, entry.before_json, entry.after_json, entry.reason, entry.op_id,
            prev, hash,
        ],
    )?;
    Ok((tx.last_insert_rowid(), hash))
}

/// Walk the whole chain; return the first `seq` whose linkage or hash is bad,
/// or `None` if the chain verifies.
pub fn verify_chain(conn: &Connection) -> rusqlite::Result<Option<i64>> {
    let mut stmt = conn.prepare(
        "SELECT seq, at, staff_id, device_id, action, \"table\", record_id, before_json, after_json, reason, op_id, prev_hash, hash \
         FROM audit_log ORDER BY seq ASC",
    )?;
    let mut rows = stmt.query([])?;
    let mut prev = String::new();
    while let Some(row) = rows.next()? {
        let seq: i64 = row.get(0)?;
        let entry = AuditEntry {
            at: row.get(1)?,
            staff_id: row.get(2)?,
            device_id: row.get(3)?,
            action: row.get(4)?,
            table: row.get(5)?,
            record_id: row.get(6)?,
            before_json: row.get(7)?,
            after_json: row.get(8)?,
            reason: row.get(9)?,
            op_id: row.get(10)?,
        };
        let stored_prev: String = row.get(11)?;
        let stored_hash: String = row.get(12)?;
        if stored_prev != prev {
            return Ok(Some(seq));
        }
        if chain_hash(&prev, &entry.canonical()) != stored_hash {
            return Ok(Some(seq));
        }
        prev = stored_hash;
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn entry(action: &str) -> AuditEntry {
        AuditEntry { at: "2026-09-23T06:00:00Z".into(), action: action.into(), ..Default::default() }
    }

    fn fresh() -> Connection {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        c
    }

    #[test]
    fn append_builds_a_verifiable_chain() {
        let mut conn = fresh();
        {
            let tx = conn.transaction().unwrap();
            append(&tx, &entry("create_student")).unwrap();
            append(&tx, &entry("record_payment")).unwrap();
            append(&tx, &entry("submit_attendance")).unwrap();
            tx.commit().unwrap();
        }
        assert_eq!(verify_chain(&conn).unwrap(), None);
        assert!(!head_hash(&conn).unwrap().is_empty());
    }

    #[test]
    fn tampering_a_row_fails_and_names_first_bad_seq() {
        let mut conn = fresh();
        {
            let tx = conn.transaction().unwrap();
            append(&tx, &entry("a")).unwrap();
            append(&tx, &entry("b")).unwrap();
            append(&tx, &entry("c")).unwrap();
            tx.commit().unwrap();
        }
        // UPDATE is blocked by the append-only trigger, so simulate a tamper by
        // recording the chain into a fresh table-shaped copy where row 2's stored
        // content differs from what its hash covers. We do this by inserting a bad
        // row directly (INSERT is allowed) with the WRONG hash at seq 2's place.
        // Build a brand-new corrupt chain to assert detection at the exact seq.
        let c2 = fresh();
        c2.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
        // seq 1: good
        let good = chain_hash("", &entry("a").canonical());
        c2.execute(
            "INSERT INTO audit_log(seq, at, action, prev_hash, hash) VALUES (1, ?1, 'a', '', ?2)",
            rusqlite::params!["2026-09-23T06:00:00Z", good],
        )
        .unwrap();
        // seq 2: stored content 'b' but hash computed over 'X' (tampered)
        let wrong = chain_hash(&good, &entry("X").canonical());
        c2.execute(
            "INSERT INTO audit_log(seq, at, action, prev_hash, hash) VALUES (2, ?1, 'b', ?2, ?3)",
            rusqlite::params!["2026-09-23T06:00:00Z", good, wrong],
        )
        .unwrap();
        assert_eq!(verify_chain(&c2).unwrap(), Some(2), "must name the first bad seq");
    }
}
