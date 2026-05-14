# Fuzzing

QRD fuzzing currently covers the most security-sensitive parse and crypto entry points.

## Targets

- `parse_header` — QRD header parser
- `parse_footer` — QRD footer parser
- `decompress` — compression decompressor path
- `decrypt_payload` — AES-256-GCM payload decryption path

## Seed Corpus

Minimal seeds live under `corpus/` and are derived from a valid QRD sample file:

- `corpus/sample.qrd`
- `corpus/parse_header/seed.bin`
- `corpus/parse_footer/seed.bin`
- `corpus/decompress/seed.bin`
- `corpus/decrypt_payload/seed.bin`

## Running Locally

Run one target with:

```bash
cargo fuzz run parse_header
```

## CI

The nightly GitHub Actions job runs all four targets for 30 minutes total and uploads any crash corpus found under `fuzz/artifacts/`.
