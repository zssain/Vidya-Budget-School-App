//! App-side licence: machine id, activation, signature verification, re-check
//! (docs/00-SYSTEM-CONTEXT.md §10, prompts/P03 Step 3).
//!
//! Business model is one-time / perpetual (no expiry, no grace). The app verifies
//! the ed25519 signature offline with the public key from build config
//! (`crate::config`) via `vidya_core::licence::verify`, then re-checks the service
//! every 30 days when online — only an explicit `revoked`/`moved` changes status.

use std::path::PathBuf;

use serde::Serialize;
use time::OffsetDateTime;
use vidya_core::licence::{CheckResult, Licence};

pub mod machine;

/// How often to re-check the licence with the service when online (§10).
pub const RECHECK_DAYS: i64 = 30;

/// A user-facing licence failure. Commands turn this into `{code, message_key, vars}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenceError {
    /// Service unreachable / 503 (offline). Activation needs internet once.
    Unreachable,
    /// 404: the code is unknown to the service.
    CodeNotFound,
    /// 409: the code was already used by another school/machine.
    CodeAlreadyUsed,
    /// Signature/verify failed, malformed response, or machine-id mismatch.
    Invalid,
}

impl LicenceError {
    /// Stable machine code (mirrors the error contract in §10).
    pub fn code(&self) -> &'static str {
        match self {
            LicenceError::Unreachable => "LICENCE_UNREACHABLE",
            LicenceError::CodeNotFound => "CODE_NOT_FOUND",
            LicenceError::CodeAlreadyUsed => "CODE_ALREADY_USED",
            LicenceError::Invalid => "LICENCE_INVALID",
        }
    }
    /// i18n key for the message the UI shows.
    pub fn message_key(&self) -> &'static str {
        match self {
            LicenceError::Unreachable => "licence.needs_internet",
            LicenceError::CodeNotFound => "licence.code_not_found",
            LicenceError::CodeAlreadyUsed => "licence.code_already_used",
            LicenceError::Invalid => "licence.invalid",
        }
    }
}

/// A verified activation, ready to persist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activation {
    pub licence: Licence,
    /// Base64 licence JSON exactly as the service returned it (stored in `raw_json`).
    pub licence_b64: String,
    /// Base64 ed25519 signature (stored in `licence.signature`).
    pub signature_b64: String,
}

#[derive(Debug, Serialize)]
struct ActivateBody<'a> {
    code: &'a str,
    school_name: &'a str,
    machine_id: &'a str,
    app_version: &'a str,
}

#[derive(Debug, Serialize)]
struct CheckBody<'a> {
    licence_id: &'a str,
    machine_id: &'a str,
}

/// Parse an `/v1/activate` HTTP result into an [`Activation`] or [`LicenceError`].
///
/// Pure (no IO) so it is unit-testable. `status` is the HTTP status code; `body`
/// is the raw response body; `machine_id` is this device's id (checked against the
/// signed `server_machine_id`); `public_key` is the build-config licence key.
pub fn parse_activate(
    status: u16,
    body: &str,
    machine_id: &str,
    public_key: &[u8; 32],
) -> Result<Activation, LicenceError> {
    match status {
        200 => {}
        404 => return Err(LicenceError::CodeNotFound),
        409 => return Err(LicenceError::CodeAlreadyUsed),
        503 => return Err(LicenceError::Unreachable),
        _ => return Err(LicenceError::Invalid),
    }
    let v: serde_json::Value = serde_json::from_str(body).map_err(|_| LicenceError::Invalid)?;
    let licence_b64 = v.get("licence").and_then(|x| x.as_str()).ok_or(LicenceError::Invalid)?;
    let signature_b64 = v.get("signature").and_then(|x| x.as_str()).ok_or(LicenceError::Invalid)?;

    // Verify the ed25519 signature offline with vidya-core.
    let licence = vidya_core::licence::verify(licence_b64, signature_b64, public_key)
        .map_err(|_| LicenceError::Invalid)?;

    // The licence must be bound to THIS machine (the server machine at activation).
    if licence.server_machine_id != machine_id {
        return Err(LicenceError::Invalid);
    }

    Ok(Activation {
        licence,
        licence_b64: licence_b64.to_string(),
        signature_b64: signature_b64.to_string(),
    })
}

/// Activate a code against the licence service and verify the returned licence.
///
/// Network/timeout/connection failures map to [`LicenceError::Unreachable`]
/// ("Activation needs internet once — please try again.").
pub async fn activate(
    client: &reqwest::Client,
    licence_api: &str,
    code: &str,
    school_name: &str,
    machine_id: &str,
    app_version: &str,
    public_key: &[u8; 32],
) -> Result<Activation, LicenceError> {
    let url = format!("{}/v1/activate", licence_api.trim_end_matches('/'));
    let resp = client
        .post(url)
        .json(&ActivateBody { code, school_name, machine_id, app_version })
        .send()
        .await
        .map_err(|_| LicenceError::Unreachable)?;
    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(|_| LicenceError::Unreachable)?;
    parse_activate(status, &body, machine_id, public_key)
}

/// Re-check a licence's status. `None` = service unreachable (licence stays
/// active under the perpetual model — §10).
pub async fn check(
    client: &reqwest::Client,
    licence_api: &str,
    licence_id: &str,
    machine_id: &str,
) -> Option<CheckResult> {
    let url = format!("{}/v1/check", licence_api.trim_end_matches('/'));
    let resp = client
        .post(url)
        .json(&CheckBody { licence_id, machine_id })
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: serde_json::Value = resp.json().await.ok()?;
    match v.get("status").and_then(|s| s.as_str())? {
        "active" => Some(CheckResult::Active),
        "revoked" => Some(CheckResult::Revoked),
        "moved" => Some(CheckResult::Moved),
        _ => None,
    }
}

/// Whether a re-check is due: no prior check, an unparseable timestamp, or more
/// than [`RECHECK_DAYS`] since `last_check_at` (RFC-3339).
pub fn should_recheck(last_check_at: Option<&str>, now: OffsetDateTime) -> bool {
    match last_check_at {
        None => true,
        Some(s) => match OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339) {
            Ok(last) => (now - last) >= time::Duration::days(RECHECK_DAYS),
            Err(_) => true,
        },
    }
}

/// Default location of the machine-id fallback file (used on Android/tests where
/// the OS keychain is unavailable). Desktop uses the keychain (see `machine`).
pub fn machine_id_fallback_path(app_data_dir: &std::path::Path) -> PathBuf {
    app_data_dir.join("machine-id")
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};

    fn keypair(seed: u8) -> (SigningKey, [u8; 32]) {
        let s = SigningKey::from_bytes(&[seed; 32]);
        let p = s.verifying_key().to_bytes();
        (s, p)
    }

    fn signed_body(lic: &Licence, signing: &SigningKey) -> String {
        let raw = serde_json::to_vec(lic).unwrap();
        let lic_b64 = STANDARD.encode(&raw);
        let sig_b64 = STANDARD.encode(signing.sign(&raw).to_bytes());
        serde_json::json!({ "licence": lic_b64, "signature": sig_b64 }).to_string()
    }

    fn sample(machine: &str) -> Licence {
        Licence {
            licence_id: "lic_1".into(),
            school_id: "sch_1".into(),
            plan: "perpetual".into(),
            max_students: None,
            max_devices: None,
            issued_at: "2026-09-23T00:00:00Z".into(),
            server_machine_id: machine.into(),
        }
    }

    #[test]
    fn good_activation_verifies_and_binds_machine() {
        let (signing, public) = keypair(7);
        let body = signed_body(&sample("machine-A"), &signing);
        let act = parse_activate(200, &body, "machine-A", &public).unwrap();
        assert_eq!(act.licence.licence_id, "lic_1");
        assert_eq!(act.licence.plan, "perpetual");
    }

    #[test]
    fn machine_mismatch_is_invalid() {
        let (signing, public) = keypair(7);
        let body = signed_body(&sample("machine-A"), &signing);
        assert_eq!(parse_activate(200, &body, "machine-B", &public), Err(LicenceError::Invalid));
    }

    #[test]
    fn wrong_key_is_invalid() {
        let (signing, _public) = keypair(7);
        let (_s2, wrong) = keypair(9);
        let body = signed_body(&sample("machine-A"), &signing);
        assert_eq!(parse_activate(200, &body, "machine-A", &wrong), Err(LicenceError::Invalid));
    }

    #[test]
    fn http_error_codes_map() {
        let (_s, public) = keypair(7);
        assert_eq!(parse_activate(404, "{}", "m", &public), Err(LicenceError::CodeNotFound));
        assert_eq!(parse_activate(409, "{}", "m", &public), Err(LicenceError::CodeAlreadyUsed));
        assert_eq!(parse_activate(503, "", "m", &public), Err(LicenceError::Unreachable));
        assert_eq!(parse_activate(500, "", "m", &public), Err(LicenceError::Invalid));
    }

    #[test]
    fn error_codes_and_keys() {
        assert_eq!(LicenceError::CodeAlreadyUsed.code(), "CODE_ALREADY_USED");
        assert_eq!(LicenceError::Unreachable.message_key(), "licence.needs_internet");
    }

    #[test]
    fn recheck_timing() {
        let fmt = &time::format_description::well_known::Rfc3339;
        let now = OffsetDateTime::parse("2026-09-23T00:00:00Z", fmt).unwrap();
        assert!(should_recheck(None, now));
        assert!(should_recheck(Some("garbage"), now));
        assert!(!should_recheck(Some("2026-09-10T00:00:00Z"), now)); // 13 days
        assert!(should_recheck(Some("2026-08-01T00:00:00Z"), now)); // > 30 days
    }
}
