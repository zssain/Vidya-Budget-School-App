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
/// On a joined device these are the school PC's switches (see [`publish_to_school`]);
/// the device's own `module_setting` rows are only migration defaults.
pub fn enabled_set(conn: &Connection) -> rusqlite::Result<BTreeSet<String>> {
    if let Some(set) = published(conn)? {
        return Ok(set);
    }
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
    if let Some(set) = published(conn)? {
        return Ok(set.contains(key));
    }
    let v: Option<i64> = conn
        .query_row("SELECT enabled FROM module_setting WHERE key=?1", params![key], |r| r.get(0))
        .optional()?;
    Ok(v.unwrap_or(0) != 0)
}

/// `school.settings_json.modules` — the school PC's module switches as published to
/// devices — when read on a JOINED device; `None` on the school PC itself or when the
/// school row carries no published set yet (older school PC → local defaults).
fn published(conn: &Connection) -> rusqlite::Result<Option<BTreeSet<String>>> {
    if !crate::sync::client::is_client(conn) {
        return Ok(None);
    }
    let raw: Option<Option<String>> = conn.query_row("SELECT settings_json FROM school LIMIT 1", [], |r| r.get(0)).optional()?;
    let v: Option<serde_json::Value> = raw.flatten().and_then(|s| serde_json::from_str(&s).ok());
    Ok(v.as_ref().and_then(|v| v.get("modules")).and_then(|m| m.as_object()).map(|m| {
        m.iter().filter(|(_, on)| on.as_bool() == Some(true)).map(|(k, _)| k.clone()).collect()
    }))
}

/// School PC: publish the module switches into `school.settings_json.modules`
/// (`{key: bool}`) so joined devices learn them through the synced school row
/// (snapshot + change feed) — `module_setting` itself is not a synced table. A
/// device needs this to gate its own commands (W1: a store sale from a second PC
/// is possible only once the store is on). Writes only when the published set
/// changed, so startup calls cause no sync churn. No-op on a joined device.
pub fn publish_to_school(conn: &Connection) -> rusqlite::Result<()> {
    if crate::sync::client::is_client(conn) {
        return Ok(());
    }
    let Some((id, raw)) = conn
        .query_row("SELECT id, settings_json FROM school LIMIT 1", [], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)))
        .optional()?
    else {
        return Ok(());
    };
    let mut v: serde_json::Value = raw.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_else(|| serde_json::json!({}));
    if !v.is_object() {
        v = serde_json::json!({});
    }
    let mut map = serde_json::Map::new();
    let mut stmt = conn.prepare("SELECT key, enabled FROM module_setting ORDER BY key")?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? != 0)))? {
        let (k, on) = row?;
        map.insert(k, serde_json::Value::Bool(on));
    }
    let modules = serde_json::Value::Object(map);
    if v.get("modules") == Some(&modules) {
        return Ok(());
    }
    v["modules"] = modules;
    conn.execute("UPDATE school SET settings_json=?1 WHERE id=?2", params![v.to_string(), id])?;
    Ok(())
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
    fn a_joined_device_follows_the_switches_the_school_pc_published() {
        let server = fresh();
        server.execute("INSERT INTO school(id,name,backup_salt,created_at,updated_at) VALUES ('sch','S',x'00','t','t')", []).unwrap();
        server.execute("UPDATE module_setting SET enabled=1 WHERE key='store'", []).unwrap();
        server.execute("UPDATE module_setting SET enabled=0 WHERE key='hr'", []).unwrap();
        publish_to_school(&server).unwrap();
        let settings: String = server.query_row("SELECT settings_json FROM school", [], |r| r.get(0)).unwrap();
        // The school PC itself still reads its own table.
        assert!(is_enabled(&server, "store").unwrap());

        // A joined device holding that school row (its own table = defaults).
        let dev = fresh();
        dev.execute("INSERT INTO school(id,name,backup_salt,settings_json,created_at,updated_at) VALUES ('sch','S',x'00',?1,'t','t')", [&settings]).unwrap();
        assert!(!is_enabled(&dev, "store").unwrap(), "not joined yet → local defaults");
        dev.execute("INSERT INTO app_kv(key, value_json, updated_at) VALUES ('client_identity','{}','t')", []).unwrap();
        assert!(is_enabled(&dev, "store").unwrap(), "joined → the school PC's switch");
        assert!(!is_enabled(&dev, "hr").unwrap());
        assert!(is_enabled(&dev, "core").unwrap());
        assert!(enabled_set(&dev).unwrap().contains("store"));
        // Publishing again with nothing changed writes nothing.
        let before: i64 = server.query_row("SELECT total_changes()", [], |r| r.get(0)).unwrap();
        publish_to_school(&server).unwrap();
        let after: i64 = server.query_row("SELECT total_changes()", [], |r| r.get(0)).unwrap();
        assert_eq!(before, after);
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
