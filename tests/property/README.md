# Property Tests

Phase 1 property tests should cover reversible behavior:

- `decode(encode(x)) == x`
- schema fingerprint stability
- row group serialization roundtrips
- footer length parsing at the end of the file image
