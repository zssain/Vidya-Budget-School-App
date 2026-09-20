//! Tauri command layer. One file per area; each command reads its session
//! token (from P2.7), calls one service method, maps errors to [`AppError`] and
//! returns a DTO. No business rules or SQL live here (see `src-tauri/AGENTS.md`).

pub mod app;
pub mod error;

pub use app::app_status;
pub use error::AppError;
