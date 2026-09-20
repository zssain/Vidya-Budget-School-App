use std::path::{Path, PathBuf};

use zeroize::Zeroizing;

#[cfg(target_os = "android")]
mod android;
#[cfg(test)]
mod fake;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(test)]
pub use fake::FakePlatform;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeInfo {
    pub volume_id: String,
    pub label: String,
    pub removable: bool,
    pub network: bool,
    pub physical_disk_id: Option<String>,
    pub free_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountedVolume {
    pub mount_path: PathBuf,
    pub info: VolumeInfo,
}

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
    fn volume_info(&self, path: &Path) -> Result<VolumeInfo, PlatformError>;
    fn removable_drives(&self) -> Result<Vec<MountedVolume>, PlatformError>;
    fn network_diagnostics(&self) -> NetworkDiagnostics;
}

/// Build the platform implementation for this operating system.
///
/// `app_data_dir` is the Tauri-resolved application data directory for the
/// bundle identifier `in.vidya.school` (macOS: `~/Library/Application
/// Support/in.vidya.school`). macOS and Android put their data folders inside
/// it; Windows ignores it and uses the machine-wide `ProgramData` folder.
pub fn current(app_data_dir: PathBuf) -> Box<dyn Platform> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacosPlatform::new(app_data_dir));

    #[cfg(target_os = "windows")]
    {
        let _ = app_data_dir;
        return Box::new(windows::WindowsPlatform);
    }

    #[cfg(target_os = "android")]
    return Box::new(android::AndroidPlatform::new(app_data_dir));

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "android")))]
    {
        let _ = app_data_dir;
        panic!("Vidya supports only macOS, Windows and Android");
    }
}
