//! Device push + server import over any [`DriveApi`] (prompts/P06 Steps 4 & 6).
//!
//! Transport-agnostic: identical against the real `drive.file` client and the
//! test [`super::fake::FakeDrive`], so the exchange mechanics are provable without
//! Google (and without the Step-0 spike — only the real client + OAuth are gated).
//!
//! * [`push_outbox`] (device) — group the outbox by audience, seal each group into
//!   a `.vop`, upload (temp name → verify-by-readback → rename), mark rows
//!   `shared_drive`. Retry-safe: the final name is a pure function of the ops.
//! * [`import_all`] (server) — discover every `ops-*` folder, download bundles,
//!   open with the right audience key **version**, apply all ops in global HLC
//!   order through the normal [`apply_op`] path (idempotent by op_id), archive
//!   processed bundles to `_done/`. Acks are written by [`write_ack`].

use std::collections::BTreeMap;

use rusqlite::{params, Connection};

use super::bundle::{self, PROP_AUDIENCE, PROP_KEYVER};
use super::keys;
use super::{DriveApi, DriveError};
use crate::sync::apply::apply_op;
use crate::sync::protocol::{Op, OpResult, OpStatus};
use crate::sync::seal;

#[derive(Debug, thiserror::Error)]
pub enum ExchangeError {
    #[error("drive: {0}")]
    Drive(#[from] DriveError),
    #[error("bundle: {0}")]
    Bundle(String),
    #[error("db: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("no held key for audience {0}")]
    NoKey(String),
}

/// A device's held audience keys (delivered at join / on change): (audience, version) → raw key.
pub type HeldKeys = BTreeMap<(String, i64), [u8; 32]>;

// ---- tables that carry the §7 sync columns (so `shared_drive` is meaningful) --
fn is_synced(table: &str) -> bool {
    matches!(
        table,
        "school" | "staff" | "student" | "enrollment" | "attendance_sheet" | "marks_sheet" | "fee_due" | "payment" | "request"
    )
}

/// The current (highest) version this device holds for `audience`, if any.
fn current_version(held: &HeldKeys, audience: &str) -> Option<(i64, [u8; 32])> {
    held.iter()
        .filter(|((a, _), _)| a == audience)
        .max_by_key(|((_, v), _)| *v)
        .map(|((_, v), k)| (*v, *k))
}

// ================================================================ push =========

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PushOutcome {
    pub bundles: usize,
    pub ops: usize,
    pub files: Vec<String>,
}

/// Read the device's outbox, seal each audience's ops into `.vop` bundles, and
/// upload them to `ops_folder`.
pub fn push_outbox(
    conn: &mut Connection,
    drive: &impl DriveApi,
    ops_folder: &str,
    held: &HeldKeys,
) -> Result<PushOutcome, ExchangeError> {
    let ops = read_outbox(conn)?;
    let mut by_aud: BTreeMap<String, Vec<Op>> = BTreeMap::new();
    for op in ops {
        by_aud.entry(op.audience.clone()).or_default().push(op);
    }

    let existing: Vec<String> = drive.list(ops_folder)?.into_iter().map(|f| f.name).collect();
    let mut out = PushOutcome::default();

    for (audience, group) in by_aud {
        let (version, key) =
            current_version(held, &audience).ok_or_else(|| ExchangeError::NoKey(audience.clone()))?;
        let chunks = bundle::chunk_ops(&group).map_err(|e| ExchangeError::Bundle(format!("{e}")))?;
        for chunk in chunks {
            let name = bundle::bundle_filename(&chunk[0].hlc, &audience, version);
            if existing.contains(&name) {
                continue; // already uploaded — retry-safe (same content → same name)
            }
            let body = bundle::seal_bundle(&key, &audience, version, &chunk)
                .map_err(|e| ExchangeError::Bundle(format!("{e}")))?;
            let props = bundle::bundle_properties(&audience, version);
            // temp name so a half-written file is never seen, then verify, then commit
            let tmp = drive.create(ops_folder, &format!(".tmp-{}", chunk[0].op_id), &body, &props)?;
            // verify-by-readback (portable across fake + real Drive; no md5 dep)
            if drive.download(&tmp.id)? != body {
                drive.delete(&tmp.id).ok();
                return Err(ExchangeError::Bundle("readback mismatch".into()));
            }
            drive.rename(&tmp.id, &name)?;
            // only now: mark the domain rows shared_drive
            for op in &chunk {
                mark_shared(conn, op)?;
            }
            out.bundles += 1;
            out.ops += chunk.len();
            out.files.push(name);
        }
    }
    Ok(out)
}

fn read_outbox(conn: &Connection) -> rusqlite::Result<Vec<Op>> {
    let mut stmt = conn.prepare(
        "SELECT op_id,hlc,device_id,staff_id,audience,\"table\",record_id,kind,payload,base_version,server_epoch \
         FROM outbox ORDER BY hlc",
    )?;
    let rows = stmt.query_map([], |r| {
        let payload_s: String = r.get(8)?;
        Ok(Op {
            op_id: r.get(0)?,
            hlc: r.get(1)?,
            device_id: r.get(2)?,
            staff_id: r.get(3)?,
            audience: r.get(4)?,
            table: r.get(5)?,
            record_id: r.get(6)?,
            kind: r.get(7)?,
            payload: serde_json::from_str(&payload_s).unwrap_or(serde_json::Value::Null),
            base_version: r.get(9)?,
            server_epoch: r.get(10)?,
        })
    })?;
    rows.collect()
}

fn mark_shared(conn: &Connection, op: &Op) -> rusqlite::Result<()> {
    if !is_synced(&op.table) {
        return Ok(()); // child tables (attendance_mark, …) carry no sync_state
    }
    // Only advance from an un-synced local state; never downgrade confirmed rows.
    let sql = format!(
        "UPDATE {} SET sync_state='shared_drive' WHERE id=?1 AND sync_state IN ('draft','on_device')",
        op.table
    );
    conn.execute(&sql, params![op.record_id])?;
    Ok(())
}

// ============================================================== import =========

/// What one device is told back (Step 6 ack; sealed with its session key).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct DeviceAck {
    pub last_hlc: String,
    pub results: Vec<OpResult>,
}

#[derive(Debug, Default)]
pub struct ImportOutcome {
    pub bundles: usize,
    pub confirmed: usize,
    pub flagged: usize,
    pub conflicted: usize,
    pub rejected: usize,
    /// bundles that failed AEAD (tampered / wrong key) — not applied.
    pub quarantined: usize,
    pub archived: usize,
    /// per source device: the ack the server would hand back (Step 7).
    pub acks: BTreeMap<String, DeviceAck>,
}

/// Server import: discover `ops-*` under `exchange`, download bundles, decrypt
/// with the server's held key version, apply in global HLC order, archive
/// processed bundles. Idempotent by op_id (re-import applies nothing new).
pub fn import_all(
    conn: &mut Connection,
    drive: &impl DriveApi,
    exchange_folder: &str,
) -> Result<ImportOutcome, ExchangeError> {
    let mut out = ImportOutcome::default();

    // 1) discover the ops-* folders
    let ops_folders: Vec<(String, String)> = drive
        .list(exchange_folder)?
        .into_iter()
        .filter(|f| f.is_folder)
        .filter_map(|f| f.name.strip_prefix("ops-").map(|d| (f.id.clone(), d.to_string())))
        .collect();

    // 2) gather (device, bundle file, decrypted ops) — quarantine AEAD failures
    struct Pending {
        device: String,
        folder: String,
        file_id: String,
        ops: Vec<Op>,
    }
    let mut pending: Vec<Pending> = Vec::new();
    for (folder_id, device) in &ops_folders {
        for f in drive.list(folder_id)? {
            if f.is_folder || !f.name.ends_with(".vop") {
                continue;
            }
            out.bundles += 1;
            let audience = f.properties.get(PROP_AUDIENCE).cloned().unwrap_or_default();
            let version: i64 = f.properties.get(PROP_KEYVER).and_then(|v| v.parse().ok()).unwrap_or(0);
            let Some(key) = keys::key_bytes(conn, &audience, version)? else {
                out.quarantined += 1; // server has no such key → cannot read
                continue;
            };
            let body = drive.download(&f.id)?;
            match bundle::open_bundle(&key, &audience, version, &body) {
                Ok(ops) => pending.push(Pending { device: device.clone(), folder: folder_id.clone(), file_id: f.id, ops }),
                Err(_) => out.quarantined += 1, // tampered / wrong key → not applied
            }
        }
    }

    // 3) apply ALL ops in global HLC order (§8.4 / Step 6)
    let mut all: Vec<(usize, Op)> = Vec::new();
    for (i, p) in pending.iter().enumerate() {
        for op in &p.ops {
            all.push((i, op.clone()));
        }
    }
    all.sort_by(|a, b| a.1.hlc.cmp(&b.1.hlc));
    for (src, op) in &all {
        let result = apply_op(conn, op)?;
        match result.status {
            OpStatus::Confirmed => out.confirmed += 1,
            OpStatus::Flagged => out.flagged += 1,
            OpStatus::Conflict => out.conflicted += 1,
            OpStatus::Rejected => out.rejected += 1,
        }
        let ack = out.acks.entry(pending[*src].device.clone()).or_insert_with(|| DeviceAck {
            last_hlc: String::new(),
            results: vec![],
        });
        if op.hlc > ack.last_hlc {
            ack.last_hlc = op.hlc.clone();
        }
        ack.results.push(result);
    }

    // 4) archive processed bundles to ops-<device>/_done/
    for p in &pending {
        let done = drive.ensure_folder(&p.folder, "_done")?;
        drive.move_to(&p.file_id, &done)?;
        out.archived += 1;
    }

    Ok(out)
}

/// Seal a device ack `{last_hlc, results}` with the device's session key and write
/// `acks/<device_id>.json` (Step 6). The device reads it to move its outbox ops to
/// confirmed/rejected/conflict without direct server contact (Step 7).
pub fn write_ack(
    drive: &impl DriveApi,
    acks_folder: &str,
    device_id: &str,
    ack: &DeviceAck,
    session_key_b64: &str,
) -> Result<(), ExchangeError> {
    let keys = seal::derive_keys(session_key_b64).map_err(|_| ExchangeError::Bundle("bad session key".into()))?;
    let json = serde_json::to_vec(ack).map_err(|e| ExchangeError::Bundle(e.to_string()))?;
    let aad = seal::associated_data("ACK", device_id, device_id, 0);
    let sealed = seal::seal(&keys.s2c, &aad, &json);
    let name = format!("{device_id}.json");
    // overwrite: delete any prior ack, then create fresh
    if let Some(prev) = drive.list(acks_folder)?.into_iter().find(|f| f.name == name) {
        drive.delete(&prev.id).ok();
    }
    drive.create(acks_folder, &name, &sealed, &BTreeMap::new())?;
    Ok(())
}

/// Read + open a device's ack (Step 7, device side).
pub fn read_ack(
    drive: &impl DriveApi,
    acks_folder: &str,
    device_id: &str,
    session_key_b64: &str,
) -> Result<Option<DeviceAck>, ExchangeError> {
    let name = format!("{device_id}.json");
    let Some(file) = drive.list(acks_folder)?.into_iter().find(|f| f.name == name) else {
        return Ok(None);
    };
    let sealed = drive.download(&file.id)?;
    let keys = seal::derive_keys(session_key_b64).map_err(|_| ExchangeError::Bundle("bad session key".into()))?;
    let aad = seal::associated_data("ACK", device_id, device_id, 0);
    let json = seal::open(&keys.s2c, &aad, &sealed).map_err(|_| ExchangeError::Bundle("ack open failed".into()))?;
    let ack: DeviceAck = serde_json::from_slice(&json).map_err(|e| ExchangeError::Bundle(e.to_string()))?;
    Ok(Some(ack))
}
