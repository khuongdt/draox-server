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
}
