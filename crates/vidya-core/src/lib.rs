//! Domain types, money, dates, validation, fee and grade maths, usernames,
//! amount in words, permission matrix, and the hybrid logical clock (HLC).
//!
//! Pure business rules only: no I/O, SQL, async, Tauri or network. Time,
//! randomness and identifiers are always passed in by the caller.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {}
}
