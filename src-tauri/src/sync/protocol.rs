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

/// A per-audience sealing key (§8.8). `key_b64` is the raw key; version supports rotation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudienceKey {
    pub audience: String,
    pub key_b64: String,
    pub version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinResp {
    pub device_id: String,
    pub device_token: String,
    pub staff: StaffLite,
    pub receipt_series: String,
    pub admission_series: String,
    pub audience_keys: Vec<AudienceKey>,
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
            audience_keys: vec![AudienceKey { audience: "class:1".into(), key_b64: "AA==".into(), version: 1 }],
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
    }
}
