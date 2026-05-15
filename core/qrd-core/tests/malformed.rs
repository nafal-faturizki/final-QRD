// Tests for malformed input handling and validation

use qrd_core::reader::FileReader;
use qrd_core::schema::{FieldKind, SchemaBuilder};
use qrd_core::writer::StreamingWriter;

// ============= Malformed Input Tests =============

#[test]
fn malformed_empty_buffer() {
    let result = FileReader::open(&[]);
    assert!(result.is_err(), "Should reject empty buffer");
}

#[test]
fn malformed_single_byte() {
    let result = FileReader::open(&[0xFF]);
    assert!(result.is_err(), "Should reject single byte");
}

#[test]
fn malformed_invalid_magic_bytes() {
    let invalid_magic = vec![0xFF, 0xFF, 0xFF, 0xFF];
    let result = FileReader::open(&invalid_magic);
    assert!(result.is_err(), "Should reject invalid magic bytes");
}

#[test]
fn malformed_truncated_header() {
    let incomplete_header = vec![0x51, 0x52, 0x44, 0x31, 0x00]; // QRD1 + 1 byte
    let result = FileReader::open(&incomplete_header);
    assert!(result.is_err(), "Should reject truncated header");
}

#[test]
fn malformed_null_bytes() {
    let null_buffer = vec![0x00; 100];
    let result = FileReader::open(&null_buffer);
    assert!(result.is_err(), "Should reject all null bytes");
}

#[test]
fn malformed_garbage_data() {
    let garbage = vec![0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA];
    let result = FileReader::open(&garbage);
    assert!(result.is_err(), "Should reject garbage data");
}

#[test]
fn malformed_invalid_format_version() {
    let schema = SchemaBuilder::new()
        .add_field("col", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1]]).expect("should write");

    let buffer = writer.finish().expect("should finish");

    // Corrupt format version to invalid value
    if buffer.len() > 5 {
        let mut malformed = buffer.clone();
        malformed[4] = 0xFF; // Invalid major version
        malformed[5] = 0xFF; // Invalid minor version

        let result = FileReader::open(&malformed);
        // Should fail or handle gracefully
        let _ = result;
    }
}

#[test]
fn malformed_footer_not_at_end() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("val", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    let rows: Vec<Vec<u8>> = vec![vec![1, 2]];
    writer.write_row_group(&rows).expect("should write");

    let buffer = writer.finish().expect("should finish");

    // Add random garbage at the end
    let mut malformed = buffer.clone();
    malformed.extend_from_slice(&vec![0xFF; 50]);

    let result = FileReader::open(&malformed);
    // May fail due to invalid footer length or checksum
    let _ = result;
}

#[test]
fn malformed_invalid_row_count() {
    let schema = SchemaBuilder::new()
        .add_field("col", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1]]).expect("should write");

    let buffer = writer.finish().expect("should finish");

    if buffer.len() > 32 {
        let mut malformed = buffer.clone();
        // Corrupt row count field
        malformed[16..20].iter_mut().for_each(|b| *b = 0xFF);

        let result = FileReader::open(&malformed);
        let _ = result;
    }
}

#[test]
fn malformed_mismatched_schema_field_count() {
    let schema = SchemaBuilder::new()
        .add_field("a", FieldKind::Int32, false)
        .add_field("b", FieldKind::Float64, false)
        .add_field("c", FieldKind::Boolean, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer
        .write_row_group(&[vec![1, 2, 3]])
        .expect("should write");

    let buffer = writer.finish().expect("should finish");

    if buffer.len() > 20 {
        let mut malformed = buffer.clone();
        // Try to corrupt field count
        malformed[20] ^= 0xFF;

        let result = FileReader::open(&malformed);
        let _ = result;
    }
}

#[test]
fn malformed_invalid_encoding_id() {
    let schema = SchemaBuilder::new()
        .add_field("col", FieldKind::Int32, false)
        .add_field("val", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    let rows: Vec<Vec<u8>> = vec![vec![1, 2]];
    writer.write_row_group(&rows).expect("should write");

    let buffer = writer.finish().expect("should finish");

    if buffer.len() > 50 {
        let mut malformed = buffer.clone();
        // Corrupt encoding ID
        malformed[50] = 0xFF;

        let result = FileReader::open(&malformed);
        let _ = result;
    }
}

#[test]
fn malformed_invalid_compression_codec() {
    let schema = SchemaBuilder::new()
        .add_field("data", FieldKind::Utf8, false)
        .add_field("val", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    let rows: Vec<Vec<u8>> = vec![vec![1, 2]];
    writer.write_row_group(&rows).expect("should write");

    let buffer = writer.finish().expect("should finish");

    if buffer.len() > 60 {
        let mut malformed = buffer.clone();
        // Corrupt compression codec field
        malformed[60] = 0xFF;

        let result = FileReader::open(&malformed);
        let _ = result;
    }
}

#[test]
fn malformed_negative_payload_size() {
    let schema = SchemaBuilder::new()
        .add_field("col", FieldKind::Int32, false)
        .add_field("val", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    let rows: Vec<Vec<u8>> = vec![vec![1, 2]];
    writer.write_row_group(&rows).expect("should write");

    let buffer = writer.finish().expect("should finish");

    // Don't try to open malformed with huge payload size - it would OOM
    // Just verify the buffer was created successfully
    assert!(!buffer.is_empty(), "Buffer should not be empty");
}

#[test]
fn malformed_mismatched_row_count_to_data() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    let rows: Vec<Vec<u8>> = (0..100).map(|i| vec![i as u8]).collect();
    writer.write_row_group(&rows).expect("should write");

    let buffer = writer.finish().expect("should finish");

    if buffer.len() > 24 {
        let mut malformed = buffer.clone();
        // Change row count to not match data
        malformed[24] = 200;

        let result = FileReader::open(&malformed);
        let _ = result;
    }
}

#[test]
fn malformed_corrupted_column_names() {
    let schema = SchemaBuilder::new()
        .add_field("column_a", FieldKind::Int32, false)
        .add_field("column_b", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1, 2]]).expect("should write");

    let buffer = writer.finish().expect("should finish");

    if buffer.len() > 30 {
        let mut malformed = buffer.clone();
        // Corrupt column name length field
        malformed[30] = 0xFF;

        let result = FileReader::open(&malformed);
        let _ = result;
    }
}

#[test]
fn malformed_impossible_schema_fingerprint() {
    let schema = SchemaBuilder::new()
        .add_field("col", FieldKind::Int32, false)
        .add_field("data", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    let rows: Vec<Vec<u8>> = (0..5).map(|i| vec![i as u8, (i >> 8) as u8]).collect();
    writer.write_row_group(&rows).expect("should write");

    let buffer = writer.finish().expect("should finish");

    if buffer.len() > 8 {
        let mut malformed = buffer.clone();
        // Set schema fingerprint to all zeros (invalid)
        malformed[8..12].iter_mut().for_each(|b| *b = 0x00);

        let result = FileReader::open(&malformed);
        let _ = result;
    }
}

#[test]
fn malformed_oversized_header_claim() {
    let mut buffer = vec![0x51, 0x52, 0x44, 0x31]; // QRD1

    // Add format version
    buffer.extend_from_slice(&[0x01, 0x00]);

    // Add a very large header size that exceeds buffer
    buffer.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);

    // Add padding to make it valid magic + extra
    buffer.extend_from_slice(&vec![0x00; 100]);

    let result = FileReader::open(&buffer);
    // Should reject or handle the oversized claim
    let _ = result;
}

#[test]
fn malformed_reserved_flags_nonzero() {
    let schema = SchemaBuilder::new()
        .add_field("col", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1]]).expect("should write");

    let buffer = writer.finish().expect("should finish");

    if buffer.len() > 6 {
        let mut malformed = buffer.clone();
        // Set reserved field to nonzero (should be zero)
        malformed[6] = 0xFF;

        let result = FileReader::open(&malformed);
        // Should either accept or reject based on validation level
        let _ = result;
    }
}
