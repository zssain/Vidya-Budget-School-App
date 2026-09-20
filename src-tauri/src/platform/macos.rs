//! macOS platform implementation (P2.4).
//!
//! Data lives under the Tauri app data directory
//! `~/Library/Application Support/in.vidya.school`, created `0700`. The 32-byte
//! database key and other secrets are kept in the login Keychain as generic
//! passwords, never written to disk in plain form.
//!
//! Note: the Keychain item is stored under the current macOS user account, so
//! the school must always sign in to the same Mac account (also in README.md).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use security_framework::passwords::{delete_generic_password, get_generic_password, set_generic_password};
use security_framework::random::SecRandom;
use zeroize::Zeroizing;

use super::{MountedVolume, NetworkDiagnostics, Platform, PlatformError, VolumeInfo};

/// `errSecItemNotFound` from the macOS Security framework. Verified in
/// `security-framework-sys` 2.17.0 (`src/base.rs`: `errSecItemNotFound = -25300`).
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

/// Keychain service that holds the raw database key.
const DB_KEY_SERVICE: &str = "in.vidya.school.dbkey";
/// Keychain account (row name) for the database key.
const DB_KEY_ACCOUNT: &str = "database";
/// Fixed Keychain account used for every named secret.
const SECRET_ACCOUNT: &str = "vidya";

pub struct MacosPlatform {
    /// `~/Library/Application Support/in.vidya.school`.
    app_data_dir: PathBuf,
}

impl MacosPlatform {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self { app_data_dir }
    }

    /// Create `dir` (and parents) with owner-only `0700` permissions.
    fn ensure_private_dir(dir: &Path) -> Result<(), PlatformError> {
        fs::create_dir_all(dir)?;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
        Ok(())
    }

    fn secret_service(name: &str) -> String {
        format!("in.vidya.school.{name}")
    }
}

impl Platform for MacosPlatform {
    fn name(&self) -> &'static str {
        "macos"
    }

    fn data_dir(&self) -> Result<PathBuf, PlatformError> {
        let dir = self.app_data_dir.join("data");
        Self::ensure_private_dir(&dir)?;
        Ok(dir)
    }

    fn backups_dir(&self) -> Result<PathBuf, PlatformError> {
        let dir = self.app_data_dir.join("backups");
        Self::ensure_private_dir(&dir)?;
        Ok(dir)
    }

    fn load_or_create_db_key(&self) -> Result<Zeroizing<[u8; 32]>, PlatformError> {
        match get_generic_password(DB_KEY_SERVICE, DB_KEY_ACCOUNT) {
            Ok(bytes) => {
                // `bytes` is zeroized when this Zeroizing wrapper is dropped.
                let bytes = Zeroizing::new(bytes);
                let key: [u8; 32] = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| PlatformError::Os("stored database key has the wrong length".into()))?;
                Ok(Zeroizing::new(key))
            }
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => {
                let mut key = Zeroizing::new([0u8; 32]);
                SecRandom::default()
                    .copy_bytes(&mut key[..])
                    .map_err(|e| PlatformError::Os(format!("secure random failed: {e}")))?;
                set_generic_password(DB_KEY_SERVICE, DB_KEY_ACCOUNT, key.as_ref())
                    .map_err(|e| PlatformError::Os(format!("could not store database key: {e}")))?;
                Ok(key)
            }
            Err(e) => Err(PlatformError::Os(format!("could not read database key: {e}"))),
        }
    }

    fn store_secret(&self, name: &str, secret: &[u8]) -> Result<(), PlatformError> {
        set_generic_password(&Self::secret_service(name), SECRET_ACCOUNT, secret)
            .map_err(|e| PlatformError::Os(format!("could not store secret: {e}")))
    }

    fn load_secret(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError> {
        match get_generic_password(&Self::secret_service(name), SECRET_ACCOUNT) {
            Ok(bytes) => Ok(Some(Zeroizing::new(bytes))),
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(None),
            Err(e) => Err(PlatformError::Os(format!("could not read secret: {e}"))),
        }
    }

    fn delete_secret(&self, name: &str) -> Result<(), PlatformError> {
        match delete_generic_password(&Self::secret_service(name), SECRET_ACCOUNT) {
            Ok(()) => Ok(()),
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(()),
            Err(e) => Err(PlatformError::Os(format!("could not delete secret: {e}"))),
        }
    }

    // Implemented in P4.2.
    fn device_values(&self) -> Result<(String, String), PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P7.1.
    fn set_keep_awake(&self, _on: bool) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P6.2.
    fn volume_info(&self, _path: &Path) -> Result<VolumeInfo, PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P6.2.
    fn removable_drives(&self) -> Result<Vec<MountedVolume>, PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P7.2. The exact trait returns a value, so its placeholder is the Unsupported variant.
    fn network_diagnostics(&self) -> NetworkDiagnostics {
        NetworkDiagnostics::Unsupported
    }
}
