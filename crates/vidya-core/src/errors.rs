//! Error codes for every business rule (prompts/P02 Step 2). Each variant maps
//! to a stable machine code and an i18n key `error.<CODE>`.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CoreError {
    #[error("locked")]
    Locked,
    #[error("forbidden: {reason}")]
    Forbidden { reason: String },
    #[error("not found")]
    NotFound,
    #[error("validation failed for {field}: {rule}")]
    Validation { field: String, rule: String },
    #[error("amount exceeds due")]
    AmountExceedsDue,
    #[error("no dues")]
    NoDues,
    #[error("sheet locked")]
    SheetLocked,
    #[error("incomplete sheet: {remaining} not marked")]
    IncompleteSheet { remaining: u32 },
    #[error("a request is already pending")]
    RequestAlreadyPending,
    #[error("request is stale")]
    RequestStale,
    #[error("duplicate admission number")]
    DuplicateAdmissionNo,
    #[error("licence invalid")]
    LicenceInvalid,
    #[error("licence is for another computer")]
    LicenceOtherMachine,
    #[error("licence revoked")]
    LicenceRevoked,
    #[error("licence moved")]
    LicenceMoved,
    #[error("licence limit reached: {what}")]
    LicenceLimit { what: String },
    #[error("wrong pin, {remaining} tries remaining")]
    PinWrong { remaining: u32 },
    #[error("pin locked until {until}")]
    PinLocked { until: String },
    #[error("session is read-only")]
    SessionReadOnly,
    #[error("device lease expired")]
    LeaseExpired,
    #[error("server epoch is old")]
    EpochOld,
    #[error("epoch marker could not be verified")]
    EpochInvalid,
    #[error("module off: {module}")]
    ModuleOff { module: String },
    #[error("internal error: {id}")]
    Internal { id: String },
}

impl CoreError {
    /// The stable machine code (matches the JSON `code` tag).
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::Locked => "LOCKED",
            CoreError::Forbidden { .. } => "FORBIDDEN",
            CoreError::NotFound => "NOT_FOUND",
            CoreError::Validation { .. } => "VALIDATION",
            CoreError::AmountExceedsDue => "AMOUNT_EXCEEDS_DUE",
            CoreError::NoDues => "NO_DUES",
            CoreError::SheetLocked => "SHEET_LOCKED",
            CoreError::IncompleteSheet { .. } => "INCOMPLETE_SHEET",
            CoreError::RequestAlreadyPending => "REQUEST_ALREADY_PENDING",
            CoreError::RequestStale => "REQUEST_STALE",
            CoreError::DuplicateAdmissionNo => "DUPLICATE_ADMISSION_NO",
            CoreError::LicenceInvalid => "LICENCE_INVALID",
            CoreError::LicenceOtherMachine => "LICENCE_OTHER_MACHINE",
            CoreError::LicenceRevoked => "LICENCE_REVOKED",
            CoreError::LicenceMoved => "LICENCE_MOVED",
            CoreError::LicenceLimit { .. } => "LICENCE_LIMIT",
            CoreError::PinWrong { .. } => "PIN_WRONG",
            CoreError::PinLocked { .. } => "PIN_LOCKED",
            CoreError::SessionReadOnly => "SESSION_READ_ONLY",
            CoreError::LeaseExpired => "LEASE_EXPIRED",
            CoreError::EpochOld => "EPOCH_OLD",
            CoreError::EpochInvalid => "EPOCH_INVALID",
            CoreError::ModuleOff { .. } => "MODULE_OFF",
            CoreError::Internal { .. } => "INTERNAL",
        }
    }

    /// The i18n key for this error, e.g. `error.AMOUNT_EXCEEDS_DUE`.
    pub fn i18n_key(&self) -> String {
        format!("error.{}", self.code())
    }

    /// Convenience constructor for a validation error.
    pub fn validation(field: impl Into<String>, rule: impl Into<String>) -> CoreError {
        CoreError::Validation { field: field.into(), rule: rule.into() }
    }
}

pub type CoreResult<T> = Result<T, CoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_and_i18n_key() {
        assert_eq!(CoreError::AmountExceedsDue.code(), "AMOUNT_EXCEEDS_DUE");
        assert_eq!(CoreError::AmountExceedsDue.i18n_key(), "error.AMOUNT_EXCEEDS_DUE");
        assert_eq!(
            CoreError::validation("mobile", "pattern").i18n_key(),
            "error.VALIDATION"
        );
    }

    #[test]
    fn serializes_with_code_tag() {
        let j = serde_json::to_value(CoreError::IncompleteSheet { remaining: 4 }).unwrap();
        assert_eq!(j["code"], "INCOMPLETE_SHEET");
        assert_eq!(j["remaining"], 4);
    }
}
