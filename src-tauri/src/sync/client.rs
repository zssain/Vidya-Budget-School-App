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

/// Join a school as a client (Phase B): POST `/v1/join` to the school server over the
/// invite's **pinned** TLS cert, trying each advertised LAN address in turn, then
/// persist the returned identity. Returns the stored [`ClientIdentity`]. Async — call
/// from a command (off the server's own runtime).
pub async fn join_school(
    conn: &Connection,
    payload: &JoinPayload,
    device_name: &str,
    platform: &str,
) -> Result<ClientIdentity, String> {
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
            Ok(r) if r.status().is_success() => {
                let jr: JoinResp = r.json().await.map_err(|e| e.to_string())?;
                apply_join_response(conn, &jr, &payload.cert_sha256, &payload.lan_addrs, payload.port)
                    .map_err(|e| e.to_string())?;
                return load(conn).ok_or_else(|| "identity not stored".to_string());
            }
            Ok(r) => last_err = format!("server rejected the join (HTTP {})", r.status()),
            Err(e) => last_err = e.to_string(),
        }
    }
    Err(last_err)
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
