//! The school-server HTTP/TLS listener (prompts/P04 Step 2/3). axum Router over
//! the `service` layer, served with tokio-rustls + hyper-util (axum's official
//! low-level rustls pattern: `TowerToHyperService` + auto `Builder`). Binds
//! 0.0.0.0:47650, falling back 47651..=47659. Limits: 5 MB body; 30 req/10 s per
//! device (429); bad/missing token → 401; protocol mismatch → 426; epoch → 409.
//!
//! Runtime module: exercised by a running app (the service layer beneath it is
//! covered by tests/sync_e2e.rs and the unit tests).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::{DefaultBodyLimit, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use hyper_util::service::TowerToHyperService;
use rusqlite::Connection;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::sync::protocol::*;

const MAX_BODY: usize = 5_000_000;
const RATE_LIMIT: u32 = 30;
const RATE_WINDOW_SECS: i64 = 10;

/// Emitted after the server applies ops so screens refresh live (Step 11).
pub const EVENT_SYNC_CHANGED: &str = "sync://changed";

/// Shared server state: the DB (its own WAL connection) + a per-device rate map +
/// an optional app handle for live `sync://changed` events.
pub struct ServerState {
    pub db: Mutex<Connection>,
    limiter: Mutex<HashMap<String, (i64, u32)>>,
    app: Option<tauri::AppHandle>,
}

impl ServerState {
    pub fn new(db: Connection) -> Self {
        Self { db: Mutex::new(db), limiter: Mutex::new(HashMap::new()), app: None }
    }
    /// Attach an app handle so the server emits `sync://changed` after applying ops.
    pub fn with_app(mut self, app: tauri::AppHandle) -> Self {
        self.app = Some(app);
        self
    }

    /// 30 requests / 10 s / device (§2). Returns false when over the limit.
    fn allow(&self, device_id: &str, now_unix: i64) -> bool {
        let mut m = self.limiter.lock().unwrap();
        let e = m.entry(device_id.to_string()).or_insert((now_unix, 0));
        if now_unix - e.0 >= RATE_WINDOW_SECS {
            *e = (now_unix, 0);
        }
        e.1 += 1;
        e.1 <= RATE_LIMIT
    }
}

fn now() -> time::OffsetDateTime {
    time::OffsetDateTime::now_utc()
}

fn err(status: StatusCode, code: &str) -> Response {
    (status, Json(serde_json::json!({ "error": code }))).into_response()
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
}

/// Authenticate + rate-limit; returns the device auth or an error response.
#[allow(clippy::result_large_err)] // the Err is an axum Response, returned immediately
fn auth_and_limit(state: &ServerState, headers: &HeaderMap) -> Result<crate::server::service::DeviceAuth, Response> {
    let token = bearer(headers).ok_or_else(|| err(StatusCode::UNAUTHORIZED, codes::DEVICE_REVOKED_OR_UNKNOWN))?;
    let conn = state.db.lock().unwrap();
    let auth = crate::server::service::authenticate(&conn, &token)
        .ok()
        .flatten()
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, codes::DEVICE_REVOKED_OR_UNKNOWN))?;
    drop(conn);
    if !state.allow(&auth.device_id, now().unix_timestamp()) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, codes::RATE_LIMITED));
    }
    Ok(auth)
}

async fn hello(State(st): State<Arc<ServerState>>) -> Response {
    let conn = st.db.lock().unwrap();
    match crate::server::service::hello(&conn, now()) {
        Ok(Some(h)) => (StatusCode::OK, Json(h)).into_response(),
        _ => err(StatusCode::NOT_FOUND, codes::NOT_FOUND),
    }
}

async fn join(State(st): State<Arc<ServerState>>, Json(req): Json<JoinReq>) -> Response {
    let mut conn = st.db.lock().unwrap();
    match crate::server::service::join(&mut conn, &req, now()) {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(_) => err(StatusCode::BAD_REQUEST, codes::INVITE_INVALID),
    }
}

async fn push(State(st): State<Arc<ServerState>>, headers: HeaderMap, Json(req): Json<PushReq>) -> Response {
    if let Err(e) = auth_and_limit(&st, &headers) {
        return e;
    }
    let result = {
        let mut conn = st.db.lock().unwrap();
        crate::server::service::push(&mut conn, &req.ops, now())
    };
    match result {
        Ok(resp) => {
            // Live-update the local screens after applying ops (Step 11).
            if let Some(app) = &st.app {
                use tauri::Emitter;
                let _ = app.emit(EVENT_SYNC_CHANGED, ());
            }
            (StatusCode::OK, Json(resp)).into_response()
        }
        Err(_) => err(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL"),
    }
}

#[derive(serde::Deserialize)]
struct PullParams {
    #[serde(default)]
    since: i64,
    #[serde(default = "default_limit")]
    limit: i64,
}
fn default_limit() -> i64 {
    500
}

async fn pull(State(st): State<Arc<ServerState>>, headers: HeaderMap, Query(p): Query<PullParams>) -> Response {
    let auth = match auth_and_limit(&st, &headers) {
        Ok(a) => a,
        Err(e) => return e,
    };
    let conn = st.db.lock().unwrap();
    match crate::server::service::pull(&conn, &auth, p.since, p.limit, now()) {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(_) => err(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL"),
    }
}

async fn snapshot(State(st): State<Arc<ServerState>>, headers: HeaderMap) -> Response {
    let auth = match auth_and_limit(&st, &headers) {
        Ok(a) => a,
        Err(e) => return e,
    };
    let conn = st.db.lock().unwrap();
    match crate::server::service::snapshot(&conn, &auth) {
        // Newline-delimited JSON (§3, streamed in a fuller build).
        Ok(changes) => {
            let body: String = changes.iter().filter_map(|c| serde_json::to_string(c).ok()).collect::<Vec<_>>().join("\n");
            (StatusCode::OK, body).into_response()
        }
        Err(_) => err(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL"),
    }
}

async fn heartbeat(State(st): State<Arc<ServerState>>, headers: HeaderMap, Json(req): Json<HeartbeatReq>) -> Response {
    let auth = match auth_and_limit(&st, &headers) {
        Ok(a) => a,
        Err(e) => return e,
    };
    let mut conn = st.db.lock().unwrap();
    match crate::server::service::heartbeat(&mut conn, &auth, &req, now()) {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(_) => err(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL"),
    }
}

/// The `/v1` router.
pub fn router(state: Arc<ServerState>) -> Router {
    Router::new()
        .route("/v1/hello", get(hello))
        .route("/v1/join", post(join))
        .route("/v1/sync/push", post(push))
        .route("/v1/sync/pull", get(pull))
        .route("/v1/sync/snapshot", post(snapshot))
        .route("/v1/device/heartbeat", post(heartbeat))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(state)
}

/// Bind 47650, falling back through 47659; return the listener + actual port.
pub async fn bind_with_fallback() -> std::io::Result<(TcpListener, u16)> {
    let mut last_err = None;
    for port in DEFAULT_PORT..=PORT_RANGE_END {
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        match TcpListener::bind(addr).await {
            Ok(l) => return Ok((l, port)),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| std::io::Error::other("no port available")))
}

/// Serve TLS connections until cancelled (axum low-level rustls pattern).
pub async fn serve(state: Arc<ServerState>, tls: Arc<rustls::ServerConfig>, listener: TcpListener) {
    let app = router(state);
    let acceptor = TlsAcceptor::from(tls);
    loop {
        let (tcp, _peer) = match listener.accept().await {
            Ok(v) => v,
            Err(_) => continue,
        };
        let acceptor = acceptor.clone();
        let app = app.clone();
        tokio::spawn(async move {
            let stream = match acceptor.accept(tcp).await {
                Ok(s) => s,
                Err(_) => return,
            };
            let io = TokioIo::new(stream);
            let svc = TowerToHyperService::new(app);
            let _ = Builder::new(TokioExecutor::new()).serve_connection_with_upgrades(io, svc).await;
        });
    }
}
