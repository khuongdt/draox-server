use chrono::{NaiveDate, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use server_core::ClientId;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::debug;

use crate::plans::PlanTier;

// ────────────────────────────────────────────────────────
// Per-client usage counters (thread-safe via atomics)
// ────────────────────────────────────────────────────────

pub struct ClientUsage {
    pub requests: AtomicU64,
    pub bandwidth_bytes: AtomicU64,
    pub date: NaiveDate,
}

impl ClientUsage {
    fn new(date: NaiveDate) -> Self {
        Self {
            requests: AtomicU64::new(0),
            bandwidth_bytes: AtomicU64::new(0),
            date,
        }
    }
}

// ────────────────────────────────────────────────────────
// Serializable snapshot of usage
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    pub client_id: ClientId,
    pub plan: PlanTier,
    pub requests: u64,
    pub bandwidth_bytes: u64,
    pub date: NaiveDate,
}

// ────────────────────────────────────────────────────────
// Usage tracker
// ────────────────────────────────────────────────────────

pub struct UsageTracker {
    usage: DashMap<ClientId, ClientUsage>,
    plans: DashMap<ClientId, PlanTier>,
}

impl UsageTracker {
    /// Creates a new empty usage tracker.
    pub fn new() -> Self {
        Self {
            usage: DashMap::new(),
            plans: DashMap::new(),
        }
    }

    /// Records a single request for the given client.
    ///
    /// If the date has changed since the last recording, counters are
    /// automatically reset for the new day.
    pub fn record_request(&self, client_id: &ClientId) {
        let today = Utc::now().date_naive();

        // Use entry API to create-or-access without requiring Clone on ClientUsage
        let entry = self.usage.entry(client_id.clone());
        let usage_ref = entry.or_insert_with(|| ClientUsage::new(today));

        // If the date rolled over, reset counters
        if usage_ref.date != today {
            usage_ref.requests.store(0, Ordering::Relaxed);
            usage_ref.bandwidth_bytes.store(0, Ordering::Relaxed);
            // Safety: we hold a mutable-equivalent ref through the DashMap entry
            // We need to use unsafe or a different approach. Since DashMap gives
            // us a RefMut, we can't mutate non-atomic fields directly.
            // Workaround: drop and re-insert.
            drop(usage_ref);
            self.usage.insert(client_id.clone(), ClientUsage::new(today));
            self.usage
                .get(client_id)
                .unwrap()
                .requests
                .fetch_add(1, Ordering::Relaxed);
        } else {
            usage_ref.requests.fetch_add(1, Ordering::Relaxed);
        }

        debug!(client_id = %client_id, "recorded request");
    }

    /// Records bandwidth usage (in bytes) for the given client.
    ///
    /// If the date has changed since the last recording, counters are
    /// automatically reset for the new day.
    pub fn record_bandwidth(&self, client_id: &ClientId, bytes: u64) {
        let today = Utc::now().date_naive();

        let entry = self.usage.entry(client_id.clone());
        let usage_ref = entry.or_insert_with(|| ClientUsage::new(today));

        if usage_ref.date != today {
            drop(usage_ref);
            self.usage.insert(client_id.clone(), ClientUsage::new(today));
            self.usage
                .get(client_id)
                .unwrap()
                .bandwidth_bytes
                .fetch_add(bytes, Ordering::Relaxed);
        } else {
            usage_ref.bandwidth_bytes.fetch_add(bytes, Ordering::Relaxed);
        }

        debug!(client_id = %client_id, bytes, "recorded bandwidth");
    }

    /// Returns a snapshot of current usage for the given client.
    pub fn get_usage(&self, client_id: &ClientId) -> UsageSummary {
        let today = Utc::now().date_naive();
        let plan = self.get_plan(client_id);

        match self.usage.get(client_id) {
            Some(usage) => {
                // If the stored date differs from today, report zero usage
                if usage.date != today {
                    UsageSummary {
                        client_id: client_id.clone(),
                        plan,
                        requests: 0,
                        bandwidth_bytes: 0,
                        date: today,
                    }
                } else {
                    UsageSummary {
                        client_id: client_id.clone(),
                        plan,
                        requests: usage.requests.load(Ordering::Relaxed),
                        bandwidth_bytes: usage.bandwidth_bytes.load(Ordering::Relaxed),
                        date: usage.date,
                    }
                }
            }
            None => UsageSummary {
                client_id: client_id.clone(),
                plan,
                requests: 0,
                bandwidth_bytes: 0,
                date: today,
            },
        }
    }

    /// Sets the plan tier for a client.
    pub fn set_plan(&self, client_id: &ClientId, tier: PlanTier) {
        self.plans.insert(client_id.clone(), tier);
        debug!(client_id = %client_id, tier = %tier, "plan updated");
    }

    /// Returns the plan tier for a client (defaults to Free).
    pub fn get_plan(&self, client_id: &ClientId) -> PlanTier {
        self.plans
            .get(client_id)
            .map(|v| *v)
            .unwrap_or(PlanTier::Free)
    }

    /// Resets daily counters for the given client.
    pub fn reset_daily(&self, client_id: &ClientId) {
        let today = Utc::now().date_naive();
        self.usage.insert(client_id.clone(), ClientUsage::new(today));
        debug!(client_id = %client_id, "daily usage reset");
    }
}

impl Default for UsageTracker {
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
    fn test_record_request() {
        let tracker = UsageTracker::new();
        let client = ClientId::new();

        tracker.record_request(&client);
        tracker.record_request(&client);
        tracker.record_request(&client);

        let summary = tracker.get_usage(&client);
        assert_eq!(summary.requests, 3);
        assert_eq!(summary.bandwidth_bytes, 0);
    }

    #[test]
    fn test_record_bandwidth() {
        let tracker = UsageTracker::new();
        let client = ClientId::new();

        tracker.record_bandwidth(&client, 1024);
        tracker.record_bandwidth(&client, 2048);

        let summary = tracker.get_usage(&client);
        assert_eq!(summary.bandwidth_bytes, 3072);
        assert_eq!(summary.requests, 0);
    }

    #[test]
    fn test_set_get_plan() {
        let tracker = UsageTracker::new();
        let client = ClientId::new();

        // Default is Free
        assert_eq!(tracker.get_plan(&client), PlanTier::Free);

        tracker.set_plan(&client, PlanTier::Pro);
        assert_eq!(tracker.get_plan(&client), PlanTier::Pro);

        tracker.set_plan(&client, PlanTier::Enterprise);
        assert_eq!(tracker.get_plan(&client), PlanTier::Enterprise);

        // Usage summary reflects the plan
        let summary = tracker.get_usage(&client);
        assert_eq!(summary.plan, PlanTier::Enterprise);
    }

    #[test]
    fn test_reset_daily() {
        let tracker = UsageTracker::new();
        let client = ClientId::new();

        tracker.record_request(&client);
        tracker.record_request(&client);
        tracker.record_bandwidth(&client, 5000);

        let before = tracker.get_usage(&client);
        assert_eq!(before.requests, 2);
        assert_eq!(before.bandwidth_bytes, 5000);

        tracker.reset_daily(&client);

        let after = tracker.get_usage(&client);
        assert_eq!(after.requests, 0);
        assert_eq!(after.bandwidth_bytes, 0);
    }

    #[test]
    fn test_usage_summary_default_for_unknown_client() {
        let tracker = UsageTracker::new();
        let unknown = ClientId::new();

        let summary = tracker.get_usage(&unknown);
        assert_eq!(summary.requests, 0);
        assert_eq!(summary.bandwidth_bytes, 0);
        assert_eq!(summary.plan, PlanTier::Free);
    }
}
