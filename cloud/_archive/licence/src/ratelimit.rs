//! Tiny in-process fixed-window rate limiter (Step 4: "rate limits per IP and per
//! code"). No external crate — a `HashMap` of window counters. Adequate for a
//! single licence instance; a multi-instance deployment would move this to a
//! shared store (documented in the handoff).

use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    windows: HashMap<String, (Instant, u32)>,
    last_prune: Instant,
}

impl RateLimiter {
    pub fn new() -> Self {
        RateLimiter { windows: HashMap::new(), last_prune: Instant::now() }
    }

    /// Returns `true` if the action is allowed (and counts it), `false` if the
    /// key has exceeded `max` within `window`.
    pub fn allow(&mut self, key: &str, max: u32, window: Duration) -> bool {
        let now = Instant::now();
        self.maybe_prune(now, window);
        let entry = self.windows.entry(key.to_string()).or_insert((now, 0));
        if now.duration_since(entry.0) > window {
            *entry = (now, 0);
        }
        if entry.1 >= max {
            return false;
        }
        entry.1 += 1;
        true
    }

    fn maybe_prune(&mut self, now: Instant, window: Duration) {
        if now.duration_since(self.last_prune) < Duration::from_secs(60) {
            return;
        }
        self.windows.retain(|_, (start, _)| now.duration_since(*start) <= window * 2);
        self.last_prune = now;
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_max_then_blocks_and_resets() {
        let mut rl = RateLimiter::new();
        let w = Duration::from_millis(50);
        assert!(rl.allow("k", 2, w));
        assert!(rl.allow("k", 2, w));
        assert!(!rl.allow("k", 2, w), "3rd within the window is blocked");
        std::thread::sleep(Duration::from_millis(60));
        assert!(rl.allow("k", 2, w), "window elapsed → allowed again");
        // Different key is independent.
        assert!(rl.allow("other", 2, w));
    }
}
