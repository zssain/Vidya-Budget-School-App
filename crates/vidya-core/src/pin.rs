//! PIN hashing & lockout — the shared pure logic so the iPhone PWA hashes, verifies
//! and locks out PINs IDENTICALLY to the desktop/Android app (§9, §18).
//! Argon2id, m = 19 MiB (19456 KiB), t = 2, p = 1, version 0x13.
//!
//! Pure (vidya-core's contract): the 16-byte salt is a CALLER parameter — supplied
//! by `rand::OsRng` on native and `crypto.getRandomValues` in the browser — so
//! vidya-core needs no `rand`/`getrandom` and compiles to wasm32. Verification is
//! deterministic. (Argon2id at 19 MiB runs in WASM; it is a one-off unlock cost.)

use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};

/// The single Argon2id configuration used for PINs on every platform (§9).
fn argon2() -> Argon2<'static> {
    let params = Params::new(19_456, 2, 1, None).expect("argon2 params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

/// Hash a PIN with a CALLER-SUPPLIED 16-byte random salt → a PHC string (embeds the
/// salt + params). The caller MUST pass a fresh random salt each time.
pub fn hash_pin_with_salt(pin: &str, salt: &[u8; 16]) -> Result<String, String> {
    let salt = SaltString::encode_b64(salt).map_err(|e| e.to_string())?;
    let hash = argon2().hash_password(pin.as_bytes(), &salt).map_err(|e| e.to_string())?;
    Ok(hash.to_string())
}

/// Verify a PIN against a stored PHC string. Deterministic; no randomness.
pub fn verify_pin(pin: &str, phc: &str) -> bool {
    match PasswordHash::new(phc) {
        Ok(parsed) => argon2().verify_password(pin.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    }
}

/// Lockout wait after `fail_count` consecutive wrong PINs (§9). `None` before the
/// 5th failure; then 30 s doubling each further failure, capped at 1 hour.
pub fn lockout_seconds(fail_count: u32) -> Option<u64> {
    if fail_count < 5 {
        return None;
    }
    let over = fail_count - 5;
    let secs = 30u64.checked_shl(over).unwrap_or(3600).min(3600);
    Some(secs)
}

/// Tries remaining before the account locks.
pub fn remaining_before_lock(fail_count: u32) -> u32 {
    5u32.saturating_sub(fail_count)
}

/// Whether the account is currently locked (times in epoch ms).
pub fn is_locked(locked_until_ms: Option<i64>, now_ms: i64) -> bool {
    matches!(locked_until_ms, Some(until) if now_ms < until)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SALT: [u8; 16] = [3u8; 16];

    #[test]
    fn hash_and_verify() {
        let phc = hash_pin_with_salt("1234", &SALT).unwrap();
        assert!(verify_pin("1234", &phc));
        assert!(!verify_pin("4321", &phc));
        assert!(!verify_pin("1234", "not-a-hash"));
        assert!(phc.starts_with("$argon2id$"));
        // v=19 (0x13) and the params travel in the PHC string, so any platform that
        // parses it verifies with the identical configuration.
        assert!(phc.contains("v=19"));
        assert!(phc.contains("m=19456,t=2,p=1"));
    }

    #[test]
    fn same_salt_is_deterministic() {
        // The cross-platform guarantee: same pin+salt → identical PHC on any target.
        assert_eq!(hash_pin_with_salt("1234", &SALT).unwrap(), hash_pin_with_salt("1234", &SALT).unwrap());
    }

    #[test]
    fn lockout_curve() {
        assert_eq!(lockout_seconds(4), None);
        assert_eq!(lockout_seconds(5), Some(30));
        assert_eq!(lockout_seconds(6), Some(60));
        assert_eq!(lockout_seconds(7), Some(120));
        assert_eq!(lockout_seconds(11), Some(1920));
        assert_eq!(lockout_seconds(12), Some(3600)); // capped at 1 hour
        assert_eq!(lockout_seconds(100), Some(3600));
    }

    #[test]
    fn remaining_and_locked() {
        assert_eq!(remaining_before_lock(0), 5);
        assert_eq!(remaining_before_lock(4), 1);
        assert_eq!(remaining_before_lock(5), 0);
        assert!(is_locked(Some(2000), 1000));
        assert!(!is_locked(Some(1000), 2000));
        assert!(!is_locked(None, 1000));
    }
}
