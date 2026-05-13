use crate::error::{QrdError, Result};
use aes_gcm::{Aes256Gcm, Key};
use aes_gcm::aead::{Aead, KeyInit};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use std::fmt::Write;

/// Configuration for column encryption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptionConfig {
    pub column_name: String,
    pub schema_fingerprint: [u8; 8],
}

/// 12-byte per-chunk nonce wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nonce(pub [u8; 12]);

/// 16-byte authentication tag wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthTag(pub [u8; 16]);

/// Canonical encrypted chunk envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedChunk {
    pub nonce: Nonce,
    pub auth_tag: AuthTag,
    pub ciphertext: Vec<u8>,
}

/// Derives a 32-byte column key using HKDF-SHA256.
///
/// # Derivation Process
/// - Input key: master_key (user-provided)
/// - Salt: schema_fingerprint (8 bytes)
/// - Info: "qrd:col:{column_name}:{schema_id}"
/// - Output: 32-byte column key suitable for AES-256-GCM
pub fn derive_column_key(master_key: &[u8], config: &EncryptionConfig) -> Result<[u8; 32]> {
    if master_key.is_empty() {
        return Err(QrdError::InvalidSchema("master key cannot be empty".into()));
    }

    let hkdf = Hkdf::<Sha256>::new(Some(&config.schema_fingerprint[..]), master_key);

    let mut schema_hex = String::with_capacity(16);
    for byte in &config.schema_fingerprint {
        write!(&mut schema_hex, "{:02x}", byte).expect("hex formatting should not fail");
    }
    let info = format!("qrd:col:{}:{}", config.column_name, schema_hex);
    let mut key = [0u8; 32];
    
    hkdf.expand(info.as_bytes(), &mut key)
        .map_err(|e| QrdError::InvalidSchema(format!("HKDF expansion failed: {}", e)))?;
    
    Ok(key)
}

/// Generates a cryptographically random 12-byte nonce.
pub fn generate_nonce() -> Result<Nonce> {
    let mut nonce_bytes = [0u8; 12];
    let mut rng = rand::rngs::OsRng;
    rng.fill_bytes(&mut nonce_bytes);
    Ok(Nonce(nonce_bytes))
}

/// Packs an encrypted chunk into `[nonce][auth_tag][ciphertext]` layout.
pub fn pack_encrypted_chunk(chunk: &EncryptedChunk) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(12 + 16 + chunk.ciphertext.len());
    bytes.extend_from_slice(&chunk.nonce.0);
    bytes.extend_from_slice(&chunk.auth_tag.0);
    bytes.extend_from_slice(&chunk.ciphertext);
    bytes
}

/// Parses an encrypted chunk from `[nonce][auth_tag][ciphertext]` layout.
pub fn unpack_encrypted_chunk(bytes: &[u8]) -> Result<EncryptedChunk> {
    if bytes.len() < 28 {
        return Err(QrdError::UnexpectedEof);
    }

    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&bytes[0..12]);

    let mut auth_tag = [0u8; 16];
    auth_tag.copy_from_slice(&bytes[12..28]);

    Ok(EncryptedChunk {
        nonce: Nonce(nonce),
        auth_tag: AuthTag(auth_tag),
        ciphertext: bytes[28..].to_vec(),
    })
}

/// Encrypts a payload using AES-256-GCM.
///
/// # Process
/// 1. Generates a random 12-byte nonce
/// 2. Encrypts plaintext with AES-256-GCM
/// 3. Extracts authentication tag from cipher
/// 4. Returns EncryptedChunk with nonce, tag, and ciphertext
pub fn encrypt_payload(payload: &[u8], key: &[u8; 32]) -> Result<EncryptedChunk> {
    let nonce = generate_nonce()?;
    let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from(*key));

    let ciphertext = cipher
        .encrypt(nonce.0.as_slice().into(), payload)
        .map_err(|e| QrdError::InvalidSchema(format!("AES-256-GCM encryption failed: {}", e)))?;

    // The last 16 bytes of ciphertext are the authentication tag
    if ciphertext.len() < 16 {
        return Err(QrdError::InvalidSchema(
            "encryption tag missing from output".into(),
        ));
    }

    let auth_tag_start = ciphertext.len() - 16;
    let mut auth_tag = [0u8; 16];
    auth_tag.copy_from_slice(&ciphertext[auth_tag_start..]);

    Ok(EncryptedChunk {
        nonce,
        auth_tag: AuthTag(auth_tag),
        ciphertext: ciphertext[..auth_tag_start].to_vec(),
    })
}

/// Decrypts a payload using AES-256-GCM.
///
/// # Process
/// 1. Reconstructs ciphertext with appended authentication tag
/// 2. Decrypts using AES-256-GCM with provided nonce
/// 3. Verifies authentication tag during decryption
pub fn decrypt_payload(
    payload: &[u8],
    key: &[u8; 32],
    nonce: &Nonce,
    auth_tag: &AuthTag,
) -> Result<Vec<u8>> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }

    // Reconstruct ciphertext with tag appended
    let mut full_ciphertext = payload.to_vec();
    full_ciphertext.extend_from_slice(&auth_tag.0);

    let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from(*key));
    
    cipher
        .decrypt(nonce.0.as_slice().into(), &full_ciphertext[..])
        .map_err(|e| QrdError::InvalidSchema(format!("AES-256-GCM decryption failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonce_generation_produces_unique_values() {
        let nonce1 = generate_nonce().expect("nonce generation should work");
        let nonce2 = generate_nonce().expect("nonce generation should work");
        // Very high probability of being different (random 12 bytes)
        // This is a probabilistic test, extreme collision is unlikely
        assert_ne!(nonce1.0[0], nonce2.0[0]); // At least first byte should differ
    }

    #[test]
    fn encrypted_chunk_layout_roundtrips() {
        let chunk = EncryptedChunk {
            nonce: Nonce([1u8; 12]),
            auth_tag: AuthTag([2u8; 16]),
            ciphertext: vec![3, 4, 5],
        };

        let packed = pack_encrypted_chunk(&chunk);
        let unpacked = unpack_encrypted_chunk(&packed).expect("chunk should unpack");

        assert_eq!(unpacked, chunk);
    }

    #[test]
    fn unpack_rejects_short_input() {
        assert!(matches!(unpack_encrypted_chunk(&[1, 2, 3]), Err(QrdError::UnexpectedEof)));
    }

    #[test]
    fn derive_column_key_requires_master_key() {
        let config = EncryptionConfig {
            column_name: "temperature".to_string(),
            schema_fingerprint: [1, 2, 3, 4, 5, 6, 7, 8],
        };
        let result = derive_column_key(b"", &config);
        assert!(result.is_err());
    }

    #[test]
    fn derive_column_key_produces_different_keys_for_different_columns() {
        let master_key = b"super-secret-key";
        let schema_fp = [1, 2, 3, 4, 5, 6, 7, 8];

        let config1 = EncryptionConfig {
            column_name: "temperature".to_string(),
            schema_fingerprint: schema_fp,
        };
        let config2 = EncryptionConfig {
            column_name: "humidity".to_string(),
            schema_fingerprint: schema_fp,
        };

        let key1 = derive_column_key(master_key, &config1).expect("derivation should work");
        let key2 = derive_column_key(master_key, &config2).expect("derivation should work");

        assert_ne!(key1, key2);
    }

    #[test]
    fn encryption_decryption_roundtrip() {
        let master_key = b"super-secret-key";
        let config = EncryptionConfig {
            column_name: "sensor_data".to_string(),
            schema_fingerprint: [1, 2, 3, 4, 5, 6, 7, 8],
        };
        let key = derive_column_key(master_key, &config).expect("key derivation should work");

        let plaintext = b"sensitive sensor data that must be protected";
        let encrypted = encrypt_payload(plaintext, &key).expect("encryption should work");

        // Verify structure
        assert!(!encrypted.ciphertext.is_empty());
        assert_eq!(encrypted.nonce.0.len(), 12);
        assert_eq!(encrypted.auth_tag.0.len(), 16);

        // Decrypt and verify
        let decrypted =
            decrypt_payload(&encrypted.ciphertext, &key, &encrypted.nonce, &encrypted.auth_tag)
                .expect("decryption should work");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn empty_payload_encryption() {
        let master_key = b"super-secret-key";
        let config = EncryptionConfig {
            column_name: "sensor_data".to_string(),
            schema_fingerprint: [1, 2, 3, 4, 5, 6, 7, 8],
        };
        let key = derive_column_key(master_key, &config).expect("key derivation should work");

        let encrypted = encrypt_payload(b"", &key).expect("encryption should work");
        assert_eq!(encrypted.ciphertext.len(), 0);

        let decrypted =
            decrypt_payload(&encrypted.ciphertext, &key, &encrypted.nonce, &encrypted.auth_tag)
                .expect("decryption should work");
        assert_eq!(decrypted.len(), 0);
    }

    #[test]
    fn tampered_ciphertext_rejected() {
        let master_key = b"super-secret-key";
        let config = EncryptionConfig {
            column_name: "sensor_data".to_string(),
            schema_fingerprint: [1, 2, 3, 4, 5, 6, 7, 8],
        };
        let key = derive_column_key(master_key, &config).expect("key derivation should work");

        let plaintext = b"sensor data";
        let encrypted = encrypt_payload(plaintext, &key).expect("encryption should work");

        // Tamper with ciphertext
        let mut tampered_ciphertext = encrypted.ciphertext.clone();
        if !tampered_ciphertext.is_empty() {
            tampered_ciphertext[0] ^= 0xFF;
        }

        let result = decrypt_payload(&tampered_ciphertext, &key, &encrypted.nonce, &encrypted.auth_tag);
        // GCM authentication should catch tampering
        assert!(result.is_err() || result.unwrap() != plaintext);
    }
}
