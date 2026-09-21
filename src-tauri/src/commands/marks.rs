//! Marks and report-card commands. Each resolves the actor, calls
//! `MarksService`, and returns a DTO.

use tauri::State;

use vidya_services::services::marks::{MarksService, MarksSheetDto, ReportCardDto, SaveMarksInput};

use super::error::AppError;
use super::with_actor;
use crate::state::AppState;

#[tauri::command]
pub async fn get_marks_sheet(
    state: State<'_, AppState>,
    token: String,
    exam_id: String,
    section_id: String,
) -> Result<MarksSheetDto, AppError> {
    with_actor(state, token, move |services, actor| {
        MarksService::new(services).sheet(actor, &exam_id, &section_id)
    })
    .await
}

#[tauri::command]
pub async fn save_marks(
    state: State<'_, AppState>,
    token: String,
    input: SaveMarksInput,
) -> Result<MarksSheetDto, AppError> {
    with_actor(state, token, move |services, actor| {
        MarksService::new(services).save(actor, input)
    })
    .await
}

#[tauri::command]
pub async fn get_report_card(
    state: State<'_, AppState>,
    token: String,
    student_id: String,
) -> Result<ReportCardDto, AppError> {
    with_actor(state, token, move |services, actor| {
        MarksService::new(services).report_card(actor, &student_id)
    })
    .await
}

#[tauri::command]
pub async fn get_class_report_cards(
    state: State<'_, AppState>,
    token: String,
    section_id: String,
) -> Result<Vec<ReportCardDto>, AppError> {
    with_actor(state, token, move |services, actor| {
        MarksService::new(services).class_report_cards(actor, &section_id)
    })
    .await
}
