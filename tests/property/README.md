# Property Tests

Phase 1 property tests should cover reversible behavior:

- `decode(encode(x)) == x`
- schema fingerprint stability
- row group serialization roundtrips
- footer length parsing at the end of the file image

Run the property suite with:

```bash
cargo test -q -p qrd-core --test property
```

Property tests are especially important for codec and parser invariants.
