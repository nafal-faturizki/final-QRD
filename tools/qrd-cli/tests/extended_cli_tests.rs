// Extended tests for qrd-cli - comprehensive test suite
// Tests for file inspection, verification, key generation, and format conversion

#[cfg(test)]
mod extended_cli_tests {
    use qrd_cli::*;
    use qrd_core::file::build_file_image;
    use qrd_core::row_group::RowGroup;
    use qrd_core::schema::{FieldKind, SchemaBuilder};
    use std::fs;

    fn create_test_file(name: &str) -> std::path::PathBuf {
        let schema = SchemaBuilder::new()
            .add_field("id", FieldKind::Int32, false)
            .add_field("value", FieldKind::Float64, true)
            .build()
            .expect("schema should build");

        let row_groups =
            vec![RowGroup::from_rows(&[vec![1, 2], vec![3, 4]]).expect("row group should build")];

        let bytes = build_file_image(&schema, &row_groups).expect("file image should build");
        let path = std::env::temp_dir().join(format!("qrd-cli-test-{}.qrd", name));
        fs::write(&path, bytes).expect("should write test file");
        path
    }

    // ============= File Inspection Tests =============

    #[test]
    fn inspect_valid_file() {
        let path = create_test_file("inspect-valid");
        let result = inspect_file(&path);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("format_major"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn inspect_file_contains_all_fields() {
        let path = create_test_file("inspect-fields");
        let output = inspect_file(&path).expect("inspect should work");
        assert!(output.contains("format_major"));
        assert!(output.contains("format_minor"));
        assert!(output.contains("flags"));
        assert!(output.contains("footer_length"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn inspect_nonexistent_file_fails() {
        let path = std::env::temp_dir().join("qrd-cli-nonexistent-999.qrd");
        let result = inspect_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn inspect_empty_file_fails() {
        let path = std::env::temp_dir().join("qrd-cli-empty.qrd");
        fs::write(&path, b"").expect("should write empty file");
        let result = inspect_file(&path);
        assert!(result.is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn inspect_truncated_file_fails() {
        let path = std::env::temp_dir().join("qrd-cli-truncated.qrd");
        fs::write(&path, b"QRD").expect("should write truncated file");
        let result = inspect_file(&path);
        assert!(result.is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn inspect_file_format_version_correct() {
        let path = create_test_file("inspect-version");
        let output = inspect_file(&path).expect("inspect should work");
        assert!(output.contains("format_major=1"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn inspect_file_multiple_times() {
        let path = create_test_file("inspect-multiple");
        for _ in 0..5 {
            let result = inspect_file(&path);
            assert!(result.is_ok());
        }
        let _ = fs::remove_file(&path);
    }

    // ============= JSON Inspection Tests =============

    #[test]
    fn inspect_json_valid_file() {
        let path = create_test_file("inspect-json-valid");
        let result = inspect_json(&path);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("format_major"));
        assert!(json.contains("schema_id"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn inspect_json_is_valid_json() {
        let path = create_test_file("inspect-json-format");
        let json = inspect_json(&path).expect("inspect_json should work");
        assert!(json.starts_with("{"));
        assert!(json.ends_with("}"));
        assert!(json.contains(":"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn inspect_json_contains_schema_id() {
        let path = create_test_file("inspect-json-schema");
        let json = inspect_json(&path).expect("inspect_json should work");
        assert!(json.contains("schema_id"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn inspect_json_nonexistent_file_fails() {
        let path = std::env::temp_dir().join("qrd-cli-json-nonexistent.qrd");
        let result = inspect_json(&path);
        assert!(result.is_err());
    }

    #[test]
    fn inspect_json_empty_file_fails() {
        let path = std::env::temp_dir().join("qrd-cli-json-empty.qrd");
        fs::write(&path, b"").expect("should write empty file");
        let result = inspect_json(&path);
        assert!(result.is_err());
        let _ = fs::remove_file(&path);
    }

    // ============= Footer Inspection Tests =============

    // ============= File Verification Tests =============

    #[test]
    fn verify_valid_file() {
        let path = create_test_file("verify-valid");
        let result = verify_file(&path);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("header_ok=true"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn verify_file_contains_all_info() {
        let path = create_test_file("verify-info");
        let output = verify_file(&path).expect("verify should work");
        assert!(output.contains("header_ok=true"));
        assert!(output.contains("schema_id"));
        assert!(output.contains("row_group_count"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn verify_nonexistent_file_fails() {
        let path = std::env::temp_dir().join("qrd-cli-verify-nonexistent.qrd");
        let result = verify_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn verify_empty_file_fails() {
        let path = std::env::temp_dir().join("qrd-cli-verify-empty.qrd");
        fs::write(&path, b"").expect("should write empty file");
        let result = verify_file(&path);
        assert!(result.is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn verify_truncated_file_fails() {
        let path = std::env::temp_dir().join("qrd-cli-verify-truncated.qrd");
        fs::write(&path, b"QRD").expect("should write truncated file");
        let result = verify_file(&path);
        assert!(result.is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn verify_multiple_files() {
        let paths: Vec<_> = (0..3)
            .map(|i| create_test_file(&format!("verify-multi-{}", i)))
            .collect();

        for path in &paths {
            assert!(verify_file(path).is_ok());
        }

        for path in paths {
            let _ = fs::remove_file(path);
        }
    }

    // ============= Schema ID Hex Conversion Tests =============

    #[test]
    fn schema_id_hex_correct_format() {
        let schema_id = [0x01u8, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef];
        let hex = schema_id_hex(&schema_id);
        assert_eq!(hex, "0123456789abcdef");
    }

    #[test]
    fn schema_id_hex_zeros() {
        let schema_id = [0u8; 8];
        let hex = schema_id_hex(&schema_id);
        assert_eq!(hex, "0000000000000000");
    }

    #[test]
    fn schema_id_hex_all_ones() {
        let schema_id = [0xffu8; 8];
        let hex = schema_id_hex(&schema_id);
        assert_eq!(hex, "ffffffffffffffff");
    }

    #[test]
    fn schema_id_hex_mixed_values() {
        let schema_id = [0x12u8, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
        let hex = schema_id_hex(&schema_id);
        assert_eq!(hex.len(), 16);
    }

    // ============= Format Conversion Tests =============

    #[test]
    fn convert_csv_format() {
        let input = std::env::temp_dir().join("qrd-cli-convert-input.csv");
        let output = std::env::temp_dir().join("qrd-cli-convert-output.qrd");
        fs::write(&input, b"data").expect("should write input");

        let result = convert_file("csv", &input, &output);
        assert!(result.is_ok());
        assert!(output.exists());
        let _ = fs::remove_file(&input);
        let _ = fs::remove_file(&output);
    }

    #[test]
    fn convert_json_format() {
        let input = std::env::temp_dir().join("qrd-cli-convert-input-json.json");
        let output = std::env::temp_dir().join("qrd-cli-convert-output-json.qrd");
        fs::write(&input, b"data").expect("should write input");

        let result = convert_file("json", &input, &output);
        assert!(result.is_ok());
        assert!(output.exists());
        let _ = fs::remove_file(&input);
        let _ = fs::remove_file(&output);
    }

    #[test]
    fn convert_parquet_format() {
        let input = std::env::temp_dir().join("qrd-cli-convert-input-parquet.parquet");
        let output = std::env::temp_dir().join("qrd-cli-convert-output-parquet.qrd");
        fs::write(&input, b"data").expect("should write input");

        let result = convert_file("parquet", &input, &output);
        assert!(result.is_ok());
        assert!(output.exists());
        let _ = fs::remove_file(&input);
        let _ = fs::remove_file(&output);
    }

    #[test]
    fn convert_nonexistent_input_fails() {
        let input = std::env::temp_dir().join("qrd-cli-convert-nonexistent-input.csv");
        let output = std::env::temp_dir().join("qrd-cli-convert-nonexistent-output.qrd");
        let result = convert_file("csv", &input, &output);
        assert!(result.is_err());
    }

    #[test]
    fn convert_placeholder_output_correct_format() {
        let input = std::env::temp_dir().join("qrd-cli-convert-test-input.csv");
        let output = std::env::temp_dir().join("qrd-cli-convert-test-output.qrd");
        fs::write(&input, b"test").expect("should write input");

        convert_file("csv", &input, &output).expect("convert should work");
        let content = fs::read(&output).expect("should read output");
        assert!(content.starts_with(b"QRD\0"));
        let _ = fs::remove_file(&input);
        let _ = fs::remove_file(&output);
    }

    // ============= Keygen Tests =============

    #[test]
    fn keygen_master_key() {
        let result = generate_key("master");
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("MASTER_KEY_HEX="));
    }

    #[test]
    fn keygen_master_key_hex_format() {
        let output = generate_key("master").expect("keygen should work");
        let parts: Vec<&str> = output.split('=').collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[1].len() > 0);
    }

    #[test]
    fn keygen_master_key_length() {
        let output = generate_key("master").expect("keygen should work");
        // Extract hex string (should be 64 chars for 32 bytes)
        let hex_part = output.split('=').nth(1).unwrap();
        assert_eq!(hex_part.len(), 64);
    }

    #[test]
    fn keygen_master_key_unique() {
        let key1 = generate_key("master").expect("keygen should work");
        let key2 = generate_key("master").expect("keygen should work");
        // Keys should be different due to randomness
        assert_ne!(key1, key2);
    }

    #[test]
    fn keygen_signing_key() {
        let result = generate_key("signing");
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("ED25519_PRIVATE_KEY"));
        assert!(output.contains("ED25519_PUBLIC_KEY"));
    }

    #[test]
    fn keygen_signing_key_format() {
        let output = generate_key("signing").expect("keygen should work");
        assert!(output.contains("="));
    }

    #[test]
    fn keygen_invalid_mode_fails() {
        let result = generate_key("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn keygen_empty_mode_fails() {
        let result = generate_key("");
        assert!(result.is_err());
    }

    #[test]
    fn keygen_multiple_calls_unique() {
        let keys: Vec<_> = (0..5)
            .map(|_| generate_key("master").expect("keygen should work"))
            .collect();

        // All should be unique
        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                assert_ne!(keys[i], keys[j]);
            }
        }
    }

    // ============= File Reading Tests =============

    #[test]
    fn read_valid_file() {
        let path = create_test_file("read-valid");
        let result = read_file(&path);
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(!data.is_empty());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn read_nonexistent_file_fails() {
        let path = std::env::temp_dir().join("qrd-cli-read-nonexistent.qrd");
        let result = read_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn read_empty_file() {
        let path = std::env::temp_dir().join("qrd-cli-read-empty.qrd");
        fs::write(&path, b"").expect("should write empty file");
        let result = read_file(&path);
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(data.is_empty());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn read_file_magic_bytes() {
        let path = create_test_file("read-magic");
        let data = read_file(&path).expect("read should work");
        assert_eq!(data[0], 0x51); // Q
        assert_eq!(data[1], 0x52); // R
        assert_eq!(data[2], 0x44); // D
        assert_eq!(data[3], 0x00); // NULL
        let _ = fs::remove_file(&path);
    }

    // ============= Integration Tests =============

    #[test]
    fn inspect_and_verify_same_file() {
        let path = create_test_file("inspect-and-verify");
        let inspect_result = inspect_file(&path).expect("inspect should work");
        let verify_result = verify_file(&path).expect("verify should work");

        assert!(inspect_result.contains("format_major"));
        assert!(verify_result.contains("header_ok=true"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn inspect_json_and_verify_consistency() {
        let path = create_test_file("json-verify-consistency");
        let json = inspect_json(&path).expect("inspect_json should work");
        let verify = verify_file(&path).expect("verify should work");

        assert!(json.contains("schema_id"));
        assert!(verify.contains("schema_id"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn full_workflow_inspect_convert_keygen() {
        let path = create_test_file("workflow-test");
        let inspect = inspect_file(&path).expect("inspect should work");
        assert!(inspect.contains("format_major"));

        let keygen_result = generate_key("master").expect("keygen should work");
        assert!(keygen_result.contains("MASTER_KEY_HEX"));

        let input = std::env::temp_dir().join("qrd-cli-workflow-input.csv");
        let output = std::env::temp_dir().join("qrd-cli-workflow-output.qrd");
        fs::write(&input, b"test").expect("should write");
        convert_file("csv", &input, &output).expect("convert should work");
        assert!(output.exists());

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&input);
        let _ = fs::remove_file(&output);
    }
}
