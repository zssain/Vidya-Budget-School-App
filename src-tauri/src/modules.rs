//! Module switches — DB access for the vidya-core module gate (§14).
//!
//! The pure mapping (`module_for`) and the `MODULE_OFF` rule live in
//! `vidya_core::modules`; here we only read the `module_setting` table (seeded by
//! migration 0006). Commands and the server apply path load the enabled set and
//! call `vidya_core::modules::require_module`.

use std::collections::BTreeSet;

use rusqlite::{params, Connection, OptionalExtension};
use vidya_core::modules::Module;

/// The keys of the modules that are currently ON (Core is always on, never stored).
pub fn enabled_set(conn: &Connection) -> rusqlite::Result<BTreeSet<String>> {
    let mut stmt = conn.prepare("SELECT key FROM module_setting WHERE enabled=1")?;
    let keys = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<BTreeSet<String>>>()?;
    Ok(keys)
}

/// Is one module key enabled? Core → always true; an unknown/absent key → false.
pub fn is_enabled(conn: &Connection, key: &str) -> rusqlite::Result<bool> {
    if key == Module::Core.key() {
        return Ok(true);
    }
    let v: Option<i64> = conn
        .query_row("SELECT enabled FROM module_setting WHERE key=?1", params![key], |r| r.get(0))
        .optional()?;
    Ok(v.unwrap_or(0) != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn fresh() -> Connection {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        c
    }

    #[test]
    fn seeded_defaults_match_section_11() {
        let c = fresh();
        // §11 defaults.
        for on in ["accounts", "classroom", "hr", "circulars"] {
            assert!(is_enabled(&c, on).unwrap(), "{on} on by default");
        }
        for off in ["wa_auto", "store", "instant_sync"] {
            assert!(!is_enabled(&c, off).unwrap(), "{off} off by default");
        }
        // Core is always on and not stored.
        assert!(is_enabled(&c, "core").unwrap());
        let set = enabled_set(&c).unwrap();
        assert_eq!(set, ["accounts", "circulars", "classroom", "hr"].iter().map(|s| s.to_string()).collect());
        assert!(!set.contains("core"));
    }

    #[test]
    fn toggling_off_then_on_keeps_no_data_change_here() {
        let c = fresh();
        c.execute("UPDATE module_setting SET enabled=0 WHERE key='accounts'", []).unwrap();
        assert!(!is_enabled(&c, "accounts").unwrap());
        c.execute("UPDATE module_setting SET enabled=1 WHERE key='accounts'", []).unwrap();
        assert!(is_enabled(&c, "accounts").unwrap());
    }
}
