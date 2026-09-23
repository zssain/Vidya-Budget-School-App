//! Recovery key + backup key (docs/00-SYSTEM-CONTEXT.md §9, prompts/P02 Step 6).
//! Recovery key: 30 Crockford base32 chars from 150 random bits, 6 groups of 5.
//! Backup key: Argon2id(recovery key, salt, m = 64 MiB, t = 3, p = 1) → 32 bytes.

use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn extract_bits(bytes: &[u8], start: usize, n: usize) -> u32 {
    let mut v = 0u32;
    for i in 0..n {
        let bit = start + i;
        let byte = bytes[bit / 8];
        let b = (byte >> (7 - (bit % 8))) & 1;
        v = (v << 1) | b as u32;
    }
    v
}

/// Generate a recovery key: `XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX` (6×5 Crockford).
pub fn generate_recovery_key() -> String {
    let mut bytes = [0u8; 19]; // 152 bits; we use the first 150
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    encode_recovery_key(&bytes)
}

fn encode_recovery_key(bytes: &[u8; 19]) -> String {
    let mut groups: Vec<String> = Vec::with_capacity(6);
    for g in 0..6 {
        let mut group = String::with_capacity(5);
        for c in 0..5 {
            let idx = g * 5 + c;
            let bits = extract_bits(bytes, idx * 5, 5) as usize;
            group.push(ALPHABET[bits] as char);
        }
        groups.push(group);
    }
    groups.join("-")
}

/// Canonicalise user input: strip separators, upper-case, map Crockford aliases
/// (I/L → 1, O → 0). Returns the 30-char body (no dashes).
pub fn normalize(input: &str) -> String {
    let mut out = String::with_capacity(30);
    for ch in input.chars() {
        let c = ch.to_ascii_uppercase();
        let mapped = match c {
            'I' | 'L' => '1',
            'O' => '0',
            '-' | ' ' => continue,
            other => other,
        };
        out.push(mapped);
    }
    out
}

/// Derive the 32-byte backup key from the recovery key and the school salt.
pub fn derive_backup_key(recovery_normalized: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    // m = 64 MiB = 65536 KiB, t = 3, p = 1.
    let params = Params::new(65_536, 3, 1, Some(32)).map_err(|e| e.to_string())?;
    let a = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    a.hash_password_into(recovery_normalized.as_bytes(), salt, &mut out)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_key_shape() {
        let k = generate_recovery_key();
        let groups: Vec<&str> = k.split('-').collect();
        assert_eq!(groups.len(), 6);
        for g in &groups {
            assert_eq!(g.len(), 5);
            assert!(g.bytes().all(|b| ALPHABET.contains(&b)));
        }
        assert_eq!(k.len(), 35); // 30 chars + 5 dashes
    }

    #[test]
    fn normalize_maps_aliases() {
        assert_eq!(normalize("iLo-1 0"), "11010"); // I→1, L→1, O→0
        assert_eq!(normalize("abcde-fghjk"), "ABCDEFGHJK"); // no I/L/O → unchanged
    }

    #[test]
    fn backup_key_is_deterministic_per_salt() {
        let salt = [7u8; 16];
        let rk = normalize("ABCDE-FGHJK-MNPQR-STVWX-YZ012-34567");
        let k1 = derive_backup_key(&rk, &salt).unwrap();
        let k2 = derive_backup_key(&rk, &salt).unwrap();
        assert_eq!(k1, k2);
        // different salt → different key
        let k3 = derive_backup_key(&rk, &[9u8; 16]).unwrap();
        assert_ne!(k1, k3);
        // different recovery key → different key
        let k4 = derive_backup_key(&normalize("ZZZZZ-ZZZZZ-ZZZZZ-ZZZZZ-ZZZZZ-ZZZZZ"), &salt).unwrap();
        assert_ne!(k1, k4);
    }
}
