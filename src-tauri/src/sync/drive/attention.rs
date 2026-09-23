//! Drive failure → "Needs attention" mapping (prompts/P06 Step 8).
//!
//! Every Drive failure becomes a specific Home "Needs attention" item with the
//! exact fix, and pending work **stays safely queued** (the outbox is never
//! cleared on a failure — push returns `Err` and the rows keep their state). This
//! module is the single place that maps a [`DriveError`] to its user-facing item;
//! the React layer renders `title_key`/`fix_key` (Hindi mirrors land with P8 i18n,
//! like every other screen). Rate limits back off with jitter and are transient,
//! so they surface only as a soft "retrying" note, not a hard action.

use super::DriveError;

/// One "Needs attention" item derived from a Drive failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeedsAttentionItem {
    /// Stable machine code.
    pub code: &'static str,
    /// i18n key for the Home item title.
    pub title_key: &'static str,
    /// i18n key for the exact fix instruction.
    pub fix_key: &'static str,
    /// English default (until P8 fills the i18n tables) — title.
    pub en_title: &'static str,
    /// English default — the exact fix.
    pub en_fix: &'static str,
    /// True when the user must act; false for transient/auto-retrying cases.
    pub needs_user_action: bool,
    /// Pending work stays safely queued in EVERY case (§Step 8) — always true.
    pub requeue_safe: bool,
}

/// Map a Drive failure to its "Needs attention" item.
pub fn needs_attention_for(err: &DriveError) -> NeedsAttentionItem {
    let (code, title_key, fix_key, en_title, en_fix, needs_user_action) = match err {
        DriveError::QuotaFull => (
            "DRIVE_QUOTA_FULL",
            "drive.attention.quota_full.title",
            "drive.attention.quota_full.fix",
            "School Google Drive is full",
            "Free up space in the school's Google Drive, then changes will sync automatically.",
            true,
        ),
        DriveError::TokenRevoked => (
            "DRIVE_RECONNECT",
            "drive.attention.reconnect.title",
            "drive.attention.reconnect.fix",
            "Reconnect Google Drive",
            "Sign in to Google again in Settings → Google Drive to keep sharing changes.",
            true,
        ),
        DriveError::NotFound => (
            "DRIVE_FOLDER_MISSING",
            "drive.attention.folder_missing.title",
            "drive.attention.folder_missing.fix",
            "School Drive folder is missing",
            "The Vidya folder was deleted or unshared. Ask the Principal to reconnect Google Drive.",
            true,
        ),
        DriveError::PermissionDenied => (
            "DRIVE_PERMISSION",
            "drive.attention.permission.title",
            "drive.attention.permission.fix",
            "No access to the school Drive folder",
            "Ask the Principal to share your Drive folder again from Settings → Google Drive.",
            true,
        ),
        DriveError::RateLimited => (
            "DRIVE_RATE_LIMITED",
            "drive.attention.rate_limited.title",
            "drive.attention.rate_limited.fix",
            "Google Drive is busy",
            "Retrying automatically. Nothing is lost.",
            false,
        ),
        DriveError::NameConflict => (
            "DRIVE_ALREADY_UPLOADED",
            "drive.attention.already_uploaded.title",
            "drive.attention.already_uploaded.fix",
            "Already shared",
            "This change was already shared to Drive. Nothing to do.",
            false,
        ),
        DriveError::Io(_) => (
            "DRIVE_UNREACHABLE",
            "drive.attention.unreachable.title",
            "drive.attention.unreachable.fix",
            "Can't reach Google Drive",
            "Retrying when the internet is back. Your changes are saved on this device.",
            false,
        ),
    };
    NeedsAttentionItem { code, title_key, fix_key, en_title, en_fix, needs_user_action, requeue_safe: true }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_maps_to_a_distinct_safe_item() {
        let errors = [
            DriveError::QuotaFull,
            DriveError::TokenRevoked,
            DriveError::NotFound,
            DriveError::PermissionDenied,
            DriveError::RateLimited,
            DriveError::NameConflict,
            DriveError::Io("net".into()),
        ];
        let mut codes = std::collections::HashSet::new();
        for e in &errors {
            let item = needs_attention_for(e);
            assert!(item.requeue_safe, "pending work must always stay queued: {e:?}");
            assert!(!item.en_title.is_empty() && !item.en_fix.is_empty());
            assert!(codes.insert(item.code), "duplicate code {}", item.code);
        }
        assert_eq!(codes.len(), errors.len());
    }

    #[test]
    fn revoked_token_says_reconnect_google_drive() {
        // §11 / Step 1 exact copy.
        let item = needs_attention_for(&DriveError::TokenRevoked);
        assert_eq!(item.en_title, "Reconnect Google Drive");
        assert!(item.needs_user_action);
    }

    #[test]
    fn rate_limit_is_transient_not_a_user_action() {
        let item = needs_attention_for(&DriveError::RateLimited);
        assert!(!item.needs_user_action);
        assert!(item.requeue_safe);
    }
}
