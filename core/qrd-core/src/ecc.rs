use crate::error::{QrdError, Result};
use reed_solomon_erasure::galois_8::Field as Gf8;
use reed_solomon_erasure::ReedSolomon;
// The crate provides GF implementations; keep a minimal GF helper for a few
// low-level unit tests that assert basic finite-field division behaviour.
const GF_PRIMITIVE: u16 = 0x11d;

#[inline]
#[allow(dead_code)]
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

// Old internal recovery helper removed — using `reed-solomon-erasure` crate
// for encoding and reconstruction.

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

    // Use reed-solomon-erasure crate to compute parity shards
    let r = ReedSolomon::<Gf8>::new(config.data_chunks, config.parity_chunks)
        .map_err(|e| QrdError::InvalidSchema(format!("reed-solomon init failed: {}", e)))?;

    // Prepare shards: data shards followed by parity shards (zeroed)
    let mut shards: Vec<Vec<u8>> = data.to_vec();
    shards.extend((0..config.parity_chunks).map(|_| vec![0u8; width]));

    // Convert to mutable slice refs
    let mut shard_refs: Vec<&mut [u8]> = shards.iter_mut().map(|s| s.as_mut_slice()).collect();
    r.encode(&mut shard_refs)
        .map_err(|e| QrdError::InvalidSchema(format!("reed-solomon encode failed: {}", e)))?;

    // Extract parity shards
    let parity = shards.split_off(config.data_chunks);
    Ok(parity)
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
            missing_count, config.parity_chunks
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

    let _width = width.ok_or(QrdError::UnexpectedEof)?;

    let r = ReedSolomon::<Gf8>::new(config.data_chunks, config.parity_chunks)
        .map_err(|e| QrdError::InvalidSchema(format!("reed-solomon init failed: {}", e)))?;

    // Build vector of Option<Vec<u8>> for reconstruct API. Present shards
    // are cloned; missing shards are left as None and will be reconstructed.
    let mut shards_opt: Vec<Option<Vec<u8>>> = data.to_vec();

    r.reconstruct(&mut shards_opt)
        .map_err(|e| QrdError::InvalidSchema(format!("reed-solomon reconstruct failed: {}", e)))?;

    // Return requested missing chunk (take first missing index)
    let requested_index = missing_indices[0];
    Ok(shards_opt[requested_index]
        .as_ref()
        .ok_or_else(|| QrdError::InvalidSchema("reconstruction failed".into()))?
        .clone())
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

    let r = ReedSolomon::<Gf8>::new(config.data_chunks, config.parity_chunks)
        .map_err(|e| QrdError::InvalidSchema(format!("reed-solomon init failed: {}", e)))?;

    let mut shards: Vec<Vec<u8>> = data.to_vec();
    shards.extend_from_slice(parity);
    let shard_refs: Vec<&[u8]> = shards.iter().map(|s| s.as_slice()).collect();
    let ok = r
        .verify(&shard_refs)
        .map_err(|e| QrdError::InvalidSchema(format!("reed-solomon verify failed: {}", e)))?;
    Ok(ok)
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
        // With a real Reed-Solomon implementation the single parity shard will
        // not necessarily be a simple XOR. Validate by checking the parity
        // verifies correctly against the data.
        assert!(verify(&data, &parity, config).unwrap());
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
        let corrupted = vec![
            Some(data[0].clone()),
            Some(data[1].clone()),
            None,
            Some(parity[1].clone()),
        ];
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
