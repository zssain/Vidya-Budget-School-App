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

/// Auto-lock after this much time without user input (§9 default: 5 minutes).
pub const AUTO_LOCK_MS: i64 = 5 * 60 * 1000;

/// Current wall-clock time in unix milliseconds.
pub fn now_ms() -> i64 {
    (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64
}

/// True when the session's last input is older than [`AUTO_LOCK_MS`]. A clock that
/// jumped backwards (last > now) also expires it — fail closed.
pub fn idle_expired(last_activity_ms: i64, now_ms: i64) -> bool {
    now_ms - last_activity_ms > AUTO_LOCK_MS || last_activity_ms > now_ms + 60_000
}

/// The Tauri-managed runtime context.
pub struct RtCtx {
    /// The encrypted connection, opened once the DB key is available. `None` when
    /// the DB key is missing (a DB file exists but the keychain entry is gone).
    pub db: Mutex<Option<Connection>>,
    /// The signed-in staff member, or `None` when locked.
    pub session: Mutex<Option<SessionStaff>>,
    /// Wall-clock ms of the last real user input in the unlocked session (§9
    /// auto-lock). Wall clock, not `Instant`, so time asleep counts toward the idle
    /// limit. Only `session_activity` (user input) and `unlock` move it — background
    /// polling and sync never do.
    pub last_activity_ms: Mutex<i64>,
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

    /// The current session, or a LOCKED error when locked (§5: only an active
    /// signed-in staff member may act). A session idle past [`AUTO_LOCK_MS`] is
    /// dropped here, so the lock holds even if the UI's own timer never fired.
    pub fn require_session(&self) -> CmdResult<SessionStaff> {
        self.current_session()?
            .ok_or_else(|| CmdError::new("LOCKED", "error.LOCKED", serde_json::Value::Null))
    }

    /// The session if one is open and not idle-expired; expires it as a side effect.
    pub fn current_session(&self) -> CmdResult<Option<SessionStaff>> {
        let mut guard = self.session.lock().map_err(|_| CmdError::internal("session lock poisoned"))?;
        if guard.is_some() {
            let last = *self.last_activity_ms.lock().map_err(|_| CmdError::internal("activity lock poisoned"))?;
            if idle_expired(last, now_ms()) {
                *guard = None;
            }
        }
        Ok(guard.clone())
    }

    /// Open a session for `staff` (after a verified PIN) and start the idle clock.
    pub fn open_session(&self, staff: SessionStaff) -> CmdResult<()> {
        *self.last_activity_ms.lock().map_err(|_| CmdError::internal("activity lock poisoned"))? = now_ms();
        *self.session.lock().map_err(|_| CmdError::internal("session lock poisoned"))? = Some(staff);
        Ok(())
    }

    /// Drop the session (lock / switch user).
    pub fn close_session(&self) -> CmdResult<()> {
        *self.session.lock().map_err(|_| CmdError::internal("session lock poisoned"))? = None;
        Ok(())
    }

    /// Record real user input: pushes the idle deadline out, but only while the
    /// session is still live (an expired session stays locked).
    pub fn touch_session(&self) -> CmdResult<()> {
        if self.current_session()?.is_some() {
            *self.last_activity_ms.lock().map_err(|_| CmdError::internal("activity lock poisoned"))? = now_ms();
        }
        Ok(())
    }

    /// The session, required to hold one of `roles` (FORBIDDEN otherwise). For
    /// commands whose data is role-restricted outright (fees, staff admin, backups);
    /// finer class-level scoping stays in each command's logic.
    pub fn require_role(&self, roles: &[&str]) -> CmdResult<SessionStaff> {
        let s = self.require_session()?;
        if roles.contains(&s.role.as_str()) {
            Ok(s)
        } else {
            Err(CmdError::forbidden("not_permitted"))
        }
    }

    /// Setup-wizard commands (create school/session/classes/principal, recovery key,
    /// first PIN) run only while setup is unfinished on a non-client device. Once setup
    /// completes they are refused, so nobody at a locked PC can re-run them (e.g. to
    /// overwrite the Principal's PIN).
    pub fn require_setup_phase(&self) -> CmdResult<()> {
        self.with_db(|conn| {
            if crate::sync::client::is_client(conn) {
                return Err(CmdError::forbidden("setup_finished"));
            }
            let step: i64 = crate::kv::get(conn, crate::state::KV_SETUP_STEP)?.unwrap_or(0);
            if step >= crate::state::SETUP_STEPS {
                return Err(CmdError::forbidden("setup_finished"));
            }
            Ok(())
        })
    }

    /// Commands that replace or bootstrap the whole database (restore, re-activation,
    /// join): open while this device holds no school yet (or its DB key is missing →
    /// Recover); once a school exists only the signed-in Principal may run them.
    pub fn require_no_school_or_principal(&self) -> CmdResult<()> {
        if *self.key_missing.lock().map_err(|_| CmdError::internal("lock"))? {
            return Ok(());
        }
        let has_school = self.with_db(|conn| {
            Ok(conn.query_row("SELECT 1 FROM school LIMIT 1", [], |_| Ok(())).is_ok() || crate::sync::client::is_client(conn))
        })?;
        if has_school {
            self.require_role(&["principal"])?;
        }
        Ok(())
    }

    /// The DB file path inside the app data dir.
    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("vidya.db")
    }

    /// The encrypted attachment store (bill photos, P15). Errors if the DB key is
    /// unavailable (attachments are encrypted with a key derived from it).
    pub fn attachment_store(&self) -> CmdResult<crate::attachments::AttachmentStore> {
        let key = self.db_key_hex.as_deref().ok_or_else(|| CmdError::internal("db key missing"))?;
        Ok(crate::attachments::AttachmentStore::new(&self.data_dir, key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_expiry_is_five_minutes_of_wall_clock() {
        let t0 = 1_700_000_000_000;
        assert!(!idle_expired(t0, t0));
        assert!(!idle_expired(t0, t0 + AUTO_LOCK_MS));
        assert!(idle_expired(t0, t0 + AUTO_LOCK_MS + 1));
        // A laptop asleep for an hour is past the deadline on wake.
        assert!(idle_expired(t0, t0 + 60 * 60 * 1000));
    }

    #[test]
    fn clock_jumping_backwards_fails_closed() {
        let t0 = 1_700_000_000_000;
        assert!(idle_expired(t0, t0 - 10 * 60 * 1000));
    }
}
