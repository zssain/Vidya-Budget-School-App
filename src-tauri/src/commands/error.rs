//! The error type every Tauri command returns.
//!
//! Serialized to the shape the frontend `AppError` expects
//! (`{ kind, messageKey, params, message, field }`). Built from a
//! `ServiceError` and translated into the actor's language at the boundary.

use std::collections::BTreeMap;

use vidya_core::roles::Lang;
use vidya_services::ServiceError;

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

    /// Builds the client error from a service error, translated into `lang`.
    pub fn from_service(error: ServiceError, lang: Lang) -> Self {
        let dto = error.to_dto(lang);
        let kind = serde_json::to_value(dto.kind)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| "internal".to_owned());
        Self {
            kind,
            message_key: dto.message_key,
            params: dto.params,
            message: dto.message,
            field: dto.field,
        }
    }
}
