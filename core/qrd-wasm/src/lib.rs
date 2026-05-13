use qrd_core::compression::{compress, decompress, CompressionKind};
use qrd_core::encryption::{derive_column_key, encrypt_payload, decrypt_payload, EncryptionConfig};
use qrd_core::parser::{parse_footer, parse_footer_length, parse_header, FileFooter, FileHeader};

// ============================================================================
// INITIALIZATION
// ============================================================================

/// Initializes the WASM layer. Must be called before other operations.
pub fn init_wasm() -> bool {
    true
}

// ============================================================================
// HEADER/FOOTER INSPECTION
// ============================================================================

/// Inspects a raw header buffer without mutating it.
pub fn inspect_header(bytes: &[u8]) -> Option<FileHeader> {
    parse_header(bytes).ok()
}

/// Inspects a raw footer-length trailer without touching payload bytes.
pub fn inspect_footer_length(bytes: &[u8]) -> Option<u32> {
    parse_footer_length(bytes).ok()
}

/// Inspects a canonical footer without touching payload bytes.
pub fn inspect_footer_bytes(bytes: &[u8]) -> Option<FileFooter> {
    parse_footer(bytes).ok()
}

/// Serializes a header to canonical bytes.
pub fn serialize_header(
    format_major: u16,
    format_minor: u16,
    schema_id: &[u8; 8],
    flags: u16,
) -> Vec<u8> {
    let header = FileHeader::new(format_major, format_minor, *schema_id, flags, *b"qrd-0.1.0\0\0\0");
    header.serialize().to_vec()
}

// ============================================================================
// COMPRESSION
// ============================================================================

/// Compresses a payload using Zstandard.
pub fn compress_zstd(payload: &[u8]) -> Result<Vec<u8>, String> {
    compress(payload, CompressionKind::Zstd)
        .map_err(|e| format!("ZSTD compression failed: {}", e))
}

/// Decompresses a Zstandard-compressed payload.
pub fn decompress_zstd(payload: &[u8]) -> Result<Vec<u8>, String> {
    decompress(payload, CompressionKind::Zstd)
        .map_err(|e| format!("ZSTD decompression failed: {}", e))
}

/// Compresses a payload using LZ4.
pub fn compress_lz4(payload: &[u8]) -> Result<Vec<u8>, String> {
    compress(payload, CompressionKind::Lz4)
        .map_err(|e| format!("LZ4 compression failed: {}", e))
}

/// Decompresses an LZ4-compressed payload.
pub fn decompress_lz4(payload: &[u8]) -> Result<Vec<u8>, String> {
    decompress(payload, CompressionKind::Lz4)
        .map_err(|e| format!("LZ4 decompression failed: {}", e))
}

// ============================================================================
// ENCRYPTION (AES-256-GCM + HKDF-SHA256)
// ============================================================================

/// Derives a 32-byte column key using HKDF-SHA256.
pub fn derive_key(
    master_key: &[u8],
    column_name: &str,
    schema_fingerprint: &[u8; 8],
) -> Result<Vec<u8>, String> {
    let config = EncryptionConfig {
        column_name: column_name.to_string(),
        schema_fingerprint: *schema_fingerprint,
    };

    derive_column_key(master_key, &config)
        .map(|key| key.to_vec())
        .map_err(|e| format!("Key derivation failed: {}", e))
}

/// Encrypts a payload using AES-256-GCM.
/// Returns (nonce, auth_tag, ciphertext) on success.
pub fn encrypt(payload: &[u8], key: &[u8; 32]) -> Result<EncryptedChunk, String> {
    encrypt_payload(payload, key)
        .map(|chunk| EncryptedChunk {
            nonce: chunk.nonce.0.to_vec(),
            auth_tag: chunk.auth_tag.0.to_vec(),
            ciphertext: chunk.ciphertext,
        })
        .map_err(|e| format!("Encryption failed: {}", e))
}

/// Decrypts a payload using AES-256-GCM.
pub fn decrypt(
    ciphertext: &[u8],
    key: &[u8; 32],
    nonce: &[u8; 12],
    auth_tag: &[u8; 16],
) -> Result<Vec<u8>, String> {
    use qrd_core::encryption::Nonce as QrdNonce;
    use qrd_core::encryption::AuthTag as QrdAuthTag;

    let nonce_wrapper = QrdNonce(*nonce);
    let auth_tag_wrapper = QrdAuthTag(*auth_tag);

    decrypt_payload(ciphertext, key, &nonce_wrapper, &auth_tag_wrapper)
        .map_err(|e| format!("Decryption failed: {}", e))
}

// ============================================================================
// DATA STRUCTURES FOR WASM
// ============================================================================

/// Encrypted chunk envelope (used for WASM serialization)
#[derive(Debug, Clone)]
pub struct EncryptedChunk {
    pub nonce: Vec<u8>,
    pub auth_tag: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_layer_initializes() {
        assert!(init_wasm());
    }

    #[test]
    fn wasm_header_inspection_roundtrips() {
        let header = qrd_core::parser::FileHeader::new(
            1,
            0,
            [9, 8, 7, 6, 5, 4, 3, 2],
            0b1010,
            *b"qrd-0.1.0\0\0\0",
        );
        let bytes = header.serialize();

        let parsed = inspect_header(&bytes).expect("header should parse");
        assert_eq!(parsed, header);
    }

    #[test]
    fn wasm_footer_length_inspection_roundtrips() {
        let mut bytes = vec![1u8, 2, 3, 4];
        bytes.extend_from_slice(&0x1122_3344u32.to_le_bytes());

        let parsed = inspect_footer_length(&bytes).expect("footer length should parse");
        assert_eq!(parsed, 0x1122_3344);
    }

    #[test]
    fn wasm_serialize_header() {
        let serialized =
            serialize_header(1, 0, &[1, 2, 3, 4, 5, 6, 7, 8], 0);
        assert_eq!(serialized.len(), qrd_core::parser::HEADER_SIZE);
        assert_eq!(serialized[0], 0x51); // 'Q'
        assert_eq!(serialized[1], 0x52); // 'R'
        assert_eq!(serialized[2], 0x44); // 'D'
    }

    #[test]
    fn wasm_compression_roundtrip() {
        let payload = b"hello world this is a test";
        let compressed = compress_zstd(payload).expect("compression should work");
        let decompressed = decompress_zstd(&compressed).expect("decompression should work");
        assert_eq!(decompressed, payload);
    }

    #[test]
    fn wasm_encryption_roundtrip() {
        let master_key = b"super-secret-key";
        let key = derive_key(master_key, "test_col", &[1, 2, 3, 4, 5, 6, 7, 8])
            .expect("key derivation should work");

        // Convert to array
        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&key);

        let plaintext = b"sensitive data";
        let encrypted = encrypt(plaintext, &key_array).expect("encryption should work");

        // Convert back from Vec to array
        let mut nonce_array = [0u8; 12];
        nonce_array.copy_from_slice(&encrypted.nonce);
        let mut auth_tag_array = [0u8; 16];
        auth_tag_array.copy_from_slice(&encrypted.auth_tag);

        let decrypted =
            decrypt(&encrypted.ciphertext, &key_array, &nonce_array, &auth_tag_array)
                .expect("decryption should work");
        assert_eq!(decrypted, plaintext);
    }
}
