//! The error type every Tauri command returns.
//!
//! Serialized to the shape the frontend `AppError` expects
//! (`{ kind, messageKey, params, message, field }`). P2.7 extends this with
//! `From<ServiceError>` and the signed-in language; for now only an internal
//! variant is needed for the `app_status` command.

use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: String,
    pub message_key: String,
    pub params: BTreeMap<String, String>,
    /// Human-readable, translated at the boundary. Never contains secrets.
    pub message: String,
    pub field: Option<String>,
}

impl AppError {
    /// An internal failure. `detail` is logged (server-side only) and never sent
    /// to the client; the user sees a generic message.
    pub fn internal(detail: impl AsRef<str>) -> Self {
        tauri_plugin_log::log::error!("internal error: {}", detail.as_ref());
        Self {
            kind: "internal".to_owned(),
            message_key: "errors.internal".to_owned(),
            params: BTreeMap::new(),
            message: "Something went wrong. Please try again.".to_owned(),
            field: None,
        }
    }
}
