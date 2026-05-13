# QRD TypeScript SDK

Phase 1 TypeScript bindings use the WASM layer for browser and Node.js usage.

## Phase 1 Scope

- Initialize WASM before any file operations.
- Write QRD files through the Rust core compiled to WASM.
- Read partial columns and inspect footer metadata.
- Verify schema and file integrity through the core engine.

## Binding Contract

- The TypeScript layer remains a thin consumer of the WASM module.
- No separate encoding or encryption logic is implemented in JS/TS.
- Public APIs should be documented with examples once code is added.

## Current Status

This directory is a scaffold until the package and build setup are added.
