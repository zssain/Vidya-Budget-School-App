//! The injected environment: clock, id generator and randomness.
//!
//! Production uses the real system clock, UUID v7 ids and the OS RNG. Tests use
//! deterministic doubles so every service is reproducible.

use std::sync::Mutex;

use chrono::{DateTime, Local, NaiveDate, Utc};
use rand::rngs::{OsRng, StdRng};
use rand::{RngCore, SeedableRng, TryRngCore};

/// Wall-clock time. `today_local` uses the computer's local timezone.
pub trait Clock: Send + Sync {
    fn now_utc(&self) -> DateTime<Utc>;
    fn today_local(&self) -> NaiveDate;
    fn now_ms(&self) -> u64;
}

/// New row ids (UUID v7 text in production).
pub trait IdGen: Send + Sync {
    fn new_id(&self) -> String;
}

/// A source of random bytes.
pub trait Random: Send + Sync {
    fn fill(&self, buf: &mut [u8]);
}

// ---------- production ----------

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc(&self) -> DateTime<Utc> {
        Utc::now()
    }
    fn today_local(&self) -> NaiveDate {
        Local::now().date_naive()
    }
    fn now_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

pub struct UuidV7;

impl IdGen for UuidV7 {
    fn new_id(&self) -> String {
        uuid::Uuid::now_v7().to_string()
    }
}

pub struct OsRandom;

impl Random for OsRandom {
    fn fill(&self, buf: &mut [u8]) {
        // OS entropy failure is unrecoverable; the app cannot run without it.
        OsRng.try_fill_bytes(buf).expect("operating system RNG failed");
    }
}

// ---------- deterministic test doubles ----------

/// A clock frozen at fixed values; tests can rebuild it to advance time.
pub struct FixedClock {
    pub now: DateTime<Utc>,
    pub today: NaiveDate,
}

impl Clock for FixedClock {
    fn now_utc(&self) -> DateTime<Utc> {
        self.now
    }
    fn today_local(&self) -> NaiveDate {
        self.today
    }
    fn now_ms(&self) -> u64 {
        self.now.timestamp_millis().max(0) as u64
    }
}

/// Sequential ids: `<prefix>00000001`, `<prefix>00000002`, …
pub struct SeqIds {
    prefix: String,
    next: Mutex<u64>,
}

impl SeqIds {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            next: Mutex::new(1),
        }
    }
}

impl IdGen for SeqIds {
    fn new_id(&self) -> String {
        let mut n = self.next.lock().expect("SeqIds mutex poisoned");
        let id = format!("{}{:08}", self.prefix, *n);
        *n += 1;
        id
    }
}

/// A deterministic RNG seeded from a `u64`.
pub struct SeededRandom {
    rng: Mutex<StdRng>,
}

impl SeededRandom {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Mutex::new(StdRng::seed_from_u64(seed)),
        }
    }
}

impl Random for SeededRandom {
    fn fill(&self, buf: &mut [u8]) {
        self.rng
            .lock()
            .expect("SeededRandom mutex poisoned")
            .fill_bytes(buf);
    }
}
