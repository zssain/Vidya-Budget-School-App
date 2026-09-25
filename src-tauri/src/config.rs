//! Build-time configuration (docs/00-SYSTEM-CONTEXT.md §10, prompts/P12 Step 6/8).
//!
//! The licence public key(s), the optional `RELAY_URL` and the Google OAuth client
//! ids come from ONE build-time config file — never hard-coded anywhere else. The
//! file for the active build profile is embedded at compile time with
//! `include_str!`:
//!
//! * debug builds → `build-config/dev.json`
//! * release builds → `build-config/release.json` (never committed; created from
//!   `release.json.example`). `build.rs` fails a release build if a required value
//!   is empty or if it still uses the dev licence key.
//!
//! v2 (Phase 12): licences are verified **offline** — there is no `LICENCE_API`.
//! `licence_public_keys` is a LIST so the owner can rotate the signing key (e.g. a
//! lost laptop) while previously issued licences keep verifying (see
//! `tools/licence-maker/README.md`). `relay_url` is optional (the off-by-default
//! "Instant sync" module, §14).

use serde::Deserialize;
use std::sync::OnceLock;

/// The raw config JSON for the active profile, embedded at compile time.
#[cfg(debug_assertions)]
const RAW: &str = include_str!("../build-config/dev.json");
#[cfg(not(debug_assertions))]
const RAW: &str = include_str!("../build-config/release.json");

/// The build-time values (docs/00-SYSTEM-CONTEXT.md §10). No other file may
/// contain these values.
#[derive(Debug, Clone, Deserialize)]
pub struct BuildConfig {
    /// STANDARD base64 of one or more 32-byte ed25519 licence public keys. A
    /// licence verifies if it is signed by ANY of these keys, so the owner can add
    /// a new key (lost-laptop rotation) without invalidating old licences.
    pub licence_public_keys: Vec<String>,
    /// Base URL of the Vidya relay — OPTIONAL (the off-by-default "Instant sync"
    /// module, §14). Empty when the relay add-on is not shipped.
    #[serde(default)]
    pub relay_url: String,
    /// Google OAuth desktop client id (used from Phase 6).
    pub google_client_id_desktop: String,
    /// Google OAuth Android client id (used from Phase 6).
    pub google_client_id_android: String,
}

/// The parsed config for this build. Panics only if the embedded JSON is
/// malformed — a build-authoring error caught the first time it is read.
pub fn get() -> &'static BuildConfig {
    static CFG: OnceLock<BuildConfig> = OnceLock::new();
    CFG.get_or_init(|| {
        serde_json::from_str(RAW).expect("build-config JSON is malformed (build authoring error)")
    })
}

/// Decode every configured licence public key into the 32 raw bytes vidya-core
/// expects. Malformed / non-32-byte entries are skipped. May be empty in a dev
/// build before a keypair has been generated.
pub fn licence_public_keys() -> Vec<[u8; 32]> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    get()
        .licence_public_keys
        .iter()
        .filter_map(|s| {
            let raw = STANDARD.decode(s.as_bytes()).ok()?;
            raw.try_into().ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_config_parses() {
        // The active profile's JSON must always deserialize.
        let cfg = get();
        // dev.json ships at least one licence public key.
        assert!(!cfg.licence_public_keys.is_empty(), "dev licence_public_keys must be set");
    }

    #[test]
    fn public_keys_are_32_bytes() {
        // dev ships one working 32-byte ed25519 key; all decoded keys are 32 bytes.
        let keys = licence_public_keys();
        assert!(!keys.is_empty(), "dev build must decode at least one public key");
        for k in keys {
            assert_eq!(k.len(), 32);
        }
    }
}
