use crate::error::{QrdError, Result};
use std::convert::TryFrom;

/// Primitive field kinds supported by the scaffold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FieldKind {
    Boolean = 0,
    Int32 = 1,
    Int64 = 2,
    Float32 = 3,
    Float64 = 4,
    Utf8 = 5,
}

/// Field definition in a QRD schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub kind: FieldKind,
    pub required: bool,
}

/// QRD schema representation for the scaffold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    fields: Vec<Field>,
}

impl Schema {
    /// Returns the schema fields.
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    /// Serializes the schema into a compact canonical binary format.
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let field_count = u8::try_from(self.fields.len())
            .map_err(|_| QrdError::InvalidSchema("schema has too many fields".into()))?;

        let mut bytes = Vec::new();
        bytes.push(field_count);
        for field in &self.fields {
            let name_bytes = field.name.as_bytes();
            let name_len = u8::try_from(name_bytes.len())
                .map_err(|_| QrdError::InvalidSchema("field name is too long".into()))?;
            bytes.push(name_len);
            bytes.extend_from_slice(name_bytes);
            bytes.push(field.kind as u8);
            bytes.push(u8::from(field.required));
        }
        Ok(bytes)
    }

    /// Parses a schema from the compact canonical binary format.
    pub fn deserialize(bytes: &[u8]) -> Result<Self> {
        let mut cursor = 0usize;
        let field_count = *bytes.get(cursor).ok_or(QrdError::UnexpectedEof)? as usize;
        cursor += 1;

        let mut fields = Vec::with_capacity(field_count);
        for _ in 0..field_count {
            let name_len = *bytes.get(cursor).ok_or(QrdError::UnexpectedEof)? as usize;
            cursor += 1;

            let end = cursor
                .checked_add(name_len)
                .ok_or_else(|| QrdError::InvalidSchema("schema length overflow".into()))?;
            let name_bytes = bytes.get(cursor..end).ok_or(QrdError::UnexpectedEof)?;
            let name = std::str::from_utf8(name_bytes)
                .map_err(|_| QrdError::InvalidSchema("field name is not UTF-8".into()))?
                .to_string();
            cursor = end;

            let kind = match *bytes.get(cursor).ok_or(QrdError::UnexpectedEof)? {
                0 => FieldKind::Boolean,
                1 => FieldKind::Int32,
                2 => FieldKind::Int64,
                3 => FieldKind::Float32,
                4 => FieldKind::Float64,
                5 => FieldKind::Utf8,
                other => {
                    return Err(QrdError::InvalidSchema(format!(
                        "unsupported field kind id: {other}"
                    )))
                }
            };
            cursor += 1;

            let required = match *bytes.get(cursor).ok_or(QrdError::UnexpectedEof)? {
                0 => false,
                1 => true,
                other => {
                    return Err(QrdError::InvalidSchema(format!(
                        "invalid required flag: {other}"
                    )))
                }
            };
            cursor += 1;

            fields.push(Field {
                name,
                kind,
                required,
            });
        }

        if cursor != bytes.len() {
            return Err(QrdError::InvalidSchema(
                "trailing schema bytes detected".into(),
            ));
        }

        Ok(Schema { fields })
    }

    /// Computes a stable 8-byte fingerprint for the schema.
    pub fn fingerprint(&self) -> [u8; 8] {
        fn mix(mut state: u64, byte: u8) -> u64 {
            state ^= u64::from(byte);
            state = state.wrapping_mul(0x100_0000_01B3);
            state
        }

        let mut state = 0xCBF2_9CE4_8422_2325u64;
        for field in &self.fields {
            for byte in field.name.as_bytes() {
                state = mix(state, *byte);
            }
            state = mix(state, 0xFF);
            state = mix(state, field.required as u8);
            state = mix(state, field.kind as u8);
        }
        state.to_le_bytes()
    }
}

/// Builder for `Schema`.
#[derive(Debug, Default)]
pub struct SchemaBuilder {
    fields: Vec<Field>,
}

impl SchemaBuilder {
    /// Creates a new schema builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a field to the schema.
    pub fn add_field(mut self, name: impl Into<String>, kind: FieldKind, required: bool) -> Self {
        self.fields.push(Field {
            name: name.into(),
            kind,
            required,
        });
        self
    }

    /// Builds a schema if all field names are unique.
    pub fn build(self) -> Result<Schema> {
        for (index, field) in self.fields.iter().enumerate() {
            if field.name.is_empty() {
                return Err(QrdError::InvalidSchema("field name cannot be empty".into()));
            }
            if self.fields[..index]
                .iter()
                .any(|existing| existing.name == field.name)
            {
                return Err(QrdError::InvalidSchema(format!(
                    "duplicate field name: {}",
                    field.name
                )));
            }
        }

        Ok(Schema { fields: self.fields })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_roundtrip_is_deterministic() {
        let schema = SchemaBuilder::new()
            .add_field("device_id", FieldKind::Utf8, true)
            .add_field("temperature", FieldKind::Float32, false)
            .build()
            .expect("schema should build");

        let first = schema.serialize().expect("schema should serialize");
        let second = schema.serialize().expect("schema should serialize");

        assert_eq!(first, second);
        assert_eq!(Schema::deserialize(&first).expect("schema should parse"), schema);
    }

    #[test]
    fn schema_rejects_truncated_input() {
        let error = Schema::deserialize(&[1, 4, b'n', b'a'])
            .expect_err("truncated schema must fail");
        assert!(matches!(error, QrdError::UnexpectedEof));
    }
}
