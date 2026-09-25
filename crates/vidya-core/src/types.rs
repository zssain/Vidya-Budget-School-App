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

/// Request types (§7). v2 (P13) adds `Leave`, `AttendanceDuty`, `ClassNotice`
/// through the approval registry; their apply functions land in P16/P17.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestType {
    MarksCorrection,
    AttendanceCorrection,
    StudentDetails,
    PaymentReversal,
    AccessChange,
    DeviceReplacement,
    Leave,
    AttendanceDuty,
    ClassNotice,
}

impl RequestType {
    /// The stable string stored in `request.type` (matches the serde snake_case).
    pub fn as_key(&self) -> &'static str {
        match self {
            RequestType::MarksCorrection => "marks_correction",
            RequestType::AttendanceCorrection => "attendance_correction",
            RequestType::StudentDetails => "student_details",
            RequestType::PaymentReversal => "payment_reversal",
            RequestType::AccessChange => "access_change",
            RequestType::DeviceReplacement => "device_replacement",
            RequestType::Leave => "leave",
            RequestType::AttendanceDuty => "attendance_duty",
            RequestType::ClassNotice => "class_notice",
        }
    }

    /// Parse a `request.type` string back into a `RequestType`.
    pub fn from_key(s: &str) -> Option<RequestType> {
        Some(match s {
            "marks_correction" => RequestType::MarksCorrection,
            "attendance_correction" => RequestType::AttendanceCorrection,
            "student_details" => RequestType::StudentDetails,
            "payment_reversal" => RequestType::PaymentReversal,
            "access_change" => RequestType::AccessChange,
            "device_replacement" => RequestType::DeviceReplacement,
            "leave" => RequestType::Leave,
            "attendance_duty" => RequestType::AttendanceDuty,
            "class_notice" => RequestType::ClassNotice,
            _ => return None,
        })
    }

    /// Every request type (for exhaustive registry tests).
    pub const ALL: [RequestType; 9] = [
        RequestType::MarksCorrection,
        RequestType::AttendanceCorrection,
        RequestType::StudentDetails,
        RequestType::PaymentReversal,
        RequestType::AccessChange,
        RequestType::DeviceReplacement,
        RequestType::Leave,
        RequestType::AttendanceDuty,
        RequestType::ClassNotice,
    ];
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
