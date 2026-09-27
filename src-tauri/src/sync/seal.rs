//! Relay/bundle sealing — a thin **platform wrapper** over the shared, pure
//! `vidya_core::seal` primitives (00-context §18, prompts/P05 Step 2).
//!
//! The byte-identical crypto — AEAD (`ChaCha20-Poly1305`), the HKDF key expansion,
//! and the associated-data builders — now lives in `vidya_core::seal` so the
//! desktop/Android app AND the iPhone PWA (`vidya-wasm`) seal the exact same way.
//! vidya-core is pure (no IO), so it does NOT generate the nonce; this wrapper adds
//! the only platform bit — a fresh **OS-random** 12-byte nonce via `rand::OsRng` —
//! and delegates the rest. (In the browser the PWA supplies a `crypto.getRandomValues`
//! nonce instead; same primitive underneath.)
//!
//! Envelope: `nonce(12) ‖ ChaCha20-Poly1305(dir_key, plaintext)` with associated
//! data binding method/path/device/epoch (relay) or audience/key-version (bundle).

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rand::RngCore;

// Re-export the shared primitives so existing callers (`sync::seal::…`) are unchanged.
pub use vidya_core::seal::{
    associated_data, bundle_aad, derive_keys, open, open_b64, DirectionKeys, SealError, NONCE_LEN,
};

/// Seal `plaintext` under `key` with `aad` using a fresh OS-random nonce →
/// `nonce(12) ‖ ciphertext+tag`. Delegates the crypto to `vidya_core::seal`.
pub fn seal(key: &[u8; 32], aad: &[u8], plaintext: &[u8]) -> Vec<u8> {
    let mut nonce = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    vidya_core::seal::seal_with_nonce(key, aad, &nonce, plaintext)
        .expect("a 12-byte nonce and a valid key make seal infallible")
}

/// Convenience: seal to base64 (the wire form used by the sealed envelope).
pub fn seal_b64(key: &[u8; 32], aad: &[u8], plaintext: &[u8]) -> String {
    STANDARD.encode(seal(key, aad, plaintext))
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
    fn seal_uses_a_fresh_random_nonce_each_time() {
        // The wrapper's contribution over the pure core: two seals of the same
        // plaintext differ (distinct OS-random nonces), yet both open.
        let k = derive_keys(&key()).unwrap();
        let aad = associated_data("POST", "/v1/sync/push", "dev-1", 1);
        let a = seal(&k.c2s, &aad, b"hello");
        let b = seal(&k.c2s, &aad, b"hello");
        assert_ne!(a, b, "fresh nonce per seal");
        assert_eq!(open(&k.c2s, &aad, &a).unwrap(), b"hello");
        assert_eq!(open(&k.c2s, &aad, &b).unwrap(), b"hello");
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
        let mut sealed2 = seal(&k.c2s, &aad, b"hello");
        sealed2[13] ^= 0x80;
        assert_eq!(open(&k.c2s, &aad, &sealed2), Err(SealError::Aead));
    }

    #[test]
    fn changed_associated_data_is_rejected() {
        let k = derive_keys(&key()).unwrap();
        let aad = associated_data("POST", "/v1/sync/push", "dev-1", 1);
        let sealed = seal(&k.c2s, &aad, b"hello");
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
        assert!(matches!(derive_keys(&STANDARD.encode([0u8; 16])), Err(SealError::BadSessionKey)));
        assert!(matches!(derive_keys("not base64!!!"), Err(SealError::BadSessionKey)));
    }

    #[test]
    fn too_short_blob_errors() {
        let k = derive_keys(&key()).unwrap();
        assert_eq!(open(&k.c2s, b"aad", &[0u8; 10]), Err(SealError::TooShort));
    }
}
