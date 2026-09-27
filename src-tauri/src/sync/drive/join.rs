//! Join-without-LAN: the school PC answers a PWA's Drive join request (Phase 19,
//! Step 5, §18). A browser can't reach the PC's self-signed LAN server, so the PWA
//! drops a sealed "join request" in `exchange/joins/`; on its next Drive check the
//! school PC runs the NORMAL `/join` (so a web device is provisioned identically to
//! an Android one) and writes back a **sealed** join response — the device token,
//! series, audience keys and session key, sealed with a key derived from the
//! single-use invite code so only the joining device can open it.
//!
//! This module is the pure request→response LOGIC (reused by the school PC's Drive
//! loop, which — like the rest of the live Drive client — is wired when the P12
//! spike is completed on the owner's machine). It is unit-tested here against a real
//! seeded DB + invite.

use crate::server::service::join;
use crate::sync::protocol::JoinReq;
use crate::sync::seal;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// The join request a PWA writes to `exchange/joins/<request_id>.vjoin`. The invite
/// code is present (in the school's own sync account) so the PC can authenticate and
/// derive the response seal key; it is single-use and short-lived.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveJoinRequest {
    pub request_id: String,
    pub invite_code: String,
    pub device_name: String,
    pub platform: String,
}

/// The response file name for a request. Written next to the request in `joins/`.
pub fn response_name(request_id: &str) -> String {
    format!("{request_id}.resp.vjoin")
}

/// Run `/join` for a Drive join request and return the JoinResp sealed to the joining
/// device (key = `join_key(invite_code)`, AAD bound to the request id). The caller
/// writes these bytes to `exchange/joins/<request_id>.resp.vjoin`.
pub fn answer_join_request(
    conn: &mut Connection,
    req_json: &[u8],
    now: time::OffsetDateTime,
) -> Result<Vec<u8>, String> {
    let req: DriveJoinRequest =
        serde_json::from_slice(req_json).map_err(|e| format!("bad join request: {e}"))?;
    // The PWA is a normal staff client — provision it via the SAME /join as Android
    // (single-use invite, device token, per-device series, audience keys, session key).
    let resp = join(
        conn,
        &JoinReq {
            invite_code: req.invite_code.clone(),
            device_name: req.device_name.clone(),
            platform: req.platform.clone(),
            google_email: None,
        },
        now,
    )
    .map_err(|e| format!("{e:?}"))?;
    let json = serde_json::to_vec(&resp).map_err(|e| e.to_string())?;
    let key = vidya_core::seal::join_key(&req.invite_code);
    let aad = vidya_core::seal::join_aad(&req.request_id);
    Ok(seal::seal(&key, &aad, &json))
}
