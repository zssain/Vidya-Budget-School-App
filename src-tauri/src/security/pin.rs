//! PIN unlock (docs/00-SYSTEM-CONTEXT.md §9, prompts/P02 Step 6). Argon2id
//! (m = 19 MiB, t = 2, p = 1). Lockout: 5 wrong → 30 s, doubling, max 1 h.

use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use rand::RngCore;

fn argon2() -> Argon2<'static> {
    // m = 19 MiB = 19456 KiB, t = 2, p = 1.
    let params = Params::new(19_456, 2, 1, None).expect("argon2 params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

/// Hash a PIN → a PHC string (embeds salt + params).
pub fn hash_pin(pin: &str) -> Result<String, String> {
    let mut salt = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    let salt = SaltString::encode_b64(&salt).map_err(|e| e.to_string())?;
    let hash = argon2().hash_password(pin.as_bytes(), &salt).map_err(|e| e.to_string())?;
    Ok(hash.to_string())
}

/// Verify a PIN against a stored PHC string.
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

    #[test]
    fn hash_and_verify() {
        let phc = hash_pin("1234").unwrap();
        assert!(verify_pin("1234", &phc));
        assert!(!verify_pin("4321", &phc));
        assert!(!verify_pin("1234", "not-a-hash"));
        assert!(phc.starts_with("$argon2id$"));
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
