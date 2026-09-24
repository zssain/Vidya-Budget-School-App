//! # vidya-core
//!
//! Pure-Rust business rules for Vidya — no IO, no async, no database, no clock
//! reads (time is passed in), no randomness (ids passed in). See
//! `docs/00-SYSTEM-CONTEXT.md` §9 and `prompts/P02`.

pub mod errors;
pub mod money;
pub mod types;

pub mod admissions;
pub mod attendance;
pub mod audience;
pub mod backup;
pub mod conflicts;
pub mod csv;
pub mod epoch;
pub mod fees;
pub mod grades;
pub mod hlc;
pub mod lease;
pub mod licence;
pub mod marks;
pub mod modules;
pub mod permissions;
pub mod receipts;
pub mod requests;
pub mod restore;
pub mod session;
pub mod validation;
pub mod words;

pub use errors::{CoreError, CoreResult};
pub use money::Paise;
