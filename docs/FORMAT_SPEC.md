# QRD Format Specification (RFC-Style)

**Document Status**: RFC Phase 2  
**Format Version**: 1.0  
**Last Updated**: May 14, 2026  

---

## 1. Introduction and Scope

### 1.1 Purpose

This document specifies the QRD (Quarantine Resilient Data) binary file format for storing structured, columnar data with built-in compression, encryption, and error correction. QRD is designed for production use in regulated industries (HIPAA, SOC 2, FIPS 140-3) requiring strong data durability and integrity guarantees.

### 1.2 Target Audience

- Format implementers building QRD readers/writers in any language
- Data preservation specialists archiving columnar formats
- Compliance engineers validating cryptographic implementations
- Security auditors reviewing format integrity mechanisms

### 1.3 Normative References

- [RFC 2119](https://tools.ietf.org/html/rfc2119) — Key words for use in RFCs
- [FIPS 180-4](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf) — SHA-256
- [FIPS 197](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.197.pdf) — AES
- [NIST SP 800-38D](https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication800-38d.pdf) — GCM
- [RFC 5869](https://tools.ietf.org/html/rfc5869) — HKDF
- [RFC 8032](https://tools.ietf.org/html/rfc8032) — EdDSA and Ed25519

---

## 2. Terminology and Key Definitions

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** in this document are to be interpreted as described in RFC 2119.

### 2.1 Data Types

| Term | Byte Size | Definition |
|------|-----------|-----------|
| **U8** | 1 | Unsigned 8-bit integer |
| **U16LE** | 2 | Unsigned 16-bit little-endian integer |
| **U32LE** | 4 | Unsigned 32-bit little-endian integer |
| **U64LE** | 8 | Unsigned 64-bit little-endian integer |
| **BYTES(n)** | n | Byte sequence of fixed length n |
| **UTF-8-STR(max_n)** | ≤n | UTF-8 encoded string, null-padded to max_n bytes |

### 2.2 Cryptographic Terms

- **IND-CPA**: Indistinguishability under Chosen Plaintext Attack (encryption security notion)
- **AEAD**: Authenticated Encryption with Associated Data
- **Nonce**: Number-used-once; 96-bit random value for GCM mode
- **Master Key**: User-provided 256-bit encryption key
- **Derived Key**: Per-column 256-bit key derived via HKDF from master key
- **Auth Tag**: 128-bit authentication tag output by AES-256-GCM

### 2.3 File Structure Terms

- **Header**: Fixed 32-byte file prefix containing magic, format version, schema ID, flags
- **Row Group**: Logical collection of rows (1M rows typical)
- **Column Chunk**: Encoded and compressed column data for one row group
- **Footer**: Metadata footer containing schema and row count
- **Checksum**: CRC-32 of preceding bytes for integrity detection

---

## 3. File Structure Overview

### 3.1 Logical Layout

```
┌──────────────────────────────────────────────────────┐
│ HEADER (32 bytes)                                    │
│ • Magic: 0x51 0x52 0x44 0x00                        │
│ • Format version: (1, 0)                             │
│ • Schema fingerprint: SHA-256 truncated to 8B       │
│ • Flags: bitfield for features (encryption, etc)   │
├──────────────────────────────────────────────────────┤
│ ROW GROUPS (variable length)                        │
│ • Row Group 0: [ColumnChunk_0...ColumnChunk_N]     │
│ • Row Group 1: [ColumnChunk_0...ColumnChunk_N]     │
│ • ...                                               │
├──────────────────────────────────────────────────────┤
│ FOOTER (variable length)                            │
│ • Version: U8 = 1                                   │
│ • Schema length: U32LE                              │
│ • Serialized schema: BYTES(schema_len)              │
│ • Row group count: U32LE                            │
│ • CRC32 checksum: U32LE                             │
├──────────────────────────────────────────────────────┤
│ FOOTER LENGTH TRAILER (4 bytes)                     │
│ • Footer byte count (excluding this trailer): U32LE │
└──────────────────────────────────────────────────────┘
```

### 3.2 Byte Order Guarantee

All multi-byte numeric values **MUST** use little-endian byte order (LSB first). This applies to all U16LE, U32LE, U64LE fields throughout the format.

---

## 4. File Header Specification

### 4.1 Header Layout (32 bytes total)

| Offset | Length | Field | Type | Constraints | Purpose |
|--------|--------|-------|------|-------------|---------|
| 0 | 4 | MAGIC | BYTES(4) | MUST be `[0x51, 0x52, 0x44, 0x00]` | File type magic identifier |
| 4 | 2 | FORMAT_MAJOR | U16LE | MUST be 1 | Major format version |
| 6 | 2 | FORMAT_MINOR | U16LE | Currently 0 | Minor format version |
| 8 | 8 | SCHEMA_ID | BYTES(8) | First 8 bytes of SHA-256(schema) | Schema fingerprint for quick ID |
| 16 | 2 | FLAGS | U16LE | See section 4.2 | Feature flags bitfield |
| 18 | 2 | RESERVED | U16LE | MUST be 0x0000 | Reserved for future use |
| 20 | 12 | WRITER_VERSION | UTF-8-STR(12) | Null-padded | Implementation identifier |

### 4.2 FLAGS Bitfield

| Bit | Value | Flag Name | Meaning |
|-----|-------|-----------|---------|
| 0 | 0x0001 | ENCRYPTED | Set if columns are encrypted with AES-256-GCM |
| 1 | 0x0002 | SCHEMA_SIGNED | Set if schema is signed with Ed25519 |
| 2-15 | Reserved | — | Reserved for future extensions |

### 4.3 Header Parsing Algorithm

1. Read first 32 bytes from file
2. Verify MAGIC bytes equal `[0x51, 0x52, 0x44, 0x00]`; if not, **reject as invalid QRD**
3. Parse FORMAT_MAJOR and FORMAT_MINOR; if MAJOR ≠ 1, **reject as unsupported version**
4. Extract SCHEMA_ID for optional schema validation
5. Parse FLAGS to determine feature support
6. Extract WRITER_VERSION for informational purposes only

### 4.4 Header Creation Algorithm

1. Initialize 32-byte array with zeros
2. Write MAGIC bytes at offset 0-3
3. Write FORMAT_MAJOR (1) as U16LE at offset 4-5
4. Write FORMAT_MINOR (0) as U16LE at offset 6-7
5. Write SCHEMA_ID (first 8 bytes of schema SHA-256) at offset 8-15
6. Write FLAGS bitfield as U16LE at offset 16-17
7. Write 0x0000 at offset 18-19 (RESERVED)
8. Write WRITER_VERSION (null-padded UTF-8) at offset 20-31

---

## 5. Row Group Format

### 5.1 Row Group Structure

Each row group contains encoded and optionally compressed/encrypted column chunks:

```
Row Group:
├─ Column 0 Chunk
│  ├─ Encoding Type: U8
│  ├─ Compressed Length: U32LE
│  ├─ Uncompressed Length: U32LE
│  ├─ [If encrypted: Nonce (12B) + Ciphertext + Auth Tag (16B)]
│  └─ [If not encrypted: Compression codec + data]
├─ Column 1 Chunk
│  └─ [Same structure as Column 0]
└─ ... (one chunk per column)
```

### 5.2 Column Chunk Encoding

**Before compression/encryption:**

| Offset | Length | Field | Type | Purpose |
|--------|--------|-------|------|---------|
| 0 | 1 | ENCODING_ID | U8 | Encoding algorithm used (0x00-0x06) |
| 1 | 4 | UNCOMPRESSED_LEN | U32LE | Byte length of encoded data (before compression) |
| 5+ | Variable | ENCODED_DATA | BYTES | Output of encoding algorithm |

**After compression (if applicable):**

Encoded data is compressed using ZSTD or LZ4 based on COMPRESSION_CODEC selector.

**After encryption (if FLAGS.ENCRYPTED):**

```
[NONCE (12 bytes)] + 
[CIPHERTEXT + AUTH_TAG (result of AES-256-GCM)]
```

---

## 6. Footer Specification

### 6.1 Footer Structure

The footer is written **before** the trailing 4-byte length field and **must be parseable independently** by seeking to the last 4 bytes, reading the footer length, and backtracking.

### 6.2 Footer Byte Layout

| Offset | Length | Field | Type | Constraints | Purpose |
|--------|--------|-------|------|-------------|---------|
| 0 | 1 | VERSION | U8 | MUST be 1 | Footer structure version |
| 1 | 4 | SCHEMA_LEN | U32LE | ≥ 4 bytes | Byte length of serialized schema |
| 5 | SCHEMA_LEN | SCHEMA_DATA | BYTES | Deterministic serialization | Complete schema metadata |
| 5+SL | 4 | ROW_GROUP_COUNT | U32LE | ≥ 1 | Total number of row groups |
| 9+SL | 4 | CRC32 | U32LE | Checksum of all above | Footer integrity verification |

### 6.3 Schema Serialization (Deterministic)

The schema MUST be serialized in a deterministic canonical form:

1. Column count as U32LE
2. For each column (in declaration order):
   - Column name length as U16LE
   - Column name as UTF-8 bytes
   - Column data type as U8 (0=I64, 1=F64, 2=UTF8, etc.)
3. No padding; stream is byte-aligned

### 6.4 Footer Parsing Algorithm (7-Step Process)

**Step 1:** Read last 4 bytes from file as U32LE → `footer_length`

**Step 2:** Verify `footer_length` is reasonable (4 ≤ footer_length ≤ file_size - 36)

**Step 3:** Calculate footer start offset: `footer_start = file_size - 4 - footer_length`

**Step 4:** Seek to `footer_start` and read `footer_length` bytes into `footer_data`

**Step 5:** Extract first byte as `version`; if version ≠ 1, **reject**

**Step 6:** Calculate `expected_crc32 = crc32(footer_data[0 : footer_length - 4])`

**Step 7:** Extract last 4 bytes of footer as actual CRC32; if mismatch, **reject with checksum error**

### 6.5 Footer Creation Algorithm

1. Serialize schema deterministically to `schema_bytes`
2. Create footer body:
   - Write VERSION = 1 as U8
   - Write schema length as U32LE
   - Write schema_bytes
   - Write row_group_count as U32LE
3. Calculate `checksum = crc32(footer_body_without_checksum)`
4. Write checksum as U32LE
5. Append footer_length (length of entire footer including checksum) as U32LE

---

## 7. Encoding Algorithms

QRD supports 7 encoding algorithms. Each column chunk MUST declare its encoding via the ENCODING_ID field.

### 7.1 Plain (0x00)

**Definition**: No encoding; data passes through unchanged.

**Serialization**: Input bytes → Output bytes (identity transformation)

**Deserialization**: Output bytes → Input bytes (identity transformation)

**Roundtrip Guarantee**: Binary identical to input.

### 7.2 Run-Length Encoding (0x01)

**Definition**: Compress runs of identical bytes.

**Serialization Algorithm**:
```
cursor ← 0
while cursor < input.length:
    value ← input[cursor]
    run_length ← count of consecutive bytes == value
    output ← run_length (U16LE) + value (U8)
    cursor ← cursor + run_length
```

**Deserialization Algorithm**:
```
cursor ← 0
output ← empty
while cursor < input.length:
    run_length ← input[cursor:cursor+2] (U16LE)
    value ← input[cursor+2] (U8)
    output ← output + [value] * run_length
    cursor ← cursor + 3
```

**Efficiency**: Excellent for data with long runs (e.g., boolean columns, many zeros).

### 7.3 Bit Packing (0x02)

**Definition**: Pack multiple small integers into fewer bytes.

**Bit Width Selection**: Automatically determined from max value in column.

**Algorithm**: Each integer packed into minimum bits required, then aligned.

### 7.4 Delta Binary (0x03)

**Definition**: Store differences between consecutive values rather than absolute values.

**Serialization Algorithm**:
```
Assumes input is sorted integers. First value stored absolute (8 bytes).
Subsequent values: delta = current - previous, then zigzag-encode.
```

**Efficiency**: Excellent for sorted or nearly-sorted numeric data.

### 7.5 Delta Byte Array (0x04)

**Definition**: Store differences for variable-length byte sequences.

**Algorithm**: First array stored verbatim; subsequent arrays store prefix delta.

**Efficiency**: Good for lexicographically sorted string columns.

### 7.6 Byte Stream Split (0x05)

**Definition**: Separate multi-byte values into per-byte streams.

**Algorithm**: 
- For 8-byte values: separate into 8 streams (byte 0 of all values, byte 1 of all values, etc.)
- Compress each stream independently

**Efficiency**: Excellent for floating-point or hash data with byte-level patterns.

### 7.7 Dictionary Run-Length (0x06)

**Definition**: Build dictionary of unique values; encode runs as dictionary indices.

**Serialization**:
1. Extract unique values; sort deterministically
2. Write dictionary size as U32LE
3. Write each dictionary entry as U16LE length + UTF-8 bytes
4. Encode each value as its dictionary index (U16LE) + run count (U32LE)

**Efficiency**: Excellent for low-cardinality columns (e.g., status fields, gender, country).

---

## 8. Compression Codecs

### 8.1 Compression Framework

After encoding, column chunks **MAY** be compressed. The compression codec is indicated by a selector in the file metadata.

### 8.2 Supported Codecs

| Codec | Identifier | Standard | Zstandard Level | Use Case |
|-------|------------|----------|-----------------|----------|
| ZSTD | 0x01 | ISO/IEC 24824-1 | Level 3 | Default; balanced speed/ratio |
| LZ4 | 0x02 | LZ4 spec | Fast | Real-time or streaming writes |
| None | 0x00 | — | — | Store uncompressed (for incompressible data) |

### 8.3 ZSTD Compression (Recommended)

**Algorithm**: Zstandard dictionary compression with preset 3

**Configuration**:
- Window size: 32 MB (default)
- Checksum: Enabled
- Dictionary: None (adaptive)

**Decompression**: Must handle any valid ZSTD frame

**Performance**: ~100 MB/s encoding, ~400 MB/s decoding on modern CPUs

### 8.4 LZ4 Compression

**Algorithm**: LZ4 fast (non-HC) mode for low-latency scenarios

**Configuration**:
- Block size: 64 KB
- Checksum: Enabled

**Decompression**: Must handle block-by-block decompression

**Performance**: ~1 GB/s encoding, ~2 GB/s decoding (fastest)

---

## 9. Encryption Specification

### 9.1 Encryption Framework

If FLAGS.ENCRYPTED is set, **all column chunks** MUST be encrypted with AES-256-GCM using a per-column derived key.

### 9.2 Key Derivation (HKDF-SHA256)

**Master Key**: User-provided 256-bit value (application responsibility)

**Per-Column Key Derivation**:

```
salt ← first 8 bytes of SHA-256(schema)
info ← "qrd-encryption" || column_name
derived_key ← HKDF-Expand(
    prk = HMAC-SHA256(salt, master_key),
    info = info,
    L = 32
)
```

**Properties**:
- Deterministic: Same column always derives same key
- Domain-separated: Different column names produce different keys
- Non-invertible: Knowing derived_key does not reveal master_key

### 9.3 AES-256-GCM Encryption

**Algorithm**: NIST SP 800-38D Galois/Counter Mode with 256-bit key

**Nonce Generation**:
- 96-bit random nonce generated via cryptographically secure RNG
- Nonce MUST change on every encryption operation (IND-CPA requirement)
- Nonce MUST be prepended to ciphertext

**Encryption Process**:

```
nonce ← 96-bit random (via OsRng or equivalent)
(ciphertext, auth_tag) ← AES-256-GCM-Encrypt(
    key = derived_key,
    plaintext = encoded_data,
    nonce = nonce,
    aad = <empty>
)
output ← nonce (12 bytes) || ciphertext || auth_tag (16 bytes)
```

**Decryption Process**:

```
nonce ← input[0:12]
auth_tag ← input[end-16:end]
ciphertext ← input[12:end-16]
plaintext ← AES-256-GCM-Decrypt(
    key = derived_key,
    ciphertext = ciphertext,
    nonce = nonce,
    auth_tag = auth_tag,
    aad = <empty>
)
```

### 9.4 Timing Side-Channel Mitigation

All cryptographic comparisons (especially auth tag verification) **MUST** use constant-time operations. Implementations using `subtle::ConstantTimeEq` or equivalent timing-safe libraries are required.

---

## 10. Error Handling and Validation

### 10.1 Validation Rules

Every QRD reader MUST perform these checks:

1. **Magic Check**: Reject if first 4 bytes ≠ [0x51, 0x52, 0x44, 0x00]
2. **Format Version**: Reject if FORMAT_MAJOR ≠ 1
3. **Header Reserved**: Reject if bytes 18-19 ≠ 0x0000
4. **Footer Checksum**: Reject if footer CRC32 mismatch
5. **Footer Length**: Reject if declared footer_length > file_size - 36
6. **Schema Determinism**: Reject if re-serializing schema produces different bytes
7. **Encoding ID**: Reject if ENCODING_ID ∉ [0x00, 0x06]
8. **Auth Tag**: Reject (constant-time) if AES-256-GCM auth tag fails
9. **Compression Codec**: Reject if decompression fails

### 10.2 Error Classification

| Error | Severity | Recovery |
|-------|----------|----------|
| Invalid magic | CRITICAL | Reject file; possibly wrong format |
| Unsupported version | CRITICAL | Reject file; need newer reader |
| Footer checksum mismatch | CRITICAL | Reject file; corruption detected |
| Invalid encoding | HIGH | Skip column; partial read possible |
| Auth tag failure | HIGH | Reject file; tampering detected |
| Truncated file | HIGH | Reject file; incomplete write |

---

## 11. Appendix: Golden Vector Tests

### 11.1 Test Vector Strategy

All format implementations MUST pass golden vector tests covering:

1. **Basic Roundtrip**: Write → Read → Binary identical
2. **All Encodings**: Test each encoding algorithm (0x00-0x06)
3. **All Compressions**: Test ZSTD, LZ4, and uncompressed
4. **Encryption**: Test AES-256-GCM with known keys
5. **Edge Cases**: Empty files, single row, 1M+ rows, NULL columns

### 11.2 Deterministic Binary Test

```
Test: Deterministic Output
Given: Same input data, same schema, same row grouping
Expected: Byte-for-byte identical output file
Validator: MD5 hash of output matches reference hash
```

### 11.3 Cross-Language Verification

Implementers MUST verify compatibility:

```
Language A: Write test.qrd
Language B: Read test.qrd → verify row count, schema, data integrity
Language C: Read test.qrd → verify row count, schema, data integrity
Expected: All languages parse identically
```

---

## 12. Normative Conformance Claims

### 12.1 Minimal Implementation (Compliance Level 1)

- Read/write QRD headers
- Parse/serialize schemas
- Support Plain encoding (0x00)
- Support uncompressed row groups
- Verify footer checksums

**Compliance**: Sufficient for basic interoperability

### 12.2 Full Implementation (Compliance Level 2)

- All of Level 1
- All 7 encoding algorithms
- ZSTD and LZ4 compression
- AES-256-GCM encryption with HKDF key derivation
- Constant-time crypto operations
- Full test coverage (396+ tests)

**Compliance**: Production-ready; satisfies FIPS 140-3 requirements

---

## References

- [QRD GitHub Repository](https://github.com/nafal-faturizki/final-QRD)
- [Rust Implementation](https://github.com/nafal-faturizki/final-QRD/tree/main/core/qrd-core)
- [Architecture Documentation](docs/architecture/ARCHITECTURE.md)
- [Compliance Tests](../core/qrd-core/tests/compliance.rs)

---

**Document Approval Status**: 🟢 Phase 2 - RFC Formal Specification  
**Last Reviewed**: May 14, 2026  
**Next Review**: After Phase 2 completion