//! Shared domain enums used across modules (docs/00-SYSTEM-CONTEXT.md §5, §7).
//! Modules define their own snapshot/input structs; these cross-cutting enums
//! live here so every module agrees on them.

use serde::{Deserialize, Serialize};

/// Staff role (§5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Principal,
    Accountant,
    Teacher,
}

/// Staff account state (§5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaffState {
    Invited,
    Active,
    Suspended,
    Removed,
}

/// Request types (§7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestType {
    MarksCorrection,
    AttendanceCorrection,
    StudentDetails,
    PaymentReversal,
    AccessChange,
    DeviceReplacement,
}

/// Payment mode (§7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMode {
    Cash,
    Upi,
    Cheque,
}

/// Attendance mark (§7): Present / Absent / Leave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Mark {
    P,
    A,
    L,
}

/// Fee head frequency (§7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeeFrequency {
    Term,
    Month,
    Once,
}

/// Row sync state (§7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    Draft,
    OnDevice,
    SharedDrive,
    Confirmed,
    Rejected,
    Conflict,
}
