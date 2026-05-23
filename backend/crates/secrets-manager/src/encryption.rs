use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::RngCore;
use crate::provider::{SecretsError, SecretsResult};

/// Encrypt a plaintext value at rest using AES-256-GCM.
/// Returns base64(nonce || ciphertext).
pub fn encrypt_at_rest(plaintext: &str, key_bytes: &[u8; 32]) -> SecretsResult<String> {
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| SecretsError::Encryption(e.to_string()))?;

    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(STANDARD.encode(combined))
}

/// Decrypt a value previously encrypted with `encrypt_at_rest`.
pub fn decrypt_at_rest(encoded: &str, key_bytes: &[u8; 32]) -> SecretsResult<String> {
    let combined = STANDARD
        .decode(encoded)
        .map_err(|e| SecretsError::Encryption(e.to_string()))?;

    if combined.len() < 12 {
        return Err(SecretsError::Encryption("ciphertext too short".to_string()));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| SecretsError::Encryption(e.to_string()))?;

    String::from_utf8(plaintext)
        .map_err(|e| SecretsError::Encryption(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [0u8; 32];
        let plaintext = "super-secret-password-123";
        let encrypted = encrypt_at_rest(plaintext, &key).unwrap();
        let decrypted = decrypt_at_rest(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_nonces_each_time() {
        let key = [42u8; 32];
        let pt = "value";
        let e1 = encrypt_at_rest(pt, &key).unwrap();
        let e2 = encrypt_at_rest(pt, &key).unwrap();
        // Same plaintext should produce different ciphertext due to random nonce
        assert_ne!(e1, e2);
    }
}
