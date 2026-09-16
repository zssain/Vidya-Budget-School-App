use std::path::PathBuf;

use zeroize::Zeroizing;

use super::{NetworkDiagnostics, Platform, PlatformError, RemovableDrive};

pub struct AndroidPlatform;

impl Platform for AndroidPlatform {
    fn name(&self) -> &'static str {
        "android"
    }
    // Implemented in P8.2.
    fn data_dir(&self) -> Result<PathBuf, PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Android has no backups directory; finalized in P8.2.
    fn backups_dir(&self) -> Result<PathBuf, PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P8.2.
    fn load_or_create_db_key(&self) -> Result<Zeroizing<[u8; 32]>, PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P8.2.
    fn store_secret(&self, _name: &str, _secret: &[u8]) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P8.2.
    fn load_secret(&self, _name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P8.2.
    fn delete_secret(&self, _name: &str) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Desktop-only method; remains unsupported on Android.
    fn device_values(&self) -> Result<(String, String), PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Desktop-only method; remains unsupported on Android.
    fn set_keep_awake(&self, _on: bool) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Desktop-only method; remains unsupported on Android.
    fn removable_drives(&self) -> Result<Vec<RemovableDrive>, PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Desktop-only method; the exact trait returns a value rather than Result.
    fn network_diagnostics(&self) -> NetworkDiagnostics {
        NetworkDiagnostics::Unsupported
    }
}
