//! Attendance commands. Each resolves the actor, calls `AttendanceService`, and
//! returns a DTO.

use tauri::State;

use vidya_services::services::attendance::{
    AttendanceService, AttendanceSheetDto, RegisterDto, SaveAttendanceInput,
};

use super::error::AppError;
use super::with_actor;
use crate::state::AppState;

#[tauri::command]
pub async fn get_attendance(
    state: State<'_, AppState>,
    token: String,
    section_id: String,
    date: String,
) -> Result<AttendanceSheetDto, AppError> {
    with_actor(state, token, move |services, actor| {
        AttendanceService::new(services).sheet(actor, &section_id, &date)
    })
    .await
}

#[tauri::command]
pub async fn save_attendance(
    state: State<'_, AppState>,
    token: String,
    input: SaveAttendanceInput,
) -> Result<AttendanceSheetDto, AppError> {
    with_actor(state, token, move |services, actor| {
        AttendanceService::new(services).save(actor, input)
    })
    .await
}

#[tauri::command]
pub async fn attendance_register(
    state: State<'_, AppState>,
    token: String,
    section_id: String,
    month: String,
) -> Result<RegisterDto, AppError> {
    with_actor(state, token, move |services, actor| {
        AttendanceService::new(services).register(actor, &section_id, &month)
    })
    .await
}
