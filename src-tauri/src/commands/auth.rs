//! Auth and account commands, and settings reading. Desktop callers always use
//! `OfficeComputer` origin (the webview cannot choose it).

use tauri::State;

use vidya_core::roles::{Lang, Origin};
use vidya_services::auth::{AuthService, CurrentUserDto, SignInResult};
use vidya_services::services::dto::SettingsDto;
use vidya_services::{ServiceError, Services};

use super::error::AppError;
use super::{services_of, with_actor, without_actor};
use crate::state::AppState;

#[derive(serde::Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum SignInDto {
    #[serde(rename = "ok")]
    Ok { token: String, user: CurrentUserDto },
    #[serde(rename = "must_change_password")]
    MustChange { pending_token: String },
}

impl From<SignInResult> for SignInDto {
    fn from(result: SignInResult) -> Self {
        match result {
            SignInResult::Ok { token, user } => SignInDto::Ok { token, user },
            SignInResult::MustChangePassword { pending_token } => SignInDto::MustChange { pending_token },
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstPasswordDto {
    pub token: String,
    pub user: CurrentUserDto,
}

fn lang_from(language: &str) -> Lang {
    if language == "hi" {
        Lang::Hi
    } else {
        Lang::En
    }
}

#[tauri::command]
pub async fn sign_in(
    state: State<'_, AppState>,
    username: String,
    password: String,
) -> Result<SignInDto, AppError> {
    without_actor(state, move |services: &Services, sessions| {
        let auth = AuthService::new(services, sessions);
        let result = auth.sign_in(&username, &password, &services.device_id, Origin::OfficeComputer)?;
        Ok(SignInDto::from(result))
    })
    .await
}

#[tauri::command]
pub async fn set_first_password(
    state: State<'_, AppState>,
    pending_token: String,
    new_password: String,
) -> Result<FirstPasswordDto, AppError> {
    without_actor(state, move |services, sessions| {
        let auth = AuthService::new(services, sessions);
        match auth.set_first_password(&pending_token, &new_password)? {
            SignInResult::Ok { token, user } => Ok(FirstPasswordDto { token, user }),
            SignInResult::MustChangePassword { .. } => Err(ServiceError::internal("unexpected pending")),
        }
    })
    .await
}

#[tauri::command]
pub fn sign_out(state: State<'_, AppState>, token: String) -> Result<(), AppError> {
    state.sessions().end(&token);
    Ok(())
}

#[tauri::command]
pub async fn current_user(
    state: State<'_, AppState>,
    token: String,
) -> Result<Option<CurrentUserDto>, AppError> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Option<CurrentUserDto>, AppError> {
        let services = services_of(&state)?;
        let sessions = state.sessions();
        let auth = AuthService::new(&services, &sessions);
        // A missing or expired session is "not signed in" (null), not an error.
        match auth.actor_for_token(&token) {
            Ok(actor) => {
                let lang = actor.lang;
                auth.current_user_dto(&actor)
                    .map(Some)
                    .map_err(|e| AppError::from_service(e, lang))
            }
            Err(_) => Ok(None),
        }
    })
    .await
    .map_err(|e| AppError::internal(format!("task join: {e}")))?
}

#[tauri::command]
pub async fn change_password(
    state: State<'_, AppState>,
    token: String,
    current_password: String,
    new_password: String,
) -> Result<(), AppError> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), AppError> {
        let services = services_of(&state)?;
        let sessions = state.sessions();
        let auth = AuthService::new(&services, &sessions);
        let actor = auth
            .actor_for_token(&token)
            .map_err(|e| AppError::from_service(e, Lang::En))?;
        let lang = actor.lang;
        auth.change_password(&actor, &current_password, &new_password, &token)
            .map_err(|e| AppError::from_service(e, lang))
    })
    .await
    .map_err(|e| AppError::internal(format!("task join: {e}")))?
}

#[tauri::command]
pub async fn set_language(
    state: State<'_, AppState>,
    token: String,
    language: String,
) -> Result<(), AppError> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), AppError> {
        let services = services_of(&state)?;
        let sessions = state.sessions();
        let auth = AuthService::new(&services, &sessions);
        let actor = auth
            .actor_for_token(&token)
            .map_err(|e| AppError::from_service(e, Lang::En))?;
        let lang = actor.lang;
        auth.set_language(&actor, lang_from(&language))
            .map_err(|e| AppError::from_service(e, lang))
    })
    .await
    .map_err(|e| AppError::internal(format!("task join: {e}")))?
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>, token: String) -> Result<SettingsDto, AppError> {
    with_actor(state, token, |services, actor| services.settings().get(actor)).await
}
