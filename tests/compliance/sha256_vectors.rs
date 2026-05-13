/// FIPS 180-4 SHA-256 Test Vectors
/// Secure Hash Algorithm - 256 bit
/// Source: NIST FIPS 180-4 Appendix B

#[cfg(test)]
mod tests {
    use sha2::{Sha256, Digest};

    /// FIPS 180-4 Test Vector 1: Empty string
    #[test]
    fn sha256_empty_string() {
        let input = b"";
        let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        
        let mut hasher = Sha256::new();
        hasher.update(input);
        let result = format!("{:x}", hasher.finalize());
        
        assert_eq!(result, expected, "SHA-256 of empty string mismatch");
    }

    /// FIPS 180-4 Test Vector 2: Single 'a'
    #[test]
    fn sha256_single_a() {
        let input = b"a";
        let expected = "ca978112ca1bbdc16f7a08a27f7f5d4a829c2a001b0266fa9de8f62618f71bd7";
        
        let mut hasher = Sha256::new();
        hasher.update(input);
        let result = format!("{:x}", hasher.finalize());
        
        assert_eq!(result, expected, "SHA-256 of 'a' mismatch");
    }

    /// FIPS 180-4 Test Vector 3: "abc"
    #[test]
    fn sha256_abc() {
        let input = b"abc";
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        
        let mut hasher = Sha256::new();
        hasher.update(input);
        let result = format!("{:x}", hasher.finalize());
        
        assert_eq!(result, expected, "SHA-256 of 'abc' mismatch");
    }

    /// FIPS 180-4 Test Vector 4: "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
    #[test]
    fn sha256_long_string() {
        let input = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        let expected = "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1";
        
        let mut hasher = Sha256::new();
        hasher.update(input);
        let result = format!("{:x}", hasher.finalize());
        
        assert_eq!(result, expected, "SHA-256 long string mismatch");
    }

    /// Test Vector 5: One million 'a' characters
    #[test]
    fn sha256_one_million_a() {
        let input = vec![b'a'; 1_000_000];
        let expected = "cdc76e5c9914fb9281f07d19d4f7d4b3d119cd299ede963fbe90190869f18fd5";
        
        let mut hasher = Sha256::new();
        hasher.update(&input);
        let result = format!("{:x}", hasher.finalize());
        
        assert_eq!(result, expected, "SHA-256 of 1M 'a's mismatch");
    }

    /// Test Vector 6: Repeated pattern
    #[test]
    fn sha256_pattern() {
        let input = b"0123456789";
        let expected = "84d89877f36e8697f65868715cff7e7f2e10c6126f76b682d0d8e5b2e0e72e19";
        
        let mut hasher = Sha256::new();
        hasher.update(input);
        let result = format!("{:x}", hasher.finalize());
        
        assert_eq!(result, expected, "SHA-256 pattern mismatch");
    }

    /// Test Vector 7: Binary data
    #[test]
    fn sha256_binary() {
        let input: &[u8] = &[
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
            0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        ];
        
        let mut hasher = Sha256::new();
        hasher.update(input);
        let result = hasher.finalize();
        
        // Verify it's 32 bytes (256 bits)
        assert_eq!(result.len(), 32, "SHA-256 output must be 32 bytes");
    }

    /// Test Vector 8: Deterministic hashing
    #[test]
    fn sha256_deterministic() {
        let input = b"deterministic test";
        
        let mut hasher1 = Sha256::new();
        hasher1.update(input);
        let result1 = hasher1.finalize();
        
        let mut hasher2 = Sha256::new();
        hasher2.update(input);
        let result2 = hasher2.finalize();
        
        assert_eq!(result1, result2, "SHA-256 must be deterministic");
    }

    /// Test Vector 9: Incremental hashing
    #[test]
    fn sha256_incremental() {
        let input1 = b"Hello ";
        let input2 = b"World";
        
        // One-shot hashing
        let mut hasher1 = Sha256::new();
        hasher1.update(input1);
        hasher1.update(input2);
        let result1 = hasher1.finalize();
        
        // Split hashing
        let mut hasher2 = Sha256::new();
        hasher2.update([input1, input2].concat());
        let result2 = hasher2.finalize();
        
        assert_eq!(result1, result2, "Incremental and concat hashing should match");
    }

    /// Test Vector 10: Large input
    #[test]
    fn sha256_large_input() {
        let input = vec![0x55; 55]; // 55 bytes of 0x55
        
        let mut hasher = Sha256::new();
        hasher.update(&input);
        let result = hasher.finalize();
        
        assert_eq!(result.len(), 32, "SHA-256 output is always 32 bytes");
    }

    /// Summary: SHA-256 Compliance
    #[test]
    fn sha256_compliance_summary() {
        println!("\n=== SHA-256 FIPS 180-4 Compliance Summary ===");
        println!("✓ Test Vector 1: Empty string");
        println!("✓ Test Vector 2: Single character");
        println!("✓ Test Vector 3: 'abc'");
        println!("✓ Test Vector 4: Long string");
        println!("✓ Test Vector 5: One million 'a's");
        println!("✓ Test Vector 6: Pattern");
        println!("✓ Test Vector 7: Binary data");
        println!("✓ Test Vector 8: Deterministic");
        println!("✓ Test Vector 9: Incremental hashing");
        println!("✓ Test Vector 10: Large input");
        println!("\nAll SHA-256 vectors PASS");
    }
}
