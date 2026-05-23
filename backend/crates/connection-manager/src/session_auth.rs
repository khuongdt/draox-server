//! Session-level authentication with role inheritance.
//!
//! A client authenticates once per session; all connections within that
//! session automatically inherit the authenticated identity. This avoids
//! re-authenticating every time a new connection is added to an existing
//! session.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use server_core::SessionId;

/// Metadata attached to an authenticated session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthInfo {
    /// The authenticated user's identifier.
    pub user_id: String,
    /// Roles granted to this user (e.g. `["admin", "player"]`).
    pub roles: Vec<String>,
    /// When authentication occurred.
    pub authenticated_at: DateTime<Utc>,
    /// SHA-256 / bcrypt hash of the token used — never store the raw token.
    pub token_hash: String,
}

/// Manages session authentication state.
///
/// Thread-safe; all operations are O(1) via `DashMap`.
pub struct SessionAuthenticator {
    authenticated: DashMap<SessionId, AuthInfo>,
}

impl SessionAuthenticator {
    /// Create an empty `SessionAuthenticator`.
    pub fn new() -> Self {
        Self {
            authenticated: DashMap::new(),
        }
    }

    /// Mark a session as authenticated with the given [`AuthInfo`].
    ///
    /// If the session was already authenticated the old info is replaced.
    pub fn authenticate(&self, session_id: &SessionId, info: AuthInfo) {
        self.authenticated.insert(session_id.clone(), info);
    }

    /// Check whether a session has been authenticated.
    pub fn is_authenticated(&self, session_id: &SessionId) -> bool {
        self.authenticated.contains_key(session_id)
    }

    /// Retrieve the [`AuthInfo`] for a session, if authenticated.
    pub fn get_auth(&self, session_id: &SessionId) -> Option<AuthInfo> {
        self.authenticated.get(session_id).map(|r| r.clone())
    }

    /// Revoke authentication for a session (e.g. on logout or token expiry).
    pub fn revoke(&self, session_id: &SessionId) {
        self.authenticated.remove(session_id);
    }

    /// Check whether an authenticated session has a specific role.
    ///
    /// Returns `false` if the session is not authenticated or does not have
    /// the requested role.
    pub fn has_role(&self, session_id: &SessionId, role: &str) -> bool {
        self.authenticated
            .get(session_id)
            .map(|info| info.roles.iter().any(|r| r == role))
            .unwrap_or(false)
    }
}

impl Default for SessionAuthenticator {
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

    fn make_auth_info(user_id: &str, roles: &[&str]) -> AuthInfo {
        AuthInfo {
            user_id: user_id.to_string(),
            roles: roles.iter().map(|r| r.to_string()).collect(),
            authenticated_at: Utc::now(),
            token_hash: "hash_placeholder".to_string(),
        }
    }

    #[test]
    fn test_authenticate_and_is_authenticated() {
        let auth = SessionAuthenticator::new();
        let sid = SessionId::new();

        assert!(!auth.is_authenticated(&sid));

        auth.authenticate(&sid, make_auth_info("user1", &["player"]));

        assert!(auth.is_authenticated(&sid));
    }

    #[test]
    fn test_get_auth_returns_correct_info() {
        let auth = SessionAuthenticator::new();
        let sid = SessionId::new();
        auth.authenticate(&sid, make_auth_info("user42", &["admin", "moderator"]));

        let info = auth.get_auth(&sid).unwrap();
        assert_eq!(info.user_id, "user42");
        assert!(info.roles.contains(&"admin".to_string()));
        assert!(info.roles.contains(&"moderator".to_string()));
    }

    #[test]
    fn test_revoke_removes_authentication() {
        let auth = SessionAuthenticator::new();
        let sid = SessionId::new();
        auth.authenticate(&sid, make_auth_info("user1", &["player"]));
        assert!(auth.is_authenticated(&sid));

        auth.revoke(&sid);
        assert!(!auth.is_authenticated(&sid));
        assert!(auth.get_auth(&sid).is_none());
    }

    #[test]
    fn test_has_role_true_and_false() {
        let auth = SessionAuthenticator::new();
        let sid = SessionId::new();
        auth.authenticate(&sid, make_auth_info("user1", &["player", "vip"]));

        assert!(auth.has_role(&sid, "player"));
        assert!(auth.has_role(&sid, "vip"));
        assert!(!auth.has_role(&sid, "admin"));
    }

    #[test]
    fn test_has_role_unauthenticated_returns_false() {
        let auth = SessionAuthenticator::new();
        let sid = SessionId::new();

        assert!(!auth.has_role(&sid, "admin"));
    }
}
