use qrd_core::compression::{compress, decompress, CompressionKind};
use qrd_core::encryption::{
    decrypt_payload, derive_column_key, encrypt_payload, AuthTag, EncryptionConfig, Nonce,
};
use qrd_core::parser::{parse_footer, parse_footer_length, parse_header, FileHeader};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct HeaderInfo {
    format_major: u16,
    format_minor: u16,
    schema_id: Vec<u8>,
    flags: u16,
    writer_version: Vec<u8>,
}

#[wasm_bindgen]
impl HeaderInfo {
    #[wasm_bindgen(getter)]
    pub fn format_major(&self) -> u16 {
        self.format_major
    }

    #[wasm_bindgen(getter)]
    pub fn format_minor(&self) -> u16 {
        self.format_minor
    }

    #[wasm_bindgen(getter)]
    pub fn schema_id(&self) -> Vec<u8> {
        self.schema_id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn flags(&self) -> u16 {
        self.flags
    }

    #[wasm_bindgen(getter)]
    pub fn writer_version(&self) -> Vec<u8> {
        self.writer_version.clone()
    }
}

#[wasm_bindgen]
pub struct EncryptedChunk {
    nonce: Vec<u8>,
    auth_tag: Vec<u8>,
    ciphertext: Vec<u8>,
}

#[wasm_bindgen]
impl EncryptedChunk {
    #[wasm_bindgen(getter)]
    pub fn nonce(&self) -> Vec<u8> {
        self.nonce.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn auth_tag(&self) -> Vec<u8> {
        self.auth_tag.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn ciphertext(&self) -> Vec<u8> {
        self.ciphertext.clone()
    }
}

#[wasm_bindgen]
pub fn init_wasm() -> bool {
    true
}

#[wasm_bindgen]
pub fn inspect_header(bytes: &[u8]) -> Result<HeaderInfo, JsValue> {
    parse_header(bytes)
        .map(|header| HeaderInfo {
            format_major: header.format_major,
            format_minor: header.format_minor,
            schema_id: header.schema_id.to_vec(),
            flags: header.flags,
            writer_version: header.writer_version.to_vec(),
        })
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

#[wasm_bindgen]
pub fn inspect_footer_length(bytes: &[u8]) -> Result<u32, JsValue> {
    parse_footer_length(bytes).map_err(|err| JsValue::from_str(&err.to_string()))
}

#[wasm_bindgen]
pub fn inspect_footer_bytes(bytes: &[u8]) -> Result<u32, JsValue> {
    parse_footer(bytes)
        .map(|footer| footer.row_group_count)
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

#[wasm_bindgen]
pub fn serialize_header(
    format_major: u16,
    format_minor: u16,
    schema_id: &[u8],
    flags: u16,
) -> Result<Vec<u8>, JsValue> {
    if schema_id.len() != 8 {
        return Err(JsValue::from_str("schema_id must be 8 bytes"));
    }

    let mut schema_id_array = [0u8; 8];
    schema_id_array.copy_from_slice(schema_id);

    let header = FileHeader::new(
        format_major,
        format_minor,
        schema_id_array,
        flags,
        *b"qrd-0.1.0\0\0\0",
    );
    Ok(header.serialize().to_vec())
}

#[wasm_bindgen]
pub fn compress_zstd(payload: &[u8]) -> Result<Vec<u8>, JsValue> {
    compress(payload, CompressionKind::Zstd).map_err(|err| JsValue::from_str(&err.to_string()))
}

#[wasm_bindgen]
pub fn decompress_zstd(payload: &[u8]) -> Result<Vec<u8>, JsValue> {
    decompress(payload, CompressionKind::Zstd).map_err(|err| JsValue::from_str(&err.to_string()))
}

#[wasm_bindgen]
pub fn compress_lz4(payload: &[u8]) -> Result<Vec<u8>, JsValue> {
    compress(payload, CompressionKind::Lz4).map_err(|err| JsValue::from_str(&err.to_string()))
}

#[wasm_bindgen]
pub fn decompress_lz4(payload: &[u8]) -> Result<Vec<u8>, JsValue> {
    decompress(payload, CompressionKind::Lz4).map_err(|err| JsValue::from_str(&err.to_string()))
}

#[wasm_bindgen]
pub fn derive_key(
    master_key: &[u8],
    column_name: &str,
    schema_fingerprint: &[u8],
) -> Result<Vec<u8>, JsValue> {
    if schema_fingerprint.len() != 8 {
        return Err(JsValue::from_str("schema_fingerprint must be 8 bytes"));
    }
    let mut fingerprint = [0u8; 8];
    fingerprint.copy_from_slice(schema_fingerprint);

    let config = EncryptionConfig {
        column_name: column_name.to_string(),
        schema_fingerprint: fingerprint,
    };

    derive_column_key(master_key, &config)
        .map(|key| key.to_vec())
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

#[wasm_bindgen]
pub fn encrypt(payload: &[u8], key: &[u8]) -> Result<EncryptedChunk, JsValue> {
    if key.len() != 32 {
        return Err(JsValue::from_str("key must be 32 bytes"));
    }
    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(key);

    encrypt_payload(payload, &key_array)
        .map(|chunk| EncryptedChunk {
            nonce: chunk.nonce.0.to_vec(),
            auth_tag: chunk.auth_tag.0.to_vec(),
            ciphertext: chunk.ciphertext,
        })
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

#[wasm_bindgen]
pub fn decrypt(
    ciphertext: &[u8],
    key: &[u8],
    nonce: &[u8],
    auth_tag: &[u8],
) -> Result<Vec<u8>, JsValue> {
    if key.len() != 32 {
        return Err(JsValue::from_str("key must be 32 bytes"));
    }
    if nonce.len() != 12 {
        return Err(JsValue::from_str("nonce must be 12 bytes"));
    }
    if auth_tag.len() != 16 {
        return Err(JsValue::from_str("auth_tag must be 16 bytes"));
    }

    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(key);
    let mut nonce_array = [0u8; 12];
    nonce_array.copy_from_slice(nonce);
    let mut auth_tag_array = [0u8; 16];
    auth_tag_array.copy_from_slice(auth_tag);

    decrypt_payload(
        ciphertext,
        &key_array,
        &Nonce(nonce_array),
        &AuthTag(auth_tag_array),
    )
    .map_err(|err| JsValue::from_str(&err.to_string()))
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
        assert_eq!(parsed.format_major(), 1);
        assert_eq!(parsed.schema_id(), vec![9, 8, 7, 6, 5, 4, 3, 2]);
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
        let serialized = serialize_header(1, 0, &[1, 2, 3, 4, 5, 6, 7, 8], 0).unwrap();
        assert_eq!(serialized.len(), qrd_core::parser::HEADER_SIZE);
        assert_eq!(serialized[0], 0x51);
        assert_eq!(serialized[1], 0x52);
        assert_eq!(serialized[2], 0x44);
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

        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&key);

        let plaintext = b"sensitive data";
        let encrypted = encrypt(plaintext, &key_array).expect("encryption should work");

        let mut nonce_array = [0u8; 12];
        nonce_array.copy_from_slice(&encrypted.nonce());
        let mut auth_tag_array = [0u8; 16];
        auth_tag_array.copy_from_slice(&encrypted.auth_tag());

        let decrypted = decrypt(
            &encrypted.ciphertext(),
            &key_array,
            &nonce_array,
            &auth_tag_array,
        )
        .expect("decryption should work");
        assert_eq!(decrypted, plaintext);
    }
}
