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
/// Maintains a secondary index from token_hash → SessionId to enable
/// token-based session binding for reconnecting clients.
pub struct SessionAuthenticator {
    authenticated: DashMap<SessionId, AuthInfo>,
    /// Secondary index: token_hash → session_id for O(1) token lookup.
    token_index: DashMap<String, SessionId>,
}

impl SessionAuthenticator {
    /// Create an empty `SessionAuthenticator`.
    pub fn new() -> Self {
        Self {
            authenticated: DashMap::new(),
            token_index: DashMap::new(),
        }
    }

    /// Mark a session as authenticated with the given [`AuthInfo`].
    ///
    /// If the session was already authenticated the old token_hash is removed
    /// from the index before the new one is inserted.
    pub fn authenticate(&self, session_id: &SessionId, info: AuthInfo) {
        if let Some(old) = self.authenticated.get(session_id) {
            self.token_index.remove(&old.token_hash);
        }
        self.token_index.insert(info.token_hash.clone(), session_id.clone());
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
    ///
    /// Also removes the token_hash from the secondary index.
    pub fn revoke(&self, session_id: &SessionId) {
        if let Some((_, info)) = self.authenticated.remove(session_id) {
            self.token_index.remove(&info.token_hash);
        }
    }

    /// Look up a session by its token hash.
    ///
    /// Used for token-based session binding: a reconnecting client presents
    /// its token; the caller hashes it and passes the hash here to find which
    /// session the token belongs to.
    pub fn find_session_by_token_hash(&self, token_hash: &str) -> Option<SessionId> {
        self.token_index.get(token_hash).map(|r| r.value().clone())
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

    #[test]
    fn test_find_session_by_token_hash() {
        let auth = SessionAuthenticator::new();
        let sid = SessionId::new();
        let info = AuthInfo {
            user_id: "u1".into(),
            roles: vec!["player".into()],
            authenticated_at: Utc::now(),
            token_hash: "sha256_abc".into(),
        };
        auth.authenticate(&sid, info);

        assert_eq!(auth.find_session_by_token_hash("sha256_abc"), Some(sid.clone()));
        assert!(auth.find_session_by_token_hash("wrong_hash").is_none());
    }

    #[test]
    fn test_revoke_removes_token_index() {
        let auth = SessionAuthenticator::new();
        let sid = SessionId::new();
        auth.authenticate(&sid, make_auth_info("u1", &["player"]));

        assert!(auth.find_session_by_token_hash("hash_placeholder").is_some());

        auth.revoke(&sid);
        assert!(auth.find_session_by_token_hash("hash_placeholder").is_none());
    }

    #[test]
    fn test_re_authenticate_replaces_token_index() {
        let auth = SessionAuthenticator::new();
        let sid = SessionId::new();

        auth.authenticate(&sid, make_auth_info("u1", &["player"]));
        assert!(auth.find_session_by_token_hash("hash_placeholder").is_some());

        // Re-authenticate with different token_hash
        let new_info = AuthInfo {
            user_id: "u1".into(),
            roles: vec!["player".into()],
            authenticated_at: Utc::now(),
            token_hash: "new_hash_xyz".into(),
        };
        auth.authenticate(&sid, new_info);

        // Old token removed, new token indexed
        assert!(auth.find_session_by_token_hash("hash_placeholder").is_none());
        assert_eq!(auth.find_session_by_token_hash("new_hash_xyz"), Some(sid));
    }
}
