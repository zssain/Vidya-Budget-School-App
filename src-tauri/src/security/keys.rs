//! Database-key storage (docs/00-SYSTEM-CONTEXT.md §9, prompts/P02 Step 6).
//! Desktop → OS keychain via `keyring`. Android → AndroidKeyStore-wrapped key
//! (a TEMP private-file store here, replaced by plugins/vidya-android in Phase 4).

use rand::RngCore;

#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    /// A DB file exists but its key is missing — never create a new DB over it.
    #[error("DB_KEY_MISSING")]
    Missing,
    #[error("keystore backend error: {0}")]
    Backend(String),
}

/// 32-byte database key.
pub type DbKey = [u8; 32];

pub fn random_key() -> DbKey {
    let mut k = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut k);
    k
}

pub fn to_hex(key: &DbKey) -> String {
    let mut s = String::with_capacity(64);
    for b in key {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

pub fn from_hex(s: &str) -> Option<DbKey> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16)?;
        let lo = (chunk[1] as char).to_digit(16)?;
        out[i] = (hi * 16 + lo) as u8;
    }
    Some(out)
}

pub trait KeyStore {
    fn get(&self) -> Result<Option<DbKey>, KeyError>;
    fn set(&self, key: &DbKey) -> Result<(), KeyError>;
}

/// Load the key, or generate+store it. If the key is absent BUT a DB file
/// already exists → `DB_KEY_MISSING` (never overwrite an existing DB).
pub fn ensure_key(store: &dyn KeyStore, db_exists: bool) -> Result<DbKey, KeyError> {
    if let Some(k) = store.get()? {
        return Ok(k);
    }
    if db_exists {
        return Err(KeyError::Missing);
    }
    let k = random_key();
    store.set(&k)?;
    Ok(k)
}

/// Desktop key store backed by the OS keychain (service `in.vidyabudget.app`,
/// account `db-key`). Stores the key as a 64-char hex password.
#[cfg(not(target_os = "android"))]
pub struct KeyringStore {
    service: String,
    account: String,
}

#[cfg(not(target_os = "android"))]
impl KeyringStore {
    pub fn new() -> Self {
        Self { service: "in.vidyabudget.app".into(), account: "db-key".into() }
    }
}

#[cfg(not(target_os = "android"))]
impl Default for KeyringStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_os = "android"))]
impl KeyStore for KeyringStore {
    fn get(&self) -> Result<Option<DbKey>, KeyError> {
        let entry = keyring::Entry::new(&self.service, &self.account)
            .map_err(|e| KeyError::Backend(e.to_string()))?;
        match entry.get_password() {
            Ok(hex) => Ok(from_hex(&hex)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(KeyError::Backend(e.to_string())),
        }
    }
    fn set(&self, key: &DbKey) -> Result<(), KeyError> {
        let entry = keyring::Entry::new(&self.service, &self.account)
            .map_err(|e| KeyError::Backend(e.to_string()))?;
        entry.set_password(&to_hex(key)).map_err(|e| KeyError::Backend(e.to_string()))
    }
}

/// TEMP: private-file key store. On Android this is replaced by
/// plugins/vidya-android (AndroidKeyStore wrapping) in Phase 4. Also used by tests.
// TEMP: replaced by plugins/vidya-android in Phase 4
pub struct FileKeyStore {
    path: std::path::PathBuf,
}

impl FileKeyStore {
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl KeyStore for FileKeyStore {
    fn get(&self) -> Result<Option<DbKey>, KeyError> {
        match std::fs::read_to_string(&self.path) {
            Ok(hex) => Ok(from_hex(hex.trim())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(KeyError::Backend(e.to_string())),
        }
    }
    fn set(&self, key: &DbKey) -> Result<(), KeyError> {
        std::fs::write(&self.path, to_hex(key)).map_err(|e| KeyError::Backend(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("vidya-key-{tag}-{}-{nanos}", std::process::id()))
    }

    #[test]
    fn hex_roundtrip() {
        let k = random_key();
        assert_eq!(from_hex(&to_hex(&k)), Some(k));
        assert_eq!(from_hex("zz"), None);
    }

    #[test]
    fn ensure_generates_then_reloads_same_key() {
        let p = tmp("gen");
        let store = FileKeyStore::new(&p);
        let k1 = ensure_key(&store, false).unwrap();
        let k2 = ensure_key(&store, true).unwrap(); // now present → returns same
        assert_eq!(k1, k2);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn missing_key_with_existing_db_errors() {
        let p = tmp("missing");
        let store = FileKeyStore::new(&p);
        // key absent + a DB already exists → DB_KEY_MISSING (never overwrite)
        let err = ensure_key(&store, true).unwrap_err();
        assert!(matches!(err, KeyError::Missing));
    }
}
