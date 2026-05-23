//! Prometheus-compatible metrics for the traffic guard layer.
//!
//! All counters use [`AtomicU64`] with [`Ordering::Relaxed`] for maximum
//! throughput — slight staleness in metrics dashboards is acceptable.
//! The reputation average is stored as `score * 100` (fixed-point) so that
//! fractional values can be recovered without floating-point atomics.

use crate::verdict::GuardVerdict;
use std::sync::atomic::{AtomicU64, Ordering};

/// Lock-free counters for the traffic-guard layer.
///
/// All fields are `pub` so that callers can read individual counters directly
/// (e.g. for admin-API exposure) without going through the snapshot machinery.
pub struct GuardMetrics {
    /// Total connections that were blocked.
    pub connections_blocked: AtomicU64,
    /// Total connections that were allowed through.
    pub connections_allowed: AtomicU64,
    /// Total connections that were throttled.
    pub connections_throttled: AtomicU64,
    /// Current number of active bans.
    pub bans_active: AtomicU64,
    /// Cumulative number of bans ever issued.
    pub bans_total: AtomicU64,
    /// Total number of reputation score adjustments applied.
    pub reputation_adjustments: AtomicU64,
    /// Average reputation score stored as `score * 100` (fixed-point).
    ///
    /// Divide by 100 to recover the floating-point average.
    pub avg_reputation_score: AtomicU64,
}

/// A serialisable, point-in-time snapshot of [`GuardMetrics`].
#[derive(Debug, Clone)]
pub struct GuardMetricsSnapshot {
    pub connections_blocked: u64,
    pub connections_allowed: u64,
    pub connections_throttled: u64,
    pub bans_active: u64,
    pub bans_total: u64,
    pub reputation_adjustments: u64,
    /// Average reputation score as a floating-point value.
    pub avg_reputation_score: f64,
}

impl GuardMetrics {
    /// Create a new `GuardMetrics` with all counters at zero.
    pub fn new() -> Self {
        Self {
            connections_blocked: AtomicU64::new(0),
            connections_allowed: AtomicU64::new(0),
            connections_throttled: AtomicU64::new(0),
            bans_active: AtomicU64::new(0),
            bans_total: AtomicU64::new(0),
            reputation_adjustments: AtomicU64::new(0),
            avg_reputation_score: AtomicU64::new(0),
        }
    }

    /// Increment the appropriate counter for the given verdict.
    pub fn record_verdict(&self, verdict: &GuardVerdict) {
        match verdict {
            GuardVerdict::Allow => {
                self.connections_allowed.fetch_add(1, Ordering::Relaxed);
            }
            GuardVerdict::Block(_) => {
                self.connections_blocked.fetch_add(1, Ordering::Relaxed);
            }
            GuardVerdict::Throttle => {
                self.connections_throttled.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Record a new ban being issued.
    ///
    /// Increments both `bans_active` (current bans) and `bans_total`
    /// (lifetime count).
    pub fn record_ban(&self) {
        self.bans_active.fetch_add(1, Ordering::Relaxed);
        self.bans_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a ban being lifted (manually or by expiry).
    ///
    /// Decrements `bans_active` using saturating subtraction so it never
    /// wraps around to `u64::MAX` due to ordering anomalies.
    pub fn record_unban(&self) {
        self.bans_active.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
            Some(v.saturating_sub(1))
        }).ok();
    }

    /// Update the stored average reputation score.
    ///
    /// `avg` is stored as `(avg * 100) as u64` for fixed-point precision.
    pub fn update_reputation_avg(&self, avg: f64) {
        // Clamp to [0, 100] range then scale to fixed-point.
        let clamped = avg.clamp(0.0, 100.0);
        let fixed = (clamped * 100.0) as u64;
        self.avg_reputation_score.store(fixed, Ordering::Relaxed);
        self.reputation_adjustments.fetch_add(1, Ordering::Relaxed);
    }

    /// Render current metrics in the Prometheus text exposition format.
    ///
    /// Each metric gets a `HELP` comment, a `TYPE` declaration, and a
    /// single gauge or counter line. `connections_*` and `bans_total` are
    /// counters; `bans_active` and `avg_reputation_score` are gauges.
    pub fn to_prometheus(&self) -> String {
        let blocked = self.connections_blocked.load(Ordering::Relaxed);
        let allowed = self.connections_allowed.load(Ordering::Relaxed);
        let throttled = self.connections_throttled.load(Ordering::Relaxed);
        let bans_active = self.bans_active.load(Ordering::Relaxed);
        let bans_total = self.bans_total.load(Ordering::Relaxed);
        let rep_adj = self.reputation_adjustments.load(Ordering::Relaxed);
        let avg_rep_fixed = self.avg_reputation_score.load(Ordering::Relaxed);
        let avg_rep = avg_rep_fixed as f64 / 100.0;

        format!(
            "# HELP draox_guard_connections_blocked_total Total connections blocked by traffic guard\n\
             # TYPE draox_guard_connections_blocked_total counter\n\
             draox_guard_connections_blocked_total {blocked}\n\
             # HELP draox_guard_connections_allowed_total Total connections allowed by traffic guard\n\
             # TYPE draox_guard_connections_allowed_total counter\n\
             draox_guard_connections_allowed_total {allowed}\n\
             # HELP draox_guard_connections_throttled_total Total connections throttled by traffic guard\n\
             # TYPE draox_guard_connections_throttled_total counter\n\
             draox_guard_connections_throttled_total {throttled}\n\
             # HELP draox_guard_bans_active Current number of active IP bans\n\
             # TYPE draox_guard_bans_active gauge\n\
             draox_guard_bans_active {bans_active}\n\
             # HELP draox_guard_bans_total Total number of bans ever issued\n\
             # TYPE draox_guard_bans_total counter\n\
             draox_guard_bans_total {bans_total}\n\
             # HELP draox_guard_reputation_adjustments_total Total reputation score adjustments\n\
             # TYPE draox_guard_reputation_adjustments_total counter\n\
             draox_guard_reputation_adjustments_total {rep_adj}\n\
             # HELP draox_guard_avg_reputation_score Average IP reputation score (0–100)\n\
             # TYPE draox_guard_avg_reputation_score gauge\n\
             draox_guard_avg_reputation_score {avg_rep:.2}\n",
        )
    }

    /// Take a point-in-time snapshot of all metrics.
    pub fn snapshot(&self) -> GuardMetricsSnapshot {
        let avg_fixed = self.avg_reputation_score.load(Ordering::Relaxed);
        GuardMetricsSnapshot {
            connections_blocked: self.connections_blocked.load(Ordering::Relaxed),
            connections_allowed: self.connections_allowed.load(Ordering::Relaxed),
            connections_throttled: self.connections_throttled.load(Ordering::Relaxed),
            bans_active: self.bans_active.load(Ordering::Relaxed),
            bans_total: self.bans_total.load(Ordering::Relaxed),
            reputation_adjustments: self.reputation_adjustments.load(Ordering::Relaxed),
            avg_reputation_score: avg_fixed as f64 / 100.0,
        }
    }
}

impl Default for GuardMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_verdict_allow() {
        let m = GuardMetrics::new();
        m.record_verdict(&GuardVerdict::Allow);
        m.record_verdict(&GuardVerdict::Allow);
        let snap = m.snapshot();
        assert_eq!(snap.connections_allowed, 2);
        assert_eq!(snap.connections_blocked, 0);
        assert_eq!(snap.connections_throttled, 0);
    }

    #[test]
    fn test_record_verdict_block_and_throttle() {
        let m = GuardMetrics::new();
        m.record_verdict(&GuardVerdict::Block("rate".to_string()));
        m.record_verdict(&GuardVerdict::Throttle);
        m.record_verdict(&GuardVerdict::Throttle);
        let snap = m.snapshot();
        assert_eq!(snap.connections_blocked, 1);
        assert_eq!(snap.connections_throttled, 2);
        assert_eq!(snap.connections_allowed, 0);
    }

    #[test]
    fn test_ban_record_and_unban() {
        let m = GuardMetrics::new();
        m.record_ban();
        m.record_ban();
        m.record_ban();
        assert_eq!(m.bans_active.load(Ordering::Relaxed), 3);
        assert_eq!(m.bans_total.load(Ordering::Relaxed), 3);

        m.record_unban();
        assert_eq!(m.bans_active.load(Ordering::Relaxed), 2);
        // Total should not decrease — it is a lifetime counter.
        assert_eq!(m.bans_total.load(Ordering::Relaxed), 3);

        // Unban more times than bans_active — must not wrap.
        m.record_unban();
        m.record_unban();
        m.record_unban();
        assert_eq!(m.bans_active.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_update_reputation_avg() {
        let m = GuardMetrics::new();
        m.update_reputation_avg(75.5);
        let snap = m.snapshot();
        assert!((snap.avg_reputation_score - 75.5).abs() < 0.01);
        assert_eq!(snap.reputation_adjustments, 1);

        // Clamping: values outside [0, 100] are clamped.
        m.update_reputation_avg(150.0);
        let snap2 = m.snapshot();
        assert!((snap2.avg_reputation_score - 100.0).abs() < 0.01);

        m.update_reputation_avg(-10.0);
        let snap3 = m.snapshot();
        assert!((snap3.avg_reputation_score - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_to_prometheus_contains_metric_names() {
        let m = GuardMetrics::new();
        m.record_verdict(&GuardVerdict::Allow);
        m.record_ban();
        m.update_reputation_avg(80.0);

        let prom = m.to_prometheus();
        assert!(prom.contains("draox_guard_connections_allowed_total 1"));
        assert!(prom.contains("draox_guard_bans_active 1"));
        assert!(prom.contains("draox_guard_bans_total 1"));
        assert!(prom.contains("draox_guard_avg_reputation_score 80.00"));
    }
}
