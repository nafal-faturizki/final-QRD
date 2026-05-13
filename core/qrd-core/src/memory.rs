use crate::error::{QrdError, Result};

/// Estimates the peak memory for a streaming writer.
pub fn estimate_writer_peak_memory(
    row_group_size: usize,
    avg_row_width_bytes: usize,
    dict_overhead_bytes: usize,
    ecc_overhead_bytes: usize,
) -> Result<usize> {
    let base = row_group_size
        .checked_mul(avg_row_width_bytes)
        .ok_or_else(|| QrdError::InvalidSchema("writer memory overflow".into()))?;
    base.checked_add(dict_overhead_bytes)
        .and_then(|value| value.checked_add(ecc_overhead_bytes))
        .ok_or_else(|| QrdError::InvalidSchema("writer memory overflow".into()))
}

/// Estimates the peak memory for a partial reader.
pub fn estimate_reader_peak_memory(
    selected_column_chunk_sizes: &[usize],
    active_parallel_row_groups: usize,
    footer_size_bytes: usize,
) -> Result<usize> {
    let column_sum = selected_column_chunk_sizes.iter().try_fold(0usize, |acc, size| {
        acc.checked_add(*size)
            .ok_or_else(|| QrdError::InvalidSchema("reader memory overflow".into()))
    })?;

    let base = column_sum
        .checked_mul(active_parallel_row_groups)
        .ok_or_else(|| QrdError::InvalidSchema("reader memory overflow".into()))?;
    base.checked_add(footer_size_bytes)
        .ok_or_else(|| QrdError::InvalidSchema("reader memory overflow".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writer_memory_estimate_is_bounded() {
        let peak = estimate_writer_peak_memory(1024, 16, 128, 64).expect("estimate should work");
        assert_eq!(peak, 1024 * 16 + 128 + 64);
    }

    #[test]
    fn reader_memory_estimate_is_bounded() {
        let peak = estimate_reader_peak_memory(&[100, 200, 300], 2, 4096)
            .expect("estimate should work");
        assert_eq!(peak, (100 + 200 + 300) * 2 + 4096);
    }

    #[test]
    fn writer_memory_estimate_overflow_is_rejected() {
        assert!(estimate_writer_peak_memory(usize::MAX, 2, 0, 0).is_err());
    }
}