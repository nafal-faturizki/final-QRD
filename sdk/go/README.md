# QRD Go SDK

Phase 1 Go bindings use CGO to call the Rust core through `core/qrd-ffi`.

## Phase 1 Scope

- Write QRD files through the Rust core writer API.
- Read partial columns through the Rust core reader API.
- Inspect schema and footer metadata without reimplementing the format.
- Verify integrity using the Rust core checksum and footer parser.

## Binding Contract

- The Go layer stays thin and delegates all format logic to Rust.
- No independent encoding, compression, or encryption logic lives here.
- All public entry points should have examples once the actual package exists.

## Current Status

This directory is a scaffold until the CGO package is added.
