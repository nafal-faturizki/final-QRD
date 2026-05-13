use crate::error::Result;
use crate::parser::{parse_footer, parse_footer_length, parse_header, FileFooter, FileHeader};
use crate::row_group::RowGroup;
use crate::schema::Schema;

/// Minimal file reader scaffold.
pub struct FileReader {
    schema: Schema,
    header: FileHeader,
}

impl FileReader {
    /// Opens a reader from a schema stub.
    pub fn new(schema: Schema) -> Self {
        let header = FileHeader::new(1, 0, schema.fingerprint(), 0, *b"qrd-0.1.0\0\0\0");
        Self { schema, header }
    }

    /// Returns the file schema.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Returns the parsed file header.
    pub fn header(&self) -> &FileHeader {
        &self.header
    }

    /// Inspects the header from a raw file image.
    pub fn inspect_header(bytes: &[u8]) -> Result<FileHeader> {
        parse_header(bytes)
    }

    /// Inspects the footer-length trailer from a raw file image.
    pub fn inspect_footer_length(bytes: &[u8]) -> Result<u32> {
        parse_footer_length(bytes)
    }

    /// Inspects a canonical footer from a raw file image.
    pub fn inspect_footer(bytes: &[u8]) -> Result<FileFooter> {
        parse_footer(bytes)
    }

    /// Reads a subset of columns.
    pub fn read_columns(&self, _columns: &[&str]) -> Result<Vec<Vec<u8>>> {
        Ok(Vec::new())
    }

    /// Parses a row group from a canonical binary representation.
    pub fn read_row_group(&self, bytes: &[u8]) -> Result<RowGroup> {
        RowGroup::deserialize(bytes)
    }
}

