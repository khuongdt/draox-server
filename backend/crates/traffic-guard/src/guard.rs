use crate::auth_failure::AuthFailureTracker;
use crate::ban_manager::BanManager;
use crate::circuit_breaker::CircuitBreaker;
use crate::concurrent_connections::ConcurrentConnectionLimiter;
use crate::ip_filter::IpFilter;
use crate::rate_limiter::RateLimiter;
use crate::reputation::ReputationTracker;
use crate::subnet_limiter::SubnetLimiter;
use crate::verdict::GuardVerdict;
use dashmap::DashMap;
use server_config::model::TrafficGuardConfig;
use server_core::event::{EventBus, ServerEvent};
use server_core::{ConnectionId, ConnectionInfo, Error, ShutdownReceiver};
use socket_server::handler::BoxFuture;
use socket_server::ConnectionHandler;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Main traffic guard component that orchestrates all sub-components.
///
/// Sits between the socket-server and connection-manager in the pipeline.
/// Implements `ConnectionHandler` and wraps a `next_handler`, acting as a
/// proxy that checks/blocks/allows connections before forwarding.
pub struct TrafficGuard {
    config: TrafficGuardConfig,
    ip_filter: IpFilter,
    rate_limiter: RateLimiter,
    ban_manager: Arc<BanManager>,
    reputation: Arc<ReputationTracker>,
    auth_failure: AuthFailureTracker,
    concurrent: ConcurrentConnectionLimiter,
    subnet_limiter: SubnetLimiter,
    circuit_breaker: CircuitBreaker,
    connection_ips: DashMap<ConnectionId, IpAddr>,
    next_handler: Arc<dyn ConnectionHandler>,
    event_bus: Arc<EventBus>,
}

impl TrafficGuard {
    /// Create a new TrafficGuard with all sub-components initialized from config.
    pub fn new(
        config: TrafficGuardConfig,
        next_handler: Arc<dyn ConnectionHandler>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        let ip_filter = IpFilter::new(&config.blacklist, &config.whitelist);
        let rate_limiter = RateLimiter::new(&config.rate_limiting);
        let ban_manager = Arc::new(BanManager::new(config.banning.clone()));
        let reputation = Arc::new(ReputationTracker::new(config.ip_reputation.clone()));
        let auth_failure = AuthFailureTracker::new(&config.banning, Arc::clone(&ban_manager));
        let concurrent = ConcurrentConnectionLimiter::new(&config.connection_limits);
        let subnet_limiter = SubnetLimiter::new(
            config.rate_limiting.default_requests_per_sec,
            config.rate_limiting.burst_size * 2,
        );
        let circuit_breaker = CircuitBreaker::new(
            config.banning.max_violations_before_ban * 10,
            5,
            config.banning.initial_ban_duration_secs * 1000,
        );

        info!("TrafficGuard initialized (enabled: {})", config.enabled);

        Self {
            config,
            ip_filter,
            rate_limiter,
            ban_manager,
            reputation,
            auth_failure,
            concurrent,
            subnet_limiter,
            circuit_breaker,
            connection_ips: DashMap::new(),
            next_handler,
            event_bus,
        }
    }

    /// Start background tasks (ban cleanup and reputation recovery).
    ///
    /// Requires two shutdown receivers — one for each background task.
    pub fn start_background_tasks(&self, shutdown1: ShutdownReceiver, shutdown2: ShutdownReceiver) {
        self.ban_manager.start_cleanup_task(shutdown1);
        self.reputation.start_recovery_task(shutdown2);
        debug!("TrafficGuard background tasks started");
    }

    /// Orchestrate all traffic guard checks for an incoming connection.
    ///
    /// Check order:
    /// 1. Whitelist -> Allow immediately
    /// 2. Blacklist -> Block
    /// 3. Ban check -> Block if banned
    /// 4. Reputation check -> Block if score too low
    /// 5. Circuit breaker -> Block if open
    /// 6. Concurrent connection limit -> Block if exceeded
    /// 7. Subnet rate limit -> Block if exceeded
    /// 8. Per-IP rate limit -> Block if exceeded (and record violation)
    /// 9. Allow
    pub fn check_connection(&self, addr: &SocketAddr) -> GuardVerdict {
        // If traffic guard is disabled, allow everything
        if !self.config.enabled {
            return GuardVerdict::Allow;
        }

        let ip = addr.ip();

        // 1. Whitelist bypasses all checks
        if self.ip_filter.is_whitelisted(ip) {
            debug!("Whitelisted IP {} — allowing", ip);
            return GuardVerdict::Allow;
        }

        // 2. Blacklist check
        if self.ip_filter.is_blacklisted(ip) {
            warn!("Blacklisted IP {} — blocking", ip);
            return GuardVerdict::Block(format!("IP {ip} is blacklisted"));
        }

        // 3. Ban check
        if let Some(ban) = self.ban_manager.is_banned(ip) {
            warn!(
                "Banned IP {} — blocking (expires: {}, reason: {})",
                ip, ban.expires_at, ban.reason
            );
            return GuardVerdict::Block(format!(
                "IP {ip} is banned until {} (reason: {})",
                ban.expires_at, ban.reason
            ));
        }

        // 4. Reputation check
        let reputation_verdict = self.reputation.check_reputation(ip);
        if let GuardVerdict::Block(reason) = &reputation_verdict {
            warn!("IP {} blocked by reputation: {}", ip, reason);
            return reputation_verdict;
        }

        // 5. Circuit breaker check
        if !self.circuit_breaker.allow_request() {
            warn!("Circuit breaker is open — blocking IP {}", ip);
            return GuardVerdict::Block("circuit breaker is open".to_string());
        }

        // 6. Concurrent connection limit check
        if !self.concurrent.try_add(ip) {
            warn!(
                "Concurrent connection limit exceeded for IP {} — blocking",
                ip
            );
            return GuardVerdict::Block(format!(
                "concurrent connection limit exceeded for {ip}"
            ));
        }

        // 7. Subnet rate limit check
        let subnet_verdict = self.subnet_limiter.check(ip);
        if let GuardVerdict::Block(_) = &subnet_verdict {
            warn!("IP {} blocked by subnet rate limit", ip);
            // Remove the concurrent connection we just added since we're blocking
            self.concurrent.remove(ip);
            return subnet_verdict;
        }

        // 8. Per-IP rate limit check
        let rate_verdict = self.rate_limiter.check(ip);
        if let GuardVerdict::Block(_) = &rate_verdict {
            warn!("IP {} rate limited — recording violation", ip);
            // Remove the concurrent connection we just added since we're blocking
            self.concurrent.remove(ip);
            // Record violation for rate limit exceeding
            self.reputation.penalize(ip);
            if let Some(ban_entry) = self.ban_manager.record_violation(ip) {
                self.event_bus.publish(ServerEvent::GuardIpBanned {
                    ip,
                    duration_secs: (ban_entry.expires_at - ban_entry.banned_at)
                        .num_seconds() as u64,
                });
            }
            return rate_verdict;
        }

        // 9. All checks passed
        GuardVerdict::Allow
    }

    /// Get a reference to the ban manager.
    pub fn ban_manager(&self) -> &Arc<BanManager> {
        &self.ban_manager
    }

    /// Get a reference to the IP filter.
    pub fn ip_filter(&self) -> &IpFilter {
        &self.ip_filter
    }

    /// Get a reference to the reputation tracker.
    pub fn reputation(&self) -> &Arc<ReputationTracker> {
        &self.reputation
    }

    /// Get a reference to the auth failure tracker.
    pub fn auth_failure(&self) -> &AuthFailureTracker {
        &self.auth_failure
    }

    /// Get a reference to the concurrent connection limiter.
    pub fn concurrent(&self) -> &ConcurrentConnectionLimiter {
        &self.concurrent
    }

    /// Get a reference to the subnet limiter.
    pub fn subnet_limiter(&self) -> &SubnetLimiter {
        &self.subnet_limiter
    }

    /// Get a reference to the circuit breaker.
    pub fn circuit_breaker(&self) -> &CircuitBreaker {
        &self.circuit_breaker
    }
}

impl ConnectionHandler for TrafficGuard {
    fn on_connect<'a>(&'a self, info: &'a ConnectionInfo) -> BoxFuture<'a, server_core::Result<()>> {
        Box::pin(async move {
            let verdict = self.check_connection(&info.remote_addr);

            match verdict {
                GuardVerdict::Allow => {
                    // Track connection IP for cleanup on disconnect
                    self.connection_ips
                        .insert(info.id.clone(), info.remote_addr.ip());

                    // Record success for the circuit breaker
                    self.circuit_breaker.record_success();

                    // Forward to next handler
                    self.next_handler.on_connect(info).await
                }
                GuardVerdict::Block(reason) => {
                    // Record failure for the circuit breaker
                    self.circuit_breaker.record_failure();

                    // Publish block event
                    self.event_bus.publish(ServerEvent::GuardConnectionBlocked {
                        remote_addr: info.remote_addr.to_string(),
                        reason: reason.clone(),
                    });

                    Err(Error::ConnectionRefused {
                        addr: info.remote_addr.to_string(),
                        reason,
                    })
                }
                GuardVerdict::Throttle => {
                    // Track connection IP for cleanup on disconnect
                    self.connection_ips
                        .insert(info.id.clone(), info.remote_addr.ip());

                    // For now, throttle means allow but could add delay in future
                    self.next_handler.on_connect(info).await
                }
            }
        })
    }

    fn on_data<'a>(&'a self, conn_id: &'a ConnectionId, data: &'a [u8]) -> BoxFuture<'a, ()> {
        self.next_handler.on_data(conn_id, data)
    }

    fn on_text<'a>(&'a self, conn_id: &'a ConnectionId, text: &'a str) -> BoxFuture<'a, ()> {
        self.next_handler.on_text(conn_id, text)
    }

    fn on_disconnect<'a>(
        &'a self,
        conn_id: &'a ConnectionId,
        reason: &'a str,
    ) -> BoxFuture<'a, ()> {
        // Decrement concurrent connection count for the disconnecting IP
        if let Some((_, ip)) = self.connection_ips.remove(conn_id) {
            self.concurrent.remove(ip);
            debug!("Connection {} disconnected, decremented count for IP {}", conn_id, ip);
        }

        self.next_handler.on_disconnect(conn_id, reason)
    }

    fn on_error<'a>(&'a self, conn_id: &'a ConnectionId, error: &'a Error) -> BoxFuture<'a, ()> {
        self.next_handler.on_error(conn_id, error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use server_core::Protocol;

    /// No-op handler for testing. Accepts all connections, ignores all data.
    struct NoopHandler;

    impl ConnectionHandler for NoopHandler {
        fn on_connect<'a>(
            &'a self,
            _info: &'a ConnectionInfo,
        ) -> BoxFuture<'a, server_core::Result<()>> {
            Box::pin(async { Ok(()) })
        }

        fn on_data<'a>(
            &'a self,
            _conn_id: &'a ConnectionId,
            _data: &'a [u8],
        ) -> BoxFuture<'a, ()> {
            Box::pin(async {})
        }

        fn on_disconnect<'a>(
            &'a self,
            _conn_id: &'a ConnectionId,
            _reason: &'a str,
        ) -> BoxFuture<'a, ()> {
            Box::pin(async {})
        }

        fn on_error<'a>(
            &'a self,
            _conn_id: &'a ConnectionId,
            _error: &'a Error,
        ) -> BoxFuture<'a, ()> {
            Box::pin(async {})
        }
    }

    fn make_guard(config: TrafficGuardConfig) -> TrafficGuard {
        let handler = Arc::new(NoopHandler);
        let event_bus = Arc::new(EventBus::new(16));
        TrafficGuard::new(config, handler, event_bus)
    }

    fn make_conn_info(addr: &str) -> ConnectionInfo {
        ConnectionInfo::new(
            ConnectionId::new(),
            Protocol::Tcp,
            addr.parse().unwrap(),
        )
    }

    #[tokio::test]
    async fn test_allow_normal_connection() {
        let config = TrafficGuardConfig::default();
        let guard = make_guard(config);

        let info = make_conn_info("203.0.113.50:12345");
        let result = guard.on_connect(&info).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_block_blacklisted() {
        let mut config = TrafficGuardConfig::default();
        config.blacklist.ips.push("198.51.100.1".to_string());

        let guard = make_guard(config);

        let info = make_conn_info("198.51.100.1:9999");
        let result = guard.on_connect(&info).await;
        assert!(result.is_err());

        if let Err(Error::ConnectionRefused { reason, .. }) = result {
            assert!(reason.contains("blacklisted"));
        } else {
            panic!("Expected ConnectionRefused error");
        }
    }

    #[tokio::test]
    async fn test_whitelist_bypasses_all() {
        let mut config = TrafficGuardConfig::default();
        // Blacklist AND whitelist the same IP — whitelist should win
        config.blacklist.ips.push("10.0.0.1".to_string());
        config.whitelist.ips.push("10.0.0.1".to_string());

        let guard = make_guard(config);

        let info = make_conn_info("10.0.0.1:8080");
        let result = guard.on_connect(&info).await;
        assert!(result.is_ok(), "Whitelisted IP should bypass blacklist");
    }
}
