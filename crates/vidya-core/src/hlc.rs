//! hlc — Hybrid Logical Clock for ordering and conflict detection (§8.1).
//!
//! An HLC is `{ wall_ms, counter, device_id }`. Its string form is fixed-width
//! and sortable: `wall_ms` zero-padded to 13 digits, `counter` zero-padded to 4
//! digits, then the raw `device_id` (§8.1). String comparison of the string
//! form equals the [`Ord`] total order (wall_ms, then counter, then device_id).
//!
//! The clock only orders events; it never decides who is "right" (§8.1). All
//! wall-clock values are passed in — this module reads no clock and has no IO.
//!
//! `now` is monotonic even when the wall clock stalls or goes **backwards**: it
//! never emits a value that is not strictly greater than `prev`. `receive`
//! implements the canonical HLC receive across a local and a remote clock.

use crate::errors::{CoreError, CoreResult};

/// Width of the zero-padded `wall_ms` field in the string form (§8.1).
/// 13 digits covers every millisecond timestamp up to the year 10889.
const WALL_WIDTH: usize = 13;
/// Width of the zero-padded `counter` field in the string form (§8.1).
const COUNTER_WIDTH: usize = 4;
/// Clock-skew threshold: 10 minutes in milliseconds (§8.1 skew flag).
const SKEW_LIMIT_MS: u64 = 600_000;

/// A Hybrid Logical Clock timestamp.
///
/// [`Ord`]/[`PartialOrd`] are a **total** order over `(wall_ms, counter,
/// device_id)`, matching a lexicographic comparison of [`Hlc::to_string_form`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hlc {
    /// Wall-clock milliseconds since the Unix epoch (passed in, never read).
    pub wall_ms: u64,
    /// Logical counter, breaking ties within the same `wall_ms`.
    pub counter: u16,
    /// The device that produced this timestamp; final tie-breaker.
    pub device_id: String,
}

impl Ord for Hlc {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Total order: wall_ms, then counter, then device_id (§8.1).
        self.wall_ms
            .cmp(&other.wall_ms)
            .then(self.counter.cmp(&other.counter))
            .then_with(|| self.device_id.cmp(&other.device_id))
    }
}

impl PartialOrd for Hlc {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hlc {
    /// Construct an [`Hlc`] directly from its parts.
    pub fn new(wall_ms: u64, counter: u16, device_id: impl Into<String>) -> Hlc {
        Hlc { wall_ms, counter, device_id: device_id.into() }
    }

    /// The fixed-width, sortable string form (§8.1):
    /// `wall_ms(13) + counter(4) + device_id`.
    ///
    /// Lexicographic comparison of two string forms equals [`Ord`] on the
    /// [`Hlc`] values.
    ///
    /// ```
    /// use vidya_core::hlc::Hlc;
    /// assert_eq!(Hlc::new(42, 7, "d1").to_string_form(), "00000000000420007d1");
    /// ```
    pub fn to_string_form(&self) -> String {
        format!(
            "{:0wall$}{:0ctr$}{}",
            self.wall_ms,
            self.counter,
            self.device_id,
            wall = WALL_WIDTH,
            ctr = COUNTER_WIDTH,
        )
    }

    /// Parse the string form produced by [`Hlc::to_string_form`].
    ///
    /// The first 13 characters are `wall_ms`, the next 4 are `counter`, and the
    /// remainder (possibly empty) is `device_id`. Returns
    /// `CoreError::validation("hlc", ...)` on any malformed input.
    pub fn parse(s: &str) -> CoreResult<Hlc> {
        let bad = |rule: &str| CoreError::validation("hlc", rule);

        // Must be long enough to hold both fixed-width numeric fields.
        if s.len() < WALL_WIDTH + COUNTER_WIDTH {
            return Err(bad("too_short"));
        }
        let (wall_str, rest) = s.split_at(WALL_WIDTH);
        let (ctr_str, device_id) = rest.split_at(COUNTER_WIDTH);

        // Both numeric fields must be pure ASCII digits so the round-trip and
        // the sort order are preserved.
        if !wall_str.bytes().all(|b| b.is_ascii_digit())
            || !ctr_str.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(bad("format"));
        }

        let wall_ms = wall_str.parse::<u64>().map_err(|_| bad("wall_ms"))?;
        let counter = ctr_str.parse::<u16>().map_err(|_| bad("counter"))?;

        Ok(Hlc { wall_ms, counter, device_id: device_id.to_string() })
    }
}

/// Produce the next local timestamp, monotonic even if the wall clock stalls or
/// goes **backwards** (§8.1).
///
/// * If `wall_ms > prev.wall_ms` → `(wall_ms, 0)` — the physical clock moved
///   forward, so the counter resets.
/// * Otherwise → `(prev.wall_ms, prev.counter + 1)` — the physical clock
///   stalled or regressed, so we keep the larger wall time and advance the
///   counter, guaranteeing the result is strictly greater than `prev`.
///
/// With no `prev` (first event on a device) the result is `(wall_ms, 0)`.
pub fn now(prev: Option<&Hlc>, wall_ms: u64, device_id: &str) -> Hlc {
    match prev {
        Some(p) if wall_ms > p.wall_ms => Hlc::new(wall_ms, 0, device_id),
        Some(p) => Hlc::new(p.wall_ms, p.counter + 1, device_id),
        None => Hlc::new(wall_ms, 0, device_id),
    }
}

/// Canonical HLC receive: merge a `local` clock, an incoming `remote` clock and
/// the current physical `wall_ms` into a new local timestamp (§8.1).
///
/// The new wall time is `max(local.wall_ms, remote.wall_ms, wall_ms)`; the
/// counter is chosen so the result is strictly greater than both inputs:
///
/// * physical clock ahead of both  → counter `0`;
/// * tie with local only           → `local.counter + 1`;
/// * tie with remote only          → `remote.counter + 1`;
/// * tie with both                 → `max(local.counter, remote.counter) + 1`.
pub fn receive(local: &Hlc, remote: &Hlc, wall_ms: u64, device_id: &str) -> Hlc {
    let new_wall = wall_ms.max(local.wall_ms).max(remote.wall_ms);

    let counter = if new_wall == local.wall_ms && new_wall == remote.wall_ms {
        // Both logical clocks are at the new wall time: beat the larger.
        local.counter.max(remote.counter) + 1
    } else if new_wall == local.wall_ms {
        // Only the local clock is at the new wall time.
        local.counter + 1
    } else if new_wall == remote.wall_ms {
        // Only the remote clock is at the new wall time.
        remote.counter + 1
    } else {
        // The physical clock advanced past both logical clocks: fresh counter.
        0
    };

    Hlc::new(new_wall, counter, device_id)
}

/// Clock-skew warning flag (§8.1): `true` when two devices' wall clocks differ
/// by more than 10 minutes (600_000 ms). Symmetric in its arguments.
pub fn skew_flag(local_wall_ms: u64, remote_wall_ms: u64) -> bool {
    local_wall_ms.abs_diff(remote_wall_ms) > SKEW_LIMIT_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_monotonic_when_wall_advances() {
        let prev = Hlc::new(1_000, 5, "d1");
        let next = now(Some(&prev), 2_000, "d1");
        assert_eq!(next, Hlc::new(2_000, 0, "d1"));
        assert!(next > prev);
    }

    #[test]
    fn now_monotonic_when_wall_goes_backward() {
        // Wall clock jumps backwards (NTP correction, timezone bug, etc.).
        let prev = Hlc::new(5_000, 3, "d1");
        let next = now(Some(&prev), 4_000, "d1");
        // Keeps the larger wall time and advances the counter.
        assert_eq!(next, Hlc::new(5_000, 4, "d1"));
        assert!(next > prev);
    }

    #[test]
    fn now_monotonic_when_wall_stalls() {
        let prev = Hlc::new(5_000, 3, "d1");
        let next = now(Some(&prev), 5_000, "d1");
        assert_eq!(next, Hlc::new(5_000, 4, "d1"));
        assert!(next > prev);
    }

    #[test]
    fn now_first_event_resets_counter() {
        assert_eq!(now(None, 9_999, "d1"), Hlc::new(9_999, 0, "d1"));
    }

    #[test]
    fn now_counter_advances_repeatedly_under_stall() {
        // A stalled wall clock still yields strictly increasing HLCs.
        let mut cur = Hlc::new(1_000, 0, "d1");
        for expected in 1..=5u16 {
            let next = now(Some(&cur), 1_000, "d1");
            assert_eq!(next.counter, expected);
            assert!(next > cur);
            cur = next;
        }
    }

    #[test]
    fn string_form_is_fixed_width() {
        let h = Hlc::new(42, 7, "dev-XYZ");
        let s = h.to_string_form();
        assert_eq!(s, "00000000000420007dev-XYZ");
        // 13 digits + 4 digits + device_id.
        assert_eq!(&s[..13], "0000000000042");
        assert_eq!(&s[13..17], "0007");
        assert_eq!(&s[17..], "dev-XYZ");
    }

    #[test]
    fn string_form_sorts_identically_to_ord() {
        let items = vec![
            Hlc::new(2_000, 0, "a"),
            Hlc::new(1_000, 9, "z"),
            Hlc::new(1_000, 9, "a"),
            Hlc::new(1_000, 0, "z"),
            Hlc::new(2_000, 0, "a"), // duplicate
            Hlc::new(1_000, 10, "a"),
        ];

        // Sort by Ord.
        let mut by_ord = items.clone();
        by_ord.sort();

        // Sort by string form (what a database TEXT index would do).
        let mut by_str = items;
        by_str.sort_by_key(|h| h.to_string_form());

        assert_eq!(by_ord, by_str);
    }

    #[test]
    fn string_form_counter_orders_before_device_id() {
        // A higher counter must sort after, even if the device_id sorts before.
        let low_ctr_high_dev = Hlc::new(1_000, 1, "zzz");
        let high_ctr_low_dev = Hlc::new(1_000, 2, "aaa");
        assert!(low_ctr_high_dev < high_ctr_low_dev);
        assert!(low_ctr_high_dev.to_string_form() < high_ctr_low_dev.to_string_form());
    }

    #[test]
    fn parse_round_trips() {
        for h in [
            Hlc::new(0, 0, ""),
            Hlc::new(42, 7, "dev-XYZ"),
            Hlc::new(1_700_000_000_000, 9999, "phone-A2"),
        ] {
            let parsed = Hlc::parse(&h.to_string_form()).unwrap();
            assert_eq!(parsed, h);
        }
    }

    #[test]
    fn parse_rejects_malformed() {
        assert!(Hlc::parse("short").is_err()); // too short
        assert!(Hlc::parse("abcdefghijklm0000d1").is_err()); // non-digit wall
        assert!(Hlc::parse("0000000000042zzzzd1").is_err()); // non-digit counter
    }

    #[test]
    fn receive_advances_past_both_when_wall_leads() {
        let local = Hlc::new(1_000, 4, "d1");
        let remote = Hlc::new(1_500, 2, "d2");
        let got = receive(&local, &remote, 3_000, "d1");
        // Physical clock is ahead of both → counter resets.
        assert_eq!(got, Hlc::new(3_000, 0, "d1"));
        assert!(got > local);
        assert!(got > remote);
    }

    #[test]
    fn receive_ties_with_remote_only() {
        let local = Hlc::new(1_000, 4, "d1");
        let remote = Hlc::new(2_000, 2, "d2");
        let got = receive(&local, &remote, 1_500, "d1");
        // new_wall = 2_000 = remote.wall → remote.counter + 1.
        assert_eq!(got, Hlc::new(2_000, 3, "d1"));
    }

    #[test]
    fn receive_ties_with_local_only() {
        let local = Hlc::new(2_000, 4, "d1");
        let remote = Hlc::new(1_000, 9, "d2");
        let got = receive(&local, &remote, 1_500, "d1");
        // new_wall = 2_000 = local.wall → local.counter + 1.
        assert_eq!(got, Hlc::new(2_000, 5, "d1"));
    }

    #[test]
    fn receive_ties_with_both() {
        let local = Hlc::new(2_000, 4, "d1");
        let remote = Hlc::new(2_000, 9, "d2");
        let got = receive(&local, &remote, 2_000, "d1");
        // All equal → max(counter) + 1.
        assert_eq!(got, Hlc::new(2_000, 10, "d1"));
        assert!(got > local);
        // Strictly greater than the remote logical position.
        assert!(got.wall_ms == remote.wall_ms && got.counter > remote.counter);
    }

    #[test]
    fn skew_flag_over_ten_minutes() {
        // Exactly 10 min is NOT flagged; one ms over is.
        assert!(!skew_flag(0, SKEW_LIMIT_MS));
        assert!(skew_flag(0, SKEW_LIMIT_MS + 1));
        // Symmetric.
        assert!(skew_flag(SKEW_LIMIT_MS + 1, 0));
        assert!(!skew_flag(1_000_000, 1_000_000));
    }
}
