# Contributing

## Scope

QRD Phase 1 contributions should focus on the Rust core, the thin interop layers, and the supporting documentation/tests that keep the binary contract stable.

## Working Rules

- Keep format logic in the Rust core.
- Do not reimplement encoding, compression, or encryption in SDK wrappers.
- Prefer small, testable changes.
- Update docs when binary contracts change.
- Keep unsafe code documented with a `SAFETY:` explanation when it is unavoidable.

## Recommended Workflow

1. Start from the relevant core module or boundary layer.
2. Add or update the smallest test that proves the expected behavior.
3. Make the implementation change.
4. Update the contract docs if a binary rule changed.
5. Verify locally with the narrowest available check first.

## Suggested Checks

- `cargo test --workspace`
- `cargo bench --package qrd-core`
- CLI smoke checks for `qrd-inspect` and `qrd-verify`

## Design Notes

- Use the Rust core as the single source of truth.
- Keep FFI and WASM layers thin.
- Preserve deterministic schema, header, footer, and file-image serialization.
- Favor explicit parse/serialize helpers over ad hoc byte manipulation.