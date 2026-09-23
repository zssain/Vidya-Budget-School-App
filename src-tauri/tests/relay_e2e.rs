//! Relay end-to-end harness (prompts/P05 Step 6). Stands up an **in-process relay**
//! (axum: a WebSocket tunnel + a device HTTP forward) on a real localhost socket,
//! connects the REAL school tunnel client (`server::tunnel::run_tunnel`), and drives
//! the REAL client relay transport (`sync::relay::RelayTransport`). So the whole
//! chain is exercised over sockets: device seals → HTTP → relay → WS tunnel → school
//! `sealed::dispatch` → apply → sealed response → back.
//!
//! Scenarios: phone reachable ONLY via the relay records an op → confirmed (DONE #1);
//! relay offline → queue, back → resumes (DONE #4); one byte tampered in transit →
//! rejected, nothing applied (DONE #3); relay drops a response, retry with the same
//! op_id → applied once; slow 3G-ish network still confirms (+ measured latency).
//!
//! The in-process relay mirrors `cloud/relay`'s frame contract; the deployable relay
//! is tested independently in `cloud/relay`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::Router;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, Notify};

use vidya_lib::server::net::ServerState;
use vidya_lib::server::tunnel::run_tunnel;
use vidya_lib::sync::engine::status_label;
use vidya_lib::sync::protocol::{Op, OpStatus};
use vidya_lib::sync::relay::{RelaySealer, RelayTransport};
use vidya_lib::sync::transport::Transport;
use vidya_lib::{db, seed};

const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn now() -> time::OffsetDateTime {
    time::OffsetDateTime::parse("2026-09-24T12:00:00Z", &time::format_description::well_known::Rfc3339).unwrap()
}

// ------------------------- in-process relay (axum) ---------------------------
// Frames match cloud/relay + server::tunnel byte-for-byte.

#[derive(Serialize, Deserialize)]
struct ReqFrame {
    id: u64,
    method: String,
    path: String,
    body_b64: String,
}
#[derive(Serialize, Deserialize)]
struct RespFrame {
    id: u64,
    status: u16,
    body_b64: String,
}

struct TunnelH {
    tx: mpsc::Sender<(ReqFrame, oneshot::Sender<RespFrame>)>,
    next_id: AtomicU64,
}

struct Hub {
    tunnels: Mutex<HashMap<String, Arc<TunnelH>>>,
    /// Injected forward latency (slow-network scenario).
    delay_ms: AtomicU64,
    /// Drop the NEXT response (relay-drops-mid-request scenario). The request still
    /// reaches the school (applied); the device just never gets the reply.
    drop_next: AtomicBool,
}

impl Hub {
    fn new() -> Arc<Self> {
        Arc::new(Hub { tunnels: Mutex::new(HashMap::new()), delay_ms: AtomicU64::new(0), drop_next: AtomicBool::new(false) })
    }
}

async fn tunnel_handler(Path(school): Path<String>, State(hub): State<Arc<Hub>>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| hub_tunnel(socket, school, hub))
}

async fn hub_tunnel(mut socket: WebSocket, school: String, hub: Arc<Hub>) {
    let (tx, mut rx) = mpsc::channel::<(ReqFrame, oneshot::Sender<RespFrame>)>(64);
    hub.tunnels.lock().unwrap().insert(school.clone(), Arc::new(TunnelH { tx, next_id: AtomicU64::new(1) }));
    let mut pending: HashMap<u64, oneshot::Sender<RespFrame>> = HashMap::new();
    loop {
        tokio::select! {
            inc = socket.recv() => match inc {
                Some(Ok(Message::Text(t))) => {
                    if let Ok(rf) = serde_json::from_str::<RespFrame>(&t) {
                        if let Some(s) = pending.remove(&rf.id) { let _ = s.send(rf); }
                    }
                }
                Some(Ok(Message::Ping(p))) => { let _ = socket.send(Message::Pong(p)).await; }
                Some(Ok(Message::Pong(_))) => {}
                _ => break,
            },
            Some((frame, reply)) = rx.recv() => {
                pending.insert(frame.id, reply);
                if socket.send(Message::Text(serde_json::to_string(&frame).unwrap())).await.is_err() { break; }
            }
        }
    }
    hub.tunnels.lock().unwrap().remove(&school);
}

async fn device_handler(Path((school, subpath)): Path<(String, String)>, State(hub): State<Arc<Hub>>, body: Bytes) -> Response {
    let delay = hub.delay_ms.load(Ordering::SeqCst);
    if delay > 0 {
        tokio::time::sleep(Duration::from_millis(delay)).await;
    }
    let tunnel = hub.tunnels.lock().unwrap().get(&school).cloned();
    let Some(tunnel) = tunnel else {
        return (StatusCode::SERVICE_UNAVAILABLE, "SCHOOL_OFFLINE").into_response();
    };
    let id = tunnel.next_id.fetch_add(1, Ordering::SeqCst);
    let frame = ReqFrame { id, method: "POST".into(), path: format!("/{subpath}"), body_b64: STANDARD.encode(&body) };
    let (rtx, rrx) = oneshot::channel();
    if tunnel.tx.send((frame, rtx)).await.is_err() {
        return (StatusCode::SERVICE_UNAVAILABLE, "SCHOOL_OFFLINE").into_response();
    }
    let resp = match tokio::time::timeout(Duration::from_secs(5), rrx).await {
        Ok(Ok(rf)) => rf,
        _ => return (StatusCode::GATEWAY_TIMEOUT, "TIMEOUT").into_response(),
    };
    // The request reached the school (applied); simulate a lost response.
    if hub.drop_next.swap(false, Ordering::SeqCst) {
        return (StatusCode::GATEWAY_TIMEOUT, "DROPPED").into_response();
    }
    let bytes = STANDARD.decode(&resp.body_b64).unwrap_or_default();
    (StatusCode::from_u16(resp.status).unwrap_or(StatusCode::BAD_GATEWAY), bytes).into_response()
}

async fn start_hub(hub: Arc<Hub>) -> SocketAddr {
    let app = Router::new()
        .route("/tunnel/:school", get(tunnel_handler))
        .route("/s/:school/*path", any(device_handler))
        .with_state(hub);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

// ------------------------------- school side ---------------------------------

/// A seeded school + a joined phone device (known session key). Returns the shared
/// server state, the device's session key, and the school id.
fn seed_school() -> (Arc<ServerState>, String, String, String) {
    let mut c = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut c).unwrap();
    seed::seed_demo_school(&mut c, now()).unwrap();
    let staff_id: String = c.query_row("SELECT id FROM staff WHERE role='principal' LIMIT 1", [], |r| r.get(0)).unwrap();
    let school_id: String = c.query_row("SELECT id FROM school LIMIT 1", [], |r| r.get(0)).unwrap();
    let session_key = STANDARD.encode([9u8; 32]);
    c.execute(
        "INSERT INTO device(id,staff_id,platform,name,token_hash,session_key,receipt_series,admission_series,last_seen_at,lease_expires_at,needs_rejoin,last_counter) \
         VALUES ('dev-phone',?1,'android','Phone','th',?2,'A9','A9','t','t',0,0)",
        params![staff_id, session_key],
    ).unwrap();
    (Arc::new(ServerState::new(c)), session_key, school_id, staff_id)
}

/// Spawn the real school tunnel client against the in-process relay.
fn start_tunnel(state: Arc<ServerState>, addr: SocketAddr, school_id: &str, epoch: i64) -> Arc<Notify> {
    let stop = Arc::new(Notify::new());
    let sid = school_id.to_string();
    let st = state;
    let stp = stop.clone();
    tokio::spawn(async move {
        run_tunnel(st, format!("ws://{addr}"), sid, "relay-secret".into(), epoch, stp).await;
    });
    stop
}

/// Wait until the relay has a tunnel registered for `school`, or panic on timeout.
async fn wait_for_tunnel(hub: &Arc<Hub>, school: &str) {
    for _ in 0..100 {
        if hub.tunnels.lock().unwrap().contains_key(school) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("tunnel never registered for {school}");
}

/// A student-insert op "recorded on the phone" (stands in for any queued change; the
/// per-table apply is covered by the P04 apply tests).
fn phone_op(op_id: &str, staff_id: &str, record_id: &str, epoch: i64) -> Op {
    Op {
        op_id: op_id.into(),
        hlc: format!("00000000000000000001{}", &op_id[op_id.len() - 1..]),
        device_id: "dev-phone".into(),
        staff_id: staff_id.into(),
        audience: "admin".into(),
        table: "student".into(),
        record_id: record_id.into(),
        kind: "insert".into(),
        payload: serde_json::json!({ "name": "Relay Kid", "status": "active" }),
        base_version: None,
        server_epoch: epoch,
    }
}

fn student_count(state: &Arc<ServerState>, id: &str) -> i64 {
    state.db.lock().unwrap().query_row("SELECT COUNT(*) FROM student WHERE id=?1", params![id], |r| r.get(0)).unwrap()
}

// --------------------------------- scenarios ---------------------------------

#[tokio::test]
async fn phone_via_relay_records_op_confirmed() {
    // DONE MEANS #1: a phone reachable ONLY via the relay records a change and gets
    // "Confirmed by school server".
    let hub = Hub::new();
    let addr = start_hub(hub.clone()).await;
    let (state, session_key, school_id, staff_id) = seed_school();
    let _stop = start_tunnel(state.clone(), addr, &school_id, 1);
    wait_for_tunnel(&hub, &school_id).await;

    let transport = RelayTransport::new(&format!("ws://{addr}"), &school_id, "dev-phone", &session_key, 1, 0).unwrap();
    let resp = transport.push(&[phone_op("op-relay-1", &staff_id, "stu-relay", 1)]).await.unwrap();
    assert_eq!(resp.results[0].status, OpStatus::Confirmed);
    assert_eq!(status_label(resp.results[0].status), "Confirmed by school server");
    assert_eq!(student_count(&state, "stu-relay"), 1, "applied on the school");
}

#[tokio::test]
async fn relay_offline_then_online_resumes() {
    // DONE MEANS #4: no tunnel → the device can't reach the school (falls back to the
    // queue); once the tunnel is up, the same push confirms.
    let hub = Hub::new();
    let addr = start_hub(hub.clone()).await;
    let (state, session_key, school_id, staff_id) = seed_school();
    let transport = RelayTransport::new(&format!("ws://{addr}"), &school_id, "dev-phone", &session_key, 1, 0).unwrap();

    // No tunnel yet → relay answers 503 SCHOOL_OFFLINE → Unreachable.
    let err = transport.push(&[phone_op("op-off", &staff_id, "stu-off", 1)]).await.unwrap_err();
    assert!(matches!(err, vidya_lib::sync::transport::TransportError::Unreachable));
    assert_eq!(student_count(&state, "stu-off"), 0, "nothing applied while offline");

    // Bring the school tunnel up → the retry confirms.
    let _stop = start_tunnel(state.clone(), addr, &school_id, 1);
    wait_for_tunnel(&hub, &school_id).await;
    let resp = transport.push(&[phone_op("op-off", &staff_id, "stu-off", 1)]).await.unwrap();
    assert_eq!(resp.results[0].status, OpStatus::Confirmed);
    assert_eq!(student_count(&state, "stu-off"), 1);
}

#[tokio::test]
async fn tampered_frame_in_transit_is_rejected() {
    // DONE MEANS #3: flip one byte of the sealed body in transit → AEAD failure at the
    // school (HTTP 400), nothing applied.
    let hub = Hub::new();
    let addr = start_hub(hub.clone()).await;
    let (state, session_key, school_id, _staff) = seed_school();
    let _stop = start_tunnel(state.clone(), addr, &school_id, 1);
    wait_for_tunnel(&hub, &school_id).await;

    // Build a valid sealed envelope, then corrupt one byte of the ciphertext.
    let sealer = RelaySealer::new(&session_key, "dev-phone", 1).unwrap();
    let mut env = sealer.seal_request("POST", "/v1/hello", 1, serde_json::json!({}));
    let mut raw = STANDARD.decode(&env.sealed_b64).unwrap();
    let n = raw.len() - 1;
    raw[n] ^= 0x01;
    env.sealed_b64 = STANDARD.encode(raw);

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/s/{school_id}/v1/sealed"))
        .json(&env)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 400, "tampered frame rejected as AEAD failure");
}

#[tokio::test]
async fn relay_drops_response_retry_applies_once() {
    // Step 6: the relay drops a response mid-request; the device retries with the same
    // op_id; the op is applied exactly once (idempotent by op_id).
    let hub = Hub::new();
    let addr = start_hub(hub.clone()).await;
    let (state, session_key, school_id, staff_id) = seed_school();
    let _stop = start_tunnel(state.clone(), addr, &school_id, 1);
    wait_for_tunnel(&hub, &school_id).await;

    let transport = RelayTransport::new(&format!("ws://{addr}"), &school_id, "dev-phone", &session_key, 1, 0).unwrap();
    hub.drop_next.store(true, Ordering::SeqCst);
    // First attempt: reaches the school (applied) but the response is dropped → error.
    assert!(transport.push(&[phone_op("op-dup", &staff_id, "stu-dup", 1)]).await.is_err());
    assert_eq!(student_count(&state, "stu-dup"), 1, "applied once on the first (dropped) attempt");
    // Retry with the SAME op_id (a fresh counter) → idempotent confirm, still one row.
    let resp = transport.push(&[phone_op("op-dup", &staff_id, "stu-dup", 1)]).await.unwrap();
    assert_eq!(resp.results[0].status, OpStatus::Confirmed);
    assert_eq!(student_count(&state, "stu-dup"), 1, "still applied exactly once");
}

#[tokio::test]
async fn slow_network_still_confirms() {
    // Step 6 (3G-ish): inject 400 ms of forward latency → still confirms; report the
    // round-trip so the handoff can quote a measured relay latency.
    let hub = Hub::new();
    let addr = start_hub(hub.clone()).await;
    let (state, session_key, school_id, staff_id) = seed_school();
    let _stop = start_tunnel(state.clone(), addr, &school_id, 1);
    wait_for_tunnel(&hub, &school_id).await;
    hub.delay_ms.store(400, Ordering::SeqCst);

    let transport = RelayTransport::new(&format!("ws://{addr}"), &school_id, "dev-phone", &session_key, 1, 0).unwrap();
    let started = Instant::now();
    let resp = transport.push(&[phone_op("op-slow", &staff_id, "stu-slow", 1)]).await.unwrap();
    let elapsed = started.elapsed();
    assert_eq!(resp.results[0].status, OpStatus::Confirmed);
    assert!(elapsed >= Duration::from_millis(400), "the injected latency applied");
    println!("relay round-trip under 400ms injected latency: {} ms", elapsed.as_millis());
}
