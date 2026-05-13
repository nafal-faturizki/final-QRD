use crate::error::{QrdError, Result};

fn xor_chunks(chunks: &[Vec<u8>]) -> Result<Vec<u8>> {
    if chunks.is_empty() {
        return Ok(Vec::new());
    }

    let width = chunks[0].len();
    if chunks.iter().any(|chunk| chunk.len() != width) {
        return Err(QrdError::InvalidSchema("ecc chunks must have uniform width".into()));
    }

    let mut parity = vec![0u8; width];
    for chunk in chunks {
        for (index, byte) in chunk.iter().enumerate() {
            parity[index] ^= *byte;
        }
    }
    Ok(parity)
}

/// Reed-Solomon scaffold configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReedSolomonConfig {
    pub data_chunks: usize,
    pub parity_chunks: usize,
}

/// Encodes parity chunks in the scaffold.
pub fn encode(data: &[Vec<u8>], config: ReedSolomonConfig) -> Result<Vec<Vec<u8>>> {
    if config.parity_chunks == 0 {
        return Ok(Vec::new());
    }

    let parity = xor_chunks(data)?;
    Ok(vec![parity; config.parity_chunks])
}

/// Attempts to recover a missing chunk using parity.
pub fn recover_missing_chunk(data: &[Option<Vec<u8>>], config: ReedSolomonConfig) -> Result<Vec<u8>> {
    if config.parity_chunks == 0 {
        return Err(QrdError::NotImplemented("parity recovery requires parity chunks"));
    }

    let available: Vec<Vec<u8>> = data.iter().filter_map(|chunk| chunk.clone()).collect();
    if available.is_empty() {
        return Err(QrdError::UnexpectedEof);
    }

    let parity = xor_chunks(&available)?;
    Ok(parity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parity_chunk_is_xor_of_input() {
        let data = vec![vec![1u8, 2, 3], vec![4, 5, 6]];
        let parity = encode(&data, ReedSolomonConfig { data_chunks: 2, parity_chunks: 1 })
            .expect("ecc should encode");

        assert_eq!(parity, vec![vec![5, 7, 5]]);
    }

    #[test]
    fn parity_encode_rejects_width_mismatch() {
        let data = vec![vec![1u8, 2], vec![3]];
        assert!(encode(&data, ReedSolomonConfig { data_chunks: 2, parity_chunks: 1 }).is_err());
    }
}
