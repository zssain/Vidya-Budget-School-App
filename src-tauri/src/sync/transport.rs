//! Sync transport (prompts/P04 Step 6). The client engine talks to a `Transport`
//! so it is testable in-process (LoopbackTransport) while production uses TLS over
//! the LAN (HttpsTransport, pinned cert). Relay (Phase 5) / Drive (Phase 6) add
//! more transports behind the same trait.

use std::sync::{Arc, Mutex};

use crate::server::service::{self, DeviceAuth};
use crate::sync::protocol::*;

/// Why a transport attempt failed (drives route fallback + status copy).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// Could not reach the server (offline / wrong address) → try next route / queue.
    Unreachable,
    /// 401 — token revoked or unknown (§3).
    Revoked,
    /// 409 — server epoch is older than we know (§8.9).
    EpochOld,
    /// 426 — protocol mismatch.
    ProtocolMismatch,
    /// Any other server/parse error.
    Other(String),
}

/// The client↔server sync operations (all `/v1` except join needs no token).
/// Used with static dispatch by the engine (no `dyn`/spawn), so the async-fn Send
/// nuance does not apply.
#[allow(async_fn_in_trait)]
pub trait Transport {
    async fn hello(&self) -> Result<HelloResp, TransportError>;
    async fn push(&self, ops: &[Op]) -> Result<PushResp, TransportError>;
    async fn pull(&self, since: i64, limit: i64) -> Result<PullResp, TransportError>;
    async fn heartbeat(&self, req: &HeartbeatReq) -> Result<HeartbeatResp, TransportError>;
}

// ---------------------------------------------------------- loopback ---------

/// In-process transport that calls the service layer against a shared server DB.
/// Used by the sync harness / engine tests (no real network).
pub struct LoopbackTransport {
    pub server: Arc<Mutex<rusqlite::Connection>>,
    pub auth: DeviceAuth,
    /// Simulated network fault: when true, every call is Unreachable (offline).
    pub offline: bool,
    pub now: time::OffsetDateTime,
}

impl LoopbackTransport {
    pub fn new(server: Arc<Mutex<rusqlite::Connection>>, auth: DeviceAuth, now: time::OffsetDateTime) -> Self {
        Self { server, auth, offline: false, now }
    }
}

impl Transport for LoopbackTransport {
    async fn hello(&self) -> Result<HelloResp, TransportError> {
        if self.offline {
            return Err(TransportError::Unreachable);
        }
        let c = self.server.lock().map_err(|_| TransportError::Other("lock".into()))?;
        service::hello(&c, self.now).map_err(|e| TransportError::Other(e.to_string()))?.ok_or(TransportError::Unreachable)
    }
    async fn push(&self, ops: &[Op]) -> Result<PushResp, TransportError> {
        if self.offline {
            return Err(TransportError::Unreachable);
        }
        let mut c = self.server.lock().map_err(|_| TransportError::Other("lock".into()))?;
        service::push(&mut c, ops, self.now).map_err(|e| TransportError::Other(e.to_string()))
    }
    async fn pull(&self, since: i64, limit: i64) -> Result<PullResp, TransportError> {
        if self.offline {
            return Err(TransportError::Unreachable);
        }
        let c = self.server.lock().map_err(|_| TransportError::Other("lock".into()))?;
        service::pull(&c, &self.auth, since, limit, self.now).map_err(|e| TransportError::Other(e.to_string()))
    }
    async fn heartbeat(&self, req: &HeartbeatReq) -> Result<HeartbeatResp, TransportError> {
        if self.offline {
            return Err(TransportError::Unreachable);
        }
        let mut c = self.server.lock().map_err(|_| TransportError::Other("lock".into()))?;
        service::heartbeat(&mut c, &self.auth, req, self.now).map_err(|e| TransportError::Other(e.to_string()))
    }
}

// ------------------------------------------------------------ https ----------

/// Production transport: HTTPS to the school server over the LAN, TLS pinned to
/// the school's cert fingerprint (`server::cert::client_config`). Runtime path.
pub struct HttpsTransport {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl HttpsTransport {
    /// Build a client that trusts ONLY the pinned fingerprint.
    pub fn new(base_url: impl Into<String>, token: impl Into<String>, fingerprint: &str) -> Result<Self, String> {
        let tls = crate::server::cert::client_config(fingerprint)?;
        let client = reqwest::Client::builder()
            .use_preconfigured_tls(tls)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { client, base_url: base_url.into(), token: token.into() })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn map_status(status: u16) -> Option<TransportError> {
        match status {
            200 => None,
            401 => Some(TransportError::Revoked),
            409 => Some(TransportError::EpochOld),
            426 => Some(TransportError::ProtocolMismatch),
            s => Some(TransportError::Other(format!("http {s}"))),
        }
    }
}

impl Transport for HttpsTransport {
    async fn hello(&self) -> Result<HelloResp, TransportError> {
        let resp = self.client.get(self.url("/v1/hello")).send().await.map_err(|_| TransportError::Unreachable)?;
        if let Some(e) = Self::map_status(resp.status().as_u16()) {
            return Err(e);
        }
        resp.json().await.map_err(|e| TransportError::Other(e.to_string()))
    }
    async fn push(&self, ops: &[Op]) -> Result<PushResp, TransportError> {
        let resp = self
            .client
            .post(self.url("/v1/sync/push"))
            .bearer_auth(&self.token)
            .json(&PushReq { ops: ops.to_vec() })
            .send()
            .await
            .map_err(|_| TransportError::Unreachable)?;
        if let Some(e) = Self::map_status(resp.status().as_u16()) {
            return Err(e);
        }
        resp.json().await.map_err(|e| TransportError::Other(e.to_string()))
    }
    async fn pull(&self, since: i64, limit: i64) -> Result<PullResp, TransportError> {
        let resp = self
            .client
            .get(self.url(&format!("/v1/sync/pull?since={since}&limit={limit}")))
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|_| TransportError::Unreachable)?;
        if let Some(e) = Self::map_status(resp.status().as_u16()) {
            return Err(e);
        }
        resp.json().await.map_err(|e| TransportError::Other(e.to_string()))
    }
    async fn heartbeat(&self, req: &HeartbeatReq) -> Result<HeartbeatResp, TransportError> {
        let resp = self
            .client
            .post(self.url("/v1/device/heartbeat"))
            .bearer_auth(&self.token)
            .json(req)
            .send()
            .await
            .map_err(|_| TransportError::Unreachable)?;
        if let Some(e) = Self::map_status(resp.status().as_u16()) {
            return Err(e);
        }
        resp.json().await.map_err(|e| TransportError::Other(e.to_string()))
    }
}
