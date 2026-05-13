# QRD Python SDK

Phase 1 Python bindings use PyO3 to expose the Rust core to Python users.

## Phase 1 Scope

- Write QRD files via the Rust core.
- Read partial columns via the Rust core.
- Inspect schema and footer metadata.
- Verify integrity and footer structure.

## Binding Contract

- The Python layer is a thin wrapper over Rust.
- No independent format implementation is allowed.
- Public APIs should include examples when the package is added.

## Current Status

This directory is a scaffold until the PyO3 package is added.
