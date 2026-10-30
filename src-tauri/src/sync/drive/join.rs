//! Join-without-LAN: the school PC answers a PWA's Drive join request (Phase 19,
//! Step 5, §18). A browser can't reach the PC's self-signed LAN server, so the PWA
//! drops a join request in `exchange/joins/`; on its next Drive check the school PC
//! runs the same invite consume + provisioning as a LAN join and writes back a
//! **sealed** join response (device token, series, audience keys, session key).
//!
//! Protocol v2 (`vidya_core::seal`): the request holds only the invite id, a random
//! request id, device name/platform and an HMAC **proof** of the invite's 256-bit web
//! secret. The secret (in the invite link's URL fragment) and the short invite code
//! are never uploaded, and the response is sealed under a key derived from the
//! secret and the request id — so anyone who can merely read the joins folder can
//! neither forge a join nor open a response. v1 requests (plaintext invite code) are
//! refused.

use crate::server::service::join_web;
use crate::sync::seal;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// The join request a PWA writes to `exchange/joins/<request_id>.vjoin`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveJoinRequest {
    /// Protocol version — must be 2.
    pub v: u32,
    pub invite_id: String,
    pub request_id: String,
    pub device_name: String,
    pub platform: String,
    /// base64url(HMAC proof of the invite web secret over the transcript).
    pub proof: String,
}

/// The response file name for a request. Written next to the request in `joins/`.
pub fn response_name(request_id: &str) -> String {
    format!("{request_id}.resp.vjoin")
}

/// Verify a Drive join request, consume its invite, and return the JoinResp sealed to
/// the joining device (key = `join_v2_response_key(secret, request_id)`, AAD bound to
/// invite + request + school). The caller writes the bytes to
/// `exchange/joins/<request_id>.resp.vjoin`. Any failure (bad proof, used/expired
/// invite, unsupported version) is an Err and nothing is written.
pub fn answer_join_request(
    conn: &mut Connection,
    req_json: &[u8],
    now: time::OffsetDateTime,
) -> Result<Vec<u8>, String> {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    let req: DriveJoinRequest =
        serde_json::from_slice(req_json).map_err(|e| format!("bad join request: {e}"))?;
    if req.v != 2 {
        return Err("unsupported join request version".into());
    }
    let proof = URL_SAFE_NO_PAD.decode(req.proof.as_bytes()).map_err(|_| "bad proof encoding".to_string())?;
    let (resp, secret) = join_web(conn, &req.invite_id, &req.request_id, &req.device_name, &req.platform, &proof, now)
        .map_err(|e| format!("{e:?}"))?;
    let json = serde_json::to_vec(&resp).map_err(|e| e.to_string())?;
    let key = vidya_core::seal::join_v2_response_key(&secret, &req.request_id);
    let aad = vidya_core::seal::join_v2_aad(&req.invite_id, &req.request_id, &resp.school.id);
    Ok(seal::seal(&key, &aad, &json))
}

