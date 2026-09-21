//! Tauri command layer. Each command reads its session token, calls one service
//! method through [`with_actor`]/[`without_actor`], maps errors to [`AppError`]
//! and returns a DTO. No business rules or SQL live here (see `src-tauri/AGENTS.md`).

pub mod app;
pub mod auth;
pub mod error;

pub use error::AppError;

use std::sync::Arc;

use vidya_core::roles::{Actor, Lang};
use vidya_services::auth::{AuthService, SessionStore};
use vidya_services::{ServiceError, Services};

use crate::state::{AppState, StartResult};

/// Waits for startup and returns the built services, or an error if startup failed.
pub(crate) fn services_of(state: &AppState) -> Result<Arc<Services>, AppError> {
    match state.wait() {
        StartResult::Ready { services, .. } => Ok(services),
        StartResult::Failed { kind, .. } => {
            Err(AppError::internal(format!("startup failed: {}", kind.as_str())))
        }
    }
}

/// Runs `f` with the signed-in actor, resolved from `token` on a blocking thread
/// so the window never freezes. Desktop callers always use `OfficeComputer`
/// origin; the webview cannot choose it.
pub async fn with_actor<T: Send + 'static>(
    state: tauri::State<'_, AppState>,
    token: String,
    f: impl FnOnce(&Services, &Actor) -> Result<T, ServiceError> + Send + 'static,
) -> Result<T, AppError> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<T, AppError> {
        let services = services_of(&state)?;
        let sessions = state.sessions();
        let auth = AuthService::new(&services, &sessions);
        let actor = auth
            .actor_for_token(&token)
            .map_err(|e| AppError::from_service(e, Lang::En))?;
        let lang = actor.lang;
        f(&services, &actor).map_err(|e| AppError::from_service(e, lang))
    })
    .await
    .map_err(|e| AppError::internal(format!("task join: {e}")))?
}

/// Runs `f` without a session (sign-in, first password, sample school).
pub async fn without_actor<T: Send + 'static>(
    state: tauri::State<'_, AppState>,
    f: impl FnOnce(&Services, &SessionStore) -> Result<T, ServiceError> + Send + 'static,
) -> Result<T, AppError> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<T, AppError> {
        let services = services_of(&state)?;
        let sessions = state.sessions();
        f(&services, &sessions).map_err(|e| AppError::from_service(e, Lang::En))
    })
    .await
    .map_err(|e| AppError::internal(format!("task join: {e}")))?
}
