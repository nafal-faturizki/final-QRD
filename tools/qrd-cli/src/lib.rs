use qrd_core::file::parse_file_image;
use qrd_core::parser::{parse_footer, parse_footer_length, parse_header};
use std::fs;
use std::path::Path;

pub fn read_file(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

pub fn schema_id_hex(schema_id: &[u8; 8]) -> String {
    schema_id.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn inspect_file(path: &Path) -> Result<String, String> {
    let bytes = read_file(path)?;
    if bytes.len() < qrd_core::parser::HEADER_SIZE {
        return Err("file too small for header inspection".into());
    }

    let header = parse_header(&bytes[0..qrd_core::parser::HEADER_SIZE])
        .map_err(|error| format!("header parse failed: {error}"))?;
    let mut output = String::new();
    output.push_str(&format!("format_major={}\n", header.format_major));
    output.push_str(&format!("format_minor={}\n", header.format_minor));
    output.push_str(&format!("flags={}\n", header.flags));

    if bytes.len() >= 4 {
        let footer_length = parse_footer_length(&bytes)
            .map_err(|error| format!("footer length parse failed: {error}"))?;
        output.push_str(&format!("footer_length={footer_length}\n"));
    }

    Ok(output)
}

pub fn inspect_json(path: &Path) -> Result<String, String> {
    let bytes = read_file(path)?;
    if bytes.len() < qrd_core::parser::HEADER_SIZE {
        return Err("file too small for header inspection".into());
    }

    let header = parse_header(&bytes[0..qrd_core::parser::HEADER_SIZE])
        .map_err(|error| format!("header parse failed: {error}"))?;

    Ok(format!(
        "{{\"format_major\":{},\"format_minor\":{},\"flags\":{},\"schema_id\":\"{}\"}}",
        header.format_major,
        header.format_minor,
        header.flags,
        schema_id_hex(&header.schema_id)
    ))
}

pub fn inspect_footer_only(path: &Path) -> Result<String, String> {
    let bytes = read_file(path)?;
    let footer = parse_footer(&bytes).map_err(|error| format!("footer parse failed: {error}"))?;
    Ok(format!(
        "row_group_count={}\nschema_fields={}\n",
        footer.row_group_count,
        footer.schema.fields().len()
    ))
}

pub fn verify_file(path: &Path) -> Result<String, String> {
    let bytes = read_file(path)?;
    if bytes.len() < qrd_core::parser::HEADER_SIZE + 4 {
        return Err("file too small to be a valid QRD scaffold".into());
    }

    let parsed = parse_file_image(&bytes).map_err(|error| format!("file parse failed: {error}"))?;
    Ok(format!(
        "header_ok=true\nschema_id={}\nrow_group_count={}\n",
        schema_id_hex(&parsed.header.schema_id),
        parsed.footer.row_group_count
    ))
}

pub fn convert_placeholder(mode: &str, input: &Path, output: &Path) -> Result<String, String> {
    let _ = read_file(input)?;
    fs::write(output, format!("qrd-convert-placeholder:{mode}"))
        .map_err(|error| format!("failed to write {}: {error}", output.display()))?;
    Ok(format!("converted {mode} -> {}", output.display()))
}

pub fn keygen_placeholder(mode: &str) -> Result<String, String> {
    Ok(match mode {
        "master" => "MASTER_KEY_HEX=placeholder".into(),
        "signing" => "ED25519_PRIVATE_KEY=placeholder\nED25519_PUBLIC_KEY=placeholder".into(),
        _ => return Err("unknown keygen mode".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qrd_core::file::build_file_image;
    use qrd_core::row_group::RowGroup;
    use qrd_core::schema::{FieldKind, SchemaBuilder};

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
    }

    #[test]
    fn keygen_and_convert_placeholders_return_output() {
        let temp_dir = std::env::temp_dir();
        let input = temp_dir.join("qrd-cli-input.txt");
        let output = temp_dir.join("qrd-cli-output.txt");
        fs::write(&input, b"input").expect("should write input file");

        let converted = convert_placeholder("csv", &input, &output).expect("convert should work");
        let keygen = keygen_placeholder("master").expect("keygen should work");

        assert!(converted.contains("converted csv"));
        assert!(keygen.contains("MASTER_KEY_HEX"));
    }
}