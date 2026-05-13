# QRD C/C++ SDK

Phase 1 C/C++ bindings use the stable C FFI layer from `core/qrd-ffi`.

## Phase 1 Scope

- Write QRD files through the Rust core.
- Read partial columns through the Rust core.
- Inspect schema and footer metadata.
- Verify integrity through the Rust parser and footer contract.

## Binding Contract

- The C header is the ABI contract; business logic stays in Rust.
- C and C++ callers must use create/free-style ownership patterns.
- Public headers should be documented with usage examples.

## Current Status

This directory is a scaffold until the C/C++ package is added.
