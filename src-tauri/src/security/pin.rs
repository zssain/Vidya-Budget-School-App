//! PIN unlock — a thin **platform wrapper** over the shared, pure `vidya_core::pin`
//! logic (docs/00-SYSTEM-CONTEXT.md §9, §18, prompts/P02 Step 6).
//!
//! The Argon2id configuration (m = 19 MiB, t = 2, p = 1), verification and the
//! lockout curve now live in `vidya_core::pin` so the iPhone PWA hashes/verifies/
//! locks out PINs IDENTICALLY. vidya-core is pure and does not generate the salt;
//! this wrapper adds the only platform bit — a fresh **OS-random** 16-byte salt —
//! and delegates. (In the browser the PWA supplies a `crypto.getRandomValues` salt.)

use rand::RngCore;

// Re-export the shared pure logic so existing callers (`security::pin::…`) are unchanged.
pub use vidya_core::pin::{is_locked, lockout_seconds, remaining_before_lock, verify_pin};

/// Hash a PIN → a PHC string (embeds salt + params), using a fresh OS-random salt.
pub fn hash_pin(pin: &str) -> Result<String, String> {
    let mut salt = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    vidya_core::pin::hash_pin_with_salt(pin, &salt)
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
    fn hash_uses_a_fresh_random_salt() {
        // The wrapper's contribution: two hashes of the same PIN differ (distinct
        // OS-random salts), yet both verify.
        let a = hash_pin("1234").unwrap();
        let b = hash_pin("1234").unwrap();
        assert_ne!(a, b, "fresh salt per hash");
        assert!(verify_pin("1234", &a) && verify_pin("1234", &b));
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
