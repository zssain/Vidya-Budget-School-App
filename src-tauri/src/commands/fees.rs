//! Fee commands. Each resolves the actor, calls `FeeService`, and returns a DTO.

use tauri::State;

use vidya_services::services::fees::{
    AlertDto, CollectInput, DayBookDto, FeeAccountDto, FeeFilter, FeeRegisterDto, FeeService, ReceiptDto,
};

use super::error::AppError;
use super::with_actor;
use crate::state::AppState;

#[tauri::command]
pub async fn fee_register(
    state: State<'_, AppState>,
    token: String,
    filter: FeeFilter,
) -> Result<FeeRegisterDto, AppError> {
    with_actor(state, token, move |services, actor| {
        FeeService::new(services).register(actor, filter)
    })
    .await
}

#[tauri::command]
pub async fn get_fee_account(
    state: State<'_, AppState>,
    token: String,
    student_id: String,
) -> Result<FeeAccountDto, AppError> {
    with_actor(state, token, move |services, actor| {
        FeeService::new(services).account(actor, &student_id)
    })
    .await
}

#[tauri::command]
pub async fn collect_fee(
    state: State<'_, AppState>,
    token: String,
    input: CollectInput,
) -> Result<ReceiptDto, AppError> {
    with_actor(state, token, move |services, actor| {
        FeeService::new(services).collect(actor, input)
    })
    .await
}

#[tauri::command]
pub async fn get_receipt(
    state: State<'_, AppState>,
    token: String,
    receipt_id: String,
) -> Result<ReceiptDto, AppError> {
    with_actor(state, token, move |services, actor| {
        FeeService::new(services).get_receipt(actor, &receipt_id)
    })
    .await
}

#[tauri::command]
pub async fn cancel_receipt(
    state: State<'_, AppState>,
    token: String,
    receipt_id: String,
    reason: String,
) -> Result<ReceiptDto, AppError> {
    with_actor(state, token, move |services, actor| {
        FeeService::new(services).cancel(actor, &receipt_id, &reason)
    })
    .await
}

#[tauri::command]
pub async fn day_book(
    state: State<'_, AppState>,
    token: String,
    date: String,
) -> Result<DayBookDto, AppError> {
    with_actor(state, token, move |services, actor| {
        FeeService::new(services).day_book(actor, &date)
    })
    .await
}

#[tauri::command]
pub async fn list_alerts(state: State<'_, AppState>, token: String) -> Result<Vec<AlertDto>, AppError> {
    with_actor(state, token, move |services, actor| {
        FeeService::new(services).list_alerts(actor)
    })
    .await
}

#[tauri::command]
pub async fn resolve_alert(
    state: State<'_, AppState>,
    token: String,
    alert_id: String,
) -> Result<(), AppError> {
    with_actor(state, token, move |services, actor| {
        FeeService::new(services).resolve_alert(actor, &alert_id)
    })
    .await
}
