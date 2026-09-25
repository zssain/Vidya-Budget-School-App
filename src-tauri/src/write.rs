//! One write = one transaction (prompts/P02 Step 5). `with_write` runs the domain
//! change, appends the hash-chained audit entry, and appends the op to `op_log`
//! (Server mode) or `outbox` (Client mode) — all-or-nothing.

use crate::security::audit::{self, AuditEntry};
use rusqlite::{params, Connection, Transaction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceMode {
    /// School server: writes `confirmed` rows + `op_log(server_seq)`.
    Server,
    /// Staff device: writes `on_device` rows + `outbox`.
    Client,
}

/// The op recorded for a change (docs/00-SYSTEM-CONTEXT.md §8.1).
#[derive(Debug, Clone)]
pub struct Op {
    pub op_id: String,
    pub hlc: String,
    pub device_id: String,
    pub staff_id: String,
    pub audience: String,
    pub table: String,
    pub record_id: String,
    pub kind: String, // insert | update | delete | action
    pub payload: String,
    pub base_version: Option<i64>,
    pub server_epoch: i64,
}

pub struct WriteCtx {
    pub mode: DeviceMode,
}

/// What a domain change produces: its value, the audit entry, and the op.
pub struct Effect<T> {
    pub value: T,
    pub audit: AuditEntry,
    pub op: Op,
}

/// Append one op to `op_log` (Server) or `outbox` (Client) on an open
/// transaction. Use this when a single logical change emits **several** ops
/// (e.g. the seven `school_week` rows of a weekly-offs edit) inside one
/// transaction; for the common one-op change use [`with_write`].
pub fn append_op(tx: &Transaction, mode: DeviceMode, op: &Op) -> rusqlite::Result<()> {
    match mode {
        DeviceMode::Server => {
            tx.execute(
                "INSERT INTO op_log(op_id, hlc, device_id, staff_id, \"table\", record_id, kind, payload, applied_at) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![op.op_id, op.hlc, op.device_id, op.staff_id, op.table, op.record_id, op.kind, op.payload, op.hlc],
            )?;
        }
        DeviceMode::Client => {
            tx.execute(
                "INSERT INTO outbox(op_id, hlc, device_id, staff_id, audience, \"table\", record_id, kind, payload, base_version, server_epoch) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![op.op_id, op.hlc, op.device_id, op.staff_id, op.audience, op.table, op.record_id, op.kind, op.payload, op.base_version, op.server_epoch],
            )?;
        }
    }
    Ok(())
}

/// Run `f` and its bookkeeping in ONE transaction; roll everything back on any error.
pub fn with_write<T>(
    conn: &mut Connection,
    ctx: &WriteCtx,
    f: impl FnOnce(&Transaction) -> rusqlite::Result<Effect<T>>,
) -> rusqlite::Result<T> {
    let tx = conn.transaction()?;
    let eff = f(&tx)?;
    audit::append(&tx, &eff.audit)?;
    append_op(&tx, ctx.mode, &eff.op)?;
    tx.commit()?;
    Ok(eff.value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn fresh() -> Connection {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        c
    }

    fn op(kind: &str) -> Op {
        Op {
            op_id: "op-1".into(),
            hlc: "0000000000000000010001dev".into(),
            device_id: "dev".into(),
            staff_id: "st1".into(),
            audience: "admin".into(),
            table: "student".into(),
            record_id: "s1".into(),
            kind: kind.into(),
            payload: "{}".into(),
            base_version: None,
            server_epoch: 1,
        }
    }

    fn insert_student(tx: &Transaction) -> rusqlite::Result<()> {
        tx.execute(
            "INSERT INTO student(id, name, created_at, updated_at, sync_state) VALUES ('s1','Riya','t','t',?1)",
            params!["on_device"],
        )?;
        Ok(())
    }

    fn counts(conn: &Connection) -> (i64, i64, i64, i64) {
        let s: i64 = conn.query_row("SELECT count(*) FROM student", [], |r| r.get(0)).unwrap();
        let a: i64 = conn.query_row("SELECT count(*) FROM audit_log", [], |r| r.get(0)).unwrap();
        let o: i64 = conn.query_row("SELECT count(*) FROM outbox", [], |r| r.get(0)).unwrap();
        let l: i64 = conn.query_row("SELECT count(*) FROM op_log", [], |r| r.get(0)).unwrap();
        (s, a, o, l)
    }

    #[test]
    fn client_write_commits_row_audit_and_outbox() {
        let mut conn = fresh();
        with_write(&mut conn, &WriteCtx { mode: DeviceMode::Client }, |tx| {
            insert_student(tx)?;
            Ok(Effect {
                value: (),
                audit: AuditEntry { at: "t".into(), action: "create_student".into(), ..Default::default() },
                op: op("insert"),
            })
        })
        .unwrap();
        assert_eq!(counts(&conn), (1, 1, 1, 0));
    }

    #[test]
    fn server_write_uses_op_log() {
        let mut conn = fresh();
        with_write(&mut conn, &WriteCtx { mode: DeviceMode::Server }, |tx| {
            insert_student(tx)?;
            Ok(Effect {
                value: (),
                audit: AuditEntry { at: "t".into(), action: "create_student".into(), ..Default::default() },
                op: op("insert"),
            })
        })
        .unwrap();
        assert_eq!(counts(&conn), (1, 1, 0, 1));
    }

    #[test]
    fn failure_after_row_write_rolls_everything_back() {
        let mut conn = fresh();
        let res: rusqlite::Result<()> =
            with_write(&mut conn, &WriteCtx { mode: DeviceMode::Client }, |tx| {
                insert_student(tx)?; // row written inside the tx...
                Err(rusqlite::Error::InvalidQuery) // ...then a failure is injected
            });
        assert!(res.is_err());
        // nothing persisted: no row, no audit, no op
        assert_eq!(counts(&conn), (0, 0, 0, 0));
    }
}
