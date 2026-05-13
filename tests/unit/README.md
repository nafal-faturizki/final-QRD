# Unit Tests

Phase 1 unit tests should target small, deterministic contracts:

- header parsing
- footer parsing
- schema serialization
- encoding roundtrips
- CRC32 verification

Run from the workspace root:

```bash
cargo test -q -p qrd-core
```

Target the core package directly for fast regression feedback.
