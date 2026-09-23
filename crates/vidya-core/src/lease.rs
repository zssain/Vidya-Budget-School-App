//! Offline access lease (docs/00-SYSTEM-CONTEXT.md §8.8).
//!
//! Each device has `lease_expires_at = last server contact + 30 days` **[OWNER]**.
//! While the lease is active the device may show school data offline. After it
//! expires the app still opens and keeps unsent work, but hides school data and
//! shows "Connect to your school to continue" until the device reaches the
//! school server again.
//!
//! Pure: times are passed in as epoch milliseconds, no IO, no clock, no floats.

use crate::errors::{CoreError, CoreResult};

/// Default offline lease length in days **[OWNER]** (docs §8.8).
pub const DEFAULT_LEASE_DAYS: i64 = 30;

/// Milliseconds in one day (24 * 60 * 60 * 1000).
const MS_PER_DAY: i64 = 86_400_000;

/// Whether a device's offline lease is still active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LeaseStatus {
    /// Within the lease window — school data may be shown offline.
    Active,
    /// Past the lease window — hide school data until the server is reached.
    Expired,
}

/// Compute the lease status from the last successful server contact.
///
/// The lease is [`LeaseStatus::Active`] while
/// `now_ms <= last_server_contact_ms + lease_days * 86_400_000` (inclusive of the
/// exact boundary), otherwise [`LeaseStatus::Expired`].
///
/// The arithmetic saturates rather than panicking on overflow, so pathological
/// inputs degrade gracefully instead of aborting.
pub fn lease_status(last_server_contact_ms: i64, now_ms: i64, lease_days: i64) -> LeaseStatus {
    let window = lease_days.saturating_mul(MS_PER_DAY);
    let expires_at = last_server_contact_ms.saturating_add(window);
    if now_ms <= expires_at {
        LeaseStatus::Active
    } else {
        LeaseStatus::Expired
    }
}

/// Return an error when the lease has expired, otherwise `Ok(())`.
///
/// Expired → [`CoreError::LeaseExpired`].
pub fn require_active(
    last_server_contact_ms: i64,
    now_ms: i64,
    lease_days: i64,
) -> CoreResult<()> {
    match lease_status(last_server_contact_ms, now_ms, lease_days) {
        LeaseStatus::Active => Ok(()),
        LeaseStatus::Expired => Err(CoreError::LeaseExpired),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTACT: i64 = 1_700_000_000_000; // arbitrary epoch-ms anchor
    const WINDOW: i64 = DEFAULT_LEASE_DAYS * MS_PER_DAY;

    #[test]
    fn default_lease_days_is_thirty() {
        assert_eq!(DEFAULT_LEASE_DAYS, 30);
    }

    #[test]
    fn within_lease_is_active() {
        // Same instant, one day in, and one ms before the boundary.
        assert_eq!(lease_status(CONTACT, CONTACT, DEFAULT_LEASE_DAYS), LeaseStatus::Active);
        assert_eq!(
            lease_status(CONTACT, CONTACT + MS_PER_DAY, DEFAULT_LEASE_DAYS),
            LeaseStatus::Active
        );
        assert_eq!(
            lease_status(CONTACT, CONTACT + WINDOW - 1, DEFAULT_LEASE_DAYS),
            LeaseStatus::Active
        );
    }

    #[test]
    fn exactly_at_boundary_is_active() {
        // Inclusive boundary: now == last_contact + window is still Active.
        assert_eq!(
            lease_status(CONTACT, CONTACT + WINDOW, DEFAULT_LEASE_DAYS),
            LeaseStatus::Active
        );
    }

    #[test]
    fn past_boundary_is_expired() {
        assert_eq!(
            lease_status(CONTACT, CONTACT + WINDOW + 1, DEFAULT_LEASE_DAYS),
            LeaseStatus::Expired
        );
    }

    #[test]
    fn require_active_maps_to_error() {
        assert!(require_active(CONTACT, CONTACT + WINDOW, DEFAULT_LEASE_DAYS).is_ok());
        assert_eq!(
            require_active(CONTACT, CONTACT + WINDOW + 1, DEFAULT_LEASE_DAYS),
            Err(CoreError::LeaseExpired)
        );
    }

    #[test]
    fn custom_lease_days_honoured() {
        let seven = 7 * MS_PER_DAY;
        assert_eq!(lease_status(CONTACT, CONTACT + seven, 7), LeaseStatus::Active);
        assert_eq!(lease_status(CONTACT, CONTACT + seven + 1, 7), LeaseStatus::Expired);
    }
}
