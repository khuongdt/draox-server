use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use crate::types::{IdentityError, IdentityResult, UserId};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: UserId,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,
    pub token_type: String,
}

pub struct TokenService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_ttl_secs: u64,
    refresh_ttl_secs: u64,
}

impl TokenService {
    pub fn new(secret: &str, access_ttl_secs: u64, refresh_ttl_secs: u64) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            access_ttl_secs,
            refresh_ttl_secs,
        }
    }

    pub fn issue_access_token(&self, user_id: &UserId) -> IdentityResult<String> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.access_ttl_secs as i64);
        let claims = Claims {
            sub: user_id.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: uuid::Uuid::new_v4().to_string(),
            token_type: "access".to_string(),
        };
        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| IdentityError::Internal(e.to_string()))
    }

    pub fn issue_refresh_token(&self, user_id: &UserId) -> IdentityResult<String> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.refresh_ttl_secs as i64);
        let claims = Claims {
            sub: user_id.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: uuid::Uuid::new_v4().to_string(),
            token_type: "refresh".to_string(),
        };
        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| IdentityError::Internal(e.to_string()))
    }

    pub fn verify_token(&self, token: &str) -> IdentityResult<Claims> {
        decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map(|data| data.claims)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => IdentityError::TokenExpired,
                _ => IdentityError::TokenInvalid,
            })
    }

    pub fn access_ttl_secs(&self) -> u64 {
        self.access_ttl_secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_token_roundtrip() {
        let svc = TokenService::new("test-secret-key", 3600, 86400);
        let uid = "usr_abc123".to_string();
        let token = svc.issue_access_token(&uid).unwrap();
        let claims = svc.verify_token(&token).unwrap();
        assert_eq!(claims.sub, uid);
        assert_eq!(claims.token_type, "access");
    }

    #[test]
    fn test_expired_token_rejected() {
        let svc = TokenService::new("test-secret-key", 3600, 86400);
        // Manually issue a token with exp far in the past (Unix epoch + 1)
        let claims = Claims {
            sub: "usr_x".to_string(),
            exp: 1,
            iat: 0,
            jti: "test-jti".to_string(),
            token_type: "access".to_string(),
        };
        let token = encode(&Header::default(), &claims, &svc.encoding_key).unwrap();
        let result = svc.verify_token(&token);
        assert!(matches!(result, Err(IdentityError::TokenExpired)));
    }
}
