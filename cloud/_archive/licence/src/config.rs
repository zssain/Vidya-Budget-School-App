//! Runtime configuration for the licence service (prompts/P10 Step 1 & Step 7).
//!
//! Everything the service needs comes from ONE place: environment variables (with
//! dev defaults). Secrets (signing key, code-encryption key, relay shared key,
//! admin bootstrap) are NEVER in git — in `--dev` they fall back to values under
//! `.dev-keys/` (gitignored) so local work needs no setup; in `serve` they are
//! required and the process refuses to start with a clear message if one is missing.
//!
//! Owner-supplied business text (company name, support contact, UPI id, prices,
//! terms URL) are placeholders per Standing Rule 4 — the code never invents them.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use std::path::PathBuf;

/// Where the process reads its secrets and defaults from.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Local development: dev keys under `.dev-keys/`, permissive cookies.
    Dev,
    /// Production: all secrets required from env; secure cookies.
    Serve,
}

#[derive(Clone)]
pub struct Config {
    pub mode: Mode,
    pub bind: String,
    pub db_path: PathBuf,
    pub base_url: String,

    // Secrets (bytes never logged).
    pub relay_shared_key: Vec<u8>,
    pub code_enc_key: [u8; 32],

    // Sessions / cookies (Step 5).
    pub session_ttl_hours: i64,
    pub cookie_secure: bool,

    // Admin bootstrap (Step 7).
    pub admin_email: Option<String>,
    pub admin_password: Option<String>,
    pub admin_ip_allowlist: Vec<String>,

    // Downloads (Step 6).
    pub releases_json: Option<PathBuf>,

    // Owner-supplied business text — placeholders until filled (Rule 4).
    pub company_name: String,
    pub support_email: String,
    pub support_phone: String,
    pub upi_id: String,
    pub upi_qr_path: Option<PathBuf>,
    pub price_text: String,
    pub terms_url: String,
}

/// Dev default for the relay shared key — matches `cloud/relay`'s dev default so
/// `--dev` works out of the box. NEVER used in `serve` (env is required there).
const DEV_RELAY_SHARED_KEY: &str = "vidya-dev-relay-shared-key-change-me";

fn env_opt(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

fn env_or(key: &str, default: &str) -> String {
    env_opt(key).unwrap_or_else(|| default.to_string())
}

/// Decode a base64 secret from env into exactly `N` bytes.
fn env_key32(key: &str) -> Result<Option<[u8; 32]>, String> {
    match env_opt(key) {
        None => Ok(None),
        Some(v) => {
            let bytes = STANDARD
                .decode(v.trim())
                .map_err(|_| format!("{key} is not valid base64"))?;
            let arr: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| format!("{key} must decode to exactly 32 bytes"))?;
            Ok(Some(arr))
        }
    }
}

impl Config {
    pub fn from_env(mode: Mode) -> Result<Config, String> {
        let dev = mode == Mode::Dev;

        let relay_shared_key = match env_opt("RELAY_SHARED_KEY") {
            Some(v) => v.into_bytes(),
            None if dev => DEV_RELAY_SHARED_KEY.as_bytes().to_vec(),
            None => return Err("RELAY_SHARED_KEY is required in serve mode".into()),
        };

        let code_enc_key = match env_key32("LICENCE_CODE_ENC_KEY")? {
            Some(k) => k,
            None if dev => {
                // Stable dev key so codes stay decryptable across restarts.
                use sha2::{Digest, Sha256};
                let mut h = Sha256::new();
                h.update(b"vidya-dev-code-enc-key");
                h.finalize().into()
            }
            None => return Err("LICENCE_CODE_ENC_KEY (base64, 32 bytes) is required in serve mode".into()),
        };

        // Store location. `LICENCE_DB` (a full path) wins; else `LICENCE_DATA_DIR`
        // (the Fly volume mount, e.g. /data) holds `vidya-licence.db`; else a dev
        // path. Keeping the DB under the mounted volume is the deploy rule.
        let default_db = if dev {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".dev-keys/licence-dev.db")
        } else {
            PathBuf::from("/data/vidya-licence.db")
        };
        let db_path = env_opt("LICENCE_DB")
            .map(PathBuf::from)
            .or_else(|| env_opt("LICENCE_DATA_DIR").map(|d| PathBuf::from(d).join("vidya-licence.db")))
            .unwrap_or(default_db);

        // Bind address. `LICENCE_BIND` may be a full `host:port`, or just a host
        // combined with `LICENCE_PORT` (the Fly `[env]` shape: BIND=0.0.0.0, PORT=8787).
        let host = env_or("LICENCE_BIND", if dev { "127.0.0.1" } else { "0.0.0.0" });
        let bind = if host.contains(':') {
            host
        } else {
            format!("{host}:{}", env_or("LICENCE_PORT", "8787"))
        };
        let base_url = env_or("LICENCE_BASE_URL", "http://127.0.0.1:8787");

        let admin_ip_allowlist = env_opt("ADMIN_IP_ALLOWLIST")
            .map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
            .unwrap_or_default();

        let admin_email = env_opt("LICENCE_ADMIN_EMAIL").or_else(|| dev.then(|| "admin@vidya.local".to_string()));
        let admin_password = env_opt("LICENCE_ADMIN_PASSWORD").or_else(|| dev.then(|| "vidya-dev-admin".to_string()));

        Ok(Config {
            mode,
            bind,
            db_path,
            base_url,
            relay_shared_key,
            code_enc_key,
            session_ttl_hours: env_or("SESSION_TTL_HOURS", "12").parse().unwrap_or(12),
            cookie_secure: env_or("COOKIE_SECURE", if dev { "false" } else { "true" }) != "false",
            admin_email,
            admin_password,
            admin_ip_allowlist,
            releases_json: env_opt("RELEASES_JSON").map(PathBuf::from),
            company_name: env_or("COMPANY_NAME", "[Company name]"),
            support_email: env_or("SUPPORT_EMAIL", "[Support email]"),
            support_phone: env_or("SUPPORT_PHONE", "[Support phone]"),
            upi_id: env_or("UPI_ID", "[UPI ID]"),
            upi_qr_path: env_opt("UPI_QR_PATH").map(PathBuf::from),
            price_text: env_or("PRICE_TEXT", "[Price]"),
            terms_url: env_or("TERMS_URL", "[Terms URL]"),
        })
    }

    /// A fixed, env-independent config for tests (dev mode, permissive cookies,
    /// deterministic keys, in-memory DB path).
    pub fn test_default() -> Config {
        Config {
            mode: Mode::Dev,
            bind: "127.0.0.1:0".into(),
            db_path: PathBuf::from(":memory:"),
            base_url: "http://test".into(),
            relay_shared_key: b"test-relay-shared-key".to_vec(),
            code_enc_key: [42u8; 32],
            session_ttl_hours: 12,
            cookie_secure: false,
            admin_email: None,
            admin_password: None,
            admin_ip_allowlist: vec![],
            releases_json: None,
            company_name: "[Company name]".into(),
            support_email: "[Support email]".into(),
            support_phone: "[Support phone]".into(),
            upi_id: "[UPI ID]".into(),
            upi_qr_path: None,
            price_text: "[Price]".into(),
            terms_url: "[Terms URL]".into(),
        }
    }
}
