//! App state machine (prompts/P03 Step 4, docs/00-SYSTEM-CONTEXT.md).
//!
//! ```text
//! no_school ─activate→ activated ─setup step 1→ setup_in_progress(step)
//!   ─finish setup→ locked ─correct PIN→ unlocked(staff) ─lock/switch→ locked
//! ```
//! Plus three off-path states: `db_key_missing` (DB file exists but the keychain
//! entry is gone — never overwrite the DB), `needs_rejoin` (a restore/transfer
//! happened; this device must re-join), and `moved` (the licence service says the
//! school moved to another PC — this one becomes read-only).
//!
//! The UI routes purely from `app_state()`. Setup progress lives in the encrypted
//! `app_kv` table so the wizard resumes after a restart.

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::kv;

/// The number of setup-wizard steps (School, Session, Classes, You, Recovery, Ready).
pub const SETUP_STEPS: i64 = 6;
/// app_kv key: the last completed wizard step (0..=6).
pub const KV_SETUP_STEP: &str = "setup_step";
/// app_kv key: the verified licence held between activation and school creation.
pub const KV_PENDING_LICENCE: &str = "pending_licence";
/// app_kv key: the relay secret the school server presents to open its outbound
/// tunnel to the Vidya relay (P05 §10). v2 (Phase 12): activation is offline and no
/// longer issues this — it is provisioned only for the optional `instant_sync`
/// add-on (§14), which is off by default; unused otherwise.
pub const KV_RELAY_SECRET: &str = "relay_secret";

/// The staff member of the current unlocked session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionStaff {
    pub id: String,
    pub name: String,
    pub role: String,
}

/// The routing state (tagged: `{ "kind": "...", ... }`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppState {
    /// Fresh install: no licence, no school.
    NoSchool,
    /// Licence verified; the setup wizard has not created the school yet.
    Activated,
    /// Setup underway; `step` is the wizard step to show (1..=6).
    SetupInProgress { step: i64 },
    /// Setup complete; waiting for a PIN unlock.
    Locked,
    /// A staff member is signed in on this device.
    Unlocked { staff: SessionStaff },
    /// A joined client device has no local PIN yet: create one before any data shows.
    PinSetup { staff: SessionStaff },
    /// A joined client is unlocked but its school data (bootstrap snapshot) has not
    /// been completely applied yet; the sync loop keeps retrying.
    BootstrapPending { staff: SessionStaff },
    /// DB file present but its key is missing (offer Recover; never overwrite).
    DbKeyMissing,
    /// A restore/transfer means this device must re-join the school.
    NeedsRejoin,
    /// The licence moved to another PC; this one is read-only.
    Moved,
}

/// The `app_state()` response: the routing state plus the licence banner status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppStateResponse {
    pub state: AppState,
    /// `active` | `revoked` | `moved` once a licence exists (drives banners).
    pub licence_status: Option<String>,
}

fn school_exists(conn: &Connection) -> rusqlite::Result<bool> {
    Ok(conn
        .query_row("SELECT 1 FROM school LIMIT 1", [], |_| Ok(()))
        .optional()?
        .is_some())
}

fn licence_status(conn: &Connection) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT status FROM licence LIMIT 1", [], |r| r.get::<_, String>(0))
        .optional()
}

fn any_device_needs_rejoin(conn: &Connection) -> rusqlite::Result<bool> {
    Ok(conn
        .query_row("SELECT 1 FROM device WHERE needs_rejoin = 1 LIMIT 1", [], |_| Ok(()))
        .optional()?
        .is_some())
}

/// Compute the state from the (opened) DB and the current in-memory session.
pub fn compute(conn: &Connection, session: Option<SessionStaff>) -> rusqlite::Result<AppStateResponse> {
    let lic_status = licence_status(conn)?;
    let has_school = school_exists(conn)?;

    // A joined CLIENT device (it stored a client identity at join). The join token
    // authenticates the DEVICE for sync; it never signs a human in. The device needs its
    // own local PIN (security::device_pin) before any school data is shown, and is locked
    // on every launch and after the idle timeout. Never fires on the school server.
    if let Some(cid) = crate::sync::client::load(conn) {
        let staff = SessionStaff { id: cid.staff_id, name: cid.staff_name, role: cid.staff_role };
        let state = if !crate::security::device_pin::is_set(conn)? {
            AppState::PinSetup { staff }
        } else {
            match session {
                // Only the staff this device joined as can hold its session.
                Some(s) if s.id == staff.id => {
                    if crate::sync::engine::bootstrap_complete(conn) {
                        AppState::Unlocked { staff: s }
                    } else {
                        AppState::BootstrapPending { staff: s }
                    }
                }
                _ => AppState::Locked,
            }
        };
        return Ok(AppStateResponse { state, licence_status: lic_status });
    }

    // Off-path states take precedence once a school exists.
    if has_school {
        if lic_status.as_deref() == Some("moved") {
            return Ok(AppStateResponse { state: AppState::Moved, licence_status: lic_status });
        }
        if any_device_needs_rejoin(conn)? {
            return Ok(AppStateResponse { state: AppState::NeedsRejoin, licence_status: lic_status });
        }
    }

    let step: i64 = kv::get(conn, KV_SETUP_STEP)?.unwrap_or(0);

    let state = if !has_school {
        // No school row yet: either a fresh install or just-activated.
        if kv::exists(conn, KV_PENDING_LICENCE)? {
            AppState::Activated
        } else {
            AppState::NoSchool
        }
    } else if step < SETUP_STEPS {
        // School created but the wizard has not finished; resume at the next step.
        AppState::SetupInProgress { step: (step + 1).min(SETUP_STEPS) }
    } else {
        match session {
            Some(staff) => AppState::Unlocked { staff },
            None => AppState::Locked,
        }
    };

    Ok(AppStateResponse { state, licence_status: lic_status })
}

/// The state when the DB cannot be opened because its key is missing.
pub fn db_key_missing() -> AppStateResponse {
    AppStateResponse { state: AppState::DbKeyMissing, licence_status: None }
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

    fn make_school(conn: &Connection) {
        conn.execute(
            "INSERT INTO school(id, name, backup_salt, created_at, updated_at) \
             VALUES ('sch1','S', x'00', 't', 't')",
            [],
        )
        .unwrap();
    }

    #[test]
    fn fresh_install_is_no_school() {
        let c = fresh();
        assert_eq!(compute(&c, None).unwrap().state, AppState::NoSchool);
    }

    #[test]
    fn pending_licence_is_activated() {
        let c = fresh();
        kv::set(&c, KV_PENDING_LICENCE, &serde_json::json!({"licence_id":"l"})).unwrap();
        assert_eq!(compute(&c, None).unwrap().state, AppState::Activated);
    }

    #[test]
    fn school_mid_setup_is_setup_in_progress() {
        let c = fresh();
        make_school(&c);
        kv::set(&c, KV_SETUP_STEP, &3i64).unwrap();
        assert_eq!(
            compute(&c, None).unwrap().state,
            AppState::SetupInProgress { step: 4 }
        );
    }

    #[test]
    fn finished_setup_locks_then_unlocks() {
        let c = fresh();
        make_school(&c);
        kv::set(&c, KV_SETUP_STEP, &SETUP_STEPS).unwrap();
        assert_eq!(compute(&c, None).unwrap().state, AppState::Locked);
        let staff = SessionStaff { id: "p1".into(), name: "Priya".into(), role: "principal".into() };
        assert_eq!(
            compute(&c, Some(staff.clone())).unwrap().state,
            AppState::Unlocked { staff }
        );
    }

    #[test]
    fn moved_licence_wins_over_setup() {
        let c = fresh();
        make_school(&c);
        kv::set(&c, KV_SETUP_STEP, &SETUP_STEPS).unwrap();
        c.execute(
            "INSERT INTO licence(licence_id, school_id, plan, issued_at, signature, raw_json, status) \
             VALUES ('l','sch1','perpetual','t','sig','{}','moved')",
            [],
        )
        .unwrap();
        let resp = compute(&c, None).unwrap();
        assert_eq!(resp.state, AppState::Moved);
        assert_eq!(resp.licence_status.as_deref(), Some("moved"));
    }

    #[test]
    fn needs_rejoin_detected() {
        let c = fresh();
        make_school(&c);
        kv::set(&c, KV_SETUP_STEP, &SETUP_STEPS).unwrap();
        c.execute(
            "INSERT INTO staff(id,name,role,created_at,updated_at) VALUES ('s1','T','teacher','t','t')",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO device(id,staff_id,platform,name,token_hash,needs_rejoin) \
             VALUES ('d1','s1','macos','Mac','h',1)",
            [],
        )
        .unwrap();
        assert_eq!(compute(&c, None).unwrap().state, AppState::NeedsRejoin);
    }

    fn join_as_teacher(c: &Connection) {
        let id = crate::sync::client::ClientIdentity {
            device_id: "dev1".into(),
            device_token: "tok".into(),
            staff_id: "stf-t".into(),
            staff_name: "Meena".into(),
            staff_role: "teacher".into(),
            session_key: String::new(),
            school_id: "sch1".into(),
            school_name: "S".into(),
            receipt_series: "R".into(),
            admission_series: "A".into(),
            audience_keys: vec![],
            server_fingerprint: "fp".into(),
            server_addrs: vec![],
            server_port: 47650,
            server_epoch: 1,
            bootstrap_cursor: 0,
            lease_expires_at: String::new(),
            protocol: 1,
        };
        crate::sync::client::store(c, &id).unwrap();
    }

    #[test]
    fn joined_client_without_pin_must_create_one() {
        let c = fresh();
        join_as_teacher(&c);
        match compute(&c, None).unwrap().state {
            AppState::PinSetup { staff } => assert_eq!(staff.id, "stf-t"),
            other => panic!("expected PinSetup, got {other:?}"),
        }
    }

    #[test]
    fn joined_client_with_pin_starts_locked_and_unlocks_only_as_its_staff() {
        let c = fresh();
        join_as_teacher(&c);
        crate::security::device_pin::create(&c, "4821").unwrap();
        // Cold start: locked (never auto-signed-in).
        assert_eq!(compute(&c, None).unwrap().state, AppState::Locked);
        let me = SessionStaff { id: "stf-t".into(), name: "Meena".into(), role: "teacher".into() };
        // Unlocked, but no complete bootstrap yet → the loading state, not an empty home.
        assert_eq!(compute(&c, Some(me.clone())).unwrap().state, AppState::BootstrapPending { staff: me.clone() });
        kv::set(&c, crate::sync::engine::KV_BOOTSTRAP_COMPLETE, &true).unwrap();
        assert_eq!(compute(&c, Some(me.clone())).unwrap().state, AppState::Unlocked { staff: me });
        // A session for any other staff id is not honoured on this device.
        let other = SessionStaff { id: "stf-p".into(), name: "P".into(), role: "principal".into() };
        assert_eq!(compute(&c, Some(other)).unwrap().state, AppState::Locked);
    }

    #[test]
    fn db_key_missing_helper() {
        assert_eq!(db_key_missing().state, AppState::DbKeyMissing);
    }
}
