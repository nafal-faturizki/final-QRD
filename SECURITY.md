# Security

## Disclosure

Report security issues privately to the maintainers. Do not open a public issue for a suspected vulnerability.

## What to Include

- Affected file or module.
- Short description of the issue.
- Reproduction steps or a minimal proof-of-concept.
- Impact assessment, if known.

## Phase 1 Focus

- Zero-panic parsing on adversarial input.
- Stable header and footer contracts.
- Bounded memory behavior.
- Thin interop layers with no business logic.
- Avoid leaking plaintext through error messages or debug output.

## Cryptography Status

QRD already uses production cryptographic primitives in the core implementation. The current stack is:

- `aes-gcm` `0.10` for AES-256-GCM encryption and authentication.
- `hkdf` `0.12` plus `sha2` `0.10` for HKDF-SHA256 key derivation.
- `ed25519-dalek` `2.1` for optional schema signatures.
- `rand` `0.8` for nonce generation.
- `subtle` `2.5` for constant-time comparisons.
- `crc32fast` `1.3` for CRC32 integrity checks.

Validation uses the repository test vectors in:

- [tests/compliance/aes_gcm_vectors.rs](tests/compliance/aes_gcm_vectors.rs)
- [tests/compliance/hkdf_vectors.rs](tests/compliance/hkdf_vectors.rs)
- [tests/compliance/sha256_vectors.rs](tests/compliance/sha256_vectors.rs)
- [tests/compliance/ed25519_vectors.rs](tests/compliance/ed25519_vectors.rs)

## Safe Reporting Expectations

- Prefer private communication for security-sensitive bugs.
- Avoid sharing secrets or production keys in repros.
- Use synthetic data when demonstrating parser or format issues.

## Current Status

This repository is in active implementation and validation for Phase 1. Security work focuses on keeping contract surfaces narrow, predictable, and resistant to malformed input.