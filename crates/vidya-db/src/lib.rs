//! SQLCipher connection, migrations, repositories (one module per table group)
//! and the change-log writer.
//!
//! Contains no business rules and no permission checks (see `crates/AGENTS.md`).

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {}
}
