//! Settings write commands (principal, office computer only). Each resolves the
//! actor, calls `SettingsService`, and returns the fresh `SettingsDto`.

use tauri::State;

use vidya_services::services::dto::SettingsDto;
use vidya_services::services::settings::{
    AppSettingsInput, ClassesInput, ExamsInput, FeePlanInput, GradeScaleInput, SchoolInput, SettingsService,
    SubjectsInput,
};

use super::error::AppError;
use super::with_actor;
use crate::state::AppState;

macro_rules! settings_write {
    ($name:ident, $input:ty, $method:ident) => {
        #[tauri::command]
        pub async fn $name(
            state: State<'_, AppState>,
            token: String,
            input: $input,
        ) -> Result<SettingsDto, AppError> {
            with_actor(state, token, move |services, actor| {
                SettingsService { services }.$method(actor, input)
            })
            .await
        }
    };
}

settings_write!(save_school, SchoolInput, save_school);
settings_write!(save_classes, ClassesInput, save_classes);
settings_write!(save_fee_plan, FeePlanInput, save_fee_plan);
settings_write!(save_subjects, SubjectsInput, save_subjects);
settings_write!(save_exams, ExamsInput, save_exams);
settings_write!(save_grade_scale, GradeScaleInput, save_grade_scale);
settings_write!(save_app_settings, AppSettingsInput, save_app_settings);

#[tauri::command]
pub async fn save_device_code(
    state: State<'_, AppState>,
    token: String,
    code: String,
) -> Result<SettingsDto, AppError> {
    with_actor(state, token, move |services, actor| {
        SettingsService { services }.save_device_code(actor, &code)
    })
    .await
}
