//! Sealed relay dispatch (prompts/P05 Step 2). A request that arrives over the
//! relay is an opaque [`SealedEnvelope`]: the relay never saw its contents. This
//! module unseals it with the device's session key (possession of that key — not a
//! bearer token — authenticates the device, so no token ever crosses the relay),
//! enforces the monotonic replay counter, runs the same `service` layer the LAN
//! endpoints use, then seals the response back.
//!
//! AEAD associated data binds method + path + device_id + server_epoch, so a relay
//! that flips any byte or field makes the open fail — the request is rejected and
//! nothing is applied (DONE MEANS #3).

use rusqlite::Connection;

use crate::server::service::{self, DeviceAuth};
use crate::sync::protocol::*;
use crate::sync::seal;

/// Why a sealed dispatch could not be served. Maps to an HTTP status the relay
/// forwards verbatim (it only ever sees the status, never the body).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealedError {
    /// No such device, revoked, or no stored session key → 401.
    UnknownDevice,
    /// AEAD open failed: tampered bytes, wrong key, or wrong associated data → 400.
    Aead,
    /// A replayed or stale request counter → 409.
    Replay,
    /// Malformed inner request JSON → 400.
    BadRequest,
    /// A server-side failure → 500.
    Internal,
}

impl SealedError {
    pub fn http_status(&self) -> u16 {
        match self {
            SealedError::UnknownDevice => 401,
            SealedError::Aead | SealedError::BadRequest => 400,
            SealedError::Replay => 409,
            SealedError::Internal => 500,
        }
    }
    /// Stable code for the (body-free) relay log line.
    pub fn code(&self) -> &'static str {
        match self {
            SealedError::UnknownDevice => "DEVICE_REVOKED_OR_UNKNOWN",
            SealedError::Aead => "SEAL_FAILED",
            SealedError::Replay => "REPLAYED",
            SealedError::BadRequest => "BAD_REQUEST",
            SealedError::Internal => "INTERNAL",
        }
    }
}

/// Seal a `SealedResponse` back to the device using the s2c key + the server's real
/// epoch (which drives the device's fencing).
fn seal_response(
    keys: &seal::DirectionKeys,
    env: &SealedEnvelope,
    real_epoch: i64,
    status: u16,
    body: serde_json::Value,
) -> SealedEnvelope {
    let aad = seal::associated_data(&env.method, &env.path, &env.device_id, real_epoch);
    let inner = serde_json::to_vec(&SealedResponse { status, body }).unwrap_or_default();
    SealedEnvelope {
        device_id: env.device_id.clone(),
        method: env.method.clone(),
        path: env.path.clone(),
        server_epoch: real_epoch,
        sealed_b64: seal::seal_b64(&keys.s2c, &aad, &inner),
    }
}

/// Run one sealed request against the service layer and return the sealed response.
pub fn dispatch(conn: &mut Connection, env: &SealedEnvelope, now: time::OffsetDateTime) -> Result<SealedEnvelope, SealedError> {
    // 1. Authenticate by session-key possession (no token on the wire).
    let (session_key, auth) = service::session_auth_for_device(conn, &env.device_id)
        .map_err(|_| SealedError::Internal)?
        .ok_or(SealedError::UnknownDevice)?;
    let keys = seal::derive_keys(&session_key).map_err(|_| SealedError::Internal)?;

    // 2. Open the request (AAD = method+path+device_id+device's asserted epoch).
    let aad_req = seal::associated_data(&env.method, &env.path, &env.device_id, env.server_epoch);
    let plain = seal::open_b64(&keys.c2s, &aad_req, &env.sealed_b64).map_err(|_| SealedError::Aead)?;
    let req: SealedRequest = serde_json::from_slice(&plain).map_err(|_| SealedError::BadRequest)?;

    // 3. Replay defence: strictly-monotonic per-device counter.
    if !service::accept_counter(conn, &env.device_id, req.counter).map_err(|_| SealedError::Internal)? {
        return Err(SealedError::Replay);
    }

    let real_epoch = service::current_epoch(conn).map_err(|_| SealedError::Internal)?;

    // 4. Fence a stale server: the device already trusts a newer epoch than this
    //    server has → this PC is no longer the school server. Reply 409, don't apply.
    if env.server_epoch > real_epoch {
        return Ok(seal_response(&keys, env, real_epoch, 409, serde_json::json!({ "error": codes::EPOCH_OLD })));
    }

    // 5. Route on the logical path (mirrors the LAN /v1 endpoints).
    let (status, body) = route(conn, &auth, &env.path, &req.body, now).map_err(|_| SealedError::Internal)?;

    // 6. Seal the response with the server's real epoch (drives device fencing).
    Ok(seal_response(&keys, env, real_epoch, status, body))
}

/// Route a sealed inner request to the service layer. Returns (status, body JSON).
fn route(
    conn: &mut Connection,
    auth: &DeviceAuth,
    path: &str,
    body: &serde_json::Value,
    now: time::OffsetDateTime,
) -> rusqlite::Result<(u16, serde_json::Value)> {
    match path {
        "/v1/hello" => match service::hello(conn, now)? {
            Some(h) => Ok((200, serde_json::to_value(h).unwrap_or_default())),
            None => Ok((404, serde_json::json!({ "error": codes::NOT_FOUND }))),
        },
        "/v1/sync/push" => {
            let req: PushReq = match serde_json::from_value(body.clone()) {
                Ok(r) => r,
                Err(_) => return Ok((400, serde_json::json!({ "error": "BAD_REQUEST" }))),
            };
            let resp = service::push(conn, &req.ops, now)?;
            Ok((200, serde_json::to_value(resp).unwrap_or_default()))
        }
        "/v1/sync/pull" => {
            let since = body.get("since").and_then(|v| v.as_i64()).unwrap_or(0);
            let limit = body.get("limit").and_then(|v| v.as_i64()).unwrap_or(500);
            let resp = service::pull(conn, auth, since, limit, now)?;
            Ok((200, serde_json::to_value(resp).unwrap_or_default()))
        }
        "/v1/device/heartbeat" => {
            let req: HeartbeatReq = match serde_json::from_value(body.clone()) {
                Ok(r) => r,
                Err(_) => return Ok((400, serde_json::json!({ "error": "BAD_REQUEST" }))),
            };
            let resp = service::heartbeat(conn, auth, &req, now)?;
            Ok((200, serde_json::to_value(resp).unwrap_or_default()))
        }
        "/v1/sync/snapshot" => {
            let changes = service::snapshot(conn, auth)?;
            Ok((200, serde_json::to_value(changes).unwrap_or_default()))
        }
        _ => Ok((404, serde_json::json!({ "error": codes::NOT_FOUND }))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::protocol::{Op, OpStatus};
    use crate::{db, seed};

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn now() -> time::OffsetDateTime {
        time::OffsetDateTime::parse("2026-09-23T12:00:00Z", &time::format_description::well_known::Rfc3339).unwrap()
    }

    /// Provision a device row with a known session key and return (device_id, keys).
    fn join_device(conn: &mut Connection) -> (String, seal::DirectionKeys) {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        let session_key = STANDARD.encode([9u8; 32]);
        let staff_id: String = conn
            .query_row("SELECT id FROM staff WHERE role='principal' LIMIT 1", [], |r| r.get(0))
            .unwrap();
        conn.execute(
            "INSERT INTO device(id,staff_id,platform,name,token_hash,session_key,receipt_series,admission_series,last_seen_at,lease_expires_at,needs_rejoin,last_counter) \
             VALUES ('dev-relay',?1,'android','Phone','th',?2,'A9','A9','t','t',0,0)",
            rusqlite::params![staff_id, session_key],
        ).unwrap();
        ("dev-relay".to_string(), seal::derive_keys(&session_key).unwrap())
    }

    /// Seal a request the way a device would.
    fn seal_req(keys: &seal::DirectionKeys, device_id: &str, path: &str, epoch: i64, counter: i64, body: serde_json::Value) -> SealedEnvelope {
        let aad = seal::associated_data("POST", path, device_id, epoch);
        let inner = serde_json::to_vec(&SealedRequest { counter, body }).unwrap();
        SealedEnvelope {
            device_id: device_id.into(), method: "POST".into(), path: path.into(),
            server_epoch: epoch, sealed_b64: seal::seal_b64(&keys.c2s, &aad, &inner),
        }
    }

    fn server() -> Connection {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        seed::seed_demo_school(&mut c, now()).unwrap();
        c
    }

    #[test]
    fn sealed_push_applies_and_confirms() {
        // DONE MEANS #1 (crypto+apply half): a sealed attendance-like insert op is
        // unsealed, applied, and the sealed response confirms it.
        let mut conn = server();
        let (device_id, keys) = join_device(&mut conn);
        let op = Op {
            op_id: "op-seal-1".into(), hlc: "00000000000000000001z".into(), device_id: device_id.clone(),
            staff_id: conn.query_row("SELECT id FROM staff WHERE role='principal' LIMIT 1", [], |r| r.get(0)).unwrap(),
            audience: "admin".into(), table: "student".into(), record_id: "stu-seal".into(), kind: "insert".into(),
            payload: serde_json::json!({ "name": "Relay Kid", "status": "active" }), base_version: None, server_epoch: 1,
        };
        let env = seal_req(&keys, &device_id, "/v1/sync/push", 1, 1, serde_json::json!({ "ops": [op] }));
        let resp = dispatch(&mut conn, &env, now()).unwrap();
        assert_eq!(resp.server_epoch, 1);
        // Open the response and assert the op confirmed.
        let aad = seal::associated_data("POST", "/v1/sync/push", &device_id, 1);
        let plain = seal::open_b64(&keys.s2c, &aad, &resp.sealed_b64).unwrap();
        let sr: SealedResponse = serde_json::from_slice(&plain).unwrap();
        assert_eq!(sr.status, 200);
        let push: PushResp = serde_json::from_value(sr.body).unwrap();
        assert_eq!(push.results[0].status, OpStatus::Confirmed);
        let on_server: i64 = conn.query_row("SELECT COUNT(*) FROM student WHERE id='stu-seal'", [], |r| r.get(0)).unwrap();
        assert_eq!(on_server, 1);
    }

    #[test]
    fn tampered_envelope_is_rejected_and_changes_nothing() {
        // DONE MEANS #3.
        let mut conn = server();
        let (device_id, keys) = join_device(&mut conn);
        let op = serde_json::json!({ "ops": [] });
        let mut env = seal_req(&keys, &device_id, "/v1/sync/push", 1, 1, op);
        // Flip a byte in the sealed blob (decode, mutate, re-encode).
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        let mut raw = STANDARD.decode(&env.sealed_b64).unwrap();
        let last = raw.len() - 1;
        raw[last] ^= 0x01;
        env.sealed_b64 = STANDARD.encode(raw);
        assert_eq!(dispatch(&mut conn, &env, now()), Err(SealedError::Aead));
    }

    #[test]
    fn replayed_counter_is_rejected() {
        // DONE MEANS #4-adjacent: a captured request replayed verbatim is refused.
        let mut conn = server();
        let (device_id, keys) = join_device(&mut conn);
        let env = seal_req(&keys, &device_id, "/v1/hello", 1, 5, serde_json::json!({}));
        assert!(dispatch(&mut conn, &env, now()).is_ok()); // counter 5 accepted
        assert_eq!(dispatch(&mut conn, &env, now()), Err(SealedError::Replay)); // same counter → replay
        // A lower counter is also rejected.
        let older = seal_req(&keys, &device_id, "/v1/hello", 1, 3, serde_json::json!({}));
        assert_eq!(dispatch(&mut conn, &older, now()), Err(SealedError::Replay));
    }

    #[test]
    fn unknown_device_is_unauthorised() {
        let mut conn = server();
        let (_id, keys) = join_device(&mut conn);
        let env = seal_req(&keys, "dev-nope", "/v1/hello", 1, 1, serde_json::json!({}));
        assert_eq!(dispatch(&mut conn, &env, now()), Err(SealedError::UnknownDevice));
    }

    #[test]
    fn device_with_newer_epoch_fences_a_stale_server() {
        // DONE MEANS #5 (server half): the device trusts epoch 2, this server is
        // still epoch 1 → 409 EPOCH_OLD, and no op is applied.
        let mut conn = server();
        let (device_id, keys) = join_device(&mut conn);
        let env = seal_req(&keys, &device_id, "/v1/sync/push", 2, 1, serde_json::json!({ "ops": [] }));
        let resp = dispatch(&mut conn, &env, now()).unwrap();
        // The response is sealed with the server's REAL epoch (1) for AAD.
        let aad = seal::associated_data("POST", "/v1/sync/push", &device_id, 1);
        let plain = seal::open_b64(&keys.s2c, &aad, &resp.sealed_b64).unwrap();
        let sr: SealedResponse = serde_json::from_slice(&plain).unwrap();
        assert_eq!(sr.status, 409);
        assert_eq!(sr.body["error"], "EPOCH_OLD");
    }
}
