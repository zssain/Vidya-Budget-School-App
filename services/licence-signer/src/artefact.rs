//! Pure licence-artefact construction: validate inputs, build the v2 payload
//! exactly as `tools/licence-maker` does, sign, and re-verify with the app's own
//! verifier (`vidya_core::licence::verify_v2`) before anything is written.
//!
//! Byte-compatibility with licence-maker (pinned by the golden-vector tests):
//! - payload = `serde_json::to_vec(&LicenceV2 { v: 2, licence_id, school_name,
//!   machine_code, issued_at, plan: "perpetual", modules })` — `vidya_core`'s
//!   `LicenceV2` has the same field order as licence-maker's mirror struct;
//! - `machine_code` is the canonical dashed upper-case form `XXXX-XXXX-XXXX-C`;
//! - compact key = base64url-no-pad(payload ‖ 64-byte signature)
//!   (`vidya_core::licence::encode_licence_key`);
//! - `.vlic` file contents = the compact key, no trailing newline;
//! - display key (`licence_issuances.licence_key`) = compact key in groups of 5
//!   separated by single spaces (licence-maker's `group5`).

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use vidya_core::licence::{encode_licence_key, parse_licence_key, parse_machine_code, verify_v2, LicenceV2};
use vidya_core::modules::TOGGLEABLE_KEYS;

use crate::key::Signer;

/// Inputs read from `licence_issuances` + `entitlements`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenceInputs {
    pub licence_id: String,
    pub school_name: String,
    pub machine_code: String,
    pub issued_at: String,
    pub modules: Vec<String>,
}

/// A signed, self-verified licence ready to persist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artefact {
    pub payload: Vec<u8>,
    pub signature: Vec<u8>,
    /// Grouped-by-5 display key (`licence_issuances.licence_key`).
    pub licence_key: String,
    /// `.vlic` file contents (the compact key).
    pub vlic: String,
    pub licence: LicenceV2,
}

/// Every module key a licence may carry: `core` + the app's toggleable modules,
/// in canonical order.
pub fn allowed_modules() -> Vec<&'static str> {
    let mut v = vec!["core"];
    v.extend_from_slice(TOGGLEABLE_KEYS);
    v
}

/// Canonical dashed display form of a typed machine code (checksum validated by
/// `vidya_core::licence::parse_machine_code`), or `None` if invalid.
pub fn canonical_machine_code(text: &str) -> Option<String> {
    let c = parse_machine_code(text)?; // 13 chars, no dashes
    Some(format!("{}-{}-{}-{}", &c[0..4], &c[4..8], &c[8..12], &c[12..13]))
}

/// licence-maker's `group5`: groups of 5 chars joined by single spaces.
pub fn group5(key: &str) -> String {
    key.as_bytes()
        .chunks(5)
        .map(|c| std::str::from_utf8(c).unwrap_or_default())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Validate and canonicalise the module list: must contain `core`, every key must
/// be in [`allowed_modules`], duplicates collapse, output is in canonical order
/// (so `core` is always first and retries always produce the same bytes).
pub fn canonical_modules(modules: &[String]) -> Result<Vec<String>, String> {
    let allowed = allowed_modules();
    for m in modules {
        if !allowed.contains(&m.as_str()) {
            return Err(format!("module '{}' is not an allowed licence module", sanitise(m)));
        }
    }
    if !modules.iter().any(|m| m == "core") {
        return Err("modules must include 'core'".into());
    }
    Ok(allowed
        .iter()
        .filter(|a| modules.iter().any(|m| m == *a))
        .map(|a| a.to_string())
        .collect())
}

fn no_control(s: &str) -> bool {
    !s.chars().any(char::is_control)
}

/// Keep user-ish text out of error messages beyond a short, printable prefix.
fn sanitise(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).take(40).collect()
}

/// Validate inputs and produce the exact payload struct to sign.
pub fn build_licence(inputs: &LicenceInputs) -> Result<LicenceV2, String> {
    let id = &inputs.licence_id;
    if id.is_empty() || id.len() > 128 || !id.bytes().all(|b| b.is_ascii_graphic()) {
        return Err("licence_id must be 1-128 printable ASCII chars".into());
    }
    let school = &inputs.school_name;
    if school.trim().is_empty() || school.chars().count() > 200 || !no_control(school) {
        return Err("school_name must be 1-200 chars with no control characters".into());
    }
    let machine_code = canonical_machine_code(&inputs.machine_code)
        .ok_or_else(|| "machine code is invalid (format or checksum)".to_string())?;
    OffsetDateTime::parse(&inputs.issued_at, &Rfc3339)
        .map_err(|_| "issued_at is not an RFC 3339 timestamp".to_string())?;
    let modules = canonical_modules(&inputs.modules)?;
    Ok(LicenceV2 {
        v: 2,
        licence_id: id.clone(),
        school_name: school.clone(),
        machine_code,
        issued_at: inputs.issued_at.clone(),
        plan: "perpetual".into(),
        modules,
    })
}

/// Validate inputs and produce the exact payload bytes to sign (no key needed).
/// The worker persists these bytes BEFORE signing, so every retry signs the same
/// bytes even if a crash happens between persisting and signing.
pub fn build_payload(inputs: &LicenceInputs) -> Result<(LicenceV2, Vec<u8>), String> {
    let licence = build_licence(inputs)?;
    let payload = serde_json::to_vec(&licence).map_err(|_| "payload serialisation failed".to_string())?;
    Ok((licence, payload))
}

/// Build, sign and **verify** a licence. Fails (nothing to persist) if the app's
/// verifier does not accept the result with the signer's own public key.
pub fn sign_licence(inputs: &LicenceInputs, signer: &Signer) -> Result<Artefact, String> {
    let (licence, payload) = build_payload(inputs)?;
    sign_payload(licence, payload, signer)
}

/// Sign already-built payload bytes and self-verify them through the app's
/// verifier. `payload` must equal `serde_json::to_vec(&licence)` (checked).
pub fn sign_payload(licence: LicenceV2, payload: Vec<u8>, signer: &Signer) -> Result<Artefact, String> {
    let rebuilt = serde_json::to_vec(&licence).map_err(|_| "payload serialisation failed".to_string())?;
    if rebuilt != payload {
        return Err("payload bytes do not match the licence fields".into());
    }
    let signature = signer.sign(&payload).to_vec();
    let compact = encode_licence_key(&payload, &signature);

    // Self-check through the exact path the app uses: parse the key, verify_v2.
    let (p, s) = parse_licence_key(&compact).map_err(|_| "self-check: key does not parse".to_string())?;
    if p != payload || s != signature {
        return Err("self-check: key does not round-trip".into());
    }
    let verified = verify_v2(&p, &s, signer.public_key(), &licence.machine_code)
        .map_err(|e| format!("self-check: verify_v2 rejected the artefact ({e:?})"))?;
    if verified != licence {
        return Err("self-check: verified payload differs from the built payload".into());
    }

    Ok(Artefact {
        payload,
        signature,
        licence_key: group5(&compact),
        vlic: compact,
        licence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modules_canonicalised() {
        let m = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            canonical_modules(&m(&["store", "core", "accounts", "core"])).unwrap(),
            m(&["core", "accounts", "store"])
        );
        assert!(canonical_modules(&m(&["accounts"])).is_err());
        assert!(canonical_modules(&m(&["core", "chess"])).is_err());
        assert!(canonical_modules(&[]).is_err());
    }

    #[test]
    fn machine_code_canonical_form() {
        let mc = vidya_core::licence::machine_code("golden-machine");
        assert_eq!(
            canonical_machine_code(&mc.to_lowercase().replace('-', " ")).as_deref(),
            Some(mc.as_str())
        );
        assert_eq!(canonical_machine_code("3N0S-10YX-JQCR-X"), None);
    }

    #[test]
    fn group5_matches_maker() {
        assert_eq!(group5("abcdefghijkl"), "abcde fghij kl");
        assert_eq!(group5(""), "");
    }
}
