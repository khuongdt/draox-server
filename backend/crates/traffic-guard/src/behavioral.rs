//! Behavioral analysis engine for detecting suspicious request patterns.
//!
//! Rather than relying solely on static rate limits, this module builds a
//! per-IP behavior profile over a sliding window of recent requests. The
//! profile tracks request timing and payload sizes, computing an anomaly
//! score that flags IPs as [`BehaviorFlag::Suspicious`] or
//! [`BehaviorFlag::Malicious`] when they exhibit bot-like patterns:
//!
//! - **Burst detection** — an unusual number of requests within one second.
//! - **Payload uniformity** — automated tools often send identically-sized
//!   payloads, producing near-zero variance.
//! - **Large payloads** — sustained large payloads may indicate data
//!   exfiltration or abuse.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::net::IpAddr;
use tracing::debug;

/// Behavioral analysis flags indicating the assessed threat level of an IP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorFlag {
    /// Normal traffic — no anomalies detected.
    Normal,
    /// Suspicious traffic — elevated anomaly score.
    Suspicious,
    /// Malicious traffic — anomaly score exceeds the malicious threshold.
    Malicious,
}

/// Per-IP behavior profile tracking request patterns over a sliding window.
#[derive(Debug, Clone)]
pub struct BehaviorProfile {
    /// Timestamps of recent requests (bounded by `max_history`).
    pub request_times: VecDeque<DateTime<Utc>>,
    /// Payload sizes of recent requests (bounded by `max_history`).
    pub payload_sizes: VecDeque<u64>,
    /// Current behavioral classification.
    pub flag: BehaviorFlag,
    /// Current anomaly score (higher = more suspicious).
    pub anomaly_score: f64,
    /// When this profile was last updated.
    pub last_updated: DateTime<Utc>,
}

impl BehaviorProfile {
    /// Create a new clean profile with no history.
    fn new() -> Self {
        Self {
            request_times: VecDeque::new(),
            payload_sizes: VecDeque::new(),
            flag: BehaviorFlag::Normal,
            anomaly_score: 0.0,
            last_updated: Utc::now(),
        }
    }
}

/// Behavioral analysis engine for detecting suspicious request patterns.
///
/// Maintains per-IP profiles with a bounded history of request timestamps
/// and payload sizes. On each recorded request the anomaly score is
/// recomputed and the IP's [`BehaviorFlag`] is updated accordingly.
pub struct BehavioralAnalyzer {
    profiles: DashMap<IpAddr, BehaviorProfile>,
    /// Maximum events to retain per IP for pattern analysis.
    max_history: usize,
    /// Anomaly score at or above which an IP is flagged suspicious.
    suspicious_threshold: f64,
    /// Anomaly score at or above which an IP is flagged malicious.
    malicious_threshold: f64,
}

impl BehavioralAnalyzer {
    /// Create a new analyzer.
    ///
    /// # Arguments
    ///
    /// * `max_history` — sliding window size (number of events per IP).
    /// * `suspicious_threshold` — anomaly score for [`BehaviorFlag::Suspicious`].
    /// * `malicious_threshold` — anomaly score for [`BehaviorFlag::Malicious`].
    pub fn new(
        max_history: usize,
        suspicious_threshold: f64,
        malicious_threshold: f64,
    ) -> Self {
        Self {
            profiles: DashMap::new(),
            max_history,
            suspicious_threshold,
            malicious_threshold,
        }
    }

    /// Record a request event and update behavioral analysis.
    ///
    /// The profile's sliding window is trimmed to `max_history`, the anomaly
    /// score is recalculated, and the flag is updated.
    pub fn record_request(&self, ip: IpAddr, payload_size: u64) {
        let now = Utc::now();
        let mut profile = self
            .profiles
            .entry(ip)
            .or_insert_with(BehaviorProfile::new);

        profile.request_times.push_back(now);
        profile.payload_sizes.push_back(payload_size);

        // Trim to max_history
        while profile.request_times.len() > self.max_history {
            profile.request_times.pop_front();
        }
        while profile.payload_sizes.len() > self.max_history {
            profile.payload_sizes.pop_front();
        }

        // Recalculate anomaly score
        profile.anomaly_score = Self::calculate_anomaly(&profile);
        profile.flag = if profile.anomaly_score >= self.malicious_threshold {
            BehaviorFlag::Malicious
        } else if profile.anomaly_score >= self.suspicious_threshold {
            BehaviorFlag::Suspicious
        } else {
            BehaviorFlag::Normal
        };
        profile.last_updated = now;

        debug!(
            "Behavioral analysis for {}: flag={:?}, score={:.2}",
            ip, profile.flag, profile.anomaly_score
        );
    }

    /// Calculate anomaly score based on burst detection and payload analysis.
    ///
    /// The score is a non-negative float where higher values indicate more
    /// anomalous behaviour. Components:
    ///
    /// - **Burst**: +0.5 per request above 10 in the last second.
    /// - **Payload uniformity**: +2.0 when variance < 1.0 with > 10 samples.
    /// - **Large payloads**: +3.0 when the average payload exceeds 1 MB.
    fn calculate_anomaly(profile: &BehaviorProfile) -> f64 {
        let mut score = 0.0;

        // Burst detection: count requests in last 1 second
        let one_sec_ago = Utc::now() - chrono::Duration::seconds(1);
        let recent_count = profile
            .request_times
            .iter()
            .filter(|t| **t > one_sec_ago)
            .count();
        if recent_count > 10 {
            score += (recent_count as f64 - 10.0) * 0.5;
        }

        // Payload size variance analysis
        if profile.payload_sizes.len() > 5 {
            let sizes: Vec<f64> = profile.payload_sizes.iter().map(|s| *s as f64).collect();
            let mean = sizes.iter().sum::<f64>() / sizes.len() as f64;
            let variance =
                sizes.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / sizes.len() as f64;
            // Very low variance (exact same size) is suspicious for automated requests
            if variance < 1.0 && sizes.len() > 10 {
                score += 2.0;
            }
            // Very large payloads
            if mean > 1_000_000.0 {
                score += 3.0;
            }
        }

        score
    }

    /// Get the behavior flag for an IP.
    ///
    /// Returns [`BehaviorFlag::Normal`] if the IP has no profile.
    pub fn get_flag(&self, ip: IpAddr) -> BehaviorFlag {
        self.profiles
            .get(&ip)
            .map(|p| p.flag)
            .unwrap_or(BehaviorFlag::Normal)
    }

    /// Get the anomaly score for an IP.
    ///
    /// Returns `0.0` if the IP has no profile.
    pub fn get_anomaly_score(&self, ip: IpAddr) -> f64 {
        self.profiles
            .get(&ip)
            .map(|p| p.anomaly_score)
            .unwrap_or(0.0)
    }

    /// Get the full behavior profile for an IP (cloned).
    ///
    /// Returns `None` if the IP has never been recorded.
    pub fn get_profile(&self, ip: IpAddr) -> Option<BehaviorProfile> {
        self.profiles.get(&ip).map(|p| p.clone())
    }

    /// Reset tracking for an IP, removing its profile entirely.
    pub fn reset(&self, ip: IpAddr) {
        self.profiles.remove(&ip);
        debug!("Behavioral profile reset for IP {}", ip);
    }

    /// Number of IPs currently being tracked.
    pub fn tracked_count(&self) -> usize {
        self.profiles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_requests_stay_normal() {
        let analyzer = BehavioralAnalyzer::new(100, 5.0, 10.0);
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // A few normal requests with varied payload sizes
        analyzer.record_request(ip, 500);
        analyzer.record_request(ip, 1200);
        analyzer.record_request(ip, 800);

        assert_eq!(analyzer.get_flag(ip), BehaviorFlag::Normal);
        assert!(analyzer.get_anomaly_score(ip) < 5.0);
    }

    #[test]
    fn test_burst_triggers_suspicious() {
        // suspicious_threshold = 1.0 so even a small burst triggers it
        let analyzer = BehavioralAnalyzer::new(200, 1.0, 20.0);
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        // Fire 15 requests instantly — all within the same second.
        // recent_count = 15 > 10, score += (15-10)*0.5 = 2.5 >= 1.0
        for i in 0..15 {
            analyzer.record_request(ip, 100 + i);
        }

        let flag = analyzer.get_flag(ip);
        let score = analyzer.get_anomaly_score(ip);
        assert!(
            flag == BehaviorFlag::Suspicious || flag == BehaviorFlag::Malicious,
            "Expected Suspicious or Malicious, got {:?} (score={:.2})",
            flag,
            score,
        );
        assert!(score >= 1.0);
    }

    #[test]
    fn test_burst_triggers_malicious() {
        // malicious_threshold = 5.0
        let analyzer = BehavioralAnalyzer::new(200, 1.0, 5.0);
        let ip: IpAddr = "10.0.0.2".parse().unwrap();

        // Fire 25 requests instantly — score += (25-10)*0.5 = 7.5 >= 5.0
        for i in 0..25 {
            analyzer.record_request(ip, 100 + i);
        }

        assert_eq!(analyzer.get_flag(ip), BehaviorFlag::Malicious);
        assert!(analyzer.get_anomaly_score(ip) >= 5.0);
    }

    #[test]
    fn test_payload_uniformity_analysis() {
        // Low suspicious_threshold so payload uniformity alone triggers it.
        // Needs > 10 samples with variance < 1.0 (identical sizes).
        let analyzer = BehavioralAnalyzer::new(200, 1.5, 10.0);
        let ip: IpAddr = "10.0.0.3".parse().unwrap();

        // 12 requests with the exact same payload size (variance = 0)
        for _ in 0..12 {
            analyzer.record_request(ip, 256);
        }

        // score should include +2.0 for low variance (>10 identical samples)
        // burst component may also contribute, but variance alone >= 1.5
        let flag = analyzer.get_flag(ip);
        assert!(
            flag == BehaviorFlag::Suspicious || flag == BehaviorFlag::Malicious,
            "Expected at least Suspicious for uniform payloads, got {:?}",
            flag,
        );
    }

    #[test]
    fn test_reset_clears_profile() {
        let analyzer = BehavioralAnalyzer::new(100, 5.0, 10.0);
        let ip: IpAddr = "10.0.0.4".parse().unwrap();

        analyzer.record_request(ip, 100);
        assert!(analyzer.get_profile(ip).is_some());
        assert_eq!(analyzer.tracked_count(), 1);

        analyzer.reset(ip);
        assert!(analyzer.get_profile(ip).is_none());
        assert_eq!(analyzer.tracked_count(), 0);
        assert_eq!(analyzer.get_flag(ip), BehaviorFlag::Normal);
        assert_eq!(analyzer.get_anomaly_score(ip), 0.0);
    }

    #[test]
    fn test_get_flag_default_for_unknown_ip() {
        let analyzer = BehavioralAnalyzer::new(100, 5.0, 10.0);
        let ip: IpAddr = "1.2.3.4".parse().unwrap();

        assert_eq!(analyzer.get_flag(ip), BehaviorFlag::Normal);
        assert_eq!(analyzer.get_anomaly_score(ip), 0.0);
    }

    #[test]
    fn test_tracked_count() {
        let analyzer = BehavioralAnalyzer::new(100, 5.0, 10.0);
        let ip1: IpAddr = "10.0.0.10".parse().unwrap();
        let ip2: IpAddr = "10.0.0.11".parse().unwrap();

        assert_eq!(analyzer.tracked_count(), 0);

        analyzer.record_request(ip1, 100);
        assert_eq!(analyzer.tracked_count(), 1);

        analyzer.record_request(ip2, 200);
        assert_eq!(analyzer.tracked_count(), 2);

        analyzer.reset(ip1);
        assert_eq!(analyzer.tracked_count(), 1);
    }

    #[test]
    fn test_profile_cloning() {
        let analyzer = BehavioralAnalyzer::new(100, 5.0, 10.0);
        let ip: IpAddr = "10.0.0.12".parse().unwrap();

        analyzer.record_request(ip, 512);
        analyzer.record_request(ip, 1024);

        let profile = analyzer.get_profile(ip);
        assert!(profile.is_some());

        let profile = profile.unwrap();
        assert_eq!(profile.request_times.len(), 2);
        assert_eq!(profile.payload_sizes.len(), 2);
        assert_eq!(profile.flag, BehaviorFlag::Normal);
        assert_eq!(*profile.payload_sizes.front().unwrap(), 512);
        assert_eq!(*profile.payload_sizes.back().unwrap(), 1024);
    }
}
