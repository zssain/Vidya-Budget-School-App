//! Start the school server in the background (prompts/P04 Step 2). Runs in Server
//! mode when a school exists: opens its OWN WAL connection to the encrypted DB,
//! ensures the TLS cert (generated once, stored in `app_kv`), binds 47650.., stores
//! the LAN facts (for invites), advertises via mDNS, and serves `/v1` until the
//! process ends. Guarded: any failure is logged, never crashes app startup.
//!
//! Runtime module — verified with a running app (the service layer is tested).

use std::path::Path;
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rusqlite::{Connection, OptionalExtension};
use tokio::sync::Notify;

use crate::server::{cert, net};

/// Best-effort primary LAN IPv4 (no extra deps): a UDP socket "connected" to a
/// public address exposes the local address the OS would route through.
fn local_ipv4() -> Option<String> {
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    Some(sock.local_addr().ok()?.ip().to_string())
}

/// Load or generate the server cert material, persisting it in `app_kv`.
fn ensure_cert(conn: &Connection) -> Result<cert::CertMaterial, String> {
    let get = |k: &str| crate::kv::get::<String>(conn, k).ok().flatten();
    if let (Some(c), Some(k), Some(fp)) = (get("server_cert_der"), get("server_key_der"), get("server_fingerprint")) {
        if let (Ok(cert_der), Ok(key_der)) = (STANDARD.decode(&c), STANDARD.decode(&k)) {
            return Ok(cert::CertMaterial { cert_der, key_der, fingerprint: fp });
        }
    }
    let m = cert::generate_school_cert()?;
    crate::kv::set(conn, "server_cert_der", &STANDARD.encode(&m.cert_der)).map_err(|e| e.to_string())?;
    crate::kv::set(conn, "server_key_der", &STANDARD.encode(&m.key_der)).map_err(|e| e.to_string())?;
    crate::kv::set(conn, "server_fingerprint", &m.fingerprint).map_err(|e| e.to_string())?;
    Ok(m)
}

fn school_exists(conn: &Connection) -> bool {
    conn.query_row("SELECT 1 FROM school LIMIT 1", [], |_| Ok(())).optional().ok().flatten().is_some()
}

/// Spawn the server if this device is the school server and a school exists.
/// Safe to call at startup: returns quietly (logging) if it should not serve.
/// `stop` is fired when this PC is fenced out (licence `moved`) → the LAN listener
/// and the relay tunnel both stop.
pub async fn run_server(app: tauri::AppHandle, db_path: std::path::PathBuf, key_hex: String, stop: Arc<Notify>) {
    if let Err(e) = try_run_server(app, &db_path, &key_hex, stop).await {
        tracing::warn!("school server not started: {e}");
    }
}

async fn try_run_server(app: tauri::AppHandle, db_path: &Path, key_hex: &str, stop: Arc<Notify>) -> Result<(), String> {
    // The server's own WAL connection to the same encrypted DB.
    let conn = crate::db::open_encrypted(db_path, key_hex).map_err(|e| e.to_string())?;
    if !school_exists(&conn) {
        return Err("no school yet".into());
    }
    let cert_material = ensure_cert(&conn)?;
    let tls = cert::server_config(&cert_material.cert_der, &cert_material.key_der)?;

    let (listener, port) = net::bind_with_fallback().await.map_err(|e| e.to_string())?;
    let (school_id, _name): (String, String) = conn
        .query_row("SELECT id, name FROM school LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| e.to_string())?;
    let epoch: i64 = conn.query_row("SELECT server_epoch FROM school LIMIT 1", [], |r| r.get(0)).unwrap_or(1);
    let relay_secret: Option<String> = crate::kv::get(&conn, crate::state::KV_RELAY_SECRET).ok().flatten();
    let lan = local_ipv4().into_iter().collect::<Vec<_>>();

    // Publish the LAN facts so invites carry the right address/port/fingerprint.
    crate::kv::set(&conn, "server_port", &port).ok();
    crate::kv::set(&conn, "server_lan_addrs", &lan).ok();

    // Advertise on the LAN (desktop) so clients can find us.
    #[cfg(not(target_os = "android"))]
    let _mdns = {
        let ip = lan.first().cloned().unwrap_or_else(|| "127.0.0.1".to_string());
        crate::sync::mdns::advertise(&school_id, port, &cert_material.fingerprint, &ip).ok()
    };

    tracing::info!("school server listening on 0.0.0.0:{port}");
    let state = Arc::new(net::ServerState::new(conn).with_app(app));

    // Keep ONE outbound tunnel to the relay so devices can reach us over the
    // internet (P05 Step 1). Skipped until the school has a relay secret (issued at
    // activation). The tunnel shares the server's DB + stops on the same signal.
    if let Some(secret) = relay_secret.filter(|s| !s.is_empty()) {
        let relay_url = crate::config::get().relay_url.clone();
        if !relay_url.is_empty() {
            let ts = state.clone();
            let stop_t = stop.clone();
            let sid = school_id.clone();
            tauri::async_runtime::spawn(async move {
                crate::server::tunnel::run_tunnel(ts, relay_url, sid, secret, epoch, stop_t).await;
            });
        }
    }

    net::serve(state, tls, listener, stop).await;
    Ok(())
}
