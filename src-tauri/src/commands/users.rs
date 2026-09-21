//! Staff login commands (principal only; office computer only). Each resolves
//! the actor, builds `UserService` and returns a DTO.

use tauri::State;

use vidya_services::services::users::{
    CreateUserInput, CredentialSlipDto, UpdateUserInput, UserDto, UserService,
};

use super::error::AppError;
use super::with_actor_sessions;
use crate::state::AppState;

#[tauri::command]
pub async fn list_users(state: State<'_, AppState>, token: String) -> Result<Vec<UserDto>, AppError> {
    with_actor_sessions(state, token, |services, sessions, actor| {
        UserService::new(services, sessions).list(actor)
    })
    .await
}

#[tauri::command]
pub async fn create_user(
    state: State<'_, AppState>,
    token: String,
    input: CreateUserInput,
) -> Result<CredentialSlipDto, AppError> {
    with_actor_sessions(state, token, move |services, sessions, actor| {
        UserService::new(services, sessions).create(actor, input)
    })
    .await
}

#[tauri::command]
pub async fn update_user(
    state: State<'_, AppState>,
    token: String,
    input: UpdateUserInput,
) -> Result<UserDto, AppError> {
    with_actor_sessions(state, token, move |services, sessions, actor| {
        UserService::new(services, sessions).update(actor, input)
    })
    .await
}

#[tauri::command]
pub async fn reset_user_password(
    state: State<'_, AppState>,
    token: String,
    user_id: String,
) -> Result<CredentialSlipDto, AppError> {
    with_actor_sessions(state, token, move |services, sessions, actor| {
        UserService::new(services, sessions).reset_password(actor, &user_id)
    })
    .await
}

#[tauri::command]
pub async fn unlock_user(
    state: State<'_, AppState>,
    token: String,
    user_id: String,
) -> Result<UserDto, AppError> {
    with_actor_sessions(state, token, move |services, sessions, actor| {
        UserService::new(services, sessions).unlock(actor, &user_id)
    })
    .await
}

#[tauri::command]
pub async fn set_user_active(
    state: State<'_, AppState>,
    token: String,
    user_id: String,
    active: bool,
) -> Result<UserDto, AppError> {
    with_actor_sessions(state, token, move |services, sessions, actor| {
        UserService::new(services, sessions).set_active(actor, &user_id, active)
    })
    .await
}
