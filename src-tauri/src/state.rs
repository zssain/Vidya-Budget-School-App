//! Application start sequence and shared state (P2.4).
//!
//! The Tauri `setup` hook runs [`start`] on a blocking thread and stores the
//! result in [`AppState`]. The `app_status` command waits for that result and
//! reports it to the React app, which shows an "Opening Vidya…" state until the
//! database is open (or a full-screen error if it is not).

use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};

use vidya_db::{Db, DbError};

use crate::platform::Platform;

/// Everything the running app needs once the database is open.
pub struct AppCore {
    pub db: Db,
    pub platform: Box<dyn Platform>,
    pub data_dir: PathBuf,
}

/// Why the app could not start. Serialized to the React `StartupError` screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupFailure {
    DataFolder,
    SecureStorage,
    WrongKey,
    Migration,
    Unknown,
}

impl StartupFailure {
    /// Stable identifier used as the `failure` field and the `startup.error.*`
    /// translation key suffix.
    pub fn as_str(self) -> &'static str {
        match self {
            StartupFailure::DataFolder => "DataFolder",
            StartupFailure::SecureStorage => "SecureStorage",
            StartupFailure::WrongKey => "WrongKey",
            StartupFailure::Migration => "Migration",
            StartupFailure::Unknown => "Unknown",
        }
    }
}

/// The outcome of the start sequence.
pub enum StartupState {
    Ready(Arc<AppCore>),
    Failed {
        kind: StartupFailure,
        /// Log-safe detail (never key bytes or personal data).
        detail_for_log: String,
    },
}

impl Clone for StartupState {
    fn clone(&self) -> Self {
        match self {
            StartupState::Ready(core) => StartupState::Ready(core.clone()),
            StartupState::Failed { kind, detail_for_log } => StartupState::Failed {
                kind: *kind,
                detail_for_log: detail_for_log.clone(),
            },
        }
    }
}

/// Run the start sequence: data folder → database key → open the encrypted
/// database. Never panics; every failure becomes a `StartupState::Failed` with
/// a log-safe detail message.
pub fn start(platform: Box<dyn Platform>) -> StartupState {
    let data_dir = match platform.data_dir() {
        Ok(dir) => dir,
        Err(error) => {
            return StartupState::Failed {
                kind: StartupFailure::DataFolder,
                detail_for_log: error.to_string(),
            }
        }
    };
    let key = match platform.load_or_create_db_key() {
        Ok(key) => key,
        Err(error) => {
            return StartupState::Failed {
                kind: StartupFailure::SecureStorage,
                detail_for_log: error.to_string(),
            }
        }
    };
    match Db::open(&data_dir.join("vidya.db"), &key) {
        Ok(db) => StartupState::Ready(Arc::new(AppCore {
            db,
            platform,
            data_dir,
        })),
        Err(DbError::WrongKey) => StartupState::Failed {
            kind: StartupFailure::WrongKey,
            detail_for_log: "database key did not unlock the data".to_owned(),
        },
        Err(DbError::Migration { version, message }) => StartupState::Failed {
            kind: StartupFailure::Migration,
            detail_for_log: format!("migration blocked at version {version}: {message}"),
        },
        Err(error) => StartupState::Failed {
            kind: StartupFailure::Unknown,
            detail_for_log: error.to_string(),
        },
    }
}

/// Managed Tauri state holding the start result behind a condition variable so a
/// command can wait for startup without blocking the UI thread. Cheap to clone
/// (shared `Arc`), so commands can move a handle onto a blocking thread.
#[derive(Clone)]
pub struct AppState {
    slot: Arc<StartupSlot>,
}

struct StartupSlot {
    ready: Mutex<Option<StartupState>>,
    condvar: Condvar,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            slot: Arc::new(StartupSlot {
                ready: Mutex::new(None),
                condvar: Condvar::new(),
            }),
        }
    }

    /// Publish the start result and wake any waiting commands.
    pub fn set(&self, state: StartupState) {
        let mut guard = self.slot.ready.lock().expect("startup mutex poisoned");
        *guard = Some(state);
        self.slot.condvar.notify_all();
    }

    /// Block until startup has finished, then return a clone of the result.
    /// Must be called on a blocking thread (see the `app_status` command).
    pub fn wait(&self) -> StartupState {
        let mut guard = self.slot.ready.lock().expect("startup mutex poisoned");
        while guard.is_none() {
            guard = self.slot.condvar.wait(guard).expect("startup mutex poisoned");
        }
        guard.as_ref().expect("startup result present").clone()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::FakePlatform;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vidya-startup-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn start_creates_then_reopens_database() {
        let dir = temp_dir("reopen");
        let key = [7u8; 32];

        // First start creates the encrypted database.
        match start(Box::new(FakePlatform::new(dir.clone(), key))) {
            StartupState::Ready(core) => {
                assert!(core.data_dir.join("vidya.db").exists());
            }
            StartupState::Failed { detail_for_log, .. } => panic!("first start failed: {detail_for_log}"),
        }

        // Second start opens the same database with the same key.
        assert!(matches!(
            start(Box::new(FakePlatform::new(dir.clone(), key))),
            StartupState::Ready(_)
        ));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn start_with_wrong_key_reports_wrong_key() {
        let dir = temp_dir("wrongkey");

        // Create the database with one key.
        assert!(matches!(
            start(Box::new(FakePlatform::new(dir.clone(), [7u8; 32]))),
            StartupState::Ready(_)
        ));

        // A different key must not unlock it.
        match start(Box::new(FakePlatform::new(dir.clone(), [9u8; 32]))) {
            StartupState::Failed { kind, .. } => assert_eq!(kind, StartupFailure::WrongKey),
            StartupState::Ready(_) => panic!("wrong key unexpectedly opened the database"),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
