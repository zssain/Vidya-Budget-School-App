//! Build-time configuration (docs/00-SYSTEM-CONTEXT.md §10, prompts/P03 Step 1).
//!
//! `LICENCE_API`, `RELAY_URL`, the licence public key and the Google OAuth client
//! ids come from ONE build-time config file — never hard-coded anywhere else. The
//! file for the active build profile is embedded at compile time with
//! `include_str!`:
//!
//! * debug builds → `build-config/dev.json`
//! * release builds → `build-config/release.json` (never committed; created from
//!   `release.json.example`). `build.rs` fails a release build if any value is
//!   empty, so a release binary can never ship blank config.

use serde::Deserialize;
use std::sync::OnceLock;

/// The raw config JSON for the active profile, embedded at compile time.
#[cfg(debug_assertions)]
const RAW: &str = include_str!("../build-config/dev.json");
#[cfg(not(debug_assertions))]
const RAW: &str = include_str!("../build-config/release.json");

/// The five build-time values (docs/00-SYSTEM-CONTEXT.md §10). No other file may
/// contain these values.
#[derive(Debug, Clone, Deserialize)]
pub struct BuildConfig {
    /// Base URL of the licence service, e.g. `http://127.0.0.1:8787` in dev.
    pub licence_api: String,
    /// STANDARD base64 of the licence service's 32-byte ed25519 public key.
    pub licence_public_key: String,
    /// Base URL of the Vidya relay (used from Phase 5; unused in Phase 3).
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

/// Decode the licence public key into the 32 raw bytes vidya-core expects.
///
/// Returns `None` when the key is absent (an empty dev value before the dev
/// licence service has been run) or not a valid 32-byte base64 key.
pub fn licence_public_key() -> Option<[u8; 32]> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    let raw = STANDARD.decode(get().licence_public_key.as_bytes()).ok()?;
    raw.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_config_parses() {
        // The active profile's JSON must always deserialize.
        let cfg = get();
        // dev.json ships a working local licence API.
        assert!(!cfg.licence_api.is_empty(), "dev licence_api must be set");
    }

    #[test]
    fn public_key_is_none_or_32_bytes() {
        // Empty (before `cloud/licence` has generated a key) → None; otherwise a
        // valid 32-byte ed25519 public key.
        match licence_public_key() {
            None => {}
            Some(k) => assert_eq!(k.len(), 32),
        }
    }
}
