//! attachments — the encrypted local blob store for bill photos (P15 Step 2,
//! §10.3). Content-addressed by SHA-256 of the *plaintext*; each blob is written
//! as `nonce(12) ‖ ChaCha20-Poly1305(key, plaintext)` (reusing `sync::seal`) into
//! `<AppData>/Vidya/attachments/<sha256>`. The key is derived from the DB key so
//! blobs are unreadable without it. Rows (e.g. `expense.bill_attachment`) hold the
//! hash; a missing blob → the row shows "photo not yet received".
//!
//! Blobs travel to the school PC as separate sealed blobs (finance audience) over
//! LAN (`POST /v1/attachments`, dedup by hash) or Drive
//! (`exchange/ops-<device>/blobs/<hash>.vbl`) and are included in backups — that
//! transport is specced in the phase-15 handoff (needs live devices to verify).

use std::fs;
use std::path::{Path, PathBuf};

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::sync::seal::{open, seal};

/// Max size of a stored (already-compressed) attachment: 2 MB (§10.3).
pub const MAX_ATTACHMENT_BYTES: usize = 2_000_000;

#[derive(Debug)]
pub enum AttachmentError {
    TooLarge,
    NotFound,
    Tamper,
    Io(std::io::Error),
}

impl std::fmt::Display for AttachmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttachmentError::TooLarge => write!(f, "attachment too large"),
            AttachmentError::NotFound => write!(f, "attachment not found"),
            AttachmentError::Tamper => write!(f, "attachment failed integrity check"),
            AttachmentError::Io(e) => write!(f, "attachment io: {e}"),
        }
    }
}
impl std::error::Error for AttachmentError {}
impl From<std::io::Error> for AttachmentError {
    fn from(e: std::io::Error) -> Self {
        AttachmentError::Io(e)
    }
}

/// The SHA-256 hex of `bytes` (the content address).
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let digest = h.finalize();
    let mut s = String::with_capacity(64);
    for b in digest {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Derive the attachment key from the DB key hex (HMAC-SHA256 with a fixed label,
/// so the attachment key never equals the DB key).
fn derive_key(db_key_hex: &str) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(db_key_hex.as_bytes()).expect("hmac accepts any key length");
    mac.update(b"vidya-attachments-v1");
    let out = mac.finalize().into_bytes();
    let mut key = [0u8; 32];
    key.copy_from_slice(&out);
    key
}

/// A content-addressed, encrypted-at-rest blob store under `<data_dir>/attachments`.
pub struct AttachmentStore {
    dir: PathBuf,
    key: [u8; 32],
}

impl AttachmentStore {
    pub fn new(data_dir: &Path, db_key_hex: &str) -> Self {
        AttachmentStore { dir: data_dir.join("attachments"), key: derive_key(db_key_hex) }
    }

    fn path_for(&self, hash: &str) -> PathBuf {
        self.dir.join(hash)
    }

    /// Store `bytes`, returning the SHA-256 hex. Deduplicates by hash (a blob that
    /// already exists is not rewritten). Rejects blobs over [`MAX_ATTACHMENT_BYTES`].
    pub fn put(&self, bytes: &[u8]) -> Result<String, AttachmentError> {
        if bytes.len() > MAX_ATTACHMENT_BYTES {
            return Err(AttachmentError::TooLarge);
        }
        let hash = sha256_hex(bytes);
        let path = self.path_for(&hash);
        if path.exists() {
            return Ok(hash); // dedup
        }
        fs::create_dir_all(&self.dir)?;
        let sealed = seal(&self.key, hash.as_bytes(), bytes);
        // Write to a temp name then rename (atomic, never a half-written blob).
        let tmp = self.dir.join(format!("{hash}.tmp"));
        fs::write(&tmp, &sealed)?;
        fs::rename(&tmp, &path)?;
        Ok(hash)
    }

    /// True if the blob for `hash` is present locally.
    pub fn has(&self, hash: &str) -> bool {
        !hash.is_empty() && self.path_for(hash).exists()
    }

    /// Read + decrypt a blob, verifying the plaintext hash matches (tamper check).
    /// `Ok(None)` if the blob is not present yet (row shows "photo not yet received").
    pub fn get(&self, hash: &str) -> Result<Option<Vec<u8>>, AttachmentError> {
        let path = self.path_for(hash);
        if !path.exists() {
            return Ok(None);
        }
        let sealed = fs::read(&path)?;
        // Any open failure (short blob, wrong key, tampered bytes) → integrity error.
        let plain = open(&self.key, hash.as_bytes(), &sealed).map_err(|_| AttachmentError::Tamper)?;
        // Second line of defence: the content must hash to its address.
        if sha256_hex(&plain) != hash {
            return Err(AttachmentError::Tamper);
        }
        Ok(Some(plain))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!("vidya-att-{}", uuid::Uuid::now_v7()))
    }
    fn store() -> AttachmentStore {
        AttachmentStore::new(&temp_dir(), KEY)
    }

    #[test]
    fn put_get_roundtrip_and_dedup() {
        let s = store();
        let bytes = b"a fake compressed jpeg of a bill".to_vec();
        let h1 = s.put(&bytes).unwrap();
        assert_eq!(h1.len(), 64);
        assert!(s.has(&h1));
        assert_eq!(s.get(&h1).unwrap().unwrap(), bytes);
        // Dedup: same content → same hash, and only one file on disk.
        let h2 = s.put(&bytes).unwrap();
        assert_eq!(h1, h2);
        let count = std::fs::read_dir(s.dir).unwrap().filter(|e| e.as_ref().unwrap().path().extension().is_none()).count();
        assert_eq!(count, 1, "dedup writes one blob");
    }

    #[test]
    fn missing_blob_reads_as_none() {
        let s = store();
        assert!(!s.has("deadbeef"));
        assert!(s.get("deadbeef").unwrap().is_none(), "missing blob → None (photo not yet received)");
    }

    #[test]
    fn tampered_blob_is_rejected() {
        let s = store();
        let h = s.put(b"original bill bytes").unwrap();
        // Flip a byte in the encrypted file → AEAD auth fails.
        let path = s.path_for(&h);
        let mut raw = std::fs::read(&path).unwrap();
        let i = raw.len() - 1;
        raw[i] ^= 0xff;
        std::fs::write(&path, &raw).unwrap();
        assert!(matches!(s.get(&h), Err(AttachmentError::Tamper)));
    }

    #[test]
    fn oversize_is_rejected() {
        let s = store();
        let big = vec![0u8; MAX_ATTACHMENT_BYTES + 1];
        assert!(matches!(s.put(&big), Err(AttachmentError::TooLarge)));
    }

    #[test]
    fn a_different_db_key_cannot_read_the_blob() {
        let dir = temp_dir();
        let s1 = AttachmentStore::new(&dir, KEY);
        let h = s1.put(b"secret bill").unwrap();
        // A store with a different DB key derives a different attachment key.
        let other = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let s2 = AttachmentStore::new(&dir, other);
        assert!(matches!(s2.get(&h), Err(AttachmentError::Tamper)));
    }
}
