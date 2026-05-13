use crate::error::{QrdError, Result};

/// Compression codecs supported by the scaffold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionKind {
    Zstd,
    Lz4,
    Adaptive,
}

/// Chooses a compression codec using a simple size heuristic.
pub fn choose_compression(payload: &[u8]) -> CompressionKind {
    if payload.len() < 1_024 {
        CompressionKind::Lz4
    } else {
        CompressionKind::Zstd
    }
}

/// Compresses a payload using the specified codec.
///
/// # Returns
/// - For empty payloads: returns empty vector
/// - For successful compression: returns compressed data
/// - For failed compression: returns Err
pub fn compress(payload: &[u8], kind: CompressionKind) -> Result<Vec<u8>> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }

    let codec = match kind {
        CompressionKind::Adaptive => choose_compression(payload),
        other => other,
    };

    match codec {
        CompressionKind::Zstd => compress_zstd(payload),
        CompressionKind::Lz4 => compress_lz4(payload),
        CompressionKind::Adaptive => unreachable!("adaptive should be resolved above"),
    }
}

/// Decompresses a payload using the specified codec.
pub fn decompress(payload: &[u8], kind: CompressionKind) -> Result<Vec<u8>> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }

    match kind {
        CompressionKind::Zstd => decompress_zstd(payload),
        CompressionKind::Lz4 => decompress_lz4(payload),
        CompressionKind::Adaptive => {
            // Try ZSTD first, then LZ4 on failure
            decompress_zstd(payload).or_else(|_| decompress_lz4(payload))
        }
    }
}

/// Compresses payload using Zstandard.
fn compress_zstd(payload: &[u8]) -> Result<Vec<u8>> {
    zstd::encode_all(payload, 3).map_err(|e| {
        QrdError::InvalidSchema(format!("ZSTD compression failed: {}", e))
    })
}

/// Decompresses payload using Zstandard.
fn decompress_zstd(payload: &[u8]) -> Result<Vec<u8>> {
    zstd::decode_all(payload).map_err(|e| {
        QrdError::InvalidSchema(format!("ZSTD decompression failed: {}", e))
    })
}

/// Compresses payload using LZ4.
fn compress_lz4(payload: &[u8]) -> Result<Vec<u8>> {
    lz4::block::compress(payload, Some(lz4::block::CompressionMode::DEFAULT), true)
        .map_err(|e| QrdError::InvalidSchema(format!("LZ4 compression failed: {}", e)))
}

/// Decompresses payload using LZ4.
fn decompress_lz4(payload: &[u8]) -> Result<Vec<u8>> {
    lz4::block::decompress(payload, None)
        .map_err(|e| QrdError::InvalidSchema(format!("LZ4 decompression failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_payload_prefers_lz4() {
        assert_eq!(choose_compression(b"small"), CompressionKind::Lz4);
    }

    #[test]
    fn large_payload_prefers_zstd() {
        let payload = vec![0u8; 1_024];
        assert_eq!(choose_compression(&payload), CompressionKind::Zstd);
    }

    #[test]
    fn empty_payload_returns_empty() {
        assert_eq!(compress(b"", CompressionKind::Zstd).unwrap(), vec![] as Vec<u8>);
        assert_eq!(decompress(b"", CompressionKind::Zstd).unwrap(), vec![] as Vec<u8>);
    }

    #[test]
    fn zstd_roundtrip() {
        let payload = b"hello world this is a test payload for compression";
        let compressed = compress(payload, CompressionKind::Zstd).expect("compression should work");
        assert!(compressed.len() > 0);
        let decompressed = decompress(&compressed, CompressionKind::Zstd)
            .expect("decompression should work");
        assert_eq!(decompressed, payload);
    }

    #[test]
    fn lz4_roundtrip() {
        let payload = b"hello world this is a test payload for compression";
        let compressed = compress(payload, CompressionKind::Lz4).expect("compression should work");
        assert!(compressed.len() > 0);
        let decompressed = decompress(&compressed, CompressionKind::Lz4)
            .expect("decompression should work");
        assert_eq!(decompressed, payload);
    }

    #[test]
    fn adaptive_chooses_correct_codec() {
        let small_payload = b"small";
        let compressed = compress(small_payload, CompressionKind::Adaptive)
            .expect("compression should work");
        let decompressed = decompress(&compressed, CompressionKind::Adaptive)
            .expect("decompression should work");
        assert_eq!(decompressed, small_payload);
    }
}
