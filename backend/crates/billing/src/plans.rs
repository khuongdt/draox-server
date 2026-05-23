use serde::{Deserialize, Serialize};

// ────────────────────────────────────────────────────────
// Plan tier
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanTier {
    Free,
    Pro,
    Enterprise,
}

impl Default for PlanTier {
    fn default() -> Self {
        Self::Free
    }
}

impl std::fmt::Display for PlanTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanTier::Free => write!(f, "Free"),
            PlanTier::Pro => write!(f, "Pro"),
            PlanTier::Enterprise => write!(f, "Enterprise"),
        }
    }
}

// ────────────────────────────────────────────────────────
// Plan definition
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub tier: PlanTier,
    pub name: String,
    pub max_requests_per_day: u64,
    pub max_connections: u32,
    pub max_bandwidth_bytes_per_day: u64,
    pub price_cents_per_month: u32,
}

impl Plan {
    /// Free tier — generous limits for small projects.
    pub fn free() -> Self {
        Self {
            tier: PlanTier::Free,
            name: "Free".to_string(),
            max_requests_per_day: 10_000,
            max_connections: 100,
            max_bandwidth_bytes_per_day: 1_073_741_824, // 1 GB
            price_cents_per_month: 0,
        }
    }

    /// Professional tier — higher limits for production workloads.
    pub fn pro() -> Self {
        Self {
            tier: PlanTier::Pro,
            name: "Professional".to_string(),
            max_requests_per_day: 1_000_000,
            max_connections: 10_000,
            max_bandwidth_bytes_per_day: 107_374_182_400, // 100 GB
            price_cents_per_month: 4900, // $49
        }
    }

    /// Enterprise tier — effectively unlimited.
    pub fn enterprise() -> Self {
        Self {
            tier: PlanTier::Enterprise,
            name: "Enterprise".to_string(),
            max_requests_per_day: u64::MAX,
            max_connections: u32::MAX,
            max_bandwidth_bytes_per_day: u64::MAX,
            price_cents_per_month: 29900, // $299
        }
    }

    /// Returns the default plan for the given tier.
    pub fn for_tier(tier: PlanTier) -> Self {
        match tier {
            PlanTier::Free => Self::free(),
            PlanTier::Pro => Self::pro(),
            PlanTier::Enterprise => Self::enterprise(),
        }
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_tiers() {
        let free = Plan::free();
        assert_eq!(free.tier, PlanTier::Free);
        assert_eq!(free.max_requests_per_day, 10_000);
        assert_eq!(free.max_connections, 100);
        assert_eq!(free.max_bandwidth_bytes_per_day, 1_073_741_824);
        assert_eq!(free.price_cents_per_month, 0);

        let pro = Plan::pro();
        assert_eq!(pro.tier, PlanTier::Pro);
        assert_eq!(pro.max_requests_per_day, 1_000_000);
        assert_eq!(pro.max_connections, 10_000);
        assert_eq!(pro.max_bandwidth_bytes_per_day, 107_374_182_400);
        assert_eq!(pro.price_cents_per_month, 4900);

        let enterprise = Plan::enterprise();
        assert_eq!(enterprise.tier, PlanTier::Enterprise);
        assert_eq!(enterprise.max_requests_per_day, u64::MAX);
        assert_eq!(enterprise.max_connections, u32::MAX);
        assert_eq!(enterprise.max_bandwidth_bytes_per_day, u64::MAX);
        assert_eq!(enterprise.price_cents_per_month, 29900);
    }

    #[test]
    fn test_plan_for_tier() {
        let free = Plan::for_tier(PlanTier::Free);
        assert_eq!(free.tier, PlanTier::Free);
        assert_eq!(free.name, "Free");

        let pro = Plan::for_tier(PlanTier::Pro);
        assert_eq!(pro.tier, PlanTier::Pro);
        assert_eq!(pro.name, "Professional");

        let enterprise = Plan::for_tier(PlanTier::Enterprise);
        assert_eq!(enterprise.tier, PlanTier::Enterprise);
        assert_eq!(enterprise.name, "Enterprise");
    }

    #[test]
    fn test_plan_tier_default() {
        assert_eq!(PlanTier::default(), PlanTier::Free);
    }

    #[test]
    fn test_plan_tier_serialization() {
        let tier = PlanTier::Pro;
        let json = serde_json::to_string(&tier).unwrap();
        assert_eq!(json, "\"pro\"");
        let deserialized: PlanTier = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, tier);
    }
}
