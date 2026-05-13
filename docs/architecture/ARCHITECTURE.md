# QRD Architecture

QRD uses a single Rust core as the source of truth for format behavior.

## Layers

- Rust core engine: schema, parser, writer, reader, encoding, compression, encryption, ECC, integrity.
- C FFI layer: thin ABI bridge for native consumers.
- WASM layer: browser and Node.js access through a shared core.
- SDK wrappers: Python, TypeScript, Go, Java, and C/C++ thin clients.

## Data Flow

1. Input rows enter the writer.
2. Rows are transposed into column-oriented chunks.
3. Columns are encoded and compressed.
4. Encrypted columns are wrapped in a nonce/auth-tag/ciphertext envelope.
5. CRC32 protects each chunk and footer.
6. The footer is written last, followed by the footer-length trailer.

## Phase 1 Goals

- Keep all format logic in Rust.
- Preserve deterministic binary output where possible.
- Keep write/read paths bounded in memory.
- Make the interop layers thin enough to reason about at the boundary.

## Maintenance Rule

Any change to the binary contract should first update the format spec and then the Rust core helpers that implement it.