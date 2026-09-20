//! The single `license` row (id = 1, office computer only).

use rusqlite::{Connection, OptionalExtension, Transaction};

use crate::DbError;

#[derive(Debug, Clone)]
pub struct LicenseRow {
    pub school_code: String,
    pub activation_code: String,
    pub payload_b64: String,
    pub signature_b64: String,
    pub device_id: String,
    pub max_users: i64,
    pub max_devices: i64,
    pub license_type: i64,
    pub issued_on: String,
    pub activated_at: String,
}

/// Inserts or replaces the license row.
pub fn upsert(tx: &Transaction<'_>, row: &LicenseRow) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO license
         (id, school_code, activation_code, payload_b64, signature_b64, device_id, max_users,
          max_devices, license_type, issued_on, activated_at)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(id) DO UPDATE SET
             school_code = excluded.school_code, activation_code = excluded.activation_code,
             payload_b64 = excluded.payload_b64, signature_b64 = excluded.signature_b64,
             device_id = excluded.device_id, max_users = excluded.max_users,
             max_devices = excluded.max_devices, license_type = excluded.license_type,
             issued_on = excluded.issued_on, activated_at = excluded.activated_at",
        rusqlite::params![
            row.school_code,
            row.activation_code,
            row.payload_b64,
            row.signature_b64,
            row.device_id,
            row.max_users,
            row.max_devices,
            row.license_type,
            row.issued_on,
            row.activated_at
        ],
    )?;
    Ok(())
}

/// The license row, or `None` before activation.
pub fn get(conn: &Connection) -> Result<Option<LicenseRow>, DbError> {
    Ok(conn
        .query_row(
            "SELECT school_code, activation_code, payload_b64, signature_b64, device_id, max_users,
                    max_devices, license_type, issued_on, activated_at FROM license WHERE id = 1",
            [],
            |row| {
                Ok(LicenseRow {
                    school_code: row.get(0)?,
                    activation_code: row.get(1)?,
                    payload_b64: row.get(2)?,
                    signature_b64: row.get(3)?,
                    device_id: row.get(4)?,
                    max_users: row.get(5)?,
                    max_devices: row.get(6)?,
                    license_type: row.get(7)?,
                    issued_on: row.get(8)?,
                    activated_at: row.get(9)?,
                })
            },
        )
        .optional()?)
}
