use crate::error::DomainError;

fn fail(key: &'static str, field: &'static str) -> DomainError {
    DomainError::validation(key).field(field)
}

fn collapsed(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// `char::is_alphabetic` excludes combining vowel and diacritic marks that are
// part of written names such as "सुनीता".
fn is_script_letter_or_mark(ch: char) -> bool {
    ch.is_alphabetic()
        || (!ch.is_numeric()
            && matches!(
                ch,
                '\u{0300}'..='\u{036f}'
                    | '\u{0591}'..='\u{08ff}'
                    | '\u{0900}'..='\u{0fff}'
                    | '\u{1ab0}'..='\u{1aff}'
                    | '\u{1dc0}'..='\u{1dff}'
                    | '\u{20d0}'..='\u{20ff}'
                    | '\u{fe20}'..='\u{fe2f}'
            ))
}

/// Trims and validates a person's name in any Unicode script.
pub fn validate_person_name(s: &str, field: &'static str) -> Result<String, DomainError> {
    let value = collapsed(s);
    let count = value.chars().count();
    let has_letter = value.chars().any(char::is_alphabetic);
    let valid_chars = value
        .chars()
        .all(|ch| is_script_letter_or_mark(ch) || matches!(ch, ' ' | '.' | '\'' | '-'));
    if (2..=80).contains(&count) && has_letter && valid_chars {
        Ok(value)
    } else {
        Err(fail("validation.name", field))
    }
}

/// Validates an Indian ten-digit mobile number beginning with 6, 7, 8 or 9.
pub fn validate_mobile(s: &str) -> Result<String, DomainError> {
    let valid = s.len() == 10
        && s.bytes().all(|byte| byte.is_ascii_digit())
        && matches!(s.as_bytes().first(), Some(b'6'..=b'9'));
    valid
        .then(|| s.to_owned())
        .ok_or_else(|| fail("validation.mobile", "mobile"))
}

/// Validates an optional 11-digit UDISE code.
pub fn validate_udise(s: &str) -> Result<String, DomainError> {
    let value = s.trim();
    if value.is_empty() || (value.len() == 11 && value.bytes().all(|b| b.is_ascii_digit())) {
        Ok(value.to_owned())
    } else {
        Err(fail("validation.udise", "udise"))
    }
}

/// Validates a lowercase school code.
pub fn validate_school_code(s: &str) -> Result<String, DomainError> {
    validate_lower_alphanumeric(s, 3, 12, "validation.school_code", "school_code")
}

/// Validates a lowercase username.
pub fn validate_username(s: &str) -> Result<String, DomainError> {
    validate_lower_alphanumeric(s, 2, 30, "validation.username", "username")
}

fn validate_lower_alphanumeric(
    s: &str,
    min: usize,
    max: usize,
    key: &'static str,
    field: &'static str,
) -> Result<String, DomainError> {
    let valid =
        (min..=max).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    valid.then(|| s.to_owned()).ok_or_else(|| fail(key, field))
}

/// Validates a replacement password against the shared password rules.
pub fn validate_new_password(
    password: &str,
    username: &str,
    same_as_current: bool,
) -> Result<String, DomainError> {
    if password.chars().count() < 8 {
        return Err(fail("password.too_short", "password"));
    }
    if !username.is_empty() && password.to_lowercase().contains(&username.to_lowercase()) {
        return Err(fail("password.contains_username", "password"));
    }
    if same_as_current {
        return Err(fail("password.same_as_current", "password"));
    }
    Ok(password.to_owned())
}

/// Validates a two-to-four-character receipt prefix.
pub fn validate_receipt_prefix(s: &str) -> Result<String, DomainError> {
    let valid = (2..=4).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
    valid
        .then(|| s.to_owned())
        .ok_or_else(|| fail("validation.receipt_prefix", "receipt_prefix"))
}

/// Trims, collapses and validates a class name.
pub fn validate_class_name(s: &str) -> Result<String, DomainError> {
    let value = collapsed(s);
    let count = value.chars().count();
    let valid = (1..=12).contains(&count)
        && value
            .chars()
            .all(|ch| is_script_letter_or_mark(ch) || ch.is_ascii_digit() || ch == ' ');
    valid
        .then_some(value)
        .ok_or_else(|| fail("validation.class_name", "class_name"))
}

/// Trims a reason and requires at least `min` Unicode characters.
pub fn validate_reason(s: &str, min: usize) -> Result<String, DomainError> {
    let value = s.trim().to_owned();
    (value.chars().count() >= min)
        .then_some(value)
        .ok_or_else(|| fail("validation.reason", "reason"))
}

/// Validates an exact 12-digit UPI reference.
pub fn validate_upi_reference(s: &str) -> Result<String, DomainError> {
    let valid = s.len() == 12 && s.bytes().all(|b| b.is_ascii_digit());
    valid
        .then(|| s.to_owned())
        .ok_or_else(|| fail("fees.error.upi_reference", "reference"))
}

/// Trims and validates a cheque reference.
pub fn validate_cheque_reference(s: &str) -> Result<String, DomainError> {
    let value = s.trim().to_owned();
    (value.chars().count() >= 4)
        .then_some(value)
        .ok_or_else(|| fail("fees.error.cheque_reference", "reference"))
}
