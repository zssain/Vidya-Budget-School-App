//! Shared, cheaply-cloneable application state handed to every axum handler.

use std::sync::{Arc, Mutex};

use ed25519_dalek::SigningKey;
use rusqlite::Connection;

use crate::config::Config;
use crate::ratelimit::RateLimiter;

#[derive(Clone)]
pub struct AppState {
    /// One serialized connection. DB work is quick + synchronous; handlers lock,
    /// do the work, drop the guard, then build the response (never hold across
    /// `.await`). Adequate for a single licence instance.
    pub db: Arc<Mutex<Connection>>,
    pub signing: Arc<SigningKey>,
    pub cfg: Arc<Config>,
    pub rl: Arc<Mutex<RateLimiter>>,
}

impl AppState {
    pub fn new(db: Connection, signing: SigningKey, cfg: Config) -> Self {
        AppState {
            db: Arc::new(Mutex::new(db)),
            signing: Arc::new(signing),
            cfg: Arc::new(cfg),
            rl: Arc::new(Mutex::new(RateLimiter::new())),
        }
    }
}
