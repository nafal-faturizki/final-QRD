use crate::error::{QrdError, Result};
use crate::integrity::crc32;
use crate::schema::Schema;
use std::convert::TryFrom;

/// QRD file header size in bytes.
pub const HEADER_SIZE: usize = 32;

/// QRD magic bytes.
pub const MAGIC_BYTES: [u8; 4] = [0x51, 0x52, 0x44, 0x00];

/// File header as defined by the Phase 1 binary contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHeader {
    pub format_major: u16,
    pub format_minor: u16,
    pub schema_id: [u8; 8],
    pub flags: u16,
    pub writer_version: [u8; 12],
}

impl FileHeader {
    /// Flag: Set if columns are encrypted
    pub const FLAG_ENCRYPTED: u16 = 0x0001;
    /// Flag: Set if schema is signed with Ed25519
    pub const FLAG_SCHEMA_SIGNED: u16 = 0x0002;

    /// Checks if the ENCRYPTED flag is set
    pub fn is_encrypted(&self) -> bool {
        (self.flags & Self::FLAG_ENCRYPTED) != 0
    }

    /// Checks if the SCHEMA_SIGNED flag is set
    pub fn is_schema_signed(&self) -> bool {
        (self.flags & Self::FLAG_SCHEMA_SIGNED) != 0
    }

    /// Sets the ENCRYPTED flag
    pub fn set_encrypted(&mut self, encrypted: bool) {
        if encrypted {
            self.flags |= Self::FLAG_ENCRYPTED;
        } else {
            self.flags &= !Self::FLAG_ENCRYPTED;
        }
    }

    /// Sets the SCHEMA_SIGNED flag
    pub fn set_schema_signed(&mut self, signed: bool) {
        if signed {
            self.flags |= Self::FLAG_SCHEMA_SIGNED;
        } else {
            self.flags &= !Self::FLAG_SCHEMA_SIGNED;
        }
    }

    /// Serializes the header into its canonical 32-byte layout.
    pub fn serialize(&self) -> [u8; HEADER_SIZE] {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[0..4].copy_from_slice(&MAGIC_BYTES);
        bytes[4..6].copy_from_slice(&self.format_major.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.format_minor.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.schema_id);
        bytes[16..18].copy_from_slice(&self.flags.to_le_bytes());
        bytes[18..20].copy_from_slice(&0u16.to_le_bytes());
        bytes[20..32].copy_from_slice(&self.writer_version);
        bytes
    }

    /// Creates a header with the required magic and reserved bytes.
    pub fn new(
        format_major: u16,
        format_minor: u16,
        schema_id: [u8; 8],
        flags: u16,
        writer_version: [u8; 12],
    ) -> Self {
        Self {
            format_major,
            format_minor,
            schema_id,
            flags,
            writer_version,
        }
    }
}

/// Parses a canonical QRD header.
pub fn parse_header(bytes: &[u8]) -> Result<FileHeader> {
    if bytes.len() != HEADER_SIZE {
        return Err(QrdError::InvalidHeaderLength);
    }
    if bytes[0..4] != MAGIC_BYTES {
        return Err(QrdError::InvalidMagic);
    }

    let mut reserved = [0u8; 2];
    reserved.copy_from_slice(&bytes[18..20]);
    if u16::from_le_bytes(reserved) != 0 {
        return Err(QrdError::InvalidReservedField);
    }

    let mut schema_id = [0u8; 8];
    schema_id.copy_from_slice(&bytes[8..16]);

    let mut writer_version = [0u8; 12];
    writer_version.copy_from_slice(&bytes[20..32]);

    Ok(FileHeader {
        format_major: u16::from_le_bytes([bytes[4], bytes[5]]),
        format_minor: u16::from_le_bytes([bytes[6], bytes[7]]),
        schema_id,
        flags: u16::from_le_bytes([bytes[16], bytes[17]]),
        writer_version,
    })
}

/// Parses the trailing footer-length field from a QRD file image.
pub fn parse_footer_length(bytes: &[u8]) -> Result<u32> {
    if bytes.len() < 4 {
        return Err(QrdError::InvalidFooterLength);
    }

    let tail = &bytes[bytes.len() - 4..];
    Ok(u32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]))
}

/// Appends a footer-length field in canonical little-endian form.
pub fn append_footer_length(bytes: &mut Vec<u8>, footer_length: u32) {
    bytes.extend_from_slice(&footer_length.to_le_bytes());
}

/// File footer as written by the scaffold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFooter {
    pub schema: Schema,
    pub row_group_count: u32,
}

/// Serializes the footer into a canonical binary representation.
pub fn build_footer(schema: &Schema, row_group_count: u32) -> Result<Vec<u8>> {
    let schema_bytes = schema.serialize()?;
    let schema_len = u32::try_from(schema_bytes.len())
        .map_err(|_| QrdError::InvalidSchema("schema payload too large".into()))?;

    let mut body = Vec::new();
    body.push(1u8);
    body.extend_from_slice(&schema_len.to_le_bytes());
    body.extend_from_slice(&schema_bytes);
    body.extend_from_slice(&row_group_count.to_le_bytes());

    let checksum = crc32(&body);
    body.extend_from_slice(&checksum.to_le_bytes());
    Ok(body)
}

/// Parses a canonical footer and verifies its checksum.
pub fn parse_footer(bytes: &[u8]) -> Result<FileFooter> {
    if bytes.len() < 1 + 4 + 4 + 4 {
        return Err(QrdError::InvalidFooterLength);
    }

    let checksum_offset = bytes.len() - 4;
    let expected_checksum = u32::from_le_bytes([
        bytes[checksum_offset],
        bytes[checksum_offset + 1],
        bytes[checksum_offset + 2],
        bytes[checksum_offset + 3],
    ]);
    let actual_checksum = crc32(&bytes[..checksum_offset]);
    if actual_checksum != expected_checksum {
        return Err(QrdError::InvalidFooterLength);
    }

    let mut cursor = 0usize;
    let version = *bytes.get(cursor).ok_or(QrdError::UnexpectedEof)?;
    cursor += 1;
    if version != 1 {
        return Err(QrdError::InvalidFooterLength);
    }

    let schema_len_bytes = bytes
        .get(cursor..cursor + 4)
        .ok_or(QrdError::UnexpectedEof)?;
    let schema_len = u32::from_le_bytes([
        schema_len_bytes[0],
        schema_len_bytes[1],
        schema_len_bytes[2],
        schema_len_bytes[3],
    ]) as usize;
    cursor += 4;

    let schema_end = cursor
        .checked_add(schema_len)
        .ok_or_else(|| QrdError::InvalidSchema("footer length overflow".into()))?;
    let schema_bytes = bytes
        .get(cursor..schema_end)
        .ok_or(QrdError::UnexpectedEof)?;
    let schema = Schema::deserialize(schema_bytes)?;
    cursor = schema_end;

    let row_group_count_bytes = bytes
        .get(cursor..cursor + 4)
        .ok_or(QrdError::UnexpectedEof)?;
    let row_group_count = u32::from_le_bytes([
        row_group_count_bytes[0],
        row_group_count_bytes[1],
        row_group_count_bytes[2],
        row_group_count_bytes[3],
    ]);
    cursor += 4;

    if cursor != checksum_offset {
        return Err(QrdError::InvalidFooterLength);
    }

    Ok(FileFooter {
        schema,
        row_group_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{FieldKind, SchemaBuilder};

    #[test]
    fn header_roundtrip_preserves_bytes() {
        let header = FileHeader::new(1, 0, [1, 2, 3, 4, 5, 6, 7, 8], 0b1010, *b"qrd-0.1.0\0\0\0");

        let serialized = header.serialize();
        let parsed = parse_header(&serialized).expect("valid header");

        assert_eq!(parsed, header);
    }

    #[test]
    fn header_rejects_wrong_magic() {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[0..4].copy_from_slice(b"BAD\0");

        let error = parse_header(&bytes).expect_err("invalid magic must fail");
        assert!(matches!(error, QrdError::InvalidMagic));
    }

    #[test]
    fn header_rejects_nonzero_reserved_field() {
        let header = FileHeader::new(1, 0, [0; 8], 0, *b"qrd-0.1.0\0\0\0");
        let mut bytes = header.serialize();
        bytes[18] = 1;

        let error = parse_header(&bytes).expect_err("reserved field must be zero");
        assert!(matches!(error, QrdError::InvalidReservedField));
    }

    #[test]
    fn footer_length_is_read_from_tail() {
        let mut bytes = vec![1u8, 2, 3, 4, 5];
        append_footer_length(&mut bytes, 0x1122_3344);

        let footer_length = parse_footer_length(&bytes).expect("footer length must parse");
        assert_eq!(footer_length, 0x1122_3344);
    }

    #[test]
    fn footer_length_rejects_truncation() {
        let error = parse_footer_length(&[1, 2, 3]).expect_err("truncated footer length must fail");
        assert!(matches!(error, QrdError::InvalidFooterLength));
    }

    #[test]
    fn footer_roundtrip_is_verified() {
        let schema = SchemaBuilder::new()
            .add_field("device_id", FieldKind::Utf8, true)
            .build()
            .expect("schema should build");

        let footer = build_footer(&schema, 7).expect("footer should build");
        let parsed = parse_footer(&footer).expect("footer should parse");

        assert_eq!(parsed.schema, schema);
        assert_eq!(parsed.row_group_count, 7);
    }
}
