use qrd_core::compression::{choose_compression, compress, decompress, CompressionKind};
use qrd_core::ecc::{encode, recover_missing_chunk, verify, ReedSolomonConfig};
use qrd_core::encoding::{decode, encode as encode_values, EncodingId};
use qrd_core::encryption::{decrypt_payload, derive_column_key, encrypt_payload, AuthTag, EncryptionConfig, Nonce};
use qrd_core::file::{build_file_image, build_file_image_with_signature, parse_file_image};
use qrd_core::parser::{append_footer_length, build_footer, parse_footer, parse_footer_length, parse_header, FileHeader, HEADER_SIZE};
use qrd_core::reader::FileReader;
use qrd_core::row_group::RowGroup;
use qrd_core::schema::{FieldKind, SchemaBuilder};
use qrd_core::signing::{SchemaSignature, SIGNATURE_ALGORITHM, SigningKeyPair, VerifyingKeyPair};
use qrd_core::integrity::{crc32, verify_crc32};
use qrd_core::error::QrdError;

// -----------------------------------------------------------------------------
// Parser Tests
// -----------------------------------------------------------------------------

#[test]
fn parse_header_rejects_short_buffer() {
    let error = parse_header(&[0u8; 10]).expect_err("short header must fail");
    assert!(matches!(error, QrdError::InvalidHeaderLength));
}

#[test]
fn parse_header_rejects_invalid_magic_bytes() {
    let mut bytes = [0u8; 32];
    bytes[0..4].copy_from_slice(b"BAD\0");
    let error = parse_header(&bytes).expect_err("bad magic should fail");
    assert!(matches!(error, QrdError::InvalidMagic));
}

#[test]
fn parse_header_rejects_reserved_flag_nonzero() {
    let header = FileHeader::new(1, 0, [0; 8], 0, *b"qrd-0.1.0\0\0\0");
    let mut bytes = header.serialize();
    bytes[18] = 0xFF;
    let error = parse_header(&bytes).expect_err("nonzero reserved should fail");
    assert!(matches!(error, QrdError::InvalidReservedField));
}

#[test]
fn parse_header_accepts_valid_header() {
    let schema_id = [10, 20, 30, 40, 50, 60, 70, 80];
    let header = FileHeader::new(1, 2, schema_id, 0x0003, *b"qrd-0.1.0\0\0\0");
    let parsed = parse_header(&header.serialize()).expect("valid header should parse");
    assert_eq!(parsed, header);
}

#[test]
fn append_and_parse_footer_length_are_inverse() {
    let mut buffer = Vec::new();
    append_footer_length(&mut buffer, 0xDEADBEEF);
    assert_eq!(parse_footer_length(&buffer).expect("footer length parse"), 0xDEADBEEF);
}

#[test]
fn parse_footer_rejects_insufficient_bytes() {
    let error = parse_footer(&[1, 2, 3]).expect_err("short footer must fail");
    assert!(matches!(error, QrdError::InvalidFooterLength | QrdError::UnexpectedEof));
}

#[test]
fn parse_footer_rejects_wrong_checksum() {
    let schema = SchemaBuilder::new().add_field("id", FieldKind::Int32, true).build().unwrap();
    let mut footer_bytes = build_footer(&schema, 1).expect("footer build");
    if let Some(last_byte) = footer_bytes.last_mut() {
        *last_byte ^= 0xFF;
    }
    assert!(parse_footer(&footer_bytes).is_err());
}

#[test]
fn parse_footer_roundtrip_with_schema_and_row_group_count() {
    let schema = SchemaBuilder::new()
        .add_field("device_id", FieldKind::Utf8, true)
        .build()
        .expect("schema build");
    let footer_bytes = build_footer(&schema, 42).expect("footer build");
    let parsed = parse_footer(&footer_bytes).expect("footer parse");
    assert_eq!(parsed.row_group_count, 42);
    assert_eq!(parsed.schema, schema);
}

#[test]
fn parse_footer_rejects_unsupported_version() {
    let mut bytes = vec![2u8];
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 4]);
    bytes.extend_from_slice(&crc32(&bytes).to_le_bytes());
    assert!(parse_footer(&bytes).is_err());
}

#[test]
fn parse_footer_rejects_truncated_schema_payload() {
    let bytes = vec![1u8, 0, 0, 0, 10, 0, 0, 0];
    assert!(parse_footer(&bytes).is_err());
}

#[test]
fn parse_footer_rejects_trailing_bytes() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Utf8, true).build().unwrap();
    let mut footer_bytes = build_footer(&schema, 1).expect("footer build");
    footer_bytes.extend_from_slice(&[1, 2, 3]);
    assert!(parse_footer(&footer_bytes).is_err());
}

#[test]
fn parse_footer_uses_expected_crc32_implementation() {
    let payload = b"hello crc";
    let checksum = crc32(payload);
    assert!(verify_crc32(payload, checksum));
}

// -----------------------------------------------------------------------------
// Schema Tests
// -----------------------------------------------------------------------------

#[test]
fn schema_builder_rejects_empty_field_names() {
    let result = SchemaBuilder::new().add_field("", FieldKind::Int32, true).build();
    assert!(matches!(result, Err(QrdError::InvalidSchema(_))));
}

#[test]
fn schema_builder_rejects_duplicate_field_names() {
    let result = SchemaBuilder::new()
        .add_field("a", FieldKind::Int32, true)
        .add_field("a", FieldKind::Utf8, false)
        .build();
    assert!(matches!(result, Err(QrdError::InvalidSchema(_))));
}

#[test]
fn schema_serialize_deserialize_roundtrip() {
    let schema = SchemaBuilder::new()
        .add_field("temperature", FieldKind::Float64, false)
        .add_field("status", FieldKind::Utf8, true)
        .build()
        .unwrap();
    let bytes = schema.serialize().unwrap();
    let parsed = qrd_core::schema::Schema::deserialize(&bytes).unwrap();
    assert_eq!(parsed, schema);
}

#[test]
fn schema_deserialize_rejects_invalid_kind() {
    let bytes = vec![1, 3, b'a', 0xFF, 1];
    let result = qrd_core::schema::Schema::deserialize(&bytes);
    assert!(matches!(result, Err(QrdError::InvalidSchema(_))));
}

#[test]
fn schema_deserialize_rejects_invalid_required_flag() {
    let bytes = vec![1, 3, b'a', 5, 0xFF];
    let result = qrd_core::schema::Schema::deserialize(&bytes);
    assert!(matches!(result, Err(QrdError::InvalidSchema(_))));
}

#[test]
fn schema_fingerprint_changes_when_field_order_changes() {
    let a = SchemaBuilder::new().add_field("a", FieldKind::Utf8, true).build().unwrap();
    let b = SchemaBuilder::new().add_field("b", FieldKind::Utf8, true).build().unwrap();
    assert_ne!(a.fingerprint(), b.fingerprint());
}

#[test]
fn schema_fingerprint_is_stable_for_same_definition() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Boolean, false).build().unwrap();
    assert_eq!(schema.fingerprint(), schema.fingerprint());
}

#[test]
fn schema_serialize_rejects_too_many_fields() {
    let mut builder = SchemaBuilder::new();
    for i in 0..260 {
        builder = builder.add_field(format!("f{i}"), FieldKind::Int32, true);
    }
    let result = builder.build().and_then(|schema| schema.serialize());
    assert!(result.is_err());
}

#[test]
fn schema_deserialize_rejects_non_utf8_name() {
    let bytes = vec![1, 2, 0xFF, 0xFE, 5, 1];
    assert!(matches!(qrd_core::schema::Schema::deserialize(&bytes), Err(QrdError::InvalidSchema(_))));
}

#[test]
fn schema_builder_accepts_many_fields_up_to_limit() {
    let mut builder = SchemaBuilder::new();
    for i in 0..250 {
        builder = builder.add_field(format!("field{i}"), FieldKind::Int32, false);
    }
    let schema = builder.build().expect("should build many fields");
    assert_eq!(schema.fields().len(), 250);
}

// -----------------------------------------------------------------------------
// RowGroup Tests
// -----------------------------------------------------------------------------

#[test]
fn row_group_from_rows_with_uniform_width_succeeds() {
    let rows = vec![vec![1, 2], vec![3, 4]];
    let row_group = RowGroup::from_rows_with_names(&rows, &["c0", "c1"]).unwrap();
    assert_eq!(row_group.row_count, 2);
    assert_eq!(row_group.columns.len(), 2);
}

#[test]
fn row_group_from_rows_rejects_mismatched_width() {
    let rows = vec![vec![1, 2], vec![3]];
    let result = RowGroup::from_rows(&rows);
    assert!(matches!(result, Err(QrdError::InvalidSchema(_))));
}

#[test]
fn row_group_from_rows_with_names_rejects_invalid_column_count() {
    let rows = vec![vec![1, 2], vec![3, 4]];
    let result = RowGroup::from_rows_with_names(&rows, &["c0"]);
    assert!(matches!(result, Err(QrdError::InvalidSchema(_))));
}

#[test]
fn row_group_serialize_deserialize_with_empty_group() {
    let row_group = RowGroup::from_rows_with_names(&[], &["a", "b"]).unwrap();
    assert_eq!(row_group.row_count, 0);
    let bytes = row_group.serialize().unwrap();
    let parsed = RowGroup::deserialize(&bytes).unwrap();
    assert_eq!(parsed, row_group);
}

#[test]
fn row_group_deserialize_rejects_invalid_column_header() {
    assert!(RowGroup::deserialize(&[0, 0, 0, 1]).is_err());
}

#[test]
fn column_chunk_decode_recovers_plain_payload() {
    let chunk = qrd_core::row_group::ColumnChunk::new("c", &[1, 2, 3], qrd_core::encoding::EncodingId::Plain).unwrap();
    assert_eq!(chunk.decode().unwrap(), vec![1, 2, 3]);
}

#[test]
fn row_group_roundtrip_with_column_names_retains_names() {
    let rows = vec![vec![7, 8], vec![9, 10]];
    let row_group = RowGroup::from_rows_with_names(&rows, &["a", "b"]).unwrap();
    let bytes = row_group.serialize().unwrap();
    let parsed = RowGroup::deserialize(&bytes).unwrap();
    assert_eq!(parsed.columns[0].name, "a");
    assert_eq!(parsed.columns[1].name, "b");
}

#[test]
fn row_group_serialize_rejects_too_long_column_name() {
    let name = "a".repeat(300);
    let row_group = RowGroup::from_rows_with_names(&[vec![1]], &[&name]).unwrap();
    assert!(row_group.serialize().is_err());
}

#[test]
fn row_group_roundtrip_transpose_matches_transpose_rows() {
    let rows = vec![vec![1, 2], vec![3, 4], vec![5, 6]];
    let row_group = RowGroup::from_rows(&rows).unwrap();
    let decoded_a = row_group.columns[0].decode().unwrap();
    let decoded_b = row_group.columns[1].decode().unwrap();
    assert_eq!(decoded_a, vec![1, 3, 5]);
    assert_eq!(decoded_b, vec![2, 4, 6]);
}

#[test]
fn row_group_from_rows_with_names_preserves_column_order() {
    let rows = vec![vec![1, 2]];
    let row_group = RowGroup::from_rows_with_names(&rows, &["first", "second"]).unwrap();
    assert_eq!(row_group.columns[0].name, "first");
    assert_eq!(row_group.columns[1].name, "second");
}

// -----------------------------------------------------------------------------
// File Tests
// -----------------------------------------------------------------------------

#[test]
fn build_and_parse_file_image_roundtrip() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Int32, true).build().unwrap();
    let row_groups = vec![RowGroup::from_rows(&[vec![1, 2], vec![3, 4]]).unwrap()];
    let bytes = build_file_image(&schema, &row_groups).unwrap();
    let parsed = parse_file_image(&bytes).unwrap();
    assert_eq!(parsed.footer.row_group_count, 1);
    assert_eq!(parsed.row_groups.len(), 1);
}

#[test]
fn build_file_image_with_signature_sets_header_flag() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Utf8, true).build().unwrap();
    let keypair = SigningKeyPair::generate();
    let signature = SchemaSignature::new(
        SIGNATURE_ALGORITHM,
        keypair.sign_schema(&schema.fingerprint()),
        keypair.verifying_key(),
    );
    let bytes = build_file_image_with_signature(&schema, &[RowGroup::from_rows(&[]).unwrap()], Some(signature)).unwrap();
    let parsed = parse_file_image(&bytes).unwrap();
    assert!(parsed.header.is_schema_signed());
}

#[test]
fn parse_file_image_rejects_truncated_header() {
    assert!(parse_file_image(&[0u8; 10]).is_err());
}

#[test]
fn parse_file_image_rejects_invalid_row_group_length() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Utf8, true).build().unwrap();
    let bytes = build_file_image(&schema, &[]).unwrap();
    let mut truncated = bytes.clone();
    truncated[35] = 0xFF;
    assert!(parse_file_image(&truncated).is_err());
}

#[test]
fn build_file_image_supports_empty_row_groups() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Int32, true).build().unwrap();
    let bytes = build_file_image(&schema, &[]).unwrap();
    let parsed = parse_file_image(&bytes).unwrap();
    assert_eq!(parsed.row_groups.len(), 0);
    assert_eq!(parsed.footer.row_group_count, 0);
}

#[test]
fn parse_file_image_rejects_invalid_signature_size() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Utf8, true).build().unwrap();
    let bytes = build_file_image(&schema, &[]).unwrap();
    let mut invalid = bytes.clone();
    invalid.extend_from_slice(&[1, 2, 3]);
    assert!(parse_file_image(&invalid).is_err());
}

#[test]
fn build_file_image_with_signature_reconstructs_signature() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Utf8, true).build().unwrap();
    let keypair = SigningKeyPair::generate();
    let sig = SchemaSignature::new(
        SIGNATURE_ALGORITHM,
        keypair.sign_schema(&schema.fingerprint()),
        keypair.verifying_key(),
    );
    let bytes = build_file_image_with_signature(&schema, &[], Some(sig.clone())).unwrap();
    let parsed = parse_file_image(&bytes).unwrap();
    assert_eq!(parsed.signature.unwrap(), sig);
}

#[test]
fn parse_file_image_rejects_invalid_footer_length_too_short() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Utf8, true).build().unwrap();
    let mut bytes = build_file_image(&schema, &[]).unwrap();
    bytes.truncate(bytes.len() - 2);
    assert!(parse_file_image(&bytes).is_err());
}

#[test]
fn parse_file_image_rejects_header_with_missing_signature_flag() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Utf8, true).build().unwrap();
    let row_groups = vec![RowGroup::from_rows(&[vec![1]]).unwrap()];
    let bytes = build_file_image(&schema, &row_groups).unwrap();
    // Set schema signed bit without actually appending signature
    let mut tampered = bytes.clone();
    tampered[16] = 0x02;
    assert!(parse_file_image(&tampered).is_err());
}

#[test]
fn build_file_image_handles_multiple_row_groups() {
    let schema = SchemaBuilder::new()
        .add_field("x", FieldKind::Int32, true)
        .build()
        .unwrap();
    let rg1 = RowGroup::from_rows(&[vec![1, 2]]).unwrap();
    let rg2 = RowGroup::from_rows(&[vec![3, 4]]).unwrap();

    let bytes = build_file_image(&schema, &[rg1.clone(), rg2.clone()]).unwrap();
    let parsed = parse_file_image(&bytes).unwrap();
    assert_eq!(parsed.footer.row_group_count, 2);
    assert_eq!(parsed.row_groups, vec![rg1, rg2]);
}

#[test]
fn parse_file_image_rejects_excess_row_group_length() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Utf8, true).build().unwrap();
    let mut bytes = build_file_image(&schema, &[]).unwrap();
    bytes[32..36].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(parse_file_image(&bytes).is_err());
}

// -----------------------------------------------------------------------------
// Reader Tests
// -----------------------------------------------------------------------------

#[test]
fn file_reader_read_columns_returns_requested_columns() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, true)
        .add_field("flag", FieldKind::Boolean, false)
        .build()
        .unwrap();

    let row_group = RowGroup::from_rows_with_names(&[vec![1, 2], vec![3, 4]], &["id", "flag"]).unwrap();
    let bytes = build_file_image(&schema, &[row_group]).unwrap();
    let reader = FileReader::open(&bytes).unwrap();

    let columns = reader.read_columns(&["id"]).unwrap();
    assert_eq!(columns, vec![vec![1, 3]]);
}

#[test]
fn file_reader_read_columns_rejects_missing_column() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Int32, true).build().unwrap();
    let bytes = build_file_image(&schema, &[RowGroup::from_rows(&[vec![1]]).unwrap()]).unwrap();
    let reader = FileReader::open(&bytes).unwrap();
    assert!(reader.read_columns(&["y"]).is_err());
}

#[test]
fn reader_verify_integrity_detects_row_group_count_mismatch() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Int32, true).build().unwrap();
    let bytes = build_file_image(&schema, &[RowGroup::from_rows(&[vec![1]]).unwrap()]).unwrap();
    let mut truncated = bytes.clone();
    truncated[HEADER_SIZE] = 0;
    let result = FileReader::open(&truncated);
    assert!(result.is_err());
}

#[test]
fn file_reader_open_parses_header_footer_and_rows() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Int32, true).build().unwrap();
    let row_group = RowGroup::from_rows(&[vec![7, 8]]).unwrap();
    let bytes = build_file_image(&schema, &[row_group.clone()]).unwrap();
    let reader = FileReader::open(&bytes).unwrap();
    assert_eq!(reader.header().schema_id, schema.fingerprint());
    assert_eq!(reader.footer().row_group_count, 1);
    assert_eq!(reader.row_count(), 1);
}

#[test]
fn file_reader_inspect_header_returns_header_from_bytes() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Int32, true).build().unwrap();
    let header = FileHeader::new(1, 0, schema.fingerprint(), 0, *b"qrd-0.1.0\0\0\0");
    let parsed = FileReader::inspect_header(&header.serialize()).unwrap();
    assert_eq!(parsed, header);
}

#[test]
fn file_reader_inspect_footer_returns_footer_from_bytes() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Int32, true).build().unwrap();
    let footer = build_footer(&schema, 1).unwrap();
    let parsed = FileReader::inspect_footer(&footer).unwrap();
    assert_eq!(parsed.row_group_count, 1);
}

#[test]
fn file_reader_inspect_footer_length_returns_length_from_bytes() {
    let mut bytes = Vec::new();
    append_footer_length(&mut bytes, 0xAABBCCDD);
    let length = FileReader::inspect_footer_length(&bytes).unwrap();
    assert_eq!(length, 0xAABBCCDD);
}

#[test]
fn file_reader_read_row_group_roundtrips_row_group_bytes() {
    let row_group = RowGroup::from_rows(&[vec![11, 12], vec![13, 14]]).unwrap();
    let bytes = row_group.serialize().unwrap();
    let reader = FileReader::new(SchemaBuilder::new().add_field("a", FieldKind::Int32, true).build().unwrap());
    let roundtrip = reader.read_row_group(&bytes).unwrap();
    assert_eq!(roundtrip, row_group);
}

#[test]
fn file_reader_verify_integrity_accepts_matching_row_group_count() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Int32, true).build().unwrap();
    let bytes = build_file_image(&schema, &[RowGroup::from_rows(&[vec![1]]).unwrap()]).unwrap();
    let reader = FileReader::open(&bytes).unwrap();
    assert!(reader.verify_integrity().is_ok());
}

#[test]
fn file_reader_open_rejects_corrupted_file_image() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Int32, true).build().unwrap();
    let mut bytes = build_file_image(&schema, &[RowGroup::from_rows(&[vec![1]]).unwrap()]).unwrap();
    bytes[0] ^= 0xFF;
    assert!(FileReader::open(&bytes).is_err());
}

// -----------------------------------------------------------------------------
// Writer Tests
// -----------------------------------------------------------------------------

#[test]
fn streaming_writer_finish_once_then_write_after_finish_is_rejected() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Int32, true).build().unwrap();
    let mut writer = qrd_core::writer::StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1]]).unwrap();
    let _ = writer.finish().unwrap();
    assert!(writer.write_row_group(&[vec![2]]).is_err());
}

#[test]
fn streaming_writer_write_after_finish_is_rejected() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Int32, true).build().unwrap();
    let mut writer = qrd_core::writer::StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1]]).unwrap();
    let bytes = writer.finish().unwrap();
    let mut writer = qrd_core::writer::StreamingWriter::new(SchemaBuilder::new().add_field("x", FieldKind::Int32, true).build().unwrap());
    writer.write_row_group(&[vec![1]]).unwrap();
    assert!(writer.finish().is_ok());
    assert!(bytes.len() > 0);
}

#[test]
fn streaming_writer_build_footer_bytes_matches_file_footer() {
    let schema = SchemaBuilder::new().add_field("a", FieldKind::Int32, true).build().unwrap();
    let mut writer = qrd_core::writer::StreamingWriter::new(schema);
    writer.write_row_group(&[vec![10]]).unwrap();
    let footer_bytes = writer.build_footer_bytes().unwrap();
    assert!(footer_bytes.len() > 4);
}

#[test]
fn streaming_writer_can_sign_schema_and_produce_readable_image() {
    let schema = SchemaBuilder::new().add_field("id", FieldKind::Int32, true).build().unwrap();
    let mut writer = qrd_core::writer::StreamingWriter::new(schema.clone());
    let keypair = SigningKeyPair::generate();
    let sig_bytes = keypair.sign_schema(&schema.fingerprint());
    let sig = SchemaSignature::new(SIGNATURE_ALGORITHM, sig_bytes, keypair.verifying_key());
    writer.set_signature(sig);
    writer.write_row_group(&[vec![1]]).unwrap();
    let bytes = writer.finish().unwrap();
    assert!(FileReader::open(&bytes).is_ok());
}

#[test]
fn streaming_writer_row_groups_are_counted_correctly() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Utf8, true).build().unwrap();
    let mut writer = qrd_core::writer::StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1], vec![2]]).unwrap();
    writer.write_row_group(&[vec![3], vec![4]]).unwrap();
    assert_eq!(writer.row_groups().len(), 2);
}

#[test]
fn streaming_writer_can_clear_signature() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Utf8, true).build().unwrap();
    let mut writer = qrd_core::writer::StreamingWriter::new(schema);
    let keypair = SigningKeyPair::generate();
    let sig_bytes = keypair.sign_schema(&SchemaBuilder::new().add_field("x", FieldKind::Utf8, true).build().unwrap().fingerprint());
    let sig = SchemaSignature::new(SIGNATURE_ALGORITHM, sig_bytes, keypair.verifying_key());
    writer.set_signature(sig);
    writer.clear_signature();
    writer.write_row_group(&[vec![1]]).unwrap();
    let bytes = writer.finish().unwrap();
    let parsed = parse_file_image(&bytes).unwrap();
    assert!(parsed.signature.is_none());
}

#[test]
fn streaming_writer_can_write_multiple_row_groups_before_finish() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Utf8, true).build().unwrap();
    let mut writer = qrd_core::writer::StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1]]).unwrap();
    writer.write_row_group(&[vec![2]]).unwrap();
    let bytes = writer.finish().unwrap();
    let parsed = parse_file_image(&bytes).unwrap();
    assert_eq!(parsed.footer.row_group_count, 2);
}

#[test]
fn streaming_writer_build_footer_bytes_works_for_empty_writer() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Utf8, true).build().unwrap();
    let writer = qrd_core::writer::StreamingWriter::new(schema);
    let footer_bytes = writer.build_footer_bytes().unwrap();
    assert!(footer_bytes.len() > 0);
}

#[test]
fn streaming_writer_handles_empty_row_group_with_names() {
    let schema = SchemaBuilder::new()
        .add_field("a", FieldKind::Utf8, true)
        .build()
        .unwrap();
    let mut writer = qrd_core::writer::StreamingWriter::new(schema);
    writer.write_row_group(&[]).unwrap();
    let bytes = writer.finish().unwrap();
    let parsed = parse_file_image(&bytes).unwrap();
    assert_eq!(parsed.footer.row_group_count, 1);
}

#[test]
fn streaming_writer_rejects_row_group_after_finish() {
    let schema = SchemaBuilder::new().add_field("x", FieldKind::Utf8, true).build().unwrap();
    let mut writer = qrd_core::writer::StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1]]).unwrap();
    let bytes = writer.finish().unwrap();
    assert!(bytes.len() > 0);
}

// -----------------------------------------------------------------------------
// ECC Tests
// -----------------------------------------------------------------------------

#[test]
fn rs_config_rejects_zero_values() {
    assert!(ReedSolomonConfig::new(0, 1).is_err());
    assert!(ReedSolomonConfig::new(1, 0).is_err());
}

#[test]
fn rs_config_total_and_capacity_are_correct() {
    let config = ReedSolomonConfig::new(4, 2).unwrap();
    assert_eq!(config.total_chunks(), 6);
    assert_eq!(config.recovery_capacity(), 2);
}

#[test]
fn rs_encode_parity_matches_expected_simple_case() {
    let data = vec![vec![1u8, 2], vec![3, 4]];
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    let parity = encode(&data, config).unwrap();
    assert_eq!(parity.len(), 1);
}

#[test]
fn rs_encode_rejects_mismatched_data_count() {
    let data = vec![vec![1u8, 2]];
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    assert!(encode(&data, config).is_err());
}

#[test]
fn rs_encode_rejects_non_uniform_chunk_width() {
    let data = vec![vec![1u8, 2], vec![3]];
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    assert!(encode(&data, config).is_err());
}

#[test]
fn rs_verify_returns_true_for_valid_chunks() {
    let data = vec![vec![10u8, 20], vec![30, 40]];
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    let parity = encode(&data, config).unwrap();
    assert!(verify(&data, &parity, config).unwrap());
}

#[test]
fn rs_verify_returns_false_for_invalid_parity() {
    let data = vec![vec![10u8, 20], vec![30, 40]];
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    let mut parity = encode(&data, config).unwrap();
    parity[0][0] ^= 1;
    assert!(!verify(&data, &parity, config).unwrap());
}

#[test]
fn rs_recover_single_missing_data_chunk() {
    let data = vec![Some(vec![1u8, 2, 3]), None, Some(vec![4, 5, 6])];
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    let recovered = recover_missing_chunk(&data, config).unwrap();
    assert_eq!(recovered.len(), 3);
}

#[test]
fn rs_recover_single_missing_parity_chunk() {
    let d0 = vec![1u8, 2, 3];
    let d1 = vec![4, 5, 6];
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    let parity = encode(&[d0.clone(), d1.clone()], config).unwrap();
    let data = vec![Some(d0), Some(d1), None];
    let recovered = recover_missing_chunk(&data, config).unwrap();
    assert_eq!(recovered, parity[0]);
}

#[test]
fn rs_recover_multiple_missing_data_chunks_is_rejected_when_too_many() {
    let data = vec![None, None, Some(vec![1, 2, 3])];
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    assert!(recover_missing_chunk(&data, config).is_err());
}

#[test]
fn rs_recover_data_chunk_with_even_width() {
    let d0 = vec![1u8, 1, 1, 1];
    let d1 = vec![2u8, 2, 2, 2];
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    let parity = encode(&[d0.clone(), d1.clone()], config).unwrap();
    let data = vec![Some(d0), None, Some(parity[0].clone())];
    let recovered = recover_missing_chunk(&data, config).unwrap();
    assert_eq!(recovered, d1);
}

#[test]
fn rs_recover_single_missing_chunk_from_parity_only() {
    let d0 = vec![7u8, 8];
    let d1 = vec![9u8, 10];
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    let parity = encode(&[d0.clone(), d1.clone()], config).unwrap();
    let data = vec![Some(d0.clone()), Some(d1.clone()), None];
    let recovered = recover_missing_chunk(&data, config).unwrap();
    assert_eq!(recovered, parity[0]);
}

#[test]
fn rs_recover_missing_chunk_rejects_width_mismatch_in_available_chunks() {
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    let data = vec![Some(vec![1, 2]), Some(vec![3]), None];
    assert!(recover_missing_chunk(&data, config).is_err());
}

#[test]
fn rs_recover_missing_chunk_rejects_incorrect_total_chunk_count() {
    let config = ReedSolomonConfig::new(2, 1).unwrap();
    let data = vec![Some(vec![1, 2])];
    assert!(recover_missing_chunk(&data, config).is_err());
}

#[test]
fn rs_encode_and_recover_roundtrip_for_multiple_parity_chunks() {
    let data = vec![vec![5u8, 6, 7], vec![1, 2, 3], vec![9, 8, 7], vec![4, 3, 2]];
    let config = ReedSolomonConfig::new(4, 2).unwrap();
    let parity = encode(&data, config).unwrap();
    let mut chunks: Vec<Option<Vec<u8>>> = data.iter().cloned().map(Some).collect();
    chunks.push(Some(parity[0].clone()));
    chunks.push(None);
    let recovered = recover_missing_chunk(&chunks, config).unwrap();
    assert_eq!(recovered, parity[1]);
}

// -----------------------------------------------------------------------------
// Encoding Tests
// -----------------------------------------------------------------------------

#[test]
fn encode_plain_roundtrip_is_identity() {
    let values = [1u8, 2, 3, 255];
    assert_eq!(encode_values(&values, EncodingId::Plain).unwrap(), values.to_vec());
}

#[test]
fn encode_rle_roundtrip_works_for_repeated_values() {
    let sample = [4u8, 4, 4, 4, 4];
    let encoded = encode_values(&sample, EncodingId::Rle).unwrap();
    assert_eq!(decode(&encoded, EncodingId::Rle).unwrap(), sample.to_vec());
}

#[test]
fn encode_bit_packed_roundtrip_handles_zeroes_and_ones() {
    let sample = [0u8, 1, 2, 3, 4, 255];
    let encoded = encode_values(&sample, EncodingId::BitPacked).unwrap();
    assert_eq!(decode(&encoded, EncodingId::BitPacked).unwrap(), sample.to_vec());
}

#[test]
fn decode_bit_packed_rejects_invalid_bit_width() {
    let bad = vec![1, 0, 0, 0, 9, 0, 0, 0];
    assert!(matches!(decode(&bad, EncodingId::BitPacked), Err(QrdError::InvalidSchema(_))));
}

#[test]
fn delta_binary_roundtrip_overflow_wraps() {
    let sample = [255u8, 0, 1, 2];
    let encoded = encode_values(&sample, EncodingId::DeltaBinary).unwrap();
    assert_eq!(decode(&encoded, EncodingId::DeltaBinary).unwrap(), sample.to_vec());
}

#[test]
fn delta_byte_array_roundtrip_works_for_empty_payload() {
    let sample = [] as [u8; 0];
    let encoded = encode_values(&sample, EncodingId::DeltaByteArray).unwrap();
    assert_eq!(decode(&encoded, EncodingId::DeltaByteArray).unwrap(), sample.to_vec());
}

#[test]
fn byte_stream_split_roundtrip_works_for_random_values() {
    let sample = [12u8, 34, 56, 78, 90];
    let encoded = encode_values(&sample, EncodingId::ByteStreamSplit).unwrap();
    assert_eq!(decode(&encoded, EncodingId::ByteStreamSplit).unwrap(), sample.to_vec());
}

#[test]
fn dict_rle_roundtrip_works_for_repeated_dictionary_values() {
    let sample = [7u8, 7, 7, 8, 8, 9];
    let encoded = encode_values(&sample, EncodingId::DictRle).unwrap();
    assert_eq!(decode(&encoded, EncodingId::DictRle).unwrap(), sample.to_vec());
}

// -----------------------------------------------------------------------------
// Compression Tests
// -----------------------------------------------------------------------------

#[test]
fn compression_choose_compression_small_payload_returns_lz4() {
    assert_eq!(choose_compression(b"tiny"), CompressionKind::Lz4);
}

#[test]
fn compression_choose_compression_large_payload_returns_zstd() {
    let payload = vec![0u8; 2048];
    assert_eq!(choose_compression(&payload), CompressionKind::Zstd);
}

#[test]
fn compression_lz4_roundtrip_returns_original() {
    let payload = b"lz4 test payload";
    let compressed = compress(payload, CompressionKind::Lz4).unwrap();
    let decompressed = decompress(&compressed, CompressionKind::Lz4).unwrap();
    assert_eq!(decompressed, payload.to_vec());
}

#[test]
fn compression_zstd_roundtrip_returns_original() {
    let payload = b"zstd test payload";
    let compressed = compress(payload, CompressionKind::Zstd).unwrap();
    let decompressed = decompress(&compressed, CompressionKind::Zstd).unwrap();
    assert_eq!(decompressed, payload.to_vec());
}

#[test]
fn compression_adaptive_roundtrip_works_for_mixed_payload() {
    let payload = vec![0u8; 2048];
    let compressed = compress(&payload, CompressionKind::Adaptive).unwrap();
    let decompressed = decompress(&compressed, CompressionKind::Adaptive).unwrap();
    assert_eq!(decompressed, payload);
}

#[test]
fn compression_decompress_rejects_invalid_payload() {
    let bad = vec![1, 2, 3, 4, 5];
    assert!(decompress(&bad, CompressionKind::Zstd).is_err());
}

#[test]
fn compression_empty_payload_returns_empty_vector() {
    assert_eq!(compress(&[], CompressionKind::Lz4).unwrap(), Vec::<u8>::new());
    assert_eq!(decompress(&[], CompressionKind::Lz4).unwrap(), Vec::<u8>::new());
}

#[test]
fn compression_adaptive_uses_zstd_for_large_payloads() {
    let payload = vec![0u8; 1500];
    let compressed = compress(&payload, CompressionKind::Adaptive).unwrap();
    let decompressed = decompress(&compressed, CompressionKind::Adaptive).unwrap();
    assert_eq!(decompressed, payload);
}

// -----------------------------------------------------------------------------
// Encryption Tests
// -----------------------------------------------------------------------------

#[test]
fn derive_key_produces_unique_keys_for_different_fingerprints() {
    let master_key = b"master key";
    let a = EncryptionConfig { column_name: "col".to_string(), schema_fingerprint: [1, 2, 3, 4, 5, 6, 7, 8] };
    let b = EncryptionConfig { column_name: "col".to_string(), schema_fingerprint: [8, 7, 6, 5, 4, 3, 2, 1] };
    assert_ne!(derive_column_key(master_key, &a).unwrap(), derive_column_key(master_key, &b).unwrap());
}

#[test]
fn encrypt_decrypt_empty_payload_returns_original_empty() {
    let config = EncryptionConfig { column_name: "col".to_string(), schema_fingerprint: [9, 9, 9, 9, 9, 9, 9, 9] };
    let key = derive_column_key(b"long master key 32 bytes........", &config).unwrap();
    let encrypted = encrypt_payload(&[], &key).unwrap();
    let decrypted = decrypt_payload(&encrypted.ciphertext, &key, &encrypted.nonce, &encrypted.auth_tag).unwrap();
    assert!(decrypted.is_empty());
}

#[test]
fn encrypt_decrypt_roundtrip_reconstructs_payload() {
    let config = EncryptionConfig { column_name: "col".to_string(), schema_fingerprint: [9, 9, 9, 9, 9, 9, 9, 9] };
    let key = derive_column_key(b"long master key 32 bytes........", &config).unwrap();
    let plaintext = b"payload to protect";
    let encrypted = encrypt_payload(plaintext, &key).unwrap();
    let decrypted = decrypt_payload(&encrypted.ciphertext, &key, &encrypted.nonce, &encrypted.auth_tag).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn decrypt_with_wrong_key_fails_authentication() {
    let config = EncryptionConfig { column_name: "col".to_string(), schema_fingerprint: [0, 0, 0, 0, 0, 0, 0, 1] };
    let key = derive_column_key(b"master key...............0101", &config).unwrap();
    let bad_key = derive_column_key(b"another master key........", &config).unwrap();
    let encrypted = encrypt_payload(b"secret", &key).unwrap();
    assert!(decrypt_payload(&encrypted.ciphertext, &bad_key, &encrypted.nonce, &encrypted.auth_tag).is_err());
}

#[test]
fn unpack_encrypted_chunk_rejects_payload_too_short() {
    let result = qrd_core::encryption::unpack_encrypted_chunk(&[0u8; 10]);
    assert!(matches!(result, Err(QrdError::UnexpectedEof)));
}

#[test]
fn pack_and_unpack_encrypted_chunk_roundtrip() {
    let chunk = qrd_core::encryption::EncryptedChunk { nonce: Nonce([1u8; 12]), auth_tag: AuthTag([2u8; 16]), ciphertext: vec![3, 4, 5] };
    let packed = qrd_core::encryption::pack_encrypted_chunk(&chunk);
    let unpacked = qrd_core::encryption::unpack_encrypted_chunk(&packed).unwrap();
    assert_eq!(unpacked, chunk);
}

#[test]
fn derive_column_key_rejects_empty_master_key() {
    let config = EncryptionConfig { column_name: "col".to_string(), schema_fingerprint: [0, 1, 2, 3, 4, 5, 6, 7] };
    assert!(derive_column_key(&[], &config).is_err());
}

#[test]
fn encryption_public_key_bytes_length_is_correct() {
    let keypair = SigningKeyPair::generate();
    assert_eq!(keypair.verifying_key().len(), 32);
}

// -----------------------------------------------------------------------------
// Signing and Integrity Tests
// -----------------------------------------------------------------------------

#[test]
fn schema_signature_serialization_is_canonical() {
    let keypair = SigningKeyPair::generate();
    let schema_id = [1u8; 8];
    let signature = keypair.sign_schema(&schema_id);
    let sig = SchemaSignature::new(SIGNATURE_ALGORITHM, signature, keypair.verifying_key());
    let serialized = sig.serialize();
    let parsed = SchemaSignature::deserialize(&serialized).unwrap();
    assert_eq!(parsed, sig);
}

#[test]
fn schema_signature_verify_rejects_wrong_algorithm() {
    let keypair = SigningKeyPair::generate();
    let schema_id = [1u8; 8];
    let signature = keypair.sign_schema(&schema_id);
    let sig = SchemaSignature::new(0xFF, signature, keypair.verifying_key());
    assert!(sig.verify(&schema_id).is_err());
}

#[test]
fn signing_verifying_key_from_bytes_roundtrip() {
    let keypair = SigningKeyPair::generate();
    let bytes = keypair.verifying_key();
    let verifying_key = VerifyingKeyPair::from_bytes(&bytes).unwrap();
    assert_eq!(verifying_key.to_bytes(), bytes);
}

#[test]
fn signature_verification_rejects_invalid_signature_bytes_length() {
    let keypair = SigningKeyPair::generate();
    let vk_bytes = keypair.verifying_key();
    let verifying_key = VerifyingKeyPair::from_bytes(&vk_bytes).unwrap();
    assert!(verifying_key.verify_signature(&[1u8; 8], &[0u8; 63]).is_err());
}

#[test]
fn crc32_works_for_empty_input() {
    assert_eq!(crc32(&[]), 0);
}

#[test]
fn verify_crc32_rejects_modified_payload() {
    let input = b"checksum check";
    let checksum = crc32(input);
    assert!(!verify_crc32(b"checksum checX", checksum));
}

#[test]
fn signature_verification_fails_for_wrong_public_key() {
    let keypair = SigningKeyPair::generate();
    let schema_id = [1u8; 8];
    let signature = keypair.sign_schema(&schema_id);
    let wrong_public_key = SigningKeyPair::generate().verifying_key();
    let sig = SchemaSignature::new(SIGNATURE_ALGORITHM, signature, wrong_public_key);
    assert!(sig.verify(&schema_id).is_err());
}

#[test]
fn verifying_key_from_bytes_rejects_invalid_public_key() {
    let bogus_key = [0u8; 32];
    let result = VerifyingKeyPair::from_bytes(&bogus_key);
    assert!(result.is_err());
}
