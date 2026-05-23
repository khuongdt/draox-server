use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::RwLock;
use tracing::{debug, info, warn};

/// The state of the circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation — all requests are allowed.
    Closed,
    /// Circuit is tripped — requests are rejected.
    Open,
    /// Testing recovery — a limited number of requests are allowed through.
    HalfOpen,
}

/// Circuit breaker pattern for protecting backend services.
///
/// Transitions:
/// - **Closed -> Open**: when `failure_count` >= `failure_threshold`
/// - **Open -> HalfOpen**: when `open_duration_ms` has elapsed since last failure
/// - **HalfOpen -> Closed**: when `success_count` >= `success_threshold`
/// - **HalfOpen -> Open**: on any failure
pub struct CircuitBreaker {
    state: RwLock<CircuitState>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    failure_threshold: u32,
    success_threshold: u32,
    open_duration_ms: u64,
    last_failure_time: AtomicU64,
}

impl CircuitBreaker {
    /// Create a new circuit breaker.
    ///
    /// - `failure_threshold`: number of failures in Closed state to trip to Open
    /// - `success_threshold`: number of successes in HalfOpen state to close
    /// - `open_duration_ms`: how long to stay Open before transitioning to HalfOpen
    pub fn new(failure_threshold: u32, success_threshold: u32, open_duration_ms: u64) -> Self {
        Self {
            state: RwLock::new(CircuitState::Closed),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            failure_threshold,
            success_threshold,
            open_duration_ms,
            last_failure_time: AtomicU64::new(0),
        }
    }

    /// Check if a request should be allowed through the circuit breaker.
    ///
    /// - **Closed**: always allows
    /// - **Open**: checks if the open duration has elapsed; if so, transitions
    ///   to HalfOpen and allows; otherwise rejects
    /// - **HalfOpen**: allows (successes/failures are tracked by the caller)
    pub fn allow_request(&self) -> bool {
        let state = *self.state.read().unwrap();

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let last_failure = self.last_failure_time.load(Ordering::Acquire);
                let now = Self::now_millis();

                if now.saturating_sub(last_failure) >= self.open_duration_ms {
                    // Transition to HalfOpen
                    if let Ok(mut s) = self.state.write() {
                        if *s == CircuitState::Open {
                            *s = CircuitState::HalfOpen;
                            self.success_count.store(0, Ordering::Release);
                            info!("Circuit breaker transitioning Open -> HalfOpen");
                        }
                    }
                    true
                } else {
                    debug!("Circuit breaker is Open — rejecting request");
                    false
                }
            }
            CircuitState::HalfOpen => {
                debug!("Circuit breaker is HalfOpen — allowing probe request");
                true
            }
        }
    }

    /// Record a successful request.
    ///
    /// - In **HalfOpen**: increments success count; if threshold reached,
    ///   transitions to Closed.
    /// - In **Closed**: resets the failure count.
    pub fn record_success(&self) {
        let state = *self.state.read().unwrap();

        match state {
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::AcqRel) + 1;
                debug!(
                    "Circuit breaker HalfOpen success {}/{}",
                    count, self.success_threshold
                );

                if count >= self.success_threshold {
                    if let Ok(mut s) = self.state.write() {
                        if *s == CircuitState::HalfOpen {
                            *s = CircuitState::Closed;
                            self.failure_count.store(0, Ordering::Release);
                            self.success_count.store(0, Ordering::Release);
                            info!("Circuit breaker transitioning HalfOpen -> Closed");
                        }
                    }
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success in closed state
                self.failure_count.store(0, Ordering::Release);
            }
            CircuitState::Open => {
                // Ignore successes while open (shouldn't normally happen)
            }
        }
    }

    /// Record a failed request.
    ///
    /// - In **Closed**: increments failure count; if threshold reached,
    ///   transitions to Open.
    /// - In **HalfOpen**: immediately transitions back to Open.
    pub fn record_failure(&self) {
        let now = Self::now_millis();
        self.last_failure_time.store(now, Ordering::Release);

        let state = *self.state.read().unwrap();

        match state {
            CircuitState::Closed => {
                let count = self.failure_count.fetch_add(1, Ordering::AcqRel) + 1;
                debug!(
                    "Circuit breaker Closed failure {}/{}",
                    count, self.failure_threshold
                );

                if count >= self.failure_threshold {
                    if let Ok(mut s) = self.state.write() {
                        if *s == CircuitState::Closed {
                            *s = CircuitState::Open;
                            warn!(
                                "Circuit breaker tripped: Closed -> Open (failures: {})",
                                count
                            );
                        }
                    }
                }
            }
            CircuitState::HalfOpen => {
                if let Ok(mut s) = self.state.write() {
                    if *s == CircuitState::HalfOpen {
                        *s = CircuitState::Open;
                        self.success_count.store(0, Ordering::Release);
                        warn!("Circuit breaker tripped: HalfOpen -> Open (failure during probe)");
                    }
                }
            }
            CircuitState::Open => {
                // Already open, just update last_failure_time (done above)
            }
        }
    }

    /// Get the current circuit state.
    pub fn state(&self) -> CircuitState {
        *self.state.read().unwrap()
    }

    /// Get the current failure count.
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::Acquire)
    }

    /// Current time in milliseconds since UNIX epoch.
    fn now_millis() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_closed() {
        let cb = CircuitBreaker::new(3, 2, 1000);
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_request());
    }

    #[test]
    fn test_closed_to_open_on_failures() {
        let cb = CircuitBreaker::new(3, 2, 5000);

        // Record failures up to threshold
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Requests should be rejected when Open
        assert!(!cb.allow_request());
    }

    #[test]
    fn test_open_to_half_open_after_timeout() {
        // Use a very short open duration so it expires immediately
        let cb = CircuitBreaker::new(2, 1, 0);

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // With 0ms duration, allow_request should transition to HalfOpen
        assert!(cb.allow_request());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_half_open_success_closes_circuit() {
        let cb = CircuitBreaker::new(2, 2, 0);

        // Trip to Open
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Transition to HalfOpen
        assert!(cb.allow_request());
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Record successes
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);

        // Should allow requests again
        assert!(cb.allow_request());
    }

    #[test]
    fn test_half_open_failure_opens_circuit_again() {
        let cb = CircuitBreaker::new(2, 2, 0);

        // Trip to Open
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Transition to HalfOpen
        assert!(cb.allow_request());
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Failure during HalfOpen should go back to Open
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_success_resets_failure_count_in_closed() {
        let cb = CircuitBreaker::new(3, 1, 1000);

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 2);

        // A success resets the counter
        cb.record_success();
        assert_eq!(cb.failure_count(), 0);

        // Need 3 more failures to trip now
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_open_rejects_before_timeout() {
        // Long timeout so we stay Open
        let cb = CircuitBreaker::new(1, 1, 60_000);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Should still reject — 60 seconds haven't passed
        assert!(!cb.allow_request());
        assert_eq!(cb.state(), CircuitState::Open);
    }
}
