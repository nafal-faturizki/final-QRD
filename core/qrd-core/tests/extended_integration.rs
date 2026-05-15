// Extended integration tests for qrd-core

use qrd_core::compression::{compress, decompress, CompressionKind};
use qrd_core::reader::FileReader;
use qrd_core::schema::{FieldKind, SchemaBuilder};
use qrd_core::writer::StreamingWriter;

// ============= Schema Tests =============

#[test]
fn schema_single_field() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .build()
        .expect("should build");

    assert_eq!(schema.fields().len(), 1);
}

#[test]
fn schema_many_fields() {
    let mut builder = SchemaBuilder::new();
    for i in 0..20 {
        builder = builder.add_field(&format!("col{}", i), FieldKind::Int32, false);
    }
    let schema = builder.build().expect("should build");
    assert_eq!(schema.fields().len(), 20);
}

#[test]
fn schema_nullable_fields() {
    let schema = SchemaBuilder::new()
        .add_field("a", FieldKind::Int32, true)
        .add_field("b", FieldKind::Int64, false)
        .build()
        .expect("should build");

    assert!(schema.fields()[0].required);
    assert!(!schema.fields()[1].required);
}

#[test]
fn schema_mixed_types() {
    let schema = SchemaBuilder::new()
        .add_field("int32_col", FieldKind::Int32, false)
        .add_field("int64_col", FieldKind::Int64, false)
        .add_field("float32_col", FieldKind::Float32, false)
        .add_field("float64_col", FieldKind::Float64, false)
        .add_field("bool_col", FieldKind::Boolean, false)
        .add_field("utf8_col", FieldKind::Utf8, false)
        .build()
        .expect("should build");

    assert_eq!(schema.fields().len(), 6);
}

// ============= Write Tests =============

#[test]
fn write_single_row_group() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1]]).expect("should write");
    assert_eq!(writer.row_groups().len(), 1);
}

#[test]
fn write_large_row_group() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("value", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    let large_rows: Vec<Vec<u8>> = (0..5000).map(|_| vec![1, 2]).collect();
    writer.write_row_group(&large_rows).expect("should write");
    assert_eq!(writer.row_groups().len(), 1);
}

#[test]
fn write_multiple_row_groups() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("value", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    for _ in 0..10 {
        let rows: Vec<Vec<u8>> = (0..500).map(|_| vec![1, 2]).collect();
        writer.write_row_group(&rows).expect("should write");
    }
    assert_eq!(writer.row_groups().len(), 10);
}

// ============= Read/Write Consistency Tests =============

#[test]
fn read_write_basic() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("value", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema.clone());
    let rows: Vec<Vec<u8>> = (0..100).map(|i| vec![i as u8, (i >> 8) as u8]).collect();
    writer.write_row_group(&rows).expect("should write");

    let bytes = writer.finish().expect("should finish");
    let reader = FileReader::open(&bytes).expect("should open");
    assert_eq!(reader.row_count(), 100);
}

#[test]
fn read_write_large_dataset() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("value", FieldKind::Float64, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema.clone());
    let rows: Vec<Vec<u8>> = (0..2000).map(|i| vec![i as u8, (i >> 8) as u8]).collect();
    writer.write_row_group(&rows).expect("should write");

    let bytes = writer.finish().expect("should finish");
    let reader = FileReader::open(&bytes).expect("should open");
    assert_eq!(reader.row_count(), 2000);
}

// ============= Compression Tests =============

#[test]
fn compress_lz4_basic() {
    let data = b"test data".to_vec();
    let result = compress(&data, CompressionKind::Lz4);
    assert!(result.is_ok());
}

#[test]
fn compress_zstd_basic() {
    let data = b"test data".to_vec();
    let result = compress(&data, CompressionKind::Zstd);
    assert!(result.is_ok());
}

#[test]
fn decompress_lz4_roundtrip() {
    let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
    let compressed = compress(&data, CompressionKind::Lz4).expect("should compress");
    let decompressed = decompress(&compressed, CompressionKind::Lz4).expect("should decompress");
    assert_eq!(data, decompressed);
}

#[test]
fn decompress_zstd_roundtrip() {
    let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
    let compressed = compress(&data, CompressionKind::Zstd).expect("should compress");
    let decompressed = decompress(&compressed, CompressionKind::Zstd).expect("should decompress");
    assert_eq!(data, decompressed);
}

#[test]
fn compression_large_payload() {
    let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
    let compressed_lz4 = compress(&data, CompressionKind::Lz4).expect("should compress");
    let compressed_zstd = compress(&data, CompressionKind::Zstd).expect("should compress");

    assert!(compressed_lz4.len() < data.len());
    assert!(compressed_zstd.len() < data.len());
}

#[test]
fn compression_repeated_data() {
    let data: Vec<u8> = vec![42; 5000];
    let compressed_lz4 = compress(&data, CompressionKind::Lz4).expect("should compress");
    let compressed_zstd = compress(&data, CompressionKind::Zstd).expect("should compress");

    assert!(compressed_lz4.len() < data.len() / 10);
    assert!(compressed_zstd.len() < data.len() / 10);
}

// ============= Column Selection Tests =============

#[test]
fn reader_select_single_column() {
    let schema = SchemaBuilder::new()
        .add_field("c1", FieldKind::Int32, false)
        .add_field("c2", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema.clone());
    writer.write_row_group(&[vec![1, 2]]).expect("should write");

    let bytes = writer.finish().expect("should finish");
    let reader = FileReader::open(&bytes).expect("should open");

    let cols = reader.read_columns(&["c1"]).expect("should read");
    assert_eq!(cols.len(), 1);
}

#[test]
fn reader_select_multiple_columns() {
    let schema = SchemaBuilder::new()
        .add_field("c1", FieldKind::Int32, false)
        .add_field("c2", FieldKind::Int32, false)
        .add_field("c3", FieldKind::Int32, false)
        .add_field("c4", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema.clone());
    writer
        .write_row_group(&[vec![1, 2, 3, 4], vec![5, 6, 7, 8]])
        .expect("should write");

    let bytes = writer.finish().expect("should finish");
    let reader = FileReader::open(&bytes).expect("should open");

    let cols = reader.read_columns(&["c2", "c4"]).expect("should read");
    assert_eq!(cols.len(), 2);
}

#[test]
fn reader_select_different_order() {
    let schema = SchemaBuilder::new()
        .add_field("c1", FieldKind::Int32, false)
        .add_field("c2", FieldKind::Int32, false)
        .add_field("c3", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema.clone());
    writer
        .write_row_group(&[vec![1, 2, 3]])
        .expect("should write");

    let bytes = writer.finish().expect("should finish");
    let reader = FileReader::open(&bytes).expect("should open");

    let cols = reader.read_columns(&["c3", "c1"]).expect("should read");
    assert_eq!(cols.len(), 2);
}

// ============= File Header Tests =============

#[test]
fn file_header_magic_bytes() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("test", FieldKind::Utf8, true)
        .build()
        .expect("schema should build");

    let mut writer = StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1, 2]]).expect("should write");
    let bytes = writer.finish().expect("should finish");

    assert_eq!(bytes[0], 0x51); // Q
    assert_eq!(bytes[1], 0x52); // R
    assert_eq!(bytes[2], 0x44); // D
    assert_eq!(bytes[3], 0x00); // NULL
}

#[test]
fn file_header_multiple_row_groups() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    for i in 0..5 {
        writer
            .write_row_group(&[vec![i as u8]])
            .expect("should write");
    }

    let bytes = writer.finish().expect("finish should work");
    let reader = FileReader::open(&bytes).expect("should open");
    assert_eq!(reader.row_count(), 5);
}

// ============= Reader Schema Tests =============

#[test]
fn reader_inspect_schema() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("name", FieldKind::Utf8, true)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema.clone());
    writer.write_row_group(&[vec![1, 2]]).expect("should write");

    let bytes = writer.finish().expect("should finish");
    let reader = FileReader::open(&bytes).expect("should open");

    let read_schema = reader.schema();
    assert_eq!(read_schema.fields().len(), 2);
    assert_eq!(read_schema.fields()[0].name, "id");
    assert_eq!(read_schema.fields()[1].name, "name");
}

#[test]
fn reader_row_count() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema);
    writer.write_row_group(&[vec![1]]).expect("should write");

    let bytes = writer.finish().expect("should finish");
    let reader = FileReader::open(&bytes).expect("should open");
    assert_eq!(reader.row_count(), 1);
}

// ============= Integration Tests =============

#[test]
fn full_pipeline_many_fields_many_rows() {
    let mut builder = SchemaBuilder::new();
    for i in 0..50 {
        builder = builder.add_field(&format!("f{}", i), FieldKind::Int32, false);
    }
    let schema = builder.build().expect("should build");

    let mut writer = StreamingWriter::new(schema.clone());
    let rows: Vec<Vec<u8>> = (0..1000)
        .map(|i| (0..50).map(|j| ((i + j) % 256) as u8).collect())
        .collect();
    writer.write_row_group(&rows).expect("should write");

    let bytes = writer.finish().expect("should finish");
    let reader = FileReader::open(&bytes).expect("should open");

    assert_eq!(reader.row_count(), 1000);
    assert_eq!(reader.schema().fields().len(), 50);
}

#[test]
fn multiple_schemas() {
    for schema_size in &[1, 5, 10, 25] {
        let mut builder = SchemaBuilder::new();
        for i in 0..*schema_size {
            builder = builder.add_field(&format!("f{}", i), FieldKind::Int32, false);
        }
        let schema = builder.build().expect("should build");

        let mut writer = StreamingWriter::new(schema);
        let rows: Vec<Vec<u8>> = (0..50)
            .map(|_| (0..*schema_size).map(|_| 1u8).collect())
            .collect();
        writer.write_row_group(&rows).expect("should write");
        let bytes = writer.finish().expect("should finish");

        assert!(!bytes.is_empty());
    }
}

#[test]
fn stress_test_row_groups() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .build()
        .expect("should build");

    let mut writer = StreamingWriter::new(schema.clone());
    for _ in 0..50 {
        let rows: Vec<Vec<u8>> = (0..50).map(|_| vec![42]).collect();
        writer.write_row_group(&rows).expect("should write");
    }

    let reader = FileReader::new(schema);
    for rg in writer.row_groups() {
        let _ = reader.read_row_group(rg);
    }
}
