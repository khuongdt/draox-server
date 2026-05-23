use serde::{Deserialize, Serialize};
use std::sync::RwLock;

// ────────────────────────────────────────────────────────
// PercentileSnapshot
// ────────────────────────────────────────────────────────

/// Immutable point-in-time snapshot of computed percentiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PercentileSnapshot {
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub min: f64,
    pub max: f64,
    pub count: usize,
    pub mean: f64,
}

// ────────────────────────────────────────────────────────
// PercentileTracker
// ────────────────────────────────────────────────────────

/// Tracks latency values and computes percentiles on demand.
///
/// Uses a sorted-`Vec` approach which is suitable for moderate data sizes.
/// When the number of recorded samples exceeds `max_samples`, the oldest
/// entries (at the front of the vector) are drained.
pub struct PercentileTracker {
    values: RwLock<Vec<f64>>,
    max_samples: usize,
}

impl PercentileTracker {
    /// Create a new tracker that retains at most `max_samples` values.
    pub fn new(max_samples: usize) -> Self {
        Self {
            values: RwLock::new(Vec::with_capacity(max_samples.min(1024))),
            max_samples,
        }
    }

    /// Record a latency value.
    pub fn record(&self, value: f64) {
        let mut values = self.values.write().unwrap();
        values.push(value);
        // If over max_samples, remove oldest (front) entries.
        if values.len() > self.max_samples {
            let excess = values.len() - self.max_samples;
            values.drain(0..excess);
        }
    }

    /// Compute a snapshot of the current percentile distribution.
    ///
    /// Returns a zeroed snapshot when no values have been recorded.
    pub fn snapshot(&self) -> PercentileSnapshot {
        let values = self.values.read().unwrap();
        if values.is_empty() {
            return PercentileSnapshot {
                p50: 0.0,
                p90: 0.0,
                p95: 0.0,
                p99: 0.0,
                min: 0.0,
                max: 0.0,
                count: 0,
                mean: 0.0,
            };
        }

        let mut sorted: Vec<f64> = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let count = sorted.len();
        let sum: f64 = sorted.iter().sum();

        PercentileSnapshot {
            p50: percentile_value(&sorted, 50.0),
            p90: percentile_value(&sorted, 90.0),
            p95: percentile_value(&sorted, 95.0),
            p99: percentile_value(&sorted, 99.0),
            min: sorted[0],
            max: sorted[count - 1],
            count,
            mean: sum / count as f64,
        }
    }

    /// Reset all recorded values.
    pub fn reset(&self) {
        let mut values = self.values.write().unwrap();
        values.clear();
    }

    /// Current number of recorded samples.
    pub fn sample_count(&self) -> usize {
        self.values.read().unwrap().len()
    }
}

/// Compute the value at a given percentile from a pre-sorted slice.
fn percentile_value(sorted: &[f64], pct: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((pct / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_snapshot_returns_zeros() {
        let tracker = PercentileTracker::new(1000);
        let snap = tracker.snapshot();

        assert_eq!(snap.count, 0);
        assert!((snap.p50 - 0.0).abs() < f64::EPSILON);
        assert!((snap.p90 - 0.0).abs() < f64::EPSILON);
        assert!((snap.p95 - 0.0).abs() < f64::EPSILON);
        assert!((snap.p99 - 0.0).abs() < f64::EPSILON);
        assert!((snap.min - 0.0).abs() < f64::EPSILON);
        assert!((snap.max - 0.0).abs() < f64::EPSILON);
        assert!((snap.mean - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_percentiles_1_to_100() {
        let tracker = PercentileTracker::new(10_000);

        for i in 1..=100 {
            tracker.record(i as f64);
        }

        let snap = tracker.snapshot();
        assert_eq!(snap.count, 100);
        assert!((snap.min - 1.0).abs() < f64::EPSILON);
        assert!((snap.max - 100.0).abs() < f64::EPSILON);

        // With 100 evenly-spaced values 1..=100:
        // p50 ~ 50, p90 ~ 90, p95 ~ 95, p99 ~ 99
        assert!((snap.p50 - 50.0).abs() < 1.5);
        assert!((snap.p90 - 90.0).abs() < 1.5);
        assert!((snap.p95 - 95.0).abs() < 1.5);
        assert!((snap.p99 - 99.0).abs() < 1.5);

        // Mean of 1..=100 = 50.5
        assert!((snap.mean - 50.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_max_samples_eviction() {
        let tracker = PercentileTracker::new(5);

        for i in 1..=10 {
            tracker.record(i as f64);
        }

        // Only the last 5 values (6..=10) should remain.
        assert_eq!(tracker.sample_count(), 5);

        let snap = tracker.snapshot();
        assert_eq!(snap.count, 5);
        assert!((snap.min - 6.0).abs() < f64::EPSILON);
        assert!((snap.max - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_reset() {
        let tracker = PercentileTracker::new(1000);

        tracker.record(1.0);
        tracker.record(2.0);
        tracker.record(3.0);
        assert_eq!(tracker.sample_count(), 3);

        tracker.reset();
        assert_eq!(tracker.sample_count(), 0);

        let snap = tracker.snapshot();
        assert_eq!(snap.count, 0);
    }

    #[test]
    fn test_single_value() {
        let tracker = PercentileTracker::new(1000);
        tracker.record(42.0);

        let snap = tracker.snapshot();
        assert_eq!(snap.count, 1);
        assert!((snap.p50 - 42.0).abs() < f64::EPSILON);
        assert!((snap.p90 - 42.0).abs() < f64::EPSILON);
        assert!((snap.p95 - 42.0).abs() < f64::EPSILON);
        assert!((snap.p99 - 42.0).abs() < f64::EPSILON);
        assert!((snap.min - 42.0).abs() < f64::EPSILON);
        assert!((snap.max - 42.0).abs() < f64::EPSILON);
        assert!((snap.mean - 42.0).abs() < f64::EPSILON);
    }
}
