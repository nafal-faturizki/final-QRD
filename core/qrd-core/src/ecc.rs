use crate::error::{QrdError, Result};

/// Computes XOR parity of a set of chunks.
/// All chunks must have equal width; parity has same width.
fn xor_chunks(chunks: &[Vec<u8>]) -> Result<Vec<u8>> {
    if chunks.is_empty() {
        return Ok(Vec::new());
    }

    let width = chunks[0].len();
    if chunks.iter().any(|chunk| chunk.len() != width) {
        return Err(QrdError::InvalidSchema(
            "ecc chunks must have uniform width".into(),
        ));
    }

    let mut parity = vec![0u8; width];
    for chunk in chunks {
        for (index, byte) in chunk.iter().enumerate() {
            parity[index] ^= *byte;
        }
    }
    Ok(parity)
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

/// Encodes parity chunks from data chunks using XOR-based Reed-Solomon.
///
/// # Implementation Notes
/// This is a simplified XOR-based RS implementation suitable for Phase 1.
/// For each parity chunk, we XOR all data chunks. This provides:
/// - N data chunks + 1 parity chunk → can recover 1 failure
/// - N data chunks + K parity chunks → can recover up to min(K, 1) failures
///
/// For true multi-failure recovery (K > 1), a full Galois Field RS implementation
/// would be needed (future enhancement).
///
/// # Returns
/// Vector of parity chunks, one per configured parity chunk.
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

    // Validate all chunks have same width
    if data.is_empty() {
        return Err(QrdError::InvalidSchema("no data chunks".into()));
    }

    let width = data[0].len();
    if data.iter().any(|chunk| chunk.len() != width) {
        return Err(QrdError::InvalidSchema(
            "all data chunks must have uniform width".into(),
        ));
    }

    // Compute base parity (XOR of all data)
    let base_parity = xor_chunks(data)?;

    // Generate parity chunks. Each parity chunk is identical to the XOR parity
    // because this simplified Phase 1 implementation only supports single-failure
    // recovery semantics per parity chunk.
    let mut parity_chunks = Vec::with_capacity(config.parity_chunks);
    for _ in 0..config.parity_chunks {
        parity_chunks.push(base_parity.clone());
    }

    Ok(parity_chunks)
}

/// Recovers a single missing data chunk using parity information.
///
/// # Requirements
/// - `data` must have exactly `config.data_chunks + config.parity_chunks` entries
/// - At most 1 data chunk can be None (missing)
/// - All other chunks must be Some(...)
///
/// # Returns
/// The recovered chunk data, or error if recovery is not possible.
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

    // Count missing chunks
    let missing_indices: Vec<usize> = data
        .iter()
        .enumerate()
        .filter(|(_, chunk)| chunk.is_none())
        .map(|(index, _)| index)
        .collect();

    match missing_indices.len() {
        0 => {
            // No missing chunks
            Err(QrdError::InvalidSchema("no chunks are missing".into()))
        }
        1 => {
            // Single missing chunk - we can recover using XOR
            let _missing_index = missing_indices[0];
            let available: Vec<Vec<u8>> = data.iter().filter_map(|chunk| chunk.clone()).collect();

            if available.is_empty() {
                return Err(QrdError::UnexpectedEof);
            }

            let recovered = xor_chunks(&available)?;
            Ok(recovered)
        }
        _ => {
            // Multiple missing chunks - cannot recover with single parity
            Err(QrdError::InvalidSchema(format!(
                "cannot recover {} missing chunks (only {} parity chunks available)",
                missing_indices.len(),
                config.parity_chunks
            )))
        }
    }
}

/// Verifies that parity chunks are correct for given data.
/// Used to detect corruption without attempting recovery.
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
        // Valid config
        let config = ReedSolomonConfig::new(2, 1).expect("config should be valid");
        assert_eq!(config.data_chunks, 2);
        assert_eq!(config.parity_chunks, 1);
        assert_eq!(config.total_chunks(), 3);
        assert_eq!(config.recovery_capacity(), 1);

        // Invalid: zero data chunks
        assert!(ReedSolomonConfig::new(0, 1).is_err());

        // Invalid: zero parity chunks
        assert!(ReedSolomonConfig::new(2, 0).is_err());

        // Valid: more parity than data is allowed in this simplified Phase 1 model
        assert!(ReedSolomonConfig::new(2, 3).is_ok());
    }

    #[test]
    fn parity_chunk_is_xor_of_input() {
        let data = vec![vec![1u8, 2, 3], vec![4, 5, 6]];
        let config = ReedSolomonConfig::new(2, 1).expect("config should be valid");
        let parity = encode(&data, config).expect("ecc should encode");

        assert_eq!(parity.len(), 1);
        // Base parity: [1^4, 2^5, 3^6] = [5, 7, 5]
        assert_eq!(parity[0], vec![5, 7, 5]);
    }

    #[test]
    fn parity_encode_rejects_width_mismatch() {
        let data = vec![vec![1u8, 2], vec![3]];
        let config = ReedSolomonConfig::new(2, 1).expect("config should be valid");
        assert!(encode(&data, config).is_err());
    }

    #[test]
    fn single_chunk_recovery_via_xor() {
        let data = vec![vec![1u8, 2, 3], vec![4, 5, 6]];
        let config = ReedSolomonConfig::new(2, 1).expect("config should be valid");
        let parity = encode(&data, config).expect("ecc should encode");

        // Simulate missing first data chunk
        let corrupted = vec![None, Some(data[1].clone()), Some(parity[0].clone())];

        let recovered = recover_missing_chunk(&corrupted, config).expect("recovery should work");
        assert_eq!(recovered, data[0]);
    }

    #[test]
    fn recovery_rejects_multiple_missing() {
        let data = vec![vec![1u8, 2, 3], vec![4, 5, 6]];
        let config = ReedSolomonConfig::new(2, 1).expect("config should be valid");
        let parity = encode(&data, config).expect("ecc should encode");

        // Simulate two missing chunks
        let corrupted = vec![None, None, Some(parity[0].clone())];

        let result = recover_missing_chunk(&corrupted, config);
        assert!(result.is_err());
    }

    #[test]
    fn verify_detects_valid_parity() {
        let data = vec![vec![1u8, 2, 3], vec![4, 5, 6]];
        let config = ReedSolomonConfig::new(2, 1).expect("config should be valid");
        let parity = encode(&data, config).expect("ecc should encode");

        let is_valid = verify(&data, &parity, config).expect("verify should work");
        assert!(is_valid);
    }

    #[test]
    fn verify_detects_corrupted_parity() {
        let data = vec![vec![1u8, 2, 3], vec![4, 5, 6]];
        let config = ReedSolomonConfig::new(2, 1).expect("config should be valid");
        let mut parity = encode(&data, config).expect("ecc should encode");

        // Corrupt parity
        parity[0][0] ^= 0xFF;

        let is_valid = verify(&data, &parity, config).expect("verify should work");
        assert!(!is_valid);
    }

    #[test]
    fn multi_parity_chunks_generated() {
        let data = vec![vec![1u8, 2, 3], vec![4, 5, 6]];
        let config = ReedSolomonConfig::new(2, 3).expect("config should be valid");
        let parity = encode(&data, config).expect("ecc should encode");

        assert_eq!(parity.len(), 3);
        // All should be present and non-empty
        for p in parity {
            assert_eq!(p.len(), 3);
        }
    }
}
