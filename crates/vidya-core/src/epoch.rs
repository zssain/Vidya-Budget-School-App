//! Server epoch fencing (docs/00-SYSTEM-CONTEXT.md §8.9).
//!
//! `school.server_epoch` starts at 1 and increases on restore to a new PC or a
//! licence transfer. Every server response carries the epoch. A device that
//! already knows some epoch MUST refuse a server whose epoch is **lower** than
//! the one it knows — that server is a stale/old machine that has been fenced
//! out. An equal or higher epoch is accepted.
//!
//! Pure: epochs are passed in, no IO, no clock, no floats.

use crate::errors::{CoreError, CoreResult};

/// Reject a server whose `server_epoch` is older than the `known_epoch` the
/// device already trusts.
///
/// * `server_epoch < known_epoch` → [`CoreError::EpochOld`]
/// * `server_epoch >= known_epoch` → `Ok(())`
pub fn check_epoch(known_epoch: u64, server_epoch: u64) -> CoreResult<()> {
    if server_epoch < known_epoch {
        Err(CoreError::EpochOld)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_epoch_is_rejected() {
        assert_eq!(check_epoch(5, 4), Err(CoreError::EpochOld));
        assert_eq!(check_epoch(2, 1), Err(CoreError::EpochOld));
        assert_eq!(check_epoch(u64::MAX, 0), Err(CoreError::EpochOld));
    }

    #[test]
    fn equal_epoch_is_ok() {
        assert!(check_epoch(1, 1).is_ok());
        assert!(check_epoch(7, 7).is_ok());
        assert!(check_epoch(0, 0).is_ok());
    }

    #[test]
    fn higher_epoch_is_ok() {
        assert!(check_epoch(1, 2).is_ok());
        assert!(check_epoch(4, 5).is_ok());
        assert!(check_epoch(0, u64::MAX).is_ok());
    }
}
