use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use crate::types::{IdentityError, IdentityResult};

pub fn hash_password(password: &str) -> IdentityResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| IdentityError::Internal(e.to_string()))
}

pub fn verify_password(password: &str, hash: &str) -> IdentityResult<bool> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| IdentityError::Internal(e.to_string()))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let hash = hash_password("secret123").unwrap();
        assert!(verify_password("secret123", &hash).unwrap());
        assert!(!verify_password("wrongpass", &hash).unwrap());
    }
}
