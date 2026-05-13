# Integration Tests

Phase 1 integration coverage should validate the full scaffold:

- writer -> footer -> reader roundtrip
- CLI inspect and verify flows
- FFI header inspection
- WASM footer inspection

Run the full integration set from the root with:

```bash
cargo test -q --tests
```

Or execute workspace integration targets explicitly:

```bash
cargo test -q -p qrd-core --test integration
```
