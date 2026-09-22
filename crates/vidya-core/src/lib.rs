//! # vidya-core
//!
//! Pure-Rust business rules for Vidya — no IO, no async, no database
//! (see `docs/00-SYSTEM-CONTEXT.md` §9). The UI and sync layers never decide
//! business rules; they call into this crate and the school server re-checks
//! every change here.
//!
//! Phase 1 is a placeholder (fresh-start path): the real modules (money, fees,
//! grades, permissions, attendance, marks, requests, conflicts, hlc, …) are
//! built in Phase 2.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(2 + 2, 4);
    }
}
