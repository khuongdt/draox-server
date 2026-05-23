//! Traffic Guard — Anti-spam, DDoS protection, rate limiting, IP reputation.
//!
//! This crate sits between `socket-server` and `connection-manager` in the
//! Draox Server pipeline. It implements the `ConnectionHandler` trait and
//! wraps a `next_handler`, acting as a proxy that checks, blocks, or allows
//! connections before forwarding them to the next handler.
//!
//! # Components
//!
//! - **IpFilter** — IP/CIDR blacklist and whitelist
//! - **RateLimiter** — Per-IP token bucket rate limiting (via governor)
//! - **BanManager** — Automatic banning with escalating durations
//! - **ReputationTracker** — IP reputation scoring with recovery
//! - **AuthFailureTracker** — Per-IP auth failure tracking with auto-ban
//! - **ConcurrentConnectionLimiter** — Per-IP active connection limiting
//! - **SubnetLimiter** — CIDR/subnet-level rate limiting
//! - **CircuitBreaker** — Circuit breaker pattern for service protection
//! - **ProtocolGuard** — Per-protocol rate limiting (HTTP, WS, UDP)
//! - **SlowlorisDetector** — Slowloris attack detection via data-rate monitoring
//! - **BehavioralAnalyzer** — Behavioral analysis with anomaly scoring
//! - **TrafficGuard** — Main orchestrator implementing `ConnectionHandler`

pub mod adaptive;
pub mod auth_failure;
pub mod ban_manager;
pub mod behavioral;
pub mod circuit_breaker;
pub mod concurrent_connections;
pub mod guard;
pub mod guard_metrics;
pub mod ip_filter;
pub mod protocol_guards;
pub mod rate_limiter;
pub mod reputation;
pub mod subnet_limiter;
pub mod syn_tracker;
pub mod verdict;

pub use adaptive::{AdaptiveConfig, AdaptiveSnapshot, AdaptiveThrottle, SystemLoad, ThrottleState};
pub use auth_failure::AuthFailureTracker;
pub use ban_manager::{BanEntry, BanManager};
pub use behavioral::{BehaviorFlag, BehaviorProfile, BehavioralAnalyzer};
pub use circuit_breaker::{CircuitBreaker, CircuitState};
pub use concurrent_connections::ConcurrentConnectionLimiter;
pub use guard::TrafficGuard;
pub use guard_metrics::{GuardMetrics, GuardMetricsSnapshot};
pub use ip_filter::IpFilter;
pub use protocol_guards::{ProtocolGuard, SlowlorisDetector};
pub use rate_limiter::RateLimiter;
pub use reputation::{ReputationEntry, ReputationTracker};
pub use subnet_limiter::SubnetLimiter;
pub use syn_tracker::SynTracker;
pub use verdict::GuardVerdict;
