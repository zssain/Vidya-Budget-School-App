//! Device push + server import over any [`DriveApi`] (prompts/P06 Steps 4 & 6).
//!
//! Transport-agnostic: identical against the real `drive.file` client and the
//! test [`super::fake::FakeDrive`], so the exchange mechanics are provable without
//! Google (and without the Step-0 spike — only the real client + OAuth are gated).
//!
//! * [`push_outbox`] (device) — seal the outbox with this device's own c2s key into
//!   `.vop` bundles (format v2), upload (temp name → verify-by-readback → rename), mark rows
//!   `shared_drive`. Retry-safe: the final name is a pure function of the ops.
//! * [`import_all`] (server) — discover every `ops-*` folder, download bundles,
//!   open with the owning live device's c2s key, apply all ops in global HLC
//!   order through the normal [`apply_op`] path (idempotent by op_id), archive
//!   processed bundles to `_done/`. Acks are written by [`write_ack`].

use std::collections::BTreeMap;

use rusqlite::{params, Connection, OptionalExtension};

use super::bundle::{self, FORMAT_DEVICE_V2, PROP_FORMAT};
use super::{DriveApi, DriveError};
use crate::sync::apply::{apply_client_op, OpAuth};
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
}

// ---- tables that carry the §7 sync columns (so `shared_drive` is meaningful) --
fn is_synced(table: &str) -> bool {
    matches!(
        table,
        "school" | "staff" | "student" | "enrollment" | "attendance_sheet" | "marks_sheet" | "fee_due" | "payment" | "request"
    )
}

// ================================================================ push =========

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PushOutcome {
    pub bundles: usize,
    pub ops: usize,
    pub files: Vec<String>,
}

/// Read the device's outbox, seal it into device bundles (format v2: this device's
/// own c2s key from `session_key_b64`, AAD bound to `device_id`) and upload them to
/// `ops_folder`. No audience key is needed to upload: a teacher can submit a leave
/// request or an admin-audience op without holding the admin key, and the school PC
/// can authenticate exactly which device wrote each bundle.
pub fn push_outbox(
    conn: &mut Connection,
    drive: &impl DriveApi,
    ops_folder: &str,
    device_id: &str,
    session_key_b64: &str,
) -> Result<PushOutcome, ExchangeError> {
    let keys = seal::derive_keys(session_key_b64).map_err(|_| ExchangeError::Bundle("bad session key".into()))?;
    let ops: Vec<Op> = read_outbox(conn)?.into_iter().filter(|o| o.device_id == device_id).collect();
    let existing: Vec<String> = drive.list(ops_folder)?.into_iter().map(|f| f.name).collect();
    let mut out = PushOutcome::default();
    let chunks = bundle::chunk_ops(&ops).map_err(|e| ExchangeError::Bundle(format!("{e}")))?;
    for chunk in chunks {
        let name = bundle::device_bundle_filename(&chunk[0].hlc);
        if existing.contains(&name) {
            continue; // already uploaded — retry-safe
        }
        let body = bundle::seal_device_bundle(&keys.c2s, device_id, &chunk).map_err(|e| ExchangeError::Bundle(format!("{e}")))?;
        let props = bundle::device_bundle_properties();
        // temp name so a half-written file is never seen, then verify, then commit
        let tmp = drive.create(ops_folder, &format!(".tmp-{}", chunk[0].op_id), &body, &props)?;
        if drive.download(&tmp.id)? != body {
            drive.delete(&tmp.id).ok();
            return Err(ExchangeError::Bundle("readback mismatch".into()));
        }
        drive.rename(&tmp.id, &name)?;
        for op in &chunk {
            mark_shared(conn, op)?;
        }
        out.bundles += 1;
        out.ops += chunk.len();
        out.files.push(name);
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

    // 2) gather (device, bundle file, decrypted ops). A bundle is accepted only if it
    //    is format device-v2 AND opens with the session key of the (live) device that
    //    owns the folder — that proves authorship. Anything else is quarantined: a
    //    legacy audience-sealed bundle, an unknown/revoked device, a tampered body.
    struct Pending {
        device: String,
        staff: String,
        folder: String,
        file_id: String,
        ops: Vec<Op>,
    }
    let mut pending: Vec<Pending> = Vec::new();
    for (folder_id, device) in &ops_folders {
        let live: Option<(String, Option<String>)> = conn
            .query_row(
                "SELECT staff_id, session_key FROM device WHERE id=?1 AND revoked_at IS NULL",
                params![device],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let device_keys = live
            .as_ref()
            .and_then(|(_, sk)| sk.as_deref())
            .and_then(|sk| seal::derive_keys(sk).ok());
        for f in drive.list(folder_id)? {
            if f.is_folder || !f.name.ends_with(".vop") {
                continue;
            }
            out.bundles += 1;
            let (Some((staff, _)), Some(k)) = (&live, &device_keys) else {
                out.quarantined += 1; // unknown / revoked device, or no session key
                continue;
            };
            if f.properties.get(PROP_FORMAT).map(String::as_str) != Some(FORMAT_DEVICE_V2) {
                out.quarantined += 1; // legacy audience-sealed bundle: authorship unprovable
                continue;
            }
            let body = drive.download(&f.id)?;
            match bundle::open_device_bundle(&k.c2s, device, &body) {
                Ok(ops) => pending.push(Pending {
                    device: device.clone(),
                    staff: staff.clone(),
                    folder: folder_id.clone(),
                    file_id: f.id,
                    ops,
                }),
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
        // Authorship was proven by the device key that opened the bundle; the staff
        // comes from the server's device row, never from the op.
        let p = &pending[*src];
        let result = apply_client_op(conn, &OpAuth { device_id: &p.device, staff_id: &p.staff, route: "drive" }, op)?;
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

// ============================================================ down export ======

/// Drive folder holding each device's sealed canonical export.
pub const DOWN: &str = "down";

/// The body of a down export (sealed per device).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DownExport {
    pub v: u32,
    pub device_id: String,
    /// The school PC's op_log position this export reflects.
    pub cursor: i64,
    pub server_epoch: i64,
    /// The device's COMPLETE role-scoped data (it replaces the device's copy).
    pub changes: Vec<crate::sync::protocol::Change>,
    /// Counts + digest of `changes` (G1), checked by the native client before it
    /// applies anything. (The PWA ignores it; the AEAD already binds the body.)
    #[serde(default)]
    pub manifest: Option<crate::sync::protocol::SnapshotManifest>,
    /// Rows the school PC deleted that are not in `changes` (newest first, bounded).
    /// The native client upserts an export, so without these a row deleted on the
    /// school PC would stay on a Drive-only second PC/phone forever. (The PWA
    /// replaces its whole store from `changes` and doesn't need them.)
    #[serde(default)]
    pub deletes: Vec<crate::sync::protocol::Delete>,
    /// 0 when `deletes` lists every deletion; otherwise the `op_log` position of the
    /// newest deletion left out. A device whose last applied export is older than
    /// this may have missed deletions, so it reconciles against `changes` instead.
    #[serde(default)]
    pub deletes_from: i64,
}

/// How many recent deletions a down export carries.
pub const DOWN_TOMBSTONES: i64 = 5000;

/// The most recent distinct deletions from synced tables, minus rows that exist
/// again (re-created after the delete — they are in the export as live rows), and
/// the `deletes_from` position (0 = the list is complete; see [`DownExport`]).
fn recent_tombstones(conn: &Connection, live: &[crate::sync::protocol::Change]) -> rusqlite::Result<(Vec<crate::sync::protocol::Delete>, i64)> {
    let live: std::collections::HashSet<(&str, &str)> = live.iter().map(|c| (c.table.as_str(), c.record_id.as_str())).collect();
    let mut st = conn.prepare(
        "SELECT \"table\", record_id, MAX(server_seq) FROM op_log WHERE kind='delete' GROUP BY \"table\", record_id ORDER BY MAX(server_seq) DESC LIMIT ?1",
    )?;
    let mut rows: Vec<(String, String, i64)> = st
        .query_map(params![DOWN_TOMBSTONES + 1], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let deletes_from = if rows.len() as i64 > DOWN_TOMBSTONES { rows.pop().map(|(_, _, seq)| seq).unwrap_or(0) } else { 0 };
    let mut out = Vec::new();
    for (table, record_id, _) in rows {
        if !crate::sync::scope::SYNC_TABLES.contains(&table.as_str()) || live.contains(&(table.as_str(), record_id.as_str())) {
            continue;
        }
        let exists: bool = conn.query_row(&format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1)"), params![record_id], |r| r.get(0))?;
        if !exists {
            out.push(crate::sync::protocol::Delete { table, record_id });
        }
    }
    Ok((out, deletes_from))
}

/// School PC: for every live device with a session key, write `down/<device>.vdown`
/// — its complete role-scoped canonical data, sealed with that device's s2c key —
/// whenever the school data moved since that device's last export. This is how a
/// device that can't reach the LAN (the iPhone app, a phone off the school Wi-Fi)
/// gets school data at all: the LAN snapshot/pull isn't reachable from a browser,
/// and peers' uploads are sealed to the school PC only. A full replace per export
/// also delivers deletions and removed access. Returns the number written.
pub fn write_down_exports(conn: &Connection, drive: &impl DriveApi, exchange_folder: &str) -> Result<usize, ExchangeError> {
    let cursor: i64 = conn.query_row("SELECT COALESCE(MAX(server_seq),0) FROM op_log", [], |r| r.get(0))?;
    let epoch: i64 = conn.query_row("SELECT server_epoch FROM school LIMIT 1", [], |r| r.get(0)).optional()?.unwrap_or(1);
    let devices: Vec<(String, String, String)> = {
        let mut st = conn.prepare(
            "SELECT id, staff_id, session_key FROM device WHERE revoked_at IS NULL AND session_key IS NOT NULL AND session_key <> ''",
        )?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    let down = drive.ensure_folder(exchange_folder, DOWN)?;
    // A revoked (or removed) device's last export is taken down: nothing new is ever
    // sealed for it, and its stale copy should not linger in the shared folder (A8).
    for f in drive.list(&down)? {
        if let Some(id) = f.name.strip_suffix(".vdown") {
            if !devices.iter().any(|(d, _, _)| d == id) {
                drive.delete(&f.id).ok();
            }
        }
    }
    let mut written = 0;
    for (device, staff, sk) in devices {
        let marker = format!("down_cursor:{device}");
        let last: Option<i64> = crate::kv::get(conn, &marker)?;
        if last == Some(cursor) {
            continue; // nothing new since this device's last export
        }
        let auth = crate::server::service::DeviceAuth { device_id: device.clone(), staff_id: staff };
        let changes = crate::server::service::snapshot(conn, &auth)?;
        let manifest = crate::sync::protocol::SnapshotManifest::build(
            uuid::Uuid::now_v7().to_string(),
            epoch,
            cursor,
            &changes,
        );
        let (deletes, deletes_from) = recent_tombstones(conn, &changes)?;
        let body = DownExport { v: 1, device_id: device.clone(), cursor, server_epoch: epoch, changes, manifest: Some(manifest), deletes, deletes_from };
        let json = serde_json::to_vec(&body).map_err(|e| ExchangeError::Bundle(e.to_string()))?;
        let Ok(keys) = seal::derive_keys(&sk) else { continue };
        let sealed = seal::seal(&keys.s2c, &vidya_core::seal::device_down_aad(&device), &json);
        let name = format!("{device}.vdown");
        if let Some(prev) = drive.list(&down)?.into_iter().find(|f| f.name == name) {
            drive.delete(&prev.id).ok();
        }
        drive.create(&down, &name, &sealed, &BTreeMap::new())?;
        crate::kv::set(conn, &marker, &cursor)?;
        written += 1;
    }
    Ok(written)
}

/// Device side: read + open this device's down export (None if not written yet).
pub fn read_down_export(
    drive: &impl DriveApi,
    exchange_folder: &str,
    device_id: &str,
    session_key_b64: &str,
) -> Result<Option<DownExport>, ExchangeError> {
    let down = drive.ensure_folder(exchange_folder, DOWN)?;
    let Some(file) = drive.list(&down)?.into_iter().find(|f| f.name == format!("{device_id}.vdown")) else {
        return Ok(None);
    };
    let sealed = drive.download(&file.id)?;
    let keys = seal::derive_keys(session_key_b64).map_err(|_| ExchangeError::Bundle("bad session key".into()))?;
    let json = seal::open(&keys.s2c, &vidya_core::seal::device_down_aad(device_id), &sealed)
        .map_err(|_| ExchangeError::Bundle("down export open failed".into()))?;
    let body: DownExport = serde_json::from_slice(&json).map_err(|e| ExchangeError::Bundle(e.to_string()))?;
    if body.device_id != device_id {
        return Err(ExchangeError::Bundle("down export for another device".into()));
    }
    Ok(Some(body))
}
