//! # Billing
//!
//! Usage billing and subscription plan management for Draox Server.
//!
//! Provides in-memory usage tracking (requests and bandwidth) per client,
//! plan tier definitions (Free / Pro / Enterprise), and quota enforcement.

pub mod enforcement;
pub mod plans;
pub mod usage;

pub use enforcement::{QuotaEnforcer, QuotaStatus};
pub use plans::{Plan, PlanTier};
pub use usage::{UsageSummary, UsageTracker};
