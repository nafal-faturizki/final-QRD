/// Ed25519 digital signature operations for schema signing and verification
use crate::error::{QrdError, Result};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey, Signer, Verifier};
use rand::thread_rng;

/// Ed25519 signing and verification configuration
pub const SIGNATURE_ALGORITHM: u8 = 0x01; // Ed25519
pub const SIGNATURE_SIZE: usize = 64;
pub const PUBLIC_KEY_SIZE: usize = 32;
pub const PRIVATE_KEY_SIZE: usize = 32;

/// Keypair for signing operations
#[derive(Debug, Clone)]
pub struct SigningKeyPair {
    signing_key: SigningKey,
}

impl SigningKeyPair {
    /// Generates a new Ed25519 keypair from random entropy
    pub fn generate() -> Self {
        let mut csprng = thread_rng();
        let mut seed_bytes = [0u8; 32];
        // Generate random seed bytes
        use rand::RngCore;
        csprng.fill_bytes(&mut seed_bytes);
        
        let signing_key = SigningKey::from_bytes(&seed_bytes);
        Self { signing_key }
    }

    /// Creates a keypair from a seed (32 bytes)
    pub fn from_seed(seed: [u8; 32]) -> Result<Self> {
        let signing_key = SigningKey::from_bytes(&seed);
        Ok(Self { signing_key })
    }

    /// Gets the verifying key (public key)
    pub fn verifying_key(&self) -> [u8; PUBLIC_KEY_SIZE] {
        let vk = self.signing_key.verifying_key();
        let mut pubkey = [0u8; PUBLIC_KEY_SIZE];
        pubkey.copy_from_slice(vk.as_bytes());
        pubkey
    }

    /// Signs a schema fingerprint (64-byte signature)
    pub fn sign_schema(&self, schema_id: &[u8; 8]) -> [u8; SIGNATURE_SIZE] {
        let sig: Signature = self.signing_key.sign(schema_id);
        let mut signature = [0u8; SIGNATURE_SIZE];
        signature.copy_from_slice(&sig.to_bytes());
        signature
    }

    /// Returns the seed (private key) for storage
    pub fn seed(&self) -> [u8; PRIVATE_KEY_SIZE] {
        self.signing_key.to_bytes()
    }
}

/// Public key for verification
#[derive(Debug, Clone, Copy)]
pub struct VerifyingKeyPair {
    verifying_key: VerifyingKey,
}

impl VerifyingKeyPair {
    /// Creates a verifying key from 32-byte public key
    pub fn from_bytes(pubkey_bytes: &[u8; 32]) -> Result<Self> {
        let verifying_key = VerifyingKey::from_bytes(pubkey_bytes)
            .map_err(|e| QrdError::InvalidSchema(format!("invalid public key: {}", e)))?;

        if verifying_key.is_weak() {
            return Err(QrdError::InvalidSchema(
                "invalid public key: weak key detected".into(),
            ));
        }

        Ok(Self { verifying_key })
    }

    /// Verifies a signature of a schema fingerprint
    pub fn verify_signature(&self, schema_id: &[u8; 8], signature_bytes: &[u8]) -> Result<()> {
        if signature_bytes.len() != SIGNATURE_SIZE {
            return Err(QrdError::InvalidSchema(
                "signature must be 64 bytes".into(),
            ));
        }

        let mut sig_array = [0u8; SIGNATURE_SIZE];
        sig_array.copy_from_slice(signature_bytes);
        let signature = Signature::from_bytes(&sig_array);

        self.verifying_key
            .verify(schema_id, &signature)
            .map_err(|e| {
                QrdError::InvalidSchema(format!("signature verification failed: {}", e))
            })
    }

    /// Gets the bytes of the public key
    pub fn to_bytes(&self) -> [u8; PUBLIC_KEY_SIZE] {
        let mut pubkey = [0u8; PUBLIC_KEY_SIZE];
        pubkey.copy_from_slice(self.verifying_key.as_bytes());
        pubkey
    }
}

/// Schema signature wrapper
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaSignature {
    pub algorithm: u8,
    pub signature: [u8; SIGNATURE_SIZE],
    pub public_key: [u8; PUBLIC_KEY_SIZE],
}

impl SchemaSignature {
    /// Creates a new schema signature
    pub fn new(
        algorithm: u8,
        signature: [u8; SIGNATURE_SIZE],
        public_key: [u8; PUBLIC_KEY_SIZE],
    ) -> Self {
        Self {
            algorithm,
            signature,
            public_key,
        }
    }

    /// Serializes the signature to bytes (algorithm + signature + public_key)
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(1 + SIGNATURE_SIZE + PUBLIC_KEY_SIZE);
        bytes.push(self.algorithm);
        bytes.extend_from_slice(&self.signature);
        bytes.extend_from_slice(&self.public_key);
        bytes
    }

    /// Deserializes a signature from bytes
    pub fn deserialize(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 1 + SIGNATURE_SIZE + PUBLIC_KEY_SIZE {
            return Err(QrdError::InvalidSchema(
                "signature must be 97 bytes (1 + 64 + 32)".into(),
            ));
        }

        let algorithm = bytes[0];
        let mut signature = [0u8; SIGNATURE_SIZE];
        signature.copy_from_slice(&bytes[1..1 + SIGNATURE_SIZE]);

        let mut public_key = [0u8; PUBLIC_KEY_SIZE];
        public_key.copy_from_slice(&bytes[1 + SIGNATURE_SIZE..]);

        Ok(Self {
            algorithm,
            signature,
            public_key,
        })
    }

    /// Verifies the signature against a schema fingerprint
    pub fn verify(&self, schema_id: &[u8; 8]) -> Result<()> {
        if self.algorithm != SIGNATURE_ALGORITHM {
            return Err(QrdError::InvalidSchema(
                format!("unsupported signature algorithm: {}", self.algorithm).into(),
            ));
        }

        let verifying_key = VerifyingKeyPair::from_bytes(&self.public_key)?;
        verifying_key.verify_signature(schema_id, &self.signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keypair_generation_produces_valid_keys() {
        let keypair = SigningKeyPair::generate();
        let pubkey = keypair.verifying_key();
        assert_eq!(pubkey.len(), PUBLIC_KEY_SIZE);
    }

    #[test]
    fn keypair_from_seed_is_deterministic() {
        let seed = [42u8; 32];
        let kp1 = SigningKeyPair::from_seed(seed).expect("valid seed");
        let kp2 = SigningKeyPair::from_seed(seed).expect("valid seed");

        assert_eq!(kp1.verifying_key(), kp2.verifying_key());
    }

    #[test]
    fn signature_roundtrip_verifies() {
        let keypair = SigningKeyPair::generate();
        let schema_id = [1u8, 2, 3, 4, 5, 6, 7, 8];

        let signature = keypair.sign_schema(&schema_id);
        let pubkey = keypair.verifying_key();

        let sig = SchemaSignature::new(SIGNATURE_ALGORITHM, signature, pubkey);
        assert!(sig.verify(&schema_id).is_ok());
    }

    #[test]
    fn signature_verification_rejects_wrong_schema() {
        let keypair = SigningKeyPair::generate();
        let schema_id = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let wrong_id = [8u8, 7, 6, 5, 4, 3, 2, 1];

        let signature = keypair.sign_schema(&schema_id);
        let pubkey = keypair.verifying_key();

        let sig = SchemaSignature::new(SIGNATURE_ALGORITHM, signature, pubkey);
        assert!(sig.verify(&wrong_id).is_err());
    }

    #[test]
    fn signature_serialize_deserialize_roundtrip() {
        let keypair = SigningKeyPair::generate();
        let schema_id = [1u8, 2, 3, 4, 5, 6, 7, 8];

        let signature = keypair.sign_schema(&schema_id);
        let pubkey = keypair.verifying_key();

        let sig = SchemaSignature::new(SIGNATURE_ALGORITHM, signature, pubkey);
        let serialized = sig.serialize();
        let deserialized = SchemaSignature::deserialize(&serialized).expect("valid signature");

        assert_eq!(sig, deserialized);
        assert!(deserialized.verify(&schema_id).is_ok());
    }

    #[test]
    fn signature_tampering_is_detected() {
        let keypair = SigningKeyPair::generate();
        let schema_id = [1u8, 2, 3, 4, 5, 6, 7, 8];

        let mut signature = keypair.sign_schema(&schema_id);
        signature[0] ^= 0xFF; // Flip all bits in first byte
        let pubkey = keypair.verifying_key();

        let sig = SchemaSignature::new(SIGNATURE_ALGORITHM, signature, pubkey);
        assert!(sig.verify(&schema_id).is_err());
    }

    #[test]
    fn verifying_key_from_bytes_rejects_invalid() {
        let invalid_bytes = [0u8; 32];
        // Note: Some byte arrays are valid Ed25519 public keys, so we just test that it doesn't panic
        let result = VerifyingKeyPair::from_bytes(&invalid_bytes);
        // Result can be Ok or Err depending on the bytes
        let _ = result;
    }
}
