use crate::error::{QrdError, Result};

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

/// Derives a placeholder 32-byte column key for the scaffold.
///
/// The Phase 1 implementation will replace this with HKDF-SHA256 + AES-GCM.
pub fn derive_column_key(master_key: &[u8], config: &EncryptionConfig) -> Result<[u8; 32]> {
    if master_key.is_empty() {
        return Err(QrdError::InvalidSchema("master key cannot be empty".into()));
    }

    let mut key = [0u8; 32];
    for (index, byte) in master_key.iter().enumerate() {
        key[index % 32] = key[index % 32]
            .wrapping_add(*byte)
            .wrapping_add(config.schema_fingerprint[index % 8]);
    }
    for (index, byte) in config.column_name.as_bytes().iter().enumerate() {
        key[index % 32] ^= *byte;
    }
    Ok(key)
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

/// Encrypts a payload in the scaffold.
pub fn encrypt_payload(payload: &[u8], _key: &[u8; 32]) -> Result<Vec<u8>> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }
    Err(QrdError::NotImplemented("AES-256-GCM"))
}

/// Decrypts a payload in the scaffold.
pub fn decrypt_payload(payload: &[u8], _key: &[u8; 32]) -> Result<Vec<u8>> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }
    Err(QrdError::NotImplemented("AES-256-GCM"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
