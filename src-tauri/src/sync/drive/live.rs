//! The school PC's live Google Drive loop (Phase C — completes the P12-deferred "live
//! Drive client"). On a timer the server, signed in as the school's sync account (full
//! access to the whole `Vidya/` tree), does one pass:
//!   1. answer pending PWA join requests in `exchange/joins/` (write the sealed
//!      response the joining device can open with its invite code);
//!   2. import + apply every client's sealed op bundles from `exchange/ops-*/`;
//!   3. write each device its sealed ack so it can confirm its outbox.
//!
//! It composes the already-tested exchange engine (`import_all` / `write_ack`) and the
//! join handshake (`answer_join_request`) with a real [`DriveApi`]. The real two-device
//! round-trip over Google is the owner's final check; this is verified against the fake
//! Drive in CI.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::Connection;
use time::OffsetDateTime;

use super::exchange::{import_all, write_ack, ExchangeError, ImportOutcome};
use super::join::{answer_join_request, response_name, DriveJoinRequest};
use super::DriveApi;

const JOINS: &str = "joins";
const ACKS: &str = "acks";

/// What one Drive pass did.
#[derive(Debug, Default)]
pub struct ServerDriveOutcome {
    pub joins_answered: usize,
    pub imported: ImportOutcome,
}

/// One pass of the school-PC Drive loop. `exchange_folder` is the id of
/// `Vidya/<school>/exchange`. Idempotent: an already-answered join is skipped and an
/// imported bundle is archived, so repeated calls are safe.
pub fn server_drive_tick(
    conn: &mut Connection,
    drive: &impl DriveApi,
    exchange_folder: &str,
    now: OffsetDateTime,
) -> Result<ServerDriveOutcome, ExchangeError> {
    let mut out = ServerDriveOutcome::default();

    // 1) Answer pending PWA join requests: exchange/joins/<id>.vjoin → <id>.resp.vjoin.
    let joins_folder = drive.ensure_folder(exchange_folder, JOINS)?;
    let files = drive.list(&joins_folder)?;
    let names: BTreeSet<String> = files.iter().map(|f| f.name.clone()).collect();
    for f in &files {
        if f.is_folder || !f.name.ends_with(".vjoin") || f.name.ends_with(".resp.vjoin") {
            continue;
        }
        let bytes = drive.download(&f.id)?;
        let Ok(req) = serde_json::from_slice::<DriveJoinRequest>(&bytes) else {
            continue; // not a request we understand — leave it for the Principal to see
        };
        let resp_name = response_name(&req.request_id);
        if names.contains(&resp_name) {
            continue; // already answered
        }
        // A bad/expired/consumed invite returns Err — leave the request in place (no
        // response) rather than fail the whole pass.
        if let Ok(sealed) = answer_join_request(conn, &bytes, now) {
            drive.create(&joins_folder, &resp_name, &sealed, &BTreeMap::new())?;
            out.joins_answered += 1;
        }
    }

    // 2) Import + apply every client's op bundles, then 3) write each device its ack.
    out.imported = import_all(conn, drive, exchange_folder)?;
    let acks_folder = drive.ensure_folder(exchange_folder, ACKS)?;
    for (device, ack) in &out.imported.acks {
        if let Some(session_key) = device_session_key(conn, device) {
            // A single device's ack failing (e.g. missing key) must not abort the pass.
            let _ = write_ack(drive, &acks_folder, device, ack, &session_key);
        }
    }

    Ok(out)
}

/// The device's base64 session key (set at join), for sealing its ack. `None` if the
/// device row or key is absent.
fn device_session_key(conn: &Connection, device_id: &str) -> Option<String> {
    conn.query_row("SELECT session_key FROM device WHERE id=?1", [device_id], |r| {
        r.get::<_, Option<String>>(0)
    })
    .ok()
    .flatten()
}

/// Resolve (creating if needed) `Vidya/<school name> (<school id>)/exchange` and return
/// its folder id. This MUST match the path clients use (web `src/lib/web/drive/sync.ts`)
/// so both sides read/write the same exchange.
pub fn resolve_exchange_folder(conn: &Connection, drive: &impl DriveApi) -> super::DriveResult<String> {
    let (id, name): (String, String) = conn
        .query_row("SELECT id, name FROM school LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| super::DriveError::Io(e.to_string()))?;
    let vidya = drive.ensure_folder("root", "Vidya")?;
    let school = drive.ensure_folder(&vidya, &format!("{name} ({id})"))?;
    drive.ensure_folder(&school, "exchange")
}

/// One real-Drive pass for the school server: when Drive is connected, build the live
/// client (reusing the Phase-A desktop Drive sign-in), resolve the exchange folder, and
/// run [`server_drive_tick`]. SYNC — the Drive client blocks internally, so call it
/// from a blocking thread (`spawn_blocking`), never an async task. Skips quietly when
/// Drive is not connected or there is no school.
pub fn server_drive_pass(conn: &mut Connection) -> Result<(), String> {
    use super::{google::GoogleDrive, oauth};
    if !oauth::is_connected(conn) {
        return Ok(()); // the school hasn't connected Drive — LAN/local only
    }
    let client_id = crate::config::get().google_client_id_desktop.clone();
    if client_id.is_empty() {
        return Ok(());
    }
    let token = oauth::valid_access_token(conn, &client_id).map_err(|e| e.to_string())?;
    let drive = GoogleDrive::new(token).map_err(|e| format!("{e:?}"))?;
    let exchange = resolve_exchange_folder(conn, &drive).map_err(|e| format!("{e:?}"))?;
    server_drive_tick(conn, &drive, &exchange, OffsetDateTime::now_utc()).map_err(|e| format!("{e:?}"))?;
    Ok(())
}
