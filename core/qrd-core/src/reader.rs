use crate::error::{QrdError, Result};
use crate::file::parse_file_image;
use crate::parser::{parse_footer, parse_footer_length, parse_header, FileFooter, FileHeader};
use crate::row_group::RowGroup;
use crate::schema::Schema;

/// Minimal file reader scaffold.
pub struct FileReader {
    schema: Schema,
    header: FileHeader,
    footer: FileFooter,
    row_groups: Vec<RowGroup>,
}

impl FileReader {
    /// Opens a reader from a complete QRD file image.
    pub fn open(bytes: &[u8]) -> Result<Self> {
        let parsed = parse_file_image(bytes)?;
        Ok(Self {
            schema: parsed.footer.schema.clone(),
            header: parsed.header,
            footer: parsed.footer,
            row_groups: parsed.row_groups,
        })
    }

    /// Creates a reader from a schema stub.
    pub fn new(schema: Schema) -> Self {
        let header = FileHeader::new(1, 0, schema.fingerprint(), 0, *b"qrd-0.1.0\0\0\0");
        let footer = FileFooter {
            schema: schema.clone(),
            row_group_count: 0,
        };
        Self {
            schema,
            header,
            footer,
            row_groups: Vec::new(),
        }
    }

    /// Returns the file schema.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Returns the parsed file header.
    pub fn header(&self) -> &FileHeader {
        &self.header
    }

    /// Returns the parsed file footer.
    pub fn footer(&self) -> &FileFooter {
        &self.footer
    }

    /// Returns the total row count across all parsed row groups.
    pub fn row_count(&self) -> usize {
        self.row_groups.iter().map(|rg| rg.row_count as usize).sum()
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

    /// Reads a subset of columns by name from the parsed file image.
    pub fn read_columns(&self, columns: &[&str]) -> Result<Vec<Vec<u8>>> {
        let mut outputs = vec![Vec::new(); columns.len()];
        for row_group in &self.row_groups {
            for (output, requested_name) in outputs.iter_mut().zip(columns.iter()) {
                let chunk = row_group
                    .columns
                    .iter()
                    .find(|column| column.name == *requested_name)
                    .ok_or_else(|| {
                        QrdError::InvalidSchema(format!(
                            "requested column not found: {requested_name}"
                        ))
                    })?;
                let decoded = chunk.decode()?;
                output.extend_from_slice(&decoded);
            }
        }
        Ok(outputs)
    }

    /// Parses a row group from a canonical binary representation.
    pub fn read_row_group(&self, bytes: &[u8]) -> Result<RowGroup> {
        RowGroup::deserialize(bytes)
    }

    /// Verifies the parsed input file integrity.
    pub fn verify_integrity(&self) -> Result<()> {
        if self.footer.row_group_count as usize != self.row_groups.len() {
            return Err(QrdError::InvalidSchema(
                "footer row group count mismatch".into(),
            ));
        }
        Ok(())
    }
}

