use dashmap::DashMap;
use std::sync::Arc;
use tracing::{info, warn};
use crate::mfa::TotpService;
use crate::password::{hash_password, verify_password};
use crate::session::{ActiveSession, SessionStore};
use crate::token::TokenService;
use crate::types::*;

/// Central identity manager — handles registration, login, token lifecycle.
pub struct IdentityManager {
    users: Arc<DashMap<UserId, User>>,
    users_by_email: Arc<DashMap<String, UserId>>,
    token_service: Arc<TokenService>,
    session_store: Arc<SessionStore>,
}

impl IdentityManager {
    pub fn new(jwt_secret: &str, access_ttl_secs: u64, refresh_ttl_secs: u64) -> Self {
        Self {
            users: Arc::new(DashMap::new()),
            users_by_email: Arc::new(DashMap::new()),
            token_service: Arc::new(TokenService::new(jwt_secret, access_ttl_secs, refresh_ttl_secs)),
            session_store: Arc::new(SessionStore::new(refresh_ttl_secs)),
        }
    }

    // ── Registration ─────────────────────────────────────────────────

    pub fn register(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> IdentityResult<User> {
        if self.users_by_email.contains_key(&email) {
            return Err(IdentityError::EmailAlreadyExists);
        }
        let password_hash = hash_password(&password)?;
        let user = User::new_local(username, email.clone(), password_hash);
        self.users_by_email.insert(email, user.id.clone());
        self.users.insert(user.id.clone(), user.clone());
        info!(user_id = %user.id, "new user registered");
        Ok(user)
    }

    pub fn register_oauth(&self, info: crate::oauth::OAuthUserInfo) -> IdentityResult<User> {
        if let Some(id) = self.users_by_email.get(&info.email) {
            return self.get_user_by_id(&id).map(|u| u.clone());
        }
        let user = User::new_oauth(
            info.display_name.clone(),
            info.email.clone(),
            info.provider,
            info.sub,
        );
        self.users_by_email.insert(info.email, user.id.clone());
        self.users.insert(user.id.clone(), user.clone());
        info!(user_id = %user.id, "oauth user registered");
        Ok(user)
    }

    // ── Authentication ────────────────────────────────────────────────

    pub fn login(
        &self,
        email: &str,
        password: &str,
        device_id: Option<String>,
        ip: Option<String>,
    ) -> IdentityResult<TokenPair> {
        let user_id = self.users_by_email
            .get(email)
            .map(|id| id.clone())
            .ok_or(IdentityError::UserNotFound)?;

        let user = self.get_user_by_id(&user_id)?;

        if !user.is_active {
            return Err(IdentityError::AccountDisabled);
        }

        let password_hash = user
            .password_hash
            .as_deref()
            .ok_or(IdentityError::InvalidCredentials)?;

        if !verify_password(password, password_hash)? {
            warn!(user_id = %user_id, "failed login attempt");
            return Err(IdentityError::InvalidCredentials);
        }

        if user.mfa_enabled {
            return Err(IdentityError::MfaRequired);
        }

        self.issue_token_pair(&user_id, device_id, ip)
    }

    pub fn login_with_mfa(
        &self,
        email: &str,
        password: &str,
        mfa_code: &str,
        device_id: Option<String>,
        ip: Option<String>,
    ) -> IdentityResult<TokenPair> {
        let user_id = self.users_by_email
            .get(email)
            .map(|id| id.clone())
            .ok_or(IdentityError::UserNotFound)?;

        let user = self.get_user_by_id(&user_id)?;

        if !user.is_active {
            return Err(IdentityError::AccountDisabled);
        }

        let password_hash = user
            .password_hash
            .as_deref()
            .ok_or(IdentityError::InvalidCredentials)?;

        if !verify_password(password, password_hash)? {
            return Err(IdentityError::InvalidCredentials);
        }

        let mfa_secret = user
            .mfa_secret
            .as_deref()
            .ok_or(IdentityError::MfaCodeInvalid)?;

        if !TotpService::verify_code(mfa_secret, mfa_code, &user.email, "Draox")? {
            return Err(IdentityError::MfaCodeInvalid);
        }

        self.issue_token_pair(&user_id, device_id, ip)
    }

    // ── Token management ─────────────────────────────────────────────

    pub fn refresh(&self, refresh_token: &str) -> IdentityResult<TokenPair> {
        let record = self.session_store.validate_refresh_token(refresh_token)?;
        // Rotate: invalidate old token
        self.session_store.rotate_refresh_token(refresh_token)?;
        self.issue_token_pair(&record.user_id, record.device_id, None)
    }

    pub fn logout(&self, refresh_token: &str) -> IdentityResult<()> {
        self.session_store.rotate_refresh_token(refresh_token)?;
        Ok(())
    }

    pub fn logout_all(&self, user_id: &UserId) {
        self.session_store.revoke_all_for_user(user_id);
        info!(user_id = %user_id, "all sessions revoked");
    }

    pub fn verify_access_token(&self, token: &str) -> IdentityResult<UserId> {
        let claims = self.token_service.verify_token(token)?;
        Ok(claims.sub)
    }

    // ── MFA setup ────────────────────────────────────────────────────

    pub fn setup_mfa(&self, user_id: &UserId) -> IdentityResult<String> {
        let mut user = self.get_user_by_id(user_id)?.clone();
        let secret = TotpService::generate_secret();
        let uri = TotpService::provisioning_uri(&secret, &user.email, "Draox")?;
        user.mfa_secret = Some(secret);
        user.mfa_enabled = true;
        self.users.insert(user_id.clone(), user);
        Ok(uri)
    }

    pub fn disable_mfa(&self, user_id: &UserId) -> IdentityResult<()> {
        let mut user = self.get_user_by_id(user_id)?.clone();
        user.mfa_secret = None;
        user.mfa_enabled = false;
        self.users.insert(user_id.clone(), user);
        Ok(())
    }

    // ── Helpers ───────────────────────────────────────────────────────

    pub fn get_user_by_id(&self, user_id: &UserId) -> IdentityResult<impl std::ops::Deref<Target = User> + '_> {
        self.users.get(user_id).ok_or(IdentityError::UserNotFound)
    }

    pub fn get_user_by_email(&self, email: &str) -> IdentityResult<UserId> {
        self.users_by_email
            .get(email)
            .map(|id| id.clone())
            .ok_or(IdentityError::UserNotFound)
    }

    fn issue_token_pair(
        &self,
        user_id: &UserId,
        device_id: Option<String>,
        ip: Option<String>,
    ) -> IdentityResult<TokenPair> {
        let access = self.token_service.issue_access_token(user_id)?;
        let refresh = self.token_service.issue_refresh_token(user_id)?;
        self.session_store.store_refresh_token(&refresh, user_id.clone(), device_id.clone());

        let session = ActiveSession {
            session_id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.clone(),
            device_id,
            ip_address: ip,
            created_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now()
                + chrono::Duration::seconds(self.token_service.access_ttl_secs() as i64),
        };
        self.session_store.create_session(session);

        Ok(TokenPair::new(access, refresh, self.token_service.access_ttl_secs()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager() -> IdentityManager {
        IdentityManager::new("test-secret", 3600, 86400)
    }

    #[test]
    fn test_register_and_login() {
        let mgr = make_manager();
        mgr.register("alice".to_string(), "alice@example.com".to_string(), "pass123".to_string()).unwrap();
        let pair = mgr.login("alice@example.com", "pass123", None, None).unwrap();
        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
    }

    #[test]
    fn test_duplicate_email_rejected() {
        let mgr = make_manager();
        mgr.register("alice".to_string(), "alice@example.com".to_string(), "pass1".to_string()).unwrap();
        let result = mgr.register("alice2".to_string(), "alice@example.com".to_string(), "pass2".to_string());
        assert!(matches!(result, Err(IdentityError::EmailAlreadyExists)));
    }

    #[test]
    fn test_wrong_password_rejected() {
        let mgr = make_manager();
        mgr.register("bob".to_string(), "bob@example.com".to_string(), "correct".to_string()).unwrap();
        let result = mgr.login("bob@example.com", "wrong", None, None);
        assert!(matches!(result, Err(IdentityError::InvalidCredentials)));
    }

    #[test]
    fn test_token_refresh() {
        let mgr = make_manager();
        mgr.register("carol".to_string(), "carol@example.com".to_string(), "pw".to_string()).unwrap();
        let pair = mgr.login("carol@example.com", "pw", None, None).unwrap();
        let new_pair = mgr.refresh(&pair.refresh_token).unwrap();
        assert!(!new_pair.access_token.is_empty());
        // Old refresh token is now invalid
        assert!(mgr.refresh(&pair.refresh_token).is_err());
    }

    #[test]
    fn test_verify_access_token() {
        let mgr = make_manager();
        mgr.register("dave".to_string(), "dave@example.com".to_string(), "pw".to_string()).unwrap();
        let pair = mgr.login("dave@example.com", "pw", None, None).unwrap();
        let uid = mgr.verify_access_token(&pair.access_token).unwrap();
        assert!(uid.starts_with("usr_"));
    }
}
