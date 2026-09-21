//! Desktop background tasks. P2.7 adds the idle-session watcher; later prompts
//! add the LAN server, backup scheduler and keep-awake here.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager};
use vidya_db::repo;

use crate::state::{AppState, StartResult};

const DEFAULT_TIMEOUT_MINUTES: i64 = 30;

/// Every 60 seconds, expire idle sessions. If any ended, emit `session-expired`
/// so the frontend returns a signed-in user to the login screen.
pub fn spawn_idle_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let state = app.state::<AppState>().inner().clone();
        loop {
            std::thread::sleep(Duration::from_secs(60));
            let StartResult::Ready { services, .. } = state.wait() else {
                continue;
            };
            let minutes = services
                .db
                .read(|conn| repo::app_settings::get(conn, "session_timeout_minutes"))
                .ok()
                .flatten()
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(DEFAULT_TIMEOUT_MINUTES)
                .max(1);
            let timeout_ms = minutes as u64 * 60_000;
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            if !state.sessions().expire_idle(now_ms, timeout_ms).is_empty() {
                let _ = app.emit("session-expired", ());
            }
        }
    });
}
