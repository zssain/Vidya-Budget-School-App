//! Device provisional pull from Drive (prompts/P06 Step 5).
//!
//! While the school server is unreachable, a device reads OTHER devices' bundles
//! it is allowed to decrypt and applies them as **provisional** rows
//! (`sync_state='shared_drive'`) marked "Shared through school Drive · waiting for
//! school" (§8.10) — never `confirmed`. The server re-validates everything on
//! import (rule §6), so a device does no permission check here: holding the
//! audience key already means the op is within the device's scope.
//!
//! Rules honoured (§8.3 route 3, Step 5):
//! * download only bundles whose (audience, version) key the device holds; a
//!   bundle for an audience it lacks is "not for me" and skipped silently;
//! * a bundle it SHOULD read but that fails AEAD → **quarantine** (tampered);
//! * **never overwrite the device's own unsent changes** — if the device has an
//!   outbox op for the same record, keep local and flag instead;
//! * a **cursor per folder** (in `drive_state`) so a bundle is processed once;
//! * payments land provisional (`shared_drive`), so the existing dashboards count
//!   them in "waiting for server", never in confirmed totals.

use std::collections::BTreeSet;

use rusqlite::{params, Connection, OptionalExtension};

use super::bundle::{self, PROP_AUDIENCE, PROP_KEYVER};
use super::exchange::{ExchangeError, HeldKeys};
use super::DriveApi;
use crate::sync::protocol::Op;

/// The exact §8.10 status a provisional row shows.
pub const PROVISIONAL_STATUS: &str = "Shared through school Drive · waiting for school";

/// Provisional-row `sync_state`.
const SHARED_DRIVE: &str = "shared_drive";

fn is_synced(table: &str) -> bool {
    matches!(
        table,
        "school" | "staff" | "student" | "enrollment" | "attendance_sheet" | "marks_sheet" | "fee_due" | "payment" | "request"
    )
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PullOutcome {
    pub bundles_seen: usize,
    /// ops applied as provisional rows.
    pub applied: usize,
    /// bundles for an audience/version this device doesn't hold (not tampered).
    pub skipped_not_for_me: usize,
    /// bundles this device SHOULD read but that failed AEAD (tampered).
    pub quarantined: usize,
    /// ops that would have overwritten the device's own unsent change → flagged.
    pub kept_local: usize,
}

enum Prov {
    Applied,
    KeptLocal,
    AlreadySeen,
}

/// Pull provisional changes from every OTHER device's folder under `exchange`.
pub fn pull_provisional(
    conn: &mut Connection,
    drive: &impl DriveApi,
    exchange_folder: &str,
    own_device_id: &str,
    held: &HeldKeys,
) -> Result<PullOutcome, ExchangeError> {
    let mut out = PullOutcome::default();
    let own_folder = format!("ops-{own_device_id}");

    // discover other devices' ops-* folders
    let folders: Vec<(String, String)> = drive
        .list(exchange_folder)?
        .into_iter()
        .filter(|f| f.is_folder && f.name.starts_with("ops-") && f.name != own_folder)
        .map(|f| (f.id, f.name))
        .collect();

    for (folder_id, folder_name) in &folders {
        let mut seen = cursor_get(conn, folder_name)?;
        for f in drive.list(folder_id)? {
            if f.is_folder || !f.name.ends_with(".vop") || seen.contains(&f.name) {
                continue;
            }
            out.bundles_seen += 1;
            let audience = f.properties.get(PROP_AUDIENCE).cloned().unwrap_or_default();
            let version: i64 = f.properties.get(PROP_KEYVER).and_then(|v| v.parse().ok()).unwrap_or(0);
            let Some(key) = held.get(&(audience.clone(), version)) else {
                out.skipped_not_for_me += 1;
                seen.insert(f.name.clone()); // won't become readable later; don't re-scan
                continue;
            };
            let body = drive.download(&f.id)?;
            match bundle::open_bundle(key, &audience, version, &body) {
                Ok(ops) => {
                    for op in &ops {
                        match provisional_apply(conn, op)? {
                            Prov::Applied => out.applied += 1,
                            Prov::KeptLocal => out.kept_local += 1,
                            Prov::AlreadySeen => {}
                        }
                    }
                    seen.insert(f.name.clone());
                }
                Err(_) => {
                    // held the key but it didn't open → tampered → quarantine + notify.
                    out.quarantined += 1;
                    quarantine(conn, folder_name, &f.name)?;
                    seen.insert(f.name.clone());
                }
            }
        }
        cursor_set(conn, folder_name, &seen)?;
    }
    Ok(out)
}

/// Apply one op provisionally (no op_log, no confirm). Idempotent by op_id; never
/// overwrites the device's own unsent change to the same record.
fn provisional_apply(conn: &mut Connection, op: &Op) -> rusqlite::Result<Prov> {
    // idempotent: this device already saw this op (provisionally or confirmed)
    if seen_op(conn, &op.op_id)? {
        return Ok(Prov::AlreadySeen);
    }
    // never overwrite own unsent: an outbox op for the same record = a local edit
    let own_unsent: bool = conn
        .query_row("SELECT 1 FROM outbox WHERE record_id=?1 LIMIT 1", params![op.record_id], |_| Ok(()))
        .optional()?
        .is_some();
    if own_unsent {
        remember_op(conn, &op.op_id)?;
        return Ok(Prov::KeptLocal);
    }
    provisional_upsert(conn, op)?;
    remember_op(conn, &op.op_id)?;
    Ok(Prov::Applied)
}

/// Upsert the row carried in the op, marking it provisional. Child tables (no sync
/// columns) just get their payload columns.
fn provisional_upsert(conn: &Connection, op: &Op) -> rusqlite::Result<()> {
    let obj = op.payload.as_object().cloned().unwrap_or_default();
    let synced = is_synced(&op.table);
    let exists: bool = conn
        .query_row(&format!("SELECT 1 FROM {} WHERE id=?1", op.table), params![op.record_id], |_| Ok(()))
        .optional()?
        .is_some();

    if exists {
        let mut sets: Vec<String> = Vec::new();
        let mut vals: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        for (col, val) in &obj {
            if col == "id" {
                continue;
            }
            sets.push(format!("{col}=?"));
            vals.push(json_to_sql(val));
        }
        if synced {
            sets.push("hlc=?".into());
            vals.push(Box::new(op.hlc.clone()));
            sets.push(format!("sync_state='{SHARED_DRIVE}'"));
        }
        if sets.is_empty() {
            return Ok(());
        }
        vals.push(Box::new(op.record_id.clone()));
        let sql = format!("UPDATE {} SET {} WHERE id=?", op.table, sets.join(", "));
        let refs: Vec<&dyn rusqlite::ToSql> = vals.iter().map(|b| b.as_ref()).collect();
        conn.execute(&sql, refs.as_slice())?;
    } else {
        let mut cols: Vec<String> = vec!["id".into()];
        let mut place: Vec<String> = vec!["?".into()];
        let mut vals: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(op.record_id.clone())];
        for (col, val) in &obj {
            if col == "id" {
                continue;
            }
            cols.push(col.clone());
            place.push("?".into());
            vals.push(json_to_sql(val));
        }
        if synced {
            for (c, v) in [
                ("hlc", op.hlc.clone()),
                ("created_at", op.hlc.clone()),
                ("updated_at", op.hlc.clone()),
                ("sync_state", SHARED_DRIVE.to_string()),
            ] {
                cols.push(c.into());
                place.push("?".into());
                vals.push(Box::new(v));
            }
        }
        let sql = format!("INSERT INTO {} ({}) VALUES ({})", op.table, cols.join(","), place.join(","));
        let refs: Vec<&dyn rusqlite::ToSql> = vals.iter().map(|b| b.as_ref()).collect();
        conn.execute(&sql, refs.as_slice())?;
    }
    Ok(())
}

// ---- provisional dedup + cursor + quarantine, in drive_state -----------------

fn seen_op(conn: &Connection, op_id: &str) -> rusqlite::Result<bool> {
    // Confirmed already (applied_ops) OR provisionally applied (drive_state).
    let in_applied: bool = conn
        .query_row("SELECT 1 FROM applied_ops WHERE op_id=?1", params![op_id], |_| Ok(()))
        .optional()?
        .is_some();
    Ok(in_applied || ds_get(conn, &format!("prov:{op_id}"))?.is_some())
}

fn remember_op(conn: &Connection, op_id: &str) -> rusqlite::Result<()> {
    ds_set(conn, &format!("prov:{op_id}"), "1")
}

fn cursor_get(conn: &Connection, folder: &str) -> rusqlite::Result<BTreeSet<String>> {
    match ds_get(conn, &format!("pull_cursor:{folder}"))? {
        Some(json) => Ok(serde_json::from_str(&json).unwrap_or_default()),
        None => Ok(BTreeSet::new()),
    }
}

fn cursor_set(conn: &Connection, folder: &str, seen: &BTreeSet<String>) -> rusqlite::Result<()> {
    let json = serde_json::to_string(seen).unwrap_or_else(|_| "[]".into());
    ds_set(conn, &format!("pull_cursor:{folder}"), &json)
}

fn quarantine(conn: &Connection, folder: &str, file: &str) -> rusqlite::Result<()> {
    ds_set(conn, &format!("quarantine:{folder}/{file}"), "tampered")
}

fn ds_get(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT value_json FROM drive_state WHERE key=?1", params![key], |r| r.get(0))
        .optional()
}

fn ds_set(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO drive_state(id,key,value_json) VALUES (?1,?1,?2) \
         ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json",
        params![key, value],
    )?;
    Ok(())
}

fn json_to_sql(v: &serde_json::Value) -> Box<dyn rusqlite::ToSql> {
    match v {
        serde_json::Value::Null => Box::new(Option::<String>::None),
        serde_json::Value::Bool(b) => Box::new(if *b { 1i64 } else { 0 }),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Box::new(i)
            } else {
                Box::new(n.to_string())
            }
        }
        serde_json::Value::String(s) => Box::new(s.clone()),
        other => Box::new(other.to_string()),
    }
}
