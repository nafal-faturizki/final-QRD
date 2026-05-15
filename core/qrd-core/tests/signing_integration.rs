// Ed25519 Schema Signing Integration Tests
// Tests for Workstream 4: Ed25519 Schema Signing

#[cfg(test)]
mod tests {
    use qrd_core::reader::FileReader;
    use qrd_core::schema::{FieldKind, SchemaBuilder};
    use qrd_core::signing::{SchemaSignature, SigningKeypair};
    use qrd_core::writer::StreamingWriter;

    // Test 1: Sign schema with keypair
    #[test]
    fn test_sign_schema_with_keypair() {
        let keypair = SigningKeypair::generate().expect("should generate keypair");
        let schema = SchemaBuilder::new()
            .add_field("col1", FieldKind::Int32, false)
            .add_field("col2", FieldKind::Float64, false)
            .build()
            .expect("should build schema");

        let schema_bytes = schema.serialize().expect("should serialize schema");
        let sig =
            SchemaSignature::from_keypair(&keypair, &schema_bytes).expect("should sign schema");

        // Verify signature
        sig.verify(&schema_bytes).expect("signature should verify");
    }

    // Test 2: Verify signature with different keypair should fail
    #[test]
    fn test_signature_verification_fails_with_different_key() {
        let kp1 = SigningKeypair::generate().expect("should generate kp1");
        let kp2 = SigningKeypair::generate().expect("should generate kp2");

        let schema = SchemaBuilder::new()
            .add_field("data", FieldKind::Utf8, false)
            .build()
            .expect("should build schema");

        let schema_bytes = schema.serialize().expect("should serialize");

        // Sign with kp1
        let sig = SchemaSignature::from_keypair(&kp1, &schema_bytes).expect("should sign");

        // Try to verify with kp2's public key (modify signature's public key)
        let mut sig_with_wrong_key = sig.clone();
        sig_with_wrong_key.public_key = kp2.public_key_bytes();

        let result = sig_with_wrong_key.verify(&schema_bytes);
        assert!(result.is_err(), "should fail with different key");
    }

    // Test 3: Tampered schema should fail verification
    #[test]
    fn test_tampered_schema_fails_verification() {
        let keypair = SigningKeypair::generate().expect("should generate keypair");
        let schema = SchemaBuilder::new()
            .add_field("original", FieldKind::Int32, false)
            .build()
            .expect("should build schema");

        let schema_bytes = schema.serialize().expect("should serialize");
        let sig = SchemaSignature::from_keypair(&keypair, &schema_bytes).expect("should sign");

        // Tamper with schema
        let mut tampered = schema_bytes.clone();
        if let Some(byte) = tampered.last_mut() {
            *byte = byte.wrapping_add(1);
        }

        let result = sig.verify(&tampered);
        assert!(result.is_err(), "should fail with tampered schema");
    }

    // Test 4: Deterministic signatures with same seed
    #[test]
    fn test_deterministic_signatures_with_same_seed() {
        let seed = [42u8; 32];
        let kp1 = SigningKeypair::from_seed(&seed).expect("should create kp1");
        let kp2 = SigningKeypair::from_seed(&seed).expect("should create kp2");

        let schema = SchemaBuilder::new()
            .add_field("test", FieldKind::Int32, false)
            .build()
            .expect("should build");

        let schema_bytes = schema.serialize().expect("should serialize");

        let sig1 =
            SchemaSignature::from_keypair(&kp1, &schema_bytes).expect("should sign with kp1");
        let sig2 =
            SchemaSignature::from_keypair(&kp2, &schema_bytes).expect("should sign with kp2");

        // Same seed → same signature
        assert_eq!(sig1.signature, sig2.signature);
        assert_eq!(sig1.public_key, sig2.public_key);
    }

    // Test 5: Signature serialization round-trip
    #[test]
    fn test_signature_serialization_roundtrip() {
        let keypair = SigningKeypair::generate().expect("should generate");
        let schema = SchemaBuilder::new()
            .add_field("data", FieldKind::Utf8, false)
            .build()
            .expect("should build");

        let schema_bytes = schema.serialize().expect("should serialize");
        let sig = SchemaSignature::from_keypair(&keypair, &schema_bytes).expect("should sign");

        // Serialize and deserialize
        let bytes = sig.to_bytes();
        assert_eq!(bytes.len(), 97);

        let deserialized = SchemaSignature::from_bytes(&bytes).expect("should deserialize");

        // Verify deserialized signature
        deserialized
            .verify(&schema_bytes)
            .expect("should verify deserialized");

        // Check fields
        assert_eq!(deserialized.algorithm, 0x01);
        assert_eq!(deserialized.signature, sig.signature);
        assert_eq!(deserialized.public_key, sig.public_key);
    }

    // Test 6: Multiple schema signing
    #[test]
    fn test_multiple_schema_signing() {
        let keypair = SigningKeypair::generate().expect("should generate");

        let schema1 = SchemaBuilder::new()
            .add_field("field1", FieldKind::Int32, false)
            .build()
            .expect("should build schema1");

        let schema2 = SchemaBuilder::new()
            .add_field("field2", FieldKind::Utf8, false)
            .add_field("field3", FieldKind::Float64, false)
            .build()
            .expect("should build schema2");

        let bytes1 = schema1.serialize().expect("should serialize");
        let bytes2 = schema2.serialize().expect("should serialize");

        let sig1 = SchemaSignature::from_keypair(&keypair, &bytes1).expect("should sign schema1");
        let sig2 = SchemaSignature::from_keypair(&keypair, &bytes2).expect("should sign schema2");

        // Signatures should be different for different schemas
        assert_ne!(sig1.signature, sig2.signature);

        // Both should verify correctly
        sig1.verify(&bytes1).expect("sig1 should verify");
        sig2.verify(&bytes2).expect("sig2 should verify");
    }

    // Test 7: File writing and reading with signature support
    #[test]
    fn test_file_write_read_with_signature_support() {
        let schema = SchemaBuilder::new()
            .add_field("id", FieldKind::Int32, false)
            .add_field("value", FieldKind::Float64, false)
            .build()
            .expect("should build schema");

        let mut writer = StreamingWriter::new(schema.clone());

        // 2 fields: each row has 2 bytes
        let rows: Vec<Vec<u8>> = vec![vec![1, 2], vec![3, 4]];

        writer.write_row_group(&rows).expect("should write rows");
        let file_data = writer.finish().expect("should finish");

        // Verify file can be read
        let _reader = FileReader::open(&file_data).expect("should open file");
    }

    // Test 8: Public key extraction and verification
    #[test]
    fn test_public_key_extraction() {
        let keypair = SigningKeypair::generate().expect("should generate");
        let pubkey = keypair.public_key_bytes();

        assert_eq!(pubkey.len(), 32);

        let _schema = SchemaBuilder::new()
            .add_field("test", FieldKind::Int32, false)
            .build()
            .expect("should build");

        let message = b"test message";

        let sig = keypair.sign(message).expect("should sign");
        assert_eq!(sig.len(), 64);

        // Verify with public key
        let vkey = qrd_core::signing::VerificationKey::from_bytes(&pubkey)
            .expect("should create verification key");
        vkey.verify(message, &sig).expect("should verify message");
    }

    // Test 9: Signature bytes format validation
    #[test]
    fn test_signature_bytes_format_validation() {
        // Too short
        let short_bytes = vec![0x01; 96];
        let result = SchemaSignature::from_bytes(&short_bytes);
        assert!(result.is_err());

        // Too long
        let long_bytes = vec![0x01; 98];
        let result = SchemaSignature::from_bytes(&long_bytes);
        assert!(result.is_err());

        // Correct length - should create signature object
        let mut valid_bytes = vec![0x01; 97];
        valid_bytes[0] = 0x01; // Algorithm 0x01

        let result = SchemaSignature::from_bytes(&valid_bytes);
        assert!(result.is_ok());
    }

    // Test 11: Signature with schema containing multiple fields
    #[test]
    fn test_signature_with_complex_schema() {
        let keypair = SigningKeypair::generate().expect("should generate");

        let schema = SchemaBuilder::new()
            .add_field("id", FieldKind::Int32, false)
            .add_field("name", FieldKind::Utf8, false)
            .add_field("value", FieldKind::Float64, false)
            .add_field("active", FieldKind::Int32, false)
            .build()
            .expect("should build");

        let schema_bytes = schema.serialize().expect("should serialize");
        let sig = SchemaSignature::from_keypair(&keypair, &schema_bytes).expect("should sign");

        sig.verify(&schema_bytes)
            .expect("should verify complex schema signature");
    }

    // Test 12: Non-repudiation verification
    #[test]
    fn test_non_repudiation_with_signature() {
        let keypair = SigningKeypair::generate().expect("should generate");
        let schema = SchemaBuilder::new()
            .add_field("claim", FieldKind::Utf8, false)
            .build()
            .expect("should build");

        let schema_bytes = schema.serialize().expect("should serialize");
        let sig = SchemaSignature::from_keypair(&keypair, &schema_bytes).expect("should sign");

        // Signature proves that the holder of keypair signed this specific schema
        // (non-repudiation property)
        let pubkey = sig.public_key;
        sig.verify(&schema_bytes).expect("signature verifies");

        // Signature contains public key for independent verification
        assert_eq!(pubkey, keypair.public_key_bytes());
    }

    // Test 13: Signature stability across runs
    #[test]
    fn test_signature_stability() {
        let seed = [99u8; 32];
        let kp = SigningKeypair::from_seed(&seed).expect("should create");

        let schema = SchemaBuilder::new()
            .add_field("stable", FieldKind::Int32, false)
            .build()
            .expect("should build");

        let schema_bytes = schema.serialize().expect("should serialize");

        let sig1 = SchemaSignature::from_keypair(&kp, &schema_bytes).expect("should sign");
        let sig2 = SchemaSignature::from_keypair(&kp, &schema_bytes).expect("should sign");

        // Signatures should be identical (deterministic)
        assert_eq!(sig1.signature, sig2.signature);
        assert_eq!(sig1.to_bytes(), sig2.to_bytes());
    }

    // Test 14: Large schema signing
    #[test]
    fn test_large_schema_signing() {
        let mut builder = SchemaBuilder::new();

        // Add many fields
        for i in 0..50 {
            builder = builder.add_field(&format!("field_{}", i), FieldKind::Int32, false);
        }

        let schema = builder.build().expect("should build");
        let keypair = SigningKeypair::generate().expect("should generate");

        let schema_bytes = schema.serialize().expect("should serialize");
        let sig = SchemaSignature::from_keypair(&keypair, &schema_bytes)
            .expect("should sign large schema");

        sig.verify(&schema_bytes)
            .expect("should verify large schema signature");
    }

    // Test 15: Cross-key signature rejection
    #[test]
    fn test_cross_key_signature_rejection() {
        let kp1 = SigningKeypair::generate().expect("should generate kp1");
        let kp2 = SigningKeypair::generate().expect("should generate kp2");

        let schema = SchemaBuilder::new()
            .add_field("data", FieldKind::Int32, false)
            .build()
            .expect("should build");

        let schema_bytes = schema.serialize().expect("should serialize");

        // Sign with kp1
        let sig = SchemaSignature::from_keypair(&kp1, &schema_bytes).expect("should sign");

        // Swap public key with kp2's
        let mut wrong_sig = sig.clone();
        wrong_sig.public_key = kp2.public_key_bytes();

        // Should fail verification
        let result = wrong_sig.verify(&schema_bytes);
        assert!(
            result.is_err(),
            "should reject signature with wrong public key"
        );
    }
}
