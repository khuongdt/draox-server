use std::fmt;

/// The result of a traffic guard check on an incoming connection or request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardVerdict {
    /// The connection/request is allowed to proceed.
    Allow,
    /// The connection/request is blocked for the given reason.
    Block(String),
    /// The connection/request should be throttled (slowed down).
    Throttle,
}

impl fmt::Display for GuardVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GuardVerdict::Allow => write!(f, "Allow"),
            GuardVerdict::Block(reason) => write!(f, "Block: {reason}"),
            GuardVerdict::Throttle => write!(f, "Throttle"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verdict_display() {
        assert_eq!(GuardVerdict::Allow.to_string(), "Allow");
        assert_eq!(
            GuardVerdict::Block("rate limited".to_string()).to_string(),
            "Block: rate limited"
        );
        assert_eq!(GuardVerdict::Throttle.to_string(), "Throttle");
    }
}
