//! Wire protocol types (prompts/P04 Step 1). Every response carries `protocol`
//! and `server_epoch`. Field names match the §3 endpoint contract exactly.

use serde::{Deserialize, Serialize};

/// The wire protocol version. A client/server mismatch → HTTP 426.
pub const PROTOCOL: u32 = 1;

/// The port the school server binds first, then 47651..=47659 (OWNER default).
pub const DEFAULT_PORT: u16 = 47650;
pub const PORT_RANGE_END: u16 = 47659;

/// Op status returned by `/sync/push` per op.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpStatus {
    Confirmed,
    Rejected,
    Conflict,
    Flagged,
}

/// Stable error/reason codes on the wire (§2/§3).
pub mod codes {
    pub const DEVICE_REVOKED_OR_UNKNOWN: &str = "DEVICE_REVOKED_OR_UNKNOWN";
    pub const EPOCH_OLD: &str = "EPOCH_OLD";
    pub const PROTOCOL_MISMATCH: &str = "PROTOCOL_MISMATCH";
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const FORBIDDEN: &str = "FORBIDDEN";
    pub const INVITE_INVALID: &str = "INVITE_INVALID";
    pub const RATE_LIMITED: &str = "RATE_LIMITED";
    pub const BODY_TOO_LARGE: &str = "BODY_TOO_LARGE";
    /// An op whose table or a payload column name is not a plain SQL identifier
    /// (rejected before any SQL is built — see `is_safe_ident`).
    pub const MALFORMED: &str = "MALFORMED";
    /// A new attendance `L` (Leave) mark carried by an op whose HLC is at/after the
    /// v2 Present/Absent cutover (00-SYSTEM-CONTEXT §7a). The device is running an
    /// old Vidya; the user is told to update. Pre-cutover L ops apply as legacy.
    pub const LEAVE_MARK_REMOVED: &str = "LEAVE_MARK_REMOVED";
    /// An op whose action belongs to a module the school has switched off
    /// (00-SYSTEM-CONTEXT §14). The server never applies it while the module is off.
    pub const MODULE_OFF: &str = "MODULE_OFF";
    /// A table/kind the server has no rule for (deny by default), or a `delete`
    /// (no tombstone path yet). Never applied.
    pub const UNSUPPORTED_OP: &str = "UNSUPPORTED_OP";
    /// An op_id already used by another device or with different content. The first
    /// result stands; this one is refused and not remembered.
    pub const DUPLICATE_OP: &str = "DUPLICATE_OP";
    /// The change targets locked (submitted) data: raise a correction request.
    pub const NEEDS_REQUEST: &str = "NEEDS_REQUEST";
    /// The Principal may change this locked data only with an audited reason, which
    /// is done on the school PC (not through a synced op).
    pub const REASON_REQUIRED: &str = "REASON_REQUIRED";
}

/// True iff `s` is a plain SQL identifier: 1–64 chars, ASCII letter/underscore
/// first, then letters/digits/underscores. Table and column names arrive inside
/// AEAD-sealed ops from peer devices and can never be bound as SQL parameters
/// (identifiers aren't parameterisable), so every dynamic-identifier SQL path in
/// the sync engine validates them through here first — a value containing a
/// quote, semicolon, parenthesis or space cannot pass, which closes SQL
/// injection even though the identifier is interpolated. (prompts/P09 §2.)
pub fn is_safe_ident(s: &str) -> bool {
    if !(1..=64).contains(&s.len()) {
        return false;
    }
    let mut bytes = s.bytes();
    match bytes.next() {
        Some(c) if c.is_ascii_alphabetic() || c == b'_' => {}
        _ => return false,
    }
    bytes.all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

/// True iff `table` and every key of `payload` (when it is a JSON object) are
/// plain SQL identifiers. Ops that fail this are rejected as `MALFORMED`.
pub fn op_identifiers_safe(table: &str, payload: &serde_json::Value) -> bool {
    if !is_safe_ident(table) {
        return false;
    }
    match payload.as_object() {
        Some(obj) => obj.keys().all(|k| is_safe_ident(k)),
        None => true,
    }
}

/// One recorded change (docs §8.1). Serde mirror of `crate::write::Op` for the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Op {
    pub op_id: String,
    pub hlc: String,
    pub device_id: String,
    pub staff_id: String,
    pub audience: String,
    pub table: String,
    pub record_id: String,
    /// insert | update | delete | action
    pub kind: String,
    /// JSON object of the changed row's fields (or action params).
    pub payload: serde_json::Value,
    pub base_version: Option<i64>,
    pub server_epoch: i64,
}

// -------------------------------------------------------------- /hello -------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloResp {
    pub school_id: String,
    pub school_name: String,
    pub server_time: String,
    pub protocol: u32,
    pub server_epoch: i64,
}

// --------------------------------------------------------------- /join -------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinReq {
    pub invite_code: String,
    pub device_name: String,
    pub platform: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub google_email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffLite {
    pub id: String,
    pub name: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchoolLite {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinResp {
    pub device_id: String,
    pub device_token: String,
    pub staff: StaffLite,
    pub receipt_series: String,
    pub admission_series: String,
    // No audience keys (P21 A8): nothing school PC -> device is sealed under an
    // audience key any more (uploads use the device's c2s key, the Drive down export
    // and acks its s2c key, LAN is pinned TLS), so a join issues none and a revoked
    // device holds nothing that opens future data. Per-device keys die with the
    // device row (`revoked_at` -> never sealed for again).
    pub session_key: String,
    pub school: SchoolLite,
    pub lease_expires_at: String,
    pub bootstrap_cursor: i64,
    pub protocol: u32,
    pub server_epoch: i64,
}

// ----------------------------------------------------------- /sync/push ------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushReq {
    pub ops: Vec<Op>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpResult {
    pub op_id: String,
    pub status: OpStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_seq: Option<i64>,
    /// The canonical row after applying (so the client replaces its optimistic copy).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushResp {
    pub results: Vec<OpResult>,
    pub server_time: String,
    pub protocol: u32,
    pub server_epoch: i64,
}

// ----------------------------------------------------------- /sync/pull ------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub table: String,
    pub record_id: String,
    pub payload: serde_json::Value,
    pub server_seq: i64,
    pub hlc: Option<String>,
}

// ------------------------------------------------------- /sync/snapshot ------

/// What a bootstrap snapshot claims to contain (G1). The client checks the rows it
/// received against it before committing anything; a mismatch (a truncated body, a
/// dropped or altered row, a server bug that lost rows) refuses the whole snapshot
/// and the sync loop's self-heal fetches a fresh one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotManifest {
    /// Manifest format version.
    pub v: u32,
    /// Unique per snapshot build (diagnostics; stored on the device once applied).
    pub snapshot_id: String,
    pub server_epoch: i64,
    /// The school PC's op_log position (`MAX(server_seq)`) the rows reflect; read
    /// under the same DB lock as the rows.
    pub cursor: i64,
    /// Total rows.
    pub row_count: usize,
    /// Rows per table.
    pub tables: std::collections::BTreeMap<String, usize>,
    /// Hex SHA-256 over the header and every row in canonical form — see
    /// [`snapshot_digest`].
    pub digest: String,
}

/// Current [`SnapshotManifest::v`].
pub const SNAPSHOT_MANIFEST_V: u32 = 1;

/// `/v1/sync/snapshot` body: the manifest plus the role-scoped canonical rows.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub manifest: SnapshotManifest,
    pub changes: Vec<Change>,
}

/// JSON with object keys sorted at every level, so the digest does not depend on
/// how either side orders a row's columns.
fn canonical_json(v: &serde_json::Value, out: &mut String) {
    match v {
        serde_json::Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::Value::String((*k).clone()).to_string());
                out.push(':');
                canonical_json(&m[k.as_str()], out);
            }
            out.push('}');
        }
        serde_json::Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                canonical_json(x, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

/// The manifest digest: SHA-256 over `vidya/snapshot/v1`, the snapshot id, epoch and
/// cursor, then every row as `table \0 record_id \0 canonical(payload)` in
/// (table, record_id) order — order-independent, so only the SET of rows counts.
pub fn snapshot_digest(snapshot_id: &str, server_epoch: i64, cursor: i64, changes: &[Change]) -> String {
    use sha2::{Digest, Sha256};
    let mut rows: Vec<String> = changes
        .iter()
        .map(|c| {
            let mut s = String::new();
            s.push_str(&c.table);
            s.push('\0');
            s.push_str(&c.record_id);
            s.push('\0');
            canonical_json(&c.payload, &mut s);
            s
        })
        .collect();
    rows.sort();
    let mut h = Sha256::new();
    h.update(format!("vidya/snapshot/v1\n{snapshot_id}\n{server_epoch}\n{cursor}\n").as_bytes());
    for r in &rows {
        h.update(r.as_bytes());
        h.update(b"\n");
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn table_counts(changes: &[Change]) -> std::collections::BTreeMap<String, usize> {
    let mut m = std::collections::BTreeMap::new();
    for c in changes {
        *m.entry(c.table.clone()).or_insert(0) += 1;
    }
    m
}

impl SnapshotManifest {
    /// Build the manifest for `changes` (school PC side).
    pub fn build(snapshot_id: impl Into<String>, server_epoch: i64, cursor: i64, changes: &[Change]) -> Self {
        let snapshot_id = snapshot_id.into();
        let digest = snapshot_digest(&snapshot_id, server_epoch, cursor, changes);
        Self {
            v: SNAPSHOT_MANIFEST_V,
            snapshot_id,
            server_epoch,
            cursor,
            row_count: changes.len(),
            tables: table_counts(changes),
            digest,
        }
    }

    /// Check `changes` against this manifest (device side). Names what differs.
    pub fn verify(&self, changes: &[Change]) -> Result<(), String> {
        if self.v != SNAPSHOT_MANIFEST_V {
            return Err(format!("snapshot manifest v{} not supported", self.v));
        }
        if self.snapshot_id.is_empty() {
            return Err("snapshot has no manifest".into());
        }
        if changes.len() != self.row_count {
            return Err(format!("snapshot row count {} ≠ manifest {}", changes.len(), self.row_count));
        }
        let got = table_counts(changes);
        if got != self.tables {
            let mut diff: Vec<String> = self
                .tables
                .keys()
                .chain(got.keys())
                .filter(|t| self.tables.get(*t) != got.get(*t))
                .map(|t| format!("{t}: {} ≠ {}", got.get(t).copied().unwrap_or(0), self.tables.get(t).copied().unwrap_or(0)))
                .collect();
            diff.sort();
            diff.dedup();
            return Err(format!("snapshot table counts differ from manifest ({})", diff.join(", ")));
        }
        if snapshot_digest(&self.snapshot_id, self.server_epoch, self.cursor, changes) != self.digest {
            return Err("snapshot digest does not match its manifest".into());
        }
        Ok(())
    }
}

impl Snapshot {
    /// Wrap `changes` with a fresh manifest (new snapshot id).
    pub fn new(server_epoch: i64, cursor: i64, changes: Vec<Change>) -> Self {
        let manifest = SnapshotManifest::build(uuid::Uuid::now_v7().to_string(), server_epoch, cursor, &changes);
        Self { manifest, changes }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delete {
    pub table: String,
    pub record_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullResp {
    pub changes: Vec<Change>,
    pub deletes: Vec<Delete>,
    pub next_cursor: i64,
    pub has_more: bool,
    pub server_time: String,
    pub lease_expires_at: String,
    pub protocol: u32,
    pub server_epoch: i64,
}

// ------------------------------------------------------ /device/heartbeat ----

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatReq {
    pub device_time: String,
    pub pending_count: i64,
    pub pending_payment_paise: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatResp {
    pub server_time: String,
    pub revoked: bool,
    pub lease_expires_at: String,
    pub protocol: u32,
    pub server_epoch: i64,
}

// ------------------------------------------------------------- /sealed ------

/// The opaque envelope carried over the relay (P05 Step 2). The relay sees only
/// these routing fields + the sealed blob; it can neither read nor alter the inner
/// request or response (rule §6). `sealed_b64` = base64(`nonce(12) ‖ ChaCha20-
/// Poly1305(dir_key, inner JSON)`), AAD = method + path + device_id + server_epoch.
///
/// * On a REQUEST, `server_epoch` is the epoch the device currently trusts.
/// * On a RESPONSE, `server_epoch` is the server's real epoch (drives fencing).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedEnvelope {
    pub device_id: String,
    pub method: String,
    pub path: String,
    pub server_epoch: i64,
    pub sealed_b64: String,
}

/// The sealed inner request. `counter` is the device's strictly-monotonic request
/// counter (replay defence): the server rejects `counter <= last seen`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedRequest {
    pub counter: i64,
    pub body: serde_json::Value,
}

/// The sealed inner response. `status` mirrors the HTTP status of the inner call
/// (200 ok; 409 EPOCH_OLD; …) so the device reacts without the relay seeing it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedResponse {
    pub status: u16,
    pub body: serde_json::Value,
}

/// The deep-link / QR join payload (`vidya://join?d=<base64url JSON>`), ≤ 2 KB.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinPayload {
    pub school_id: String,
    pub school_name: String,
    pub lan_addrs: Vec<String>,
    pub port: u16,
    pub cert_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay_url: Option<String>,
    pub code: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every type round-trips through JSON unchanged.
    fn roundtrip<T: Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug>(v: &T) {
        let s = serde_json::to_string(v).unwrap();
        let back: T = serde_json::from_str(&s).unwrap();
        assert_eq!(&back, v);
    }

    #[test]
    fn safe_ident_accepts_real_table_and_column_names() {
        for ok in ["student", "attendance_mark", "fee_due", "class_subject", "server_seq", "_hidden", "a", "V123"] {
            assert!(is_safe_ident(ok), "{ok} should be accepted");
        }
    }

    #[test]
    fn safe_ident_rejects_injection_and_junk() {
        for bad in [
            "",                               // empty
            "1student",                       // leading digit
            "payment; DROP TABLE payment",    // stacked statement
            "student WHERE 1=1",              // space
            "student--",                      // comment
            "pay)ment",                       // paren
            "na'me",                          // quote
            "col,other",                      // comma
            "tab\tle",                        // control char
            "школа",                          // non-ascii
        ] {
            assert!(!is_safe_ident(bad), "{bad:?} must be rejected");
        }
        // 64 chars ok, 65 rejected.
        assert!(is_safe_ident(&"a".repeat(64)));
        assert!(!is_safe_ident(&"a".repeat(65)));
    }

    #[test]
    fn op_identifiers_safe_checks_table_and_every_payload_key() {
        let good = serde_json::json!({ "id": "x", "amount_paise": 100 });
        assert!(op_identifiers_safe("payment", &good));
        // Bad table.
        assert!(!op_identifiers_safe("payment); DROP TABLE payment; --", &good));
        // Bad column key.
        let bad = serde_json::json!({ "id": "x", "amount_paise) VALUES (0); --": 1 });
        assert!(!op_identifiers_safe("payment", &bad));
        // Non-object payload (action params) is allowed as long as the table is safe.
        assert!(op_identifiers_safe("payment", &serde_json::json!("noop")));
    }

    #[test]
    fn op_status_wire_form() {
        assert_eq!(serde_json::to_string(&OpStatus::Confirmed).unwrap(), "\"confirmed\"");
        assert_eq!(serde_json::to_string(&OpStatus::Conflict).unwrap(), "\"conflict\"");
        assert_eq!(serde_json::to_string(&OpStatus::Flagged).unwrap(), "\"flagged\"");
    }

    #[test]
    fn all_types_roundtrip() {
        roundtrip(&HelloResp {
            school_id: "s".into(), school_name: "S".into(), server_time: "t".into(),
            protocol: PROTOCOL, server_epoch: 1,
        });
        roundtrip(&JoinReq {
            invite_code: "VIDYA123".into(), device_name: "Phone".into(), platform: "android".into(),
            google_email: Some("a@b.c".into()),
        });
        roundtrip(&JoinResp {
            device_id: "d".into(), device_token: "tok".into(),
            staff: StaffLite { id: "st".into(), name: "N".into(), role: "teacher".into() },
            receipt_series: "A2".into(), admission_series: "A2".into(),
            session_key: "AA==".into(), school: SchoolLite { id: "s".into(), name: "S".into() },
            lease_expires_at: "t".into(), bootstrap_cursor: 0, protocol: PROTOCOL, server_epoch: 1,
        });
        let op = Op {
            op_id: "op1".into(), hlc: "0000000000000000010001dev".into(), device_id: "dev".into(),
            staff_id: "st".into(), audience: "admin".into(), table: "student".into(),
            record_id: "stu1".into(), kind: "update".into(),
            payload: serde_json::json!({ "address": "12 Road" }), base_version: Some(1), server_epoch: 1,
        };
        roundtrip(&PushReq { ops: vec![op.clone()] });
        roundtrip(&PushResp {
            results: vec![OpResult { op_id: "op1".into(), status: OpStatus::Confirmed, server_seq: Some(7), record: Some(serde_json::json!({"id":"stu1"})), reason_code: None }],
            server_time: "t".into(), protocol: PROTOCOL, server_epoch: 1,
        });
        roundtrip(&PullResp {
            changes: vec![Change { table: "student".into(), record_id: "stu1".into(), payload: serde_json::json!({}), server_seq: 7, hlc: Some("h".into()) }],
            deletes: vec![Delete { table: "student".into(), record_id: "stu2".into() }],
            next_cursor: 7, has_more: false, server_time: "t".into(), lease_expires_at: "t".into(),
            protocol: PROTOCOL, server_epoch: 1,
        });
        roundtrip(&HeartbeatReq { device_time: "t".into(), pending_count: 3, pending_payment_paise: 2400 });
        roundtrip(&HeartbeatResp { server_time: "t".into(), revoked: false, lease_expires_at: "t".into(), protocol: PROTOCOL, server_epoch: 1 });
        roundtrip(&JoinPayload {
            school_id: "s".into(), school_name: "Saraswati".into(), lan_addrs: vec!["192.168.1.5".into()],
            port: DEFAULT_PORT, cert_sha256: "ab12".into(), relay_url: None, code: "VIDYA123".into(),
        });
        roundtrip(&SealedEnvelope {
            device_id: "dev-a2".into(), method: "POST".into(), path: "/v1/sync/push".into(),
            server_epoch: 1, sealed_b64: "AAAA".into(),
        });
        roundtrip(&SealedRequest { counter: 7, body: serde_json::json!({ "ops": [] }) });
        roundtrip(&SealedResponse { status: 200, body: serde_json::json!({ "results": [] }) });
    }

    #[test]
    fn snapshot_digest_ignores_key_and_row_order_but_not_content() {
        let ch = |t: &str, id: &str, p: serde_json::Value| Change { table: t.into(), record_id: id.into(), payload: p, server_seq: 0, hlc: None };
        let a = vec![ch("school", "s", serde_json::json!({"id":"s","name":"A","n":{"x":1,"y":[1,2]}})), ch("class", "c", serde_json::json!({"id":"c"}))];
        let mut b_obj = serde_json::Map::new();
        b_obj.insert("n".into(), serde_json::json!({"y":[1,2],"x":1}));
        b_obj.insert("name".into(), "A".into());
        b_obj.insert("id".into(), "s".into());
        let b = vec![ch("class", "c", serde_json::json!({"id":"c"})), ch("school", "s", serde_json::Value::Object(b_obj))];
        let m = SnapshotManifest::build("snap-1", 2, 9, &a);
        assert!(m.verify(&b).is_ok());
        let mut c = a.clone();
        c[0].payload["n"]["y"] = serde_json::json!([2, 1]);
        assert!(m.verify(&c).unwrap_err().contains("digest"));
        assert_ne!(snapshot_digest("snap-1", 2, 9, &a), snapshot_digest("snap-2", 2, 9, &a), "header is bound");
    }
}
