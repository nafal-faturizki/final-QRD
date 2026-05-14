# QRD CLI

This crate provides production CLI support for QRD tooling.

## Commands

- `qrd-inspect <file>`
- `qrd-inspect --schema <file>`
- `qrd-inspect --footer <file>`
- `qrd-inspect --json <file>`
- `qrd-verify <file>`
- `qrd-convert <mode> <input> <output>`
- `qrd-keygen <mode>`

## Supported conversion modes

- `csv` / `json` / `parquet` - convert a source file into a QRD container
- `qrd-to-csv` - extract QRD payload back to CSV-like bytes
- `qrd-to-parquet` - extract QRD payload back to Parquet-like bytes

## Features

- Inspect QRD file headers, footers, and schema metadata
- Export inspection output as JSON
- Verify QRD integrity and schema signatures
- Convert files into QRD container format and back
- Generate master keys and Ed25519 signing keypairs
