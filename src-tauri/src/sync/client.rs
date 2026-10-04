//! Client-side identity for a device that JOINED a school (Phase B). A client keeps
//! its own credentials (device id/token, session key, audience keys) plus how to reach
//! its school server (pinned TLS fingerprint + LAN addresses), so the app can run as a
//! Client (`DeviceMode::Client`) and sync. Stored as one JSON blob in the encrypted
//! `app_kv` table.
//!
//! This is the JOINING device's half of the protocol — `server::service::join` is the
//! server's half. The client runtime (join call, role detection, sync loop) was
//! deferred in P04; this module is its foundation.

use crate::kv;
use crate::sync::protocol::{AudienceKey, JoinPayload, JoinReq, JoinResp};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// `app_kv` key for the client-identity blob.
pub const KV_CLIENT: &str = "client_identity";

/// Everything a joined device needs to be a client + reach its server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientIdentity {
    pub device_id: String,
    pub device_token: String,
    pub staff_id: String,
    pub staff_name: String,
    pub staff_role: String,
    /// Base64 session key (relay sealing).
    pub session_key: String,
    pub school_id: String,
    pub school_name: String,
    pub receipt_series: String,
    pub admission_series: String,
    /// Per-audience keys this device holds (to open sealed bundles on the Drive/relay
    /// route; the LAN route is plain over pinned TLS and does not need them).
    pub audience_keys: Vec<AudienceKey>,
    /// The school server's pinned TLS cert fingerprint (from the invite).
    pub server_fingerprint: String,
    /// LAN addresses advertised for the server (from the invite; refreshed by mDNS).
    pub server_addrs: Vec<String>,
    pub server_port: u16,
    pub server_epoch: i64,
    pub bootstrap_cursor: i64,
    pub lease_expires_at: String,
    pub protocol: u32,
}

/// Persist the identity from a successful join + record the known server epoch (so the
/// client is immediately epoch-fenced). Idempotent (upserts the blob).
pub fn apply_join_response(
    conn: &Connection,
    resp: &JoinResp,
    server_fingerprint: &str,
    server_addrs: &[String],
    server_port: u16,
) -> rusqlite::Result<()> {
    let id = ClientIdentity {
        device_id: resp.device_id.clone(),
        device_token: resp.device_token.clone(),
        staff_id: resp.staff.id.clone(),
        staff_name: resp.staff.name.clone(),
        staff_role: resp.staff.role.clone(),
        session_key: resp.session_key.clone(),
        school_id: resp.school.id.clone(),
        school_name: resp.school.name.clone(),
        receipt_series: resp.receipt_series.clone(),
        admission_series: resp.admission_series.clone(),
        audience_keys: resp.audience_keys.clone(),
        server_fingerprint: server_fingerprint.to_string(),
        server_addrs: server_addrs.to_vec(),
        server_port,
        server_epoch: resp.server_epoch,
        bootstrap_cursor: resp.bootstrap_cursor,
        lease_expires_at: resp.lease_expires_at.clone(),
        protocol: resp.protocol,
    };
    store(conn, &id)?;
    kv::set(conn, crate::sync::engine::KV_KNOWN_EPOCH, &resp.server_epoch)?;
    Ok(())
}

pub fn store(conn: &Connection, id: &ClientIdentity) -> rusqlite::Result<()> {
    kv::set(conn, KV_CLIENT, id)
}

pub fn load(conn: &Connection) -> Option<ClientIdentity> {
    kv::get::<ClientIdentity>(conn, KV_CLIENT).ok().flatten()
}

/// True once this device has joined a school as a client (→ `DeviceMode::Client`).
pub fn is_client(conn: &Connection) -> bool {
    kv::exists(conn, KV_CLIENT).unwrap_or(false)
}

/// Forget the client identity (e.g. revoked / leave a school).
pub fn clear(conn: &Connection) -> rusqlite::Result<()> {
    kv::delete(conn, KV_CLIENT)
}

/// Call `/v1/join` on the school server over the invite's **pinned** TLS cert, trying
/// each advertised LAN address in turn, and return the server's `JoinResp`. No DB —
/// so the future is `Send` (it holds no `&Connection` across the network awaits), which
/// a Tauri async command requires. Persist the result with [`apply_join_response`].
pub async fn fetch_join(payload: &JoinPayload, device_name: &str, platform: &str) -> Result<JoinResp, String> {
    let req = JoinReq {
        invite_code: payload.code.clone(),
        device_name: device_name.to_string(),
        platform: platform.to_string(),
        google_email: None,
    };
    let tls = crate::server::cert::client_config(&payload.cert_sha256)?;
    let http = reqwest::Client::builder()
        .use_preconfigured_tls(tls)
        .build()
        .map_err(|e| e.to_string())?;

    let mut last_err = "no server address in the invitation".to_string();
    for addr in &payload.lan_addrs {
        let url = format!("https://{addr}:{}/v1/join", payload.port);
        match http.post(&url).json(&req).send().await {
            Ok(r) if r.status().is_success() => return r.json::<JoinResp>().await.map_err(|e| e.to_string()),
            Ok(r) => last_err = format!("server rejected the join (HTTP {})", r.status()),
            Err(e) => last_err = e.to_string(),
        }
    }
    Err(last_err)
}

/// Fetch + persist in one call (convenience for tests / single-connection callers).
/// Holds a `&Connection` across the await, so its future is `!Send` — fine for a
/// current-thread test runtime, but a Tauri async command must instead call
/// [`fetch_join`] then [`apply_join_response`] (see commands::join_school).
pub async fn join_school(
    conn: &Connection,
    payload: &JoinPayload,
    device_name: &str,
    platform: &str,
) -> Result<ClientIdentity, String> {
    let jr = fetch_join(payload, device_name, platform).await?;
    apply_join_response(conn, &jr, &payload.cert_sha256, &payload.lan_addrs, payload.port).map_err(|e| e.to_string())?;
    load(conn).ok_or_else(|| "identity not stored".to_string())
}

/// Run one sync cycle as a client: build the LAN route(s) from the stored identity and
/// sync over the pinned TLS transport. Returns the route label that worked (e.g. "On
/// school Wi-Fi"). Used by the background sync loop (B3) and a manual "Sync now".
/// (mDNS re-discovery of a changed server IP is a later refinement; the invite's
/// advertised addresses are used here.)
pub async fn client_sync_tick(conn: &mut Connection) -> Result<String, String> {
    use crate::sync::engine::{sync_once_routed, Route};
    use crate::sync::transport::HttpsTransport;
    let id = load(conn).ok_or("this device has not joined a school")?;

    // Candidate (addr, port) targets: the mDNS-discovered CURRENT address first (so
    // sync survives the server's LAN IP/port changing on DHCP/reboot), then the
    // invite's stored addresses as a fallback. All are pinned to the stored FULL
    // fingerprint — the mDNS record only carries a short prefix, used just to confirm
    // it is the same server.
    let mut targets: Vec<(String, u16)> = Vec::new();
    #[cfg(not(target_os = "android"))]
    if let Some(found) = discover_current(&id.school_id, &id.server_fingerprint).await {
        targets.push(found);
    }
    for addr in &id.server_addrs {
        targets.push((addr.clone(), id.server_port));
    }
    targets.dedup();

    let mut routes = Vec::new();
    for (addr, port) in &targets {
        if let Ok(t) = HttpsTransport::new(
            format!("https://{addr}:{port}"),
            id.device_token.clone(),
            &id.server_fingerprint,
        ) {
            routes.push(Route::Lan(t));
        }
    }
    if routes.is_empty() {
        return Err("no reachable server route".into());
    }
    let (_outcome, label) = sync_once_routed(conn, &routes).await.map_err(|e| format!("{e:?}"))?;
    Ok(label.to_string())
}

/// One Drive sync pass as a CLIENT (§8.3 route 3): push our outbox as sealed `.vop`
/// bundles, pull peers' bundles for audiences we hold a key for (provisional apply),
/// and apply the server's ack (dropping confirmed ops from the outbox). This is the
/// off-LAN route — used when the school server is unreachable but the internet + a
/// connected Google Drive are. SYNC — the Drive client blocks internally, so call from
/// a blocking context. Composes the already-built exchange engine; the real Drive
/// round-trip is device-verified.
pub fn client_drive_tick(conn: &mut Connection) -> Result<(), String> {
    use crate::sync::drive::{exchange, google::GoogleDrive, oauth, pull, DriveApi};
    use base64::Engine;

    if !oauth::is_connected(conn) {
        return Ok(()); // Drive not connected on this device — LAN only
    }
    let id = load(conn).ok_or("this device has not joined a school")?;

    // The audience keys this device holds (audience+version → 32-byte key), from join.
    let mut held: exchange::HeldKeys = std::collections::BTreeMap::new();
    for ak in &id.audience_keys {
        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&ak.key_b64) {
            if let Ok(k) = <[u8; 32]>::try_from(bytes.as_slice()) {
                held.insert((ak.audience.clone(), ak.version), k);
            }
        }
    }

    // Refresh the access token with the client id that minted it (Android vs desktop).
    #[cfg(target_os = "android")]
    let client_id = crate::config::get().google_client_id_android.clone();
    #[cfg(not(target_os = "android"))]
    let client_id = crate::config::get().google_client_id_desktop.clone();
    if client_id.is_empty() {
        return Ok(());
    }
    let token = oauth::valid_access_token(conn, &client_id).map_err(|e| e.to_string())?;
    let drive = GoogleDrive::new(token).map_err(|e| format!("{e:?}"))?;

    // Resolve Vidya/<school> (<id>)/exchange — the SAME path the server (C1) + PWA use.
    let vidya = drive.ensure_folder("root", "Vidya").map_err(|e| format!("{e:?}"))?;
    let school = drive
        .ensure_folder(&vidya, &format!("{} ({})", id.school_name, id.school_id))
        .map_err(|e| format!("{e:?}"))?;
    let exchange_folder = drive.ensure_folder(&school, "exchange").map_err(|e| format!("{e:?}"))?;

    // 1) push our outbox, 2) pull peers' bundles, 3) read our ack + confirm the outbox.
    let ops_folder = drive
        .ensure_folder(&exchange_folder, &format!("ops-{}", id.device_id))
        .map_err(|e| format!("{e:?}"))?;
    exchange::push_outbox(conn, &drive, &ops_folder, &held).map_err(|e| format!("{e:?}"))?;
    pull::pull_provisional(conn, &drive, &exchange_folder, &id.device_id, &held).map_err(|e| format!("{e:?}"))?;
    let acks_folder = drive.ensure_folder(&exchange_folder, "acks").map_err(|e| format!("{e:?}"))?;
    if let Some(ack) = exchange::read_ack(&drive, &acks_folder, &id.device_id, &id.session_key).map_err(|e| format!("{e:?}"))? {
        for res in &ack.results {
            let _ = conn.execute("DELETE FROM outbox WHERE op_id=?1", rusqlite::params![res.op_id]);
        }
    }
    Ok(())
}

/// Discover the school server's current LAN address via mDNS (desktop), confirming the
/// advertised fingerprint prefix matches our pinned one. Returns `(addr, port)` or
/// `None` (not found / different cert / mDNS unavailable). Runs the blocking browse on
/// a blocking thread.
#[cfg(not(target_os = "android"))]
async fn discover_current(school_id: &str, full_fingerprint: &str) -> Option<(String, u16)> {
    let school = school_id.to_string();
    let found = tokio::task::spawn_blocking(move || {
        crate::sync::mdns::discover(&school, std::time::Duration::from_secs(2)).ok().flatten()
    })
    .await
    .ok()
    .flatten()?;
    // The TXT record carries only a 16-char fingerprint prefix; confirm it matches the
    // full pinned fingerprint before trusting the address (defence against spoofing).
    if !found.fp16.is_empty() && full_fingerprint.starts_with(&found.fp16) {
        Some((found.addr, found.port))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::sync::protocol::{SchoolLite, StaffLite};

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn sample_resp() -> JoinResp {
        JoinResp {
            device_id: "dev-1".into(),
            device_token: "tok-abc".into(),
            staff: StaffLite { id: "stf-1".into(), name: "Meena".into(), role: "teacher".into() },
            receipt_series: "A2".into(),
            admission_series: "ADM".into(),
            audience_keys: vec![AudienceKey { audience: "class:5".into(), key_b64: "AAAA".into(), version: 1 }],
            session_key: "c2Vzc2lvbg==".into(),
            school: SchoolLite { id: "sch-1".into(), name: "Saraswati".into() },
            lease_expires_at: "2026-10-31T00:00:00Z".into(),
            bootstrap_cursor: 42,
            protocol: 1,
            server_epoch: 3,
        }
    }

    #[test]
    fn apply_join_persists_identity_epoch_and_role() {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        assert!(!is_client(&c), "not a client until joined");

        apply_join_response(&c, &sample_resp(), "FP123", &["192.168.1.5".into(), "192.168.1.6".into()], 47650).unwrap();

        assert!(is_client(&c));
        let id = load(&c).expect("identity stored");
        assert_eq!(id.device_id, "dev-1");
        assert_eq!(id.staff_id, "stf-1");
        assert_eq!(id.school_id, "sch-1");
        assert_eq!(id.server_fingerprint, "FP123");
        assert_eq!(id.server_addrs, vec!["192.168.1.5", "192.168.1.6"]);
        assert_eq!(id.server_port, 47650);
        assert_eq!(id.bootstrap_cursor, 42);
        assert_eq!(id.audience_keys.len(), 1);
        // The known epoch is fenced to the server's epoch.
        assert_eq!(crate::sync::engine::known_epoch(&c).unwrap(), 3);

        clear(&c).unwrap();
        assert!(!is_client(&c));
    }
}
