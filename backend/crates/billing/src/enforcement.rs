use serde::{Deserialize, Serialize};

use crate::plans::Plan;
use crate::usage::UsageSummary;

// ────────────────────────────────────────────────────────
// Quota status
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaStatus {
    Ok,
    Warning { usage_percent: u32 },
    Exceeded { resource: String },
}

impl QuotaStatus {
    /// Returns `true` if the status indicates the quota has been exceeded.
    pub fn is_exceeded(&self) -> bool {
        matches!(self, QuotaStatus::Exceeded { .. })
    }

    /// Returns `true` if the status is a warning (usage > 80%).
    pub fn is_warning(&self) -> bool {
        matches!(self, QuotaStatus::Warning { .. })
    }

    /// Returns the worse of two statuses.
    /// Exceeded > Warning > Ok. Between two Warnings, the higher percentage wins.
    fn worst(self, other: QuotaStatus) -> QuotaStatus {
        match (&self, &other) {
            (QuotaStatus::Exceeded { .. }, _) => self,
            (_, QuotaStatus::Exceeded { .. }) => other,
            (QuotaStatus::Warning { usage_percent: a }, QuotaStatus::Warning { usage_percent: b }) => {
                if a >= b { self } else { other }
            }
            (QuotaStatus::Warning { .. }, _) => self,
            (_, QuotaStatus::Warning { .. }) => other,
            _ => QuotaStatus::Ok,
        }
    }
}

// ────────────────────────────────────────────────────────
// Quota enforcer
// ────────────────────────────────────────────────────────

pub struct QuotaEnforcer;

impl QuotaEnforcer {
    /// Checks whether the client has exceeded their daily request quota.
    pub fn check_request(usage: &UsageSummary, plan: &Plan) -> QuotaStatus {
        if usage.requests >= plan.max_requests_per_day {
            return QuotaStatus::Exceeded {
                resource: "requests".to_string(),
            };
        }

        let percent = Self::usage_percent(usage.requests, plan.max_requests_per_day);
        if percent > 80 {
            return QuotaStatus::Warning {
                usage_percent: percent,
            };
        }

        QuotaStatus::Ok
    }

    /// Checks whether the client has exceeded their daily bandwidth quota.
    pub fn check_bandwidth(usage: &UsageSummary, plan: &Plan) -> QuotaStatus {
        if usage.bandwidth_bytes >= plan.max_bandwidth_bytes_per_day {
            return QuotaStatus::Exceeded {
                resource: "bandwidth".to_string(),
            };
        }

        let percent = Self::usage_percent(usage.bandwidth_bytes, plan.max_bandwidth_bytes_per_day);
        if percent > 80 {
            return QuotaStatus::Warning {
                usage_percent: percent,
            };
        }

        QuotaStatus::Ok
    }

    /// Checks all quotas and returns the worst status.
    pub fn check_all(usage: &UsageSummary, plan: &Plan) -> QuotaStatus {
        let request_status = Self::check_request(usage, plan);
        let bandwidth_status = Self::check_bandwidth(usage, plan);
        request_status.worst(bandwidth_status)
    }

    /// Calculates usage percentage, capped at 100 to avoid overflow issues
    /// with u64::MAX limits on enterprise plans.
    fn usage_percent(current: u64, max: u64) -> u32 {
        if max == 0 {
            return 100;
        }
        // Use u128 to avoid overflow for large values
        let pct = (current as u128 * 100) / (max as u128);
        pct.min(100) as u32
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plans::{Plan, PlanTier};
    use chrono::Utc;
    use server_core::ClientId;

    fn make_summary(requests: u64, bandwidth_bytes: u64) -> UsageSummary {
        UsageSummary {
            client_id: ClientId::new(),
            plan: PlanTier::Free,
            requests,
            bandwidth_bytes,
            date: Utc::now().date_naive(),
        }
    }

    #[test]
    fn test_quota_ok() {
        let plan = Plan::free(); // 10,000 requests/day, 1 GB bandwidth
        let usage = make_summary(1_000, 100_000_000); // 10% requests, ~9% bandwidth

        assert_eq!(QuotaEnforcer::check_request(&usage, &plan), QuotaStatus::Ok);
        assert_eq!(QuotaEnforcer::check_bandwidth(&usage, &plan), QuotaStatus::Ok);
        assert_eq!(QuotaEnforcer::check_all(&usage, &plan), QuotaStatus::Ok);
    }

    #[test]
    fn test_quota_warning() {
        let plan = Plan::free(); // 10,000 requests/day
        let usage = make_summary(8_500, 0); // 85% of request limit

        let status = QuotaEnforcer::check_request(&usage, &plan);
        assert_eq!(
            status,
            QuotaStatus::Warning { usage_percent: 85 }
        );
        assert!(status.is_warning());
    }

    #[test]
    fn test_quota_exceeded() {
        let plan = Plan::free(); // 10,000 requests/day
        let usage = make_summary(10_000, 0); // exactly at limit

        let status = QuotaEnforcer::check_request(&usage, &plan);
        assert_eq!(
            status,
            QuotaStatus::Exceeded {
                resource: "requests".to_string()
            }
        );
        assert!(status.is_exceeded());
    }

    #[test]
    fn test_quota_bandwidth_exceeded() {
        let plan = Plan::free(); // 1 GB bandwidth
        let usage = make_summary(0, 2_000_000_000); // 2 GB, over limit

        let status = QuotaEnforcer::check_bandwidth(&usage, &plan);
        assert_eq!(
            status,
            QuotaStatus::Exceeded {
                resource: "bandwidth".to_string()
            }
        );
    }

    #[test]
    fn test_check_all_returns_worst() {
        let plan = Plan::free();
        // Requests exceeded, bandwidth ok
        let usage = make_summary(10_000, 0);
        let status = QuotaEnforcer::check_all(&usage, &plan);
        assert!(status.is_exceeded());

        // Requests ok, bandwidth warning (85% of 1 GB)
        let usage2 = make_summary(100, 912_680_550); // ~85%
        let status2 = QuotaEnforcer::check_all(&usage2, &plan);
        assert!(status2.is_warning());
    }

    #[test]
    fn test_enterprise_plan_never_exceeds() {
        let plan = Plan::enterprise(); // u64::MAX limits
        let usage = make_summary(999_999_999, 999_999_999_999);
        assert_eq!(QuotaEnforcer::check_all(&usage, &plan), QuotaStatus::Ok);
    }

    #[test]
    fn test_quota_status_serialization() {
        let status = QuotaStatus::Warning { usage_percent: 85 };
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: QuotaStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, status);
    }
}
