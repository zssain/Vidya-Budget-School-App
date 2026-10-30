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
    /// The public iPhone web-app (PWA) origin used in web invite links, e.g.
    /// `https://app.example.org` (no trailing slash). OPTIONAL: empty → no web invite
    /// is offered. Must be HTTPS, a stable production host (not a per-deployment
    /// preview URL), and listed in the Web OAuth client's JavaScript origins.
    #[serde(default)]
    pub pwa_origin: String,
}

/// Validate a PWA origin: `Ok(())` for a usable public origin. Rejects placeholders,
/// paths/trailing slashes, non-HTTPS (except localhost in dev builds) and
/// per-deployment Vercel preview hosts (`<project>-<hash>-<team>.vercel.app`), which
/// change on every deploy and sit behind Vercel login.
pub fn validate_pwa_origin(origin: &str, allow_localhost: bool) -> Result<(), String> {
    if origin.is_empty() || origin.contains("REPLACE") {
        return Err("pwa_origin is not set".into());
    }
    let host = if let Some(h) = origin.strip_prefix("https://") {
        h
    } else if let Some(h) = origin.strip_prefix("http://") {
        if !(allow_localhost && (h.starts_with("localhost") || h.starts_with("127.0.0.1"))) {
            return Err("pwa_origin must be https://".into());
        }
        h
    } else {
        return Err("pwa_origin must be https://".into());
    };
    if host.is_empty() || host.contains('/') || host.contains('#') || host.contains('?') {
        return Err("pwa_origin must be a bare origin (no path or trailing slash)".into());
    }
    if let Some(sub) = host.strip_suffix(".vercel.app") {
        // A preview/deployment URL has ≥3 dash-separated parts incl. a random hash.
        if sub.split('-').count() >= 3 {
            return Err("pwa_origin is a per-deployment Vercel preview URL; use the production domain".into());
        }
    }
    Ok(())
}

/// The PWA origin to put in web invites, or `None` when this build has none (or it
/// is invalid). Debug builds fall back to the local `npm run dev:web` server.
pub fn pwa_origin() -> Option<String> {
    let configured = get().pwa_origin.trim_end_matches('/').to_string();
    if !configured.is_empty() {
        return validate_pwa_origin(&configured, cfg!(debug_assertions)).ok().map(|_| configured);
    }
    cfg!(debug_assertions).then(|| "http://localhost:5273".to_string())
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
    fn pwa_origin_validation() {
        assert!(validate_pwa_origin("https://app.example.org", false).is_ok());
        assert!(validate_pwa_origin("https://vidya.vercel.app", false).is_ok());
        assert!(validate_pwa_origin("https://vidyaschoolmanagement-1d29vhmfi-zssains-projects.vercel.app", false).is_err());
        assert!(validate_pwa_origin("http://app.example.org", false).is_err());
        assert!(validate_pwa_origin("http://localhost:5273", false).is_err());
        assert!(validate_pwa_origin("http://localhost:5273", true).is_ok());
        assert!(validate_pwa_origin("https://app.example.org/", false).is_err());
        assert!(validate_pwa_origin("https://app.example.org/join", false).is_err());
        assert!(validate_pwa_origin("", false).is_err());
        assert!(validate_pwa_origin("REPLACE-with-pwa-origin", false).is_err());
    }

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
