# QRD-SDK — Phase 4: Extended Ecosystem

> **Prasyarat:** Phase 3 harus 100% selesai sebelum pekerjaan Phase 4 dimulai.
> **Fokus:** Interoperabilitas, integrasi ekosistem yang lebih luas, dan tipe data lanjutan.

---

## Ringkasan Fase

Phase 4 memposisikan QRD sebagai format yang terintegrasi dengan baik dalam ekosistem data modern — bukan sebagai format yang berdiri sendiri. Fase ini menghadirkan konversi bidireksional dengan Parquet, integrasi mendalam dengan Apache Arrow IPC, kemampuan streaming real-time melalui TCP/QUIC, tipe `MAP`, abstraksi multi-file, dan eksplorasi awal sistem ZK proof post-quantum.

**Cakupan:** Parquet ↔ QRD · Arrow IPC integration · Streaming protocol · MAP type · Multi-file dataset · Formal ZK proof exploration

---

## Panduan Implementasi

### 1. Konversi Bidireksional Parquet ↔ QRD

**Tujuan:** Memudahkan adopsi QRD di ekosistem yang sudah menggunakan Parquet, dan memungkinkan export data QRD ke tooling yang hanya memahami Parquet.

**Arah konversi dan batasannya:**

```
Parquet → QRD (with encryption):
  ✓ Semua tipe Parquet yang didukung dapat dikonversi
  ✓ Enkripsi dapat ditambahkan saat konversi
  ✓ Schema di-mapping ke QRD type system
  ✗ Parquet-specific features (nested maps, delta encoding tertentu) mungkin
    memerlukan mapping manual

QRD → Parquet (dengan encryption caveats):
  ✓ Kolom plainteks dapat diekspor penuh
  ⚠️ Kolom terenkripsi: HARUS di-dekripsi terlebih dahulu sebelum export
  ⚠️ Pengguna HARUS menerima peringatan eksplisit bahwa export menghilangkan enkripsi
  ✗ ECC parity chunks tidak ada padanannya di Parquet (diabaikan saat export)
```

**Type mapping QRD ↔ Parquet:**

| QRD Type        | Parquet Physical Type  | Parquet Logical Type     | Catatan                        |
|-----------------|------------------------|--------------------------|--------------------------------|
| `INT8`          | INT32                  | INT(8, true)             |                                |
| `INT16`         | INT32                  | INT(16, true)            |                                |
| `INT32`         | INT32                  | —                        |                                |
| `INT64`         | INT64                  | —                        |                                |
| `FLOAT32`       | FLOAT                  | —                        |                                |
| `FLOAT64`       | DOUBLE                 | —                        |                                |
| `BOOLEAN`       | BOOLEAN                | —                        |                                |
| `UTF8_STRING`   | BYTE_ARRAY             | STRING                   |                                |
| `TIMESTAMP`     | INT64                  | TIMESTAMP(MICROS, UTC)   |                                |
| `DATE`          | INT32                  | DATE                     |                                |
| `UUID`          | FIXED_LEN_BYTE_ARRAY   | UUID                     |                                |
| `DECIMAL`       | BYTE_ARRAY             | DECIMAL(precision, scale)|                                |
| `BLOB`          | BYTE_ARRAY             | —                        |                                |
| `ENUM`          | BYTE_ARRAY             | ENUM                     |                                |
| `STRUCT`        | GROUP                  | —                        |                                |
| `ARRAY`         | LIST group             | LIST                     |                                |
| `MAP`           | MAP group              | MAP                      | Phase 4                        |

**CLI `qrd-convert` (diperluas dari Phase 2):**

```bash
# Parquet → QRD
qrd-convert parquet input.parquet output.qrd
qrd-convert parquet input.parquet output.qrd --encrypt-columns col1,col2 --key-env QRD_KEY

# QRD → Parquet (dengan peringatan wajib)
qrd-convert qrd-to-parquet input.qrd output.parquet
# Output:
# WARNING: Converting QRD to Parquet will REMOVE encryption from all columns.
# Encrypted columns will be decrypted and stored as plaintext in the Parquet file.
# Type --confirm-plaintext to proceed.

qrd-convert qrd-to-parquet input.qrd output.parquet --key-env QRD_KEY --confirm-plaintext
```

**Aturan konversi:**
- Peringatan plaintext WAJIB ditampilkan dan WAJIB dikonfirmasi untuk QRD → Parquet.
- Konversi QRD → Parquet HARUS gagal jika `--key-env` tidak disediakan untuk file yang berisi kolom terenkripsi (tidak ada silent partial export).
- Type mapping yang tidak didukung HARUS menghasilkan error eksplisit, bukan silent data loss.
- `qrd-convert` HARUS memverifikasi integritas (CRC32/auth tag) file QRD input sebelum konversi.

---

### 2. Arrow IPC Integration

**Tujuan:** Memposisikan QRD sebagai persistent storage layer yang melengkapi Arrow IPC sebagai in-memory layer, bukan bersaing dengannya.

**Model integrasi:**

```
Pipeline persistence pattern:

[Data Source]
     │
     ▼ QRD Writer (encrypted at rest, streaming)
[QRD File Storage]
     │
     ▼ QRD Reader → Arrow RecordBatch
[Arrow In-Memory Layer]
     │
     ▼ Arrow compute kernels / ML inference / analytics
[Results]
     │
     ▼ (optional) Arrow RecordBatch → QRD Writer
[QRD Results Storage]
```

**API yang harus disediakan:**

```rust
// Rust: QRD → Arrow RecordBatch
use arrow::record_batch::RecordBatch;
use qrd_core::{FileReader, ArrowConverter};

let reader = FileReader::open("data.qrd")?;
let batch: RecordBatch = ArrowConverter::to_record_batch(
    reader.read_columns(&["device_id", "temperature"])?,
    reader.schema(),
)?;

// Rust: Arrow RecordBatch → QRD
let writer = StreamingWriter::new(file, qrd_schema, config)?;
ArrowConverter::write_record_batch(&mut writer, &batch)?;
writer.finish()?;
```

**Python interoperability:**

```python
import qrd
import pyarrow as pa

# QRD → PyArrow Table (zero-copy jika kolom plaintext)
reader = qrd.FileReader("data.qrd", master_key=key)
table: pa.Table = reader.to_arrow(columns=["device_id", "temperature"])

# PyArrow Table → QRD
writer = qrd.FileWriter("output.qrd", schema=qrd_schema)
writer.write_arrow(table)
writer.finish()
```

**Aturan Arrow IPC integration:**
- Konversi QRD → Arrow HARUS mendukung semua primitive types yang ada di kedua sistem.
- Kolom terenkripsi HARUS di-dekripsi sebelum disalin ke Arrow RecordBatch.
- Untuk kolom plaintext numerik, implementasi HARUS berusaha zero-copy jika memory layout kompatibel.
- Arrow schema HARUS dapat diturunkan dari QRD schema secara otomatis.
- Dokumen `docs/ARROW_INTEGRATION.md` HARUS menjelaskan model integrasi dan batasan.

---

### 3. Streaming Protocol: QRD over TCP/QUIC

**Tujuan:** Memungkinkan streaming row groups QRD secara real-time dari edge ke cloud tanpa menunggu keseluruhan file selesai ditulis.

**Protocol design:**

```
QRD Streaming Protocol v1

Connection setup:
  Client → Server: [PROTOCOL_MAGIC: 4B]["QRDS"][VERSION: 2B][FLAGS: 2B]
  Server → Client: [ACK: 4B][SESSION_ID: 16B]

Stream framing:
  ┌───────────────────────────────────────────────────────────┐
  │  Frame Header (16 bytes)                                  │
  │    [FRAME_TYPE: U8]    ← 0x01=SCHEMA, 0x02=ROW_GROUP,    │
  │                           0x03=FOOTER, 0x04=END           │
  │    [FLAGS: U8]                                            │
  │    [SEQUENCE_NUM: U32LE]  ← monotonically increasing     │
  │    [PAYLOAD_LEN: U32LE]                                   │
  │    [CHECKSUM: U32LE]   ← CRC32 dari payload              │
  ├───────────────────────────────────────────────────────────┤
  │  Payload (PAYLOAD_LEN bytes)                              │
  │    SCHEMA frame: serialized QRD schema                    │
  │    ROW_GROUP frame: complete row group binary             │
  │    FOOTER frame: complete QRD file footer                 │
  │    END frame: empty, signals stream completion            │
  └───────────────────────────────────────────────────────────┘

Error recovery:
  Server → Client: [NACK: 4B][SEQUENCE_NUM: U32LE] ← request retransmit
  Client → Server: retransmit frame dengan sequence_num yang diminta
```

**Transport layer:**

- **TCP:** baseline compatibility, untuk environment yang tidak mendukung QUIC.
- **QUIC:** preferred untuk edge environments (0-RTT, multiplexing, connection migration).

**Aturan streaming protocol:**
- Schema frame HARUS dikirim pertama sebelum row group frame apapun.
- Footer frame HARUS dikirim terakhir setelah semua row group.
- Enkripsi per-kolom berlaku pada level row group frame (tidak pada level transport).
- Transport HARUS menggunakan TLS 1.3 minimum untuk TCP; QUIC sudah mengandung enkripsi transport.
- Client HARUS dapat reconnect dan melanjutkan streaming dari sequence number terakhir yang di-ACK.
- Server HARUS dapat menyusun kembali file QRD yang valid dari sequence frames.
- Dokumen `docs/STREAMING_PROTOCOL.md` HARUS mendefinisikan protocol secara lengkap.

---

### 4. MAP Type

**Definisi:** Key-value pairs dengan typed key dan value, memungkinkan representasi dictionary-like data dengan schema dinamis.

**Format biner MAP:**

```
Physical layout dalam column chunk:
  ┌─────────────────────────────────────────────┐
  │  MAP header                                 │
  │    [key_type: U8]    ← tipe kunci           │
  │    [value_type: U8]  ← tipe nilai           │
  │    [key_encoding: U8]                       │
  │    [value_encoding: U8]                     │
  ├─────────────────────────────────────────────┤
  │  Per-row count array:                       │
  │    [pair_counts: U16LE × row_count]         │
  │    pair_counts[i] = jumlah key-value pairs  │
  │                     dalam row i             │
  ├─────────────────────────────────────────────┤
  │  Keys (flat, concatenated):                 │
  │    [key_0][key_1]...[key_N]                 │
  ├─────────────────────────────────────────────┤
  │  Values (flat, concatenated):               │
  │    [val_0][val_1]...[val_N]                 │
  └─────────────────────────────────────────────┘
```

**Tipe kunci yang didukung:** `UTF8_STRING`, `INT32`, `INT64`, `UUID`

**Aturan MAP:**
- Kunci dalam satu row HARUS unik (duplikasi kunci dalam satu row adalah error).
- MAP kosong (0 pairs) per row diperbolehkan.
- MAP dengan tipe nilai STRUCT atau ARRAY diperbolehkan.
- MAP tidak dapat bersarang langsung sebagai nilai (MAP of MAP dilarang di Phase 4).
- Enkripsi pada kolom MAP berlaku pada keseluruhan chunk.

---

### 5. Multi-File Dataset Abstraction

**Tujuan:** Memungkinkan pengelolaan kumpulan file QRD sebagai satu logical dataset, dengan schema yang konsisten dan optional shared schema registry.

**Model dataset:**

```
Dataset struktur:
  dataset/
  ├── _schema.qrd-meta    ← master schema definition
  ├── _manifest.json      ← daftar file, metadata, partisi
  ├── part-000001.qrd
  ├── part-000002.qrd
  └── ...

_manifest.json:
{
  "schema_id": "a1b2c3d4...",     ← SHA-256 semua file HARUS sama
  "schema_version": "1.2.0",
  "files": [
    {
      "path": "part-000001.qrd",
      "row_count": 500000,
      "byte_size": 12345678,
      "created_at": "2026-01-15T10:00:00Z",
      "partition": {"date": "2026-01-15"}
    }
  ],
  "total_row_count": 2500000
}
```

**API dataset yang harus disediakan:**

```rust
// Rust: membaca dataset sebagai satu logical unit
let dataset = QrdDataset::open("dataset/")
    .with_master_key(&key)
    .build()?;

// Query across semua files (dengan predicate pushdown per file)
let results = dataset.query()
    .columns(&["device_id", "temperature"])
    .filter(col("temperature").gt(50.0))
    .execute()?;

// Statistik dataset
println!("Total rows: {}", dataset.row_count());
println!("Files: {}", dataset.file_count());
```

**Shared Schema Registry (opsional):**

```
Registry endpoint (HTTP REST):
  GET  /schemas/{schema_id}        ← ambil schema by ID
  POST /schemas                    ← registrasi schema baru
  GET  /schemas/{schema_id}/compat ← cek kompatibilitas

Penggunaan di writer:
  writer config: registry_url = "https://registry.example.com"
  → Writer memvalidasi schema terhadap registry sebelum menulis
  → Writer mendaftarkan schema baru jika belum terdaftar
```

**Aturan multi-file:**
- Semua file dalam dataset HARUS memiliki `SCHEMA_ID` yang sama.
- `_manifest.json` HARUS di-update secara atomic setiap kali file baru ditambahkan.
- Query pada dataset HARUS menggunakan predicate pushdown per file.
- Partial query (membaca sebagian file) HARUS didukung berdasarkan partition metadata.

---

### 6. Formal ZK Proof System Integration (Eksplorasi)

**Status:** Fase ini adalah fase eksplorasi dan proof-of-concept. Tidak ada binary format change yang diizinkan tanpa RFC proposal terlebih dahulu.

**Tujuan eksplorasi:**

1. **Verifiable computation claims:** Membuktikan bahwa agregasi tertentu (misal: rata-rata suhu = X) dihitung secara benar tanpa mengungkap data individual.
2. **Zero-knowledge range proof:** Membuktikan bahwa semua nilai dalam kolom berada dalam range tertentu tanpa mengungkap nilai individual.
3. **Post-quantum preparation:** Mengevaluasi CRYSTALS-Kyber/ML-KEM untuk key encapsulation sebagai persiapan Phase 5.

**Output yang diharapkan dari eksplorasi:**
- Dokumen `docs/ZK_EXPLORATION.md` dengan temuan, trade-off, dan rekomendasi.
- Proof-of-concept (tidak production-ready) yang mendemonstrasikan minimal satu use case.
- RFC proposal untuk format extension jika ZK proof akan diintegrasikan ke format di masa depan.
- Evaluasi kandidat library: `arkworks`, `bellman`, `halo2`.

**Aturan ZK eksplorasi:**
- Tidak ada perubahan format biner tanpa RFC proposal yang disetujui.
- Proof-of-concept HARUS diberi label jelas sebagai `experimental` dan TIDAK diikutkan dalam binary release.
- Eksplorasi post-quantum HARUS mengevaluasi ML-KEM (NIST FIPS 203) sebagai kandidat utama.

---

## Aturan Wajib (Rules)

1. **QRD → Parquet memerlukan konfirmasi eksplisit.** Tidak ada silent plaintext export.
2. **Arrow integration tidak boleh mengubah format.** Arrow adalah layer in-memory; QRD tetap sebagai format file.
3. **Streaming protocol frames harus immutable.** Frame yang sudah di-ACK tidak dapat dimodifikasi retroaktively.
4. **Multi-file dataset harus atomic manifest update.** Race condition pada `_manifest.json` harus dihindari.
5. **ZK exploration tidak boleh mengubah production format.** Semua ZK work berstatus experimental sampai ada RFC yang disetujui.
6. **MAP key harus unik per row.** Violation harus menghasilkan error saat write, bukan saat read.
7. **Type mapping yang tidak didukung harus error, bukan silent data loss.**

---

## Checklist Wajib — Phase 4 Exit Criteria

### Parquet ↔ QRD Konversi

- [ ] `qrd-convert parquet` mengkonversi Parquet → QRD dengan semua tipe yang didukung
- [ ] `qrd-convert qrd-to-parquet` memerlukan `--confirm-plaintext` untuk file terenkripsi
- [ ] Peringatan plaintext ditampilkan sebelum konversi QRD → Parquet
- [ ] Konversi gagal dengan error eksplisit jika key tidak disediakan untuk file terenkripsi
- [ ] Type mapping QRD ↔ Parquet terdokumentasi lengkap
- [ ] Type mapping yang tidak didukung menghasilkan error eksplisit
- [ ] Integritas file QRD input diverifikasi sebelum konversi
- [ ] Integration test: Parquet → QRD → Parquet roundtrip (plaintext columns)

### Arrow IPC Integration

- [ ] `ArrowConverter::to_record_batch()` berfungsi untuk semua primitive types
- [ ] `ArrowConverter::write_record_batch()` berfungsi
- [ ] Python `reader.to_arrow()` berfungsi
- [ ] Python `writer.write_arrow()` berfungsi
- [ ] Kolom terenkripsi di-dekripsi sebelum disalin ke Arrow
- [ ] Zero-copy optimisasi untuk plaintext numerik columns (jika memory layout kompatibel)
- [ ] `docs/ARROW_INTEGRATION.md` tersedia
- [ ] Benchmark: QRD → Arrow conversion throughput terdokumentasi

### Streaming Protocol

- [ ] Protocol specification terdokumentasi di `docs/STREAMING_PROTOCOL.md`
- [ ] Frame types diimplementasikan: SCHEMA, ROW_GROUP, FOOTER, END
- [ ] Sequence number monoton increasing diimplementasikan
- [ ] CRC32 per frame payload diimplementasikan
- [ ] NACK/retransmit mechanism berfungsi
- [ ] TCP transport berfungsi dengan TLS 1.3
- [ ] QUIC transport berfungsi (implementasi referensi menggunakan `quinn`)
- [ ] Server dapat menyusun kembali file QRD yang valid dari stream frames
- [ ] Client dapat reconnect dan melanjutkan dari sequence number terakhir yang di-ACK
- [ ] Integration test: streaming 1M rows dari edge ke cloud

### MAP Type

- [ ] `MAP` type diimplementasikan di binary format
- [ ] Writer dapat menulis kolom MAP (UTF8_STRING, INT32, INT64, UUID keys)
- [ ] Reader dapat membaca kolom MAP
- [ ] Validasi key uniqueness per row saat write
- [ ] MAP of STRUCT berfungsi
- [ ] Empty MAP per row ditangani dengan benar
- [ ] Unit test: MAP roundtrip (empty, single pair, multiple pairs, variable per row)
- [ ] Golden vector: file MAP dapat dibaca semua SDK

### Multi-File Dataset

- [ ] `QrdDataset::open()` dapat membaca dataset multi-file
- [ ] `_manifest.json` format diimplementasikan dan terdokumentasi
- [ ] `_schema.qrd-meta` format diimplementasikan
- [ ] Validasi SCHEMA_ID konsisten di semua file dataset
- [ ] Predicate pushdown berlaku per file dalam dataset query
- [ ] `_manifest.json` update bersifat atomic
- [ ] Partition metadata didukung dalam manifest
- [ ] Partial query berdasarkan partition berfungsi
- [ ] Shared schema registry (opsional): implementasi referensi HTTP REST tersedia

### ZK Exploration

- [ ] `docs/ZK_EXPLORATION.md` tersedia dengan temuan dan rekomendasi
- [ ] Minimal satu proof-of-concept use case didemonstrasikan
- [ ] Proof-of-concept diberi label `experimental` dan tidak dimasukkan dalam binary release
- [ ] Evaluasi ML-KEM (NIST FIPS 203) sebagai kandidat post-quantum terdokumentasi
- [ ] RFC proposal draft untuk format extension (jika ZK akan diintegrasikan)

### Format Version & Dokumentasi

- [ ] FORMAT_MINOR di-bump untuk MAP type
- [ ] `docs/FORMAT_SPEC.md` diperbarui dengan MAP specification
- [ ] `CHANGELOG.md` diperbarui
- [ ] Semua SDK Phase 1-2 diperbarui untuk mendukung MAP type

---

## Definisi Done (Definition of Done)

Phase 4 **SELESAI** ketika:

1. Semua 100% item checklist di atas tercentang.
2. Parquet ↔ QRD konversi berfungsi dua arah dengan semua safeguard.
3. Arrow IPC integration tersedia di Rust dan Python SDK.
4. Streaming protocol berfungsi di TCP dan QUIC dengan test integration end-to-end.
5. MAP type tersedia dan lulus golden vector test di semua SDK.
6. Multi-file dataset API berfungsi dengan predicate pushdown per file.
7. ZK exploration menghasilkan dokumen rekomendasi yang jelas untuk Phase 5.

**Jika salah satu item di atas belum terpenuhi, Phase 5 TIDAK BOLEH dimulai.**

---

*QRD-SDK Phase 4 Implementation Guide · Extended Ecosystem & Interoperability*
