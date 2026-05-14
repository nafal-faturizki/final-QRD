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

    /// Backwards-compatible constructor used by tests and external callers.
    /// Identical to `open` but matches historical API name `from_bytes`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Self::open(bytes)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::build_file_image;
    use crate::row_group::RowGroup;
    use crate::schema::{FieldKind, SchemaBuilder};

    #[test]
    fn reader_open_roundtrip_reads_selected_columns() {
        let schema = SchemaBuilder::new()
            .add_field("device_id", FieldKind::Utf8, true)
            .add_field("temperature", FieldKind::Int32, false)
            .build()
            .expect("schema should build");

        let row_groups = vec![RowGroup::from_rows_with_names(
            &[vec![1, 2], vec![3, 4]],
            &["device_id", "temperature"],
        )
        .expect("row group should build")];

        let bytes = build_file_image(&schema, &row_groups).expect("file image should build");
        let reader = FileReader::open(&bytes).expect("reader should open");

        let columns = reader
            .read_columns(&["device_id"])
            .expect("should read requested columns");

        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0], vec![1, 3]);
        assert_eq!(reader.row_count(), 2);
        assert_eq!(reader.footer.row_group_count, 1);
    }

    #[test]
    fn reader_inspect_header_footer_and_verify_integrity() {
        let schema = SchemaBuilder::new()
            .add_field("id", FieldKind::Int32, true)
            .build()
            .expect("schema should build");

        let row_groups = vec![
            RowGroup::from_rows_with_names(&[vec![10], vec![20]], &["id"])
                .expect("row group should build"),
        ];
        let bytes = build_file_image(&schema, &row_groups).expect("file image should build");
        let reader = FileReader::open(&bytes).expect("reader should open");

        let header =
            FileReader::inspect_header(&reader.header.serialize()).expect("header should inspect");
        assert_eq!(header.schema_id, schema.fingerprint());

        let footer = FileReader::inspect_footer(
            &crate::footer::build_footer(&schema, 1).expect("footer should build"),
        )
        .expect("footer should inspect");
        assert_eq!(footer.row_group_count, 1);

        reader.verify_integrity().expect("integrity should verify");
    }

    #[test]
    fn read_row_group_parses_serialized_row_group() {
        let schema = SchemaBuilder::new()
            .add_field("col0", FieldKind::Int32, true)
            .add_field("col1", FieldKind::Int32, true)
            .build()
            .expect("schema should build");

        let reader = FileReader::new(schema);
        let row_group =
            RowGroup::from_rows_with_names(&[vec![7, 8], vec![9, 10]], &["col0", "col1"])
                .expect("row group should build");

        let bytes = row_group.serialize().expect("row group should serialize");
        let parsed = reader
            .read_row_group(&bytes)
            .expect("row group should parse");
        assert_eq!(parsed, row_group);
    }
}
