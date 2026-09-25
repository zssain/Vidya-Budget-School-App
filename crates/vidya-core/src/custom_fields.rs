//! custom_fields — custom-field rules (P13, foundation §8.9). Pure: no IO, no clock.
//!
//! Schools add their own fields to students and staff (text / number / date /
//! choice), with a label in en/hi/te. This module validates a field definition
//! and a value against its type. No floats are used (§ no_floats) — number values
//! are validated by shape, never parsed as a floating-point number.

use crate::errors::{CoreError, CoreResult};

/// Which entity a custom field is attached to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entity {
    Student,
    Staff,
}

impl Entity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Entity::Student => "student",
            Entity::Staff => "staff",
        }
    }
    pub fn parse(s: &str) -> Option<Entity> {
        match s {
            "student" => Some(Entity::Student),
            "staff" => Some(Entity::Staff),
            _ => None,
        }
    }
}

/// A custom field's value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Text,
    Number,
    Date,
    Choice,
}

impl FieldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FieldType::Text => "text",
            FieldType::Number => "number",
            FieldType::Date => "date",
            FieldType::Choice => "choice",
        }
    }
    pub fn parse(s: &str) -> Option<FieldType> {
        Some(match s {
            "text" => FieldType::Text,
            "number" => FieldType::Number,
            "date" => FieldType::Date,
            "choice" => FieldType::Choice,
            _ => return None,
        })
    }
}

/// A field `key` is a stable identifier used as the CSV header: 1–40 chars,
/// lowercase ASCII letters/digits/underscore, starting with a letter.
pub fn validate_key(key: &str) -> CoreResult<()> {
    let k = key.trim();
    if k.is_empty() || k.len() > 40 {
        return Err(CoreError::validation("key", "length"));
    }
    let mut chars = k.bytes();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() {
        return Err(CoreError::validation("key", "start_letter"));
    }
    if !k.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') {
        return Err(CoreError::validation("key", "charset"));
    }
    Ok(())
}

/// Validate a field definition: the key, the label, and — for `choice` — a
/// non-empty option list.
pub fn validate_field_def(key: &str, label: &str, field_type: FieldType, options: &[String]) -> CoreResult<()> {
    validate_key(key)?;
    if label.trim().is_empty() {
        return Err(CoreError::validation("label", "required"));
    }
    if field_type == FieldType::Choice && options.iter().filter(|o| !o.trim().is_empty()).count() == 0 {
        return Err(CoreError::validation("options", "required_for_choice"));
    }
    Ok(())
}

/// True if `s` is a plain decimal number (optional leading `-`, digits, optional
/// single `.` with digits on both sides). No float parse (no floats).
fn is_number(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    if s.is_empty() {
        return false;
    }
    match s.split_once('.') {
        Some((int, frac)) => !int.is_empty() && !frac.is_empty() && int.bytes().all(|b| b.is_ascii_digit()) && frac.bytes().all(|b| b.is_ascii_digit()),
        None => s.bytes().all(|b| b.is_ascii_digit()),
    }
}

/// Validate a value for a field. An empty value fails only when the field is
/// `required`; otherwise it is accepted (the field is simply unset).
pub fn validate_value(field_type: FieldType, value: &str, options: &[String], required: bool) -> CoreResult<()> {
    let v = value.trim();
    if v.is_empty() {
        return if required {
            Err(CoreError::validation("value", "required"))
        } else {
            Ok(())
        };
    }
    match field_type {
        FieldType::Text => Ok(()),
        FieldType::Number => {
            if is_number(v) {
                Ok(())
            } else {
                Err(CoreError::validation("value", "number"))
            }
        }
        FieldType::Date => {
            if crate::calendar::parse_date(v).is_some() {
                Ok(())
            } else {
                Err(CoreError::validation("value", "date"))
            }
        }
        FieldType::Choice => {
            if options.iter().any(|o| o == v) {
                Ok(())
            } else {
                Err(CoreError::validation("value", "not_a_choice"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn entity_and_type_round_trip() {
        assert_eq!(Entity::parse(Entity::Student.as_str()), Some(Entity::Student));
        assert_eq!(Entity::parse(Entity::Staff.as_str()), Some(Entity::Staff));
        for t in [FieldType::Text, FieldType::Number, FieldType::Date, FieldType::Choice] {
            assert_eq!(FieldType::parse(t.as_str()), Some(t));
        }
        assert_eq!(FieldType::parse("colour"), None);
    }

    #[test]
    fn key_rules() {
        assert!(validate_key("blood_group").is_ok());
        assert!(validate_key("route2").is_ok());
        assert_eq!(validate_key("2route").unwrap_err().code(), "VALIDATION"); // must start with a letter
        assert_eq!(validate_key("Blood").unwrap_err().code(), "VALIDATION"); // uppercase
        assert_eq!(validate_key("has space").unwrap_err().code(), "VALIDATION");
        assert_eq!(validate_key("").unwrap_err().code(), "VALIDATION");
    }

    #[test]
    fn choice_needs_options() {
        assert!(validate_field_def("bg", "Blood group", FieldType::Choice, &opts(&["A+", "B+"])).is_ok());
        assert_eq!(validate_field_def("bg", "Blood group", FieldType::Choice, &[]).unwrap_err().code(), "VALIDATION");
        // Non-choice types don't need options.
        assert!(validate_field_def("note", "Note", FieldType::Text, &[]).is_ok());
        // Empty label rejected.
        assert_eq!(validate_field_def("note", "  ", FieldType::Text, &[]).unwrap_err().code(), "VALIDATION");
    }

    #[test]
    fn value_by_type() {
        // text
        assert!(validate_value(FieldType::Text, "anything", &[], false).is_ok());
        // number (no floats parse — shape only)
        assert!(validate_value(FieldType::Number, "42", &[], true).is_ok());
        assert!(validate_value(FieldType::Number, "-3.5", &[], true).is_ok());
        assert_eq!(validate_value(FieldType::Number, "3.", &[], true).unwrap_err().code(), "VALIDATION");
        assert_eq!(validate_value(FieldType::Number, "1,000", &[], true).unwrap_err().code(), "VALIDATION");
        assert_eq!(validate_value(FieldType::Number, "abc", &[], true).unwrap_err().code(), "VALIDATION");
        // date
        assert!(validate_value(FieldType::Date, "2026-09-25", &[], true).is_ok());
        assert_eq!(validate_value(FieldType::Date, "25-09-2026", &[], true).unwrap_err().code(), "VALIDATION");
        // choice
        assert!(validate_value(FieldType::Choice, "A+", &opts(&["A+", "B+"]), true).is_ok());
        assert_eq!(validate_value(FieldType::Choice, "O-", &opts(&["A+", "B+"]), true).unwrap_err().code(), "VALIDATION");
    }

    #[test]
    fn required_vs_optional_empty() {
        assert_eq!(validate_value(FieldType::Text, "", &[], true).unwrap_err().code(), "VALIDATION");
        assert!(validate_value(FieldType::Text, "", &[], false).is_ok());
        assert!(validate_value(FieldType::Number, "   ", &[], false).is_ok());
    }
}
