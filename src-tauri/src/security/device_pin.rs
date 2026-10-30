//! The local PIN of a **joined client device** (second PC / Android).
//!
//! Joining enrols the *device* (its token authenticates sync); it does not sign a
//! human in. A joined device therefore needs its own PIN before any school data is
//! shown, and starts locked on every launch. The PIN is per device, never copied from
//! the school PC (a client only ever receives the public staff columns), and lives in
//! the encrypted `app_kv` table — not in `staff.pin_hash` — so a snapshot or a pull
//! can never overwrite or clear it. Hashing and the lockout curve are the shared
//! `vidya_core::pin` rules (§9), the same as the school PC and the iPhone app.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::{CmdError, CmdResult};
use crate::kv;
use crate::security::pin;
use vidya_core::errors::CoreError;

/// `app_kv` key for the device PIN record.
pub const KV_DEVICE_PIN: &str = "device_pin";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DevicePin {
    /// Argon2id PHC string.
    phc: String,
    fail_count: u32,
    /// Unix ms until which unlock attempts are refused.
    locked_until_ms: Option<i64>,
}

fn load(conn: &Connection) -> rusqlite::Result<Option<DevicePin>> {
    kv::get(conn, KV_DEVICE_PIN)
}

/// True once this device has a PIN.
pub fn is_set(conn: &Connection) -> rusqlite::Result<bool> {
    Ok(load(conn)?.is_some())
}

/// Create the device PIN. Refused if one already exists (changing it is a separate,
/// authenticated action), so this can never be used to take over a set-up device.
pub fn create(conn: &Connection, pin_value: &str) -> CmdResult<()> {
    vidya_core::validation::validate_pin(pin_value)?;
    if is_set(conn)? {
        return Err(CmdError::forbidden("pin_already_set"));
    }
    let phc = pin::hash_pin(pin_value).map_err(CmdError::internal)?;
    kv::set(conn, KV_DEVICE_PIN, &DevicePin { phc, fail_count: 0, locked_until_ms: None })?;
    Ok(())
}

/// Verify the device PIN with the persisted §9 lockout (survives restarts).
pub fn verify(conn: &Connection, pin_value: &str, now_ms: i64) -> CmdResult<()> {
    let mut rec = load(conn)?.ok_or_else(|| CmdError::validation("pin", "not_set"))?;
    if pin::is_locked(rec.locked_until_ms, now_ms) {
        return Err(CoreError::PinLocked { until: iso_from_ms(rec.locked_until_ms.unwrap_or(now_ms)) }.into());
    }
    if pin::verify_pin(pin_value, &rec.phc) {
        rec.fail_count = 0;
        rec.locked_until_ms = None;
        kv::set(conn, KV_DEVICE_PIN, &rec)?;
        return Ok(());
    }
    rec.fail_count += 1;
    rec.locked_until_ms = pin::lockout_seconds(rec.fail_count).map(|s| now_ms + (s as i64) * 1000);
    kv::set(conn, KV_DEVICE_PIN, &rec)?;
    match rec.locked_until_ms {
        Some(until) => Err(CoreError::PinLocked { until: iso_from_ms(until) }.into()),
        None => Err(CoreError::PinWrong { remaining: pin::remaining_before_lock(rec.fail_count) }.into()),
    }
}

fn iso_from_ms(ms: i64) -> String {
    time::OffsetDateTime::from_unix_timestamp(ms / 1000)
        .ok()
        .and_then(|t| t.format(&time::format_description::well_known::Rfc3339).ok())
        .unwrap_or_default()
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
    fn create_then_verify() {
        let c = fresh();
        assert!(!is_set(&c).unwrap());
        create(&c, "4821").unwrap();
        assert!(is_set(&c).unwrap());
        verify(&c, "4821", 1_000).unwrap();
    }

    #[test]
    fn create_twice_is_refused() {
        let c = fresh();
        create(&c, "4821").unwrap();
        let e = create(&c, "9999").unwrap_err();
        assert_eq!(e.code, "FORBIDDEN");
        verify(&c, "4821", 1_000).unwrap();
    }

    #[test]
    fn invalid_pin_rejected() {
        let c = fresh();
        assert!(create(&c, "12").is_err());
        assert!(create(&c, "1234567").is_err());
        assert!(!is_set(&c).unwrap());
    }

    #[test]
    fn wrong_pins_lock_out_and_lockout_persists() {
        let c = fresh();
        create(&c, "4821").unwrap();
        let now = 10_000_000;
        for i in 1..=4 {
            let e = verify(&c, "0000", now).unwrap_err();
            assert_eq!(e.code, "PIN_WRONG", "attempt {i}");
        }
        let e = verify(&c, "0000", now).unwrap_err();
        assert_eq!(e.code, "PIN_LOCKED");
        // Even the right PIN is refused during the lockout (state is in the DB, so it
        // survives an app restart).
        assert_eq!(verify(&c, "4821", now + 1_000).unwrap_err().code, "PIN_LOCKED");
        // After the wait the right PIN works and resets the counter.
        verify(&c, "4821", now + 31_000).unwrap();
        assert_eq!(verify(&c, "0000", now + 32_000).unwrap_err().code, "PIN_WRONG");
    }
}
