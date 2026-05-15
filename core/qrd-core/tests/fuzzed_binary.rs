// Tests for fuzzed binary input handling and robustness

use qrd_core::reader::FileReader;
use qrd_core::schema::{FieldKind, SchemaBuilder};
use qrd_core::writer::StreamingWriter;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// ============= Fuzzed Binary Tests =============

#[test]
fn fuzz_random_bytes_valid_magic() {
    // Create buffer with valid magic but random data
    let mut buffer = vec![0x51, 0x52, 0x44, 0x31]; // QRD1

    // Add random data
    let random_data = vec![
        0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE,
        0xF0, 0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88,
    ];
    buffer.extend_from_slice(&random_data);

    let result = FileReader::open(&buffer);
    // Should fail gracefully
    let _ = result;
}

#[test]
fn fuzz_alternating_bit_pattern() {
    // Test alternating bit patterns
    let patterns = vec![
        vec![0xAA; 100], // 10101010
        vec![0x55; 100], // 01010101
        vec![0xFF; 100], // 11111111
        vec![0x00; 100], // 00000000
    ];

    for pattern in patterns {
        let result = FileReader::open(&pattern);
        // Should not crash
        let _ = result;
    }
}

#[test]
fn fuzz_gradually_increasing_bytes() {
    let mut buffer = Vec::new();
    for i in 0..256 {
        buffer.push(i as u8);
    }

    let result = FileReader::open(&buffer);
    let _ = result;
}

#[test]
fn fuzz_gradually_decreasing_bytes() {
    let mut buffer = Vec::new();
    for i in (0..256).rev() {
        buffer.push(i as u8);
    }

    let result = FileReader::open(&buffer);
    let _ = result;
}

#[test]
fn fuzz_random_bytes_with_valid_header() {
    // Start with valid header then random data
    let schema = SchemaBuilder::new()
        .add_field("col", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1]]).expect("should write");

    let mut buffer = Vec::new();
    writer.finish().expect("should finish");

    // Keep valid header, randomize the rest
    if buffer.len() > 32 {
        for i in 32..buffer.len() {
            buffer[i] = ((i as u32).wrapping_mul(31)) as u8;
        }
    }

    let result = FileReader::open(&buffer);
    let _ = result;
}

#[test]
fn fuzz_sparse_data() {
    // Test with mostly zeros and occasional non-zero bytes
    let mut buffer = vec![0x00; 1000];

    // Insert random values at specific positions
    for i in [10, 50, 100, 200, 500, 999].iter() {
        if *i < buffer.len() {
            buffer[*i] = 0xFF;
        }
    }

    let result = FileReader::open(&buffer);
    let _ = result;
}

#[test]
fn fuzz_dense_data() {
    // Test with highly random/entropy data
    let mut buffer = Vec::new();
    let seed = 0x12345678u32;

    for i in 0..1000 {
        let mut hasher = DefaultHasher::new();
        (seed.wrapping_add(i as u32)).hash(&mut hasher);
        let hash = hasher.finish();
        buffer.push((hash & 0xFF) as u8);
    }

    let result = FileReader::open(&buffer);
    let _ = result;
}

#[test]
fn fuzz_repeating_patterns() {
    // Test repeating patterns
    let patterns = vec![
        vec![0x01, 0x02],
        vec![0xAA, 0xBB, 0xCC, 0xDD],
        vec![1, 2, 3, 4, 5],
    ];

    for pattern in patterns {
        let mut buffer = Vec::new();
        for _ in 0..(500 / pattern.len()) {
            buffer.extend_from_slice(&pattern);
        }

        let result = FileReader::open(&buffer);
        let _ = result;
    }
}

#[test]
fn fuzz_variable_length_buffers() {
    // Test with various buffer sizes
    for size in [1, 2, 3, 5, 8, 16, 32, 64, 128, 256, 512, 1024, 2048].iter() {
        let buffer: Vec<u8> = (0..*size)
            .map(|i| ((i as u32).wrapping_mul(17)) as u8)
            .collect();

        let result = FileReader::open(&buffer);
        let _ = result;
    }
}

#[test]
fn fuzz_all_single_byte_values() {
    // Test each single-byte value
    for byte_val in 0..=255u8 {
        let buffer = vec![byte_val; 100];
        let result = FileReader::open(&buffer);
        let _ = result;
    }
}

#[test]
fn fuzz_byte_pairs() {
    // Test repeating byte pairs
    for byte1 in [0x00u8, 0x55, 0xAA, 0xFF] {
        for byte2 in [0x00u8, 0x55, 0xAA, 0xFF] {
            let mut buffer = Vec::new();
            for _ in 0..50 {
                buffer.push(byte1);
                buffer.push(byte2);
            }

            let result = FileReader::open(&buffer);
            let _ = result;
        }
    }
}

#[test]
fn fuzz_mixed_valid_and_invalid_headers() {
    // Test with partially valid headers
    let valid_magic = vec![0x51, 0x52, 0x44, 0x31];

    let mut buffer = valid_magic.clone();
    buffer.extend_from_slice(&[0xFF; 96]);

    let result = FileReader::open(&buffer);
    let _ = result;
}

#[test]
fn fuzz_highly_compressible_data() {
    // All same byte - highly compressible
    let buffer = vec![0x42; 10000];
    let result = FileReader::open(&buffer);
    let _ = result;
}

#[test]
fn fuzz_incompressible_data() {
    // Random-looking incompressible data
    let mut buffer = Vec::new();
    let mut seed = 0x12345678u32;

    for _ in 0..10000 {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        buffer.push((seed >> 24) as u8);
    }

    let result = FileReader::open(&buffer);
    let _ = result;
}

#[test]
fn fuzz_boundary_patterns() {
    // Test patterns at boundaries between valid and invalid ranges
    let test_values = vec![
        vec![0x7F; 100], // Max signed byte
        vec![0x80; 100], // Min signed byte
        vec![0xFF; 100], // Max unsigned byte
        vec![0x00; 100], // Min unsigned byte
    ];

    for pattern in test_values {
        let result = FileReader::open(&pattern);
        let _ = result;
    }
}

#[test]
fn fuzz_nested_repetitions() {
    // Test nested repeating patterns
    let base = vec![1u8, 2, 3];
    let mut buffer = Vec::new();

    for _ in 0..10 {
        for _ in 0..10 {
            buffer.extend_from_slice(&base);
        }
    }

    let result = FileReader::open(&buffer);
    let _ = result;
}

#[test]
fn fuzz_entropy_increase() {
    // Gradually increase entropy
    let mut buffer = Vec::new();

    // First part: low entropy
    buffer.extend_from_slice(&vec![0x00; 50]);
    buffer.extend_from_slice(&vec![0xFF; 50]);

    // Middle part: medium entropy
    for i in 0..100 {
        buffer.push((i % 256) as u8);
    }

    // Last part: high entropy
    let mut seed = 0xDEADBEEFu32;
    for _ in 0..100 {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        buffer.push(seed as u8);
    }

    let result = FileReader::open(&buffer);
    let _ = result;
}

#[test]
fn fuzz_zero_then_max_pattern() {
    let mut buffer = Vec::new();

    for _ in 0..500 {
        buffer.push(0x00);
        buffer.push(0xFF);
    }

    let result = FileReader::open(&buffer);
    let _ = result;
}

#[test]
fn fuzz_pseudo_valid_header() {
    // Almost valid header with slight modifications
    let mut buffer = vec![0x51, 0x52, 0x44, 0x31]; // QRD1

    // Valid format version
    buffer.extend_from_slice(&[0x01, 0x00]);

    // Random data that might coincidentally match field offsets
    for _ in 0..1000 {
        buffer.push(0x42);
    }

    let result = FileReader::open(&buffer);
    let _ = result;
}

#[test]
fn fuzz_compressed_lookalikes() {
    // Data that might look like valid compression headers
    let lz4_lookalike = vec![0x04, 0x22, 0x4D, 0x18]; // LZ4 frame magic
    let mut buffer = lz4_lookalike.clone();
    buffer.extend_from_slice(&vec![0xFF; 1000]);

    let result = FileReader::open(&buffer);
    let _ = result;
}

#[test]
fn fuzz_multiple_fuzzed_passes() {
    // Apply multiple rounds of fuzzing
    let schema = SchemaBuilder::new()
        .add_field("col", FieldKind::Int32, false)
        .add_field("val", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    let rows: Vec<Vec<u8>> = (0..10).map(|i| vec![i as u8, (i >> 8) as u8]).collect();
    writer.write_row_group(&rows).expect("should write");

    let buffer = writer.finish().expect("should finish");

    // Apply multiple fuzzing passes
    for pass in 0..3 {
        let mut fuzzed = buffer.clone();

        // Fuzz with different seed each pass
        for i in 0..fuzzed.len() {
            let seed = ((pass as u32) << 16) | (i as u32);
            let mut hasher = DefaultHasher::new();
            seed.hash(&mut hasher);
            fuzzed[i] ^= hasher.finish() as u8;
        }

        let result = FileReader::open(&fuzzed);
        let _ = result;
    }
}
