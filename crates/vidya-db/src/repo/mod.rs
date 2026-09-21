//! Repositories: one module per table group. Plain functions taking
//! `&Connection` (reads) or `&Transaction` (writes), typed row structs, and
//! parameterised SQL only. No business rules and no permission checks live here
//! (see `crates/AGENTS.md`); services authorize and validate before calling.

pub mod alerts;
pub mod app_settings;
pub mod attendance;
pub mod cancellations;
pub mod classes;
pub mod enrollments;
pub mod exams;
pub mod fee_plans;
pub mod grade_scale;
pub mod license;
pub mod marks;
pub mod meta;
pub mod receipts;
pub mod school;
pub mod sessions;
pub mod students;
pub mod subjects;
pub mod users;
