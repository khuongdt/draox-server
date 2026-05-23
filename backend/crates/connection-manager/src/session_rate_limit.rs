//! Per-session rate limiting.
//!
//! Aggregates traffic across all connections in a session and enforces a
//! maximum request rate per session. Uses a fixed time window (token bucket
//! approximation): the counter resets when a new window starts.

use dashmap::DashMap;
use server_core::SessionId;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Rate-limit state for one session window.
struct WindowState {
    /// Number of requests recorded in the current window.
    count: AtomicU64,
    /// When the current window started.
    window_start: Instant,
}

impl WindowState {
    fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
            window_start: Instant::now(),
        }
    }
}

/// Enforces a per-session rate limit across all connections belonging to a
/// session.
///
/// A new window starts each time `window_size` elapses since the previous
/// window start. Within a window the counter increments on every call to
/// `check`; the call returns `false` (denied) once the count exceeds
/// `max_per_second * window_size_secs`.
pub struct SessionRateLimiter {
    rates: DashMap<SessionId, WindowState>,
    /// Maximum number of requests per second.
    max_per_second: u64,
    /// Length of each counting window.
    window_size: Duration,
}

impl SessionRateLimiter {
    /// Create a new `SessionRateLimiter` with a 1-second window.
    ///
    /// `max_per_second` is the maximum number of requests allowed per window.
    pub fn new(max_per_second: u64) -> Self {
        Self::with_window(max_per_second, Duration::from_secs(1))
    }

    /// Create a `SessionRateLimiter` with a custom window size.
    ///
    /// Useful in tests and for sub-second or multi-second buckets.
    pub fn with_window(max_per_second: u64, window_size: Duration) -> Self {
        Self {
            rates: DashMap::new(),
            max_per_second,
            window_size,
        }
    }

    /// Check whether the session is within its rate limit.
    ///
    /// Returns `true` if the request is allowed, `false` if it should be
    /// rejected. A new time window resets the counter automatically.
    ///
    /// The per-window cap is always `max_per_second` — the window size
    /// controls how often the counter resets, not the magnitude of the cap.
    pub fn check(&self, session_id: &SessionId) -> bool {
        let window_cap = self.max_per_second;
        let now = Instant::now();
        let window_size = self.window_size;

        // Check whether the existing window is expired before touching the entry.
        let needs_reset = self
            .rates
            .get(session_id)
            .map(|e| now.duration_since(e.window_start) >= window_size)
            .unwrap_or(false);

        if needs_reset {
            // Drop the read guard before mutating.
            self.rates.insert(session_id.clone(), WindowState::new());
        }

        // Now fetch-or-create and increment.
        let entry = self.rates.entry(session_id.clone()).or_insert_with(WindowState::new);
        let prev = entry.count.fetch_add(1, Ordering::Relaxed);
        prev < window_cap
    }

    /// Remove entries for sessions that haven't been seen for longer than
    /// `window_size`. Reduces memory usage over time.
    pub fn cleanup(&self) {
        let now = Instant::now();
        let window_size = self.window_size;
        self.rates.retain(|_id, state| {
            now.duration_since(state.window_start) < window_size * 2
        });
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_within_limit() {
        let limiter = SessionRateLimiter::new(10);
        let sid = SessionId::new();

        for _ in 0..10 {
            assert!(limiter.check(&sid), "request within limit should be allowed");
        }
    }

    #[test]
    fn test_deny_over_limit() {
        let limiter = SessionRateLimiter::new(3);
        let sid = SessionId::new();

        assert!(limiter.check(&sid)); // 1
        assert!(limiter.check(&sid)); // 2
        assert!(limiter.check(&sid)); // 3
        assert!(!limiter.check(&sid), "4th request should be denied");
        assert!(!limiter.check(&sid), "5th request should be denied");
    }

    #[test]
    fn test_independent_sessions_do_not_share_quotas() {
        let limiter = SessionRateLimiter::new(2);
        let sid1 = SessionId::new();
        let sid2 = SessionId::new();

        assert!(limiter.check(&sid1));
        assert!(limiter.check(&sid1));
        // sid1 is now exhausted.
        assert!(!limiter.check(&sid1));

        // sid2 should still have its full quota.
        assert!(limiter.check(&sid2));
        assert!(limiter.check(&sid2));
    }

    #[test]
    fn test_window_reset_allows_new_requests() {
        // Use a very short window so we can wait for it to expire.
        let limiter = SessionRateLimiter::with_window(2, Duration::from_millis(50));

        let sid = SessionId::new();
        assert!(limiter.check(&sid)); // 1st — ok
        assert!(limiter.check(&sid)); // 2nd — ok (limit = 2)
        assert!(!limiter.check(&sid)); // 3rd — denied (window exhausted)

        // Wait comfortably past the window boundary.
        std::thread::sleep(Duration::from_millis(100));

        // New window — the first request must be allowed.
        assert!(
            limiter.check(&sid),
            "first request in new window should be allowed"
        );
    }
}
