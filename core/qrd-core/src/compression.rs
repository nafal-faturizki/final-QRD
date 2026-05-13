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

/// Compresses a payload in the scaffold.
pub fn compress(payload: &[u8], _kind: CompressionKind) -> Result<Vec<u8>> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }
    Err(QrdError::NotImplemented("compression codec"))
}

/// Decompresses a payload in the scaffold.
pub fn decompress(payload: &[u8], _kind: CompressionKind) -> Result<Vec<u8>> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }
    Err(QrdError::NotImplemented("compression codec"))
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
}
