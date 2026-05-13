# QRD Format Specification

This document describes the Phase 1 binary contract for QRD.

## Terminology

The words MUST, MUST NOT, SHOULD, and MAY are used as normative guidance for the Phase 1 contract.

## Normative Scope

- File header is 32 bytes and uses canonical little-endian fields.
- Magic bytes MUST be `0x51 0x52 0x44 0x00`.
- Reserved header bytes MUST be zero.
- Footer length MUST be stored as a trailing `U32LE` value.
- Schema serialization MUST be deterministic.
- Writer version is an implementation string, not a format version.

## File Header

| Offset | Length | Field | Rule |
|---|---:|---|---|
| 0 | 4 | MAGIC | MUST be `QRD\0` |
| 4 | 2 | FORMAT_MAJOR | U16LE |
| 6 | 2 | FORMAT_MINOR | U16LE |
| 8 | 8 | SCHEMA_ID | Truncated schema fingerprint |
| 16 | 2 | FLAGS | Bitfield |
| 18 | 2 | RESERVED | MUST be `0x0000` |
| 20 | 12 | WRITER_VERSION | UTF-8, null-padded |

## Footer Contract

- Footer bytes are serialized before the trailing footer-length trailer.
- Footer checksum is included in the scaffold to detect accidental corruption.
- Footer parsing MUST fail if the trailer length does not match the actual footer bytes.

## Implemented Surface

- Header parsing and serialization.
- Footer parsing and serialization.
- Schema serialization and fingerprinting.
- Row group and column chunk scaffolds.
- Encoding identifiers `0x00` through `0x06`.

## Notes

This spec is intentionally aligned to the current Rust core scaffold and is expected to evolve with explicit version bumps when the binary contract changes.