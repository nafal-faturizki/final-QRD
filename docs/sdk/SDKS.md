# QRD SDKs

This document summarizes the Phase 1 SDK scaffolds.

## Shared Rules

- All SDKs are thin wrappers over the Rust core.
- No SDK reimplements encoding, compression, or encryption logic.
- Public APIs should include examples.
- File inspection and integrity verification should flow through the shared core contract.

## SDK Status

- Rust: core implementation and source of truth.
- Python: PyO3 scaffold.
- TypeScript: WASM scaffold.
- Go: CGO scaffold.
- Java: JNI scaffold.
- C/C++: C FFI scaffold.
