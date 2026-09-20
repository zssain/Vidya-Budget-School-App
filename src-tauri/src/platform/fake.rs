use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};

use zeroize::Zeroizing;

use super::{MountedVolume, NetworkDiagnostics, Platform, PlatformError, VolumeInfo};

pub struct FakePlatform {
    data_dir: PathBuf,
    db_key: [u8; 32],
    secrets: Mutex<HashMap<String, Vec<u8>>>,
    awake: AtomicBool,
}

impl FakePlatform {
    /// A fake platform rooted at `data_dir` that returns `db_key` from
    /// `load_or_create_db_key` (so a test can simulate a replaced key).
    pub fn new(data_dir: PathBuf, db_key: [u8; 32]) -> Self {
        Self {
            data_dir,
            db_key,
            secrets: Mutex::new(HashMap::new()),
            awake: AtomicBool::new(false),
        }
    }
}

impl Default for FakePlatform {
    fn default() -> Self {
        Self::new(std::env::temp_dir().join("vidya-platform-test"), [7; 32])
    }
}

impl Platform for FakePlatform {
    fn name(&self) -> &'static str {
        "fake"
    }
    fn data_dir(&self) -> Result<PathBuf, PlatformError> {
        // Real platforms create the data folder inside data_dir(); the fake does
        // the same so Db::open can create the database file.
        std::fs::create_dir_all(&self.data_dir)?;
        Ok(self.data_dir.clone())
    }
    fn backups_dir(&self) -> Result<PathBuf, PlatformError> {
        Ok(self.data_dir.join("backups"))
    }
    fn load_or_create_db_key(&self) -> Result<Zeroizing<[u8; 32]>, PlatformError> {
        Ok(Zeroizing::new(self.db_key))
    }
    fn store_secret(&self, name: &str, secret: &[u8]) -> Result<(), PlatformError> {
        self.secrets
            .lock()
            .map_err(|e| PlatformError::Os(e.to_string()))?
            .insert(name.to_owned(), secret.to_vec());
        Ok(())
    }
    fn load_secret(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError> {
        Ok(self
            .secrets
            .lock()
            .map_err(|e| PlatformError::Os(e.to_string()))?
            .get(name)
            .cloned()
            .map(Zeroizing::new))
    }
    fn delete_secret(&self, name: &str) -> Result<(), PlatformError> {
        self.secrets
            .lock()
            .map_err(|e| PlatformError::Os(e.to_string()))?
            .remove(name);
        Ok(())
    }
    fn device_values(&self) -> Result<(String, String), PlatformError> {
        Ok(("machine".into(), "system".into()))
    }
    fn set_keep_awake(&self, on: bool) -> Result<(), PlatformError> {
        self.awake.store(on, Ordering::SeqCst);
        Ok(())
    }
    fn volume_info(&self, _path: &Path) -> Result<VolumeInfo, PlatformError> {
        Ok(VolumeInfo {
            volume_id: "fake-volume".into(),
            label: "Fake volume".into(),
            removable: false,
            network: false,
            physical_disk_id: Some("fake-disk".into()),
            free_bytes: 512,
            total_bytes: 1024,
        })
    }
    fn removable_drives(&self) -> Result<Vec<MountedVolume>, PlatformError> {
        Ok(Vec::new())
    }
    fn network_diagnostics(&self) -> NetworkDiagnostics {
        NetworkDiagnostics::Unsupported
    }
}

#[test]
fn fake_stores_and_deletes_secrets() {
    let platform = FakePlatform::default();
    platform.store_secret("test", b"secret").expect("store secret");
    assert_eq!(
        platform
            .load_secret("test")
            .expect("load secret")
            .map(|secret| secret.to_vec()),
        Some(b"secret".to_vec())
    );
    platform.delete_secret("test").expect("delete secret");
    assert!(platform.load_secret("test").expect("load after delete").is_none());
}
