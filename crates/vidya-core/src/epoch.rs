//! Server epoch fencing (docs/00-SYSTEM-CONTEXT.md §8.9).
//!
//! `school.server_epoch` starts at 1 and increases on restore to a new PC or a
//! licence transfer. Every server response carries the epoch. A device that
//! already knows some epoch MUST refuse a server whose epoch is **lower** than
//! the one it knows — that server is a stale/old machine that has been fenced
//! out. An equal or higher epoch is accepted.
//!
//! v2 (Phase 12, prompts/P12 Step 5): without a licence server, fencing travels
//! through the school's Google Drive as a signed `exchange/epoch.json`. The school
//! server holds an ed25519 key pair (generated at setup); it signs `epoch.json` on
//! setup, on every epoch change and daily. On every import the server reads the
//! file: if the file's epoch is higher than its own, another PC has taken over and
//! this one switches to a read-only fenced state ("This computer is no longer the
//! school server"). Devices store the highest epoch seen from either the server or
//! the file and refuse anything lower.
//!
//! Pure: epochs/keys/bytes are passed in, no IO, no clock, no randomness (ed25519
//! signing is deterministic, RFC 8032), no floats.

use std::cmp::Ordering;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::errors::{CoreError, CoreResult};

/// Reject a server whose `server_epoch` is older than the `known_epoch` the
/// device already trusts.
///
/// * `server_epoch < known_epoch` → [`CoreError::EpochOld`]
/// * `server_epoch >= known_epoch` → `Ok(())`
pub fn check_epoch(known_epoch: u64, server_epoch: u64) -> CoreResult<()> {
    if server_epoch < known_epoch {
        Err(CoreError::EpochOld)
    } else {
        Ok(())
    }
}

/// The signed content of `exchange/epoch.json` (docs §8.9, §11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpochFile {
    pub school_id: String,
    pub server_epoch: u64,
    /// The machine code of the PC that currently holds the school-server role.
    pub server_machine_code: String,
    /// RFC-3339 time the marker was written (informational).
    pub at: String,
}

/// The on-Drive `epoch.json` document: a base64url payload + its ed25519 signature.
/// The signature is always verified over the RAW payload bytes (never a
/// re-serialisation), so field ordering is irrelevant to verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedEpoch {
    /// base64url (no pad) of the canonical [`EpochFile`] JSON.
    pub payload: String,
    /// base64url (no pad) of the 64-byte ed25519 signature over the payload bytes.
    pub sig: String,
}

/// Sign an [`EpochFile`] with the school server's ed25519 private seed → the
/// `epoch.json` document the server writes to Drive.
pub fn sign_epoch(epoch: &EpochFile, signing_key_seed: &[u8; 32]) -> SignedEpoch {
    let payload = serde_json::to_vec(epoch).expect("EpochFile serialises");
    let sk = SigningKey::from_bytes(signing_key_seed);
    let sig = sk.sign(&payload).to_bytes();
    SignedEpoch {
        payload: URL_SAFE_NO_PAD.encode(&payload),
        sig: URL_SAFE_NO_PAD.encode(sig),
    }
}

/// Verify an `epoch.json` document against the school server's public key. Any
/// decode/signature/parse failure → [`CoreError::EpochInvalid`] (a tampered or
/// corrupt marker is ignored, never trusted).
pub fn verify_epoch(signed: &SignedEpoch, public_key: &[u8; 32]) -> CoreResult<EpochFile> {
    let payload = URL_SAFE_NO_PAD
        .decode(signed.payload.as_bytes())
        .map_err(|_| CoreError::EpochInvalid)?;
    let sig_bytes = URL_SAFE_NO_PAD
        .decode(signed.sig.as_bytes())
        .map_err(|_| CoreError::EpochInvalid)?;
    let vk = VerifyingKey::from_bytes(public_key).map_err(|_| CoreError::EpochInvalid)?;
    let sig = Signature::from_slice(&sig_bytes).map_err(|_| CoreError::EpochInvalid)?;
    vk.verify_strict(&payload, &sig).map_err(|_| CoreError::EpochInvalid)?;
    serde_json::from_slice(&payload).map_err(|_| CoreError::EpochInvalid)
}

/// What a running school server should do after reading a verified `epoch.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceOutcome {
    /// The file's epoch equals ours — we are still the school server.
    Current,
    /// The file's epoch is HIGHER — another PC took over; go read-only fenced
    /// ("This computer is no longer the school server").
    Fenced,
    /// The file's epoch is lower — our epoch is authoritative; (re)write the file.
    Stale,
}

/// Decide the outcome by comparing our `own_epoch` with a verified `file_epoch`.
pub fn fence_outcome(own_epoch: u64, file_epoch: u64) -> FenceOutcome {
    match file_epoch.cmp(&own_epoch) {
        Ordering::Greater => FenceOutcome::Fenced,
        Ordering::Equal => FenceOutcome::Current,
        Ordering::Less => FenceOutcome::Stale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_epoch_is_rejected() {
        assert_eq!(check_epoch(5, 4), Err(CoreError::EpochOld));
        assert_eq!(check_epoch(2, 1), Err(CoreError::EpochOld));
        assert_eq!(check_epoch(u64::MAX, 0), Err(CoreError::EpochOld));
    }

    #[test]
    fn equal_epoch_is_ok() {
        assert!(check_epoch(1, 1).is_ok());
        assert!(check_epoch(7, 7).is_ok());
        assert!(check_epoch(0, 0).is_ok());
    }

    #[test]
    fn higher_epoch_is_ok() {
        assert!(check_epoch(1, 2).is_ok());
        assert!(check_epoch(4, 5).is_ok());
        assert!(check_epoch(0, u64::MAX).is_ok());
    }

    // ----- Signed epoch.json (Step 5) -----

    fn seed_and_public(seed: u8) -> ([u8; 32], [u8; 32]) {
        let sk = SigningKey::from_bytes(&[seed; 32]);
        ([seed; 32], sk.verifying_key().to_bytes())
    }

    fn sample_epoch(n: u64) -> EpochFile {
        EpochFile {
            school_id: "sch_1".into(),
            server_epoch: n,
            server_machine_code: "7KQ2-M9XD-4TRA-P".into(),
            at: "2026-09-25T00:00:00Z".into(),
        }
    }

    #[test]
    fn epoch_sign_verify_round_trip() {
        let (seed, public) = seed_and_public(11);
        let signed = sign_epoch(&sample_epoch(3), &seed);
        let got = verify_epoch(&signed, &public).unwrap();
        assert_eq!(got, sample_epoch(3));
    }

    #[test]
    fn epoch_wrong_key_is_invalid() {
        let (seed, _public) = seed_and_public(11);
        let (_s2, wrong) = seed_and_public(22);
        let signed = sign_epoch(&sample_epoch(3), &seed);
        assert_eq!(verify_epoch(&signed, &wrong), Err(CoreError::EpochInvalid));
    }

    #[test]
    fn epoch_tampered_payload_is_invalid() {
        let (seed, public) = seed_and_public(11);
        let mut signed = sign_epoch(&sample_epoch(3), &seed);
        // Re-encode a DIFFERENT payload (epoch 9) but keep the old signature.
        signed.payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&sample_epoch(9)).unwrap());
        assert_eq!(verify_epoch(&signed, &public), Err(CoreError::EpochInvalid));
    }

    #[test]
    fn epoch_garbage_is_invalid() {
        let (_seed, public) = seed_and_public(11);
        let bad = SignedEpoch { payload: "!!!".into(), sig: "!!!".into() };
        assert_eq!(verify_epoch(&bad, &public), Err(CoreError::EpochInvalid));
    }

    #[test]
    fn fence_outcome_covers_all_cases() {
        // A higher file epoch fences this PC (another PC took over).
        assert_eq!(fence_outcome(1, 2), FenceOutcome::Fenced);
        // Equal → still the server.
        assert_eq!(fence_outcome(2, 2), FenceOutcome::Current);
        // Lower/older file → our epoch wins; rewrite the marker.
        assert_eq!(fence_outcome(3, 2), FenceOutcome::Stale);
    }

    #[test]
    fn transfer_flow_fences_the_old_pc() {
        // The old PC is at epoch 1; a transfer licence bumps the school to epoch 2
        // and the new PC writes a signed epoch.json. The old PC reads it and fences.
        let (seed, public) = seed_and_public(7);
        let new_marker = sign_epoch(&sample_epoch(2), &seed);
        let verified = verify_epoch(&new_marker, &public).unwrap();
        assert_eq!(fence_outcome(1, verified.server_epoch), FenceOutcome::Fenced);
        // A device that already saw epoch 2 refuses the old epoch-1 server.
        assert_eq!(check_epoch(2, 1), Err(CoreError::EpochOld));
    }
}
