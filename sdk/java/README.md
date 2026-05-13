# QRD Java SDK

Phase 1 Java bindings use JNI to call the Rust core through `core/qrd-ffi`.

## Phase 1 Scope

- Write QRD files using Rust core services.
- Read partial columns using Rust core services.
- Inspect schema and footer metadata.
- Verify integrity and parse QRD headers/footers.

## Binding Contract

- Java remains a thin wrapper and does not reimplement the format.
- JNI is only the bridge; all business logic stays in Rust.
- Public APIs should be documented with examples when the package is created.

## Current Status

This directory is a scaffold until JNI sources are added.
