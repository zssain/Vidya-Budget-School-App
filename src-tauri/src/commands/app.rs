//! App status command (P2.4) — the first real Tauri command.

use super::error::AppError;
use crate::state::{AppState, StartupState};

/// Startup and school status reported to the React app on launch.
///
/// P2.7 and P4.2 add `licensed`, `serverAllowed` and the school name/code once
/// the license and service layers exist.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatusDto {
    /// The encrypted database is open and the app can run.
    pub ready: bool,
    /// When not ready, the `StartupFailure` identifier for the error screen.
    pub failure: Option<String>,
    /// A school row exists (setup is complete).
    pub has_school: bool,
    pub platform: String,
    pub version: String,
}

#[tauri::command]
pub async fn app_status(state: tauri::State<'_, AppState>) -> Result<AppStatusDto, AppError> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || build_status(&state))
        .await
        .map_err(|e| AppError::internal(format!("startup task failed: {e}")))?
}

/// Wait for the start sequence to finish, then report the result. Runs on a
/// blocking thread so the waiting condvar never stalls the UI thread.
fn build_status(state: &AppState) -> Result<AppStatusDto, AppError> {
    let version = env!("CARGO_PKG_VERSION").to_owned();
    match state.wait() {
        StartupState::Ready(core) => {
            let has_school = core
                .db
                .read(|conn| {
                    let count: i64 = conn.query_row("SELECT count(*) FROM school", [], |row| row.get(0))?;
                    Ok(count > 0)
                })
                .map_err(|e| AppError::internal(format!("school count failed: {e}")))?;
            Ok(AppStatusDto {
                ready: true,
                failure: None,
                has_school,
                platform: core.platform.name().to_owned(),
                version,
            })
        }
        StartupState::Failed { kind, detail_for_log } => {
            tauri_plugin_log::log::error!("startup failed ({}): {detail_for_log}", kind.as_str());
            Ok(AppStatusDto {
                ready: false,
                failure: Some(kind.as_str().to_owned()),
                has_school: false,
                platform: std::env::consts::OS.to_owned(),
                version,
            })
        }
    }
}
