//! In-memory session store. Only `SHA-256(token)` is kept, so a memory dump of
//! the map never reveals a usable token.

use std::collections::HashMap;
use std::sync::Mutex;

use sha2::{Digest, Sha256};
use vidya_core::roles::Origin;

type TokenHash = [u8; 32];

/// Ten minutes for a pending (first-password) session.
const PENDING_TTL_MS: u64 = 10 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    Full,
    Pending,
}

struct SessionEntry {
    user_id: String,
    kind: SessionKind,
    created_ms: u64,
    last_active_ms: u64,
    device_id: String,
    origin: Origin,
}

/// The resolved facts of a live session.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub user_id: String,
    pub kind: SessionKind,
    pub device_id: String,
    pub origin: Origin,
}

#[derive(Default)]
pub struct SessionStore {
    inner: Mutex<HashMap<TokenHash, SessionEntry>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn hash(token: &str) -> TokenHash {
        Sha256::digest(token.as_bytes()).into()
    }

    fn insert(
        &self,
        token: &str,
        user_id: String,
        kind: SessionKind,
        device_id: String,
        origin: Origin,
        now_ms: u64,
    ) {
        let entry = SessionEntry {
            user_id,
            kind,
            created_ms: now_ms,
            last_active_ms: now_ms,
            device_id,
            origin,
        };
        self.inner
            .lock()
            .expect("session mutex poisoned")
            .insert(Self::hash(token), entry);
    }

    pub fn create_full(&self, token: &str, user_id: String, device_id: String, origin: Origin, now_ms: u64) {
        self.insert(token, user_id, SessionKind::Full, device_id, origin, now_ms);
    }

    pub fn create_pending(
        &self,
        token: &str,
        user_id: String,
        device_id: String,
        origin: Origin,
        now_ms: u64,
    ) {
        self.insert(token, user_id, SessionKind::Pending, device_id, origin, now_ms);
    }

    fn is_expired(entry: &SessionEntry, now_ms: u64, timeout_ms: u64) -> bool {
        match entry.kind {
            SessionKind::Pending => now_ms.saturating_sub(entry.created_ms) > PENDING_TTL_MS,
            SessionKind::Full => now_ms.saturating_sub(entry.last_active_ms) > timeout_ms,
        }
    }

    /// Resolves a token, refreshing `last_active_ms` for full sessions. Returns
    /// `None` for a missing or expired session (which it removes).
    pub fn resolve(&self, token: &str, now_ms: u64, timeout_ms: u64) -> Option<Resolved> {
        let mut map = self.inner.lock().expect("session mutex poisoned");
        let key = Self::hash(token);
        let expired = map
            .get(&key)
            .is_some_and(|e| Self::is_expired(e, now_ms, timeout_ms));
        if expired {
            map.remove(&key);
            return None;
        }
        let entry = map.get_mut(&key)?;
        if entry.kind == SessionKind::Full {
            entry.last_active_ms = now_ms;
        }
        Some(Resolved {
            user_id: entry.user_id.clone(),
            kind: entry.kind,
            device_id: entry.device_id.clone(),
            origin: entry.origin,
        })
    }

    pub fn end(&self, token: &str) {
        self.inner
            .lock()
            .expect("session mutex poisoned")
            .remove(&Self::hash(token));
    }

    pub fn end_all_for_user(&self, user_id: &str) {
        self.inner
            .lock()
            .expect("session mutex poisoned")
            .retain(|_, e| e.user_id != user_id);
    }

    /// Ends every session of `user_id` except the one for `keep_token`.
    pub fn end_others_for_user(&self, user_id: &str, keep_token: &str) {
        let keep = Self::hash(keep_token);
        self.inner
            .lock()
            .expect("session mutex poisoned")
            .retain(|hash, e| e.user_id != user_id || *hash == keep);
    }

    /// Removes idle/expired sessions and returns the ids of users whose sessions ended.
    pub fn expire_idle(&self, now_ms: u64, timeout_ms: u64) -> Vec<String> {
        let mut map = self.inner.lock().expect("session mutex poisoned");
        let mut ended = Vec::new();
        map.retain(|_, e| {
            if Self::is_expired(e, now_ms, timeout_ms) {
                ended.push(e.user_id.clone());
                false
            } else {
                true
            }
        });
        ended
    }
}
