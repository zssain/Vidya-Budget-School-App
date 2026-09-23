//! App runtime context (prompts/P03 Step 5). Tauri-managed shared state: the
//! encrypted DB connection, the current unlocked session, machine id, device mode
//! (Server on a single-PC school), the HTTP client for the licence service, and
//! the in-memory recovery key held only between generate + confirm.
//!
//! Commands stay thin (rule §6): they lock this state, call the pure logic in
//! `commands`/`dash`/vidya-core, and return DTOs or a `CmdError`.

use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::{CmdError, CmdResult};
use crate::state::SessionStaff;
use crate::write::DeviceMode;

/// The Tauri-managed runtime context.
pub struct RtCtx {
    /// The encrypted connection, opened once the DB key is available. `None` when
    /// the DB key is missing (a DB file exists but the keychain entry is gone).
    pub db: Mutex<Option<Connection>>,
    /// The signed-in staff member, or `None` when locked.
    pub session: Mutex<Option<SessionStaff>>,
    /// This device's stable machine id (keychain on desktop).
    pub machine_id: String,
    /// This device's `device` row id once setup created it.
    pub device_id: Mutex<Option<String>>,
    /// Single-PC school → Server (writes go to `op_log` and are confirmed).
    pub device_mode: DeviceMode,
    /// HTTP client for the licence service.
    pub http: reqwest::Client,
    /// The freshly generated recovery key, held ONLY between `create_recovery_key`
    /// and `confirm_recovery_key`. Never written to disk.
    pub recovery: Mutex<Option<String>>,
    /// App data directory (DB file, machine-id file fallback).
    pub data_dir: PathBuf,
    /// The DB key (hex) when available — lets the school server open its own WAL
    /// connection to the same DB (`None` when the key is missing).
    pub db_key_hex: Option<String>,
    /// True when the DB key is missing → the UI shows the Recover screen.
    pub key_missing: Mutex<bool>,
    /// Fired to stop the school server + relay tunnel when this PC is fenced out
    /// (licence `moved`, P05 Step 5). The LAN accept loop and the tunnel both watch it.
    pub server_stop: std::sync::Arc<tokio::sync::Notify>,
}

impl RtCtx {
    /// Run `f` with the open connection, or error if the DB is not available.
    pub fn with_db<T>(&self, f: impl FnOnce(&mut Connection) -> CmdResult<T>) -> CmdResult<T> {
        let mut guard = self.db.lock().map_err(|_| CmdError::internal("db lock poisoned"))?;
        match guard.as_mut() {
            Some(conn) => f(conn),
            None => Err(CmdError::new("DB_UNAVAILABLE", "error.DB_KEY_MISSING", serde_json::Value::Null)),
        }
    }

    /// The current session, or a FORBIDDEN error when locked (§5: only an active
    /// signed-in staff member may act).
    pub fn require_session(&self) -> CmdResult<SessionStaff> {
        self.session
            .lock()
            .map_err(|_| CmdError::internal("session lock poisoned"))?
            .clone()
            .ok_or_else(|| CmdError::new("LOCKED", "error.LOCKED", serde_json::Value::Null))
    }

    /// The DB file path inside the app data dir.
    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("vidya.db")
    }
}
