//! The change-log writer. Every service write appends exactly one entry, which
//! is what sync replays. Payloads are scanned so no secret can ever be logged.

use rusqlite::Transaction;
use serde_json::Value;
use vidya_core::roles::Actor;

use crate::{error::ServiceError, Services};
use vidya_db::{repo, DbError};

/// The kind of change, matching the `op` column check constraint.
#[derive(Debug, Clone, Copy)]
pub enum Op {
    Insert,
    UpdateFields,
    Append,
    ReplaceSet,
    Event,
}

impl Op {
    pub fn as_str(self) -> &'static str {
        match self {
            Op::Insert => "insert",
            Op::UpdateFields => "update_fields",
            Op::Append => "append",
            Op::ReplaceSet => "replace_set",
            Op::Event => "event",
        }
    }
}

/// One change to record. `payload` is the minimal data sync needs to replay it.
pub struct ChangeRecord<'a> {
    pub kind: &'a str,
    pub entity: &'a str,
    pub entity_id: &'a str,
    pub op: Op,
    pub summary_key: &'a str,
    pub params: Value,
    pub payload: Value,
}

/// The metadata of a recorded change.
#[derive(Debug, Clone)]
pub struct ChangeMeta {
    pub seq: i64,
    pub change_id: String,
    pub hlc: String,
}

/// Field names that must never appear anywhere in a change-log payload.
const FORBIDDEN_KEYS: &[&str] = &["password", "passwordHash", "password_hash", "key", "secret"];

/// True if `value` contains a forbidden key at any depth.
fn contains_secret(value: &Value) -> bool {
    match value {
        Value::Object(map) => map
            .iter()
            .any(|(key, child)| FORBIDDEN_KEYS.contains(&key.as_str()) || contains_secret(child)),
        Value::Array(items) => items.iter().any(contains_secret),
        _ => false,
    }
}

/// Rejects a record whose payload or params contain a secret at any depth.
/// Call this before `write_entry` when a payload could contain user data;
/// `record` does it for you.
pub fn ensure_payload_safe(rec: &ChangeRecord<'_>) -> Result<(), ServiceError> {
    if contains_secret(&rec.payload) || contains_secret(&rec.params) {
        return Err(ServiceError::internal(
            "change-log payload contains a forbidden secret field",
        ));
    }
    Ok(())
}

/// Writes one change-log row inside the caller's transaction. Assigns the
/// `change_id` (`<device_id>:<counter>`) and the hybrid logical clock, storing
/// both `change_counter` and `hlc_last` in the same transaction.
///
/// Returns only `DbError`, so it composes inside `Db::write` closures. The
/// caller must have checked the payload (via `ensure_payload_safe` or `record`).
pub fn write_entry(
    services: &Services,
    tx: &Transaction<'_>,
    actor: Option<&Actor>,
    rec: ChangeRecord<'_>,
) -> Result<ChangeMeta, DbError> {
    let counter = repo::meta::get(tx, "change_counter")?
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0)
        + 1;
    repo::meta::set(tx, "change_counter", &counter.to_string())?;
    let change_id = format!("{}:{}", services.device_id, counter);

    let hlc = services.next_hlc();
    let hlc_text = hlc.to_text();
    repo::meta::set(tx, "hlc_last", &hlc_text)?;

    let now = services.clock.now_utc().to_rfc3339();
    tx.execute(
        "INSERT INTO change_log
         (change_id, hlc, device_id, user_id, kind, entity, entity_id, op, summary_key, params_json, payload_json, at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        rusqlite::params![
            change_id,
            hlc_text,
            services.device_id,
            actor.map(|a| a.user_id.as_str()),
            rec.kind,
            rec.entity,
            rec.entity_id,
            rec.op.as_str(),
            rec.summary_key,
            rec.params.to_string(),
            rec.payload.to_string(),
            now
        ],
    )?;
    let seq = tx.last_insert_rowid();

    Ok(ChangeMeta {
        seq,
        change_id,
        hlc: hlc_text,
    })
}

/// Checks the payload for secrets, then writes the entry. Returns a
/// `ServiceError` (secret refusal is `Internal`).
pub fn record(
    services: &Services,
    tx: &Transaction<'_>,
    actor: Option<&Actor>,
    rec: ChangeRecord<'_>,
) -> Result<ChangeMeta, ServiceError> {
    ensure_payload_safe(&rec)?;
    Ok(write_entry(services, tx, actor, rec)?)
}
