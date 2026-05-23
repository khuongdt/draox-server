use ipnet::IpNet;
use server_config::model::IpListConfig;
use std::net::IpAddr;
use std::sync::RwLock;
use tracing::{debug, warn};

/// IP/CIDR blacklist and whitelist filter.
///
/// Supports both individual IP addresses and CIDR ranges for both
/// blacklisting and whitelisting. Whitelisted IPs bypass all other checks.
pub struct IpFilter {
    blacklisted_ips: RwLock<Vec<IpAddr>>,
    blacklisted_cidrs: RwLock<Vec<IpNet>>,
    whitelisted_ips: RwLock<Vec<IpAddr>>,
    whitelisted_cidrs: RwLock<Vec<IpNet>>,
}

impl IpFilter {
    /// Create a new IpFilter from blacklist and whitelist configs.
    pub fn new(blacklist: &IpListConfig, whitelist: &IpListConfig) -> Self {
        let filter = Self {
            blacklisted_ips: RwLock::new(Vec::new()),
            blacklisted_cidrs: RwLock::new(Vec::new()),
            whitelisted_ips: RwLock::new(Vec::new()),
            whitelisted_cidrs: RwLock::new(Vec::new()),
        };

        // Load blacklist
        for ip_str in &blacklist.ips {
            if let Err(e) = filter.add_blacklist(ip_str) {
                warn!("Failed to parse blacklist IP '{}': {}", ip_str, e);
            }
        }
        for cidr_str in &blacklist.cidrs {
            if let Err(e) = filter.add_blacklist(cidr_str) {
                warn!("Failed to parse blacklist CIDR '{}': {}", cidr_str, e);
            }
        }

        // Load whitelist
        for ip_str in &whitelist.ips {
            if let Err(e) = filter.add_whitelist(ip_str) {
                warn!("Failed to parse whitelist IP '{}': {}", ip_str, e);
            }
        }
        for cidr_str in &whitelist.cidrs {
            if let Err(e) = filter.add_whitelist(cidr_str) {
                warn!("Failed to parse whitelist CIDR '{}': {}", cidr_str, e);
            }
        }

        filter
    }

    /// Check if an IP address is blacklisted.
    pub fn is_blacklisted(&self, ip: IpAddr) -> bool {
        // Check individual IPs
        if let Ok(ips) = self.blacklisted_ips.read() {
            if ips.contains(&ip) {
                return true;
            }
        }

        // Check CIDR ranges
        if let Ok(cidrs) = self.blacklisted_cidrs.read() {
            for cidr in cidrs.iter() {
                if cidr.contains(&ip) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if an IP address is whitelisted.
    pub fn is_whitelisted(&self, ip: IpAddr) -> bool {
        // Check individual IPs
        if let Ok(ips) = self.whitelisted_ips.read() {
            if ips.contains(&ip) {
                return true;
            }
        }

        // Check CIDR ranges
        if let Ok(cidrs) = self.whitelisted_cidrs.read() {
            for cidr in cidrs.iter() {
                if cidr.contains(&ip) {
                    return true;
                }
            }
        }

        false
    }

    /// Add an IP address or CIDR range to the blacklist.
    pub fn add_blacklist(&self, ip_or_cidr: &str) -> Result<(), String> {
        // Try parsing as CIDR first
        if ip_or_cidr.contains('/') {
            let cidr: IpNet = ip_or_cidr
                .parse()
                .map_err(|e| format!("invalid CIDR: {e}"))?;
            let mut cidrs = self
                .blacklisted_cidrs
                .write()
                .map_err(|e| format!("lock poisoned: {e}"))?;
            if !cidrs.contains(&cidr) {
                cidrs.push(cidr);
                debug!("Added CIDR {} to blacklist", ip_or_cidr);
            }
        } else {
            let ip: IpAddr = ip_or_cidr
                .parse()
                .map_err(|e| format!("invalid IP: {e}"))?;
            let mut ips = self
                .blacklisted_ips
                .write()
                .map_err(|e| format!("lock poisoned: {e}"))?;
            if !ips.contains(&ip) {
                ips.push(ip);
                debug!("Added IP {} to blacklist", ip_or_cidr);
            }
        }
        Ok(())
    }

    /// Remove an IP address or CIDR range from the blacklist.
    pub fn remove_blacklist(&self, ip_or_cidr: &str) -> Result<(), String> {
        if ip_or_cidr.contains('/') {
            let cidr: IpNet = ip_or_cidr
                .parse()
                .map_err(|e| format!("invalid CIDR: {e}"))?;
            let mut cidrs = self
                .blacklisted_cidrs
                .write()
                .map_err(|e| format!("lock poisoned: {e}"))?;
            cidrs.retain(|c| c != &cidr);
            debug!("Removed CIDR {} from blacklist", ip_or_cidr);
        } else {
            let ip: IpAddr = ip_or_cidr
                .parse()
                .map_err(|e| format!("invalid IP: {e}"))?;
            let mut ips = self
                .blacklisted_ips
                .write()
                .map_err(|e| format!("lock poisoned: {e}"))?;
            ips.retain(|i| i != &ip);
            debug!("Removed IP {} from blacklist", ip_or_cidr);
        }
        Ok(())
    }

    /// Add an IP address or CIDR range to the whitelist.
    pub fn add_whitelist(&self, ip_or_cidr: &str) -> Result<(), String> {
        if ip_or_cidr.contains('/') {
            let cidr: IpNet = ip_or_cidr
                .parse()
                .map_err(|e| format!("invalid CIDR: {e}"))?;
            let mut cidrs = self
                .whitelisted_cidrs
                .write()
                .map_err(|e| format!("lock poisoned: {e}"))?;
            if !cidrs.contains(&cidr) {
                cidrs.push(cidr);
                debug!("Added CIDR {} to whitelist", ip_or_cidr);
            }
        } else {
            let ip: IpAddr = ip_or_cidr
                .parse()
                .map_err(|e| format!("invalid IP: {e}"))?;
            let mut ips = self
                .whitelisted_ips
                .write()
                .map_err(|e| format!("lock poisoned: {e}"))?;
            if !ips.contains(&ip) {
                ips.push(ip);
                debug!("Added IP {} to whitelist", ip_or_cidr);
            }
        }
        Ok(())
    }

    /// Remove an IP address or CIDR range from the whitelist.
    pub fn remove_whitelist(&self, ip_or_cidr: &str) -> Result<(), String> {
        if ip_or_cidr.contains('/') {
            let cidr: IpNet = ip_or_cidr
                .parse()
                .map_err(|e| format!("invalid CIDR: {e}"))?;
            let mut cidrs = self
                .whitelisted_cidrs
                .write()
                .map_err(|e| format!("lock poisoned: {e}"))?;
            cidrs.retain(|c| c != &cidr);
            debug!("Removed CIDR {} from whitelist", ip_or_cidr);
        } else {
            let ip: IpAddr = ip_or_cidr
                .parse()
                .map_err(|e| format!("invalid IP: {e}"))?;
            let mut ips = self
                .whitelisted_ips
                .write()
                .map_err(|e| format!("lock poisoned: {e}"))?;
            ips.retain(|i| i != &ip);
            debug!("Removed IP {} from whitelist", ip_or_cidr);
        }
        Ok(())
    }

    /// Number of blacklisted IPs and CIDR ranges.
    pub fn blacklist_count(&self) -> usize {
        let ips = self.blacklisted_ips.read().map(|v| v.len()).unwrap_or(0);
        let cidrs = self.blacklisted_cidrs.read().map(|v| v.len()).unwrap_or(0);
        ips + cidrs
    }

    /// Number of whitelisted IPs and CIDR ranges.
    pub fn whitelist_count(&self) -> usize {
        let ips = self.whitelisted_ips.read().map(|v| v.len()).unwrap_or(0);
        let cidrs = self.whitelisted_cidrs.read().map(|v| v.len()).unwrap_or(0);
        ips + cidrs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_config() -> IpListConfig {
        IpListConfig {
            ips: vec![],
            cidrs: vec![],
        }
    }

    #[test]
    fn test_blacklist_ip() {
        let blacklist = IpListConfig {
            ips: vec!["192.168.1.100".to_string()],
            cidrs: vec![],
        };
        let filter = IpFilter::new(&blacklist, &empty_config());

        let blocked: IpAddr = "192.168.1.100".parse().unwrap();
        let allowed: IpAddr = "192.168.1.101".parse().unwrap();

        assert!(filter.is_blacklisted(blocked));
        assert!(!filter.is_blacklisted(allowed));
    }

    #[test]
    fn test_blacklist_cidr() {
        let blacklist = IpListConfig {
            ips: vec![],
            cidrs: vec!["10.0.0.0/8".to_string()],
        };
        let filter = IpFilter::new(&blacklist, &empty_config());

        let blocked: IpAddr = "10.1.2.3".parse().unwrap();
        let allowed: IpAddr = "192.168.1.1".parse().unwrap();

        assert!(filter.is_blacklisted(blocked));
        assert!(!filter.is_blacklisted(allowed));
    }

    #[test]
    fn test_whitelist_ip() {
        let whitelist = IpListConfig {
            ips: vec!["127.0.0.1".to_string()],
            cidrs: vec![],
        };
        let filter = IpFilter::new(&empty_config(), &whitelist);

        let whitelisted: IpAddr = "127.0.0.1".parse().unwrap();
        let not_whitelisted: IpAddr = "192.168.1.1".parse().unwrap();

        assert!(filter.is_whitelisted(whitelisted));
        assert!(!filter.is_whitelisted(not_whitelisted));
    }

    #[test]
    fn test_add_remove_dynamic() {
        let filter = IpFilter::new(&empty_config(), &empty_config());
        let ip: IpAddr = "1.2.3.4".parse().unwrap();

        // Initially not blacklisted
        assert!(!filter.is_blacklisted(ip));

        // Add to blacklist
        filter.add_blacklist("1.2.3.4").unwrap();
        assert!(filter.is_blacklisted(ip));

        // Remove from blacklist
        filter.remove_blacklist("1.2.3.4").unwrap();
        assert!(!filter.is_blacklisted(ip));

        // Add CIDR to whitelist
        filter.add_whitelist("10.0.0.0/8").unwrap();
        let ip_in_range: IpAddr = "10.5.5.5".parse().unwrap();
        assert!(filter.is_whitelisted(ip_in_range));

        // Remove CIDR from whitelist
        filter.remove_whitelist("10.0.0.0/8").unwrap();
        assert!(!filter.is_whitelisted(ip_in_range));
    }
}
