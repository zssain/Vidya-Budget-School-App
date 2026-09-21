//! `ServiceError` — the failure type every service method returns.
//!
//! It carries a stable `kind`, a translation `message_key`, `params`, an
//! optional `field`, and an optional `source` (logged, never sent to clients).
//! Database and SQL details never leak into `message_key` or `params`.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;
use vidya_core::error::{DomainError, ErrorKind};
use vidya_core::i18n;
use vidya_core::roles::Lang;
use vidya_db::DbError;

#[derive(Debug, thiserror::Error)]
#[error("{message_key}")]
pub struct ServiceError {
    pub kind: ErrorKind,
    pub message_key: String,
    pub params: BTreeMap<String, String>,
    pub field: Option<String>,
    #[source]
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

/// The serialised error the command/HTTP boundary returns to a client.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDto {
    pub kind: ErrorKind,
    pub message_key: String,
    pub params: BTreeMap<String, String>,
    pub message: String,
    pub field: Option<String>,
}

static REF_COUNTER: AtomicU32 = AtomicU32::new(1);

/// A short, human-readable support reference like `ERR-7K2Q`.
fn new_reference() -> String {
    let n = REF_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mixed = n.wrapping_mul(2_654_435_761);
    // Crockford-style alphabet without easily confused letters.
    const ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut reference = String::from("ERR-");
    for i in 0..4 {
        reference.push(ALPHABET[((mixed >> (i * 5)) & 31) as usize] as char);
    }
    reference
}

impl ServiceError {
    pub fn new(kind: ErrorKind, message_key: impl Into<String>) -> Self {
        Self {
            kind,
            message_key: message_key.into(),
            params: BTreeMap::new(),
            field: None,
            source: None,
        }
    }

    /// A validation error.
    pub fn validation(message_key: impl Into<String>) -> Self {
        Self::new(ErrorKind::Validation, message_key)
    }

    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    pub fn param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(name.into(), value.into());
        self
    }

    /// An internal failure. The source is logged with a short reference; the
    /// client sees only a generic message carrying that reference.
    pub fn internal(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        let source = source.into();
        let reference = new_reference();
        tracing::error!(reference = %reference, error = %source, "internal service error");
        let mut params = BTreeMap::new();
        params.insert("ref".to_owned(), reference);
        Self {
            kind: ErrorKind::Internal,
            message_key: "errors.internal".to_owned(),
            params,
            field: None,
            source: Some(source),
        }
    }

    /// Translates into the client-facing DTO for the given language.
    pub fn to_dto(&self, lang: Lang) -> ErrorDto {
        ErrorDto {
            kind: self.kind,
            message_key: self.message_key.clone(),
            params: self.params.clone(),
            message: i18n::message(lang, &self.message_key, &self.params),
            field: self.field.clone(),
        }
    }
}

impl From<DomainError> for ServiceError {
    fn from(error: DomainError) -> Self {
        Self {
            kind: error.kind,
            message_key: error.message_key.to_owned(),
            params: error.params,
            field: error.field.map(|f| f.to_owned()),
            source: None,
        }
    }
}

impl From<DbError> for ServiceError {
    fn from(error: DbError) -> Self {
        match error {
            // Never expose SQL text; map to safe internal messages.
            DbError::WrongKey => {
                let mut params = BTreeMap::new();
                params.insert("ref".to_owned(), new_reference());
                Self {
                    kind: ErrorKind::Internal,
                    message_key: "db.error.unlock".to_owned(),
                    params,
                    field: None,
                    source: Some(Box::new(error)),
                }
            }
            other => ServiceError::internal(Box::new(other)),
        }
    }
}
