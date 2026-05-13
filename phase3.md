# QRD-SDK — Phase 3: Composite Types & Query Layer

> **Prasyarat:** Phase 2 harus 100% selesai sebelum pekerjaan Phase 3 dimulai.
> **Fokus:** Ekspresivitas tipe, kemampuan analitik, dan query layer minimal.

---

## Ringkasan Fase

Phase 3 memperluas kemampuan ekspresif format QRD dengan menambahkan composite types (`STRUCT`, `ARRAY`), meningkatkan performa pembacaan analitik melalui predicate pushdown dan bloom filters, serta menghadirkan minimal query engine untuk single-file analytical workloads. Fase ini juga mengkonsolidasikan schema evolution tooling.

**Cakupan:** STRUCT type · ARRAY type · Predicate pushdown · Bloom filter · qrd-query engine · Schema evolution tooling

---

## Panduan Implementasi

### 1. STRUCT Type

**Definisi:** Named nested field set yang memungkinkan representasi data hierarkis dalam satu kolom QRD.

**Format biner STRUCT:**

```
Physical layout dalam column chunk:
  ┌─────────────────────────────────────────────┐
  │  STRUCT header                               │
  │    [field_count: U8]                         │
  │    [struct_schema_ref: U16LE]  ← index ke    │
  │                                 schema section│
  ├─────────────────────────────────────────────┤
  │  Untuk setiap field:                         │
  │    [presence_bitmap: ⌈N/8⌉ bytes]            │
  │    [field_data: encoding sesuai tipe field]  │
  └─────────────────────────────────────────────┘

Nested STRUCT (maksimal kedalaman 8 level):
  STRUCT → STRUCT → ... (max depth 8)
```

**Type system mapping:**

```
QRD STRUCT {                    JSON equivalent:
  "name":    UTF8_STRING,   →   { "name": "...",
  "age":     INT32,              "age": 42,
  "active":  BOOLEAN             "active": true }
}
```

**Aturan STRUCT:**
- Kedalaman nesting maksimal 8 level. Lebih dari 8 level HARUS ditolak saat schema build dengan `Error::StructNestingTooDeep`.
- Setiap field STRUCT HARUS memiliki nama unik dalam scope STRUCT tersebut.
- STRUCT yang berisi kolom terenkripsi: enkripsi berlaku pada level column chunk (seluruh STRUCT chunk dienkripsi bersama).
- Schema fingerprint (SCHEMA_ID) HARUS berubah jika definisi STRUCT berubah.
- STRUCT HARUS mendukung nullability (`REQUIRED` / `OPTIONAL`) pada level field individual.
- Partial read pada STRUCT: tidak dapat membaca subset field dari STRUCT dalam satu operasi — seluruh STRUCT column chunk dibaca.

---

### 2. ARRAY Type

**Definisi:** Homogeneous variable-length list — setiap row dapat memiliki jumlah elemen berbeda dalam kolom yang sama.

**Format biner ARRAY:**

```
Physical layout dalam column chunk:
  ┌─────────────────────────────────────────────┐
  │  ARRAY header                               │
  │    [element_type: U8]  ← tipe elemen        │
  │    [element_encoding: U8]                   │
  ├─────────────────────────────────────────────┤
  │  Offset array (per-row batas):              │
  │    [offsets: U32LE × (row_count + 1)]       │
  │    offsets[i]..offsets[i+1] = elemen baris i│
  ├─────────────────────────────────────────────┤
  │  Flat element data:                         │
  │    [semua elemen terserialisasi secara flat] │
  └─────────────────────────────────────────────┘
```

**Contoh:**

```
Row 0: [1, 2, 3]
Row 1: []
Row 2: [7, 8]

offsets: [0, 3, 3, 5]
elements: [1, 2, 3, 7, 8]
```

**Aturan ARRAY:**
- Tipe elemen HARUS homogen: semua elemen dalam satu kolom ARRAY bertipe sama.
- ARRAY tidak boleh bersarang langsung (`ARRAY of ARRAY` dilarang di Phase 3; akan dipertimbangkan di Phase 4).
- ARRAY dapat berisi STRUCT sebagai elemen tipe.
- Ukuran offset array adalah `(row_count + 1) × 4 bytes` — HARUS divalidasi terhadap data aktual.
- Enkripsi pada kolom ARRAY berlaku pada keseluruhan chunk termasuk offset array.

---

### 3. Predicate Pushdown

**Tujuan:** Membaca hanya row group yang relevan berdasarkan statistik footer, tanpa membaca payload data.

**Mekanisme:**

```
Query: SELECT * FROM file.qrd WHERE temperature > 50.0

1. Baca footer (selalu pertama)
2. Untuk setiap row group, periksa statistik kolom "temperature":
     min_val = 10.5, max_val = 48.9 → SKIP (max < 50.0)
     min_val = 35.2, max_val = 92.1 → READ (max > 50.0, mungkin ada yang > 50)
     min_val = 55.0, max_val = 98.3 → READ (min > 50.0, pasti ada)
3. Baca hanya row group yang tidak di-skip
```

**Operator predicate yang wajib didukung:**

| Operator | Tipe Kolom                   | Mekanisme Pushdown         |
|----------|------------------------------|----------------------------|
| `=`      | Semua comparable             | min ≤ val ≤ max            |
| `!=`     | Semua comparable             | tidak dapat di-prune       |
| `>`      | Numerik, temporal            | max > val                  |
| `>=`     | Numerik, temporal            | max >= val                 |
| `<`      | Numerik, temporal            | min < val                  |
| `<=`     | Numerik, temporal            | min <= val                 |
| `IN`     | Semua (dengan bloom filter)  | bloom filter lookup        |
| `IS NULL`| OPTIONAL columns             | null_count > 0             |

**Aturan predicate pushdown:**
- Pushdown HANYA berlaku jika statistik tersedia (`statistics_flag = 0x01`).
- Jika statistik terenkripsi (`STATS_ENCRYPTED = 1`), predicate pushdown TIDAK DAPAT dilakukan tanpa key.
- Pushdown bersifat conservative: row group yang mungkin memiliki data yang cocok HARUS dibaca. False negatives dilarang.
- Pushdown TIDAK menggantikan filter di level row — hasil akhir HARUS diverifikasi row per row.

---

### 4. Bloom Filter Per Column Chunk

**Tujuan:** Mempercepat point lookup pada kolom dengan kardinalitas tinggi.

**Spesifikasi:**

```
Bloom filter format (per column chunk):
  [bloom_filter_version: U8]   ← 0x01 = Split Block Bloom Filter
  [num_bytes: U32LE]           ← ukuran filter dalam bytes
  [filter_bytes...]            ← data bloom filter

Recommended parameters:
  Target FPR (False Positive Rate): 1% (untuk read-heavy workloads)
  Implementasi: Split Block Bloom Filter (kompatibel dengan Parquet)
```

**Aturan bloom filter:**
- Bloom filter bersifat opsional per column chunk; keberadaannya diindikasikan di column chunk header.
- Bloom filter HARUS disimpan SETELAH column chunk data, sebelum ECC parity (jika ada).
- Query engine HARUS menggunakan bloom filter untuk `IN` dan `=` predicate sebelum membaca payload.
- False positive DIPERBOLEHKAN (sifat bloom filter); false negative DILARANG.
- Bloom filter pada kolom terenkripsi HARUS dalam keadaan terenkripsi juga (enkripsi berlaku pada seluruh chunk).

---

### 5. `qrd-query` — Minimal SQL-like Query Engine

**Tujuan:** Memungkinkan analytical queries sederhana langsung pada file QRD tanpa memerlukan database eksternal.

**Grammar SQL yang didukung (subset minimal):**

```sql
-- SELECT kolom
SELECT col1, col2, col3 FROM 'file.qrd'

-- SELECT dengan filter
SELECT col1, col2 FROM 'file.qrd' WHERE col1 > 100

-- SELECT dengan multiple predicates
SELECT * FROM 'file.qrd' WHERE status = 'active' AND timestamp > 1700000000

-- COUNT
SELECT COUNT(*) FROM 'file.qrd'
SELECT COUNT(*) FROM 'file.qrd' WHERE temperature > 50.0

-- LIMIT
SELECT * FROM 'file.qrd' LIMIT 100

-- ORDER BY (single column, single direction)
SELECT device_id, timestamp FROM 'file.qrd' ORDER BY timestamp DESC LIMIT 100

-- Schema inspection
DESCRIBE 'file.qrd'
```

**Yang TIDAK didukung di Phase 3 (reserved untuk masa depan):**
- JOIN (multi-file query)
- GROUP BY / aggregate functions (kecuali COUNT)
- Subquery
- UPDATE / INSERT / DELETE

**Arsitektur `qrd-query`:**

```
SQL String
     │
     ▼ [Parser: pest.rs atau nom]
  AST (Abstract Syntax Tree)
     │
     ▼ [Planner]
  Physical Plan (column selection, predicate list)
     │
     ▼ [Executor]
  Footer Read → Predicate Pushdown → Row Group Filter
     │
     ▼ [Formatter]
  Output: CSV / JSON / Table (--format flag)
```

**Aturan `qrd-query`:**
- Engine HARUS menggunakan predicate pushdown dan bloom filter.
- Engine HARUS menghormati bounded memory guarantee (tidak boleh memuat seluruh file ke memory).
- Engine HARUS mendukung kolom terenkripsi dengan `--key-env ENV_VAR`.
- Engine HARUS mengembalikan error eksplisit untuk syntax yang tidak didukung (bukan silent incorrect result).
- Output CSV dan JSON HARUS UTF-8 encoded.

---

### 6. Schema Evolution Tooling

**Tujuan:** Membantu developer mendeteksi, memvalidasi, dan melakukan migrasi schema yang kompatibel.

**Klasifikasi perubahan schema:**

| Perubahan                          | Kompatibel? | SCHEMA_ID Berubah? |
|------------------------------------|-------------|---------------------|
| Tambah kolom `OPTIONAL` di akhir   | Ya          | Ya                  |
| Tambah optional metadata field     | Ya          | Tidak               |
| Rename field                       | Tidak       | Ya                  |
| Ubah tipe field                    | Tidak       | Ya                  |
| Ubah `REQUIRED` → `OPTIONAL`       | Tidak       | Ya                  |
| Ubah `OPTIONAL` → `REQUIRED`       | Tidak       | Ya                  |
| Reorder kolom                      | Tidak       | Ya                  |

**Tool `qrd-schema-diff`:**

```bash
# Bandingkan dua file QRD
qrd-schema-diff old.qrd new.qrd

# Output contoh:
Schema compatibility: INCOMPATIBLE
Changes:
  [BREAKING] Field 'device_id': type changed ENUM → UTF8_STRING
  [COMPATIBLE] Field 'battery_level' added (OPTIONAL FLOAT32)

# Bandingkan dua schema JSON
qrd-schema-diff --json old-schema.json new-schema.json
```

**Tool `qrd-migrate`:**

```bash
# Migrasi file lama ke schema baru (hanya untuk perubahan kompatibel)
qrd-migrate --old old.qrd --schema new-schema.json --output migrated.qrd

# Dry run (preview tanpa menulis)
qrd-migrate --dry-run --old old.qrd --schema new-schema.json
```

**Aturan schema evolution:**
- `qrd-migrate` HANYA boleh memproses perubahan yang diklasifikasikan sebagai kompatibel.
- Untuk perubahan breaking, tool HARUS menolak dengan error eksplisit dan penjelasan.
- Setelah migrasi, SCHEMA_ID file output HARUS sesuai dengan schema baru.
- `qrd-schema-diff` HARUS tersedia sebagai library (tidak hanya CLI) untuk integrasi CI/CD.

---

## Aturan Wajib (Rules)

1. **STRUCT nesting maksimal 8 level.** Implementasi HARUS menolak schema yang lebih dalam dengan error eksplisit.
2. **ARRAY elemen harus homogen.** Mixed-type array dilarang di Phase 3.
3. **Predicate pushdown conservative.** False negative (skip row group yang berisi data cocok) DILARANG. False positive diperbolehkan.
4. **Bloom filter tidak menghalangi correctness.** Hasil query HARUS benar bahkan jika bloom filter menghasilkan false positive.
5. **qrd-query tidak mengganti database.** Tidak ada support untuk multi-file JOIN atau transaksi.
6. **Schema migration hanya untuk perubahan kompatibel.** Untuk perubahan breaking, user harus menulis ulang file QRD secara penuh.
7. **Binary format changes require format version bump.** Penambahan STRUCT/ARRAY mengubah FORMAT_MINOR minimal.

---

## Checklist Wajib — Phase 3 Exit Criteria

### STRUCT Type

- [ ] `STRUCT` type diimplementasikan di binary format
- [ ] Schema builder mendukung definisi STRUCT dengan field bertipe
- [ ] Validasi kedalaman nesting maks 8 level saat schema build
- [ ] `Error::StructNestingTooDeep` diimplementasikan
- [ ] Writer dapat menulis kolom STRUCT
- [ ] Reader dapat membaca kolom STRUCT
- [ ] Enkripsi berlaku pada STRUCT column chunk secara utuh
- [ ] SCHEMA_ID berubah saat STRUCT definition berubah
- [ ] Unit test: STRUCT roundtrip semua tipe primitif sebagai field
- [ ] Property test: STRUCT dengan arbitrary field values
- [ ] Golden vector: file STRUCT dapat dibaca semua SDK

### ARRAY Type

- [ ] `ARRAY` type diimplementasikan di binary format
- [ ] Offset array format diimplementasikan (U32LE × row_count+1)
- [ ] Writer dapat menulis kolom ARRAY dengan row length bervariasi
- [ ] Reader dapat membaca kolom ARRAY
- [ ] ARRAY of STRUCT berfungsi
- [ ] Empty array per row ditangani dengan benar (offsets[i] == offsets[i+1])
- [ ] Validasi offset array size vs data aktual saat membaca
- [ ] Unit test: ARRAY roundtrip (empty, single element, variable length)
- [ ] Golden vector: file ARRAY dapat dibaca semua SDK

### Predicate Pushdown

- [ ] Statistics dibaca dari footer sebelum row group dibaca
- [ ] Row group skip berdasarkan min/max statistics berfungsi untuk `>`, `>=`, `<`, `<=`, `=`
- [ ] `IS NULL` pushdown berfungsi berdasarkan `null_count`
- [ ] Pushdown disabled ketika `STATS_ENCRYPTED = 1` tanpa key
- [ ] Benchmark: predicate pushdown mengurangi I/O secara terukur
- [ ] Test: false negative tidak pernah terjadi (semua matching rows selalu dikembalikan)

### Bloom Filter

- [ ] Bloom filter format Split Block per column chunk diimplementasikan
- [ ] Writer dapat membuat bloom filter saat dikonfigurasi
- [ ] Reader menggunakan bloom filter untuk `IN` dan `=` predicate
- [ ] Bloom filter pada kolom terenkripsi dienkripsi bersama chunk
- [ ] Unit test: false positive rate sesuai parameter yang dikonfigurasi
- [ ] Test: false negative tidak pernah terjadi

### `qrd-query`

- [ ] SQL parser mengimplementasikan grammar yang didefinisikan
- [ ] `SELECT col1, col2 FROM 'file.qrd'` berfungsi
- [ ] `WHERE` clause dengan operator `>`, `>=`, `<`, `<=`, `=`, `AND` berfungsi
- [ ] `COUNT(*)` berfungsi
- [ ] `LIMIT N` berfungsi
- [ ] `ORDER BY col ASC/DESC LIMIT N` berfungsi
- [ ] `DESCRIBE 'file.qrd'` menampilkan schema
- [ ] Predicate pushdown aktif saat menjalankan query
- [ ] Bloom filter aktif saat menjalankan query
- [ ] Dukungan kolom terenkripsi via `--key-env`
- [ ] Output format: tabel, CSV, JSON (`--format`)
- [ ] Bounded memory: query pada file besar tidak melebihi configured memory limit
- [ ] Error message eksplisit untuk syntax tidak didukung
- [ ] Benchmark: query dengan pushdown vs tanpa pushdown menunjukkan perbedaan nyata

### Schema Evolution Tooling

- [ ] `qrd-schema-diff` dapat membandingkan dua file QRD
- [ ] `qrd-schema-diff` mengklasifikasikan setiap perubahan sebagai COMPATIBLE/BREAKING
- [ ] `qrd-schema-diff --json` tersedia untuk integrasi CI/CD
- [ ] `qrd-migrate` menangani penambahan kolom OPTIONAL dengan benar
- [ ] `qrd-migrate` menolak perubahan breaking dengan error eksplisit
- [ ] SCHEMA_ID file output `qrd-migrate` sesuai schema baru
- [ ] `qrd-schema-diff` tersedia sebagai library Rust

### Format Version

- [ ] FORMAT_MINOR di-bump untuk penambahan STRUCT dan ARRAY
- [ ] `docs/FORMAT_SPEC.md` diperbarui dengan STRUCT dan ARRAY specification
- [ ] Backward compatibility: reader v1.1 dapat membaca file v1.0 (kolom STRUCT/ARRAY tidak ada)
- [ ] CHANGELOG.md diperbarui

---

## Definisi Done (Definition of Done)

Phase 3 **SELESAI** ketika:

1. Semua 100% item checklist di atas tercentang.
2. `qrd-query` dapat menjalankan semua contoh query dalam panduan ini.
3. STRUCT dan ARRAY golden vectors tersedia dan lulus di semua SDK Phase 1 + Phase 2.
4. Predicate pushdown terbukti mengurangi I/O dalam benchmark (minimum 50% reduction pada selective queries).
5. `qrd-schema-diff` dapat diintegrasikan sebagai library dalam CI/CD pipeline.
6. `docs/FORMAT_SPEC.md` telah diperbarui dan disetujui oleh maintainer.

**Jika salah satu item di atas belum terpenuhi, Phase 4 TIDAK BOLEH dimulai.**

---

*QRD-SDK Phase 3 Implementation Guide · Composite Types & Query Layer*
