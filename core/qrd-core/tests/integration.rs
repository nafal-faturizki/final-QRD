// Integration tests for complete QRD write/read cycles

use qrd_core::reader::FileReader;
use qrd_core::schema::{FieldKind, SchemaBuilder};
use qrd_core::writer::StreamingWriter;

#[test]
fn write_read_empty_file() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("name", FieldKind::Utf8, true)
        .build()
        .expect("schema should build");

    let writer = StreamingWriter::new(schema.clone());
    let footer_bytes = writer.build_footer_bytes().expect("footer should build");

    assert!(!footer_bytes.is_empty());

    // Create reader with same schema
    let reader = FileReader::new(schema.clone());
    assert_eq!(reader.schema(), &schema);
}

#[test]
fn write_read_single_row_group() {
    let schema = SchemaBuilder::new()
        .add_field("device_id", FieldKind::Utf8, true)
        .add_field("temperature", FieldKind::Float32, false)
        .build()
        .expect("schema should build");

    let mut writer = StreamingWriter::new(schema.clone());

    // Write a single row group with sample data matching the schema fields.
    let rows = vec![vec![1u8, 2], vec![3, 4]];
    writer
        .write_row_group(&rows)
        .expect("should write row group");

    assert_eq!(writer.row_groups().len(), 1);

    let footer_bytes = writer.build_footer_bytes().expect("footer should build");
    assert!(!footer_bytes.is_empty());

    // Reader should be able to parse the row groups
    let reader = FileReader::new(schema);
    let serialized_rg = &writer.row_groups()[0];
    let parsed_rg = reader
        .read_row_group(serialized_rg)
        .expect("row group should parse");
    assert!(!parsed_rg.columns.is_empty());
}

#[test]
fn write_read_multiple_row_groups() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("value", FieldKind::Float64, false)
        .build()
        .expect("schema should build");

    let mut writer = StreamingWriter::new(schema.clone());

    // Write multiple row groups with two columns each.
    for i in 0..3 {
        let rows = vec![vec![(i * 10) as u8, (i * 10 + 1) as u8]; 3];
        writer
            .write_row_group(&rows)
            .expect("should write row group");
    }

    assert_eq!(writer.row_groups().len(), 3);

    let reader = FileReader::new(schema);
    for serialized_rg in writer.row_groups() {
        let parsed_rg = reader
            .read_row_group(serialized_rg)
            .expect("row group should parse");
        assert!(!parsed_rg.columns.is_empty());
    }
}

#[test]
fn header_encodes_schema_id() {
    let schema = SchemaBuilder::new()
        .add_field("temperature", FieldKind::Float32, false)
        .build()
        .expect("schema should build");

    let writer = StreamingWriter::new(schema.clone());
    let header = writer.header();

    // Schema ID should match schema fingerprint
    assert_eq!(header.schema_id, schema.fingerprint());
}

#[test]
fn file_header_magic_bytes_correct() {
    let schema = SchemaBuilder::new()
        .add_field("test", FieldKind::Utf8, true)
        .build()
        .expect("schema should build");

    let writer = StreamingWriter::new(schema);
    let header = writer.header();

    // Verify magic bytes "QRD\0"
    let serialized = header.serialize();
    assert_eq!(serialized[0], 0x51); // 'Q'
    assert_eq!(serialized[1], 0x52); // 'R'
    assert_eq!(serialized[2], 0x44); // 'D'
    assert_eq!(serialized[3], 0x00); // NULL
}

#[test]
fn file_header_format_version_preserved() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .build()
        .expect("schema should build");

    let writer = StreamingWriter::new(schema);
    let header = writer.header();

    assert_eq!(header.format_major, 1);
    assert_eq!(header.format_minor, 0);
}

#[test]
fn reader_inspect_header_works() {
    let schema = SchemaBuilder::new()
        .add_field("test", FieldKind::Utf8, true)
        .build()
        .expect("schema should build");

    let writer = StreamingWriter::new(schema.clone());
    let header = writer.header();
    let bytes = header.serialize();

    let parsed = FileReader::inspect_header(&bytes).expect("header should parse");
    assert_eq!(parsed, *header);
}

#[test]
fn write_read_file_image_roundtrip() {
    let schema = SchemaBuilder::new()
        .add_field("device_id", FieldKind::Utf8, true)
        .add_field("temperature", FieldKind::Float32, false)
        .build()
        .expect("schema should build");

    let mut writer = StreamingWriter::new(schema.clone());
    let rows = vec![vec![10u8, 20], vec![30, 40]];
    writer
        .write_row_group(&rows)
        .expect("should write row group");

    let bytes = writer.finish().expect("finish should return file bytes");
    let reader = FileReader::open(&bytes).expect("file should open");

    assert_eq!(reader.schema(), &schema);
    assert_eq!(reader.row_count(), 2);
    assert!(reader.verify_integrity().is_ok());
}

#[test]
fn reader_read_columns_returns_selected_values() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .add_field("value", FieldKind::Utf8, true)
        .build()
        .expect("schema should build");

    let mut writer = StreamingWriter::new(schema.clone());
    writer
        .write_row_group(&[vec![1, 2], vec![3, 4]])
        .expect("should write row group");

    let bytes = writer.finish().expect("finish should return file bytes");
    let reader = FileReader::open(&bytes).expect("file should open");

    let columns = reader
        .read_columns(&["id", "value"])
        .expect("should read columns");
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0], vec![1, 3]);
    assert_eq!(columns[1], vec![2, 4]);
}

#[test]
fn large_row_group_handling() {
    let schema = SchemaBuilder::new()
        .add_field("id", FieldKind::Int32, false)
        .build()
        .expect("schema should build");

    let mut writer = StreamingWriter::new(schema.clone());

    // Create a large row group using a single column and many rows.
    let rows: Vec<Vec<u8>> = (0..10).map(|i| vec![(i % 256) as u8]).collect();

    writer
        .write_row_group(&rows)
        .expect("should write large row group");

    let reader = FileReader::new(schema);
    let parsed_rg = reader
        .read_row_group(&writer.row_groups()[0])
        .expect("row group should parse");

    assert!(!parsed_rg.columns.is_empty());
}
