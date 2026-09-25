//! Vidya licence service — library crate.
//!
//! The company-side service (website + manual UPI purchase flow, production
//! licence API, admin panel). Split into a lib so the integration tests in
//! `tests/` can drive the real router and domain logic. A SEPARATE workspace —
//! never a member of the app workspace, so none of this ships in the apps.

pub mod admin;
pub mod api;
pub mod audit;
pub mod config;
pub mod crypto;
pub mod db;
pub mod domain;
pub mod error;
pub mod keys;
pub mod ratelimit;
pub mod releases;
pub mod state;
pub mod ui;
pub mod web;

use axum::Router;
use state::AppState;

/// Assemble the full application router (website + API + admin) over the state.
pub fn app_router(state: AppState) -> Router {
    Router::new()
        .merge(web::router())
        .merge(api::router())
        .merge(admin::router())
        .with_state(state)
}
