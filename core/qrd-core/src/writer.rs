use crate::error::{QrdError, Result};
use crate::parser::{build_footer, FileHeader};
use crate::row_group::RowGroup;
use crate::schema::Schema;

/// Minimal streaming writer scaffold.
pub struct StreamingWriter {
    schema: Schema,
    finished: bool,
    header: FileHeader,
    row_group_count: u32,
    row_groups: Vec<Vec<u8>>,
}

impl StreamingWriter {
    /// Creates a new writer.
    pub fn new(schema: Schema) -> Self {
        let header = FileHeader::new(1, 0, schema.fingerprint(), 0, *b"qrd-0.1.0\0\0\0");
        Self {
            schema,
            finished: false,
            header,
            row_group_count: 0,
            row_groups: Vec::new(),
        }
    }

    /// Returns the canonical file header that will be written.
    pub fn header(&self) -> &FileHeader {
        &self.header
    }

    /// Writes a row group.
    pub fn write_row_group(&mut self, _rows: &[Vec<u8>]) -> Result<()> {
        if self.finished {
            return Err(QrdError::InvalidSchema("writer already finished".into()));
        }
        let row_group = RowGroup::from_rows(_rows)?;
        let serialized = row_group.serialize()?;
        self.row_group_count = self
            .row_group_count
            .checked_add(1)
            .ok_or_else(|| QrdError::InvalidSchema("row group count overflow".into()))?;
        self.row_groups.push(serialized);
        Ok(())
    }

    /// Builds the canonical footer bytes for the current write session.
    pub fn build_footer_bytes(&self) -> Result<Vec<u8>> {
        build_footer(&self.schema, self.row_group_count)
    }

    /// Returns the serialized row groups accumulated so far.
    pub fn row_groups(&self) -> &[Vec<u8>] {
        &self.row_groups
    }

    /// Finalizes the file.
    pub fn finish(mut self) -> Result<Schema> {
        self.finished = true;
        Ok(self.schema)
    }
}
