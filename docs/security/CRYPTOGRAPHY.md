# QRD Cryptography

This document records the cryptographic choices used in the Phase 1 scaffold.

## Primitives

- CRC32 for non-adversarial corruption detection.
- AES-256-GCM envelope shape for encrypted chunks.
- HKDF-SHA256 derived column keys.
- 12-byte nonces and 16-byte auth tags in the chunk envelope.

## Current Status

- The crate currently defines data structures and envelope packing.
- Production cryptographic primitives are still to be wired into the scaffold.