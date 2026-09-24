//! The production licence API (docs §10, prompts/P10 Step 4). JSON in/out, generic
//! error bodies (never reveal whether other schools exist), rate-limited per IP
//! and per code/licence.

use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::domain::{self, DomainError};
use crate::error::{json_err, AppError};
use crate::state::AppState;

// Rate-limit budgets.
const IP_MAX: u32 = 120;
const IP_WINDOW: Duration = Duration::from_secs(60);
const CODE_MAX: u32 = 8;
const CODE_WINDOW: Duration = Duration::from_secs(600);

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/activate", post(activate))
        .route("/v1/check", post(check))
        .route("/v1/transfer", post(transfer))
        .route("/healthz", get(healthz))
}

// --- request/response DTOs -------------------------------------------------

#[derive(Deserialize)]
struct ActivateReq {
    code: String,
    #[serde(default)]
    #[allow(dead_code)]
    school_name: String,
    machine_id: String,
    #[serde(default)]
    app_version: String,
    /// Optional (P10 [VERIFY] contract addition): base64 32-byte recovery-transfer
    /// verifier key the app derives from the school recovery key. Enables
    /// self-service transfer later; absent → admin-approved transfer only.
    #[serde(default)]
    recovery_verifier: Option<String>,
}

#[derive(Serialize)]
struct ActivateResp {
    licence: String,
    signature: String,
    relay_secret: String,
}

#[derive(Deserialize)]
struct CheckReq {
    licence_id: String,
    #[serde(default)]
    machine_id: String,
    #[serde(default)]
    app_version: String,
}

#[derive(Serialize)]
struct CheckResp {
    status: String,
}

#[derive(Deserialize)]
struct TransferReq {
    licence_id: String,
    #[serde(default)]
    recovery_proof: String,
    new_machine_id: String,
    #[serde(default)]
    timestamp: String,
}

// --- helpers ---------------------------------------------------------------

/// Client IP for rate limiting: the reverse proxy's `X-Forwarded-For` (first hop)
/// or `X-Real-IP`, else the socket peer.
pub(crate) fn client_ip(headers: &HeaderMap, peer: SocketAddr) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()).map(|s| s.to_string()))
        .unwrap_or_else(|| peer.ip().to_string())
}

fn allow(state: &AppState, key: &str, max: u32, window: Duration) -> bool {
    state.rl.lock().unwrap().allow(key, max, window)
}

fn map_err(e: DomainError) -> Response {
    match e {
        DomainError::NotFound => json_err(StatusCode::NOT_FOUND, "CODE_NOT_FOUND"),
        DomainError::AlreadyUsed => json_err(StatusCode::CONFLICT, "CODE_ALREADY_USED"),
        DomainError::Revoked => json_err(StatusCode::FORBIDDEN, "REVOKED"),
        DomainError::ProofInvalid => json_err(StatusCode::FORBIDDEN, "PROOF_INVALID"),
        DomainError::TransferUnavailable => json_err(StatusCode::FORBIDDEN, "TRANSFER_UNAVAILABLE"),
    }
}

fn too_many() -> Response {
    json_err(StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED")
}

// --- handlers --------------------------------------------------------------

async fn activate(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ActivateReq>,
) -> Result<Response, AppError> {
    let ip = client_ip(&headers, peer);
    if !allow(&state, &format!("ip:{ip}"), IP_MAX, IP_WINDOW) {
        return Ok(too_many());
    }
    // Per-code limit keyed on the code hash (does not store the code).
    let code_key = format!("code:{}", crate::crypto::hash_code(&req.code));
    if !allow(&state, &code_key, CODE_MAX, CODE_WINDOW) {
        return Ok(too_many());
    }

    let mut conn = state.db.lock().unwrap();
    let result = domain::activate(
        &mut conn,
        &state.cfg,
        &state.signing,
        &req.code,
        &req.machine_id,
        &req.app_version,
        req.recovery_verifier.as_deref(),
    )?;
    drop(conn);

    Ok(match result {
        Ok(issued) => (
            StatusCode::OK,
            Json(ActivateResp {
                licence: issued.licence_b64,
                signature: issued.signature_b64,
                relay_secret: issued.relay_secret,
            }),
        )
            .into_response(),
        Err(e) => map_err(e),
    })
}

async fn check(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<CheckReq>,
) -> Result<Response, AppError> {
    let ip = client_ip(&headers, peer);
    if !allow(&state, &format!("ip:{ip}"), IP_MAX, IP_WINDOW) {
        return Ok(too_many());
    }
    let conn = state.db.lock().unwrap();
    let status = domain::check(&conn, &req.licence_id, &req.machine_id, &req.app_version)?;
    drop(conn);
    Ok(match status {
        Some(status) => (StatusCode::OK, Json(CheckResp { status })).into_response(),
        None => json_err(StatusCode::NOT_FOUND, "CODE_NOT_FOUND"),
    })
}

async fn transfer(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<TransferReq>,
) -> Result<Response, AppError> {
    let ip = client_ip(&headers, peer);
    if !allow(&state, &format!("ip:{ip}"), IP_MAX, IP_WINDOW) {
        return Ok(too_many());
    }
    if !allow(&state, &format!("lic:{}", req.licence_id), CODE_MAX, CODE_WINDOW) {
        return Ok(too_many());
    }
    let now_unix = time::OffsetDateTime::now_utc().unix_timestamp();
    let mut conn = state.db.lock().unwrap();
    let result = domain::transfer(
        &mut conn,
        &state.cfg,
        &state.signing,
        &req.licence_id,
        &req.new_machine_id,
        &req.recovery_proof,
        &req.timestamp,
        now_unix,
    )?;
    drop(conn);
    Ok(match result {
        Ok(issued) => (
            StatusCode::OK,
            Json(ActivateResp {
                licence: issued.licence_b64,
                signature: issued.signature_b64,
                relay_secret: issued.relay_secret,
            }),
        )
            .into_response(),
        Err(e) => map_err(e),
    })
}

/// Liveness + DB reachability (Step 7). No secrets, no counts that leak scale.
async fn healthz(State(state): State<AppState>) -> Response {
    let ok = {
        let conn = state.db.lock().unwrap();
        conn.query_row("SELECT 1", [], |r| r.get::<_, i64>(0)).is_ok()
    };
    if ok {
        (StatusCode::OK, "ok").into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "unavailable").into_response()
    }
}
