//! The licence signing key (ed25519).
//!
//! Standing Rule 6: the private key lives ONLY on the server (env/secret store),
//! never in git, logs or app builds. In `serve` it comes from `LICENCE_SIGNING_KEY`
//! (base64 32-byte seed). In `--dev` it falls back to `.dev-keys/ed25519.key`
//! (gitignored) — the same dev key P03 generated, so the app's `dev.json`
//! `licence_public_key` keeps verifying signatures offline.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::SigningKey;
use rand::RngCore;
use std::path::{Path, PathBuf};

use crate::config::Mode;

fn dev_keys_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(".dev-keys")
}

/// Load the signing key for the given mode.
pub fn load_signing_key(mode: Mode) -> Result<SigningKey, String> {
    if let Ok(seed_b64) = std::env::var("LICENCE_SIGNING_KEY") {
        if !seed_b64.trim().is_empty() {
            let seed = STANDARD
                .decode(seed_b64.trim())
                .map_err(|_| "LICENCE_SIGNING_KEY is not valid base64".to_string())?;
            let seed32: [u8; 32] = seed
                .as_slice()
                .try_into()
                .map_err(|_| "LICENCE_SIGNING_KEY must decode to exactly 32 bytes".to_string())?;
            return Ok(SigningKey::from_bytes(&seed32));
        }
    }
    match mode {
        Mode::Serve => Err("LICENCE_SIGNING_KEY (base64 ed25519 seed) is required in serve mode".into()),
        Mode::Dev => Ok(ensure_dev_key()),
    }
}

/// Load (or generate + persist) the dev signing key under `.dev-keys/`.
fn ensure_dev_key() -> SigningKey {
    let dir = dev_keys_dir();
    let _ = std::fs::create_dir_all(&dir);
    let key_path = dir.join("ed25519.key");
    if let Ok(seed_b64) = std::fs::read_to_string(&key_path) {
        if let Ok(seed) = STANDARD.decode(seed_b64.trim()) {
            if let Ok(seed32) = <[u8; 32]>::try_from(seed.as_slice()) {
                return SigningKey::from_bytes(&seed32);
            }
        }
    }
    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);
    let signing = SigningKey::from_bytes(&seed);
    let _ = std::fs::write(&key_path, STANDARD.encode(seed));
    let _ = std::fs::write(dir.join("ed25519.pub"), STANDARD.encode(signing.verifying_key().to_bytes()));
    signing
}

pub fn public_key_b64(signing: &SigningKey) -> String {
    STANDARD.encode(signing.verifying_key().to_bytes())
}

/// Generate a fresh production ed25519 keypair, returning `(private_seed_b64,
/// public_key_b64)`. The private seed goes to `LICENCE_SIGNING_KEY` (a secret,
/// never committed); the public key goes into `src-tauri/build-config/release.json`.
pub fn generate_seed_b64() -> (String, String) {
    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);
    let signing = SigningKey::from_bytes(&seed);
    (STANDARD.encode(seed), public_key_b64(&signing))
}

/// Generate a random 32-byte symmetric key (base64) for `LICENCE_CODE_ENC_KEY`.
pub fn generate_enc_key_b64() -> String {
    let mut k = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut k);
    STANDARD.encode(k)
}
