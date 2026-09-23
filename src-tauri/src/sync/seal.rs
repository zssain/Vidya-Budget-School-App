//! End-to-end sealing for relay traffic (prompts/P05 Step 2).
//!
//! Over the LAN the client talks TLS pinned to the school's certificate, so the
//! LAN path needs no extra sealing. Over the relay, TLS terminates AT the relay —
//! so every relayed body is sealed end-to-end between the device and the school
//! server. The relay only ever sees `nonce ‖ ciphertext` and cannot read or alter
//! it (rule §6, docs §9).
//!
//! Envelope: `nonce(12) ‖ ChaCha20-Poly1305(dir_key, plaintext)` with associated
//! data = `method + path + device_id + server_epoch` (bound so a tampered method,
//! path, device or epoch fails the AEAD).
//!
//! The session key agreed at join (Phase 4, one per device) is expanded with an
//! HKDF-style single-block SHA-256 expansion (`hmac` + `sha2`, no extra crates)
//! into TWO per-direction keys so client→server and server→client never share a
//! keystream:
//!   * `c2s` — device seals requests, server opens them
//!   * `s2c` — server seals responses, device opens them

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Info strings for the two directions (HKDF-Expand `info`). Stable on the wire.
const INFO_C2S: &[u8] = b"vidya/relay/c2s/v1";
const INFO_S2C: &[u8] = b"vidya/relay/s2c/v1";

/// A sealing/opening failure. `Aead` covers a tampered byte, wrong key or wrong
/// associated data — all indistinguishable and all rejected the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealError {
    /// The base64 session key did not decode to 32 bytes.
    BadSessionKey,
    /// The sealed blob was too short to contain a 12-byte nonce + tag.
    TooShort,
    /// AEAD open failed: tampered ciphertext, wrong key, or wrong associated data.
    Aead,
}

/// The two per-direction keys derived from a device's session key.
#[derive(Clone)]
pub struct DirectionKeys {
    /// client → server (device seals, server opens).
    pub c2s: [u8; 32],
    /// server → client (server seals, device opens).
    pub s2c: [u8; 32],
}

/// HKDF-Expand, single 32-byte block: `T(1) = HMAC-SHA256(prk, info ‖ 0x01)`.
/// One block is exactly the 32 bytes a ChaCha20-Poly1305 key needs.
fn expand(prk: &[u8], info: &[u8]) -> [u8; 32] {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(prk).expect("HMAC accepts any key length");
    mac.update(info);
    mac.update(&[0x01]);
    let out = Mac::finalize(mac).into_bytes();
    let mut k = [0u8; 32];
    k.copy_from_slice(&out);
    k
}

/// Derive the per-direction keys from a base64 32-byte session key.
pub fn derive_keys(session_key_b64: &str) -> Result<DirectionKeys, SealError> {
    let prk = STANDARD.decode(session_key_b64.as_bytes()).map_err(|_| SealError::BadSessionKey)?;
    if prk.len() != 32 {
        return Err(SealError::BadSessionKey);
    }
    Ok(DirectionKeys { c2s: expand(&prk, INFO_C2S), s2c: expand(&prk, INFO_S2C) })
}

/// Build the associated data bound into the AEAD (P05 Step 2).
/// A change to any of method/path/device_id/server_epoch fails `open`.
pub fn associated_data(method: &str, path: &str, device_id: &str, server_epoch: i64) -> Vec<u8> {
    format!("{method}\n{path}\n{device_id}\n{server_epoch}").into_bytes()
}

/// Seal `plaintext` under `key` with `aad` → `nonce(12) ‖ ciphertext+tag`.
pub fn seal(key: &[u8; 32], aad: &[u8], plaintext: &[u8]) -> Vec<u8> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), Payload { msg: plaintext, aad })
        .expect("ChaCha20-Poly1305 encryption is infallible for valid inputs");
    let mut out = Vec::with_capacity(12 + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    out
}

/// Open a `nonce(12) ‖ ciphertext+tag` blob under `key` with `aad`.
pub fn open(key: &[u8; 32], aad: &[u8], sealed: &[u8]) -> Result<Vec<u8>, SealError> {
    if sealed.len() < 12 + 16 {
        return Err(SealError::TooShort);
    }
    let (nonce_bytes, ct) = sealed.split_at(12);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    cipher
        .decrypt(Nonce::from_slice(nonce_bytes), Payload { msg: ct, aad })
        .map_err(|_| SealError::Aead)
}

/// Convenience: seal to base64 (the wire form used by the sealed envelope).
pub fn seal_b64(key: &[u8; 32], aad: &[u8], plaintext: &[u8]) -> String {
    STANDARD.encode(seal(key, aad, plaintext))
}

/// Convenience: base64-decode then open.
pub fn open_b64(key: &[u8; 32], aad: &[u8], sealed_b64: &str) -> Result<Vec<u8>, SealError> {
    let raw = STANDARD.decode(sealed_b64.as_bytes()).map_err(|_| SealError::Aead)?;
    open(key, aad, &raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> String {
        // A fixed 32-byte session key (base64) for deterministic tests.
        STANDARD.encode([7u8; 32])
    }

    #[test]
    fn round_trips_both_directions() {
        let k = derive_keys(&key()).unwrap();
        let aad = associated_data("POST", "/v1/sync/push", "dev-1", 1);
        let msg = br#"{"ops":[]}"#;
        let sealed = seal(&k.c2s, &aad, msg);
        assert_eq!(open(&k.c2s, &aad, &sealed).unwrap(), msg);
        let resp = br#"{"results":[]}"#;
        let sealed_r = seal(&k.s2c, &aad, resp);
        assert_eq!(open(&k.s2c, &aad, &sealed_r).unwrap(), resp);
    }

    #[test]
    fn directions_have_distinct_keys() {
        let k = derive_keys(&key()).unwrap();
        assert_ne!(k.c2s, k.s2c, "per-direction keys must differ");
    }

    #[test]
    fn tampering_one_byte_is_rejected() {
        // DONE MEANS #3: flip a byte of the sealed blob → AEAD open fails.
        let k = derive_keys(&key()).unwrap();
        let aad = associated_data("POST", "/v1/sync/push", "dev-1", 1);
        let mut sealed = seal(&k.c2s, &aad, b"hello");
        let last = sealed.len() - 1;
        sealed[last] ^= 0x01;
        assert_eq!(open(&k.c2s, &aad, &sealed), Err(SealError::Aead));
        // Flip a byte inside the ciphertext body too.
        let mut sealed2 = seal(&k.c2s, &aad, b"hello");
        sealed2[13] ^= 0x80;
        assert_eq!(open(&k.c2s, &aad, &sealed2), Err(SealError::Aead));
    }

    #[test]
    fn changed_associated_data_is_rejected() {
        let k = derive_keys(&key()).unwrap();
        let aad = associated_data("POST", "/v1/sync/push", "dev-1", 1);
        let sealed = seal(&k.c2s, &aad, b"hello");
        // A different epoch / path / device fails to open.
        for bad in [
            associated_data("POST", "/v1/sync/push", "dev-1", 2),
            associated_data("GET", "/v1/sync/push", "dev-1", 1),
            associated_data("POST", "/v1/sync/pull", "dev-1", 1),
            associated_data("POST", "/v1/sync/push", "dev-2", 1),
        ] {
            assert_eq!(open(&k.c2s, &bad, &sealed), Err(SealError::Aead));
        }
    }

    #[test]
    fn wrong_direction_key_is_rejected() {
        let k = derive_keys(&key()).unwrap();
        let aad = associated_data("POST", "/v1/sync/push", "dev-1", 1);
        let sealed = seal(&k.c2s, &aad, b"hello");
        // The response key cannot open a request envelope.
        assert_eq!(open(&k.s2c, &aad, &sealed), Err(SealError::Aead));
    }

    #[test]
    fn base64_helpers_round_trip() {
        let k = derive_keys(&key()).unwrap();
        let aad = associated_data("POST", "/v1/sync/push", "dev-1", 3);
        let b64 = seal_b64(&k.c2s, &aad, b"payload");
        assert_eq!(open_b64(&k.c2s, &aad, &b64).unwrap(), b"payload");
    }

    #[test]
    fn bad_session_key_length_errors() {
        // DirectionKeys deliberately has no PartialEq/Debug (it holds key material),
        // so assert on the error arm via `matches!`.
        assert!(matches!(derive_keys(&STANDARD.encode([0u8; 16])), Err(SealError::BadSessionKey)));
        assert!(matches!(derive_keys("not base64!!!"), Err(SealError::BadSessionKey)));
    }

    #[test]
    fn too_short_blob_errors() {
        let k = derive_keys(&key()).unwrap();
        assert_eq!(open(&k.c2s, b"aad", &[0u8; 10]), Err(SealError::TooShort));
    }
}
