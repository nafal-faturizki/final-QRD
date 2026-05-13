# QRD-SDK — Phase 1: Foundation

> **Maturity Gate:** Phase 1 harus diselesaikan 100% sebelum pekerjaan Phase 2 dimulai.
> Semua exit criteria bersifat **hard gate** — tidak ada pengecualian.

---

## Ringkasan Fase

Phase 1 membangun fondasi teknis yang seluruh fase berikutnya bergantung padanya: Rust core engine, lapisan FFI/WASM, semua SDK bahasa pertama, infrastruktur pengujian, dan suite benchmark. Semua komponen di fase ini harus berstatus **Stable** sebelum fase ini dinyatakan selesai.

**Cakupan:** Rust core · FFI · WASM · Python · TypeScript · Go · Java · C/C++ · Test suite · Benchmark suite

---

## Panduan Implementasi

### 1. Rust Core Engine (`core/qrd-core/`)

Core engine adalah **satu-satunya sumber kebenaran format**. Semua SDK bahasa lain harus memanggil core ini — tidak ada reimplementasi mandiri di bahasa lain.

**Struktur modul yang wajib ada:**

```
core/qrd-core/src/
├── schema/        ← SchemaBuilder, serialisasi, SHA-256 fingerprint
├── writer/        ← StreamingWriter, row group flush, footer write
├── reader/        ← FileReader, partial reads, footer parse
├── encoding/      ← PLAIN, RLE, BIT_PACKED, DELTA_BINARY, DELTA_BYTE_ARRAY,
│                     BYTE_STREAM_SPLIT, DICT_RLE
├── compression/   ← ZSTD, LZ4, adaptive selection, entropy estimation
├── encryption/    ← AES-256-GCM, HKDF-SHA256, nonce management
├── ecc/           ← Reed-Solomon encode/decode/recovery
├── columnar/      ← row-to-column transposition
├── integrity/     ← CRC32 per-chunk dan per-footer
└── error/         ← error types dengan taxonomy terstruktur
```

**Kontrak pipeline write (urutan ini adalah kontrak, bukan detail implementasi):**

```
Input Row
  → Row Buffer (bounded O(row_group_size × avg_row_width))
  → Columnar Transpose
  → Per-Column Encoding
  → Per-Chunk Compression      ← HARUS sebelum enkripsi
  → AES-256-GCM Encryption     ← opsional per-kolom, nonce unik per chunk
  → CRC32 / Auth Tag Append
  → Reed-Solomon ECC (opsional)
  → Row Group Flush
  → File Footer Write (terakhir)
  → FOOTER_LENGTH (4 bytes U32LE, paling akhir)
```

> ⚠️ **Aturan wajib:** Kompresi SELALU mendahului enkripsi. Enkripsi sebelum kompresi adalah kesalahan format yang tidak dapat diterima.

---

### 2. Encoding Algorithms (7 wajib diimplementasikan)

| ID     | Algoritma           | Penggunaan terbaik                                 |
|--------|---------------------|----------------------------------------------------|
| `0x00` | PLAIN               | Data entropi tinggi (hash, UUID, float acak)       |
| `0x01` | RLE                 | Kolom low-cardinality / terurut, boolean sorted    |
| `0x02` | BIT_PACKED          | Boolean, integer kecil, kode kategori 4-bit        |
| `0x03` | DELTA_BINARY        | Timestamp monoton, ID auto-increment               |
| `0x04` | DELTA_BYTE_ARRAY    | URL prefix, file path, log prefix, string panjang  |
| `0x05` | BYTE_STREAM_SPLIT   | Data float sensor (suhu, tekanan, koordinat)       |
| `0x06` | DICT_RLE            | String dengan kardinalitas rendah (enum, status)   |

**Aturan encoding:**
- Setiap column chunk menyimpan `ENCODING_ID` di header chunk.
- `decode(encode(x)) == x` untuk semua tipe dan semua encoding — ini diverifikasi oleh property test.
- Enkoding tidak boleh mengubah semantik nilai; hanya representasi fisik.

---

### 3. File Header (32 bytes) — Kontrak Biner

```
Offset  Len  Type     Field           Nilai / Aturan
────────────────────────────────────────────────────────────────────
0        4   [u8;4]  MAGIC           0x51 0x52 0x44 0x00 ("QRD\0")
4        2   U16LE   FORMAT_MAJOR    Breaking format version
6        2   U16LE   FORMAT_MINOR    Non-breaking version
8        8   [u8;8]  SCHEMA_ID       SHA-256 truncated dari schema content
16       2   U16LE   FLAGS           ENCRYPTED | STATS_ENCRYPTED | ECC | SCHEMA_SIGNED
18       2   U16LE   RESERVED        HARUS 0x0000
20      12   [u8;12] WRITER_VERSION  Semver string (UTF-8, null-padded)
```

**Aturan header:**
- Magic bytes HARUS `0x51 0x52 0x44 0x00`. File dengan magic berbeda HARUS ditolak.
- `RESERVED` HARUS `0x0000`. Writer yang mengisi field ini akan dianggap format yang rusak.
- `WRITER_VERSION` diisi dengan semver implementasi yang menulis, bukan format version.

---

### 4. Enkripsi Per-Kolom (AES-256-GCM + HKDF)

**Skema derivasi kunci:**

```
master_key  (dipegang klien, tidak pernah dikirim ke server)
     │
     ▼  HKDF-SHA256(master_key, salt, info="qrd:col:{col_name}:{schema_id}")
column_key_N
     │
     ▼  AES-256-GCM(column_key_N, nonce_random_12bytes)
     │
     ▼  Output: [NONCE (12 bytes)][AUTH_TAG (16 bytes)][CIPHERTEXT (B bytes)]
```

**Aturan enkripsi:**
- Nonce HARUS dibangkitkan secara kriptografis acak per chunk (`OsRng`).
- Auth tag membuktikan integritas DAN autentisitas payload terenkripsi.
- Kolom tidak terenkripsi tetap plainteks dan dapat dibaca tanpa kunci.
- Statistik (min/max/distinct_count) HARUS dienkripsi bersama payload ketika `FLAGS.STATS_ENCRYPTED = 1`.
- File yang menulis dua kali dengan data yang sama AKAN menghasilkan binary output yang berbeda (karena nonce acak) — ini **disengaja** dan diperlukan untuk IND-CPA.

**Primitif kriptografis yang diizinkan:**

| Fungsi              | Library Rust               | Standar               |
|---------------------|----------------------------|-----------------------|
| AES-256-GCM         | `aes-gcm` (RustCrypto)     | NIST SP 800-38D       |
| HKDF-SHA256         | `hkdf` + `sha2`            | RFC 5869              |
| SHA-256             | `sha2` (RustCrypto)        | FIPS 180-4            |
| CRC32               | `crc32fast`                | IEEE 802.3 (0xEDB88320) |
| CSPRNG              | `rand::rngs::OsRng`        | OS entropy source     |
| Ed25519 (opsional)  | `ed25519-dalek`            | RFC 8032              |

---

### 5. Reed-Solomon ECC

**Konfigurasi per Row Group:**

```
DATA_CHUNKS   : N column chunks (data aktual)
PARITY_CHUNKS : K chunk tambahan (diturunkan dari data)
Recovery      : hingga K chunk yang hilang/korup dapat direkonstruksi

Konfigurasi tipikal:
  RS(32,8)  → toleransi 8 chunk korup dari 32 total
  RS(16,4)  → toleransi 4 chunk korup dari 16 total
```

---

### 6. Parser Hardening — Zero-Panic Policy

Core engine HARUS memiliki komitmen zero-panic pada input adversarial:

- Strict bounds check pada semua input eksternal sebelum digunakan.
- Tolak header/footer dengan magic bytes salah, size field overflow, atau truncation.
- Fail-fast dengan error eksplisit untuk encoding/compression ID yang tidak dikenal.
- Semua `unsafe` Rust HARUS didokumentasikan dengan `// SAFETY:` comment yang lengkap.
- Aritmatika integer HARUS menggunakan `checked_*` variant — tidak ada wrapping.
- Fuzz targets aktif untuk semua entry point parser.

---

### 7. Bounded Memory Guarantee

```
Writer:
  peak_memory = row_group_size × avg_row_width_bytes
              + column_dict_overhead (untuk kolom DICT_RLE)
              + ecc_parity_overhead  (jika ECC aktif)

Reader (partial column read):
  peak_memory = Σ(selected_column_chunk_size) × active_parallel_row_groups
              + footer_size (selalu dimuat)

Memory TIDAK PERNAH bergantung pada total ukuran file.
```

Constraint ini HARUS diverifikasi oleh memory regression test di dalam suite.

---

### 8. FFI Layer (`core/qrd-ffi/`)

- ABI HARUS stabil dan kompatibel dengan C.
- FFI bindings HARUS tipis — tidak ada business logic di layer ini.
- Semua opaque pointer HARUS dikelola dengan benar (create/free pair).
- Header file `include/qrd.h` adalah kontrak ABI kanonik.

---

### 9. WASM Target (`core/qrd-wasm/`)

- Target: WebAssembly (WASI + browser).
- Kompatibel dengan Node.js 18+ dan browser modern.
- `initWasm()` HARUS diselesaikan sebelum operasi lain dipanggil.
- Footer inspection HARUS tersedia tanpa membaca payload (`inspectFooter()`).
- Master key TIDAK BOLEH meninggalkan klien di implementasi WASM.

---

### 10. SDK Multi-Bahasa

Semua SDK menggunakan satu Rust core engine — tidak ada reimplementasi format di bahasa lain.

| Bahasa     | Path               | Mekanisme      | Status Target |
|------------|--------------------|----------------|---------------|
| Rust       | `core/qrd-core/`   | Native         | Stable        |
| Python     | `sdk/python/`      | PyO3           | Stable        |
| TypeScript | `sdk/typescript/`  | WASM           | Stable        |
| Go         | `sdk/go/`          | CGO            | Stable        |
| Java       | `sdk/java/`        | JNI            | Stable        |
| C/C++      | `core/qrd-ffi/`    | C FFI          | Stable        |

**Aturan SDK:**
- Setiap SDK HARUS lulus cross-language golden vector test sebelum dinyatakan Stable.
- Setiap SDK HARUS mengimplementasikan: write, read partial column, schema inspection, integrity verification.
- Dokumentasi public API HARUS ada (`///` doc comments + contoh).

---

## Aturan Wajib (Rules)

1. **Format pipeline order adalah kontrak.** Compress → Encrypt, tidak boleh dibalik.
2. **Zero-panic pada adversarial input.** Tidak ada `unwrap()` atau `expect()` pada jalur parsing yang menerima input eksternal tanpa validasi sebelumnya.
3. **Tidak ada reimplementasi format di SDK.** Semua bahasa memanggil Rust core via FFI/WASM.
4. **Determinisme.** Input identik menghasilkan binary output identik di semua bahasa dan platform (kecuali nonce kriptografis).
5. **Master key tidak pernah meninggalkan klien.** Key tidak boleh dikirim ke server, di-log, atau disimpan dalam plaintext.
6. **`finish()` wajib dipanggil.** Footer tidak ditulis tanpa memanggil `finish()`.
7. **Semua `unsafe` Rust harus memiliki `// SAFETY:` comment lengkap.**
8. **Public API harus memiliki `///` doc comment dan minimal satu contoh.**

---

## Checklist Wajib — Phase 1 Exit Criteria

Centang setiap item. Phase 1 dinyatakan selesai hanya jika **semua 100% terceklis**.

### Core Engine

- [ ] Rust core engine dikompilasi tanpa warning (`cargo clippy -- -D warnings`)
- [ ] `cargo fmt` bersih (tidak ada perbedaan format)
- [ ] 7 encoding algorithms diimplementasikan: PLAIN, RLE, BIT_PACKED, DELTA_BINARY, DELTA_BYTE_ARRAY, BYTE_STREAM_SPLIT, DICT_RLE
- [ ] Adaptive compression selection (ZSTD + LZ4) berfungsi
- [ ] AES-256-GCM per-column encryption dengan HKDF-SHA256 berfungsi
- [ ] Reed-Solomon ECC encode/decode/recovery berfungsi
- [ ] CRC32 per-chunk dan per-footer berfungsi
- [ ] Footer parse protocol (7 langkah) diimplementasikan lengkap
- [ ] File header 32 bytes sesuai spesifikasi
- [ ] Zero-panic policy terverifikasi pada input adversarial
- [ ] Semua `unsafe` Rust memiliki `// SAFETY:` comment lengkap
- [ ] Memory bounds (writer & reader) tidak bergantung pada total file size

### Enkripsi & Keamanan

- [ ] Nonce 12 bytes dibangkitkan via `OsRng` per chunk
- [ ] Auth tag AES-GCM memverifikasi integritas DAN autentisitas
- [ ] Statistik terenkripsi saat `FLAGS.STATS_ENCRYPTED = 1`
- [ ] HKDF derivasi kunci per-kolom dengan info string `"qrd:col:{col_name}:{schema_id}"`
- [ ] `Error::AuthenticationFailed` tidak mengekspos detail plaintext
- [ ] Schema fingerprint (SHA-256 truncated) tersimpan di header `SCHEMA_ID`

### FFI & WASM

- [ ] C FFI layer (`core/qrd-ffi/`) berstatus stable ABI
- [ ] `include/qrd.h` tersedia sebagai header kanonik
- [ ] WASM target dikompilasi untuk browser + Node.js
- [ ] `initWasm()` berfungsi sebelum operasi lain dipanggil
- [ ] `inspectFooter()` berfungsi tanpa membaca payload

### SDK Bahasa

- [ ] Python SDK (PyO3) — write, read, schema inspect, verify
- [ ] TypeScript SDK (WASM) — write, read, schema inspect, verify
- [ ] Go SDK (CGO) — write, read, schema inspect, verify
- [ ] Java SDK (JNI) — write, read, schema inspect, verify
- [ ] C/C++ SDK (FFI) — write, read, schema inspect, verify
- [ ] Semua SDK lulus cross-language golden vector test

### Test Suite

- [ ] Total test cases ≥ 10.000
- [ ] Unit tests (`tests/unit/`) ≥ 2.500 cases
- [ ] Property tests (`tests/property/`) ≥ 2.000 cases (proptest)
- [ ] Golden vector tests (`tests/golden/`) ≥ 1.500 cases
- [ ] Integration tests (`tests/integration/`) ≥ 1.500 cases
- [ ] Fuzzing targets aktif: `parse_header`, `parse_footer`, `parse_column_chunk`, `decode_rle`, `decode_delta`, `decrypt_chunk`
- [ ] Fuzzing corpus ≥ 100.000 entries per target
- [ ] `cargo test --workspace` lulus 100%
- [ ] Cross-language roundtrip test: Rust → Python → Go → TypeScript → Java
- [ ] Memory regression test memverifikasi bounded memory guarantees

### Benchmark Suite

- [ ] Criterion benchmark suite berfungsi (`cargo bench --package qrd-core`)
- [ ] Benchmark mencakup: encode, streaming, compression, encryption
- [ ] Hasil benchmark direkam dengan spesifikasi hardware

### Audit Kriptografi

- [ ] Independent cryptographic audit telah dijadwalkan atau selesai
- [ ] Hasil audit terdokumentasi di `docs/security/SECURITY_AUDIT.md`
- [ ] Semua temuan audit beresolusi atau memiliki mitigasi terdokumentasi

### Dokumentasi

- [ ] `docs/FORMAT_SPEC.md` — spesifikasi binary format lengkap dan normatif
- [ ] `docs/architecture/ARCHITECTURE.md` — desain sistem dan overview komponen
- [ ] `docs/security/CRYPTOGRAPHY.md` — justifikasi pemilihan primitif kriptografis
- [ ] `docs/security/FUZZING.md` — coverage fuzz target dan manajemen corpus
- [ ] `CONTRIBUTING.md` — panduan kontribusi lengkap
- [ ] `SECURITY.md` — kebijakan responsible disclosure dan PGP key
- [ ] `CHANGELOG.md` — history versi tersedia

### Tools CLI

- [ ] `qrd-inspect` — dapat inspect footer, schema, stats tanpa full read
- [ ] `qrd-verify` — dapat verifikasi integritas semua chunk + ECC check
- [ ] `qrd-convert` — dapat konversi CSV/Parquet → QRD
- [ ] `qrd-keygen` — dapat generate master key dengan entropi yang tepat

---

## Definisi Done (Definition of Done)

Phase 1 **SELESAI** ketika:

1. Semua 100% item checklist di atas tercentang.
2. `cargo test --workspace` lulus tanpa failure.
3. Fuzzing corpus ≥ 100K entries per target tersedia di repository.
4. Independent cryptographic audit telah dimulai atau selesai.
5. Semua 6 SDK (Rust + 5 bahasa) berstatus Stable.
6. Tidak ada known security issue yang belum diselesaikan.

**Jika salah satu item di atas belum terpenuhi, Phase 2 TIDAK BOLEH dimulai.**

---

*QRD-SDK Phase 1 Implementation Guide · Versi dokumen ini mengacu pada format v1.0.0*
