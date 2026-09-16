use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::error::DomainError;

/// A saved attendance mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttendanceStatus {
    P,
    A,
    L,
}

/// Totals for an attendance day or report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttendanceCounts {
    pub present: u32,
    pub absent: u32,
    pub leave: u32,
}

/// Advances an attendance cell through none, present, absent and leave.
pub fn next_status(current: Option<AttendanceStatus>) -> AttendanceStatus {
    match current {
        None | Some(AttendanceStatus::L) => AttendanceStatus::P,
        Some(AttendanceStatus::P) => AttendanceStatus::A,
        Some(AttendanceStatus::A) => AttendanceStatus::L,
    }
}

/// Calculates a whole-number attendance percentage rounded half up.
pub fn percent(present: u32, total: u32) -> Option<u32> {
    (total > 0).then(|| ((u64::from(present) * 100 + u64::from(total) / 2) / u64::from(total)) as u32)
}

/// Rejects future dates and unauthorized changes to past attendance.
pub fn check_attendance_date(
    date: NaiveDate,
    today: NaiveDate,
    can_edit_past: bool,
) -> Result<(), DomainError> {
    if date > today {
        Err(DomainError::validation("attendance.error.future").field("date"))
    } else if date < today && !can_edit_past {
        Err(DomainError::validation("attendance.error.past_read_only").field("date"))
    } else {
        Ok(())
    }
}
