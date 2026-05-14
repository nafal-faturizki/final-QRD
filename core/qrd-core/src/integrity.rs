/// Computes CRC32 using the IEEE polynomial.
pub fn crc32(bytes: &[u8]) -> u32 {
    crc32fast::hash(bytes)
}

/// Verifies a CRC32 value against the provided payload.
pub fn verify_crc32(bytes: &[u8], expected: u32) -> bool {
    crc32(bytes) == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_known_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn verify_crc32_accepts_matching_checksum() {
        let payload = b"qrd";
        let checksum = crc32(payload);

        assert!(verify_crc32(payload, checksum));
        assert!(!verify_crc32(payload, checksum ^ 1));
    }
}
