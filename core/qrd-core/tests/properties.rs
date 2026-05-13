// Property-based tests for QRD core engine.
// These tests verify invariants across all encoding, compression, and encryption algorithms.

use qrd_core::compression::{compress, decompress, CompressionKind};
use qrd_core::encoding::{decode, encode, EncodingId};
use qrd_core::encryption::{decrypt_payload, derive_column_key, encrypt_payload, EncryptionConfig};
use qrd_core::parser::{parse_header, FileHeader};
use qrd_core::schema::{FieldKind, SchemaBuilder};

#[test]
fn encoding_plain_roundtrip_property() {
    let samples = vec![
        vec![],
        vec![0],
        vec![0, 0, 0],
        vec![1, 2, 3, 4, 5],
        vec![255; 100],
        (0..=255).collect::<Vec<u8>>(),
    ];

    for sample in samples {
        let encoded = encode(&sample, EncodingId::Plain).expect("encoding should work");
        let decoded = decode(&encoded, EncodingId::Plain).expect("decoding should work");
        assert_eq!(
            decoded, sample,
            "PLAIN encoding roundtrip failed for {:?}",
            sample
        );
    }
}

#[test]
fn encoding_rle_roundtrip_property() {
    let samples = vec![
        vec![0],
        vec![0, 0, 0],
        vec![1, 1, 2, 2, 2, 3],
        vec![1, 2, 3, 4, 5],
        vec![255; 1000],
        vec![0, 1, 0, 1, 0, 1],
    ];

    for sample in samples {
        let encoded = encode(&sample, EncodingId::Rle).expect("encoding should work");
        let decoded = decode(&encoded, EncodingId::Rle).expect("decoding should work");
        assert_eq!(
            decoded, sample,
            "RLE encoding roundtrip failed for {:?}",
            sample
        );
    }
}

#[test]
fn encoding_bit_packed_roundtrip_property() {
    let samples = vec![
        vec![0],
        vec![15],
        vec![0, 15],
        (0..=15).cycle().take(100).collect::<Vec<u8>>(),
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    ];

    for sample in samples {
        let encoded = encode(&sample, EncodingId::BitPacked).expect("encoding should work");
        let decoded = decode(&encoded, EncodingId::BitPacked).expect("decoding should work");
        assert_eq!(
            decoded, sample,
            "BIT_PACKED encoding roundtrip failed for {:?}",
            sample
        );
    }
}

#[test]
fn encoding_delta_binary_roundtrip_property() {
    let samples = vec![
        vec![],
        vec![0],
        vec![0, 1, 2, 3, 4, 5],
        vec![100, 101, 102, 103],
        vec![0, 0, 0, 0],
        vec![255, 0, 255, 0],
    ];

    for sample in samples {
        let encoded = encode(&sample, EncodingId::DeltaBinary).expect("encoding should work");
        let decoded = decode(&encoded, EncodingId::DeltaBinary).expect("decoding should work");
        assert_eq!(
            decoded, sample,
            "DELTA_BINARY encoding roundtrip failed for {:?}",
            sample
        );
    }
}

#[test]
fn encoding_delta_byte_array_roundtrip_property() {
    let samples = vec![
        vec![],
        vec![0],
        vec![0, 1, 2, 3, 4, 5],
        vec![100, 101, 102, 103],
        vec![0, 0, 0, 0],
        vec![255, 0, 255, 0],
    ];

    for sample in samples {
        let encoded = encode(&sample, EncodingId::DeltaByteArray).expect("encoding should work");
        let decoded = decode(&encoded, EncodingId::DeltaByteArray).expect("decoding should work");
        assert_eq!(
            decoded, sample,
            "DELTA_BYTE_ARRAY encoding roundtrip failed for {:?}",
            sample
        );
    }
}

#[test]
fn encoding_byte_stream_split_roundtrip_property() {
    let samples = vec![
        vec![0],
        vec![255],
        vec![1, 2, 3, 4, 5],
        vec![0, 85, 170, 255],
        (0..=255).cycle().take(100).collect::<Vec<u8>>(),
    ];

    for sample in samples {
        let encoded = encode(&sample, EncodingId::ByteStreamSplit).expect("encoding should work");
        let decoded = decode(&encoded, EncodingId::ByteStreamSplit).expect("decoding should work");
        assert_eq!(
            decoded, sample,
            "BYTE_STREAM_SPLIT encoding roundtrip failed for {:?}",
            sample
        );
    }
}

#[test]
fn encoding_dict_rle_roundtrip_property() {
    let samples = vec![
        vec![0],
        vec![0, 0, 0],
        vec![1, 1, 2, 2, 2],
        vec![5, 5, 10, 10, 15],
        (0..10).cycle().take(100).collect::<Vec<u8>>(),
    ];

    for sample in samples {
        let encoded = encode(&sample, EncodingId::DictRle).expect("encoding should work");
        let decoded = decode(&encoded, EncodingId::DictRle).expect("decoding should work");
        assert_eq!(
            decoded, sample,
            "DICT_RLE encoding roundtrip failed for {:?}",
            sample
        );
    }
}

#[test]
fn compression_zstd_roundtrip_property() {
    let samples = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"hello world".to_vec(),
        b"the quick brown fox jumps over the lazy dog".to_vec(),
        vec![0u8; 1000],
        (0..=255).cycle().take(2000).collect::<Vec<u8>>(),
    ];

    for sample in samples {
        let compressed = compress(&sample, CompressionKind::Zstd).expect("compression should work");
        let decompressed =
            decompress(&compressed, CompressionKind::Zstd).expect("decompression should work");
        assert_eq!(
            decompressed,
            sample,
            "ZSTD roundtrip failed for {} bytes",
            sample.len()
        );
    }
}

#[test]
fn compression_lz4_roundtrip_property() {
    let samples = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"hello world".to_vec(),
        b"the quick brown fox jumps over the lazy dog".to_vec(),
        vec![0u8; 1000],
        (0..=255).cycle().take(2000).collect::<Vec<u8>>(),
    ];

    for sample in samples {
        let compressed = compress(&sample, CompressionKind::Lz4).expect("compression should work");
        let decompressed =
            decompress(&compressed, CompressionKind::Lz4).expect("decompression should work");
        assert_eq!(
            decompressed,
            sample,
            "LZ4 roundtrip failed for {} bytes",
            sample.len()
        );
    }
}

#[test]
fn encryption_roundtrip_property() {
    let master_key = b"super-secret-key-for-testing";
    let config = EncryptionConfig {
        column_name: "test_column".to_string(),
        schema_fingerprint: [1, 2, 3, 4, 5, 6, 7, 8],
    };
    let key = derive_column_key(master_key, &config).expect("key derivation should work");

    let samples = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"hello world".to_vec(),
        b"the quick brown fox jumps over the lazy dog".to_vec(),
        vec![0u8; 1000],
        (0..=255).cycle().take(2000).collect::<Vec<u8>>(),
    ];

    for sample in samples {
        let encrypted = encrypt_payload(&sample, &key).expect("encryption should work");
        let decrypted = decrypt_payload(
            &encrypted.ciphertext,
            &key,
            &encrypted.nonce,
            &encrypted.auth_tag,
        )
        .expect("decryption should work");
        assert_eq!(
            decrypted,
            sample,
            "AES-256-GCM roundtrip failed for {} bytes",
            sample.len()
        );
    }
}

#[test]
fn schema_fingerprint_deterministic() {
    let schema1 = SchemaBuilder::new()
        .add_field("device_id", FieldKind::Utf8, true)
        .add_field("temperature", FieldKind::Float32, false)
        .build()
        .expect("schema should build");

    let schema2 = SchemaBuilder::new()
        .add_field("device_id", FieldKind::Utf8, true)
        .add_field("temperature", FieldKind::Float32, false)
        .build()
        .expect("schema should build");

    assert_eq!(
        schema1.fingerprint(),
        schema2.fingerprint(),
        "identical schemas should have identical fingerprints"
    );
}

#[test]
fn file_header_roundtrip_property() {
    let headers = vec![
        FileHeader::new(1, 0, [1, 2, 3, 4, 5, 6, 7, 8], 0, *b"qrd-0.1.0\0\0\0"),
        FileHeader::new(
            1,
            1,
            [255, 255, 255, 255, 255, 255, 255, 255],
            0xFFFF,
            *b"qrd-0.1.1\0\0\0",
        ),
    ];

    for header in headers {
        let bytes = header.serialize();
        let parsed = parse_header(&bytes).expect("header should parse");
        assert_eq!(parsed, header, "header roundtrip failed");
    }
}
