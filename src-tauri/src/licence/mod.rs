//! App-side licence: machine id/code, **offline** verification (docs §10,
//! prompts/P12 Step 6).
//!
//! v2 licences are offline files: a **licence key** (or `.vlic`) minted by
//! `tools/licence-maker` and pasted / loaded on Welcome → Set up. The app verifies
//! the ed25519 signature against the build-config public key(s) and checks the
//! licence is bound to THIS computer's machine code. Perpetual: no expiry, no
//! online check, no remote revoke.
//!
//! v1 licences (already-activated installs) keep verifying via
//! `vidya_core::licence::verify` and are now treated as perpetual — the 30-day
//! online re-check and grace logic were removed in Phase 12.

use std::path::PathBuf;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

pub mod machine;

/// A user-facing licence failure. Commands turn this into `{code, message_key, vars}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenceError {
    /// Signature invalid, not a v2 licence, or a malformed key / file.
    Invalid,
    /// Correctly signed, but issued for a different computer.
    OtherMachine,
}

impl LicenceError {
    /// Stable machine code (mirrors the error contract in §10).
    pub fn code(&self) -> &'static str {
        match self {
            LicenceError::Invalid => "LICENCE_INVALID",
            LicenceError::OtherMachine => "LICENCE_OTHER_MACHINE",
        }
    }
    /// i18n key for the message the UI shows.
    pub fn message_key(&self) -> &'static str {
        match self {
            LicenceError::Invalid => "licence.invalid",
            LicenceError::OtherMachine => "licence.other_machine",
        }
    }
}

/// A verified offline licence, ready to persist into the `licence` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflineLicence {
    pub licence_id: String,
    pub school_name: String,
    pub plan: String,
    pub issued_at: String,
    /// STANDARD base64 of the signed payload bytes (stored in `licence.raw_json`).
    pub payload_b64: String,
    /// STANDARD base64 of the 64-byte signature (stored in `licence.signature`).
    pub signature_b64: String,
}

/// Verify a pasted licence key OR the text contents of a `.vlic` file against this
/// computer's machine code and the build-config public key(s).
///
/// Every public key is tried (supporting key rotation): a valid signature for THIS
/// machine → `Ok`; a valid signature for another machine → [`LicenceError::OtherMachine`];
/// nothing valid → [`LicenceError::Invalid`].
pub fn verify_offline(
    key_or_vlic_text: &str,
    machine_code: &str,
    public_keys: &[[u8; 32]],
) -> Result<OfflineLicence, LicenceError> {
    let (payload, signature) = vidya_core::licence::parse_licence_key(key_or_vlic_text)
        .map_err(|_| LicenceError::Invalid)?;

    let mut saw_other_machine = false;
    for key in public_keys {
        match vidya_core::licence::verify_v2(&payload, &signature, key, machine_code) {
            Ok(lic) => {
                return Ok(OfflineLicence {
                    licence_id: lic.licence_id,
                    school_name: lic.school_name,
                    plan: lic.plan,
                    issued_at: lic.issued_at,
                    payload_b64: STANDARD.encode(&payload),
                    signature_b64: STANDARD.encode(&signature),
                });
            }
            Err(vidya_core::errors::CoreError::LicenceOtherMachine) => saw_other_machine = true,
            Err(_) => {}
        }
    }
    Err(if saw_other_machine {
        LicenceError::OtherMachine
    } else {
        LicenceError::Invalid
    })
}

/// Default location of the machine-id fallback file (used on Android/tests where
/// the OS keychain is unavailable). Desktop uses the keychain (see `machine`).
pub fn machine_id_fallback_path(app_data_dir: &std::path::Path) -> PathBuf {
    app_data_dir.join("machine-id")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use vidya_core::licence::{encode_licence_key, machine_code, LicenceV2};

    fn keypair(seed: u8) -> (SigningKey, [u8; 32]) {
        let s = SigningKey::from_bytes(&[seed; 32]);
        let p = s.verifying_key().to_bytes();
        (s, p)
    }

    fn make_key(machine: &str, signing: &SigningKey) -> String {
        let lic = LicenceV2 {
            v: 2,
            licence_id: "lic-1".into(),
            school_name: "Test School".into(),
            machine_code: machine.into(),
            issued_at: "2026-09-25T00:00:00Z".into(),
            plan: "perpetual".into(),
            modules: vec!["core".into()],
        };
        let payload = serde_json::to_vec(&lic).unwrap();
        let sig = signing.sign(&payload).to_bytes();
        encode_licence_key(&payload, &sig)
    }

    #[test]
    fn verifies_for_this_machine() {
        let (signing, public) = keypair(5);
        let mc = machine_code("this-pc");
        let key = make_key(&mc, &signing);
        let lic = verify_offline(&key, &mc, &[public]).unwrap();
        assert_eq!(lic.licence_id, "lic-1");
        assert_eq!(lic.school_name, "Test School");
        assert_eq!(lic.plan, "perpetual");
        assert!(!lic.payload_b64.is_empty() && !lic.signature_b64.is_empty());
    }

    #[test]
    fn wrong_machine_is_other_machine() {
        let (signing, public) = keypair(5);
        let key = make_key(&machine_code("pc-A"), &signing);
        assert_eq!(
            verify_offline(&key, &machine_code("pc-B"), &[public]),
            Err(LicenceError::OtherMachine)
        );
    }

    #[test]
    fn garbage_is_invalid() {
        let (_s, public) = keypair(5);
        assert_eq!(verify_offline("not-a-key", "0000-0000-0000-0", &[public]), Err(LicenceError::Invalid));
    }

    #[test]
    fn tries_all_public_keys_for_rotation() {
        let (signing, public) = keypair(5);
        let (_s2, wrong) = keypair(9);
        let mc = machine_code("this-pc");
        let key = make_key(&mc, &signing);
        // The right key is second in the list (rotation scenario).
        let lic = verify_offline(&key, &mc, &[wrong, public]).unwrap();
        assert_eq!(lic.licence_id, "lic-1");
        // No matching key at all → Invalid.
        assert_eq!(verify_offline(&key, &mc, &[wrong]), Err(LicenceError::Invalid));
    }
}
