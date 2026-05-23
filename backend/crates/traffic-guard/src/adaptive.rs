//! Adaptive throttling based on system load.
//!
//! Monitors CPU and memory usage via `sysinfo` and adjusts throttling
//! when the system is overloaded. The [`AdaptiveThrottle`] manager tracks
//! consecutive overload readings and transitions between [`ThrottleState::Normal`]
//! and [`ThrottleState::Throttled`], exposing a multiplicative factor that
//! callers can apply to their rate limits.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sysinfo::System;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for adaptive throttling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveConfig {
    /// CPU usage threshold (0.0–100.0) above which the system is considered
    /// overloaded.
    #[serde(default = "default_cpu_threshold")]
    pub cpu_threshold_percent: f64,

    /// Memory usage threshold (0.0–100.0) above which the system is considered
    /// overloaded.
    #[serde(default = "default_memory_threshold")]
    pub memory_threshold_percent: f64,

    /// Multiplicative factor applied to rate limits while throttled (0.0–1.0).
    /// For example, `0.5` halves all rate limits.
    #[serde(default = "default_throttle_factor")]
    pub throttle_factor: f64,

    /// Interval in seconds between system metric refreshes.
    #[serde(default = "default_check_interval")]
    pub check_interval_secs: u64,

    /// Number of consecutive overload readings required before throttling
    /// activates.
    #[serde(default = "default_overload_count")]
    pub overload_count_threshold: u32,
}

fn default_cpu_threshold() -> f64 {
    80.0
}
fn default_memory_threshold() -> f64 {
    85.0
}
fn default_throttle_factor() -> f64 {
    0.5
}
fn default_check_interval() -> u64 {
    5
}
fn default_overload_count() -> u32 {
    3
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            cpu_threshold_percent: default_cpu_threshold(),
            memory_threshold_percent: default_memory_threshold(),
            throttle_factor: default_throttle_factor(),
            check_interval_secs: default_check_interval(),
            overload_count_threshold: default_overload_count(),
        }
    }
}

// ---------------------------------------------------------------------------
// System load snapshot
// ---------------------------------------------------------------------------

/// Point-in-time snapshot of system resource usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemLoad {
    /// CPU usage as a percentage (0.0–100.0).
    pub cpu_percent: f64,
    /// Memory usage as a percentage (0.0–100.0).
    pub memory_percent: f64,
    /// Used memory in bytes.
    pub memory_used_bytes: u64,
    /// Total physical memory in bytes.
    pub memory_total_bytes: u64,
    /// When this snapshot was taken.
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Throttle state
// ---------------------------------------------------------------------------

/// Whether the adaptive throttle is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThrottleState {
    /// System load is within acceptable bounds — no throttling applied.
    Normal,
    /// System load exceeded thresholds — rate limits are reduced.
    Throttled,
}

// ---------------------------------------------------------------------------
// Adaptive snapshot (for API responses)
// ---------------------------------------------------------------------------

/// Serialisable snapshot of the full adaptive throttle state, suitable for
/// admin API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveSnapshot {
    pub state: ThrottleState,
    pub current_factor: f64,
    pub consecutive_overloads: u64,
    pub last_load: Option<SystemLoad>,
    pub config: AdaptiveConfig,
}

// ---------------------------------------------------------------------------
// AdaptiveThrottle
// ---------------------------------------------------------------------------

/// Monitors system load and transitions between [`ThrottleState::Normal`] and
/// [`ThrottleState::Throttled`] based on CPU/memory thresholds.
///
/// The caller is expected to invoke [`AdaptiveThrottle::refresh`] periodically
/// (e.g. every `config.check_interval_secs` seconds). All public methods use
/// `std::sync` locks so the struct is `Send + Sync` without requiring a Tokio
/// runtime.
pub struct AdaptiveThrottle {
    config: AdaptiveConfig,
    system: RwLock<System>,
    state: RwLock<ThrottleState>,
    consecutive_overloads: AtomicU64,
    last_load: RwLock<Option<SystemLoad>>,
}

impl AdaptiveThrottle {
    /// Create a new `AdaptiveThrottle` with the given configuration.
    ///
    /// The initial state is [`ThrottleState::Normal`].
    pub fn new(config: AdaptiveConfig) -> Self {
        Self {
            config,
            system: RwLock::new(System::new()),
            state: RwLock::new(ThrottleState::Normal),
            consecutive_overloads: AtomicU64::new(0),
            last_load: RwLock::new(None),
        }
    }

    /// Refresh system metrics and update the throttle state.
    ///
    /// Call this periodically (e.g. every [`AdaptiveConfig::check_interval_secs`]
    /// seconds). The method:
    ///
    /// 1. Refreshes CPU and memory readings via `sysinfo`.
    /// 2. Stores a [`SystemLoad`] snapshot.
    /// 3. Compares readings against thresholds and updates state.
    pub fn refresh(&self) {
        // 1. Refresh sysinfo data.
        let (cpu_percent, memory_used, memory_total) = {
            let mut sys = self.system.write().expect("system lock poisoned");
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            (
                sys.global_cpu_usage() as f64,
                sys.used_memory(),
                sys.total_memory(),
            )
        };

        // 2. Compute memory percentage (guard against zero total).
        let memory_percent = if memory_total > 0 {
            (memory_used as f64 / memory_total as f64) * 100.0
        } else {
            0.0
        };

        // 3. Store the load snapshot.
        let load = SystemLoad {
            cpu_percent,
            memory_percent,
            memory_used_bytes: memory_used,
            memory_total_bytes: memory_total,
            timestamp: Utc::now(),
        };

        {
            let mut last = self.last_load.write().expect("last_load lock poisoned");
            *last = Some(load);
        }

        // 4. Determine if the system is overloaded.
        let overloaded = cpu_percent > self.config.cpu_threshold_percent
            || memory_percent > self.config.memory_threshold_percent;

        // 5. Update consecutive counter and throttle state.
        if overloaded {
            let count = self.consecutive_overloads.fetch_add(1, Ordering::Relaxed) + 1;
            if count >= self.config.overload_count_threshold as u64 {
                let mut state = self.state.write().expect("state lock poisoned");
                *state = ThrottleState::Throttled;
            }
        } else {
            self.consecutive_overloads.store(0, Ordering::Relaxed);
            let mut state = self.state.write().expect("state lock poisoned");
            *state = ThrottleState::Normal;
        }
    }

    /// Get the current throttle state.
    pub fn state(&self) -> ThrottleState {
        *self.state.read().expect("state lock poisoned")
    }

    /// Get the current throttle factor.
    ///
    /// Returns `1.0` when [`ThrottleState::Normal`] (no reduction) and
    /// [`AdaptiveConfig::throttle_factor`] when [`ThrottleState::Throttled`].
    pub fn current_factor(&self) -> f64 {
        match self.state() {
            ThrottleState::Normal => 1.0,
            ThrottleState::Throttled => self.config.throttle_factor,
        }
    }

    /// Get the latest system load snapshot, if any.
    ///
    /// Returns `None` until [`refresh`](Self::refresh) has been called at least
    /// once.
    pub fn last_load(&self) -> Option<SystemLoad> {
        self.last_load.read().expect("last_load lock poisoned").clone()
    }

    /// Check if the latest readings indicate an overloaded system.
    ///
    /// Returns `true` when either CPU **or** memory usage exceeds its
    /// configured threshold. Returns `false` if no load snapshot is available.
    pub fn is_overloaded(&self) -> bool {
        let load = self.last_load.read().expect("last_load lock poisoned");
        match load.as_ref() {
            Some(l) => {
                l.cpu_percent > self.config.cpu_threshold_percent
                    || l.memory_percent > self.config.memory_threshold_percent
            }
            None => false,
        }
    }

    /// Get a reference to the current configuration.
    pub fn config(&self) -> &AdaptiveConfig {
        &self.config
    }

    /// Create a serialisable snapshot of the full adaptive throttle state.
    pub fn snapshot(&self) -> AdaptiveSnapshot {
        AdaptiveSnapshot {
            state: self.state(),
            current_factor: self.current_factor(),
            consecutive_overloads: self.consecutive_overloads.load(Ordering::Relaxed),
            last_load: self.last_load(),
            config: self.config.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = AdaptiveConfig::default();
        assert!((cfg.cpu_threshold_percent - 80.0).abs() < f64::EPSILON);
        assert!((cfg.memory_threshold_percent - 85.0).abs() < f64::EPSILON);
        assert!((cfg.throttle_factor - 0.5).abs() < f64::EPSILON);
        assert_eq!(cfg.check_interval_secs, 5);
        assert_eq!(cfg.overload_count_threshold, 3);
    }

    #[test]
    fn test_initial_state_is_normal() {
        let throttle = AdaptiveThrottle::new(AdaptiveConfig::default());
        assert_eq!(throttle.state(), ThrottleState::Normal);
    }

    #[test]
    fn test_current_factor_normal() {
        let throttle = AdaptiveThrottle::new(AdaptiveConfig::default());
        assert!((throttle.current_factor() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_refresh_updates_load() {
        let throttle = AdaptiveThrottle::new(AdaptiveConfig::default());
        assert!(throttle.last_load().is_none());
        throttle.refresh();
        assert!(throttle.last_load().is_some());
    }

    #[test]
    fn test_snapshot() {
        let throttle = AdaptiveThrottle::new(AdaptiveConfig::default());
        throttle.refresh();

        let snap = throttle.snapshot();
        assert_eq!(snap.state, ThrottleState::Normal);
        assert!((snap.current_factor - 1.0).abs() < f64::EPSILON);
        assert!(snap.last_load.is_some());
        assert!((snap.config.cpu_threshold_percent - 80.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_throttle_activation() {
        // Thresholds of 0.0 mean any real system will exceed them.
        let cfg = AdaptiveConfig {
            cpu_threshold_percent: 0.0,
            memory_threshold_percent: 0.0,
            throttle_factor: 0.3,
            check_interval_secs: 1,
            overload_count_threshold: 1,
        };
        let throttle = AdaptiveThrottle::new(cfg);

        // The first refresh should already trigger throttling because
        // memory usage is always > 0% and overload_count_threshold is 1.
        throttle.refresh();

        assert_eq!(throttle.state(), ThrottleState::Throttled);
        assert!((throttle.current_factor() - 0.3).abs() < f64::EPSILON);
        assert!(throttle.is_overloaded());
    }
}
