//! roles — the roles-as-data repository (P13, foundation §8.10).
//!
//! The `role` rows are seeded by migration 0015; `role_permission` is derived data
//! seeded here from `vidya_core::permissions::default_permissions()` (which is
//! itself derived from `can`, so the DB matrix can never disagree with the code).
//! `can` stays the authoritative decision point; this module just materialises and
//! reads the matrix so future role templates can be added without code.

use rusqlite::{params, Connection};
use vidya_core::permissions::{Action, Effect};
use vidya_core::types::Role;

/// The built-in role id for a role.
fn role_id(role: Role) -> &'static str {
    match role {
        Role::Principal => "role-principal",
        Role::Accountant => "role-accountant",
        Role::Teacher => "role-teacher",
    }
}

fn role_from_key(key: &str) -> Option<Role> {
    match key {
        "principal" => Some(Role::Principal),
        "accountant" => Some(Role::Accountant),
        "teacher" => Some(Role::Teacher),
        _ => None,
    }
}

/// Seed `role_permission` from the built-in matrix if it is empty (idempotent).
/// Returns how many rows were inserted. Called at startup after migrations.
pub fn seed_role_permissions(conn: &mut Connection) -> rusqlite::Result<usize> {
    let existing: i64 = conn.query_row("SELECT COUNT(*) FROM role_permission", [], |r| r.get(0))?;
    if existing > 0 {
        return Ok(0);
    }
    let mut n = 0;
    let tx = conn.transaction()?;
    for (role, action, effect) in vidya_core::permissions::default_permissions() {
        tx.execute(
            "INSERT OR IGNORE INTO role_permission(role_id, action, effect) VALUES (?1,?2,?3)",
            params![role_id(role), action.as_key(), effect.as_key()],
        )?;
        n += 1;
    }
    tx.commit()?;
    Ok(n)
}

/// Load the permission set from `role_permission` (joined to `role.key`), as the
/// pure `(Role, Action, Effect)` list `vidya_core::permissions::effect_of` reads.
/// Rows for unknown roles/actions (e.g. a future custom role) are skipped.
pub fn load_permission_set(conn: &Connection) -> rusqlite::Result<Vec<(Role, Action, Effect)>> {
    let mut stmt = conn.prepare(
        "SELECT r.key, rp.action, rp.effect FROM role_permission rp JOIN role r ON r.id=rp.role_id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (rkey, akey, ekey) = row?;
        if let (Some(role), Some(action), Some(effect)) = (role_from_key(&rkey), Action::from_key(&akey), Effect::from_key(&ekey)) {
            out.push((role, action, effect));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use vidya_core::permissions::{default_permissions, effect_of};

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn seeds_and_loads_the_built_in_matrix() {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        // Migrations seed the 3 built-in roles.
        let roles: i64 = c.query_row("SELECT COUNT(*) FROM role WHERE built_in=1", [], |r| r.get(0)).unwrap();
        assert_eq!(roles, 3);

        // Seed role_permission from the built-in matrix.
        let expected = default_permissions();
        let n = seed_role_permissions(&mut c).unwrap();
        assert_eq!(n, expected.len(), "one row per (role, action) capability");
        // Re-seeding does nothing (idempotent).
        assert_eq!(seed_role_permissions(&mut c).unwrap(), 0);

        // The loaded set exactly reproduces the built-in matrix.
        let loaded = load_permission_set(&c).unwrap();
        assert_eq!(loaded.len(), expected.len());
        for role in [Role::Principal, Role::Accountant, Role::Teacher] {
            for action in Action::ALL {
                assert_eq!(effect_of(&loaded, role, action), effect_of(&expected, role, action), "{role:?} {action:?}");
            }
        }
    }
}
