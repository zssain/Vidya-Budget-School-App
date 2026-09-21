//! Application start sequence and shared state (P2.4, extended in P2.7).
//!
//! The Tauri `setup` hook runs [`start`] on a blocking thread and publishes the
//! result to [`AppState`]. Commands wait for it. On success the state holds the
//! open database, the built `Services`, and the process-wide `SessionStore`.

use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};

use vidya_db::{repo, Db, DbError};
use vidya_services::auth::SessionStore;
use vidya_services::env::{OsRandom, SystemClock, UuidV7};
use vidya_services::{Mode, Services};

use crate::platform::Platform;

/// Everything the running app needs once the database is open.
pub struct AppCore {
    pub db: Arc<Db>,
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

/// The published start result: services ready, or a startup failure.
pub enum StartResult {
    Ready {
        core: Arc<AppCore>,
        services: Arc<Services>,
    },
    Failed {
        kind: StartupFailure,
        /// Log-safe detail (never key bytes or personal data).
        detail: String,
    },
}

impl Clone for StartResult {
    fn clone(&self) -> Self {
        match self {
            StartResult::Ready { core, services } => StartResult::Ready {
                core: core.clone(),
                services: services.clone(),
            },
            StartResult::Failed { kind, detail } => StartResult::Failed {
                kind: *kind,
                detail: detail.clone(),
            },
        }
    }
}

/// Run the full start sequence: open the database, then build the services
/// (ensuring this device's id exists). Never panics.
pub fn start(platform: Box<dyn Platform>) -> StartResult {
    // Open the database first; only build services on success.
    let core = {
        let data_dir = match platform.data_dir() {
            Ok(dir) => dir,
            Err(error) => {
                return StartResult::Failed {
                    kind: StartupFailure::DataFolder,
                    detail: error.to_string(),
                }
            }
        };
        let key = match platform.load_or_create_db_key() {
            Ok(key) => key,
            Err(error) => {
                return StartResult::Failed {
                    kind: StartupFailure::SecureStorage,
                    detail: error.to_string(),
                }
            }
        };
        match Db::open(&data_dir.join("vidya.db"), &key) {
            Ok(db) => Arc::new(AppCore {
                db: Arc::new(db),
                platform,
                data_dir,
            }),
            Err(DbError::WrongKey) => {
                return StartResult::Failed {
                    kind: StartupFailure::WrongKey,
                    detail: "database key did not unlock the data".to_owned(),
                }
            }
            Err(DbError::Migration { version, message }) => {
                return StartResult::Failed {
                    kind: StartupFailure::Migration,
                    detail: format!("migration blocked at version {version}: {message}"),
                }
            }
            Err(error) => {
                return StartResult::Failed {
                    kind: StartupFailure::Unknown,
                    detail: error.to_string(),
                }
            }
        }
    };

    match build_services(&core) {
        Ok(services) => StartResult::Ready {
            core,
            services: Arc::new(services),
        },
        Err(detail) => StartResult::Failed {
            kind: StartupFailure::Unknown,
            detail,
        },
    }
}

/// Build the server-mode `Services` from an open database.
fn build_services(core: &Arc<AppCore>) -> Result<Services, String> {
    let device_id = ensure_device_id(&core.db).map_err(|e| e.to_string())?;
    Services::new(
        core.db.clone(),
        Arc::new(SystemClock),
        Arc::new(UuidV7),
        Arc::new(OsRandom),
        Mode::Server,
        device_id,
    )
    .map_err(|e| e.to_string())
}

/// Reads this computer's device id from `meta`, creating a UUID v7 on first run.
fn ensure_device_id(db: &Db) -> Result<String, DbError> {
    if let Some(id) = db.read(|conn| repo::meta::get(conn, "device_id"))? {
        if !id.is_empty() {
            return Ok(id);
        }
    }
    let id = uuid::Uuid::now_v7().to_string();
    db.write(|tx| repo::meta::set(tx, "device_id", &id))?;
    Ok(id)
}

/// Managed Tauri state: the start result (behind a condition variable so a
/// command can wait without blocking the UI thread) and the session store.
/// Cheap to clone (shared `Arc`s).
#[derive(Clone)]
pub struct AppState {
    slot: Arc<Slot>,
    sessions: Arc<SessionStore>,
}

struct Slot {
    result: Mutex<Option<StartResult>>,
    ready: Condvar,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            slot: Arc::new(Slot {
                result: Mutex::new(None),
                ready: Condvar::new(),
            }),
            sessions: Arc::new(SessionStore::new()),
        }
    }

    /// Publish the start result and wake any waiting commands.
    pub fn publish(&self, result: StartResult) {
        let mut guard = self.slot.result.lock().expect("startup mutex poisoned");
        *guard = Some(result);
        self.slot.ready.notify_all();
    }

    /// Block until startup has finished, then return a clone of the result.
    /// Must be called on a blocking thread.
    pub fn wait(&self) -> StartResult {
        let mut guard = self.slot.result.lock().expect("startup mutex poisoned");
        while guard.is_none() {
            guard = self.slot.ready.wait(guard).expect("startup mutex poisoned");
        }
        guard.as_ref().expect("startup result present").clone()
    }

    pub fn sessions(&self) -> Arc<SessionStore> {
        self.sessions.clone()
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

        match start(Box::new(FakePlatform::new(dir.clone(), key))) {
            StartResult::Ready { core, .. } => assert!(core.data_dir.join("vidya.db").exists()),
            StartResult::Failed { detail, .. } => panic!("first start failed: {detail}"),
        }
        assert!(matches!(
            start(Box::new(FakePlatform::new(dir.clone(), key))),
            StartResult::Ready { .. }
        ));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn start_with_wrong_key_reports_wrong_key() {
        let dir = temp_dir("wrongkey");
        assert!(matches!(
            start(Box::new(FakePlatform::new(dir.clone(), [7u8; 32]))),
            StartResult::Ready { .. }
        ));
        match start(Box::new(FakePlatform::new(dir.clone(), [9u8; 32]))) {
            StartResult::Failed { kind, .. } => assert_eq!(kind, StartupFailure::WrongKey),
            StartResult::Ready { .. } => panic!("wrong key unexpectedly opened the database"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
