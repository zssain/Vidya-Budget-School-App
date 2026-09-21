//! Student commands. Each resolves the actor, calls `StudentService`, and
//! returns a role-shaped DTO.

use tauri::State;

use vidya_services::services::students::{
    StudentDetailDto, StudentFilter, StudentInput, StudentListDto, StudentService, UpdateStudentInput,
};

use super::error::AppError;
use super::with_actor;
use crate::state::AppState;

#[tauri::command]
pub async fn list_students(
    state: State<'_, AppState>,
    token: String,
    filter: StudentFilter,
) -> Result<StudentListDto, AppError> {
    with_actor(state, token, move |services, actor| {
        StudentService::new(services).list(actor, filter)
    })
    .await
}

#[tauri::command]
pub async fn get_student(
    state: State<'_, AppState>,
    token: String,
    student_id: String,
) -> Result<StudentDetailDto, AppError> {
    with_actor(state, token, move |services, actor| {
        StudentService::new(services).get(actor, &student_id)
    })
    .await
}

#[tauri::command]
pub async fn add_student(
    state: State<'_, AppState>,
    token: String,
    input: StudentInput,
) -> Result<StudentDetailDto, AppError> {
    with_actor(state, token, move |services, actor| {
        StudentService::new(services).add(actor, input)
    })
    .await
}

#[tauri::command]
pub async fn update_student(
    state: State<'_, AppState>,
    token: String,
    input: UpdateStudentInput,
) -> Result<StudentDetailDto, AppError> {
    with_actor(state, token, move |services, actor| {
        StudentService::new(services).update(actor, input)
    })
    .await
}

#[tauri::command]
pub async fn mark_student_left(
    state: State<'_, AppState>,
    token: String,
    student_id: String,
    left_on: String,
    reason: String,
) -> Result<StudentDetailDto, AppError> {
    with_actor(state, token, move |services, actor| {
        StudentService::new(services).mark_left(actor, &student_id, &left_on, &reason)
    })
    .await
}
