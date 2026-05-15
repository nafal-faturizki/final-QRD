use qrd_core::file::{build_file_image, parse_file_image, ParsedFile};
use qrd_core::parser::{parse_footer, parse_footer_length, HEADER_SIZE};
use qrd_core::row_group::RowGroup;
use qrd_core::schema::{FieldKind, SchemaBuilder};
use qrd_core::signing::SigningKeyPair;
use rand::rngs::OsRng;
use rand::RngCore;
use serde_json::json;
use std::fs;
use std::path::Path;

pub type Result<T> = std::result::Result<T, String>;

pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

pub fn schema_id_hex(schema_id: &[u8; 8]) -> String {
    schema_id.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn inspect_file(path: &Path) -> Result<String> {
    let bytes = read_file(path)?;
    if bytes.len() < HEADER_SIZE {
        return Err("file too small for header inspection".into());
    }

    let parsed = parse_file_image(&bytes).map_err(|error| format!("file parse failed: {error}"))?;
    let footer_length = parse_footer_length(&bytes)
        .map_err(|error| format!("footer length parse failed: {error}"))?;

    let mut output = String::new();
    output.push_str(&format!("format_major={}\n", parsed.header.format_major));
    output.push_str(&format!("format_minor={}\n", parsed.header.format_minor));
    output.push_str(&format!("flags={}\n", parsed.header.flags));
    output.push_str(&format!(
        "schema_id={}\n",
        schema_id_hex(&parsed.header.schema_id)
    ));
    output.push_str(&format!("footer_length={}\n", footer_length));
    output.push_str(&format!(
        "row_group_count={}\n",
        parsed.footer.row_group_count
    ));
    output.push_str(&format!(
        "schema_fields={}\n",
        parsed.footer.schema.fields().len()
    ));
    output.push_str(&format!(
        "signature_present={}\n",
        parsed.signature.is_some()
    ));
    Ok(output)
}

pub fn inspect_schema(path: &Path) -> Result<String> {
    let bytes = read_file(path)?;
    if bytes.len() < HEADER_SIZE {
        return Err("file too small for schema inspection".into());
    }

    let parsed = parse_file_image(&bytes).map_err(|error| format!("file parse failed: {error}"))?;
    let mut output = String::new();
    for field in parsed.footer.schema.fields() {
        output.push_str(&format!(
            "name={} kind={:?} required={}\n",
            field.name, field.kind, field.required
        ));
    }
    Ok(output)
}

pub fn inspect_json(path: &Path) -> Result<String> {
    let bytes = read_file(path)?;
    if bytes.len() < HEADER_SIZE {
        return Err("file too small for header inspection".into());
    }

    let parsed = parse_file_image(&bytes).map_err(|error| format!("file parse failed: {error}"))?;
    let schema_fields: Vec<_> = parsed
        .footer
        .schema
        .fields()
        .iter()
        .map(|field| {
            json!({
                "name": field.name,
                "kind": format!("{:?}", field.kind),
                "required": field.required
            })
        })
        .collect();

    let object = json!({
        "format_major": parsed.header.format_major,
        "format_minor": parsed.header.format_minor,
        "flags": parsed.header.flags,
        "schema_id": schema_id_hex(&parsed.header.schema_id),
        "footer_length": parse_footer_length(&bytes).unwrap_or(0),
        "row_group_count": parsed.footer.row_group_count,
        "schema_fields": schema_fields,
        "signature_present": parsed.signature.is_some()
    });

    serde_json::to_string(&object).map_err(|error| format!("json serialization failed: {error}"))
}

pub fn inspect_footer_only(path: &Path) -> Result<String> {
    let bytes = read_file(path)?;
    let footer = parse_footer(&bytes).map_err(|error| format!("footer parse failed: {error}"))?;
    Ok(format!(
        "row_group_count={}\nschema_fields={}\n",
        footer.row_group_count,
        footer.schema.fields().len()
    ))
}

pub fn verify_file(path: &Path) -> Result<String> {
    let bytes = read_file(path)?;
    if bytes.len() < HEADER_SIZE + 4 {
        return Err("file too small to be a valid QRD scaffold".into());
    }

    let parsed = parse_file_image(&bytes).map_err(|error| format!("file parse failed: {error}"))?;
    Ok(format!(
        "header_ok=true\nschema_id={}\nrow_group_count={}\nsignature_present={}\n",
        schema_id_hex(&parsed.header.schema_id),
        parsed.footer.row_group_count,
        parsed.signature.is_some()
    ))
}

pub fn convert_file(mode: &str, input: &Path, output: &Path) -> Result<String> {
    let input_bytes = read_file(input)?;
    let converted_bytes = match mode {
        "csv" | "json" | "parquet" | "csv-to-qrd" | "parquet-to-qrd" => {
            build_qrd_from_bytes(&input_bytes)?
        }
        "qrd-to-csv" | "qrd-to-parquet" | "qrd-to-json" => {
            let parsed = parse_file_image(&input_bytes)
                .map_err(|error| format!("file parse failed: {error}"))?;
            build_raw_file_from_qrd(&parsed)?
        }
        _ => return Err("unknown conversion mode".into()),
    };

    fs::write(output, converted_bytes)
        .map_err(|error| format!("failed to write {}: {error}", output.display()))?;
    Ok(format!("converted {mode} -> {}", output.display()))
}

pub fn generate_key(mode: &str) -> Result<String> {
    match mode {
        "master" => {
            let mut key = [0u8; 32];
            OsRng.fill_bytes(&mut key);
            Ok(format!("MASTER_KEY_HEX={}", bytes_to_hex(&key)))
        }
        "signing" => {
            let keypair = SigningKeyPair::generate();
            let private_key = keypair.seed();
            let public_key = keypair.verifying_key();
            Ok(format!(
                "ED25519_PRIVATE_KEY={}\nED25519_PUBLIC_KEY={}",
                bytes_to_hex(&private_key),
                bytes_to_hex(&public_key)
            ))
        }
        _ => Err("unknown keygen mode".into()),
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{:02x}", byte)).collect()
}

fn build_qrd_from_bytes(payload: &[u8]) -> Result<Vec<u8>> {
    let schema = SchemaBuilder::new()
        .add_field("qrd_source", FieldKind::Utf8, true)
        .build()
        .map_err(|error| format!("schema build failed: {error}"))?;

    let row_group = if payload.is_empty() {
        RowGroup::from_rows_with_names(&[], &["qrd_source"])
            .map_err(|error| format!("row group build failed: {error}"))?
    } else {
        RowGroup::from_rows(&[payload.to_vec()])
            .map_err(|error| format!("row group build failed: {error}"))?
    };

    build_file_image(&schema, &[row_group]).map_err(|error| format!("file build failed: {error}"))
}

fn build_raw_file_from_qrd(parsed: &ParsedFile) -> Result<Vec<u8>> {
    let output_rows = if let Some(row_group) = parsed.row_groups.first() {
        row_group_to_rows(row_group)?
    } else {
        Vec::new()
    };

    let mut output = Vec::new();
    for (index, row) in output_rows.into_iter().enumerate() {
        if index > 0 {
            output.push(b'\n');
        }
        output.extend_from_slice(&row);
    }
    Ok(output)
}

fn row_group_to_rows(row_group: &RowGroup) -> Result<Vec<Vec<u8>>> {
    let row_count = row_group.row_count as usize;
    let column_count = row_group.columns.len();
    if row_count == 0 {
        return Ok(Vec::new());
    }

    let mut decoded_columns = Vec::with_capacity(column_count);
    for column in &row_group.columns {
        decoded_columns.push(
            column
                .decode()
                .map_err(|error| format!("row group decode failed: {error}"))?,
        );
    }

    if decoded_columns.iter().any(|col| col.len() != row_count) {
        return Err("row group column length mismatch".into());
    }

    let mut rows = vec![vec![0u8; column_count]; row_count];
    for (column_index, column_data) in decoded_columns.iter().enumerate() {
        for (row_index, value) in column_data.iter().enumerate() {
            rows[row_index][column_index] = *value;
        }
    }

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qrd_core::file::build_file_image;
    use qrd_core::row_group::RowGroup;
    use qrd_core::schema::{FieldKind, SchemaBuilder};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(prefix: &str, suffix: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be monotonic")
            .as_nanos();
        std::env::temp_dir().join(format!("qrd-cli-{prefix}-{stamp}-{suffix}"))
    }

    #[test]
    fn inspect_and_verify_helpers_work_end_to_end() {
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
        let temp_path = std::env::temp_dir().join("qrd-cli-scaffold.qrd");
        fs::write(&temp_path, bytes).expect("should write temp file");

        let inspect = inspect_file(&temp_path).expect("inspect should work");
        let verify = verify_file(&temp_path).expect("verify should work");

        assert!(inspect.contains("format_major=1"));
        assert!(verify.contains("row_group_count=2"));
        let _ = fs::remove_file(&temp_path);
    }

    #[test]
    fn keygen_and_convert_functions_return_output() {
        let temp_dir = std::env::temp_dir();
        let input = temp_dir.join("qrd-cli-input.txt");
        let output = temp_dir.join("qrd-cli-output.qrd");
        fs::write(&input, b"input").expect("should write input file");

        let converted = convert_file("csv", &input, &output).expect("convert should work");
        let keygen = generate_key("master").expect("keygen should work");

        assert!(converted.contains("converted csv"));
        assert!(keygen.contains("MASTER_KEY_HEX"));
        let _ = fs::remove_file(&input);
        let _ = fs::remove_file(&output);
    }

    #[test]
    fn convert_qrd_to_csv_roundtrip_returns_original_content() {
        let input = unique_temp_path("convert-input", "csv");
        let qrd_output = unique_temp_path("convert-output", "qrd");
        let roundtrip = unique_temp_path("convert-roundtrip", "csv");
        let content = b"id,name\n42,example\n";

        fs::write(&input, content).expect("should write input");
        convert_file("csv", &input, &qrd_output).expect("convert should work");
        convert_file("qrd-to-csv", &qrd_output, &roundtrip).expect("reverse convert should work");

        assert_eq!(fs::read(&input).unwrap(), fs::read(&roundtrip).unwrap());
        let _ = fs::remove_file(&input);
        let _ = fs::remove_file(&qrd_output);
        let _ = fs::remove_file(&roundtrip);
    }

    #[test]
    fn convert_qrd_to_parquet_roundtrip_returns_raw_bytes() {
        let input = unique_temp_path("convert-input", "parquet");
        let qrd_output = unique_temp_path("convert-output", "qrd");
        let roundtrip = unique_temp_path("convert-roundtrip", "parquet");
        let content = b"PAR1\n";

        fs::write(&input, content).expect("should write input");
        convert_file("parquet", &input, &qrd_output).expect("convert should work");
        convert_file("qrd-to-parquet", &qrd_output, &roundtrip)
            .expect("reverse convert should work");

        assert_eq!(fs::read(&input).unwrap(), fs::read(&roundtrip).unwrap());
        let _ = fs::remove_file(&input);
        let _ = fs::remove_file(&qrd_output);
        let _ = fs::remove_file(&roundtrip);
    }
}
