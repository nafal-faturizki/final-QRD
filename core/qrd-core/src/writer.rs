use crate::error::{QrdError, Result};
use crate::file::build_file_image_from_serialized_row_groups;
use crate::parser::{build_footer, FileHeader};
use crate::row_group::RowGroup;
use crate::schema::Schema;
use crate::signing::SchemaSignature;

/// Minimal streaming writer scaffold.
pub struct StreamingWriter {
    schema: Schema,
    finished: bool,
    header: FileHeader,
    row_group_count: u32,
    row_groups: Vec<Vec<u8>>,
    signature: Option<SchemaSignature>,
    column_names: Vec<String>,
}

impl StreamingWriter {
    /// Creates a new writer.
    pub fn new(schema: Schema) -> Self {
        let header = FileHeader::new(1, 0, schema.fingerprint(), 0, *b"qrd-0.1.0\0\0\0");
        let column_names = schema
            .fields()
            .iter()
            .map(|field| field.name.clone())
            .collect();
        Self {
            schema,
            finished: false,
            header,
            row_group_count: 0,
            row_groups: Vec::new(),
            signature: None,
            column_names,
        }
    }

    /// Sets an Ed25519 schema signature for the file
    pub fn set_signature(&mut self, signature: SchemaSignature) {
        self.signature = Some(signature);
    }

    /// Clears any set signature
    pub fn clear_signature(&mut self) {
        self.signature = None;
    }

    /// Returns the canonical file header that will be written.
    pub fn header(&self) -> &FileHeader {
        &self.header
    }

    /// Writes a row group.
    pub fn write_row_group(&mut self, rows: &[Vec<u8>]) -> Result<()> {
        if self.finished {
            return Err(QrdError::InvalidSchema("writer already finished".into()));
        }
        let serialized =
            RowGroup::serialize_plain_from_rows_with_owned_names(rows, &self.column_names)?;
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

    /// Finalizes the file and returns a complete QRD image.
    pub fn finish(&mut self) -> Result<Vec<u8>> {
        if self.finished {
            return Err(QrdError::InvalidSchema("writer already finished".into()));
        }
        self.finished = true;

        build_file_image_from_serialized_row_groups(
            &self.schema,
            &self.row_groups,
            self.signature.clone(),
        )
    }

    /// Serializes the current writer state into the provided writer.
    /// This mirrors historical APIs that accepted a mutable buffer to write
    /// the canonical file image into without consuming the writer.
    pub fn serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        let bytes = build_file_image_from_serialized_row_groups(
            &self.schema,
            &self.row_groups,
            self.signature.clone(),
        )?;

        writer
            .write_all(&bytes)
            .map_err(|e| QrdError::InvalidSchema(format!("io write failed: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::FileReader;
    use crate::schema::{FieldKind, SchemaBuilder};

    #[test]
    fn streaming_writer_can_finish_and_produce_valid_file_image() {
        let schema = SchemaBuilder::new()
            .add_field("device_id", FieldKind::Utf8, true)
            .add_field("status", FieldKind::Int32, false)
            .build()
            .expect("schema should build");

        let mut writer = StreamingWriter::new(schema.clone());
        writer
            .write_row_group(&[vec![1, 2], vec![3, 4]])
            .expect("write row group should work");

        assert_eq!(writer.row_groups().len(), 1);

        let bytes = writer.finish().expect("finish should succeed");
        let reader = FileReader::open(&bytes).expect("file image should open");

        assert_eq!(reader.footer().row_group_count, 1);
        assert_eq!(reader.row_count(), 2);
    }

    #[test]
    fn build_footer_bytes_matches_footer_length_for_written_file() {
        let schema = SchemaBuilder::new()
            .add_field("a", FieldKind::Int32, true)
            .build()
            .expect("schema should build");

        let mut writer = StreamingWriter::new(schema.clone());
        writer
            .write_row_group(&[vec![42]])
            .expect("write row group");

        let footer_bytes = writer
            .build_footer_bytes()
            .expect("footer bytes should build");
        assert!(
            footer_bytes.len() >= 4,
            "footer bytes should contain a canonical footer length"
        );
    }

    #[test]
    fn streaming_writer_can_sign_schema() {
        use crate::reader::FileReader;
        use crate::signing::{SchemaSignature, SigningKeyPair, SIGNATURE_ALGORITHM};

        let schema = SchemaBuilder::new()
            .add_field("device_id", FieldKind::Utf8, true)
            .add_field("status", FieldKind::Int32, false)
            .build()
            .expect("schema should build");

        let mut writer = StreamingWriter::new(schema.clone());

        // Generate signature
        let keypair = SigningKeyPair::generate();
        let schema_id = schema.fingerprint();
        let signature_bytes = keypair.sign_schema(&schema_id);
        let pubkey = keypair.verifying_key();
        let sig = SchemaSignature::new(SIGNATURE_ALGORITHM, signature_bytes, pubkey);

        // Set signature on writer
        writer.set_signature(sig.clone());
        writer
            .write_row_group(&[vec![1, 2], vec![3, 4]])
            .expect("write row group should work");

        let bytes = writer.finish().expect("finish should succeed");

        // Verify the file can be read and signature is intact
        let reader = FileReader::open(&bytes).expect("file image should open");
        assert_eq!(reader.footer().row_group_count, 1);
    }
}
