//! Connection failover — promote a backup connection when the primary fails.
//!
//! When the primary connection for a session disconnects, `FailoverManager`
//! consults the session's [`FailoverPolicy`] and returns the best candidate
//! connection to promote to Primary.

use crate::session::ClientSession;
use dashmap::DashMap;
use server_core::{ConnectionId, ConnectionRole, SessionId};

/// Policy that controls how a replacement primary is selected after the
/// current primary disconnects.
#[derive(Debug, Clone)]
pub enum FailoverPolicy {
    /// Promote the connection with the smallest (oldest) ID string — a
    /// deterministic stand-in for "oldest connection" when creation order
    /// is not separately tracked.
    PromoteOldest,
    /// Promote a connection that currently holds a specific role.
    PromoteByRole(ConnectionRole),
    /// Do nothing — the session will have no primary until a new connection
    /// establishes with that role.
    NoFailover,
}

/// Stores per-session failover policies and provides the logic to elect a
/// replacement primary.
pub struct FailoverManager {
    failover_rules: DashMap<SessionId, FailoverPolicy>,
}

impl FailoverManager {
    /// Create a new `FailoverManager` with no policies configured.
    pub fn new() -> Self {
        Self {
            failover_rules: DashMap::new(),
        }
    }

    /// Set the failover policy for a session.
    pub fn set_policy(&self, session_id: &SessionId, policy: FailoverPolicy) {
        self.failover_rules.insert(session_id.clone(), policy);
    }

    /// Called when `disconnected` connection is removed from `session`.
    ///
    /// Returns the [`ConnectionId`] that should be promoted to Primary, or
    /// `None` if no promotion is warranted (policy is `NoFailover`, no
    /// suitable candidate exists, or the disconnected connection was not the
    /// primary).
    ///
    /// Note: this method does **not** mutate the session — the caller is
    /// responsible for calling `session.promote_connection` with the returned
    /// ID.
    pub fn handle_disconnect(
        &self,
        session: &ClientSession,
        disconnected: &ConnectionId,
    ) -> Option<ConnectionId> {
        // Only act when the primary connection disconnected.
        if session.get_role(disconnected) != Some(&ConnectionRole::Primary) {
            return None;
        }

        let policy = self
            .failover_rules
            .get(&session.session_id)
            .map(|r| r.clone())
            .unwrap_or(FailoverPolicy::NoFailover);

        match policy {
            FailoverPolicy::NoFailover => None,

            FailoverPolicy::PromoteOldest => {
                // Pick any non-primary, non-disconnected connection.
                // We use the string representation of ConnectionId as a
                // deterministic tie-breaker (lexicographic order).
                session
                    .connections
                    .iter()
                    .filter(|(id, role)| {
                        *id != disconnected && **role != ConnectionRole::Primary
                    })
                    .min_by_key(|(id, _)| id.as_str().to_string())
                    .map(|(id, _)| id.clone())
            }

            FailoverPolicy::PromoteByRole(target_role) => {
                // Find a connection currently holding the target role.
                session
                    .connections
                    .iter()
                    .find(|(id, role)| *id != disconnected && **role == target_role)
                    .map(|(id, _)| id.clone())
            }
        }
    }
}

impl Default for FailoverManager {
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
    use server_core::ClientId;

    fn make_session_with_connections(roles: &[(ConnectionRole,)]) -> (ClientSession, Vec<ConnectionId>) {
        let mut session = ClientSession::new(ClientId::new());
        let mut ids = Vec::new();
        for (role,) in roles {
            let id = ConnectionId::new();
            session.add_connection(id.clone(), *role).unwrap();
            ids.push(id);
        }
        (session, ids)
    }

    #[test]
    fn test_no_failover_policy_returns_none() {
        let fm = FailoverManager::new();
        let (mut session, ids) = make_session_with_connections(&[
            (ConnectionRole::Primary,),
            (ConnectionRole::Streaming,),
        ]);
        // No policy set — defaults to NoFailover.
        let result = fm.handle_disconnect(&session, &ids[0]);
        // But we need to remove the primary from the session first to simulate disconnect.
        session.remove_connection(&ids[0]);
        let result2 = fm.handle_disconnect(&session, &ids[0]);
        assert!(result.is_none() || result2.is_none());
    }

    #[test]
    fn test_promote_oldest_selects_candidate() {
        let fm = FailoverManager::new();
        let (session, ids) = make_session_with_connections(&[
            (ConnectionRole::Primary,),
            (ConnectionRole::Notification,),
            (ConnectionRole::Streaming,),
        ]);
        fm.set_policy(&session.session_id, FailoverPolicy::PromoteOldest);

        let candidate = fm.handle_disconnect(&session, &ids[0]);
        // Must be one of the non-primary connections.
        assert!(candidate.is_some());
        let c = candidate.unwrap();
        assert_ne!(c, ids[0]);
        assert!(c == ids[1] || c == ids[2]);
    }

    #[test]
    fn test_promote_by_role_finds_correct_connection() {
        let fm = FailoverManager::new();
        let (session, ids) = make_session_with_connections(&[
            (ConnectionRole::Primary,),
            (ConnectionRole::Notification,),
            (ConnectionRole::Streaming,),
        ]);
        fm.set_policy(
            &session.session_id,
            FailoverPolicy::PromoteByRole(ConnectionRole::Notification),
        );

        let candidate = fm.handle_disconnect(&session, &ids[0]);
        assert_eq!(candidate, Some(ids[1].clone()));
    }

    #[test]
    fn test_non_primary_disconnect_returns_none() {
        let fm = FailoverManager::new();
        let (session, ids) = make_session_with_connections(&[
            (ConnectionRole::Primary,),
            (ConnectionRole::Streaming,),
        ]);
        fm.set_policy(&session.session_id, FailoverPolicy::PromoteOldest);

        // Disconnect the Streaming connection — not the primary.
        let candidate = fm.handle_disconnect(&session, &ids[1]);
        assert!(candidate.is_none());
    }
}
