# QRD Cryptography

This document records the cryptographic choices currently implemented in QRD.

## Implemented Primitives

- `AES-256-GCM` for per-column authenticated encryption.
- `HKDF-SHA256` for per-column key derivation from the master key.
- `SHA-256` for schema fingerprinting and related integrity checks.
- `Ed25519` for optional schema signatures.
- `CRC32` for non-adversarial corruption detection.
- 12-byte nonces and 16-byte authentication tags in the chunk envelope.

## Cryptography Crates

| Crate | Declared Version | Role | Status |
|---|---|---|---|
| `aes-gcm` | `0.10` | AES-256-GCM encryption and tag verification | Implemented and covered by AES-GCM test vectors in [tests/compliance/aes_gcm_vectors.rs](../../tests/compliance/aes_gcm_vectors.rs) |
| `hkdf` | `0.12` | HKDF extract-and-expand key derivation | Implemented and covered by RFC 5869 vectors in [tests/compliance/hkdf_vectors.rs](../../tests/compliance/hkdf_vectors.rs) |
| `sha2` | `0.10` | SHA-256 used by HKDF and schema fingerprints | Implemented and covered by SHA-256 vectors in [tests/compliance/sha256_vectors.rs](../../tests/compliance/sha256_vectors.rs) |
| `ed25519-dalek` | `2.1` | Optional Ed25519 schema signatures | Implemented and covered by Ed25519 vectors in [tests/compliance/ed25519_vectors.rs](../../tests/compliance/ed25519_vectors.rs) |
| `rand` | `0.8` | Cryptographic randomness for nonces and key material | Implemented; exercised indirectly through encryption and signing flows |
| `subtle` | `2.5` | Constant-time comparison helpers | Implemented; used for sensitive comparisons |
| `crc32fast` | `1.3` | CRC32 integrity checks | Implemented; covered by parser and corruption tests |

## Validation Coverage

- AES-GCM vectors: [tests/compliance/aes_gcm_vectors.rs](../../tests/compliance/aes_gcm_vectors.rs)
- HKDF vectors: [tests/compliance/hkdf_vectors.rs](../../tests/compliance/hkdf_vectors.rs)
- SHA-256 vectors: [tests/compliance/sha256_vectors.rs](../../tests/compliance/sha256_vectors.rs)
- Ed25519 vectors: [tests/compliance/ed25519_vectors.rs](../../tests/compliance/ed25519_vectors.rs)
- Cross-suite execution: [tests/compliance_tests.rs](../../tests/compliance_tests.rs)

## Current Status

- QRD uses production cryptographic primitives in the core crate.
- The implementation is validated by unit, integration, and compliance vector tests.
- Security-sensitive code paths avoid exposing raw secrets in normal error reporting.