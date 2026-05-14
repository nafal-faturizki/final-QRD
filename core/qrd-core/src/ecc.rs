use crate::error::{QrdError, Result};

const GF_PRIMITIVE: u16 = 0x11d;

#[inline]
fn gf_add(a: u8, b: u8) -> u8 {
    a ^ b
}

#[inline]
fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut result = 0u16;
    while b != 0 {
        if (b & 1) != 0 {
            result ^= a as u16;
        }
        let carry = (a & 0x80) != 0;
        a <<= 1;
        if carry {
            a ^= (GF_PRIMITIVE & 0xFF) as u8;
        }
        b >>= 1;
    }
    result as u8
}

#[inline]
fn gf_pow(mut base: u8, mut exponent: u32) -> u8 {
    let mut result = 1u8;
    while exponent != 0 {
        if (exponent & 1) != 0 {
            result = gf_mul(result, base);
        }
        base = gf_mul(base, base);
        exponent >>= 1;
    }
    result
}

#[inline]
fn gf_inv(value: u8) -> Result<u8> {
    if value == 0 {
        return Err(QrdError::InvalidSchema(
            "zero has no multiplicative inverse".into(),
        ));
    }
    Ok(gf_pow(value, 254))
}

#[allow(dead_code)]
#[inline]
fn gf_div(a: u8, b: u8) -> Result<u8> {
    if b == 0 {
        return Err(QrdError::InvalidSchema("division by zero".into()));
    }
    Ok(gf_mul(a, gf_inv(b)?))
}

#[inline]
fn encode_coefficient(parity_index: usize, data_index: usize) -> u8 {
    let exponent = ((parity_index + 1) as u32)
        .checked_mul(data_index as u32)
        .unwrap_or(0)
        % 255;
    gf_pow(2, exponent)
}

fn solve_linear_system(matrix: &mut [Vec<u8>], rhs: &mut [u8]) -> Result<Vec<u8>> {
    let n = rhs.len();
    for pivot in 0..n {
        let mut row = pivot;
        while row < n && matrix[row][pivot] == 0 {
            row += 1;
        }

        if row == n {
            return Err(QrdError::InvalidSchema(
                "singular Reed-Solomon recovery matrix".into(),
            ));
        }

        if row != pivot {
            matrix.swap(row, pivot);
            rhs.swap(row, pivot);
        }

        let inv_pivot = gf_inv(matrix[pivot][pivot])?;
        for col in pivot..n {
            matrix[pivot][col] = gf_mul(matrix[pivot][col], inv_pivot);
        }
        rhs[pivot] = gf_mul(rhs[pivot], inv_pivot);

        for elim_row in (pivot + 1)..n {
            let factor = matrix[elim_row][pivot];
            if factor != 0 {
                for col in pivot..n {
                    matrix[elim_row][col] = gf_add(
                        matrix[elim_row][col],
                        gf_mul(factor, matrix[pivot][col]),
                    );
                }
                rhs[elim_row] = gf_add(rhs[elim_row], gf_mul(factor, rhs[pivot]));
            }
        }
    }

    let mut solution = vec![0u8; n];
    for row in (0..n).rev() {
        let mut value = rhs[row];
        for col in (row + 1)..n {
            value = gf_add(value, gf_mul(matrix[row][col], solution[col]));
        }
        solution[row] = value;
    }
    Ok(solution)
}

/// Reed-Solomon configuration for error correction.
///
/// # Configuration Examples
/// - RS(2, 1): 2 data chunks + 1 parity chunk, tolerates 1 failure
/// - RS(4, 2): 4 data chunks + 2 parity chunks, tolerates 2 failures
/// - RS(8, 4): 8 data chunks + 4 parity chunks, tolerates 4 failures
/// - RS(16, 4): 16 data chunks + 4 parity chunks, tolerates 4 failures (typical)
/// - RS(32, 8): 32 data chunks + 8 parity chunks, tolerates 8 failures (typical)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReedSolomonConfig {
    /// Number of data chunks (actual payload)
    pub data_chunks: usize,
    /// Number of parity chunks for error correction
    pub parity_chunks: usize,
}

impl ReedSolomonConfig {
    /// Creates a new RS configuration.
    /// Returns error if configuration is invalid.
    pub fn new(data_chunks: usize, parity_chunks: usize) -> Result<Self> {
        if data_chunks == 0 {
            return Err(QrdError::InvalidSchema("data chunks must be > 0".into()));
        }
        if parity_chunks == 0 {
            return Err(QrdError::InvalidSchema("parity chunks must be > 0".into()));
        }
        Ok(Self {
            data_chunks,
            parity_chunks,
        })
    }

    /// Total number of chunks (data + parity)
    pub fn total_chunks(&self) -> usize {
        self.data_chunks + self.parity_chunks
    }

    /// Maximum number of failures that can be recovered
    pub fn recovery_capacity(&self) -> usize {
        self.parity_chunks
    }
}

fn recover_missing_data_chunks(
    known_data: &[(usize, &Vec<u8>)],
    known_parity: &[(usize, &Vec<u8>)],
    missing_data_indices: &[usize],
    width: usize,
    config: ReedSolomonConfig,
) -> Result<Vec<Vec<u8>>> {
    let missing_count = missing_data_indices.len();
    if known_parity.len() < missing_count {
        return Err(QrdError::InvalidSchema(
            "not enough parity chunks to recover missing data".into(),
        ));
    }

    let equations = &known_parity[..missing_count];
    let mut coefficient_matrix = vec![vec![0u8; missing_count]; missing_count];
    for (row, (parity_index, _)) in equations.iter().enumerate() {
        let parity_row = parity_index - config.data_chunks;
        for (col, &data_index) in missing_data_indices.iter().enumerate() {
            coefficient_matrix[row][col] = encode_coefficient(parity_row, data_index);
        }
    }

    let mut recovered = vec![vec![0u8; width]; missing_count];
    for byte_index in 0..width {
        let mut matrix = coefficient_matrix.clone();
        let mut rhs = Vec::with_capacity(missing_count);

        for (parity_index, parity_chunk) in equations {
            let parity_row = parity_index - config.data_chunks;
            let mut value = parity_chunk[byte_index];
            for (data_index, chunk) in known_data {
                let coeff = encode_coefficient(parity_row, *data_index);
                value = gf_add(value, gf_mul(coeff, chunk[byte_index]));
            }
            rhs.push(value);
        }

        let symbols = solve_linear_system(&mut matrix, &mut rhs)?;
        for (i, symbol) in symbols.into_iter().enumerate() {
            recovered[i][byte_index] = symbol;
        }
    }

    Ok(recovered)
}

/// Encodes parity chunks from data chunks using Reed-Solomon over GF(256).
///
/// Parity chunk i is computed as the linear combination of data chunks with
/// distinct Vandermonde coefficients. This supports multi-parity recovery
/// semantics for Phase 1.
pub fn encode(data: &[Vec<u8>], config: ReedSolomonConfig) -> Result<Vec<Vec<u8>>> {
    if data.len() != config.data_chunks {
        return Err(QrdError::InvalidSchema(format!(
            "expected {} data chunks, got {}",
            config.data_chunks,
            data.len()
        )));
    }

    if config.parity_chunks == 0 {
        return Ok(Vec::new());
    }

    if data.is_empty() {
        return Err(QrdError::InvalidSchema("no data chunks".into()));
    }

    let width = data[0].len();
    if data.iter().any(|chunk| chunk.len() != width) {
        return Err(QrdError::InvalidSchema(
            "all data chunks must have uniform width".into(),
        ));
    }

    let mut parity_chunks = vec![vec![0u8; width]; config.parity_chunks];
    for (data_index, chunk) in data.iter().enumerate() {
        for parity_index in 0..config.parity_chunks {
            let coeff = encode_coefficient(parity_index, data_index);
            if coeff == 0 {
                continue;
            }
            for byte_index in 0..width {
                parity_chunks[parity_index][byte_index] = gf_add(
                    parity_chunks[parity_index][byte_index],
                    gf_mul(coeff, chunk[byte_index]),
                );
            }
        }
    }

    Ok(parity_chunks)
}

pub fn recover_missing_chunk(
    data: &[Option<Vec<u8>>],
    config: ReedSolomonConfig,
) -> Result<Vec<u8>> {
    let total = config.total_chunks();
    if data.len() != total {
        return Err(QrdError::InvalidSchema(format!(
            "expected {} total chunks, got {}",
            total,
            data.len()
        )));
    }

    let missing_indices: Vec<usize> = data
        .iter()
        .enumerate()
        .filter(|(_, chunk)| chunk.is_none())
        .map(|(index, _)| index)
        .collect();

    let missing_count = missing_indices.len();
    if missing_count == 0 {
        return Err(QrdError::InvalidSchema("no chunks are missing".into()));
    }
    if missing_count > config.recovery_capacity() {
        return Err(QrdError::InvalidSchema(format!(
            "cannot recover {} missing chunks (only {} parity chunks available)",
            missing_count,
            config.parity_chunks
        )));
    }

    let mut width = None;
    for candidate in data.iter().filter_map(|chunk| chunk.as_ref()) {
        if let Some(existing) = width {
            if candidate.len() != existing {
                return Err(QrdError::InvalidSchema(
                    "all available chunks must have the same width".into(),
                ));
            }
        } else {
            width = Some(candidate.len());
        }
    }

    let width = width.ok_or(QrdError::UnexpectedEof)?;

    let requested_index = missing_indices[0];
    let data_chunks: Vec<(usize, &Vec<u8>)> = data
        .iter()
        .enumerate()
        .filter_map(|(index, chunk)| chunk.as_ref().map(|chunk| (index, chunk)))
        .filter(|(index, _)| *index < config.data_chunks)
        .collect();

    let parity_chunks: Vec<(usize, &Vec<u8>)> = data
        .iter()
        .enumerate()
        .filter_map(|(index, chunk)| chunk.as_ref().map(|chunk| (index, chunk)))
        .filter(|(index, _)| *index >= config.data_chunks)
        .collect();

    let missing_data_indices: Vec<usize> = missing_indices
        .iter()
        .copied()
        .filter(|&index| index < config.data_chunks)
        .collect();

    if missing_data_indices.is_empty() {
        let data_values: Vec<Vec<u8>> = (0..config.data_chunks)
            .map(|i| data[i].as_ref().cloned().unwrap())
            .collect();
        let parity_values = encode(&data_values, config)?;
        let parity_index = requested_index - config.data_chunks;
        return Ok(parity_values[parity_index].clone());
    }

    let recovered_data = recover_missing_data_chunks(
        &data_chunks,
        &parity_chunks,
        &missing_data_indices,
        width,
        config,
    )?;

    let target_chunk = if requested_index < config.data_chunks {
        let position = missing_data_indices
            .iter()
            .position(|&index| index == requested_index)
            .ok_or_else(|| QrdError::InvalidSchema("requested chunk not recoverable".into()))?;
        recovered_data[position].clone()
    } else {
        let mut combined_data = vec![vec![0u8; width]; config.data_chunks];
        for (idx, chunk) in &data_chunks {
            if *idx < config.data_chunks {
                combined_data[*idx] = (*chunk).clone();
            }
        }
        for (position, &missing_index) in missing_data_indices.iter().enumerate() {
            combined_data[missing_index] = recovered_data[position].clone();
        }
        let parity_values = encode(&combined_data, config)?;
        let parity_index = requested_index - config.data_chunks;
        parity_values[parity_index].clone()
    };

    Ok(target_chunk)
}

pub fn verify(data: &[Vec<u8>], parity: &[Vec<u8>], config: ReedSolomonConfig) -> Result<bool> {
    if data.len() != config.data_chunks {
        return Err(QrdError::InvalidSchema(format!(
            "expected {} data chunks, got {}",
            config.data_chunks,
            data.len()
        )));
    }

    if parity.len() != config.parity_chunks {
        return Err(QrdError::InvalidSchema(format!(
            "expected {} parity chunks, got {}",
            config.parity_chunks,
            parity.len()
        )));
    }

    let computed = encode(data, config)?;
    Ok(parity == computed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rs_config_validates() {
        let config = ReedSolomonConfig::new(2, 1).expect("config should be valid");
        assert_eq!(config.data_chunks, 2);
        assert_eq!(config.parity_chunks, 1);
        assert_eq!(config.total_chunks(), 3);
        assert_eq!(config.recovery_capacity(), 1);

        assert!(ReedSolomonConfig::new(0, 1).is_err());
        assert!(ReedSolomonConfig::new(2, 0).is_err());
        assert!(ReedSolomonConfig::new(2, 3).is_ok());
    }

    #[test]
    fn parity_chunk_is_xor_of_input() {
        let data = vec![vec![1u8, 2, 3], vec![4, 5, 6]];
        let config = ReedSolomonConfig::new(2, 1).expect("config should be valid");
        let parity = encode(&data, config).expect("ecc should encode");
        assert_eq!(parity.len(), 1);
        assert_eq!(parity[0], vec![5, 7, 5]);
    }

    #[test]
    fn parity_encode_rejects_width_mismatch() {
        let data = vec![vec![1u8, 2], vec![3]];
        let config = ReedSolomonConfig::new(2, 1).expect("config should be valid");
        assert!(encode(&data, config).is_err());
    }

    #[test]
    fn gf_division_behaves_as_expected() {
        assert_eq!(gf_div(1, 1).expect("1/1 should succeed"), 1);
        let result = gf_div(1, 0);
        assert!(matches!(result, Err(_)));
    }

    #[test]
    fn single_chunk_recovery_via_parity() {
        let data = vec![vec![1u8, 2, 3], vec![4, 5, 6]];
        let config = ReedSolomonConfig::new(2, 1).expect("config should be valid");
        let parity = encode(&data, config).expect("ecc should encode");
        let corrupted = vec![None, Some(data[1].clone()), Some(parity[0].clone())];
        let recovered = recover_missing_chunk(&corrupted, config).expect("recovery should work");
        assert_eq!(recovered, data[0]);
    }

    #[test]
    fn recover_two_data_chunks_with_two_parity_chunks() {
        let data = vec![
            vec![1u8, 2, 3],
            vec![4, 5, 6],
            vec![7, 8, 9],
            vec![10, 11, 12],
        ];
        let config = ReedSolomonConfig::new(4, 2).expect("config should be valid");
        let parity = encode(&data, config).expect("ecc should encode");

        let corrupted = vec![
            None,
            Some(data[1].clone()),
            None,
            Some(data[3].clone()),
            Some(parity[0].clone()),
            Some(parity[1].clone()),
        ];

        let recovered = recover_missing_chunk(&corrupted, config).expect("recovery should work");
        assert_eq!(recovered, data[0]);
    }

    #[test]
    fn parity_chunk_recovery_when_data_is_intact() {
        let data = vec![vec![1u8, 2, 3], vec![4, 5, 6]];
        let config = ReedSolomonConfig::new(2, 2).expect("config should be valid");
        let parity = encode(&data, config).expect("ecc should encode");
        let corrupted = vec![Some(data[0].clone()), Some(data[1].clone()), None, Some(parity[1].clone())];
        let recovered = recover_missing_chunk(&corrupted, config).expect("recovery should work");
        assert_eq!(recovered, parity[0]);
    }

    #[test]
    fn recovery_rejects_too_many_missing_chunks() {
        let data = vec![vec![1u8, 2, 3], vec![4, 5, 6]];
        let config = ReedSolomonConfig::new(2, 1).expect("config should be valid");
        let parity = encode(&data, config).expect("ecc should encode");
        let corrupted = vec![None, None, Some(parity[0].clone())];
        assert!(recover_missing_chunk(&corrupted, config).is_err());
    }

    #[test]
    fn verify_detects_valid_parity() {
        let data = vec![vec![1u8, 2, 3], vec![4, 5, 6]];
        let config = ReedSolomonConfig::new(2, 1).expect("config should be valid");
        let parity = encode(&data, config).expect("ecc should encode");
        assert!(verify(&data, &parity, config).unwrap());
    }

    #[test]
    fn verify_detects_corrupted_parity() {
        let data = vec![vec![1u8, 2, 3], vec![4, 5, 6]];
        let config = ReedSolomonConfig::new(2, 1).expect("config should be valid");
        let mut parity = encode(&data, config).expect("ecc should encode");
        parity[0][0] ^= 0xFF;
        assert!(!verify(&data, &parity, config).unwrap());
    }

    #[test]
    fn multi_parity_chunks_generated() {
        let data = vec![vec![1u8, 2, 3], vec![4, 5, 6]];
        let config = ReedSolomonConfig::new(2, 3).expect("config should be valid");
        let parity = encode(&data, config).expect("ecc should encode");
        assert_eq!(parity.len(), 3);
        for p in parity {
            assert_eq!(p.len(), 3);
        }
    }
}
