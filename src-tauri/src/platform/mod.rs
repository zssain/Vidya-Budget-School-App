use std::path::PathBuf;

use zeroize::Zeroizing;

#[cfg(target_os = "android")]
mod android;
#[cfg(test)]
mod fake;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(test)]
pub use fake::FakePlatform;

/// Placeholder populated with drive details in P6.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovableDrive;

/// Placeholder populated with diagnostic results in P7.2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkDiagnostics {
    Unsupported,
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("operation is not supported on this platform yet")]
    Unsupported,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("operating system error: {0}")]
    Os(String),
    #[error("item was not found")]
    NotFound,
}

pub trait Platform: Send + Sync {
    fn name(&self) -> &'static str;
    fn data_dir(&self) -> Result<PathBuf, PlatformError>;
    fn backups_dir(&self) -> Result<PathBuf, PlatformError>;
    fn load_or_create_db_key(&self) -> Result<Zeroizing<[u8; 32]>, PlatformError>;
    fn store_secret(&self, name: &str, secret: &[u8]) -> Result<(), PlatformError>;
    fn load_secret(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError>;
    fn delete_secret(&self, name: &str) -> Result<(), PlatformError>;
    fn device_values(&self) -> Result<(String, String), PlatformError>;
    fn set_keep_awake(&self, on: bool) -> Result<(), PlatformError>;
    fn removable_drives(&self) -> Result<Vec<RemovableDrive>, PlatformError>;
    fn network_diagnostics(&self) -> NetworkDiagnostics;
}

pub fn current() -> Box<dyn Platform> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacosPlatform);

    #[cfg(target_os = "windows")]
    return Box::new(windows::WindowsPlatform);

    #[cfg(target_os = "android")]
    return Box::new(android::AndroidPlatform);

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "android")))]
    panic!("Vidya supports only macOS, Windows and Android");
}
