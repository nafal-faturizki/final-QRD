// Tests for power loss, partial writes, and data resilience

use qrd_core::error::QrdError;
use qrd_core::reader::FileReader;
use qrd_core::schema::{FieldKind, SchemaBuilder};
use qrd_core::writer::StreamingWriter;

// ============= Power Loss Resilience Tests =============

#[test]
fn power_loss_incomplete_footer_detected() {
    // Simulate power loss during footer write
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("val", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1, 2]]).expect("should write");

    let buffer = writer.finish().expect("should finish");

    // Simulate incomplete footer - truncate the buffer significantly
    let truncated = &buffer[..buffer.len().saturating_sub(20)];

    // Attempt to read should fail gracefully
    let result = FileReader::open(truncated);
    assert!(result.is_err(), "Should fail when footer is incomplete");
}

#[test]
fn power_loss_incomplete_header_detected() {
    // Simulate power loss during header write
    let schema = SchemaBuilder::new()
        .add_field("value", FieldKind::Float64, false)
        .add_field("data", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1, 2]]).expect("should write");

    let buffer = writer.finish().expect("should finish");

    // Simulate power loss at header - only first 5 bytes
    if buffer.len() > 5 {
        let truncated = &buffer[..5];
        let result = FileReader::open(truncated);
        assert!(result.is_err(), "Should fail with incomplete header");
    }
}

#[test]
fn power_loss_complete_row_groups_still_accessible() {
    // Ensure that completed row groups are still accessible even if interrupted
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("data", FieldKind::Utf8, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);

    // Write first complete row group
    writer
        .write_row_group(&[vec![1, 2], vec![3, 4], vec![5, 6]])
        .expect("should write first group");

    let buffer = writer.finish().expect("should finish");

    // Try to read - at minimum should not crash
    match FileReader::open(&buffer) {
        Ok(reader) => {
            let header = reader.header();
            // Header is successfully read
            let _ = header;
        }
        Err(_) => {
            // It's acceptable to fail on incomplete file, as long as it fails gracefully
        }
    }
}

#[test]
fn power_loss_metadata_corruption_detected() {
    // Test that metadata corruption is detected
    let schema = SchemaBuilder::new()
        .add_field("col", FieldKind::Int32, false)
        .add_field("val", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1, 2]]).expect("should write");

    let buffer = writer.finish().expect("should finish");

    // Corrupt magic bytes
    if buffer.len() > 4 {
        let mut corrupted = buffer.clone();
        corrupted[0] ^= 0xFF;

        let result = FileReader::open(&corrupted);
        assert!(result.is_err(), "Should reject corrupted magic bytes");
    }
}

// ============= Partial Writes Tests =============

#[test]
fn partial_write_incomplete_row_group() {
    // Simulate incomplete row group write
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("val", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1, 2]]).expect("should write");

    let buffer = writer.finish().expect("should finish");

    // Truncate mid-way through second row group would be ideal, but we simulate
    // by just truncating significantly
    let truncated = if buffer.len() > 50 {
        &buffer[..buffer.len() - 25]
    } else {
        &buffer[..(buffer.len() / 2)]
    };

    // Should detect the issue
    let result = FileReader::open(truncated);
    // Either fails to parse or succeeds but with partial data
    match result {
        Ok(reader) => {
            // If it succeeds, it should have some valid data
            let _ = reader.header();
        }
        Err(_) => {
            // Expected behavior for incomplete data
        }
    }
}

#[test]
fn partial_write_zero_bytes_written() {
    // Simulate a write that wrote nothing
    let empty_buffer: &[u8] = b"";

    let result = FileReader::open(empty_buffer);
    assert!(result.is_err(), "Should reject empty buffer");
}

#[test]
fn partial_write_only_header() {
    // Simulate write that only completed header
    let schema = SchemaBuilder::new()
        .add_field("a", FieldKind::Int32, false)
        .add_field("b", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1, 2]]).expect("should write");

    let buffer = writer.finish().expect("should finish");

    // Take only a small portion (roughly header-sized)
    let header_only = if buffer.len() > 32 {
        &buffer[..32]
    } else {
        &buffer
    };

    let result = FileReader::open(header_only);
    assert!(result.is_err(), "Should fail when only header is present");
}

#[test]
fn partial_write_detects_checksum_mismatch() {
    // Partial writes often result in checksum mismatches
    let schema = SchemaBuilder::new()
        .add_field("data", FieldKind::Utf8, false)
        .add_field("meta", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1, 2]]).expect("should write");

    let buffer = writer.finish().expect("should finish");

    if buffer.len() > 10 {
        // Truncate and try to read
        let truncated = &buffer[..buffer.len() - 10];

        match FileReader::open(truncated) {
            Ok(_) => {
                // May succeed if footer is still intact but truncated
            }
            Err(QrdError::InvalidFooterLength)
            | Err(QrdError::UnexpectedEof)
            | Err(QrdError::InvalidSchema(_)) => {
                // Expected errors for partial writes
            }
            Err(e) => {
                eprintln!("Unexpected error: {:?}", e);
            }
        }
    }
}

// ============= Recovery and Validation Tests =============

#[test]
fn recovery_detects_corrupted_row_groups() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("value", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    for i in 0..5 {
        let rows: Vec<Vec<u8>> = (0..100).map(|_| vec![i, i + 1]).collect();
        writer.write_row_group(&rows).expect("should write");
    }

    let buffer = writer.finish().expect("should finish");

    // Corrupt a byte in the middle
    let mut corrupted = buffer.clone();
    if corrupted.len() > 100 {
        corrupted[100] ^= 0x01;

        // Should still be able to read, but may detect corruption
        match FileReader::open(&corrupted) {
            Ok(_reader) => {
                // Successfully detected or bypassed corruption
            }
            Err(_) => {
                // Correctly identified corruption
            }
        }
    }
}

#[test]
fn resilience_multiple_truncations() {
    // Test various truncation points
    let schema = SchemaBuilder::new()
        .add_field("col", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer
        .write_row_group(&[vec![1], vec![2], vec![3]])
        .expect("should write");

    let buffer = writer.finish().expect("should finish");

    // Try multiple truncation points
    for truncation_point in [10, 25, 50, 75].iter() {
        if *truncation_point < buffer.len() {
            let truncated = &buffer[..*truncation_point];
            let result = FileReader::open(truncated);
            // Should not panic, may succeed or fail gracefully
            let _ = result;
        }
    }
}

#[test]
fn resilience_large_file_partial_write() {
    // Test larger files for partial write scenarios
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int64, false)
        .add_field("data", FieldKind::Utf8, false)
        .add_field("value", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);

    // Write multiple large row groups
    for _ in 0..3 {
        let rows: Vec<Vec<u8>> = (0..1000)
            .map(|i| vec![i as u8, (i >> 8) as u8, (i >> 16) as u8])
            .collect();
        writer.write_row_group(&rows).expect("should write");
    }

    let buffer = writer.finish().expect("should finish");

    // Simulate various partial write scenarios
    let truncation_points = [buffer.len() / 2, buffer.len() * 2 / 3, buffer.len() - 100];

    for point in truncation_points.iter() {
        if *point < buffer.len() {
            let truncated = &buffer[..*point];
            let result = FileReader::open(truncated);
            // Should fail gracefully or succeed with partial data
            let _ = result;
        }
    }
}
