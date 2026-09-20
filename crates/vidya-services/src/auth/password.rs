//! Argon2id password hashing (m=19456 KiB, t=2, p=1), with a constant-time
//! dummy verify so unknown usernames take similar time to real ones.

use std::sync::OnceLock;

use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use argon2::{Algorithm, Argon2, Params, Version};

use crate::env::Random;
use crate::error::ServiceError;

/// Argon2id configured with the Vidya parameters.
fn argon2() -> Result<Argon2<'static>, ServiceError> {
    let params =
        Params::new(19_456, 2, 1, None).map_err(|e| ServiceError::internal(format!("argon2 params: {e}")))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

/// Hashes a password with a fresh 16-byte random salt, returning a PHC string.
pub fn hash_password(random: &dyn Random, password: &str) -> Result<String, ServiceError> {
    let mut salt = [0u8; 16];
    random.fill(&mut salt);
    let hash = argon2()?
        .hash_password_with_salt(password.as_bytes(), &salt)
        .map_err(|e| ServiceError::internal(format!("argon2 hash: {e}")))?;
    Ok(hash.to_string())
}

/// Verifies a password against a stored PHC hash. Returns false on any error
/// (unparseable hash, wrong password) so callers cannot distinguish causes.
pub fn verify_password(hash: &str, password: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// A hash computed once at first use, so `dummy_verify` costs about the same as
/// a real verify.
fn dummy_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| {
        argon2()
            .expect("argon2 params")
            .hash_password_with_salt(b"vidya-dummy-password", &[0u8; 16])
            .expect("argon2 hash")
            .to_string()
    })
}

/// Runs a verify against a fixed dummy hash to equalise timing for unknown
/// usernames. The result is intentionally discarded.
pub fn dummy_verify(password: &str) {
    let _ = verify_password(dummy_hash(), password);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::SeededRandom;

    #[test]
    fn hash_verifies_and_rejects() {
        let random = SeededRandom::new(1);
        let hash = hash_password(&random, "vidya123").expect("hash");
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password(&hash, "vidya123"));
        assert!(!verify_password(&hash, "wrong"));
    }

    #[test]
    fn same_password_hashes_differ() {
        let random = SeededRandom::new(2);
        let a = hash_password(&random, "vidya123").expect("hash a");
        let b = hash_password(&random, "vidya123").expect("hash b");
        assert_ne!(a, b);
    }
}
