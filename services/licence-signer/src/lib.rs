//! Vidya isolated licence signer (owner licensing portal, spec Phase 13).
//!
//! A queue worker with **no network listener**: it claims `issue_licence` jobs
//! from the `billing` Postgres schema, signs the v2 licence payload with the
//! owner's ed25519 key (loaded only from `LICENCE_SIGNING_KEY_FILE`), verifies the
//! result with the app's own `vidya_core::licence::verify_v2`, and writes the
//! artefact back. See `docs/implementation/licence-signer.md`.

pub mod artefact;
pub mod config;
pub mod key;
pub mod worker;

pub use artefact::{sign_licence, Artefact, LicenceInputs};
pub use config::Config;
pub use key::Signer;
pub use worker::{process_one, recover_expired_leases, Outcome};
