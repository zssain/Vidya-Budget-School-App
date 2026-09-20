//! Application services: the single path for every action. A service takes an
//! `Actor`, authorizes it with `vidya-core`, validates, runs repositories inside
//! one transaction, and writes the change log. No Tauri or HTTP types live here
//! (see `crates/AGENTS.md`).

use std::sync::{Arc, Mutex};

use vidya_core::hlc::{Hlc, HlcClock};
use vidya_db::{repo, Db};

pub mod auth;
pub mod change_log;
pub mod env;
pub mod error;
pub mod services;

pub use error::{ErrorDto, ServiceError};

use env::{Clock, IdGen, Random};

/// Whether this process is the office computer (server) or a phone (client).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Server,
    Client,
}

/// The container every service borrows: database, injected environment, mode,
/// this device's id, and the hybrid logical clock.
pub struct Services {
    pub db: Arc<Db>,
    pub clock: Arc<dyn Clock>,
    pub ids: Arc<dyn IdGen>,
    pub random: Arc<dyn Random>,
    pub mode: Mode,
    pub device_id: String,
    hlc: Mutex<HlcClock>,
}

impl Services {
    /// Builds the container, loading the last hybrid logical clock from `meta`.
    pub fn new(
        db: Arc<Db>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGen>,
        random: Arc<dyn Random>,
        mode: Mode,
        device_id: String,
    ) -> Result<Self, ServiceError> {
        let last_text = db.read(|conn| repo::meta::get(conn, "hlc_last"))?;
        let last = match last_text {
            Some(text) => Some(Hlc::parse(&text)?),
            None => None,
        };
        let hlc = HlcClock::new(device_id.clone(), last);
        Ok(Self {
            db,
            clock,
            ids,
            random,
            mode,
            device_id,
            hlc: Mutex::new(hlc),
        })
    }

    /// Advances the hybrid logical clock using the injected wall clock and
    /// returns the new timestamp. Every change-log entry and syncable write
    /// stamps a fresh value from here.
    pub fn next_hlc(&self) -> Hlc {
        let physical_ms = self.clock.now_ms();
        self.hlc.lock().expect("hlc mutex poisoned").tick(physical_ms)
    }

    pub fn school(&self) -> services::school::SchoolService<'_> {
        services::school::SchoolService { services: self }
    }

    pub fn settings(&self) -> services::settings::SettingsService<'_> {
        services::settings::SettingsService { services: self }
    }
}
