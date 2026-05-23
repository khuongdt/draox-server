use crate::verdict::GuardVerdict;
use dashmap::DashMap;
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter as GovernorRateLimiter};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::num::NonZeroU32;
use std::sync::Arc;
use tracing::debug;

/// Type alias for the governor rate limiter used per subnet.
type SubnetRateLimiter = GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Default subnet rate limit: requests per second.
const DEFAULT_RATE_PER_SEC: u32 = 1000;

/// Default subnet burst size.
const DEFAULT_BURST_SIZE: u32 = 2000;

/// Subnet-level rate limiter that aggregates IPs into /24 (IPv4) or /48 (IPv6)
/// prefixes and rate-limits at the subnet level.
///
/// This prevents coordinated attacks from multiple IPs within the same subnet
/// from bypassing per-IP rate limits.
pub struct SubnetLimiter {
    limiters: DashMap<IpAddr, Arc<SubnetRateLimiter>>,
    rate_per_sec: NonZeroU32,
    burst_size: NonZeroU32,
}

impl SubnetLimiter {
    /// Create a new subnet limiter with the given rate and burst parameters.
    ///
    /// If `rate_per_sec` or `burst_size` is 0, defaults are used
    /// (1000 req/s, 2000 burst).
    pub fn new(rate_per_sec: u32, burst_size: u32) -> Self {
        let rate_per_sec =
            NonZeroU32::new(rate_per_sec).unwrap_or(NonZeroU32::new(DEFAULT_RATE_PER_SEC).unwrap());
        let burst_size =
            NonZeroU32::new(burst_size).unwrap_or(NonZeroU32::new(DEFAULT_BURST_SIZE).unwrap());

        Self {
            limiters: DashMap::new(),
            rate_per_sec,
            burst_size,
        }
    }

    /// Check whether a request from the given IP is allowed under subnet rate limits.
    ///
    /// The IP is mapped to its subnet key (/24 for IPv4, /48 for IPv6),
    /// and the shared subnet rate limiter is consulted.
    pub fn check(&self, ip: IpAddr) -> GuardVerdict {
        let key = Self::subnet_key(ip);

        let limiter = self
            .limiters
            .entry(key)
            .or_insert_with(|| {
                let quota =
                    Quota::per_second(self.rate_per_sec).allow_burst(self.burst_size);
                Arc::new(GovernorRateLimiter::direct(quota))
            })
            .clone();

        match limiter.check() {
            Ok(()) => GuardVerdict::Allow,
            Err(_) => {
                debug!("Subnet rate limit exceeded for {} (subnet: {})", ip, key);
                GuardVerdict::Block(format!("subnet rate limit exceeded for {key}"))
            }
        }
    }

    /// Extract the subnet key from an IP address.
    ///
    /// - IPv4: /24 prefix (zeroes last octet)
    /// - IPv6: /48 prefix (zeroes last 5 segments)
    fn subnet_key(ip: IpAddr) -> IpAddr {
        match ip {
            IpAddr::V4(v4) => {
                let octets = v4.octets();
                IpAddr::V4(Ipv4Addr::new(octets[0], octets[1], octets[2], 0))
            }
            IpAddr::V6(v6) => {
                let segments = v6.segments();
                IpAddr::V6(Ipv6Addr::new(
                    segments[0],
                    segments[1],
                    segments[2],
                    0,
                    0,
                    0,
                    0,
                    0,
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subnet_key_ipv4() {
        let ip: IpAddr = "192.168.1.100".parse().unwrap();
        let key = SubnetLimiter::subnet_key(ip);
        assert_eq!(key, "192.168.1.0".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_subnet_key_ipv4_different_hosts_same_subnet() {
        let ip1: IpAddr = "10.0.1.50".parse().unwrap();
        let ip2: IpAddr = "10.0.1.200".parse().unwrap();
        assert_eq!(
            SubnetLimiter::subnet_key(ip1),
            SubnetLimiter::subnet_key(ip2)
        );
    }

    #[test]
    fn test_subnet_key_ipv4_different_subnets() {
        let ip1: IpAddr = "10.0.1.50".parse().unwrap();
        let ip2: IpAddr = "10.0.2.50".parse().unwrap();
        assert_ne!(
            SubnetLimiter::subnet_key(ip1),
            SubnetLimiter::subnet_key(ip2)
        );
    }

    #[test]
    fn test_subnet_key_ipv6() {
        let ip: IpAddr = "2001:db8:abcd:1234:5678:9abc:def0:1234".parse().unwrap();
        let key = SubnetLimiter::subnet_key(ip);
        assert_eq!(key, "2001:db8:abcd::".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_same_subnet_shares_limit() {
        // Use a very small burst (1) so we can exhaust it with 2 IPs in the same /24
        let limiter = SubnetLimiter::new(1, 1);

        let ip1: IpAddr = "192.168.1.10".parse().unwrap();
        let ip2: IpAddr = "192.168.1.20".parse().unwrap();

        // First request from the subnet — should be allowed
        let v1 = limiter.check(ip1);
        assert_eq!(v1, GuardVerdict::Allow);

        // Second request from a different IP in the same /24 — should be blocked
        let v2 = limiter.check(ip2);
        assert!(matches!(v2, GuardVerdict::Block(_)));
    }

    #[test]
    fn test_different_subnets_independent() {
        let limiter = SubnetLimiter::new(1, 1);

        let ip1: IpAddr = "192.168.1.10".parse().unwrap();
        let ip2: IpAddr = "192.168.2.10".parse().unwrap();

        let v1 = limiter.check(ip1);
        assert_eq!(v1, GuardVerdict::Allow);

        // Different /24 subnet — should also be allowed
        let v2 = limiter.check(ip2);
        assert_eq!(v2, GuardVerdict::Allow);
    }

    #[test]
    fn test_ipv6_subnet_sharing() {
        let limiter = SubnetLimiter::new(1, 1);

        let ip1: IpAddr = "2001:db8:1::1".parse().unwrap();
        let ip2: IpAddr = "2001:db8:1::ffff".parse().unwrap();

        // Same /48 prefix
        let v1 = limiter.check(ip1);
        assert_eq!(v1, GuardVerdict::Allow);

        let v2 = limiter.check(ip2);
        assert!(matches!(v2, GuardVerdict::Block(_)));
    }

    #[test]
    fn test_default_values_on_zero() {
        // Passing 0 should use defaults and not panic
        let limiter = SubnetLimiter::new(0, 0);
        let ip: IpAddr = "1.2.3.4".parse().unwrap();
        let verdict = limiter.check(ip);
        assert_eq!(verdict, GuardVerdict::Allow);
    }
}
