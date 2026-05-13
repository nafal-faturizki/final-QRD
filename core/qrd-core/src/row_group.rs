use crate::columnar::transpose_rows;
use crate::encoding::{decode, encode, EncodingId};
use crate::error::{QrdError, Result};
use crate::integrity::crc32;
use std::convert::TryFrom;

/// Column chunk stored inside a row group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnChunk {
    pub name: String,
    pub encoding: EncodingId,
    pub data: Vec<u8>,
}

impl ColumnChunk {
    /// Builds a column chunk from raw bytes using the selected encoding.
    pub fn new(name: impl Into<String>, data: &[u8], encoding: EncodingId) -> Result<Self> {
        Ok(Self {
            name: name.into(),
            encoding,
            data: encode(data, encoding)?,
        })
    }

    /// Returns the decoded payload.
    pub fn decode(&self) -> Result<Vec<u8>> {
        decode(&self.data, self.encoding)
    }
}

/// Row group container for Phase 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowGroup {
    pub row_count: u32,
    pub columns: Vec<ColumnChunk>,
}

impl RowGroup {
    /// Creates a row group from a list of row buffers using generic column labels.
    pub fn from_rows(rows: &[Vec<u8>]) -> Result<Self> {
        let width = rows.first().map(Vec::len).unwrap_or(0);
        let mut column_names = Vec::with_capacity(width);
        for column_index in 0..width {
            column_names.push(format!("col{column_index}"));
        }
        let column_refs: Vec<&str> = column_names.iter().map(String::as_str).collect();
        Self::from_rows_with_names(rows, &column_refs)
    }

    /// Creates a row group from a list of row buffers and explicit column names.
    pub fn from_rows_with_names(rows: &[Vec<u8>], column_names: &[&str]) -> Result<Self> {
        if !rows.is_empty() {
            let row_count = u32::try_from(rows.len())
                .map_err(|_| QrdError::InvalidSchema("row group is too large".into()))?;
            let width = rows[0].len();
            if rows.iter().any(|row| row.len() != width) {
                return Err(QrdError::InvalidSchema(
                    "rows must have uniform width".into(),
                ));
            }
            if column_names.len() != width {
                return Err(QrdError::InvalidSchema(
                    "column count does not match row width".into(),
                ));
            }

            let columns_data = transpose_rows(rows)?;
            let mut columns = Vec::with_capacity(width);
            for (name, column) in column_names.iter().zip(columns_data) {
                columns.push(ColumnChunk::new(*name, &column, EncodingId::Plain)?);
            }

            return Ok(Self { row_count, columns });
        }

        // Empty row group preserves schema field names when provided.
        let mut columns = Vec::with_capacity(column_names.len());
        for name in column_names {
            columns.push(ColumnChunk::new(*name, &[], EncodingId::Plain)?);
        }

        Ok(Self {
            row_count: 0,
            columns,
        })
    }

    /// Serializes the row group into a canonical binary representation.
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let column_count = u32::try_from(self.columns.len())
            .map_err(|_| QrdError::InvalidSchema("too many columns in row group".into()))?;

        let mut body = Vec::new();
        body.extend_from_slice(&self.row_count.to_le_bytes());
        body.extend_from_slice(&column_count.to_le_bytes());

        for column in &self.columns {
            let name_bytes = column.name.as_bytes();
            let name_len = u8::try_from(name_bytes.len())
                .map_err(|_| QrdError::InvalidSchema("column name too long".into()))?;
            let data_len = u32::try_from(column.data.len())
                .map_err(|_| QrdError::InvalidSchema("column chunk too large".into()))?;

            body.push(name_len);
            body.extend_from_slice(name_bytes);
            body.push(column.encoding.as_u8());
            body.extend_from_slice(&data_len.to_le_bytes());
            body.extend_from_slice(&column.data);
            let checksum = crc32(&column.data);
            body.extend_from_slice(&checksum.to_le_bytes());
        }

        Ok(body)
    }

    /// Parses a row group from the canonical binary representation.
    pub fn deserialize(bytes: &[u8]) -> Result<Self> {
        let mut cursor = 0usize;
        let row_count_bytes = bytes
            .get(cursor..cursor + 4)
            .ok_or(QrdError::UnexpectedEof)?;
        let row_count = u32::from_le_bytes([
            row_count_bytes[0],
            row_count_bytes[1],
            row_count_bytes[2],
            row_count_bytes[3],
        ]);
        cursor += 4;

        let column_count_bytes = bytes
            .get(cursor..cursor + 4)
            .ok_or(QrdError::UnexpectedEof)?;
        let column_count = u32::from_le_bytes([
            column_count_bytes[0],
            column_count_bytes[1],
            column_count_bytes[2],
            column_count_bytes[3],
        ]) as usize;
        cursor += 4;

        let mut columns = Vec::with_capacity(column_count);
        for _ in 0..column_count {
            let name_len = *bytes.get(cursor).ok_or(QrdError::UnexpectedEof)? as usize;
            cursor += 1;

            let name_end = cursor
                .checked_add(name_len)
                .ok_or_else(|| QrdError::InvalidSchema("column header overflow".into()))?;
            let name_bytes = bytes.get(cursor..name_end).ok_or(QrdError::UnexpectedEof)?;
            let name = std::str::from_utf8(name_bytes)
                .map_err(|_| QrdError::InvalidSchema("column name is not utf-8".into()))?
                .to_string();
            cursor = name_end;

            let encoding = EncodingId::from_u8(*bytes.get(cursor).ok_or(QrdError::UnexpectedEof)?)?;
            cursor += 1;

            let data_len_bytes = bytes
                .get(cursor..cursor + 4)
                .ok_or(QrdError::UnexpectedEof)?;
            let data_len = u32::from_le_bytes([
                data_len_bytes[0],
                data_len_bytes[1],
                data_len_bytes[2],
                data_len_bytes[3],
            ]) as usize;
            cursor += 4;

            let data_end = cursor
                .checked_add(data_len)
                .ok_or_else(|| QrdError::InvalidSchema("column data overflow".into()))?;
            let data = bytes
                .get(cursor..data_end)
                .ok_or(QrdError::UnexpectedEof)?
                .to_vec();
            cursor = data_end;

            let checksum_bytes = bytes
                .get(cursor..cursor + 4)
                .ok_or(QrdError::UnexpectedEof)?;
            let expected_checksum = u32::from_le_bytes([
                checksum_bytes[0],
                checksum_bytes[1],
                checksum_bytes[2],
                checksum_bytes[3],
            ]);
            cursor += 4;

            if crc32(&data) != expected_checksum {
                return Err(QrdError::InvalidSchema("column checksum mismatch".into()));
            }

            columns.push(ColumnChunk {
                name,
                encoding,
                data,
            });
        }

        if cursor != bytes.len() {
            return Err(QrdError::InvalidSchema(
                "trailing row group bytes detected".into(),
            ));
        }

        Ok(Self { row_count, columns })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_group_roundtrip_is_stable() {
        let rows = vec![vec![1, 2, 3], vec![4, 5, 6]];
        let row_group = RowGroup::from_rows(&rows).expect("row group should build");
        let bytes = row_group.serialize().expect("row group should serialize");
        let parsed = RowGroup::deserialize(&bytes).expect("row group should parse");

        assert_eq!(parsed, row_group);
        assert_eq!(parsed.row_count, 2);
        assert_eq!(parsed.columns.len(), 3);
    }

    #[test]
    fn row_group_rejects_truncated_input() {
        assert!(RowGroup::deserialize(&[0, 0, 0]).is_err());
    }
}
