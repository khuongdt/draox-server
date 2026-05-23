use std::collections::HashMap;
use crate::node::NodeInfo;

/// Sticky session routing strategies for the load balancer.
#[derive(Debug, Clone)]
pub enum StickyStrategy {
    /// Route by hashed client IP.
    IpHash,
    /// Route by a cookie value.
    Cookie { cookie_name: String },
    /// Least-connections routing (select node with fewest connections).
    LeastConnections,
}

/// Select the target node for a new connection based on the configured strategy.
pub fn select_node<'a>(
    nodes: &'a [NodeInfo],
    strategy: &StickyStrategy,
    ip: Option<&str>,
    cookies: Option<&HashMap<String, String>>,
) -> Option<&'a NodeInfo> {
    let alive: Vec<&NodeInfo> = nodes.iter().filter(|n| n.is_alive(30)).collect();
    if alive.is_empty() {
        return None;
    }

    match strategy {
        StickyStrategy::IpHash => {
            let ip_str = ip.unwrap_or("unknown");
            let hash = fnv_hash(ip_str);
            Some(alive[hash % alive.len()])
        }
        StickyStrategy::Cookie { cookie_name } => {
            if let Some(node_id) = cookies.and_then(|c| c.get(cookie_name.as_str())) {
                alive.iter().find(|n| &n.node_id == node_id).copied()
            } else {
                // Fall back to least connections when no cookie
                alive.iter().min_by_key(|n| n.connection_count).copied()
            }
        }
        StickyStrategy::LeastConnections => {
            alive.iter().min_by_key(|n| n.connection_count).copied()
        }
    }
}

/// Suggest the sticky node header/cookie value to embed in responses.
pub fn sticky_cookie_value(node: &NodeInfo) -> String {
    node.node_id.clone()
}

fn fnv_hash(s: &str) -> usize {
    const FNV_PRIME: usize = 1099511628211;
    const FNV_OFFSET: usize = 14695981039346656037;
    s.bytes().fold(FNV_OFFSET, |acc, b| (acc ^ b as usize).wrapping_mul(FNV_PRIME))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_node(id: &str, connections: usize) -> NodeInfo {
        let mut n = NodeInfo::new(id.to_string(), "127.0.0.1".to_string(), 9003, 9002);
        n.connection_count = connections;
        n.last_heartbeat = Utc::now();
        n
    }

    #[test]
    fn test_least_connections() {
        let nodes = vec![make_node("n1", 10), make_node("n2", 2), make_node("n3", 7)];
        let selected = select_node(&nodes, &StickyStrategy::LeastConnections, None, None).unwrap();
        assert_eq!(selected.node_id, "n2");
    }

    #[test]
    fn test_ip_hash_deterministic() {
        let nodes = vec![make_node("n1", 0), make_node("n2", 0), make_node("n3", 0)];
        let ip = "192.168.1.100";
        let a = select_node(&nodes, &StickyStrategy::IpHash, Some(ip), None).unwrap().node_id.clone();
        let b = select_node(&nodes, &StickyStrategy::IpHash, Some(ip), None).unwrap().node_id.clone();
        assert_eq!(a, b);
    }
}
