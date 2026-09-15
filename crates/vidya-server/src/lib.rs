//! axum LAN server, TLS, discovery responder, rate limits and request-signing
//! checks.
//!
//! Contains no business rules; it calls into `vidya-services` (see
//! `crates/AGENTS.md`).

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {}
}
