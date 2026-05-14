use crate::error::{QrdError, Result};

/// Transposes row-oriented records into column-oriented buffers.
pub fn transpose_rows(rows: &[Vec<u8>]) -> Result<Vec<Vec<u8>>> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let width = rows[0].len();
    if rows.iter().any(|row| row.len() != width) {
        return Err(QrdError::InvalidSchema(
            "rows must have uniform width".into(),
        ));
    }

    let mut columns = (0..width)
        .map(|_| Vec::with_capacity(rows.len()))
        .collect::<Vec<Vec<u8>>>();
    for row in rows {
        for (column, value) in columns.iter_mut().zip(row) {
            column.push(*value);
        }
    }
    Ok(columns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transpose_rows_roundtrips_uniform_rows() {
        let rows = vec![vec![1, 2, 3], vec![4, 5, 6]];
        let columns = transpose_rows(&rows).expect("rows should transpose");

        assert_eq!(columns, vec![vec![1, 4], vec![2, 5], vec![3, 6]]);
    }

    #[test]
    fn transpose_rows_rejects_mismatched_width() {
        let rows = vec![vec![1, 2], vec![3]];
        assert!(transpose_rows(&rows).is_err());
    }
}
