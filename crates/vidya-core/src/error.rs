use std::collections::BTreeMap;

use serde::Serialize;

/// Stable error categories used at command, HTTP and sync boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    Validation,
    Permission,
    NotFound,
    Conflict,
    Auth,
    Locked,
    License,
    Offline,
    Internal,
}

/// A localisable domain failure with optional field and message parameters.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[error("{message_key}")]
pub struct DomainError {
    pub kind: ErrorKind,
    pub message_key: &'static str,
    pub params: BTreeMap<String, String>,
    pub field: Option<&'static str>,
}

impl DomainError {
    /// Creates an error with an explicit category and translation key.
    pub fn new(kind: ErrorKind, message_key: &'static str) -> Self {
        Self {
            kind,
            message_key,
            params: BTreeMap::new(),
            field: None,
        }
    }

    /// Creates a validation error.
    pub fn validation(message_key: &'static str) -> Self {
        Self::new(ErrorKind::Validation, message_key)
    }

    /// Associates the error with a form field.
    pub fn field(mut self, field: &'static str) -> Self {
        self.field = Some(field);
        self
    }

    /// Adds a replacement parameter for the translated message.
    pub fn param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(name.into(), value.into());
        self
    }
}
