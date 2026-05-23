use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use hkdf::Hkdf;
use sha2::Sha256;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RatchetError {
    #[error("encryption failed")]
    Encrypt,
    #[error("decryption failed: message key mismatch or tampered ciphertext")]
    Decrypt,
}

const INFO_ROOT: &[u8] = b"draox-e2ee-root-v1";
const INFO_CHAIN: &[u8] = b"draox-e2ee-chain-v1";
const INFO_MESSAGE: &[u8] = b"draox-e2ee-message-v1";

/// Symmetric-key Double Ratchet (simplified: only the symmetric sending chain).
/// Each call to `encrypt` / `decrypt` advances the chain key, producing a fresh
/// message key so that compromise of one message key does not expose past messages.
pub struct SymmetricRatchet {
    chain_key: [u8; 32],
    message_index: u64,
}

impl SymmetricRatchet {
    /// Derive root & initial chain key from a shared DH secret using HKDF-SHA256.
    pub fn from_shared_secret(dh_secret: &[u8; 32]) -> Self {
        let hk = Hkdf::<Sha256>::new(None, dh_secret);
        let mut chain_key = [0u8; 32];
        hk.expand(INFO_ROOT, &mut chain_key).expect("HKDF expand");
        Self {
            chain_key,
            message_index: 0,
        }
    }

    /// Advance the chain key, returning the next message key.
    fn advance(&mut self) -> [u8; 32] {
        let hk = Hkdf::<Sha256>::new(None, &self.chain_key);
        let mut new_chain = [0u8; 32];
        let mut msg_key = [0u8; 32];
        hk.expand(INFO_CHAIN, &mut new_chain).expect("HKDF chain");
        hk.expand(INFO_MESSAGE, &mut msg_key).expect("HKDF msg");
        self.chain_key = new_chain;
        self.message_index += 1;
        msg_key
    }

    /// Encrypt `plaintext` and return `(ciphertext, message_index)`.
    /// The message index is sent alongside so the receiver can sync their ratchet.
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<(Vec<u8>, u64), RatchetError> {
        let msg_key = self.advance();
        let cipher = ChaCha20Poly1305::new((&msg_key).into());
        // Use message_index as deterministic nonce (12 bytes; index is 8 bytes padded).
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..].copy_from_slice(&(self.message_index - 1).to_be_bytes());
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, plaintext).map_err(|_| RatchetError::Encrypt)?;
        Ok((ciphertext, self.message_index - 1))
    }

    /// Decrypt `ciphertext` that was produced at `message_index`.
    /// The receiver must have advanced their ratchet to the same index.
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, RatchetError> {
        let msg_key = self.advance();
        let cipher = ChaCha20Poly1305::new((&msg_key).into());
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..].copy_from_slice(&(self.message_index - 1).to_be_bytes());
        let nonce = Nonce::from_slice(&nonce_bytes);
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| RatchetError::Decrypt)
    }

    pub fn message_index(&self) -> u64 {
        self.message_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pair() -> (SymmetricRatchet, SymmetricRatchet) {
        let shared = [0xab_u8; 32];
        (
            SymmetricRatchet::from_shared_secret(&shared),
            SymmetricRatchet::from_shared_secret(&shared),
        )
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let (mut sender, mut receiver) = make_pair();
        let plaintext = b"hello e2ee";
        let (ct, _idx) = sender.encrypt(plaintext).unwrap();
        let pt = receiver.decrypt(&ct).unwrap();
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn test_multiple_messages() {
        let (mut sender, mut receiver) = make_pair();
        for i in 0..10u8 {
            let msg = vec![i; 32];
            let (ct, _) = sender.encrypt(&msg).unwrap();
            let pt = receiver.decrypt(&ct).unwrap();
            assert_eq!(pt, msg);
        }
    }

    #[test]
    fn test_tampered_ciphertext_rejected() {
        let (mut sender, mut receiver) = make_pair();
        let (mut ct, _) = sender.encrypt(b"secret").unwrap();
        ct[0] ^= 0xff;
        assert!(receiver.decrypt(&ct).is_err());
    }
}
