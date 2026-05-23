use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use crate::types::{IdentityError, IdentityResult, RefreshTokenRecord, UserId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSession {
    pub session_id: String,
    pub user_id: UserId,
    pub device_id: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// In-memory session store. In production, back this with Redis.
#[derive(Clone)]
pub struct SessionStore {
    refresh_tokens: Arc<DashMap<String, RefreshTokenRecord>>,
    sessions: Arc<DashMap<String, ActiveSession>>,
    refresh_ttl_secs: u64,
}

impl SessionStore {
    pub fn new(refresh_ttl_secs: u64) -> Self {
        Self {
            refresh_tokens: Arc::new(DashMap::new()),
            sessions: Arc::new(DashMap::new()),
            refresh_ttl_secs,
        }
    }

    pub fn store_refresh_token(
        &self,
        token: &str,
        user_id: UserId,
        device_id: Option<String>,
    ) -> RefreshTokenRecord {
        let hash = Self::hash_token(token);
        let now = Utc::now();
        let record = RefreshTokenRecord {
            token_hash: hash.clone(),
            user_id,
            device_id,
            created_at: now,
            expires_at: now + Duration::seconds(self.refresh_ttl_secs as i64),
            revoked: false,
        };
        self.refresh_tokens.insert(hash, record.clone());
        record
    }

    pub fn validate_refresh_token(&self, token: &str) -> IdentityResult<RefreshTokenRecord> {
        let hash = Self::hash_token(token);
        let record = self
            .refresh_tokens
            .get(&hash)
            .map(|r| r.clone())
            .ok_or(IdentityError::TokenInvalid)?;

        if record.revoked {
            return Err(IdentityError::TokenRevoked);
        }
        if record.expires_at < Utc::now() {
            return Err(IdentityError::TokenExpired);
        }
        Ok(record)
    }

    /// Rotate: revoke old token and issue a new record placeholder.
    pub fn rotate_refresh_token(&self, old_token: &str) -> IdentityResult<RefreshTokenRecord> {
        let hash = Self::hash_token(old_token);
        if let Some(mut entry) = self.refresh_tokens.get_mut(&hash) {
            if entry.revoked || entry.expires_at < Utc::now() {
                return Err(IdentityError::TokenInvalid);
            }
            entry.revoked = true;
            Ok(entry.clone())
        } else {
            Err(IdentityError::TokenInvalid)
        }
    }

    /// Revoke all refresh tokens for a user (logout from all devices).
    pub fn revoke_all_for_user(&self, user_id: &UserId) {
        self.refresh_tokens.iter_mut().for_each(|mut entry| {
            if &entry.user_id == user_id {
                entry.revoked = true;
            }
        });
    }

    /// Revoke a specific device's refresh token.
    pub fn revoke_device(&self, user_id: &UserId, device_id: &str) {
        self.refresh_tokens.iter_mut().for_each(|mut entry| {
            if &entry.user_id == user_id && entry.device_id.as_deref() == Some(device_id) {
                entry.revoked = true;
            }
        });
    }

    pub fn create_session(&self, session: ActiveSession) {
        self.sessions.insert(session.session_id.clone(), session);
    }

    pub fn get_sessions_for_user(&self, user_id: &UserId) -> Vec<ActiveSession> {
        self.sessions
            .iter()
            .filter(|entry| &entry.value().user_id == user_id && entry.value().expires_at > Utc::now())
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn remove_session(&self, session_id: &str) {
        self.sessions.remove(session_id);
    }

    fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_validate_refresh_token() {
        let store = SessionStore::new(3600);
        let token = "my-refresh-token";
        store.store_refresh_token(token, "usr_1".to_string(), None);
        let record = store.validate_refresh_token(token).unwrap();
        assert_eq!(record.user_id, "usr_1");
        assert!(!record.revoked);
    }

    #[test]
    fn test_revoke_all_for_user() {
        let store = SessionStore::new(3600);
        store.store_refresh_token("tok1", "usr_1".to_string(), None);
        store.store_refresh_token("tok2", "usr_1".to_string(), None);
        store.revoke_all_for_user(&"usr_1".to_string());
        assert!(matches!(
            store.validate_refresh_token("tok1"),
            Err(IdentityError::TokenRevoked)
        ));
    }
}
