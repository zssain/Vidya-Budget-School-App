//! guardians — guardian rules (P13, foundation §8.2). Pure: no IO, no clock.
//!
//! v2 moves guardian data out of the inline `student.guardian_*` columns into a
//! separate `guardian` table linked to students by `student_guardian` (one
//! primary). This module holds the validation and the dedup key used to migrate
//! the old columns; the tables, migration and repository live in `src-tauri`.

use crate::errors::{CoreError, CoreResult};

/// Up to two guardians per student, exactly one primary (§8.2).
pub const MAX_GUARDIANS_PER_STUDENT: usize = 2;

/// Allowed guardian message/UI languages (§4a): English, Hindi, Telugu.
pub fn validate_language(s: &str) -> CoreResult<()> {
    match s {
        "en" | "hi" | "te" => Ok(()),
        _ => Err(CoreError::validation("language", "one_of_en_hi_te")),
    }
}

/// A guardian's editable fields, borrowed for validation.
pub struct GuardianInput<'a> {
    pub name: &'a str,
    pub relation: Option<&'a str>,
    pub mobile: Option<&'a str>,
    pub email: Option<&'a str>,
    pub language: &'a str,
    pub whatsapp_ok: bool,
}

/// Validate a guardian and return the trimmed name. `name` is required; `mobile`
/// (if non-empty) must be a valid Indian mobile; `language` must be en|hi|te; an
/// `email` (if non-empty) must look like an address.
pub fn validate_guardian(g: &GuardianInput) -> CoreResult<String> {
    let name = crate::validation::validate_name(g.name)?;
    if let Some(m) = g.mobile.filter(|s| !s.is_empty()) {
        crate::validation::validate_mobile(m)?;
    }
    validate_language(g.language)?;
    if let Some(e) = g.email.filter(|s| !s.is_empty()) {
        // Minimal shape check (no full RFC): one '@', not at either end, a dot after.
        let at = e.find('@');
        let ok = matches!(at, Some(i) if i > 0 && i < e.len() - 1) && e[at.unwrap() + 1..].contains('.');
        if !ok {
            return Err(CoreError::validation("email", "invalid"));
        }
    }
    Ok(name)
}

/// The dedup key for migrating v1 inline guardian columns: two students share one
/// guardian row iff they have the same (mobile, name). Mirrors the `0009`
/// migration's `GROUP BY COALESCE(mobile,''), COALESCE(name,'')` exactly (an
/// empty/absent mobile groups by name alone). No normalisation — an exact match.
pub fn dedup_key(mobile: Option<&str>, name: Option<&str>) -> (String, String) {
    (mobile.unwrap_or("").to_string(), name.unwrap_or("").to_string())
}

/// True if this student row carries any guardian data worth migrating.
pub fn has_guardian_data(mobile: Option<&str>, name: Option<&str>) -> bool {
    !mobile.unwrap_or("").is_empty() || !name.unwrap_or("").is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input<'a>(name: &'a str, mobile: Option<&'a str>, email: Option<&'a str>, lang: &'a str) -> GuardianInput<'a> {
        GuardianInput { name, relation: Some("Father"), mobile, email, language: lang, whatsapp_ok: true }
    }

    #[test]
    fn language_must_be_en_hi_or_te() {
        assert!(validate_language("en").is_ok());
        assert!(validate_language("hi").is_ok());
        assert!(validate_language("te").is_ok());
        assert_eq!(validate_language("fr").unwrap_err().code(), "VALIDATION");
    }

    #[test]
    fn validates_name_mobile_language_email() {
        assert_eq!(validate_guardian(&input("Ramesh Kumar", Some("9876543210"), None, "en")).unwrap(), "Ramesh Kumar");
        // Empty mobile is allowed (optional).
        assert!(validate_guardian(&input("Ramesh Kumar", Some(""), None, "hi")).is_ok());
        // Bad mobile rejected.
        assert_eq!(validate_guardian(&input("Ramesh", Some("12345"), None, "en")).unwrap_err().code(), "VALIDATION");
        // Bad language rejected.
        assert_eq!(validate_guardian(&input("Ramesh", None, None, "xx")).unwrap_err().code(), "VALIDATION");
        // Empty name rejected (via validate_name).
        assert!(validate_guardian(&input("", None, None, "en")).is_err());
        // Bad email rejected; good email accepted.
        assert_eq!(validate_guardian(&input("Ramesh", None, Some("nope"), "en")).unwrap_err().code(), "VALIDATION");
        assert!(validate_guardian(&input("Ramesh", None, Some("a@b.com"), "en")).is_ok());
    }

    #[test]
    fn dedup_groups_by_exact_mobile_and_name() {
        assert_eq!(dedup_key(Some("9876543210"), Some("Ramesh Kumar")), ("9876543210".into(), "Ramesh Kumar".into()));
        // Same mobile + name → same key (siblings share the guardian).
        assert_eq!(
            dedup_key(Some("9876543210"), Some("Ramesh Kumar")),
            dedup_key(Some("9876543210"), Some("Ramesh Kumar"))
        );
        // Different name (same mobile) → different key.
        assert_ne!(
            dedup_key(Some("9876543210"), Some("Ramesh Kumar")),
            dedup_key(Some("9876543210"), Some("Suresh Kumar"))
        );
        // Absent mobile groups by name alone.
        assert_eq!(dedup_key(None, Some("Ramesh")), ("".into(), "Ramesh".into()));
    }

    #[test]
    fn has_guardian_data_detects_any_field() {
        assert!(has_guardian_data(Some("9876543210"), None));
        assert!(has_guardian_data(None, Some("Ramesh")));
        assert!(!has_guardian_data(None, None));
        assert!(!has_guardian_data(Some(""), Some("")));
    }
}
