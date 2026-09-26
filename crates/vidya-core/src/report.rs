//! report — report-card remark rules (P16 §10.4, Step 4).
//!
//! Pure Rust. The class teacher enters one remark per student per exam; it is
//! locked once the Principal makes the exam's report cards final (the lock row is
//! checked by `src-tauri`). Here we only validate the remark text.

use crate::errors::{CoreError, CoreResult};

/// A remark fits on a printed report card.
pub const MAX_REMARK_CHARS: usize = 300;

/// Validate a report-card remark: non-empty (after trim) and ≤ 300 characters.
pub fn validate_remark(text: &str) -> CoreResult<()> {
    let t = text.trim();
    if t.is_empty() {
        return Err(CoreError::validation("remark", "empty"));
    }
    if t.chars().count() > MAX_REMARK_CHARS {
        return Err(CoreError::validation("remark", "too_long"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_normal_remark() {
        assert!(validate_remark("Kavya is attentive and asks good questions.").is_ok());
    }

    #[test]
    fn rejects_empty_and_overlong() {
        assert!(validate_remark("   ").is_err());
        let long: String = "क".repeat(MAX_REMARK_CHARS + 1);
        assert!(validate_remark(&long).is_err());
        // Exactly the limit is fine.
        assert!(validate_remark(&"a".repeat(MAX_REMARK_CHARS)).is_ok());
    }
}
