use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type UserId = String;
pub type TokenString = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthProvider {
    Local,
    Google,
    Discord,
    Apple,
    Steam,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub password_hash: Option<String>,
    pub provider: AuthProvider,
    pub provider_sub: Option<String>,
    pub mfa_enabled: bool,
    pub mfa_secret: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

impl User {
    pub fn new_local(username: String, email: String, password_hash: String) -> Self {
        let now = Utc::now();
        Self {
            id: format!("usr_{}", Uuid::new_v4().as_simple()),
            username,
            email,
            display_name: String::new(),
            password_hash: Some(password_hash),
            provider: AuthProvider::Local,
            provider_sub: None,
            mfa_enabled: false,
            mfa_secret: None,
            created_at: now,
            updated_at: now,
            last_login_at: None,
            is_active: true,
        }
    }

    pub fn new_oauth(
        username: String,
        email: String,
        provider: AuthProvider,
        provider_sub: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: format!("usr_{}", Uuid::new_v4().as_simple()),
            username,
            email: email.clone(),
            display_name: email,
            password_hash: None,
            provider,
            provider_sub: Some(provider_sub),
            mfa_enabled: false,
            mfa_secret: None,
            created_at: now,
            updated_at: now,
            last_login_at: None,
            is_active: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: TokenString,
    pub refresh_token: TokenString,
    pub expires_in: u64,
    pub token_type: String,
}

impl TokenPair {
    pub fn new(access_token: String, refresh_token: String, expires_in: u64) -> Self {
        Self {
            access_token,
            refresh_token,
            expires_in,
            token_type: "Bearer".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenRecord {
    pub token_hash: String,
    pub user_id: UserId,
    pub device_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("user not found")]
    UserNotFound,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("email already registered")]
    EmailAlreadyExists,
    #[error("username already taken")]
    UsernameAlreadyTaken,
    #[error("token expired")]
    TokenExpired,
    #[error("token invalid")]
    TokenInvalid,
    #[error("token revoked")]
    TokenRevoked,
    #[error("MFA required")]
    MfaRequired,
    #[error("MFA code invalid")]
    MfaCodeInvalid,
    #[error("account disabled")]
    AccountDisabled,
    #[error("internal error: {0}")]
    Internal(String),
}

pub type IdentityResult<T> = Result<T, IdentityError>;
