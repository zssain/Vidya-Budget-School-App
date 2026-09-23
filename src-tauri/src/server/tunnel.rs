//! School-side relay tunnel client (prompts/P05 Step 1). The school server keeps
//! ONE outbound WebSocket to `RELAY_URL/tunnel/<school_id>`, authenticated with its
//! relay secret. It receives framed device requests, serves them against the SAME
//! `sealed` dispatch the LAN listener uses, and streams framed responses back —
//! reconnecting with backoff, answering keep-alive pings, and stopping for good when
//! the relay fences it (`4409 EPOCH_OLD`) or the app asks it to (licence `moved`).
//!
//! Runtime module: the framing + dispatch are unit-tested (`process_frame`) and the
//! full socket path is exercised by `tests/relay_e2e.rs`; the live wss reconnect
//! loop against a hosted relay is verified once hosting exists (STOP: do not deploy).

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

use crate::server::net::ServerState;
use crate::server::sealed;
use crate::sync::protocol::SealedEnvelope;

/// Relay close code for a superseded tunnel (matches `cloud/relay`).
const CLOSE_EPOCH_OLD: u16 = 4409;
/// Reconnect backoff (seconds): 1 → 2 → 4 … capped at 30 (Step 1 "reconnect with backoff").
fn backoff_secs(attempt: u32) -> u64 {
    (1u64 << attempt.min(5)).min(30)
}

// -------- frames (byte-compatible with cloud/relay's ReqFrame/RespFrame) ------

#[derive(Debug, Clone, Deserialize)]
struct ReqFrame {
    id: u64,
    #[allow(dead_code)]
    method: String,
    path: String,
    body_b64: String,
}

#[derive(Debug, Clone, Serialize)]
struct RespFrame {
    id: u64,
    status: u16,
    body_b64: String,
}

/// Why the serve loop ended.
#[derive(Debug, PartialEq, Eq)]
enum TunnelEnd {
    /// The relay fenced us (superseded by a newer epoch) — stop for good.
    Fenced,
    /// The app asked us to stop (licence `moved`) — stop for good.
    Stopped,
    /// Auth was rejected (bad secret) — stop; retrying won't help this run.
    AuthFailed,
    /// The connection dropped — reconnect after backoff.
    Disconnected,
}

fn now() -> time::OffsetDateTime {
    time::OffsetDateTime::now_utc()
}

/// Serve one framed device request against the sealed dispatch. Pure w.r.t. the
/// socket (takes the decoded body, returns the response frame) so it is unit-tested.
fn process_frame(state: &ServerState, id: u64, path: &str, body: &[u8]) -> (RespFrame, bool) {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    // Devices only ever reach the school over the relay through the sealed endpoint.
    if !path.ends_with("/v1/sealed") {
        return (RespFrame { id, status: 404, body_b64: STANDARD.encode(b"NOT_FOUND") }, false);
    }
    let env: SealedEnvelope = match serde_json::from_slice(body) {
        Ok(e) => e,
        Err(_) => return (RespFrame { id, status: 400, body_b64: STANDARD.encode(b"BAD_REQUEST") }, false),
    };
    // Only a push can change data → refresh local screens (parity with the LAN
    // push handler). The envelope's `path` is plaintext routing metadata.
    let is_push = env.path.ends_with("/v1/sync/push");
    let mut conn = state.db.lock().unwrap();
    match sealed::dispatch(&mut conn, &env, now()) {
        Ok(resp_env) => {
            let body = serde_json::to_vec(&resp_env).unwrap_or_default();
            (RespFrame { id, status: 200, body_b64: STANDARD.encode(body) }, is_push)
        }
        Err(e) => (RespFrame { id, status: e.http_status(), body_b64: STANDARD.encode(e.code().as_bytes()) }, false),
    }
}

/// Connect once and serve until the socket closes or `stop` fires.
async fn connect_and_serve(
    state: &Arc<ServerState>,
    relay_url: &str,
    school_id: &str,
    relay_secret: &str,
    epoch: i64,
    stop: &Notify,
) -> TunnelEnd {
    let url = format!("{}/tunnel/{}?epoch={}", relay_url.trim_end_matches('/'), school_id, epoch);
    let mut req = match url.into_client_request() {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("relay tunnel: bad url: {e}");
            return TunnelEnd::AuthFailed;
        }
    };
    match format!("Bearer {relay_secret}").parse() {
        Ok(v) => {
            req.headers_mut().insert(axum::http::header::AUTHORIZATION, v);
        }
        Err(_) => return TunnelEnd::AuthFailed,
    }

    let ws = match tokio_tungstenite::connect_async(req).await {
        Ok((ws, _resp)) => ws,
        Err(tokio_tungstenite::tungstenite::Error::Http(resp)) => {
            return match resp.status().as_u16() {
                409 => {
                    tracing::warn!("relay tunnel fenced (EPOCH_OLD) school={school_id}");
                    TunnelEnd::Fenced
                }
                401 => {
                    tracing::warn!("relay tunnel auth rejected school={school_id}");
                    TunnelEnd::AuthFailed
                }
                _ => TunnelEnd::Disconnected,
            };
        }
        Err(_) => return TunnelEnd::Disconnected,
    };
    tracing::info!("relay tunnel up school={school_id} epoch={epoch}");

    let mut ws = ws;
    loop {
        tokio::select! {
            _ = stop.notified() => {
                let _ = ws.close(None).await;
                return TunnelEnd::Stopped;
            }
            inc = ws.next() => match inc {
                Some(Ok(Message::Text(t))) => {
                    if let Ok(rf) = serde_json::from_str::<ReqFrame>(&t) {
                        if serve_and_reply(state, &mut ws, rf).await.is_err() {
                            return TunnelEnd::Disconnected;
                        }
                    }
                }
                Some(Ok(Message::Binary(b))) => {
                    if let Ok(rf) = serde_json::from_slice::<ReqFrame>(&b) {
                        if serve_and_reply(state, &mut ws, rf).await.is_err() {
                            return TunnelEnd::Disconnected;
                        }
                    }
                }
                Some(Ok(Message::Ping(p))) => {
                    if ws.send(Message::Pong(p)).await.is_err() {
                        return TunnelEnd::Disconnected;
                    }
                }
                Some(Ok(Message::Pong(_))) => {}
                Some(Ok(Message::Close(frame))) => {
                    if frame.map(|f| u16::from(f.code)) == Some(CLOSE_EPOCH_OLD) {
                        tracing::warn!("relay tunnel fenced (EPOCH_OLD close) school={school_id}");
                        return TunnelEnd::Fenced;
                    }
                    return TunnelEnd::Disconnected;
                }
                _ => return TunnelEnd::Disconnected, // frame/error/stream end
            }
        }
    }
}

/// Decode a request frame, serve it, send the reply; emit `sync://changed` when the
/// request applied ops locally.
async fn serve_and_reply(
    state: &Arc<ServerState>,
    ws: &mut tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    rf: ReqFrame,
) -> Result<(), ()> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    let body = STANDARD.decode(&rf.body_b64).unwrap_or_default();
    let (resp, applied) = process_frame(state, rf.id, &rf.path, &body);
    if applied {
        state.emit_changed();
    }
    let txt = serde_json::to_string(&resp).map_err(|_| ())?;
    ws.send(Message::Text(txt)).await.map_err(|_| ())?;
    Ok(())
}

/// Run the tunnel with reconnect/backoff until fenced, auth-failed, or stopped.
pub async fn run_tunnel(
    state: Arc<ServerState>,
    relay_url: String,
    school_id: String,
    relay_secret: String,
    epoch: i64,
    stop: Arc<Notify>,
) {
    let mut attempt = 0u32;
    loop {
        match connect_and_serve(&state, &relay_url, &school_id, &relay_secret, epoch, &stop).await {
            TunnelEnd::Fenced | TunnelEnd::Stopped | TunnelEnd::AuthFailed => {
                tracing::info!("relay tunnel stopped school={school_id}");
                return;
            }
            TunnelEnd::Disconnected => {
                let wait = backoff_secs(attempt);
                attempt = attempt.saturating_add(1);
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_secs(wait)) => {}
                    _ = stop.notified() => return,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::protocol::{SealedRequest, SealedResponse};
    use crate::sync::seal;
    use crate::{db, seed};
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use rusqlite::Connection;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn t_now() -> time::OffsetDateTime {
        time::OffsetDateTime::parse("2026-09-23T12:00:00Z", &time::format_description::well_known::Rfc3339).unwrap()
    }

    fn server_state(session_key: &str) -> (Arc<ServerState>, seal::DirectionKeys) {
        let mut c: Connection = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        seed::seed_demo_school(&mut c, t_now()).unwrap();
        let staff_id: String = c.query_row("SELECT id FROM staff WHERE role='principal' LIMIT 1", [], |r| r.get(0)).unwrap();
        c.execute(
            "INSERT INTO device(id,staff_id,platform,name,token_hash,session_key,receipt_series,admission_series,last_seen_at,lease_expires_at,needs_rejoin,last_counter) \
             VALUES ('dev-relay',?1,'android','Phone','th',?2,'A9','A9','t','t',0,0)",
            rusqlite::params![staff_id, session_key],
        ).unwrap();
        (Arc::new(ServerState::new(c)), seal::derive_keys(session_key).unwrap())
    }

    #[test]
    fn backoff_curve() {
        assert_eq!(backoff_secs(0), 1);
        assert_eq!(backoff_secs(1), 2);
        assert_eq!(backoff_secs(2), 4);
        assert_eq!(backoff_secs(9), 30); // capped
    }

    #[test]
    fn process_frame_serves_sealed_hello() {
        let session_key = STANDARD.encode([9u8; 32]);
        let (state, keys) = server_state(&session_key);
        // Build a sealed /v1/hello request the way a device would, wrap it as a body.
        let aad = seal::associated_data("POST", "/v1/hello", "dev-relay", 1);
        let inner = serde_json::to_vec(&SealedRequest { counter: 1, body: serde_json::json!({}) }).unwrap();
        let env = SealedEnvelope {
            device_id: "dev-relay".into(), method: "POST".into(), path: "/v1/hello".into(),
            server_epoch: 1, sealed_b64: seal::seal_b64(&keys.c2s, &aad, &inner),
        };
        let body = serde_json::to_vec(&env).unwrap();
        let (resp, applied) = process_frame(&state, 42, "/v1/sealed", &body);
        assert_eq!(resp.id, 42);
        assert_eq!(resp.status, 200);
        assert!(!applied); // hello applies no ops
        // The response body is a sealed envelope the device can open.
        let resp_env: SealedEnvelope = serde_json::from_slice(&STANDARD.decode(resp.body_b64).unwrap()).unwrap();
        let raad = seal::associated_data("POST", "/v1/hello", "dev-relay", 1);
        let plain = seal::open_b64(&keys.s2c, &raad, &resp_env.sealed_b64).unwrap();
        let sr: SealedResponse = serde_json::from_slice(&plain).unwrap();
        assert_eq!(sr.status, 200);
    }

    #[test]
    fn process_frame_rejects_non_sealed_path() {
        let session_key = STANDARD.encode([9u8; 32]);
        let (state, _keys) = server_state(&session_key);
        let (resp, applied) = process_frame(&state, 1, "/v1/hello", b"{}");
        assert_eq!(resp.status, 404);
        assert!(!applied);
    }
}
