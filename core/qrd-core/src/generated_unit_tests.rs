use crate::columnar::transpose_rows;
use crate::file::{build_file_image, build_file_image_with_signature, parse_file_image};
use crate::row_group::RowGroup;
use crate::schema::{FieldKind, Schema, SchemaBuilder};
use crate::signing::{SchemaSignature, SigningKeyPair, SIGNATURE_ALGORITHM};

fn make_schema(seed: u8, field_count: usize) -> Schema {
    let mut builder = SchemaBuilder::new();

    for index in 0..field_count {
        let kind = match (seed as usize + index) % 6 {
            0 => FieldKind::Boolean,
            1 => FieldKind::Int32,
            2 => FieldKind::Int64,
            3 => FieldKind::Float32,
            4 => FieldKind::Float64,
            _ => FieldKind::Utf8,
        };
        let required = (seed as usize + index) % 2 == 0;
        builder = builder.add_field(format!("field_{seed}_{index}"), kind, required);
    }

    builder.build().expect("schema should build")
}

fn make_rows(seed: u8, row_count: usize, width: usize) -> Vec<Vec<u8>> {
    let mut rows = Vec::with_capacity(row_count);

    for row_index in 0..row_count {
        let mut row = Vec::with_capacity(width);
        for column_index in 0..width {
            let value = seed
                .wrapping_add((row_index as u8).wrapping_mul(31))
                .wrapping_add((column_index as u8).wrapping_mul(17));
            row.push(value);
        }
        rows.push(row);
    }

    rows
}

fn run_schema_roundtrip(seed: u8) {
    let field_count = 1 + (seed as usize % 5);
    let schema = make_schema(seed, field_count);
    let bytes = schema.serialize().expect("schema should serialize");
    let parsed = Schema::deserialize(&bytes).expect("schema should deserialize");

    assert_eq!(parsed, schema);
    assert_eq!(parsed.fingerprint(), schema.fingerprint());
}

fn run_transpose_roundtrip(seed: u8) {
    let row_count = 1 + (seed as usize % 5);
    let width = 1 + ((seed as usize / 5) % 5);
    let rows = make_rows(seed, row_count, width);

    let columns = transpose_rows(&rows).expect("rows should transpose");
    assert_eq!(columns.len(), width);

    for column_index in 0..width {
        for row_index in 0..row_count {
            assert_eq!(
                columns[column_index][row_index],
                rows[row_index][column_index]
            );
        }
    }

    let reconstructed = transpose_rows(&columns).expect("columns should transpose back");
    assert_eq!(reconstructed, rows);
}

fn run_row_group_roundtrip(seed: u8) {
    let row_count = 1 + (seed as usize % 5);
    let width = 1 + ((seed as usize / 5) % 5);
    let rows = make_rows(seed, row_count, width);

    let row_group = RowGroup::from_rows(&rows).expect("row group should build");
    let bytes = row_group.serialize().expect("row group should serialize");
    let parsed = RowGroup::deserialize(&bytes).expect("row group should deserialize");

    assert_eq!(parsed, row_group);
    assert_eq!(parsed.row_count as usize, row_count);
    assert_eq!(parsed.columns.len(), width);
}

fn run_file_image_roundtrip(seed: u8) {
    let width = 1 + (seed as usize % 5);
    let row_count = 1 + ((seed as usize / 5) % 5);
    let schema = make_schema(seed, width);
    let rows = make_rows(seed.wrapping_add(11), row_count, width);
    let primary_row_group = RowGroup::from_rows(&rows).expect("row group should build");

    let mut row_groups = vec![primary_row_group.clone()];
    if seed % 2 == 0 {
        let secondary_rows = make_rows(seed.wrapping_add(19), 1 + (seed as usize % 3), width);
        row_groups.push(RowGroup::from_rows(&secondary_rows).expect("row group should build"));
    }

    if seed % 5 == 0 {
        let keypair = SigningKeyPair::generate();
        let schema_id = schema.fingerprint();
        let signature_bytes = keypair.sign_schema(&schema_id);
        let signature = SchemaSignature::new(
            SIGNATURE_ALGORITHM,
            signature_bytes,
            keypair.verifying_key(),
        );

        let bytes = build_file_image_with_signature(&schema, &row_groups, Some(signature.clone()))
            .expect("file image should build");
        let parsed = parse_file_image(&bytes).expect("file image should parse");

        assert!(parsed.header.is_schema_signed());
        assert_eq!(parsed.signature, Some(signature));
        assert_eq!(parsed.footer.schema, schema);
        assert_eq!(parsed.row_groups, row_groups);
        assert_eq!(parsed.footer.row_group_count, row_groups.len() as u32);
        return;
    }

    let bytes = build_file_image(&schema, &row_groups).expect("file image should build");
    let parsed = parse_file_image(&bytes).expect("file image should parse");

    assert!(!parsed.header.is_schema_signed());
    assert!(parsed.signature.is_none());
    assert_eq!(parsed.footer.schema, schema);
    assert_eq!(parsed.row_groups, row_groups);
    assert_eq!(parsed.footer.row_group_count, row_groups.len() as u32);
}

macro_rules! generate_tests {
    ($helper:ident, $( $name:ident => $seed:expr ),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                $helper($seed);
            }
        )+
    };
}

generate_tests!(
    run_schema_roundtrip,
    schema_roundtrip_case_00 => 0,
    schema_roundtrip_case_01 => 1,
    schema_roundtrip_case_02 => 2,
    schema_roundtrip_case_03 => 3,
    schema_roundtrip_case_04 => 4,
    schema_roundtrip_case_05 => 5,
    schema_roundtrip_case_06 => 6,
    schema_roundtrip_case_07 => 7,
    schema_roundtrip_case_08 => 8,
    schema_roundtrip_case_09 => 9,
    schema_roundtrip_case_10 => 10,
    schema_roundtrip_case_11 => 11,
    schema_roundtrip_case_12 => 12,
    schema_roundtrip_case_13 => 13,
    schema_roundtrip_case_14 => 14,
    schema_roundtrip_case_15 => 15,
    schema_roundtrip_case_16 => 16,
    schema_roundtrip_case_17 => 17,
    schema_roundtrip_case_18 => 18,
    schema_roundtrip_case_19 => 19,
    schema_roundtrip_case_20 => 20,
    schema_roundtrip_case_21 => 21,
    schema_roundtrip_case_22 => 22,
    schema_roundtrip_case_23 => 23,
    schema_roundtrip_case_24 => 24
);

generate_tests!(
    run_transpose_roundtrip,
    transpose_roundtrip_case_25 => 25,
    transpose_roundtrip_case_26 => 26,
    transpose_roundtrip_case_27 => 27,
    transpose_roundtrip_case_28 => 28,
    transpose_roundtrip_case_29 => 29,
    transpose_roundtrip_case_30 => 30,
    transpose_roundtrip_case_31 => 31,
    transpose_roundtrip_case_32 => 32,
    transpose_roundtrip_case_33 => 33,
    transpose_roundtrip_case_34 => 34,
    transpose_roundtrip_case_35 => 35,
    transpose_roundtrip_case_36 => 36,
    transpose_roundtrip_case_37 => 37,
    transpose_roundtrip_case_38 => 38,
    transpose_roundtrip_case_39 => 39,
    transpose_roundtrip_case_40 => 40,
    transpose_roundtrip_case_41 => 41,
    transpose_roundtrip_case_42 => 42,
    transpose_roundtrip_case_43 => 43,
    transpose_roundtrip_case_44 => 44,
    transpose_roundtrip_case_45 => 45,
    transpose_roundtrip_case_46 => 46,
    transpose_roundtrip_case_47 => 47,
    transpose_roundtrip_case_48 => 48,
    transpose_roundtrip_case_49 => 49
);

generate_tests!(
    run_row_group_roundtrip,
    row_group_roundtrip_case_50 => 50,
    row_group_roundtrip_case_51 => 51,
    row_group_roundtrip_case_52 => 52,
    row_group_roundtrip_case_53 => 53,
    row_group_roundtrip_case_54 => 54,
    row_group_roundtrip_case_55 => 55,
    row_group_roundtrip_case_56 => 56,
    row_group_roundtrip_case_57 => 57,
    row_group_roundtrip_case_58 => 58,
    row_group_roundtrip_case_59 => 59,
    row_group_roundtrip_case_60 => 60,
    row_group_roundtrip_case_61 => 61,
    row_group_roundtrip_case_62 => 62,
    row_group_roundtrip_case_63 => 63,
    row_group_roundtrip_case_64 => 64,
    row_group_roundtrip_case_65 => 65,
    row_group_roundtrip_case_66 => 66,
    row_group_roundtrip_case_67 => 67,
    row_group_roundtrip_case_68 => 68,
    row_group_roundtrip_case_69 => 69,
    row_group_roundtrip_case_70 => 70,
    row_group_roundtrip_case_71 => 71,
    row_group_roundtrip_case_72 => 72,
    row_group_roundtrip_case_73 => 73,
    row_group_roundtrip_case_74 => 74
);

generate_tests!(
    run_file_image_roundtrip,
    file_image_roundtrip_case_75 => 75,
    file_image_roundtrip_case_76 => 76,
    file_image_roundtrip_case_77 => 77,
    file_image_roundtrip_case_78 => 78,
    file_image_roundtrip_case_79 => 79,
    file_image_roundtrip_case_80 => 80,
    file_image_roundtrip_case_81 => 81,
    file_image_roundtrip_case_82 => 82,
    file_image_roundtrip_case_83 => 83,
    file_image_roundtrip_case_84 => 84,
    file_image_roundtrip_case_85 => 85,
    file_image_roundtrip_case_86 => 86,
    file_image_roundtrip_case_87 => 87,
    file_image_roundtrip_case_88 => 88,
    file_image_roundtrip_case_89 => 89,
    file_image_roundtrip_case_90 => 90,
    file_image_roundtrip_case_91 => 91,
    file_image_roundtrip_case_92 => 92,
    file_image_roundtrip_case_93 => 93,
    file_image_roundtrip_case_94 => 94,
    file_image_roundtrip_case_95 => 95,
    file_image_roundtrip_case_96 => 96,
    file_image_roundtrip_case_97 => 97,
    file_image_roundtrip_case_98 => 98,
    file_image_roundtrip_case_99 => 99
);
