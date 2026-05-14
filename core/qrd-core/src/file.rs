use crate::error::{QrdError, Result};
use crate::parser::{
    append_footer_length, build_footer, parse_footer, parse_header, FileFooter, FileHeader,
    HEADER_SIZE,
};
use crate::row_group::RowGroup;
use crate::schema::Schema;
use crate::signing::SchemaSignature;
use std::convert::TryFrom;

/// A fully parsed QRD file image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFile {
    pub header: FileHeader,
    pub row_groups: Vec<RowGroup>,
    pub footer: FileFooter,
    pub signature: Option<SchemaSignature>,
}

/// Builds a full QRD file image from schema and row groups with optional signature.
pub fn build_file_image(schema: &Schema, row_groups: &[RowGroup]) -> Result<Vec<u8>> {
    build_file_image_with_signature(schema, row_groups, None)
}

/// Builds a full QRD file image with optional schema signature.
pub fn build_file_image_with_signature(
    schema: &Schema,
    row_groups: &[RowGroup],
    signature: Option<SchemaSignature>,
) -> Result<Vec<u8>> {
    let serialized_row_groups: Vec<Vec<u8>> = row_groups
        .iter()
        .map(RowGroup::serialize)
        .collect::<Result<Vec<Vec<u8>>>>()?;
    build_file_image_from_serialized_row_groups(schema, &serialized_row_groups, signature)
}

pub(crate) fn build_file_image_from_serialized_row_groups(
    schema: &Schema,
    row_groups: &[Vec<u8>],
    signature: Option<SchemaSignature>,
) -> Result<Vec<u8>> {
    let mut header = FileHeader::new(1, 0, schema.fingerprint(), 0, *b"qrd-0.1.0\0\0\0");
    if signature.is_some() {
        header.set_schema_signed(true);
    }

    let footer = build_footer(
        schema,
        u32::try_from(row_groups.len())
            .map_err(|_| QrdError::InvalidSchema("too many row groups".into()))?,
    )?;
    let footer_length = u32::try_from(footer.len())
        .map_err(|_| QrdError::InvalidSchema("footer too large".into()))?;

    let signature_bytes = match signature {
        Some(sig) => Some(sig.serialize()),
        None => None,
    };
    let signature_length = signature_bytes.as_ref().map(Vec::len).unwrap_or(0);
    let row_group_bytes_len: usize = row_groups.iter().map(|bytes| 4 + bytes.len()).sum();

    let mut bytes = Vec::with_capacity(HEADER_SIZE + row_group_bytes_len + footer.len() + signature_length + 4);
    bytes.extend_from_slice(&header.serialize());

    for serialized in row_groups {
        let row_group_len = u32::try_from(serialized.len())
            .map_err(|_| QrdError::InvalidSchema("row group too large".into()))?;
        bytes.extend_from_slice(&row_group_len.to_le_bytes());
        bytes.extend_from_slice(serialized);
    }

    bytes.reserve(footer.len() + signature_length + 4);
    bytes.extend_from_slice(&footer);

    // Add signature if present (before footer length)
    if let Some(signature_bytes) = signature_bytes {
        bytes.extend_from_slice(&signature_bytes);
    }

    append_footer_length(&mut bytes, footer_length);
    Ok(bytes)
}

/// Parses a full QRD file image.
pub fn parse_file_image(bytes: &[u8]) -> Result<ParsedFile> {
    if bytes.len() < HEADER_SIZE + 4 {
        return Err(QrdError::InvalidHeaderLength);
    }

    let header = parse_header(&bytes[0..HEADER_SIZE])?;
    let footer_length = crate::parser::parse_footer_length(bytes)? as usize;
    if bytes.len() < HEADER_SIZE + footer_length + 4 {
        return Err(QrdError::InvalidFooterLength);
    }

    // Determine if there's a signature before the footer length
    let signature_size = if header.is_schema_signed() { 97 } else { 0 }; // 1 + 64 + 32
    let footer_start = bytes
        .len()
        .checked_sub(4)
        .and_then(|size| size.checked_sub(footer_length))
        .and_then(|size| size.checked_sub(signature_size))
        .ok_or(QrdError::InvalidFooterLength)?;

    if footer_start < HEADER_SIZE {
        return Err(QrdError::InvalidFooterLength);
    }

    let footer = parse_footer(&bytes[footer_start..footer_start + footer_length])?;

    // Parse signature if present
    let signature = if header.is_schema_signed() {
        let sig_start = footer_start + footer_length;
        let sig_bytes = &bytes[sig_start..sig_start + signature_size];
        Some(SchemaSignature::deserialize(sig_bytes)?)
    } else {
        None
    };

    // Verify signature if present
    if let Some(ref sig) = signature {
        sig.verify(&header.schema_id)?;
    }

    let mut row_groups = Vec::with_capacity(footer.row_group_count as usize);
    let mut cursor = HEADER_SIZE;
    while cursor < footer_start {
        let len_bytes = bytes
            .get(cursor..cursor + 4)
            .ok_or(QrdError::UnexpectedEof)?;
        let row_group_len =
            u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;
        cursor += 4;

        let end = cursor
            .checked_add(row_group_len)
            .ok_or_else(|| QrdError::InvalidSchema("row group length overflow".into()))?;
        let row_group_bytes = bytes.get(cursor..end).ok_or(QrdError::UnexpectedEof)?;
        row_groups.push(RowGroup::deserialize(row_group_bytes)?);
        cursor = end;
    }

    Ok(ParsedFile {
        header,
        row_groups,
        footer,
        signature,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::row_group::RowGroup;
    use crate::schema::{FieldKind, SchemaBuilder};

    #[test]
    fn file_image_roundtrip_is_end_to_end() {
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

        assert_eq!(parsed.header.schema_id, schema.fingerprint());
        assert_eq!(parsed.row_groups, row_groups);
        assert_eq!(parsed.footer.schema, schema);
        assert_eq!(parsed.footer.row_group_count, row_groups.len() as u32);
        assert!(parsed.signature.is_none());
    }

    #[test]
    fn file_image_with_signature_roundtrip() {
        use crate::signing::{SigningKeyPair, SchemaSignature, SIGNATURE_ALGORITHM};

        let schema = SchemaBuilder::new()
            .add_field("device_id", FieldKind::Utf8, true)
            .build()
            .expect("schema should build");

        let keypair = SigningKeyPair::generate();
        let schema_id = schema.fingerprint();
        let signature_bytes = keypair.sign_schema(&schema_id);
        let pubkey = keypair.verifying_key();

        let sig = SchemaSignature::new(SIGNATURE_ALGORITHM, signature_bytes, pubkey);
        let row_groups = vec![RowGroup::from_rows(&[vec![1, 2]]).expect("row group should build")];

        let bytes = build_file_image_with_signature(&schema, &row_groups, Some(sig.clone()))
            .expect("file image should build");
        let parsed = parse_file_image(&bytes).expect("file image should parse");

        assert!(parsed.header.is_schema_signed());
        assert_eq!(parsed.signature.unwrap(), sig);
    }

    #[test]
    fn file_image_handles_empty_row_groups() {
        let schema = SchemaBuilder::new()
            .add_field("device_id", FieldKind::Utf8, true)
            .build()
            .expect("schema should build");

        let bytes = build_file_image(&schema, &[]).expect("file image should build");
        let parsed = parse_file_image(&bytes).expect("file image should parse");

        assert!(parsed.row_groups.is_empty());
        assert_eq!(parsed.footer.row_group_count, 0);
        assert!(parsed.signature.is_none());
    }

    #[test]
    fn file_image_rejects_truncated_footer_length() {
        let schema = SchemaBuilder::new()
            .add_field("device_id", FieldKind::Utf8, true)
            .build()
            .expect("schema should build");

        let mut bytes = build_file_image(&schema, &[]).expect("file image should build");
        bytes.pop();

        assert!(parse_file_image(&bytes).is_err());
    }

    #[test]
    fn file_image_rejects_invalid_signature() {
        use crate::signing::{SigningKeyPair, SchemaSignature, SIGNATURE_ALGORITHM};

        let schema = SchemaBuilder::new()
            .add_field("device_id", FieldKind::Utf8, true)
            .build()
            .expect("schema should build");

        let keypair = SigningKeyPair::generate();
        let schema_id = schema.fingerprint();
        let mut signature_bytes = keypair.sign_schema(&schema_id);
        // Tamper with signature
        signature_bytes[0] ^= 0xFF;
        let pubkey = keypair.verifying_key();

        let sig = SchemaSignature::new(SIGNATURE_ALGORITHM, signature_bytes, pubkey);
        let row_groups = vec![RowGroup::from_rows(&[vec![1, 2]]).expect("row group should build")];

        let bytes = build_file_image_with_signature(&schema, &row_groups, Some(sig))
            .expect("file image should build");

        // Should fail because signature is invalid
        assert!(parse_file_image(&bytes).is_err());
    }
}

