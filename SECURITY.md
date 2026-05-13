# Security

## Disclosure

Report security issues privately to the maintainers. Do not open a public issue for a suspected vulnerability.

## What to Include

- Affected file or module.
- Short description of the issue.
- Reproduction steps or a minimal proof-of-concept.
- Impact assessment, if known.

## Phase 1 Focus

- Zero-panic parsing on adversarial input.
- Stable header and footer contracts.
- Bounded memory behavior.
- Thin interop layers with no business logic.
- Avoid leaking plaintext through error messages or debug output.

## Safe Reporting Expectations

- Prefer private communication for security-sensitive bugs.
- Avoid sharing secrets or production keys in repros.
- Use synthetic data when demonstrating parser or format issues.

## Current Status

This repository is in scaffold mode for Phase 1, so the current security work is about keeping the contract surfaces narrow and predictable.