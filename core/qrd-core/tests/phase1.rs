use qrd_core::memory::{estimate_reader_peak_memory, estimate_writer_peak_memory};
use qrd_core::file::{build_file_image, parse_file_image};
use qrd_core::parser::{append_footer_length, build_footer, parse_footer, parse_footer_length, parse_header, FileHeader};
use qrd_core::row_group::RowGroup;
use qrd_core::schema::{FieldKind, SchemaBuilder};

#[test]
fn file_header_roundtrip_is_stable() {
    let header = FileHeader::new(1, 0, [9, 8, 7, 6, 5, 4, 3, 2], 0b11, *b"qrd-0.1.0\0\0\0");
    let bytes = header.serialize();
    let parsed = parse_header(&bytes).expect("header should parse");

    assert_eq!(parsed, header);
}

#[test]
fn footer_roundtrip_is_stable() {
    let schema = SchemaBuilder::new()
        .add_field("device_id", FieldKind::Utf8, true)
        .add_field("temperature", FieldKind::Float32, false)
        .build()
        .expect("schema should build");

    let footer = build_footer(&schema, 3).expect("footer should build");
    let parsed = parse_footer(&footer).expect("footer should parse");

    assert_eq!(parsed.schema, schema);
    assert_eq!(parsed.row_group_count, 3);
}

#[test]
fn row_group_roundtrip_is_stable() {
    let rows = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
    let row_group = RowGroup::from_rows(&rows).expect("row group should build");
    let bytes = row_group.serialize().expect("row group should serialize");
    let parsed = RowGroup::deserialize(&bytes).expect("row group should parse");

    assert_eq!(parsed, row_group);
}

#[test]
fn footer_length_trailer_is_read_from_tail() {
    let mut bytes = vec![1u8, 2, 3, 4];
    append_footer_length(&mut bytes, 0xAABB_CCDD);

    let length = parse_footer_length(&bytes).expect("footer length should parse");
    assert_eq!(length, 0xAABB_CCDD);
}

#[test]
fn bounded_memory_helpers_return_expected_values() {
    let writer_peak = estimate_writer_peak_memory(100, 32, 16, 8).expect("estimate should work");
    let reader_peak = estimate_reader_peak_memory(&[10, 20, 30], 3, 64).expect("estimate should work");

    assert_eq!(writer_peak, 100 * 32 + 16 + 8);
    assert_eq!(reader_peak, (10 + 20 + 30) * 3 + 64);
}

#[test]
fn full_file_image_roundtrip_is_stable() {
    let schema = SchemaBuilder::new()
        .add_field("device_id", FieldKind::Utf8, true)
        .add_field("temperature", FieldKind::Float32, false)
        .build()
        .expect("schema should build");

    let row_groups = vec![
        RowGroup::from_rows(&[vec![1, 2], vec![3, 4]]).expect("row group should build"),
        RowGroup::from_rows(&[vec![5, 6]]).expect("row group should build"),
    ];

    let bytes = build_file_image(&schema, &row_groups).expect("file image should build");
    let parsed = parse_file_image(&bytes).expect("file image should parse");

    assert_eq!(parsed.footer.schema, schema);
    assert_eq!(parsed.footer.row_group_count, row_groups.len() as u32);
    assert_eq!(parsed.row_groups, row_groups);
}
