use serde::{Deserialize, Serialize};

/// Authenticated caller identity, extracted from an inbound request
/// (a JWT in the `Authorization: Bearer` header for REST, or the session
/// authentication for a WebSocket connection).
///
/// `admin-api` inserts this into Axum request extensions via the
/// `auth_extract` middleware. Plugin handlers extract it with
/// `Extension<Identity>`. Defining it in `plugin-sdk` lets plugin
/// crates depend on it without importing `admin-api`, preserving the
/// layer hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// User identifier (username or account-id).
    pub user_id: String,
    /// Role of the caller (e.g. "admin", "operator", "viewer", "user").
    pub role: String,
}

impl Identity {
    pub fn new(user_id: impl Into<String>, role: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            role: role.into(),
        }
    }

    /// `true` when the caller is the platform admin.
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }

    /// `true` when the caller is an operator (treated as "mod" for the
    /// purposes of joining system channels/clans).
    pub fn is_operator(&self) -> bool {
        self.role == "operator"
    }

    /// `true` when the caller may join admin-only resources or take
    /// moderate-level actions (`admin` or `operator`).
    pub fn can_moderate(&self) -> bool {
        self.is_admin() || self.is_operator()
    }
}
