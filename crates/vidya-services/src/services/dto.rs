//! Data transfer objects returned by the read services. Serialised camelCase for
//! the frontend and the LAN API.

use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchoolHeaderDto {
    pub name: String,
    pub address: String,
    pub udise: String,
    pub board: String,
    pub phone: String,
    pub session_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    pub id: String,
    pub name: String,
    pub starts_on: String,
    pub ends_on: String,
    pub terms: i64,
    pub transport_fee_per_term: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionDto {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubjectDto {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeePlanDto {
    pub tuition: i64,
    pub exam: i64,
    pub other: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassDto {
    pub id: String,
    pub name: String,
    pub sort_order: i64,
    pub sections: Vec<SectionDto>,
    pub subjects: Vec<SubjectDto>,
    pub fee_plan: Option<FeePlanDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamDto {
    pub id: String,
    pub name: String,
    pub max_marks: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GradeBandDto {
    pub grade: String,
    pub min_percent: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub school: SchoolHeaderDto,
    pub session: Option<SessionDto>,
    pub classes: Vec<ClassDto>,
    pub exams: Vec<ExamDto>,
    pub grade_scale: Vec<GradeBandDto>,
    pub app_settings: BTreeMap<String, String>,
}
