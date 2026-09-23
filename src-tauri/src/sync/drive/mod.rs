//! Google Drive fallback (prompts/P06).
//!
//! When the school server is unreachable but the internet works, devices exchange
//! encrypted op **bundles** through the school's Google Drive (§8.3 route 3, §11).
//!
//! **Gate:** the real Drive REST v3 + OAuth transport is deliberately NOT built in
//! this module yet — Step 0's two spikes (does `drive.file` let staff read each
//! other's files inside a shared folder; which Android sign-in works) must clear
//! first, and standing rule 4 forbids asserting Google behaviour without real test
//! accounts. See `docs/phase-notes/phase-6-spikes.md`.
//!
//! What IS built here is everything that is provable offline and independent of
//! that spike outcome:
//! * [`DriveApi`] — the minimal Drive surface the sync engine needs, so the real
//!   `drive.file` client and the test [`fake::FakeDrive`] are interchangeable.
//! * [`bundle`] — the `.vop` seal/verify + chunking format (§11).
//! * [`keys`] — the server-side versioned audience-key store + rotation (§8.8).
//! * [`fake::FakeDrive`] — a local, permission-enforcing, fault-injecting Drive
//!   for the Step 9 harness (a "local folder implementing the same trait").

pub mod bundle;
pub mod fake;
pub mod keys;

use std::collections::BTreeMap;

/// One file or folder as Drive reports it. `checksum` is Drive's content hash
/// (the real client uses the `md5Checksum` field Drive returns; the fake uses a
/// content SHA-256) — used to verify an upload by reading the metadata back
/// (Step 4). `properties` carries app metadata (audience + key version) so a
/// reader can tell which bundles are addressed to an audience it holds *without*
/// downloading them (Step 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
    pub is_folder: bool,
    pub size: usize,
    pub checksum: String,
    pub properties: BTreeMap<String, String>,
}

/// A Drive failure, mapped to the Step 8 "Needs attention" cases. Each maps to a
/// real Drive v3 error the live client will translate (HTTP status / reason).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DriveError {
    /// 403 `storageQuotaExceeded` — the Principal's Drive is full.
    #[error("drive quota full")]
    QuotaFull,
    /// 429 or 403 `rateLimitExceeded`/`userRateLimitExceeded` — back off + jitter.
    #[error("drive rate limited")]
    RateLimited,
    /// 401 — the OAuth token was revoked or expired ("Reconnect Google Drive").
    #[error("drive token revoked")]
    TokenRevoked,
    /// 404 — the file or folder is gone (deleted/unshared out from under us).
    #[error("drive item not found")]
    NotFound,
    /// 403 `insufficientFilePermissions` — the actor may not touch this item.
    #[error("drive permission denied")]
    PermissionDenied,
    /// A name/rename collision (a file with that name already exists).
    #[error("drive name conflict")]
    NameConflict,
    /// Any transport/other failure.
    #[error("drive io: {0}")]
    Io(String),
}

pub type DriveResult<T> = Result<T, DriveError>;

/// The minimal Google Drive surface the sync engine needs. Each staff device
/// drives Drive with its **own** OAuth token, so an implementation is bound to a
/// single acting identity and Drive enforces the permissions the Principal
/// granted (reader on `exchange/`, writer only on the device's own
/// `ops-<device_id>/`). The trait therefore has no "act as" parameter — the
/// implementation is the identity.
pub trait DriveApi {
    /// Direct children (files and folders) of `folder_id`. Requires read access.
    fn list(&self, folder_id: &str) -> DriveResult<Vec<DriveFile>>;

    /// Metadata for one item (verify-by-readback). Requires read access.
    fn metadata(&self, file_id: &str) -> DriveResult<DriveFile>;

    /// Full bytes of a file. Requires read access.
    fn download(&self, file_id: &str) -> DriveResult<Vec<u8>>;

    /// Create a file `name` with `bytes` and `properties` inside `parent`.
    /// Requires write access to `parent`. (Uploads use a temp name then
    /// [`DriveApi::rename`] so a half-written file is never seen — Step 4.)
    fn create(
        &self,
        parent: &str,
        name: &str,
        bytes: &[u8],
        properties: &BTreeMap<String, String>,
    ) -> DriveResult<DriveFile>;

    /// Rename a file (commit a temp upload to its final name). Requires write
    /// access to the file's parent.
    fn rename(&self, file_id: &str, new_name: &str) -> DriveResult<DriveFile>;

    /// Move a file to `new_parent` (server archives processed bundles to
    /// `_done/` — Step 6). Requires write access to both parents.
    fn move_to(&self, file_id: &str, new_parent: &str) -> DriveResult<DriveFile>;

    /// Delete a file. Requires write access to its parent — so a staff device can
    /// never delete `backups/` or another device's bundles (§11).
    fn delete(&self, file_id: &str) -> DriveResult<()>;

    /// Return the id of the subfolder `name` under `parent`, creating it if it
    /// does not exist. Requires write access to `parent`.
    fn ensure_folder(&self, parent: &str, name: &str) -> DriveResult<String>;
}
