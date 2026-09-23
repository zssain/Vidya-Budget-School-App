//! Client relay transport (prompts/P05 Step 2/3). The mirror of
//! `server::sealed`: the device seals each logical `/v1` request into a
//! [`SealedEnvelope`], POSTs it to `RELAY_URL/s/<school_id>/v1/sealed`, and opens
//! the sealed response. The relay only ever forwards the opaque envelope — it
//! cannot read or alter the request or response (rule §6).
//!
//! `RelaySealer` is the pure crypto half (unit-tested against the server dispatch
//! in-process). `RelayTransport` adds the reqwest hop and satisfies the same
//! [`Transport`] trait the LAN route uses, so the engine treats routes uniformly.

use std::sync::Mutex;

use crate::sync::protocol::*;
use crate::sync::seal::{self, DirectionKeys};
use crate::sync::transport::{Transport, TransportError};

/// A relay reachability / crypto failure at the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayError {
    /// Could not reach the relay, or the relay has no tunnel for this school
    /// (`503 SCHOOL_OFFLINE`) → fall back to the next route / queue.
    Unreachable,
    /// 401 — the device is unknown/revoked at the school.
    Revoked,
    /// The server (or relay) fenced us with a newer epoch (§8.9).
    EpochOld,
    /// AEAD open failed on the response (tampered / wrong key).
    Seal,
    /// Malformed response.
    Parse,
    /// Any other relay status.
    Other(u16),
}

impl From<RelayError> for TransportError {
    fn from(e: RelayError) -> Self {
        match e {
            RelayError::Unreachable => TransportError::Unreachable,
            RelayError::Revoked => TransportError::Revoked,
            RelayError::EpochOld => TransportError::EpochOld,
            RelayError::Seal => TransportError::Other("seal failed".into()),
            RelayError::Parse => TransportError::Other("bad relay response".into()),
            RelayError::Other(s) => TransportError::Other(format!("relay http {s}")),
        }
    }
}

/// The pure sealing half of the relay transport. Holds the per-direction keys and
/// the device id; seals requests and opens responses, applying epoch fencing.
pub struct RelaySealer {
    keys: DirectionKeys,
    device_id: String,
    /// The epoch this device currently trusts (asserted in each request; a lower
    /// server epoch in a response is refused — §8.9).
    known_epoch: i64,
}

impl RelaySealer {
    pub fn new(session_key_b64: &str, device_id: impl Into<String>, known_epoch: i64) -> Result<Self, RelayError> {
        let keys = seal::derive_keys(session_key_b64).map_err(|_| RelayError::Seal)?;
        Ok(Self { keys, device_id: device_id.into(), known_epoch })
    }

    /// Seal a logical request into an envelope the relay forwards verbatim.
    pub fn seal_request(&self, method: &str, path: &str, counter: i64, body: serde_json::Value) -> SealedEnvelope {
        let aad = seal::associated_data(method, path, &self.device_id, self.known_epoch);
        let inner = serde_json::to_vec(&SealedRequest { counter, body }).unwrap_or_default();
        SealedEnvelope {
            device_id: self.device_id.clone(),
            method: method.into(),
            path: path.into(),
            server_epoch: self.known_epoch,
            sealed_b64: seal::seal_b64(&self.keys.c2s, &aad, &inner),
        }
    }

    /// Open a sealed response envelope, applying fencing: a server epoch LOWER than
    /// the one we trust is refused (`EpochOld`). Returns the inner response.
    pub fn open_response(&self, env: &SealedEnvelope) -> Result<SealedResponse, RelayError> {
        // Fencing (§8.9): never trust a server older than the highest epoch we know.
        if env.server_epoch < self.known_epoch {
            return Err(RelayError::EpochOld);
        }
        let aad = seal::associated_data(&env.method, &env.path, &env.device_id, env.server_epoch);
        let plain = seal::open_b64(&self.keys.s2c, &aad, &env.sealed_b64).map_err(|_| RelayError::Seal)?;
        let resp: SealedResponse = serde_json::from_slice(&plain).map_err(|_| RelayError::Parse)?;
        // The server may itself signal EPOCH_OLD inside the sealed body (409).
        if resp.status == 409 && resp.body.get("error").and_then(|e| e.as_str()) == Some(codes::EPOCH_OLD) {
            return Err(RelayError::EpochOld);
        }
        Ok(resp)
    }

    /// The server epoch reported by a response (for the engine to advance its
    /// stored high-water mark after a successful, non-fenced exchange).
    pub fn response_epoch(env: &SealedEnvelope) -> i64 {
        env.server_epoch
    }
}

/// Convert a relay URL (`ws://` / `wss://` for the school tunnel) into the HTTPS
/// base the device uses. Non-ws schemes pass through unchanged.
pub fn device_base(relay_url: &str) -> String {
    if let Some(rest) = relay_url.strip_prefix("wss://") {
        format!("https://{rest}")
    } else if let Some(rest) = relay_url.strip_prefix("ws://") {
        format!("http://{rest}")
    } else {
        relay_url.to_string()
    }
}

/// Production client relay transport: seals over the reqwest hop to the relay.
pub struct RelayTransport {
    client: reqwest::Client,
    /// Precomputed `{device_base}/s/{school_id}/v1/sealed`.
    sealed_url: String,
    sealer: RelaySealer,
    /// Strictly-monotonic per-device request counter (replay defence). Seeded from
    /// the engine's persisted value so it never repeats across restarts.
    counter: Mutex<i64>,
}

impl RelayTransport {
    pub fn new(
        relay_url: &str,
        school_id: &str,
        device_id: &str,
        session_key_b64: &str,
        known_epoch: i64,
        start_counter: i64,
    ) -> Result<Self, RelayError> {
        let base = device_base(relay_url);
        let sealed_url = format!("{}/s/{}/v1/sealed", base.trim_end_matches('/'), school_id);
        Ok(Self {
            client: reqwest::Client::new(),
            sealed_url,
            sealer: RelaySealer::new(session_key_b64, device_id, known_epoch)?,
            counter: Mutex::new(start_counter),
        })
    }

    /// Build a transport with a caller-supplied client + explicit sealed URL (used
    /// by the in-process relay harness, which binds a random localhost port).
    pub fn with_url(client: reqwest::Client, sealed_url: String, sealer: RelaySealer, start_counter: i64) -> Self {
        Self { client, sealed_url, sealer, counter: Mutex::new(start_counter) }
    }

    /// The next request counter (advances the local high-water mark).
    pub fn next_counter(&self) -> i64 {
        let mut g = self.counter.lock().unwrap();
        *g += 1;
        *g
    }

    /// Seal `body`, POST it, and open the sealed response — returning the inner
    /// response body when the inner status is 200.
    async fn call(&self, path: &str, body: serde_json::Value) -> Result<serde_json::Value, RelayError> {
        let counter = self.next_counter();
        let env = self.sealer.seal_request("POST", path, counter, body);
        let resp = self
            .client
            .post(&self.sealed_url)
            .json(&env)
            .send()
            .await
            .map_err(|_| RelayError::Unreachable)?;
        match resp.status().as_u16() {
            200 => {}
            401 => return Err(RelayError::Revoked),
            409 => return Err(RelayError::EpochOld),
            // The relay reports `503 SCHOOL_OFFLINE` when no tunnel is connected.
            503 => return Err(RelayError::Unreachable),
            426 => return Err(RelayError::Other(426)),
            s => return Err(RelayError::Other(s)),
        }
        let out_env: SealedEnvelope = resp.json().await.map_err(|_| RelayError::Parse)?;
        let sealed = self.sealer.open_response(&out_env)?;
        if sealed.status != 200 {
            return Err(RelayError::Other(sealed.status));
        }
        Ok(sealed.body)
    }
}

impl Transport for RelayTransport {
    async fn hello(&self) -> Result<HelloResp, TransportError> {
        let body = self.call("/v1/hello", serde_json::json!({})).await?;
        serde_json::from_value(body).map_err(|e| TransportError::Other(e.to_string()))
    }
    async fn push(&self, ops: &[Op]) -> Result<PushResp, TransportError> {
        let body = self.call("/v1/sync/push", serde_json::json!({ "ops": ops })).await?;
        serde_json::from_value(body).map_err(|e| TransportError::Other(e.to_string()))
    }
    async fn pull(&self, since: i64, limit: i64) -> Result<PullResp, TransportError> {
        let body = self.call("/v1/sync/pull", serde_json::json!({ "since": since, "limit": limit })).await?;
        serde_json::from_value(body).map_err(|e| TransportError::Other(e.to_string()))
    }
    async fn heartbeat(&self, req: &HeartbeatReq) -> Result<HeartbeatResp, TransportError> {
        let body = self.call("/v1/device/heartbeat", serde_json::to_value(req).unwrap_or_default()).await?;
        serde_json::from_value(body).map_err(|e| TransportError::Other(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::sealed;
    use crate::{db, seed};
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use rusqlite::Connection;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn now() -> time::OffsetDateTime {
        time::OffsetDateTime::parse("2026-09-23T12:00:00Z", &time::format_description::well_known::Rfc3339).unwrap()
    }

    fn server_with_device(session_key: &str) -> (Connection, String) {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        seed::seed_demo_school(&mut c, now()).unwrap();
        let staff_id: String = c.query_row("SELECT id FROM staff WHERE role='principal' LIMIT 1", [], |r| r.get(0)).unwrap();
        c.execute(
            "INSERT INTO device(id,staff_id,platform,name,token_hash,session_key,receipt_series,admission_series,last_seen_at,lease_expires_at,needs_rejoin,last_counter) \
             VALUES ('dev-relay',?1,'android','Phone','th',?2,'A9','A9','t','t',0,0)",
            rusqlite::params![staff_id, session_key],
        ).unwrap();
        (c, "dev-relay".into())
    }

    #[test]
    fn client_seal_matches_server_open_and_back() {
        // The client seals a request; the server opens+serves it; the client opens
        // the server's sealed response — proving both halves agree (no socket).
        let session_key = STANDARD.encode([9u8; 32]);
        let (mut conn, device_id) = server_with_device(&session_key);
        let sealer = RelaySealer::new(&session_key, &device_id, 1).unwrap();

        let env = sealer.seal_request("POST", "/v1/hello", 1, serde_json::json!({}));
        let resp_env = sealed::dispatch(&mut conn, &env, now()).unwrap();
        let resp = sealer.open_response(&resp_env).unwrap();
        assert_eq!(resp.status, 200);
        let hello: HelloResp = serde_json::from_value(resp.body).unwrap();
        assert_eq!(hello.server_epoch, 1);
    }

    #[test]
    fn client_refuses_a_lower_epoch_response() {
        // Fencing (§8.9): a device that already trusts epoch 2 refuses an epoch-1
        // server's (validly sealed) response.
        let session_key = STANDARD.encode([9u8; 32]);
        let sealer_e1 = RelaySealer::new(&session_key, "dev-relay", 1).unwrap();
        let (mut conn, _d) = server_with_device(&session_key);
        let env = sealer_e1.seal_request("POST", "/v1/hello", 1, serde_json::json!({}));
        let resp_env = sealed::dispatch(&mut conn, &env, now()).unwrap(); // real epoch 1
        // Now the same device knows epoch 2 → it must refuse the epoch-1 response.
        let sealer_e2 = RelaySealer::new(&session_key, "dev-relay", 2).unwrap();
        assert_eq!(sealer_e2.open_response(&resp_env), Err(RelayError::EpochOld));
    }

    #[test]
    fn device_base_maps_ws_schemes() {
        assert_eq!(device_base("ws://127.0.0.1:8788"), "http://127.0.0.1:8788");
        assert_eq!(device_base("wss://relay.example"), "https://relay.example");
        assert_eq!(device_base("https://relay.example"), "https://relay.example");
    }
}
