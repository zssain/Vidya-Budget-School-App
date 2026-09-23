//! Machine id (prompts/P03 Step 3). A random UUID stored in the OS keychain on
//! desktop, so it is **stable across app updates** but **new after an OS
//! reinstall** (the keychain entry is gone). Android / tests use a private file
//! fallback (replaced by `plugins/vidya-android` in a later phase).

/// A store for the machine id.
pub trait MachineIdStore {
    fn get(&self) -> Option<String>;
    fn set(&self, id: &str) -> Result<(), String>;
}

/// Return the existing machine id, or generate + persist a new random UUID (v7).
pub fn get_or_create(store: &dyn MachineIdStore) -> Result<String, String> {
    if let Some(id) = store.get() {
        if !id.trim().is_empty() {
            return Ok(id);
        }
    }
    let id = uuid::Uuid::now_v7().to_string();
    store.set(&id)?;
    Ok(id)
}

/// Desktop keychain-backed store (service `in.vidyabudget.app`, account
/// `machine-id`). Same service as the DB key, a different account.
#[cfg(not(target_os = "android"))]
pub struct KeyringMachineStore {
    service: String,
    account: String,
}

#[cfg(not(target_os = "android"))]
impl KeyringMachineStore {
    pub fn new() -> Self {
        Self { service: "in.vidyabudget.app".into(), account: "machine-id".into() }
    }
}

#[cfg(not(target_os = "android"))]
impl Default for KeyringMachineStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_os = "android"))]
impl MachineIdStore for KeyringMachineStore {
    fn get(&self) -> Option<String> {
        let entry = keyring::Entry::new(&self.service, &self.account).ok()?;
        entry.get_password().ok()
    }
    fn set(&self, id: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(&self.service, &self.account).map_err(|e| e.to_string())?;
        entry.set_password(id).map_err(|e| e.to_string())
    }
}

/// Private-file store (Android / tests).
pub struct FileMachineStore {
    path: std::path::PathBuf,
}

impl FileMachineStore {
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl MachineIdStore for FileMachineStore {
    fn get(&self) -> Option<String> {
        std::fs::read_to_string(&self.path).ok().map(|s| s.trim().to_string())
    }
    fn set(&self, id: &str) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&self.path, id).map_err(|e| e.to_string())
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
        std::env::temp_dir().join(format!("vidya-machine-{tag}-{}-{nanos}", std::process::id()))
    }

    #[test]
    fn generates_then_reloads_same_id() {
        let p = tmp("gen");
        let store = FileMachineStore::new(&p);
        let a = get_or_create(&store).unwrap();
        let b = get_or_create(&store).unwrap(); // present now → same id
        assert_eq!(a, b);
        assert!(!a.is_empty());
        // Looks like a UUID (8-4-4-4-12).
        assert_eq!(a.split('-').count(), 5);
        let _ = std::fs::remove_file(&p);
    }
}
