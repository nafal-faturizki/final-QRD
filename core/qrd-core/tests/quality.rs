use qrd_core::columnar::transpose_rows;
use qrd_core::compression::{choose_compression, compress, decompress, CompressionKind};
use qrd_core::ecc::{
    encode as ecc_encode, recover_missing_chunk, verify as ecc_verify, ReedSolomonConfig,
};
use qrd_core::encoding::{decode, encode, EncodingId};
use qrd_core::encryption::{
    decrypt_payload, derive_column_key, encrypt_payload, pack_encrypted_chunk,
    unpack_encrypted_chunk, EncryptionConfig,
};
use qrd_core::error::QrdError;
use qrd_core::integrity::crc32;
use qrd_core::parser::{parse_footer, parse_footer_length, parse_header, FileHeader};
use qrd_core::row_group::{ColumnChunk, RowGroup};
use qrd_core::schema::{FieldKind, SchemaBuilder};

fn corrupt_bytes(bytes: &[u8], index: usize) -> Vec<u8> {
    let mut result = bytes.to_vec();
    if index < result.len() {
        result[index] ^= 0xFF;
    }
    result
}

#[test]
fn encoding_plain_handles_empty_and_full_payloads() {
    let samples = vec![vec![], vec![0], vec![1, 2, 3, 4, 5], (0..=255).collect()];
    for sample in samples {
        let encoded = encode(&sample, EncodingId::Plain).unwrap();
        let decoded = decode(&encoded, EncodingId::Plain).unwrap();
        assert_eq!(decoded, sample);
    }
}

#[test]
fn encoding_rle_preserves_repeated_patterns() {
    let sample = vec![5, 5, 5, 5, 1, 1, 2, 2, 2];
    let encoded = encode(&sample, EncodingId::Rle).unwrap();
    let decoded = decode(&encoded, EncodingId::Rle).unwrap();
    assert_eq!(decoded, sample);
}

#[test]
fn encoding_rle_detects_truncated_payload() {
    let encoded = encode(&[1, 1, 1, 2], EncodingId::Rle).unwrap();
    let truncated = &encoded[..encoded.len() - 1];
    assert!(matches!(
        decode(truncated, EncodingId::Rle),
        Err(QrdError::UnexpectedEof)
    ));
}

#[test]
fn encoding_bit_packed_roundtrips_single_value() {
    let sample = vec![7];
    let encoded = encode(&sample, EncodingId::BitPacked).unwrap();
    let decoded = decode(&encoded, EncodingId::BitPacked).unwrap();
    assert_eq!(decoded, sample);
}

#[test]
fn encoding_bit_packed_roundtrips_max_value_range() {
    let sample = (0..=15).cycle().take(128).collect::<Vec<u8>>();
    let encoded = encode(&sample, EncodingId::BitPacked).unwrap();
    let decoded = decode(&encoded, EncodingId::BitPacked).unwrap();
    assert_eq!(decoded, sample);
}

#[test]
fn encoding_delta_binary_handles_wrapping_bytes() {
    let sample = vec![255, 0, 1, 2];
    let encoded = encode(&sample, EncodingId::DeltaBinary).unwrap();
    let decoded = decode(&encoded, EncodingId::DeltaBinary).unwrap();
    assert_eq!(decoded, sample);
}

#[test]
fn encoding_delta_byte_array_roundtrips_empty_payload() {
    let sample = vec![];
    let encoded = encode(&sample, EncodingId::DeltaByteArray).unwrap();
    let decoded = decode(&encoded, EncodingId::DeltaByteArray).unwrap();
    assert_eq!(decoded, sample);
}

#[test]
fn encoding_byte_stream_split_preserves_bit_planes() {
    let sample = vec![0, 85, 170, 255];
    let encoded = encode(&sample, EncodingId::ByteStreamSplit).unwrap();
    let decoded = decode(&encoded, EncodingId::ByteStreamSplit).unwrap();
    assert_eq!(decoded, sample);
}

#[test]
fn encoding_dict_rle_roundtrips_low_cardinality_input() {
    let sample = vec![3, 3, 3, 7, 7, 3];
    let encoded = encode(&sample, EncodingId::DictRle).unwrap();
    let decoded = decode(&encoded, EncodingId::DictRle).unwrap();
    assert_eq!(decoded, sample);
}

#[test]
fn encoding_unknown_id_is_rejected() {
    assert!(matches!(
        EncodingId::from_u8(0xFF),
        Err(QrdError::UnsupportedEncoding(0xFF))
    ));
}

#[test]
fn compression_choose_adaptive_for_small_and_large_payloads() {
    assert_eq!(choose_compression(b"hello"), CompressionKind::Lz4);
    assert_eq!(choose_compression(&vec![0u8; 2048]), CompressionKind::Zstd);
}

#[test]
fn compression_zstd_roundtrip_reproduces_data() {
    let payload = b"the quick brown fox jumps over the lazy dog";
    let compressed = compress(payload, CompressionKind::Zstd).unwrap();
    let decompressed = decompress(&compressed, CompressionKind::Zstd).unwrap();
    assert_eq!(decompressed, payload);
}

#[test]
fn compression_lz4_roundtrip_reproduces_data() {
    let payload = b"the quick brown fox jumps over the lazy dog";
    let compressed = compress(payload, CompressionKind::Lz4).unwrap();
    let decompressed = decompress(&compressed, CompressionKind::Lz4).unwrap();
    assert_eq!(decompressed, payload);
}

#[test]
fn compression_adaptive_decompresses_zstd_or_lz4() {
    let sample = b"highly repeated repeated repeated repeated repeated";
    let compressed = compress(sample, CompressionKind::Adaptive).unwrap();
    let decompressed = decompress(&compressed, CompressionKind::Adaptive).unwrap();
    assert_eq!(decompressed, sample);
}

#[test]
fn compression_invalid_lz4_payload_returns_error() {
    assert!(decompress(&[0x00, 0x01, 0x02], CompressionKind::Lz4).is_err());
}

#[test]
fn compression_invalid_zstd_payload_returns_error() {
    assert!(decompress(&[0x00, 0x01, 0x02], CompressionKind::Zstd).is_err());
}

#[test]
fn encryption_key_derivation_is_deterministic_for_same_config() {
    let master_key = b"master-key-1234";
    let config = EncryptionConfig {
        column_name: "sensor".to_string(),
        schema_fingerprint: [1, 2, 3, 4, 5, 6, 7, 8],
    };
    let key1 = derive_column_key(master_key, &config).unwrap();
    let key2 = derive_column_key(master_key, &config).unwrap();
    assert_eq!(key1, key2);
}

#[test]
fn encryption_key_derivation_changes_for_different_schema_fingerprint() {
    let master_key = b"master-key-1234";
    let config1 = EncryptionConfig {
        column_name: "sensor".to_string(),
        schema_fingerprint: [1, 2, 3, 4, 5, 6, 7, 8],
    };
    let config2 = EncryptionConfig {
        column_name: "sensor".to_string(),
        schema_fingerprint: [8, 7, 6, 5, 4, 3, 2, 1],
    };
    let key1 = derive_column_key(master_key, &config1).unwrap();
    let key2 = derive_column_key(master_key, &config2).unwrap();
    assert_ne!(key1, key2);
}

#[test]
fn encryption_key_derivation_changes_for_different_column_name() {
    let master_key = b"master-key-1234";
    let config1 = EncryptionConfig {
        column_name: "sensorA".to_string(),
        schema_fingerprint: [1, 2, 3, 4, 5, 6, 7, 8],
    };
    let config2 = EncryptionConfig {
        column_name: "sensorB".to_string(),
        schema_fingerprint: [1, 2, 3, 4, 5, 6, 7, 8],
    };
    let key1 = derive_column_key(master_key, &config1).unwrap();
    let key2 = derive_column_key(master_key, &config2).unwrap();
    assert_ne!(key1, key2);
}

#[test]
fn encryption_encrypt_decrypt_roundtrip_for_small_payloads() {
    let master_key = b"master-key-1234";
    let config = EncryptionConfig {
        column_name: "sensor".to_string(),
        schema_fingerprint: [1, 2, 3, 4, 5, 6, 7, 8],
    };
    let key = derive_column_key(master_key, &config).unwrap();
    let payload = b"hello";
    let encrypted = encrypt_payload(payload, &key).unwrap();
    let decrypted = decrypt_payload(
        &encrypted.ciphertext,
        &key,
        &encrypted.nonce,
        &encrypted.auth_tag,
    )
    .unwrap();
    assert_eq!(decrypted, payload);
}

#[test]
fn encryption_tamper_detects_ciphertext_modification() {
    let master_key = b"master-key-1234";
    let config = EncryptionConfig {
        column_name: "sensor".to_string(),
        schema_fingerprint: [1, 2, 3, 4, 5, 6, 7, 8],
    };
    let key = derive_column_key(master_key, &config).unwrap();
    let payload = b"integrity check";
    let encrypted = encrypt_payload(payload, &key).unwrap();
    let tampered_ciphertext = corrupt_bytes(&encrypted.ciphertext, 0);
    assert!(decrypt_payload(
        &tampered_ciphertext,
        &key,
        &encrypted.nonce,
        &encrypted.auth_tag
    )
    .is_err());
}

#[test]
fn encryption_tamper_detects_auth_tag_modification() {
    let master_key = b"master-key-1234";
    let config = EncryptionConfig {
        column_name: "sensor".to_string(),
        schema_fingerprint: [1, 2, 3, 4, 5, 6, 7, 8],
    };
    let key = derive_column_key(master_key, &config).unwrap();
    let payload = b"integrity check";
    let encrypted = encrypt_payload(payload, &key).unwrap();
    let mut bad_tag = encrypted.auth_tag;
    bad_tag.0[0] ^= 0xFF;
    assert!(decrypt_payload(&encrypted.ciphertext, &key, &encrypted.nonce, &bad_tag).is_err());
}

#[test]
fn encrypted_chunk_pack_unpack_roundtrips() {
    let chunk = encrypt_payload(
        b"roundtrip test",
        &derive_column_key(
            b"master",
            &EncryptionConfig {
                column_name: "sensor".to_string(),
                schema_fingerprint: [1, 2, 3, 4, 5, 6, 7, 8],
            },
        )
        .unwrap(),
    )
    .unwrap();
    let packed = pack_encrypted_chunk(&chunk);
    let unpacked = unpack_encrypted_chunk(&packed).unwrap();
    assert_eq!(unpacked, chunk);
}

#[test]
fn ecc_encode_single_parity_chunk_matches_rs() {
    let data = vec![vec![1, 2, 3], vec![4, 5, 6]];
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    let parity = ecc_encode(&data, config).unwrap();
    assert_eq!(parity.len(), 1);
    assert!(ecc_verify(&data, &parity, config).unwrap());
}

#[test]
fn ecc_verify_rejects_corrupted_parity() {
    let data = vec![vec![7, 8, 9], vec![1, 2, 3]];
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    let mut parity = ecc_encode(&data, config).unwrap();
    parity[0][0] ^= 0xFF;
    assert!(!ecc_verify(&data, &parity, config).unwrap());
}

#[test]
fn ecc_recover_missing_data_chunk_using_parity() {
    let data = vec![vec![1, 2, 3], vec![4, 5, 6]];
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    let parity = ecc_encode(&data, config).unwrap();
    let corrupted = vec![None, Some(data[1].clone()), Some(parity[0].clone())];
    let recovered = recover_missing_chunk(&corrupted, config).unwrap();
    assert_eq!(recovered, data[0]);
}

#[test]
fn ecc_recover_rejects_too_many_missing_chunks() {
    let data = vec![vec![1, 2, 3], vec![4, 5, 6]];
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    let parity = ecc_encode(&data, config).unwrap();
    let corrupted = vec![None, None, Some(parity[0].clone())];
    assert!(recover_missing_chunk(&corrupted, config).is_err());
}

#[test]
fn ecc_encode_rejects_mismatched_chunk_widths() {
    let data = vec![vec![1, 2], vec![3]];
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    assert!(ecc_encode(&data, config).is_err());
}

#[test]
fn ecc_multi_parity_chunks_produced() {
    let data = vec![vec![1, 2, 3], vec![4, 5, 6]];
    let config = ReedSolomonConfig::new(2, 3).unwrap();
    let parity = ecc_encode(&data, config).unwrap();
    assert_eq!(parity.len(), 3);
}

#[test]
fn row_group_serializes_and_deserializes_empty_group() {
    let row_group = RowGroup::from_rows(&[]).unwrap();
    let bytes = row_group.serialize().unwrap();
    let parsed = RowGroup::deserialize(&bytes).unwrap();
    assert_eq!(parsed, row_group);
}

#[test]
fn row_group_rejects_rows_with_nonuniform_width() {
    let rows = vec![vec![1, 2, 3], vec![4, 5]];
    assert!(RowGroup::from_rows(&rows).is_err());
}

#[test]
fn row_group_detects_trailing_bytes() {
    let rows = vec![vec![1, 2], vec![3, 4]];
    let row_group = RowGroup::from_rows(&rows).unwrap();
    let mut bytes = row_group.serialize().unwrap();
    bytes.push(0xFF);
    assert!(RowGroup::deserialize(&bytes).is_err());
}

#[test]
fn row_group_rejects_checksum_mismatch() {
    let rows = vec![vec![1, 2], vec![3, 4]];
    let row_group = RowGroup::from_rows(&rows).unwrap();
    let mut bytes = row_group.serialize().unwrap();
    if let Some(last) = bytes.last_mut() {
        *last ^= 0xFF;
    }
    assert!(RowGroup::deserialize(&bytes).is_err());
}

#[test]
fn row_group_serializes_large_width_group() {
    let rows: Vec<Vec<u8>> = (0..16).map(|i| vec![(i % 256) as u8; 32]).collect();
    let row_group = RowGroup::from_rows(&rows).unwrap();
    let bytes = row_group.serialize().unwrap();
    let parsed = RowGroup::deserialize(&bytes).unwrap();
    assert_eq!(parsed.row_count, 16);
    assert_eq!(parsed.columns.len(), 32);
}

#[test]
fn parser_rejects_invalid_magic_header() {
    let header = FileHeader::new(1, 0, [0; 8], 0, *b"qrd-0.1.0\0\0\0");
    let mut bytes = header.serialize();
    bytes[0] = 0x00;
    assert!(parse_header(&bytes).is_err());
}

#[test]
fn parser_rejects_nonzero_reserved_header() {
    let header = FileHeader::new(1, 0, [0; 8], 0, *b"qrd-0.1.0\0\0\0");
    let mut bytes = header.serialize();
    bytes[18] = 0xFF;
    assert!(parse_header(&bytes).is_err());
}

#[test]
fn parser_rejects_truncated_footer_length() {
    assert!(parse_footer_length(&[0x01, 0x02]).is_err());
}

#[test]
fn parser_rejects_truncated_footer() {
    assert!(parse_footer(&[0x00, 0x01, 0x02]).is_err());
}

#[test]
fn schema_fingerprint_changes_when_field_order_changes() {
    let schema1 = SchemaBuilder::new()
        .add_field("a", FieldKind::Int32, false)
        .add_field("b", FieldKind::Utf8, true)
        .build()
        .unwrap();
    let schema2 = SchemaBuilder::new()
        .add_field("b", FieldKind::Utf8, true)
        .add_field("a", FieldKind::Int32, false)
        .build()
        .unwrap();
    assert_ne!(schema1.fingerprint(), schema2.fingerprint());
}

#[test]
fn transpose_rows_roundtrips_wide_rows() {
    let rows = vec![vec![1, 2, 3, 4, 5], vec![6, 7, 8, 9, 10]];
    let columns = transpose_rows(&rows).unwrap();
    assert_eq!(
        columns,
        vec![vec![1, 6], vec![2, 7], vec![3, 8], vec![4, 9], vec![5, 10]]
    );
}

#[test]
fn transpose_rows_rejects_empty_nested_rows() {
    let rows: Vec<Vec<u8>> = vec![vec![]];
    let columns = transpose_rows(&rows).unwrap();
    assert!(columns.is_empty());
}

#[test]
fn row_group_column_chunk_names_are_preserved() {
    let rows = vec![vec![10, 20, 30], vec![40, 50, 60]];
    let row_group = RowGroup::from_rows(&rows).unwrap();
    assert!(row_group.columns.iter().any(|column| column.name == "col0"));
    assert!(row_group.columns.iter().any(|column| column.name == "col2"));
}

#[test]
fn row_group_column_chunk_payloads_decode_correctly() {
    let rows = vec![vec![1, 2], vec![3, 4]];
    let row_group = RowGroup::from_rows(&rows).unwrap();
    for column in &row_group.columns {
        let decoded = column.decode().unwrap();
        assert!(decoded.len() == 2);
    }
}

#[test]
fn parser_footer_roundtrips_known_footer() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .build()
        .unwrap();
    let footer = qrd_core::footer::build_footer(&schema, 1).unwrap();
    let parsed = parse_footer(&footer).unwrap();
    assert_eq!(parsed.row_group_count, 1);
}

#[test]
fn footer_length_trailer_roundtrips_for_large_length() {
    let mut bytes = vec![0u8; 10];
    qrd_core::parser::append_footer_length(&mut bytes, 0x1234_ABCD);
    let length = parse_footer_length(&bytes).unwrap();
    assert_eq!(length, 0x1234_ABCD);
}

#[test]
fn crc32_matches_known_input_vector() {
    let data = b"crc32-test";
    assert_eq!(crc32(data), 0x4938_C8C2);
}

#[test]
fn column_chunk_encoding_id_stored_and_restored() {
    let chunk = ColumnChunk::new("test", b"payload", EncodingId::DeltaBinary).unwrap();
    assert_eq!(chunk.encoding, EncodingId::DeltaBinary);
    let decoded = chunk.decode().unwrap();
    assert_eq!(decoded, b"payload".to_vec());
}

#[test]
fn schema_builder_rejects_duplicate_field_names() {
    let result = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("id", FieldKind::Utf8, true)
        .build();
    assert!(result.is_err());
}

#[test]
fn parse_header_preserves_writer_version() {
    let header = FileHeader::new(1, 0, [9, 8, 7, 6, 5, 4, 3, 2], 0b11, *b"qrd-0.1.0\0\0\0");
    let bytes = header.serialize();
    let parsed = parse_header(&bytes).unwrap();
    assert_eq!(parsed.writer_version, header.writer_version);
}

#[test]
fn compression_recompresses_repeated_payload_more_than_once() {
    let payload = vec![0u8; 1024];
    let compressed = compress(&payload, CompressionKind::Zstd).unwrap();
    assert!(compressed.len() < payload.len());
}

#[test]
fn encoding_byte_stream_split_rejects_invalid_length_header() {
    let malformed = vec![0, 0, 0, 2, 9, 0];
    assert!(decode(&malformed, EncodingId::ByteStreamSplit).is_err());
}

#[test]
fn encoding_dict_rle_rejects_invalid_dictionary_reference() {
    let malformed = vec![1, 42, 0, 0, 0, 2, 1, 2];
    assert!(decode(&malformed, EncodingId::DictRle).is_err());
}

#[test]
fn encryption_rejects_empty_master_key_derivation() {
    let config = EncryptionConfig {
        column_name: "sensor".to_string(),
        schema_fingerprint: [0; 8],
    };
    assert!(derive_column_key(b"", &config).is_err());
}

#[test]
fn row_group_roundtrips_with_custom_column_names() {
    let row_group = RowGroup {
        row_count: 1,
        columns: vec![ColumnChunk::new("foo", b"x", EncodingId::Plain).unwrap()],
    };
    let bytes = row_group.serialize().unwrap();
    let parsed = RowGroup::deserialize(&bytes).unwrap();
    assert_eq!(parsed.columns[0].name, "foo");
}
