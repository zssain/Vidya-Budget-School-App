//! Vidya relay (prompts/P05 Step 1, docs §8.3 route 2).
//!
//! A small, SEPARATE Rust binary — NOT a member of the app workspace, so it never
//! ships inside the apps. It lets staff reach a school server from anywhere without
//! the school opening router ports:
//!
//! * The school server keeps ONE outbound WebSocket to `/tunnel/<school_id>`,
//!   authenticated with its relay secret (`HMAC-SHA256(RELAY_SHARED_KEY, school_id)`
//!   — the same recipe `cloud/licence` issues, so the relay verifies it locally and
//!   never calls the licence service; it stays stateless).
//! * Devices call `POST /s/<school_id>/v1/...`; the relay forwards each request as a
//!   frame over the tunnel and streams the response back.
//!
//! The relay is STATELESS: no database, no disk writes except rotated logs. Every
//! device body is end-to-end SEALED between the device and the school (see the app's
//! `sync::seal`), so the relay can read neither the request nor the response — it
//! only ever sees the school id, sizes, timings and status codes it logs (DONE #2).
//!
//! Env config:
//!   RELAY_BIND        bind address (default 0.0.0.0:8788)
//!   RELAY_SHARED_KEY  shared key for relay-secret verification (must match cloud/licence)
//!
//! TLS: terminate at a reverse proxy (Caddy/nginx/Cloud Run) in front of the relay —
//! this is what the Dockerfile assumes. See docs/phase-notes/phase-5.md for the two
//! documented options (proxy vs in-process rustls).

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::Bytes;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::Router;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tokio::sync::{mpsc, oneshot, Notify, Semaphore};

// ------------------------------------------------------------------ config --

const DEFAULT_BIND: &str = "0.0.0.0:8788";
const DEV_RELAY_SHARED_KEY: &str = "vidya-dev-relay-shared-key-change-me";
/// Max body a device may send (§ Step 1: 5 MB).
const MAX_BODY: usize = 5_000_000;
/// Max concurrent in-flight requests per school.
const MAX_CONCURRENT_PER_SCHOOL: usize = 64;
/// Per-IP fixed-window rate limit.
const IP_RATE_LIMIT: u32 = 240;
const IP_RATE_WINDOW: Duration = Duration::from_secs(10);
/// Forward timeout for a single device request.
const FORWARD_TIMEOUT: Duration = Duration::from_secs(30);
/// Idle keep-alive ping interval on the tunnel.
const PING_SECS: u64 = 25;

/// Private-use WebSocket close code for a superseded tunnel (§8.9 fencing).
const CLOSE_EPOCH_OLD: u16 = 4409;

// ----------------------------------------------------------------- frames ---
// The tunnel wire format (relay <-> school). Kept byte-compatible with the app's
// `server::tunnel` frames. Bodies are base64 of the RAW HTTP body bytes — which are
// themselves the device's SEALED envelope, so the relay never sees plaintext.

/// Relay → school: forward this device request.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReqFrame {
    id: u64,
    method: String,
    path: String,
    body_b64: String,
}

/// School → relay: the response for request `id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RespFrame {
    id: u64,
    status: u16,
    body_b64: String,
}

// ----------------------------------------------------------------- state ----

/// One in-flight device request awaiting its school response.
struct Pending {
    frame: ReqFrame,
    reply: oneshot::Sender<RespFrame>,
}

/// A live school tunnel.
struct Tunnel {
    /// The school server's epoch when it connected (drives one-tunnel-per-school).
    epoch: i64,
    /// Unique per-connection id (so a tunnel only deregisters itself, not a replacement).
    uid: u64,
    /// Send outgoing request frames to the tunnel task.
    outgoing: mpsc::Sender<Pending>,
    /// Frame-id source.
    next_id: AtomicU64,
    /// Per-school concurrency limiter.
    sem: Arc<Semaphore>,
    /// Fired to close this tunnel when a newer-epoch tunnel replaces it.
    close: Arc<Notify>,
}

struct AppState {
    tunnels: Mutex<HashMap<String, Arc<Tunnel>>>,
    shared_key: Vec<u8>,
    ip_limiter: Mutex<HashMap<IpAddr, (Instant, u32)>>,
    next_uid: AtomicU64,
}

impl AppState {
    /// Fixed-window per-IP rate limit. Returns false when over the limit.
    fn allow_ip(&self, ip: IpAddr) -> bool {
        let mut m = self.ip_limiter.lock().unwrap();
        let e = m.entry(ip).or_insert((Instant::now(), 0));
        if e.0.elapsed() >= IP_RATE_WINDOW {
            *e = (Instant::now(), 0);
        }
        e.1 += 1;
        e.1 <= IP_RATE_LIMIT
    }
}

// --------------------------------------------------------------- helpers ----

/// The relay secret for a school: `base64(HMAC-SHA256(shared_key, school_id))`.
/// Must stay byte-identical to `cloud/licence`'s `relay_secret`.
fn relay_secret(shared_key: &[u8], school_id: &str) -> String {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(shared_key).expect("HMAC accepts any key length");
    mac.update(school_id.as_bytes());
    STANDARD.encode(mac.finalize().into_bytes())
}

/// Constant-time byte compare.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
}

/// The single access-log line for a forwarded request. Deliberately contains ONLY
/// the school id, method, path, byte sizes, status and timing — never a body,
/// header, token, name or number (DONE MEANS #2).
fn access_log(school: &str, method: &str, path: &str, req_bytes: usize, status: u16, resp_bytes: usize, ms: u128) -> String {
    format!("relay school={school} method={method} path={path} req_bytes={req_bytes} status={status} resp_bytes={resp_bytes} ms={ms}")
}

fn err(status: StatusCode, code: &str) -> Response {
    (status, code.to_string()).into_response()
}

// -------------------------------------------------------------- handlers ----

async fn healthz() -> &'static str {
    "ok"
}

#[derive(Deserialize)]
struct TunnelQuery {
    #[serde(default = "default_epoch")]
    epoch: i64,
}
fn default_epoch() -> i64 {
    1
}

/// `GET /tunnel/<school_id>` — the school server's outbound WebSocket. Verifies the
/// relay secret + epoch, then upgrades and runs the tunnel.
async fn tunnel_handler(
    Path(school): Path<String>,
    Query(q): Query<TunnelQuery>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let presented = bearer(&headers).unwrap_or_default();
    let expected = relay_secret(&state.shared_key, &school);
    if !ct_eq(presented.as_bytes(), expected.as_bytes()) {
        tracing::warn!("tunnel auth rejected school={school}");
        return err(StatusCode::UNAUTHORIZED, "UNAUTHORIZED");
    }
    // Reject a stale server (its epoch is older than the tunnel we already hold).
    {
        let m = state.tunnels.lock().unwrap();
        if let Some(t) = m.get(&school) {
            if t.epoch > q.epoch {
                tracing::warn!("tunnel refused (stale epoch) school={school} epoch={} have={}", q.epoch, t.epoch);
                return err(StatusCode::CONFLICT, "EPOCH_OLD");
            }
        }
    }
    let uid = state.next_uid.fetch_add(1, Ordering::SeqCst);
    let epoch = q.epoch;
    ws.on_upgrade(move |socket| run_tunnel(socket, school, epoch, uid, state))
}

/// Own the tunnel socket: register it (replacing any older-epoch tunnel), then
/// pump request frames out and response frames back until it closes.
async fn run_tunnel(mut socket: WebSocket, school: String, epoch: i64, uid: u64, state: Arc<AppState>) {
    let (tx, mut rx) = mpsc::channel::<Pending>(256);
    let tunnel = Arc::new(Tunnel {
        epoch,
        uid,
        outgoing: tx,
        next_id: AtomicU64::new(1),
        sem: Arc::new(Semaphore::new(MAX_CONCURRENT_PER_SCHOOL)),
        close: Arc::new(Notify::new()),
    });
    {
        let mut m = state.tunnels.lock().unwrap();
        if let Some(old) = m.insert(school.clone(), tunnel.clone()) {
            tracing::info!("tunnel replaced school={school} old_epoch={} new_epoch={epoch}", old.epoch);
            old.close.notify_waiters();
        } else {
            tracing::info!("tunnel up school={school} epoch={epoch}");
        }
    }

    let close = tunnel.close.clone();
    let mut pending: HashMap<u64, oneshot::Sender<RespFrame>> = HashMap::new();
    let mut ping = tokio::time::interval(Duration::from_secs(PING_SECS));
    ping.tick().await; // consume the immediate first tick

    loop {
        tokio::select! {
            _ = close.notified() => {
                let _ = socket
                    .send(Message::Close(Some(CloseFrame { code: CLOSE_EPOCH_OLD, reason: "EPOCH_OLD".into() })))
                    .await;
                break;
            }
            inc = socket.recv() => match inc {
                Some(Ok(Message::Text(t))) => deliver(&mut pending, serde_json::from_str(&t).ok()),
                Some(Ok(Message::Binary(b))) => deliver(&mut pending, serde_json::from_slice(&b).ok()),
                Some(Ok(Message::Ping(p))) => { let _ = socket.send(Message::Pong(p)).await; }
                Some(Ok(Message::Pong(_))) => {}
                _ => break, // Close / error / stream end
            },
            Some(p) = rx.recv() => {
                pending.insert(p.frame.id, p.reply);
                let txt = serde_json::to_string(&p.frame).unwrap_or_default();
                if socket.send(Message::Text(txt)).await.is_err() {
                    break;
                }
            }
            _ = ping.tick() => {
                if socket.send(Message::Ping(Vec::new())).await.is_err() {
                    break;
                }
            }
        }
    }

    // Deregister — but only if we are still the current tunnel for this school.
    {
        let mut m = state.tunnels.lock().unwrap();
        if m.get(&school).map(|t| t.uid) == Some(uid) {
            m.remove(&school);
            tracing::info!("tunnel down school={school} epoch={epoch}");
        }
    }
    // Dropping `pending` fails every in-flight device request → they return 503.
}

/// Deliver a parsed response frame to the waiting device handler.
fn deliver(pending: &mut HashMap<u64, oneshot::Sender<RespFrame>>, frame: Option<RespFrame>) {
    if let Some(rf) = frame {
        if let Some(reply) = pending.remove(&rf.id) {
            let _ = reply.send(rf);
        }
    }
}

/// `ANY /s/<school_id>/<...>` — a device request. Forwarded over the school tunnel;
/// `503 SCHOOL_OFFLINE` when no tunnel is connected.
async fn device_handler(
    Path((school, subpath)): Path<(String, String)>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>,
    method: Method,
    body: Bytes,
) -> Response {
    let started = Instant::now();
    let path = format!("/{subpath}");
    let req_bytes = body.len();
    let done = |status: u16, resp_bytes: usize| {
        tracing::info!("{}", access_log(&school, method.as_str(), &path, req_bytes, status, resp_bytes, started.elapsed().as_millis()));
    };

    if !state.allow_ip(addr.ip()) {
        done(429, 0);
        return err(StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED");
    }
    let tunnel = { state.tunnels.lock().unwrap().get(&school).cloned() };
    let tunnel = match tunnel {
        Some(t) => t,
        None => {
            done(503, 0);
            return err(StatusCode::SERVICE_UNAVAILABLE, "SCHOOL_OFFLINE");
        }
    };
    // Per-school concurrency limit.
    let _permit = match tunnel.sem.clone().try_acquire_owned() {
        Ok(p) => p,
        Err(_) => {
            done(429, 0);
            return err(StatusCode::TOO_MANY_REQUESTS, "SCHOOL_BUSY");
        }
    };

    let id = tunnel.next_id.fetch_add(1, Ordering::SeqCst);
    let frame = ReqFrame { id, method: method.as_str().to_string(), path: path.clone(), body_b64: STANDARD.encode(&body) };
    let (reply_tx, reply_rx) = oneshot::channel();
    if tunnel.outgoing.send(Pending { frame, reply: reply_tx }).await.is_err() {
        done(503, 0);
        return err(StatusCode::SERVICE_UNAVAILABLE, "SCHOOL_OFFLINE");
    }
    match tokio::time::timeout(FORWARD_TIMEOUT, reply_rx).await {
        Ok(Ok(rf)) => {
            let bytes = STANDARD.decode(&rf.body_b64).unwrap_or_default();
            done(rf.status, bytes.len());
            (StatusCode::from_u16(rf.status).unwrap_or(StatusCode::BAD_GATEWAY), bytes).into_response()
        }
        Ok(Err(_)) => {
            done(503, 0);
            err(StatusCode::SERVICE_UNAVAILABLE, "SCHOOL_OFFLINE")
        }
        Err(_) => {
            done(504, 0);
            err(StatusCode::GATEWAY_TIMEOUT, "TIMEOUT")
        }
    }
}

fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/tunnel/:school_id", get(tunnel_handler))
        .route("/s/:school_id/*path", any(device_handler))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(state)
}

fn make_state() -> Arc<AppState> {
    let shared_key = std::env::var("RELAY_SHARED_KEY")
        .unwrap_or_else(|_| DEV_RELAY_SHARED_KEY.to_string())
        .into_bytes();
    Arc::new(AppState {
        tunnels: Mutex::new(HashMap::new()),
        shared_key,
        ip_limiter: Mutex::new(HashMap::new()),
        next_uid: AtomicU64::new(1),
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_target(false).init();
    let bind = std::env::var("RELAY_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_string());
    let state = make_state();
    let listener = tokio::net::TcpListener::bind(&bind).await.expect("bind relay");
    tracing::info!("vidya relay listening on {bind}");
    axum::serve(listener, app(state).into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("relay shutting down");
        })
        .await
        .expect("serve relay");
}

// -------------------------------------------------------------------- tests --

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_secret_matches_recipe() {
        // Byte-identical to cloud/licence's relay_secret; bound to school + key.
        let k = b"shared-key";
        assert_eq!(relay_secret(k, "sch_abc"), relay_secret(k, "sch_abc"));
        assert_ne!(relay_secret(k, "sch_abc"), relay_secret(k, "sch_xyz"));
        assert_ne!(relay_secret(k, "sch_abc"), relay_secret(b"other", "sch_abc"));
        assert_eq!(STANDARD.decode(relay_secret(k, "sch_abc")).unwrap().len(), 32);
    }

    #[test]
    fn ct_eq_matches_and_rejects() {
        assert!(ct_eq(b"abc", b"abc"));
        assert!(!ct_eq(b"abc", b"abd"));
        assert!(!ct_eq(b"abc", b"abcd"));
    }

    #[test]
    fn access_log_carries_only_safe_fields() {
        // DONE MEANS #2: no body/token/name in the log line.
        let line = access_log("sch_1", "POST", "/v1/sealed", 812, 200, 640, 7);
        assert_eq!(line, "relay school=sch_1 method=POST path=/v1/sealed req_bytes=812 status=200 resp_bytes=640 ms=7");
        for secret in ["Bearer", "token", "Kavya", "3100", "password", "name"] {
            assert!(!line.contains(secret), "log must not contain {secret}");
        }
    }

    // ---- integration: a fake school (WS client) + a device (HTTP client) --------

    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::Message as WsMsg;

    const TEST_KEY: &str = "test-shared-key";

    async fn spawn_relay() -> SocketAddr {
        let state = Arc::new(AppState {
            tunnels: Mutex::new(HashMap::new()),
            shared_key: TEST_KEY.as_bytes().to_vec(),
            ip_limiter: Mutex::new(HashMap::new()),
            next_uid: AtomicU64::new(1),
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app(state).into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
        });
        addr
    }

    /// Connect a fake school tunnel that replies to each request frame with
    /// `status`/`reply_body`; returns the read half so a test can watch for a close.
    async fn fake_school(addr: SocketAddr, school: &str, epoch: i64, reply_body: &'static [u8]) {
        let secret = relay_secret(TEST_KEY.as_bytes(), school);
        let mut req = format!("ws://{addr}/tunnel/{school}?epoch={epoch}").into_client_request().unwrap();
        req.headers_mut().insert("Authorization", format!("Bearer {secret}").parse().unwrap());
        let (ws, _resp) = tokio_tungstenite::connect_async(req).await.unwrap();
        let (mut write, mut read) = ws.split();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = read.next().await {
                if let WsMsg::Text(t) = msg {
                    let rf: ReqFrame = serde_json::from_str(&t).unwrap();
                    let resp = RespFrame { id: rf.id, status: 200, body_b64: STANDARD.encode(reply_body) };
                    if write.send(WsMsg::Text(serde_json::to_string(&resp).unwrap())).await.is_err() {
                        break;
                    }
                }
            }
        });
    }

    #[tokio::test]
    async fn device_gets_503_when_no_tunnel() {
        let addr = spawn_relay().await;
        let client = reqwest::Client::new();
        let resp = client.post(format!("http://{addr}/s/sch_x/v1/sealed")).body("blob").send().await.unwrap();
        assert_eq!(resp.status().as_u16(), 503);
        assert_eq!(resp.text().await.unwrap(), "SCHOOL_OFFLINE");
    }

    #[tokio::test]
    async fn forwards_request_and_streams_response() {
        // DONE MEANS #1 (relay half): a device request is forwarded over the tunnel
        // and the school's response is streamed back unchanged.
        let addr = spawn_relay().await;
        fake_school(addr, "sch_1", 1, b"pong").await;
        tokio::time::sleep(Duration::from_millis(150)).await; // let the tunnel register
        let client = reqwest::Client::new();
        let resp = client.post(format!("http://{addr}/s/sch_1/v1/sealed")).body("sealed-bytes").send().await.unwrap();
        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(resp.bytes().await.unwrap().as_ref(), b"pong");
    }

    #[tokio::test]
    async fn bad_relay_secret_is_rejected() {
        let addr = spawn_relay().await;
        let mut req = format!("ws://{addr}/tunnel/sch_1?epoch=1").into_client_request().unwrap();
        req.headers_mut().insert("Authorization", "Bearer wrong".parse().unwrap());
        // The upgrade must fail (HTTP 401, no websocket).
        assert!(tokio_tungstenite::connect_async(req).await.is_err());
    }

    #[tokio::test]
    async fn newer_epoch_replaces_older_tunnel() {
        // §8.9: a newer-epoch tunnel replaces the older one; device traffic then
        // routes to the newer server.
        let addr = spawn_relay().await;
        fake_school(addr, "sch_1", 1, b"one").await;
        tokio::time::sleep(Duration::from_millis(150)).await;
        fake_school(addr, "sch_1", 2, b"two").await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        let client = reqwest::Client::new();
        let resp = client.post(format!("http://{addr}/s/sch_1/v1/sealed")).body("x").send().await.unwrap();
        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(resp.bytes().await.unwrap().as_ref(), b"two", "routes to the newer-epoch server");
    }
}
