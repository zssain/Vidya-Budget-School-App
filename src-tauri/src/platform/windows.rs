use std::path::PathBuf;

use zeroize::Zeroizing;

use super::{NetworkDiagnostics, Platform, PlatformError, RemovableDrive};

pub struct WindowsPlatform;

impl Platform for WindowsPlatform {
    fn name(&self) -> &'static str {
        "windows"
    }
    // Implemented in P2.4.
    fn data_dir(&self) -> Result<PathBuf, PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P6.1.
    fn backups_dir(&self) -> Result<PathBuf, PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P2.4.
    fn load_or_create_db_key(&self) -> Result<Zeroizing<[u8; 32]>, PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P2.4.
    fn store_secret(&self, _name: &str, _secret: &[u8]) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P2.4.
    fn load_secret(&self, _name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P2.4.
    fn delete_secret(&self, _name: &str) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P4.2.
    fn device_values(&self) -> Result<(String, String), PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P7.1.
    fn set_keep_awake(&self, _on: bool) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P6.1.
    fn removable_drives(&self) -> Result<Vec<RemovableDrive>, PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P7.2. The exact trait returns a value, so its placeholder is the Unsupported variant.
    fn network_diagnostics(&self) -> NetworkDiagnostics {
        NetworkDiagnostics::Unsupported
    }
}
