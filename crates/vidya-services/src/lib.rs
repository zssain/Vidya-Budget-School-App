//! Application services: authorize, validate with `vidya-core`, run repositories
//! inside one transaction, and write the change log.
//!
//! Contains no Tauri types and no HTTP types. Services take an `Actor`, never
//! raw ids from callers (see `crates/AGENTS.md`).

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {}
}
