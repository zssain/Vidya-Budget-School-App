//! Environment configuration. Secrets never come from env-var *contents*: the
//! signing key is a file path, and the process refuses to start if anything that
//! looks like inline key material is present in the environment.

use std::path::PathBuf;
use std::time::Duration;

/// Validated configuration.
pub struct Config {
    pub key_file: PathBuf,
    pub key_id: String,
    pub expected_fingerprints: String,
    pub database_url: String,
    pub poll_interval: Duration,
    pub worker_id: String,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // database_url may carry a password: never print it.
        f.debug_struct("Config")
            .field("key_file", &self.key_file)
            .field("key_id", &self.key_id)
            .field("poll_interval", &self.poll_interval)
            .field("worker_id", &self.worker_id)
            .finish_non_exhaustive()
    }
}

/// Env vars that would mean someone put key material straight into the
/// environment. Their presence is a hard error.
pub const FORBIDDEN_VARS: &[&str] = &[
    "LICENCE_SIGNING_KEY",
    "LICENCE_SIGNING_KEY_B64",
    "LICENCE_SIGNING_KEY_BASE64",
];

impl Config {
    /// Read from a lookup function (the real one is `std::env::var`).
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        for v in FORBIDDEN_VARS {
            if get(v).is_some() {
                return Err(format!(
                    "{v} is set: key material must never be passed in the environment — use LICENCE_SIGNING_KEY_FILE"
                ));
            }
        }
        let req = |k: &str| -> Result<String, String> {
            get(k)
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| format!("{k} is required"))
        };
        let poll_secs = match get("SIGNER_POLL_INTERVAL_SECS") {
            None => 5,
            Some(s) => s
                .trim()
                .parse::<u64>()
                .ok()
                .filter(|n| (1..=3600).contains(n))
                .ok_or("SIGNER_POLL_INTERVAL_SECS must be 1-3600")?,
        };
        let worker_id = get("SIGNER_WORKER_ID")
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| format!("signer-{}-{}", std::process::id(), uuid::Uuid::new_v4().simple()));
        Ok(Self {
            key_file: PathBuf::from(req("LICENCE_SIGNING_KEY_FILE")?),
            key_id: req("LICENCE_SIGNING_KEY_ID")?,
            expected_fingerprints: req("EXPECTED_PUBLIC_KEY_FINGERPRINT")?,
            database_url: req("SIGNER_DATABASE_URL")?,
            poll_interval: Duration::from_secs(poll_secs),
            worker_id,
        })
    }

    pub fn from_env() -> Result<Self, String> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let m: HashMap<String, String> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |k| m.get(k).cloned()
    }

    const BASE: &[(&str, &str)] = &[
        ("LICENCE_SIGNING_KEY_FILE", "/run/secrets/k"),
        ("LICENCE_SIGNING_KEY_ID", "k1"),
        ("EXPECTED_PUBLIC_KEY_FINGERPRINT", "sha256:00"),
        ("SIGNER_DATABASE_URL", "postgres://u:secretpw@h/db"),
    ];

    #[test]
    fn requires_all_and_hides_url() {
        let c = Config::from_lookup(env(BASE)).unwrap();
        assert!(!format!("{c:?}").contains("secretpw"));
        assert_eq!(c.poll_interval, Duration::from_secs(5));
        for skip in 0..BASE.len() {
            let partial: Vec<_> = BASE
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != skip)
                .map(|(_, p)| *p)
                .collect();
            assert!(Config::from_lookup(env(&partial)).is_err());
        }
    }

    #[test]
    fn inline_key_material_is_refused() {
        let mut v = BASE.to_vec();
        v.push(("LICENCE_SIGNING_KEY", "AAAA"));
        let e = Config::from_lookup(env(&v)).unwrap_err();
        assert!(e.contains("LICENCE_SIGNING_KEY_FILE") && !e.contains("AAAA"));
    }
}
