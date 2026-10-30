//! Signing-key loading and the public-key fingerprint gate.
//!
//! **Key file format** (exactly what `tools/licence-maker init` writes to
//! `licence-signing.key`): the 32-byte ed25519 *seed* (secret key), encoded as
//! STANDARD base64 (with `=` padding, 44 chars), no newline required. Leading /
//! trailing whitespace is ignored (licence-maker trims too). Anything else is
//! rejected.
//!
//! **Fingerprint**: `sha256:` + lowercase hex of SHA-256 over the 32 raw
//! public-key bytes (the bytes inside licence-maker's `public.key`, i.e. what the
//! app's `licence_public_keys` build config holds, base64-decoded).
//!
//! The key is only ever read from a file path; its bytes are never logged, never
//! put in an error message and are zeroised after use. [`Signer`]'s `Debug`
//! shows only the key id and fingerprint.

use std::path::Path;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signer as _, SigningKey};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

/// A key file larger than this is certainly not a base64 seed.
const MAX_KEY_FILE_BYTES: u64 = 4096;

/// Why the signer refused to load its key. Messages never contain key material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyError(pub String);

impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for KeyError {}

fn err(msg: impl Into<String>) -> KeyError {
    KeyError(msg.into())
}

/// `sha256:<hex>` fingerprint of a raw 32-byte ed25519 public key.
pub fn fingerprint(public_key: &[u8; 32]) -> String {
    let digest = Sha256::digest(public_key);
    let mut out = String::with_capacity(7 + 64);
    out.push_str("sha256:");
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Normalise one configured fingerprint: trim, lowercase, optional `sha256:`
/// prefix. Returns `None` unless it is exactly 64 hex chars.
fn normalise_fingerprint(text: &str) -> Option<String> {
    let t = text.trim().to_ascii_lowercase();
    let hex = t.strip_prefix("sha256:").unwrap_or(&t);
    if hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        Some(format!("sha256:{hex}"))
    } else {
        None
    }
}

/// Parse `EXPECTED_PUBLIC_KEY_FINGERPRINT`: one or more fingerprints separated by
/// commas (an allow-list, useful during a rotation). Empty / malformed → error.
pub fn parse_expected_fingerprints(text: &str) -> Result<Vec<String>, KeyError> {
    let mut out = Vec::new();
    for part in text.split(',') {
        if part.trim().is_empty() {
            continue;
        }
        let fp = normalise_fingerprint(part)
            .ok_or_else(|| err("EXPECTED_PUBLIC_KEY_FINGERPRINT entries must be sha256:<64 hex chars>"))?;
        out.push(fp);
    }
    if out.is_empty() {
        return Err(err("EXPECTED_PUBLIC_KEY_FINGERPRINT is empty"));
    }
    Ok(out)
}

/// Decode the licence-maker key-file text (base64 seed) into a signing key.
pub fn signing_key_from_text(text: &str) -> Result<SigningKey, KeyError> {
    let mut bytes = Zeroizing::new(
        STANDARD
            .decode(text.trim().as_bytes())
            .map_err(|_| err("signing key file is not valid base64 (expected licence-maker's licence-signing.key)"))?,
    );
    if bytes.len() != 32 {
        let n = bytes.len();
        bytes.zeroize();
        return Err(err(format!("signing key must decode to 32 bytes, got {n}")));
    }
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&bytes);
    let key = SigningKey::from_bytes(&seed);
    seed.zeroize();
    Ok(key)
}

/// Read the key file. On Unix the file must not be readable or writable by group
/// or others (licence-maker creates it 0600).
pub fn read_key_file(path: &Path) -> Result<SigningKey, KeyError> {
    let meta = std::fs::metadata(path).map_err(|e| {
        err(format!(
            "cannot read LICENCE_SIGNING_KEY_FILE ({}): {}",
            path.display(),
            e.kind()
        ))
    })?;
    if !meta.is_file() {
        return Err(err("LICENCE_SIGNING_KEY_FILE is not a regular file"));
    }
    if meta.len() > MAX_KEY_FILE_BYTES {
        return Err(err("LICENCE_SIGNING_KEY_FILE is too large to be a licence-maker key"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(err(format!(
                "LICENCE_SIGNING_KEY_FILE has permissions {mode:o}; it must be readable by the signer user only (chmod 600 or 400)"
            )));
        }
    }
    let text = Zeroizing::new(
        std::fs::read_to_string(path)
            .map_err(|e| err(format!("cannot read LICENCE_SIGNING_KEY_FILE: {}", e.kind())))?,
    );
    signing_key_from_text(&text)
}

/// Read a licence-maker `public.key` file (base64 of the 32 public-key bytes).
pub fn read_public_key_file(path: &Path) -> Result<[u8; 32], KeyError> {
    let text = std::fs::read_to_string(path).map_err(|e| err(format!("cannot read public key file: {}", e.kind())))?;
    let bytes = STANDARD
        .decode(text.trim().as_bytes())
        .map_err(|_| err("public key file is not valid base64"))?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| err("public key must decode to 32 bytes"))
}

/// The loaded, fingerprint-checked signing identity.
pub struct Signer {
    key: SigningKey,
    public: [u8; 32],
    key_id: String,
    fingerprint: String,
}

impl std::fmt::Debug for Signer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signer")
            .field("key_id", &self.key_id)
            .field("fingerprint", &self.fingerprint)
            .finish_non_exhaustive()
    }
}

/// `LICENCE_SIGNING_KEY_ID`: 1–64 chars of `[A-Za-z0-9._-]`.
fn valid_key_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

impl Signer {
    /// Build a signer from an in-memory key, refusing unless its public key's
    /// fingerprint is in `expected` (the allow-list the app releases trust).
    pub fn new(key: SigningKey, key_id: &str, expected: &[String]) -> Result<Self, KeyError> {
        if !valid_key_id(key_id) {
            return Err(err("LICENCE_SIGNING_KEY_ID must be 1-64 chars of [A-Za-z0-9._-]"));
        }
        let public = key.verifying_key().to_bytes();
        let fp = fingerprint(&public);
        if !expected.iter().any(|e| e == &fp) {
            return Err(err(format!(
                "refusing to start: signing key fingerprint {fp} is not in EXPECTED_PUBLIC_KEY_FINGERPRINT — \
                 app releases would not verify what this key signs"
            )));
        }
        Ok(Self {
            key,
            public,
            key_id: key_id.to_string(),
            fingerprint: fp,
        })
    }

    /// Load from `LICENCE_SIGNING_KEY_FILE` and check the fingerprint allow-list.
    pub fn load(path: &Path, key_id: &str, expected_fingerprints: &str) -> Result<Self, KeyError> {
        let expected = parse_expected_fingerprints(expected_fingerprints)?;
        let key = read_key_file(path)?;
        Self::new(key, key_id, &expected)
    }

    pub fn public_key(&self) -> &[u8; 32] {
        &self.public
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Raw ed25519 signature (deterministic, RFC 8032) over `msg`.
    pub fn sign(&self, msg: &[u8]) -> [u8; 64] {
        self.key.sign(msg).to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_text(seed: u8) -> String {
        STANDARD.encode([seed; 32])
    }

    #[test]
    fn fingerprint_format_and_normalisation() {
        let key = signing_key_from_text(&seed_text(1)).unwrap();
        let fp = fingerprint(&key.verifying_key().to_bytes());
        assert!(fp.starts_with("sha256:") && fp.len() == 71);
        let bare_upper = fp.trim_start_matches("sha256:").to_ascii_uppercase();
        assert_eq!(
            parse_expected_fingerprints(&format!(" {bare_upper} ")).unwrap(),
            vec![fp.clone()]
        );
        assert!(parse_expected_fingerprints("").is_err());
        assert!(parse_expected_fingerprints("sha256:abc").is_err());
    }

    #[test]
    fn key_text_rejects_bad_input() {
        assert!(signing_key_from_text("***").is_err());
        assert!(signing_key_from_text(&STANDARD.encode([0u8; 31])).is_err());
        assert!(signing_key_from_text(&format!("{}\n", seed_text(3))).is_ok());
    }

    #[test]
    fn wrong_fingerprint_refuses_and_debug_hides_key() {
        let key = signing_key_from_text(&seed_text(5)).unwrap();
        let other = signing_key_from_text(&seed_text(6)).unwrap();
        let other_fp = fingerprint(&other.verifying_key().to_bytes());
        let e = Signer::new(key.clone(), "k1", &[other_fp]).unwrap_err();
        assert!(e.0.contains("refusing to start"));
        let fp = fingerprint(&key.verifying_key().to_bytes());
        let s = Signer::new(key, "k1", &[fp]).unwrap();
        let dbg = format!("{s:?}");
        assert!(!dbg.contains(&seed_text(5)));
        assert!(dbg.contains("k1"));
    }

    #[test]
    fn key_id_is_validated() {
        let key = signing_key_from_text(&seed_text(5)).unwrap();
        let fp = fingerprint(&key.verifying_key().to_bytes());
        assert!(Signer::new(key.clone(), "", std::slice::from_ref(&fp)).is_err());
        assert!(Signer::new(key, "bad id!", &[fp]).is_err());
    }
}
