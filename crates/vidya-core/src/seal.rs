//! End-to-end sealing primitives — the ONE implementation shared by the desktop/
//! Android app (`src-tauri`) and the iPhone PWA (`vidya-wasm`), so a sealed bundle
//! or relay envelope is byte-compatible across platforms (00-context §18, §11, §9).
//!
//! This module is **pure** (vidya-core's contract: no IO): the 12-byte nonce is
//! supplied by the caller, never generated here. The platform provides randomness —
//! `rand::OsRng` on native (`src-tauri`), `crypto.getRandomValues` in the browser
//! (the PWA) — so vidya-core needs no `rand`/`getrandom` and compiles cleanly to
//! `wasm32`. Given the same key, AAD, nonce and plaintext the output is identical.
//!
//! Envelope: `nonce(12) ‖ ChaCha20-Poly1305(key, plaintext)` with caller-chosen
//! associated data. Two AAD builders are provided so both sides bind the same
//! context: [`associated_data`] for the relay route (method/path/device/epoch) and
//! [`bundle_aad`] for the Drive `.vop` bundle (audience + key version).
//!
//! The session key agreed at join is expanded with an HKDF-style single-block
//! SHA-256 expansion into TWO per-direction keys (`c2s`, `s2c`) so client→server and
//! server→client never share a keystream.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// The AEAD nonce length (bytes). ChaCha20-Poly1305 uses a 96-bit nonce.
pub const NONCE_LEN: usize = 12;

/// Info strings for the two directions (HKDF-Expand `info`). Stable on the wire.
const INFO_C2S: &[u8] = b"vidya/relay/c2s/v1";
const INFO_S2C: &[u8] = b"vidya/relay/s2c/v1";

/// A sealing/opening failure. `Aead` covers a tampered byte, wrong key or wrong
/// associated data — all indistinguishable and all rejected the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealError {
    /// The base64 session key did not decode to 32 bytes.
    BadSessionKey,
    /// The nonce supplied to `seal_with_nonce` was not exactly 12 bytes.
    BadNonce,
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

/// Associated data for the RELAY route (prompts/P05 Step 2). A change to any of
/// method/path/device_id/server_epoch fails `open`.
pub fn associated_data(method: &str, path: &str, device_id: &str, server_epoch: i64) -> Vec<u8> {
    format!("{method}\n{path}\n{device_id}\n{server_epoch}").into_bytes()
}

/// Associated data for a Drive `.vop` BUNDLE (§11). A change to the audience or key
/// version fails the open, so a bundle cannot be relabelled to another folder.
pub fn bundle_aad(audience: &str, key_version: i64) -> Vec<u8> {
    format!("vidya/vop/v1\n{audience}\n{key_version}").into_bytes()
}

/// Associated data for a **device-sealed** Drive `.vop` bundle (format v2). Uploads
/// are sealed with the device's own c2s key (from its session key, held only by that
/// device and the school PC), so opening one proves which device wrote it; the AAD
/// binds the device id, so a bundle can't be moved to another device's folder.
pub fn device_bundle_aad(device_id: &str) -> Vec<u8> {
    format!("vidya/vop/v2\n{device_id}").into_bytes()
}

/// Associated data for the school PC's per-device **down export** on Drive (the
/// device's role-scoped canonical data, sealed with its s2c key). Binds the device id
/// so one device's export can't be passed off as another's.
pub fn device_down_aad(device_id: &str) -> Vec<u8> {
    format!("vidya/down/v1\n{device_id}").into_bytes()
}

// ---------------------------------------------------------------------------
// Drive JOIN, protocol v2 (join-without-LAN for the iPhone PWA, §18).
//
// v1 sealed the response under SHA-256(invite code) and put the code in the
// plaintext request, so anyone able to read `exchange/joins/` could open the
// response (device token, session key). v2:
//   * the web invite carries a fresh 256-bit `secret` in the URL FRAGMENT (never
//     sent to the host, never uploaded); the short code is not used on this route;
//   * the request carries only non-secret fields + `proof` =
//     HMAC(K_proof, transcript), K_proof = HMAC-expand(secret, "…/proof");
//   * the response is sealed under K_resp = HMAC(HMAC-expand(secret, "…/resp"),
//     request_id) with AAD binding invite, request and school.
// A Drive reader without the secret can neither forge a request nor open a response.

const INFO_JOIN_PROOF: &[u8] = b"vidya/join/v2/proof";
const INFO_JOIN_RESP: &[u8] = b"vidya/join/v2/resp";

/// The exact bytes the join proof covers (fields newline-separated, versioned).
pub fn join_v2_transcript(invite_id: &str, request_id: &str, device_name: &str, platform: &str) -> Vec<u8> {
    format!("vidya/join-req/v2\n{invite_id}\n{request_id}\n{device_name}\n{platform}").into_bytes()
}

fn hmac32(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(msg);
    let mut out = [0u8; 32];
    out.copy_from_slice(&Mac::finalize(mac).into_bytes());
    out
}

/// The joining device's proof that it holds the invite secret.
pub fn join_v2_proof(secret: &[u8], transcript: &[u8]) -> [u8; 32] {
    hmac32(&expand(secret, INFO_JOIN_PROOF), transcript)
}

/// Constant-time check of a join proof (school PC side).
pub fn join_v2_verify(secret: &[u8], transcript: &[u8], proof: &[u8]) -> bool {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(&expand(secret, INFO_JOIN_PROOF)).expect("HMAC accepts any key length");
    mac.update(transcript);
    mac.verify_slice(proof).is_ok()
}

/// The per-request key that seals the join response.
pub fn join_v2_response_key(secret: &[u8], request_id: &str) -> [u8; 32] {
    hmac32(&expand(secret, INFO_JOIN_RESP), request_id.as_bytes())
}

/// Associated data binding a join response to its invite, request and school.
pub fn join_v2_aad(invite_id: &str, request_id: &str, school_id: &str) -> Vec<u8> {
    format!("vidya/join-resp/v2\n{invite_id}\n{request_id}\n{school_id}").into_bytes()
}

/// Seal `plaintext` under `key` with `aad` and a CALLER-SUPPLIED 12-byte `nonce` →
/// `nonce(12) ‖ ciphertext+tag`. The caller MUST pass a unique random nonce per
/// message (OsRng on native, WebCrypto in the browser). Pure/deterministic.
pub fn seal_with_nonce(key: &[u8; 32], aad: &[u8], nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, SealError> {
    if nonce.len() != NONCE_LEN {
        return Err(SealError::BadNonce);
    }
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let ct = cipher
        .encrypt(Nonce::from_slice(nonce), Payload { msg: plaintext, aad })
        .map_err(|_| SealError::Aead)?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Open a `nonce(12) ‖ ciphertext+tag` blob under `key` with `aad`.
pub fn open(key: &[u8; 32], aad: &[u8], sealed: &[u8]) -> Result<Vec<u8>, SealError> {
    if sealed.len() < NONCE_LEN + 16 {
        return Err(SealError::TooShort);
    }
    let (nonce_bytes, ct) = sealed.split_at(NONCE_LEN);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    cipher
        .decrypt(Nonce::from_slice(nonce_bytes), Payload { msg: ct, aad })
        .map_err(|_| SealError::Aead)
}

/// Convenience: base64-decode then open (the wire form of a sealed envelope).
pub fn open_b64(key: &[u8; 32], aad: &[u8], sealed_b64: &str) -> Result<Vec<u8>, SealError> {
    let raw = STANDARD.decode(sealed_b64.as_bytes()).map_err(|_| SealError::Aead)?;
    open(key, aad, &raw)
}

/// Convenience: `seal_with_nonce` then base64-encode.
pub fn seal_b64_with_nonce(key: &[u8; 32], aad: &[u8], nonce: &[u8], plaintext: &[u8]) -> Result<String, SealError> {
    Ok(STANDARD.encode(seal_with_nonce(key, aad, nonce, plaintext)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_b64() -> String {
        STANDARD.encode([7u8; 32])
    }
    const NONCE: [u8; 12] = [9u8; 12];

    #[test]
    fn round_trips_both_directions() {
        let k = derive_keys(&key_b64()).unwrap();
        let aad = associated_data("POST", "/v1/sync/push", "dev-1", 1);
        let msg = br#"{"ops":[]}"#;
        let sealed = seal_with_nonce(&k.c2s, &aad, &NONCE, msg).unwrap();
        assert_eq!(open(&k.c2s, &aad, &sealed).unwrap(), msg);
        let resp = br#"{"results":[]}"#;
        let sealed_r = seal_with_nonce(&k.s2c, &aad, &NONCE, resp).unwrap();
        assert_eq!(open(&k.s2c, &aad, &sealed_r).unwrap(), resp);
    }

    #[test]
    fn deterministic_given_the_same_nonce() {
        // The cross-platform interop guarantee: same key+aad+nonce+plaintext →
        // identical bytes on native and in WASM.
        let k = derive_keys(&key_b64()).unwrap();
        let aad = bundle_aad("class:c1", 3);
        let a = seal_with_nonce(&k.c2s, &aad, &NONCE, b"hello").unwrap();
        let b = seal_with_nonce(&k.c2s, &aad, &NONCE, b"hello").unwrap();
        assert_eq!(a, b);
        assert_eq!(&a[..NONCE_LEN], &NONCE, "nonce is the prefix");
    }

    #[test]
    fn directions_have_distinct_keys() {
        let k = derive_keys(&key_b64()).unwrap();
        assert_ne!(k.c2s, k.s2c, "per-direction keys must differ");
    }

    #[test]
    fn tampering_one_byte_is_rejected() {
        let k = derive_keys(&key_b64()).unwrap();
        let aad = associated_data("POST", "/v1/sync/push", "dev-1", 1);
        let mut sealed = seal_with_nonce(&k.c2s, &aad, &NONCE, b"hello").unwrap();
        let last = sealed.len() - 1;
        sealed[last] ^= 0x01;
        assert_eq!(open(&k.c2s, &aad, &sealed), Err(SealError::Aead));
    }

    #[test]
    fn changed_associated_data_is_rejected() {
        let k = derive_keys(&key_b64()).unwrap();
        let aad = associated_data("POST", "/v1/sync/push", "dev-1", 1);
        let sealed = seal_with_nonce(&k.c2s, &aad, &NONCE, b"hello").unwrap();
        for bad in [
            associated_data("POST", "/v1/sync/push", "dev-1", 2),
            associated_data("GET", "/v1/sync/push", "dev-1", 1),
            associated_data("POST", "/v1/sync/pull", "dev-1", 1),
            associated_data("POST", "/v1/sync/push", "dev-2", 1),
            bundle_aad("class:c1", 1),
        ] {
            assert_eq!(open(&k.c2s, &bad, &sealed), Err(SealError::Aead));
        }
    }

    #[test]
    fn wrong_direction_key_is_rejected() {
        let k = derive_keys(&key_b64()).unwrap();
        let aad = associated_data("POST", "/v1/sync/push", "dev-1", 1);
        let sealed = seal_with_nonce(&k.c2s, &aad, &NONCE, b"hello").unwrap();
        assert_eq!(open(&k.s2c, &aad, &sealed), Err(SealError::Aead));
    }

    #[test]
    fn bundle_aad_relabelling_fails() {
        let k = derive_keys(&key_b64()).unwrap();
        let sealed = seal_with_nonce(&k.c2s, &bundle_aad("class:c1", 1), &NONCE, b"ops").unwrap();
        assert_eq!(open(&k.c2s, &bundle_aad("finance", 1), &sealed), Err(SealError::Aead));
        assert_eq!(open(&k.c2s, &bundle_aad("class:c1", 2), &sealed), Err(SealError::Aead));
    }

    #[test]
    fn b64_helpers_round_trip() {
        let k = derive_keys(&key_b64()).unwrap();
        let aad = associated_data("POST", "/v1/sync/push", "dev-1", 3);
        let b64 = seal_b64_with_nonce(&k.c2s, &aad, &NONCE, b"payload").unwrap();
        assert_eq!(open_b64(&k.c2s, &aad, &b64).unwrap(), b"payload");
    }

    #[test]
    fn bad_inputs_error() {
        assert!(matches!(derive_keys(&STANDARD.encode([0u8; 16])), Err(SealError::BadSessionKey)));
        assert!(matches!(derive_keys("not base64!!!"), Err(SealError::BadSessionKey)));
        let k = derive_keys(&key_b64()).unwrap();
        assert_eq!(seal_with_nonce(&k.c2s, b"", &[0u8; 8], b"x"), Err(SealError::BadNonce));
        assert_eq!(open(&k.c2s, b"aad", &[0u8; 10]), Err(SealError::TooShort));
    }

    const SECRET: [u8; 32] = [0x42; 32];

    #[test]
    fn join_v2_proof_verifies_only_with_the_secret_and_exact_transcript() {
        let t = join_v2_transcript("inv-1", "req-1", "iPhone", "web");
        let proof = join_v2_proof(&SECRET, &t);
        assert!(join_v2_verify(&SECRET, &t, &proof));
        // wrong secret
        assert!(!join_v2_verify(&[0x43; 32], &t, &proof));
        // any transcript field changed (request id, device, platform, invite)
        for other in [
            join_v2_transcript("inv-1", "req-2", "iPhone", "web"),
            join_v2_transcript("inv-1", "req-1", "Android", "web"),
            join_v2_transcript("inv-1", "req-1", "iPhone", "android"),
            join_v2_transcript("inv-2", "req-1", "iPhone", "web"),
        ] {
            assert!(!join_v2_verify(&SECRET, &other, &proof));
        }
        // truncated proof
        assert!(!join_v2_verify(&SECRET, &t, &proof[..16]));
    }

    #[test]
    fn join_v2_response_opens_only_with_secret_request_and_binding() {
        let k = join_v2_response_key(&SECRET, "req-1");
        assert_ne!(k, join_v2_response_key(&SECRET, "req-2"), "per-request key");
        assert_ne!(k, join_v2_response_key(&[0x43; 32], "req-1"));
        // The response key is not derivable from what a request file contains.
        assert_ne!(k.to_vec(), join_v2_proof(&SECRET, &join_v2_transcript("inv-1", "req-1", "iPhone", "web")).to_vec());
        let aad = join_v2_aad("inv-1", "req-1", "sch-1");
        let sealed = seal_with_nonce(&k, &aad, &NONCE, br#"{"device_token":"t"}"#).unwrap();
        assert_eq!(open(&k, &aad, &sealed).unwrap(), br#"{"device_token":"t"}"#);
        assert_eq!(open(&join_v2_response_key(&[0x43; 32], "req-1"), &aad, &sealed), Err(SealError::Aead));
        assert_eq!(open(&k, &join_v2_aad("inv-1", "req-2", "sch-1"), &sealed), Err(SealError::Aead));
        assert_eq!(open(&k, &join_v2_aad("inv-1", "req-1", "sch-2"), &sealed), Err(SealError::Aead));
    }

    /// Golden vector: the PWA (WASM) and the school PC must derive identical bytes.
    #[test]
    fn join_v2_golden_vector() {
        let t = join_v2_transcript("inv-1", "req-1", "iPhone", "web");
        assert_eq!(t, b"vidya/join-req/v2\ninv-1\nreq-1\niPhone\nweb".to_vec());
        let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        let proof = hex(&join_v2_proof(&SECRET, &t));
        let key = hex(&join_v2_response_key(&SECRET, "req-1"));
        assert_eq!(proof.len(), 64);
        assert_eq!(key.len(), 64);
        // Cross-checked with Python hmac/hashlib; a change to the derivation must be deliberate.
        assert_eq!(proof, GOLDEN_PROOF);
        assert_eq!(key, GOLDEN_RESP_KEY);
    }
    const GOLDEN_PROOF: &str = "c45b29299a85100333463b9993c2f137a135d0d033f5d8507e94eef80ed41ed3";
    const GOLDEN_RESP_KEY: &str = "3a02159ceadc648a14bda0ed79571d9854dfc295cbdfacc486fff962f93accf6";
}
