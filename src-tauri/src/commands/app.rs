//! App status and (debug-only) sample-school commands.

use super::error::AppError;
use crate::state::{AppState, StartResult};
use vidya_db::repo;

/// Startup and school status reported to the React app on launch.
///
/// `licensed`/`schoolName`/`schoolCode` are read once the database is open;
/// `serverAllowed` stays `false` until P4.2 issues the server permit.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatusDto {
    pub ready: bool,
    pub failure: Option<String>,
    pub has_school: bool,
    pub licensed: bool,
    pub server_allowed: bool,
    pub school_name: Option<String>,
    pub school_code: Option<String>,
    pub platform: String,
    pub version: String,
}

#[tauri::command]
pub async fn app_status(state: tauri::State<'_, AppState>) -> Result<AppStatusDto, AppError> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || build_status(&state))
        .await
        .map_err(|e| AppError::internal(format!("task join: {e}")))?
}

fn build_status(state: &AppState) -> Result<AppStatusDto, AppError> {
    let version = env!("CARGO_PKG_VERSION").to_owned();
    match state.wait() {
        StartResult::Ready { core, .. } => {
            let (has_school, school_name, school_code, licensed) = core
                .db
                .read(|conn| {
                    let school = repo::school::get(conn)?;
                    let license = repo::license::get(conn)?;
                    let code = license.as_ref().map(|l| l.school_code.clone());
                    Ok((school.is_some(), school.map(|s| s.name), code, license.is_some()))
                })
                .map_err(|e| AppError::internal(format!("status read: {e}")))?;
            Ok(AppStatusDto {
                ready: true,
                failure: None,
                has_school,
                licensed,
                server_allowed: false,
                school_name,
                school_code,
                platform: core.platform.name().to_owned(),
                version,
            })
        }
        StartResult::Failed { kind, detail } => {
            tauri_plugin_log::log::error!("startup failed ({}): {detail}", kind.as_str());
            Ok(AppStatusDto {
                ready: false,
                failure: Some(kind.as_str().to_owned()),
                has_school: false,
                licensed: false,
                server_allowed: false,
                school_name: None,
                school_code: None,
                platform: std::env::consts::OS.to_owned(),
                version,
            })
        }
    }
}

/// Loads the deterministic sample school into the real database. Debug builds
/// only; refuses if a school already exists.
#[cfg(debug_assertions)]
#[tauri::command]
pub async fn load_sample_school(state: tauri::State<'_, AppState>) -> Result<(), AppError> {
    use vidya_core::error::ErrorKind;
    use vidya_services::ServiceError;

    super::without_actor(state, |services, _sessions| {
        let has_school = services.db.read(|conn| Ok(repo::school::get(conn)?.is_some()))?;
        if has_school {
            return Err(ServiceError::new(
                ErrorKind::Conflict,
                "setup.error.not_available",
            ));
        }
        vidya_testkit::SampleSchool::build(services, 1)?;
        Ok(())
    })
    .await
}
