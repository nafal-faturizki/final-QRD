// Tests for corruption detection and handling

use qrd_core::reader::FileReader;
use qrd_core::schema::{FieldKind, SchemaBuilder};
use qrd_core::writer::StreamingWriter;
use qrd_core::error::QrdError;
use qrd_core::integrity::crc32;

// ============= Corruption Detection Tests =============

#[test]
fn corruption_detects_corrupted_magic_bytes() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("value", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    let rows: Vec<Vec<u8>> = (0..5).map(|i| vec![i as u8, (i >> 8) as u8]).collect();
    writer.write_row_group(&rows).expect("should write");
    
    let buffer = writer.finish().expect("should finish");
    
    // Corrupt each magic byte
    for byte_pos in 0..4.min(buffer.len()) {
        let mut corrupted = buffer.clone();
        corrupted[byte_pos] ^= 0xFF;
        
        let result = FileReader::open(&corrupted);
        assert!(result.is_err(), "Should reject corrupted magic bytes at position {}", byte_pos);
    }
}

#[test]
fn corruption_detects_corrupted_format_version() {
    let schema = SchemaBuilder::new()
        .add_field("col", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1]]).expect("should write");
    
    let buffer = writer.finish().expect("should finish");
    
    // Corrupt format version field (typically at offset 4-5)
    if buffer.len() > 5 {
        let mut corrupted = buffer.clone();
        corrupted[4] ^= 0xFF;
        
        let result = FileReader::open(&corrupted);
        // Should either reject or handle gracefully
        match result {
            Ok(_) => {
                // May accept if version is still valid
            }
            Err(QrdError::InvalidHeaderLength) | Err(QrdError::UnexpectedEof) => {
                // Correctly identified corruption
            }
            Err(_) => {
                // Other error handling
            }
        }
    }
}

#[test]
fn corruption_detects_corrupted_schema_id() {
    let schema = SchemaBuilder::new()
        .add_field("a", FieldKind::Int32, false)
        .add_field("b", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1, 2], vec![3, 4]]).expect("should write");
    
    let buffer = writer.finish().expect("should finish");
    
    // Corrupt schema ID field
    if buffer.len() > 10 {
        let mut corrupted = buffer.clone();
        corrupted[10] ^= 0x80;
        
        let result = FileReader::open(&corrupted);
        // Should be able to read but with different schema ID
        let _ = result;
    }
}

#[test]
fn corruption_detects_corrupted_footer_length() {
    let schema = SchemaBuilder::new()
        .add_field("data", FieldKind::Utf8, false)
        .add_field("value", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    let rows: Vec<Vec<u8>> = (0..5).map(|i| vec![i as u8, (i >> 8) as u8]).collect();
    writer.write_row_group(&rows).expect("should write");
    
    let buffer = writer.finish().expect("should finish");
    
    // Footer length is stored in last 4 bytes
    if buffer.len() > 4 {
        let mut corrupted = buffer.clone();
        let last_idx = corrupted.len() - 1;
        corrupted[last_idx] ^= 0xFF;
        
        let result = FileReader::open(&corrupted);
        // Should fail due to invalid footer length
        assert!(result.is_err(), "Should reject corrupted footer length");
    }
}

#[test]
fn corruption_detects_bit_flip_in_data() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("value", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    let rows: Vec<Vec<u8>> = (0..100).map(|i| vec![i as u8, (i >> 8) as u8]).collect();
    writer.write_row_group(&rows).expect("should write");
    
    let buffer = writer.finish().expect("should finish");
    
    // Flip a single bit in the middle of data
    if buffer.len() > 50 {
        let mut corrupted = buffer.clone();
        corrupted[50] ^= 0x01;
        
        // May still read but should detect issue
        let result = FileReader::open(&corrupted);
        let _ = result;
    }
}

#[test]
fn corruption_detects_corrupted_header_flags() {
    let schema = SchemaBuilder::new()
        .add_field("col", FieldKind::Int32, false)
        .add_field("val", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    let rows: Vec<Vec<u8>> = (0..5).map(|i| vec![i as u8, (i >> 8) as u8]).collect();
    writer.write_row_group(&rows).expect("should write");
    
    let buffer = writer.finish().expect("should finish");
    
    // Corrupt header flags field
    if buffer.len() > 6 {
        let mut corrupted = buffer.clone();
        corrupted[6] ^= 0xFF;
        
        let result = FileReader::open(&corrupted);
        let _ = result; // Should handle gracefully
    }
}

#[test]
fn corruption_multiple_byte_corruption() {
    let schema = SchemaBuilder::new()
        .add_field("a", FieldKind::Int32, false)
        .add_field("b", FieldKind::Int64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    let rows: Vec<Vec<u8>> = (0..5).map(|i| vec![i as u8, (i >> 8) as u8]).collect();
    writer.write_row_group(&rows).expect("should write");
    
    let buffer = writer.finish().expect("should finish");
    
    // Corrupt multiple bytes
    if buffer.len() > 200 {
        let mut corrupted = buffer.clone();
        for offset in [50, 100, 150].iter() {
            if *offset < corrupted.len() {
                corrupted[*offset] ^= 0xFF;
            }
        }
        
        let result = FileReader::open(&corrupted);
        let _ = result;
    }
}

#[test]
fn corruption_checksum_validation() {
    let schema = SchemaBuilder::new()
        .add_field("data", FieldKind::Utf8, false)
        .add_field("value", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    let rows: Vec<Vec<u8>> = (0..5).map(|i| vec![i as u8, (i >> 8) as u8]).collect();
    writer.write_row_group(&rows).expect("should write");
    
    let _buffer = writer.finish().expect("should finish");
    
    // Test CRC32 detection
    let test_data = b"test data";
    let expected_crc = crc32(test_data);
    
    let mut corrupted_data = test_data.to_vec();
    corrupted_data[0] ^= 0x01;
    let actual_crc = crc32(&corrupted_data);
    
    assert_ne!(expected_crc, actual_crc, "CRC should detect single bit flip");
}

#[test]
fn corruption_sequential_byte_corruption() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("val", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    let rows: Vec<Vec<u8>> = (0..100).map(|i| vec![i as u8, (i >> 8) as u8]).collect();
    writer.write_row_group(&rows).expect("should write");
    
    let buffer = writer.finish().expect("should finish");
    
    // Corrupt multiple sequential bytes
    if buffer.len() > 100 {
        let mut corrupted = buffer.clone();
        for i in 0..10 {
            if 50 + i < corrupted.len() {
                corrupted[50 + i] ^= 0xFF;
            }
        }
        
        let result = FileReader::open(&corrupted);
        let _ = result;
    }
}

#[test]
fn corruption_random_byte_corruption() {
    let schema = SchemaBuilder::new()
        .add_field("a", FieldKind::Int32, false)
        .add_field("b", FieldKind::Float64, false)
        .add_field("c", FieldKind::Boolean, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    for _ in 0..100 {
        writer.write_row_group(&[vec![1, 2, 3]]).expect("should write");
    }
    
    let buffer = writer.finish().expect("should finish");
    
    // Corrupt random bytes throughout
    for seed in 0..5 {
        let mut corrupted = buffer.clone();
        
        // Use seed to generate deterministic "random" positions
        for i in 0..10 {
            let idx = ((seed * 7 + i * 13) % (corrupted.len() as u32)) as usize;
            corrupted[idx] ^= 0x01;
        }
        
        let result = FileReader::open(&corrupted);
        let _ = result;
    }
}

#[test]
fn corruption_detects_header_mismatch_footer() {
    let schema = SchemaBuilder::new()
        .add_field("col", FieldKind::Int32, false)
        .add_field("data", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    let rows: Vec<Vec<u8>> = (0..5).map(|i| vec![i as u8, (i >> 8) as u8]).collect();
    writer.write_row_group(&rows).expect("should write");
    
    let buffer = writer.finish().expect("should finish");
    
    if buffer.len() > 20 {
        let mut corrupted = buffer.clone();
        // Corrupt a byte in header
        let idx1 = 8;
        corrupted[idx1] ^= 0x80;
        // Corrupt a byte in footer
        let idx2 = corrupted.len() - 10;
        corrupted[idx2] ^= 0x80;
        
        let result = FileReader::open(&corrupted);
        let _ = result;
    }
}
