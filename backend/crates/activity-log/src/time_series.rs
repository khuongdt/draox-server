use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::RwLock;

// ────────────────────────────────────────────────────────
// BucketSize
// ────────────────────────────────────────────────────────

/// Predefined time bucket sizes for time-series aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BucketSize {
    OneMinute,
    FiveMinutes,
    OneHour,
    OneDay,
}

impl BucketSize {
    /// Return the chrono [`Duration`] that corresponds to this bucket size.
    pub fn duration(&self) -> Duration {
        match self {
            BucketSize::OneMinute => Duration::minutes(1),
            BucketSize::FiveMinutes => Duration::minutes(5),
            BucketSize::OneHour => Duration::hours(1),
            BucketSize::OneDay => Duration::days(1),
        }
    }
}

// ────────────────────────────────────────────────────────
// TimeSeriesBucket
// ────────────────────────────────────────────────────────

/// A single time bucket that aggregates recorded values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesBucket {
    pub timestamp: DateTime<Utc>,
    pub count: u64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
}

impl TimeSeriesBucket {
    /// Create an empty bucket starting at `timestamp`.
    pub fn new(timestamp: DateTime<Utc>) -> Self {
        Self {
            timestamp,
            count: 0,
            sum: 0.0,
            min: f64::MAX,
            max: f64::MIN,
        }
    }

    /// Record a single value into this bucket.
    pub fn record(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
    }

    /// Compute the arithmetic mean of all recorded values.
    ///
    /// Returns `0.0` when the bucket is empty.
    pub fn avg(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }
}

// ────────────────────────────────────────────────────────
// TimeSeries
// ────────────────────────────────────────────────────────

/// Time-series data structure that maintains buckets of a fixed size.
///
/// Buckets are stored in a ring buffer (`VecDeque`), evicting the oldest
/// entries once `max_buckets` is exceeded.  All public methods are
/// thread-safe via an internal `RwLock`.
pub struct TimeSeries {
    bucket_size: BucketSize,
    buckets: RwLock<VecDeque<TimeSeriesBucket>>,
    max_buckets: usize,
}

impl TimeSeries {
    /// Create a new, empty time series.
    pub fn new(bucket_size: BucketSize, max_buckets: usize) -> Self {
        Self {
            bucket_size,
            buckets: RwLock::new(VecDeque::with_capacity(max_buckets)),
            max_buckets,
        }
    }

    /// Record a value at the current time.
    pub fn record(&self, value: f64) {
        self.record_at(Utc::now(), value);
    }

    /// Record a value at a specific timestamp (useful for testing).
    pub fn record_at(&self, time: DateTime<Utc>, value: f64) {
        let bucket_start = self.bucket_start(time);
        let mut buckets = self.buckets.write().unwrap();

        // Append to the current bucket if it matches.
        if let Some(last) = buckets.back_mut() {
            if last.timestamp == bucket_start {
                last.record(value);
                return;
            }
        }

        // Otherwise create a new bucket.
        let mut bucket = TimeSeriesBucket::new(bucket_start);
        bucket.record(value);
        buckets.push_back(bucket);

        // Evict the oldest buckets when over capacity.
        while buckets.len() > self.max_buckets {
            buckets.pop_front();
        }
    }

    /// Return all buckets whose timestamp falls within `[from, to)`.
    pub fn range(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<TimeSeriesBucket> {
        let buckets = self.buckets.read().unwrap();
        buckets
            .iter()
            .filter(|b| b.timestamp >= from && b.timestamp < to)
            .cloned()
            .collect()
    }

    /// Return the last `n` buckets (or fewer if not enough exist).
    pub fn last_n(&self, n: usize) -> Vec<TimeSeriesBucket> {
        let buckets = self.buckets.read().unwrap();
        let start = buckets.len().saturating_sub(n);
        buckets.iter().skip(start).cloned().collect()
    }

    /// Total number of buckets currently stored.
    pub fn bucket_count(&self) -> usize {
        self.buckets.read().unwrap().len()
    }

    /// Align `time` to the start of its enclosing bucket.
    fn bucket_start(&self, time: DateTime<Utc>) -> DateTime<Utc> {
        let duration = self.bucket_size.duration();
        let secs = duration.num_seconds();
        let ts = time.timestamp();
        let aligned = (ts / secs) * secs;
        DateTime::from_timestamp(aligned, 0).unwrap_or(time)
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_record_and_avg() {
        let ts = Utc::now();
        let mut bucket = TimeSeriesBucket::new(ts);
        assert_eq!(bucket.avg(), 0.0);

        bucket.record(10.0);
        bucket.record(20.0);
        bucket.record(30.0);

        assert_eq!(bucket.count, 3);
        assert!((bucket.sum - 60.0).abs() < f64::EPSILON);
        assert!((bucket.min - 10.0).abs() < f64::EPSILON);
        assert!((bucket.max - 30.0).abs() < f64::EPSILON);
        assert!((bucket.avg() - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_timeseries_record_same_bucket() {
        let ts = TimeSeries::new(BucketSize::OneHour, 100);
        let now = Utc::now();

        // Two values in the same bucket should not create two buckets.
        ts.record_at(now, 5.0);
        ts.record_at(now, 15.0);

        assert_eq!(ts.bucket_count(), 1);

        let buckets = ts.last_n(10);
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].count, 2);
        assert!((buckets[0].avg() - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_timeseries_range_query() {
        let ts = TimeSeries::new(BucketSize::OneHour, 100);

        // Record into three different hourly buckets.
        let base = DateTime::from_timestamp(3600 * 1000, 0).unwrap(); // some aligned hour
        let hour1 = base;
        let hour2 = base + Duration::hours(1);
        let hour3 = base + Duration::hours(2);

        ts.record_at(hour1, 1.0);
        ts.record_at(hour2, 2.0);
        ts.record_at(hour3, 3.0);

        // Query a range that covers only the middle bucket.
        let result = ts.range(hour2, hour3);
        assert_eq!(result.len(), 1);
        assert!((result[0].sum - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_timeseries_eviction() {
        let ts = TimeSeries::new(BucketSize::OneMinute, 3);

        let base = DateTime::from_timestamp(60 * 1000, 0).unwrap();
        for i in 0..5 {
            let t = base + Duration::minutes(i);
            ts.record_at(t, i as f64);
        }

        // max_buckets=3, so only the last 3 should remain.
        assert_eq!(ts.bucket_count(), 3);

        let buckets = ts.last_n(10);
        assert_eq!(buckets.len(), 3);
        // The first two buckets (i=0, i=1) should have been evicted.
        assert!((buckets[0].sum - 2.0).abs() < f64::EPSILON);
        assert!((buckets[1].sum - 3.0).abs() < f64::EPSILON);
        assert!((buckets[2].sum - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_bucket_size_durations() {
        assert_eq!(BucketSize::OneMinute.duration().num_seconds(), 60);
        assert_eq!(BucketSize::FiveMinutes.duration().num_seconds(), 300);
        assert_eq!(BucketSize::OneHour.duration().num_seconds(), 3600);
        assert_eq!(BucketSize::OneDay.duration().num_seconds(), 86400);
    }
}
