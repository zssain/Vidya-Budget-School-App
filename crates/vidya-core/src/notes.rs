//! notes — homework & class-notes rules (P16 §10.4, Step 3).
//!
//! Pure Rust. A note is `homework` or `notes`, has study-material text and/or
//! attachments (photos compressed on the device, PDFs). Size limits: **≤ 10 MB
//! each, ≤ 20 MB total** (§10.4). The warning "Study material only — no student
//! photos or marks" is UI copy; the rule enforced here is only the size cap and
//! the kind. Deletion of one's own note is time-limited to 24 h (checked with the
//! elapsed milliseconds passed in — this stays clockless).

use serde::{Deserialize, Serialize};

use crate::errors::{CoreError, CoreResult};

/// ≤ 10 MB per attachment (§10.4).
pub const MAX_ATTACHMENT_BYTES: i64 = 10_000_000;
/// ≤ 20 MB total across a note's attachments (§10.4).
pub const MAX_TOTAL_BYTES: i64 = 20_000_000;
/// A teacher may delete only their own note within 24 h (§10.4).
pub const DELETE_WINDOW_MS: i64 = 24 * 60 * 60 * 1000;

/// Homework or class notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteKind {
    Homework,
    Notes,
}

impl NoteKind {
    pub fn as_key(self) -> &'static str {
        match self {
            NoteKind::Homework => "homework",
            NoteKind::Notes => "notes",
        }
    }
    pub fn parse(s: &str) -> Option<NoteKind> {
        match s {
            "homework" => Some(NoteKind::Homework),
            "notes" => Some(NoteKind::Notes),
            _ => None,
        }
    }
}

/// Validate the note kind string.
pub fn validate_kind(kind: &str) -> CoreResult<NoteKind> {
    NoteKind::parse(kind).ok_or_else(|| CoreError::validation("kind", "invalid"))
}

/// Validate the attachment sizes (bytes): each ≤ 10 MB, sum ≤ 20 MB. A negative
/// size is invalid.
pub fn validate_attachments(sizes: &[i64]) -> CoreResult<()> {
    let mut total: i64 = 0;
    for &s in sizes {
        if s < 0 {
            return Err(CoreError::validation("attachment", "size"));
        }
        if s > MAX_ATTACHMENT_BYTES {
            return Err(CoreError::validation("attachment", "too_large"));
        }
        total += s;
    }
    if total > MAX_TOTAL_BYTES {
        return Err(CoreError::validation("attachment", "total_too_large"));
    }
    Ok(())
}

/// A note must carry either some text or at least one attachment.
pub fn validate_note(text: &str, attachment_count: usize) -> CoreResult<()> {
    if text.trim().is_empty() && attachment_count == 0 {
        return Err(CoreError::validation("note", "empty"));
    }
    Ok(())
}

/// May the author delete this note now? Only if they created it and it is within
/// the 24 h window. `elapsed_ms` = now − created (computed by the caller).
pub fn can_delete_own(is_author: bool, elapsed_ms: i64) -> bool {
    is_author && (0..=DELETE_WINDOW_MS).contains(&elapsed_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_round_trips() {
        assert_eq!(NoteKind::parse("homework"), Some(NoteKind::Homework));
        assert_eq!(NoteKind::parse("notes"), Some(NoteKind::Notes));
        assert_eq!(NoteKind::parse("x"), None);
        assert_eq!(NoteKind::Homework.as_key(), "homework");
    }

    #[test]
    fn attachment_size_limits() {
        assert!(validate_attachments(&[]).is_ok());
        assert!(validate_attachments(&[9_000_000, 9_000_000]).is_ok()); // 18 MB total
        // One file over 10 MB is rejected.
        assert_eq!(
            validate_attachments(&[10_000_001]).unwrap_err().code(),
            "VALIDATION"
        );
        // Total over 20 MB is rejected (three 8 MB files = 24 MB).
        assert!(validate_attachments(&[8_000_000, 8_000_000, 8_000_000]).is_err());
        assert!(validate_attachments(&[-1]).is_err());
    }

    #[test]
    fn a_note_needs_text_or_a_file() {
        assert!(validate_note("Do exercise 5.2", 0).is_ok());
        assert!(validate_note("", 1).is_ok());
        assert!(validate_note("   ", 0).is_err());
    }

    #[test]
    fn delete_window_is_24h_and_author_only() {
        assert!(can_delete_own(true, 0));
        assert!(can_delete_own(true, DELETE_WINDOW_MS));
        assert!(!can_delete_own(true, DELETE_WINDOW_MS + 1));
        assert!(!can_delete_own(false, 0));
    }
}
