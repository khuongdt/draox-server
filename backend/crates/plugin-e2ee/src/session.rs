use serde::{Deserialize, Serialize};
use x25519_dalek::PublicKey;
use crate::keypair::{EphemeralKeyPair, IdentityKeyPair};
use crate::ratchet::SymmetricRatchet;

/// Serializable handshake bundle sent to the remote party to establish a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeBundle {
    /// Sender's identity public key (32 bytes, base64).
    pub identity_key: [u8; 32],
    /// Ephemeral public key for this session (X3DH-style forward secrecy).
    pub ephemeral_key: [u8; 32],
}

/// An established E2EE session between two clients.
/// Contains separate ratchets for the send and receive directions.
pub struct E2EESession {
    pub peer_id: String,
    send_ratchet: SymmetricRatchet,
    recv_ratchet: SymmetricRatchet,
}

impl E2EESession {
    /// Called by the session **initiator** (Alice).
    /// `my_identity` is Alice's long-term key; `peer_ephemeral` is Bob's ephemeral public key.
    pub fn initiate(
        peer_id: impl Into<String>,
        my_identity: &IdentityKeyPair,
        my_ephemeral: &EphemeralKeyPair,
        peer_identity_pub: &PublicKey,
        peer_ephemeral_pub: &PublicKey,
    ) -> Self {
        // Combine four DH outputs (X3DH simplified: IK·EK + EK·IK)
        let dh1 = my_identity.dh(peer_ephemeral_pub);
        let dh2 = my_ephemeral.dh(peer_identity_pub);
        let mut master = [0u8; 64];
        master[..32].copy_from_slice(&dh1);
        master[32..].copy_from_slice(&dh2);
        let shared = Self::kdf(&master);

        Self {
            peer_id: peer_id.into(),
            send_ratchet: SymmetricRatchet::from_shared_secret(&shared),
            recv_ratchet: SymmetricRatchet::from_shared_secret(&shared),
        }
    }

    /// Called by the session **responder** (Bob) upon receiving a [`HandshakeBundle`].
    pub fn respond(
        peer_id: impl Into<String>,
        my_identity: &IdentityKeyPair,
        my_ephemeral: &EphemeralKeyPair,
        initiator_identity_pub: &PublicKey,
        initiator_ephemeral_pub: &PublicKey,
    ) -> Self {
        // Mirror DH order so both parties derive the same shared secret.
        let dh1 = my_ephemeral.dh(initiator_identity_pub);
        let dh2 = my_identity.dh(initiator_ephemeral_pub);
        let mut master = [0u8; 64];
        master[..32].copy_from_slice(&dh1);
        master[32..].copy_from_slice(&dh2);
        let shared = Self::kdf(&master);

        Self {
            peer_id: peer_id.into(),
            send_ratchet: SymmetricRatchet::from_shared_secret(&shared),
            recv_ratchet: SymmetricRatchet::from_shared_secret(&shared),
        }
    }

    fn kdf(material: &[u8; 64]) -> [u8; 32] {
        use hkdf::Hkdf;
        use sha2::Sha256;
        let hk = Hkdf::<Sha256>::new(None, material);
        let mut out = [0u8; 32];
        hk.expand(b"draox-e2ee-session-v1", &mut out).expect("HKDF");
        out
    }

    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<EncryptedMessage, crate::ratchet::RatchetError> {
        let (ciphertext, index) = self.send_ratchet.encrypt(plaintext)?;
        Ok(EncryptedMessage { ciphertext, index })
    }

    pub fn decrypt(&mut self, msg: &EncryptedMessage) -> Result<Vec<u8>, crate::ratchet::RatchetError> {
        self.recv_ratchet.decrypt(&msg.ciphertext)
    }
}

/// Wire format for an encrypted message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedMessage {
    pub ciphertext: Vec<u8>,
    /// Sender's ratchet index; used by receiver to detect out-of-order delivery.
    pub index: u64,
}
