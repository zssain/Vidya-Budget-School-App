//! The command error shape returned to the UI: `{ code, message_key, vars }`
//! (prompts/P03 Step 5). vidya-core errors, licence errors, DB errors and key
//! errors all convert into this so the frontend can look up i18n copy by
//! `message_key` and interpolate `vars`.

use serde::Serialize;
use vidya_core::errors::CoreError;

use crate::licence::LicenceError;
use crate::security::keys::KeyError;

/// A structured, serializable error for a Tauri command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CmdError {
    /// Stable machine code (e.g. `AMOUNT_EXCEEDS_DUE`, `FORBIDDEN`, `CODE_ALREADY_USED`).
    pub code: String,
    /// i18n key the UI resolves to copy (e.g. `error.AMOUNT_EXCEEDS_DUE`).
    pub message_key: String,
    /// Interpolation variables for the message (e.g. `{ "remaining": 4 }`).
    pub vars: serde_json::Value,
}

impl CmdError {
    pub fn new(code: impl Into<String>, message_key: impl Into<String>, vars: serde_json::Value) -> Self {
        Self { code: code.into(), message_key: message_key.into(), vars }
    }

    /// A generic internal error (never leaks details to the UI copy).
    pub fn internal(id: impl Into<String>) -> Self {
        Self::new("INTERNAL", "error.INTERNAL", serde_json::json!({ "id": id.into() }))
    }

    /// A forbidden error with a stable reason code.
    pub fn forbidden(reason: impl Into<String>) -> Self {
        Self::new("FORBIDDEN", "error.FORBIDDEN", serde_json::json!({ "reason": reason.into() }))
    }

    /// A not-found error.
    pub fn not_found() -> Self {
        Self::new("NOT_FOUND", "error.NOT_FOUND", serde_json::Value::Null)
    }

    /// A validation error for a field.
    pub fn validation(field: impl Into<String>, rule: impl Into<String>) -> Self {
        Self::new(
            "VALIDATION",
            "error.VALIDATION",
            serde_json::json!({ "field": field.into(), "rule": rule.into() }),
        )
    }
}

impl From<CoreError> for CmdError {
    fn from(e: CoreError) -> Self {
        let code = e.code().to_string();
        let message_key = e.i18n_key();
        // Serialize the variant to capture its fields as `vars`.
        let vars = serde_json::to_value(&e).unwrap_or(serde_json::Value::Null);
        CmdError { code, message_key, vars }
    }
}

impl From<LicenceError> for CmdError {
    fn from(e: LicenceError) -> Self {
        CmdError::new(e.code(), e.message_key(), serde_json::Value::Null)
    }
}

impl From<KeyError> for CmdError {
    fn from(e: KeyError) -> Self {
        match e {
            KeyError::Missing => CmdError::new("DB_KEY_MISSING", "error.DB_KEY_MISSING", serde_json::Value::Null),
            KeyError::Backend(m) => CmdError::internal(format!("keystore: {m}")),
        }
    }
}

impl From<rusqlite::Error> for CmdError {
    fn from(e: rusqlite::Error) -> Self {
        CmdError::internal(format!("db: {e}"))
    }
}

pub type CmdResult<T> = Result<T, CmdError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_maps_code_key_and_vars() {
        let e: CmdError = CoreError::IncompleteSheet { remaining: 4 }.into();
        assert_eq!(e.code, "INCOMPLETE_SHEET");
        assert_eq!(e.message_key, "error.INCOMPLETE_SHEET");
        assert_eq!(e.vars["remaining"], 4);
    }

    #[test]
    fn licence_error_maps() {
        let e: CmdError = LicenceError::CodeAlreadyUsed.into();
        assert_eq!(e.code, "CODE_ALREADY_USED");
        assert_eq!(e.message_key, "licence.code_already_used");
    }

    #[test]
    fn key_missing_maps() {
        let e: CmdError = KeyError::Missing.into();
        assert_eq!(e.code, "DB_KEY_MISSING");
    }
}
