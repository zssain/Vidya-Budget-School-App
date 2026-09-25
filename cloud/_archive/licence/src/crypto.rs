//! Pure crypto helpers for the licence service (no IO, no clock — timestamps are
//! passed in). Kept in one place so the recipes stay byte-identical to the app
//! and to `cloud/relay` where they must interoperate.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Crockford base32 alphabet (excludes I, L, O, U) — docs §9 / P03 Step 2.
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a `VIDYA-XXXX-XXXX-XXXX` activation code from Crockford base32.
pub fn generate_code() -> String {
    let mut bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let groups: Vec<String> = (0..3)
        .map(|g| {
            (0..4)
                .map(|c| CROCKFORD[(bytes[g * 4 + c] % 32) as usize] as char)
                .collect::<String>()
        })
        .collect();
    format!("VIDYA-{}-{}-{}", groups[0], groups[1], groups[2])
}

/// Canonicalise a user-typed code: drop separators/spaces, upper-case, map the
/// Crockford visual aliases (I/L → 1, O → 0) so honest typos still resolve.
pub fn normalize_code(input: &str) -> String {
    let mut out = String::with_capacity(20);
    for ch in input.chars() {
        let c = ch.to_ascii_uppercase();
        match c {
            'I' | 'L' => out.push('1'),
            'O' => out.push('0'),
            '-' | ' ' | '\t' => {}
            other => out.push(other),
        }
    }
    out
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let d = h.finalize();
    let mut s = String::with_capacity(64);
    for b in d {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Look-up hash for an activation code (SHA-256 of the normalised code).
pub fn hash_code(code: &str) -> String {
    sha256_hex(normalize_code(code).as_bytes())
}

type HmacSha256 = Hmac<Sha256>;

pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

/// A school's relay secret: `base64(HMAC-SHA256(shared_key, school_id))`.
/// `cloud/relay` recomputes this exact value to authenticate a tunnel (stateless
/// relay) — keep the recipe byte-identical in both services (P05 §10).
pub fn relay_secret(shared_key: &[u8], school_id: &str) -> String {
    STANDARD.encode(hmac_sha256(shared_key, school_id.as_bytes()))
}

/// The message a self-service transfer proof is computed over:
/// `licence_id ‖ '\n' ‖ new_machine_id ‖ '\n' ‖ timestamp`.
pub fn transfer_proof_message(licence_id: &str, new_machine_id: &str, ts: &str) -> Vec<u8> {
    format!("{licence_id}\n{new_machine_id}\n{ts}").into_bytes()
}

/// Compute the expected transfer proof `base64(HMAC-SHA256(verifier_key, message))`.
pub fn transfer_proof(verifier_key: &[u8], licence_id: &str, new_machine_id: &str, ts: &str) -> String {
    STANDARD.encode(hmac_sha256(
        verifier_key,
        &transfer_proof_message(licence_id, new_machine_id, ts),
    ))
}

/// Constant-time equality for two byte slices (length is not secret here).
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Encrypt bytes at rest: `base64(nonce(12) ‖ ChaCha20-Poly1305(key, plaintext))`.
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> String {
    let cipher = ChaCha20Poly1305::new(key.into());
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher.encrypt(nonce, plaintext).expect("chacha encrypt");
    let mut out = Vec::with_capacity(12 + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    STANDARD.encode(out)
}

/// Reverse [`encrypt`]. Returns `None` on any decode/auth failure.
pub fn decrypt(key: &[u8; 32], b64: &str) -> Option<Vec<u8>> {
    let raw = STANDARD.decode(b64).ok()?;
    if raw.len() < 12 {
        return None;
    }
    let (nonce_bytes, ct) = raw.split_at(12);
    let cipher = ChaCha20Poly1305::new(key.into());
    cipher.decrypt(Nonce::from_slice(nonce_bytes), ct).ok()
}

/// Decode a base64 32-byte key (the app's recovery-transfer verifier). `None` if
/// it is not exactly 32 bytes.
pub fn decode_key32(b64: &str) -> Option<[u8; 32]> {
    let bytes = STANDARD.decode(b64.trim()).ok()?;
    bytes.as_slice().try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_shape_is_valid_crockford() {
        let code = generate_code();
        assert!(code.starts_with("VIDYA-"));
        let groups: Vec<&str> = code.split('-').collect();
        assert_eq!(groups.len(), 4);
        for g in &groups[1..] {
            assert_eq!(g.len(), 4);
            assert!(g.bytes().all(|b| CROCKFORD.contains(&b)), "only Crockford chars");
        }
        for bad in ['I', 'L', 'O', 'U'] {
            assert!(!code[6..].contains(bad), "must exclude {bad}");
        }
    }

    #[test]
    fn normalize_maps_visual_aliases_and_strips_separators() {
        // I/L → 1, O → 0 (the fixed 'VIDYA' prefix normalises the same way every
        // time, so storage and lookup stay consistent).
        assert_eq!(normalize_code("vidya-abcd 1234\tijko"), "V1DYAABCD12341JK0");
        // Same code with different separators hashes identically.
        assert_eq!(hash_code("VIDYA-ABCD-1234-5678"), hash_code("vidya abcd 1234 5678"));
    }

    #[test]
    fn relay_secret_is_stable_hmac_of_school_id() {
        let key = b"shared-key";
        let a = relay_secret(key, "sch_abc");
        assert_eq!(a, relay_secret(key, "sch_abc"), "stable for the same inputs");
        assert_ne!(a, relay_secret(key, "sch_xyz"), "bound to school_id");
        assert_ne!(a, relay_secret(b"other-key", "sch_abc"), "bound to the shared key");
        assert_eq!(STANDARD.decode(a).unwrap().len(), 32);
    }

    #[test]
    fn encrypt_roundtrips_and_rejects_tampering() {
        let key = [7u8; 32];
        let ct = encrypt(&key, b"VIDYA-ABCD-1234-5678");
        assert_eq!(decrypt(&key, &ct).unwrap(), b"VIDYA-ABCD-1234-5678");
        // Wrong key fails.
        assert!(decrypt(&[8u8; 32], &ct).is_none());
        // Flip a byte → auth failure.
        let mut raw = STANDARD.decode(&ct).unwrap();
        let last = raw.len() - 1;
        raw[last] ^= 1;
        assert!(decrypt(&key, &STANDARD.encode(raw)).is_none());
    }

    #[test]
    fn transfer_proof_is_deterministic_and_bound() {
        let vk = [3u8; 32];
        let p = transfer_proof(&vk, "lic_1", "machine_B", "2026-09-24T10:00:00Z");
        assert_eq!(p, transfer_proof(&vk, "lic_1", "machine_B", "2026-09-24T10:00:00Z"));
        assert_ne!(p, transfer_proof(&vk, "lic_1", "machine_C", "2026-09-24T10:00:00Z"));
        assert_ne!(p, transfer_proof(&[4u8; 32], "lic_1", "machine_B", "2026-09-24T10:00:00Z"));
        assert!(ct_eq(p.as_bytes(), p.as_bytes()));
        assert!(!ct_eq(b"abc", b"abd"));
    }
}
