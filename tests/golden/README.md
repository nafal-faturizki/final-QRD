# Golden Tests

Phase 1 golden vectors should validate deterministic output for:

- file header serialization
- schema serialization
- footer serialization
- row group serialization

Run golden regression tests from the workspace root:

```bash
cargo test -q -p qrd-core --test golden
```

Golden tests must remain stable across CI and release branches.
