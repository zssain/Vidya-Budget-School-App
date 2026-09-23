//! Licence verification and status (prompts/P02 licence.rs, docs §10).
//!
//! Business model: **one-time purchase, perpetual licence, no expiry.** There is
//! no expiry, no subscription, no grace period. The app verifies the ed25519
//! signature offline against the public key from build config, and re-checks the
//! service every 30 days when online. If the service is unreachable the licence
//! stays active indefinitely; only an explicit `revoked`/`moved` answer changes
//! the status.
//!
//! Pure: no IO, no clock, no randomness (all inputs passed in), no floats.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::errors::{CoreError, CoreResult};

/// A signed, server-issued licence (docs §10). The JSON of this struct is what
/// the `cloud/licence` service signs with its ed25519 private key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Licence {
    pub licence_id: String,
    pub school_id: String,
    pub plan: String,
    /// `null` = unlimited students.
    pub max_students: Option<u32>,
    /// `null` = unlimited devices.
    pub max_devices: Option<u32>,
    pub issued_at: String,
    pub server_machine_id: String,
}

/// The answer from the licence service's `/v1/check` endpoint (docs §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckResult {
    Active,
    Revoked,
    Moved,
}

/// The most recent licence check. `result: None` means the service was
/// unreachable at the last attempt (perpetual model → licence stays active).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastCheck {
    pub result: Option<CheckResult>,
}

/// The effective licence status a device should act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenceStatus {
    Active,
    Revoked,
    Moved,
}

/// Verify a base64-encoded licence JSON against its base64 ed25519 signature.
///
/// Both `licence_b64_json` and `signature_b64` are STANDARD base64. The
/// signature is verified (with `verify_strict`) over the RAW decoded licence
/// bytes — the exact bytes the service signed, not a re-serialised form. Only on
/// a valid signature are those bytes parsed into a [`Licence`].
///
/// Any failure (bad base64, wrong signature length, wrong key, tampered bytes,
/// or malformed JSON) → [`CoreError::LicenceInvalid`].
pub fn verify(
    licence_b64_json: &str,
    signature_b64: &str,
    public_key: &[u8; 32],
) -> CoreResult<Licence> {
    // Decode the raw licence bytes and the signature.
    let licence_bytes = STANDARD
        .decode(licence_b64_json.as_bytes())
        .map_err(|_| CoreError::LicenceInvalid)?;
    let signature_bytes = STANDARD
        .decode(signature_b64.as_bytes())
        .map_err(|_| CoreError::LicenceInvalid)?;

    // Build the verifying key and signature (both length-checked here).
    let verifying_key =
        VerifyingKey::from_bytes(public_key).map_err(|_| CoreError::LicenceInvalid)?;
    let signature =
        Signature::from_slice(&signature_bytes).map_err(|_| CoreError::LicenceInvalid)?;

    // Verify over the RAW decoded licence bytes.
    verifying_key
        .verify_strict(&licence_bytes, &signature)
        .map_err(|_| CoreError::LicenceInvalid)?;

    // Only now parse the JSON.
    serde_json::from_slice::<Licence>(&licence_bytes).map_err(|_| CoreError::LicenceInvalid)
}

/// Resolve the effective licence status from the last check (docs §10).
///
/// Perpetual model: an unreachable service (`result: None`) keeps the licence
/// **Active**; only an explicit `Revoked`/`Moved` answer changes the status.
pub fn status(check: LastCheck) -> LicenceStatus {
    match check.result {
        None | Some(CheckResult::Active) => LicenceStatus::Active,
        Some(CheckResult::Revoked) => LicenceStatus::Revoked,
        Some(CheckResult::Moved) => LicenceStatus::Moved,
    }
}

/// Enforce the licence's device/student caps (docs §10).
///
/// A `null` (`None`) cap means unlimited. If a non-null cap is exceeded the first
/// breach reported is students (then devices) → [`CoreError::LicenceLimit`].
pub fn check_limits(lic: &Licence, current_students: u32, current_devices: u32) -> CoreResult<()> {
    if let Some(max) = lic.max_students {
        if current_students > max {
            return Err(CoreError::LicenceLimit { what: "students".into() });
        }
    }
    if let Some(max) = lic.max_devices {
        if current_devices > max {
            return Err(CoreError::LicenceLimit { what: "devices".into() });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    /// A fixed 32-byte seed → deterministic keypair (no RNG needed; the
    /// `generate` helper is feature-gated and we only depend on the crate, not
    /// the `rand_core` feature).
    fn test_keypair(seed: u8) -> (SigningKey, [u8; 32]) {
        let secret = [seed; 32];
        let signing = SigningKey::from_bytes(&secret);
        let public = signing.verifying_key().to_bytes();
        (signing, public)
    }

    fn sample_licence() -> Licence {
        Licence {
            licence_id: "lic_123".into(),
            school_id: "sch_abc".into(),
            plan: "perpetual".into(),
            max_students: Some(200),
            max_devices: Some(5),
            issued_at: "2026-09-23T00:00:00Z".into(),
            server_machine_id: "machine_xyz".into(),
        }
    }

    /// Serialise, sign the RAW bytes, and base64-encode both — the exact shape
    /// the licence service produces.
    fn make_fixture(lic: &Licence, signing: &SigningKey) -> (String, String) {
        let raw = serde_json::to_vec(lic).unwrap();
        let sig = signing.sign(&raw);
        let lic_b64 = STANDARD.encode(&raw);
        let sig_b64 = STANDARD.encode(sig.to_bytes());
        (lic_b64, sig_b64)
    }

    #[test]
    fn good_signature_verifies() {
        let (signing, public) = test_keypair(7);
        let lic = sample_licence();
        let (lic_b64, sig_b64) = make_fixture(&lic, &signing);

        let got = verify(&lic_b64, &sig_b64, &public).unwrap();
        assert_eq!(got, lic);
    }

    #[test]
    fn tampered_licence_is_invalid() {
        let (signing, public) = test_keypair(7);
        let lic = sample_licence();
        let (lic_b64, sig_b64) = make_fixture(&lic, &signing);

        // Tamper with the licence bytes after signing: flip the plan.
        let mut raw = STANDARD.decode(&lic_b64).unwrap();
        // Flip one byte inside the JSON payload.
        raw[10] ^= 0x01;
        let tampered_b64 = STANDARD.encode(&raw);

        assert_eq!(
            verify(&tampered_b64, &sig_b64, &public),
            Err(CoreError::LicenceInvalid)
        );
    }

    #[test]
    fn wrong_key_is_invalid() {
        let (signing, _public) = test_keypair(7);
        let (_other_signing, wrong_public) = test_keypair(9);
        let lic = sample_licence();
        let (lic_b64, sig_b64) = make_fixture(&lic, &signing);

        assert_eq!(
            verify(&lic_b64, &sig_b64, &wrong_public),
            Err(CoreError::LicenceInvalid)
        );
    }

    #[test]
    fn bad_base64_is_invalid() {
        let (_signing, public) = test_keypair(7);
        assert_eq!(
            verify("!!!not base64!!!", "also***bad", &public),
            Err(CoreError::LicenceInvalid)
        );
    }

    #[test]
    fn wrong_signature_length_is_invalid() {
        let (signing, public) = test_keypair(7);
        let lic = sample_licence();
        let (lic_b64, _sig_b64) = make_fixture(&lic, &signing);
        // A short "signature" that base64-decodes but is not 64 bytes.
        let short_sig = STANDARD.encode([0u8; 10]);
        assert_eq!(
            verify(&lic_b64, &short_sig, &public),
            Err(CoreError::LicenceInvalid)
        );
    }

    #[test]
    fn status_unreachable_stays_active() {
        // Perpetual: no answer keeps the licence Active.
        assert_eq!(status(LastCheck { result: None }), LicenceStatus::Active);
    }

    #[test]
    fn status_explicit_answers() {
        assert_eq!(
            status(LastCheck { result: Some(CheckResult::Active) }),
            LicenceStatus::Active
        );
        assert_eq!(
            status(LastCheck { result: Some(CheckResult::Revoked) }),
            LicenceStatus::Revoked
        );
        assert_eq!(
            status(LastCheck { result: Some(CheckResult::Moved) }),
            LicenceStatus::Moved
        );
    }

    #[test]
    fn limits_null_is_unlimited() {
        let lic = Licence {
            max_students: None,
            max_devices: None,
            ..sample_licence()
        };
        // Huge counts are fine when caps are null.
        assert!(check_limits(&lic, u32::MAX, u32::MAX).is_ok());
    }

    #[test]
    fn limits_within_caps_ok() {
        let lic = sample_licence(); // 200 students, 5 devices
        assert!(check_limits(&lic, 200, 5).is_ok()); // exactly at cap
        assert!(check_limits(&lic, 199, 4).is_ok());
    }

    #[test]
    fn limits_over_students_errors() {
        let lic = sample_licence();
        assert_eq!(
            check_limits(&lic, 201, 5),
            Err(CoreError::LicenceLimit { what: "students".into() })
        );
    }

    #[test]
    fn limits_over_devices_errors() {
        let lic = sample_licence();
        assert_eq!(
            check_limits(&lic, 200, 6),
            Err(CoreError::LicenceLimit { what: "devices".into() })
        );
    }
}
