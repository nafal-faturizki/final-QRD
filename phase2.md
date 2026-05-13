# QRD-SDK — Phase 2: Hardening & Compliance

> **Prasyarat:** Phase 1 harus 100% selesai sebelum pekerjaan Phase 2 dimulai.
> **Fokus:** Production readiness untuk regulated industries — HIPAA, SOC 2, FIPS 140-3.

---

## Ringkasan Fase

Phase 2 mengubah QRD dari format yang telah berfungsi menjadi format yang siap untuk lingkungan produksi teregulasi. Semua item di fase ini berfokus pada keamanan tingkat enterprise, compliance, kelengkapan tooling, dan ekspansi SDK ke platform mobile dan .NET.

**Cakupan:** FIPS 140-3 alignment · Constant-time crypto · Formal specification · Ed25519 signing · Production tooling · Deployment guides · Swift/Kotlin/C# SDKs

---

## Panduan Implementasi

### 1. FIPS 140-3 Level 1 Alignment

**Tujuan:** Verifikasi operasional bahwa implementasi sesuai dengan FIPS 140-3 Level 1. Ini bukan sertifikasi penuh — melainkan verifikasi bahwa algoritma dan penggunaannya sesuai dengan standar yang dipersyaratkan.

**Algoritma yang harus diverifikasi:**

| Fungsi              | Standar          | Verifikasi yang Diperlukan                          |
|---------------------|------------------|-----------------------------------------------------|
| AES-256-GCM         | NIST SP 800-38D  | NIST test vectors lulus di semua implementasi       |
| HKDF-SHA256         | RFC 5869         | RFC test vectors lulus                              |
| SHA-256             | FIPS 180-4       | NIST CAVP vectors lulus                             |
| CSPRNG (OsRng)      | NIST SP 800-90A  | Entropy source terdokumentasi per platform          |
| Ed25519             | RFC 8032         | RFC test vectors lulus                              |

**Aturan alignment:**
- Penggunaan algoritma non-FIPS (misalnya MD5, SHA-1, DES) DILARANG di semua jalur kriptografi.
- Dokumen `docs/security/CRYPTOGRAPHY.md` HARUS memetakan setiap primitif ke standar NIST/RFC-nya.
- Test suite HARUS mencakup `tests/compliance/` dengan NIST CAVP vectors untuk setiap algoritma.

---

### 2. Constant-Time AES-GCM Verification Path

**Masalah:** Implementasi AES-GCM standar dapat memiliki timing variance pada verifikasi auth tag, yang membuka jalur timing side-channel attack.

**Persyaratan:**

```rust
// Verifikasi auth tag HARUS menggunakan constant-time comparison
// DILARANG: tag_a == tag_b  (bisa short-circuit)
// WAJIB: subtle::ConstantTimeEq
use subtle::ConstantTimeEq;
fn verify_auth_tag(expected: &[u8; 16], computed: &[u8; 16]) -> bool {
    expected.ct_eq(computed).into()
}
```

**Aturan constant-time:**
- Setiap operasi perbandingan auth tag HARUS menggunakan `subtle::ConstantTimeEq`.
- Benchmark HARUS memverifikasi bahwa waktu eksekusi tidak berbeda signifikan antara tag valid dan invalid.
- Untuk high-security deployments, dokumentasikan rekomendasi untuk menggunakan implementasi hardware AES (AES-NI).
- Audit constant-time paths HARUS terdokumentasi di `docs/security/CRYPTOGRAPHY.md`.

---

### 3. Formal Specification (RFC-Style)

**Tujuan:** Memberikan spesifikasi formal dalam format RFC sehingga pihak ketiga dapat mengimplementasikan pembaca/penulis QRD yang kompatibel tanpa bergantung pada kode sumber.

**Struktur dokumen spesifikasi formal:**

```
docs/FORMAT_SPEC.md (normatif)
├── 1. Pendahuluan & Scope
├── 2. Terminologi (MUST, SHALL, SHOULD, MAY per RFC 2119)
├── 3. File Structure Overview
├── 4. File Header Specification (setiap field, setiap bit)
├── 5. Row Group Format
│   ├── 5.1 Row Group Header
│   ├── 5.2 Column Chunk Header (setiap byte)
│   └── 5.3 ECC Parity Chunks
├── 6. File Footer Specification
│   ├── 6.1 Schema Section
│   ├── 6.2 Row Group Section
│   ├── 6.3 Statistics Section
│   ├── 6.4 Encryption Metadata
│   └── 6.5 Footer Parsing Protocol (7 langkah)
├── 7. Encoding Algorithms (setiap ID, setiap format)
├── 8. Compression Codec IDs
├── 9. Encryption Specification
├── 10. Error Handling Requirements
└── Appendix: Golden Vectors (non-normative)
```

**Aturan spesifikasi:**
- Setiap SHALL/MUST requirement HARUS memiliki test case yang mengverifikasi compliance-nya.
- Spesifikasi HARUS cukup lengkap sehingga implementasi independent dapat dibangun hanya dari dokumen ini.
- Setiap perubahan binary format HARUS memperbarui spesifikasi sebelum PR dapat di-merge.

---

### 4. Ed25519 Schema Signing (Stable Feature)

**Tujuan:** Memungkinkan penerima file QRD untuk memverifikasi bahwa schema tidak dimodifikasi setelah penulisan, memberikan non-repudiation untuk audit trail.

**Kontrak format:**

```
[schema_signature]   ← hanya ada jika FLAGS.SCHEMA_SIGNED = 1
  [sig_algo: U8]     ← 0x01 = Ed25519
  [signature: 64 bytes]
  [public_key: 32 bytes]
```

**Aturan schema signing:**
- Signature HARUS dibuat atas hash SHA-256 dari konten schema section footer.
- Public key HARUS tersimpan di dalam file (self-contained verification).
- Verifikasi HARUS gagal jika `FLAGS.SCHEMA_SIGNED = 1` tetapi signature tidak valid.
- Tool `qrd-verify` HARUS mampu memverifikasi schema signature.
- `qrd-keygen` HARUS mampu generate Ed25519 keypair untuk schema signing.

---

### 5. Production CLI Tools

Semua tool CLI HARUS berstatus production-ready di Phase 2:

**`qrd-inspect`**
```
Fungsi wajib:
  qrd-inspect <file>             ← tampilkan schema, row count, column stats
  qrd-inspect --schema <file>    ← tampilkan schema detail
  qrd-inspect --footer <file>    ← tampilkan raw footer metadata
  qrd-inspect --json <file>      ← output JSON untuk scripting
Output: exit code 0 = valid, non-zero = invalid/error
```

**`qrd-verify`**
```
Fungsi wajib:
  qrd-verify <file>                    ← verifikasi CRC32 semua chunk
  qrd-verify --ecc <file>              ← verifikasi + laporan ECC status
  qrd-verify --signature <file>        ← verifikasi schema signature
  qrd-verify --auth-tags <file> --key  ← verifikasi auth tag (butuh key)
Output: laporan pass/fail per chunk, exit code non-zero jika ada failure
```

**`qrd-convert`**
```
Fungsi wajib:
  qrd-convert csv <input.csv> <output.qrd>      ← CSV → QRD
  qrd-convert parquet <input.parquet> <output.qrd>  ← Parquet → QRD
  qrd-convert --encrypt-columns col1,col2 --key-env ENV_VAR
  qrd-convert --row-group-size N
Catatan: konversi bersifat one-way (QRD → format lain tidak di scope Phase 2)
```

**`qrd-keygen`**
```
Fungsi wajib:
  qrd-keygen master            ← generate 256-bit master key (hex)
  qrd-keygen signing           ← generate Ed25519 keypair
  qrd-keygen --output-env      ← output sebagai export statement shell
  qrd-keygen --output-json     ← output sebagai JSON
```

**Aturan CLI:**
- Semua tool HARUS mengembalikan exit code yang benar: 0 = sukses, 1 = user error, 2 = file error, 3 = integrity failure.
- Semua tool HARUS mendukung `--help` dan `--version`.
- Output HARUS dapat digunakan dalam pipeline shell (stdout/stderr yang tepat).
- Tidak ada tool yang boleh mencetak kunci kriptografis ke stdout kecuali secara eksplisit diminta (`qrd-keygen`).

---

### 6. Deployment Guides

Panduan deployment HARUS tersedia untuk tiga domain regulasi:

**Healthcare (HIPAA) — `docs/deployment/HIPAA.md`**

Harus mencakup:
- Kolom mana yang perlu dienkripsi untuk PHI (Protected Health Information)
- Key management yang sesuai (tidak menyimpan master key bersama data)
- Audit logging menggunakan QRD dengan Ed25519 schema signing
- Retensi data dan penghapusan file QRD dengan aman
- Business Associate Agreement (BAA) considerations

**Financial (SOC 2) — `docs/deployment/SOC2.md`**

Harus mencakup:
- Trust Service Criteria yang dipenuhi QRD (Security, Availability, Confidentiality)
- Enkripsi at-rest dan at-transit (QRD + TLS)
- Audit trail dengan schema signing untuk non-repudiation
- Key rotation procedures untuk kolom terenkripsi
- Incident response untuk kunci yang terkompromis

**Edge Telemetry — `docs/deployment/EDGE_TELEMETRY.md`**

Harus mencakup:
- Pemilihan `row_group_size` untuk perangkat RAM-terbatas
- Prioritas LZ4 vs ZSTD berdasarkan resource constraint
- Batch write pattern untuk throughput optimal
- File rotation dan cleanup strategy
- Koneksi intermittent: menulis QRD secara offline, sync saat online

---

### 7. SDK Ekspansi: Swift, Kotlin/Android, .NET/C#

**Swift (iOS/macOS Edge)**
- Target: Swift Package Manager compatible
- Binding: C FFI (`qrd-ffi`) via Swift `@_cdecl`
- Platform minimum: iOS 16+, macOS 13+
- Harus support: `async/await` Swift concurrency model

**Kotlin/Android**
- Target: Android AAR via Maven Central
- Binding: JNI (dapat reuse Java binding dengan Kotlin wrapper)
- Android API level minimum: 26 (Android 8.0)
- Harus support: Coroutines untuk operasi async

**.NET/C#**
- Target: NuGet package
- Binding: P/Invoke ke C FFI (`qrd-ffi`)
- Platform: .NET 6+ (cross-platform: Windows, Linux, macOS)
- Harus support: `async/await` dan `Span<T>` untuk zero-copy reads

**Aturan SDK baru:**
- Setiap SDK baru HARUS lulus cross-language golden vector test sebelum dinyatakan Stable.
- Setiap SDK baru HARUS memiliki CI pipeline sendiri.
- Dokumentasi instalasi HARUS ada di `docs/sdk/SDKS.md`.

---

## Aturan Wajib (Rules)

1. **Constant-time verification wajib.** Auth tag comparison HARUS menggunakan `subtle::ConstantTimeEq`. Operasi comparison non-constant-time dilarang di jalur kriptografi.
2. **NIST compliance vectors wajib.** Semua algoritma kriptografi HARUS lulus NIST CAVP test vectors.
3. **Ed25519 signing harus stabil.** Setelah fitur ini masuk ke `main`, format signature tidak boleh berubah tanpa format version bump.
4. **CLI tools harus idempoten.** `qrd-verify` pada file yang sama HARUS menghasilkan output yang sama.
5. **Tidak ada key di stdout.** Semua tool HARUS mencetak kunci ke file atau environment variable output, bukan stdout biasa.
6. **Deployment guides wajib diverifikasi.** Setiap panduan deployment HARUS diverifikasi oleh setidaknya satu orang dengan keahlian domain yang relevan (HIPAA/SOC2/IoT).
7. **Format spec harus normatif.** Setiap kata MUST/SHALL/SHOULD dalam spesifikasi harus memiliki test case yang mengverifikasinya.

---

## Checklist Wajib — Phase 2 Exit Criteria

### FIPS 140-3 Alignment

- [ ] NIST CAVP test vectors untuk AES-256-GCM lulus di `tests/compliance/`
- [ ] NIST CAVP test vectors untuk SHA-256 lulus
- [ ] RFC 5869 test vectors untuk HKDF-SHA256 lulus
- [ ] RFC 8032 test vectors untuk Ed25519 lulus
- [ ] `docs/security/CRYPTOGRAPHY.md` memetakan setiap primitif ke standar NIST/RFC
- [ ] Tidak ada penggunaan algoritma non-FIPS (MD5, SHA-1, DES) di jalur kriptografi
- [ ] Verifikasi alignment terdokumentasi (bukan sertifikasi penuh)

### Constant-Time Crypto

- [ ] Semua auth tag comparison menggunakan `subtle::ConstantTimeEq`
- [ ] Code review: tidak ada `==` comparison pada tag/key material
- [ ] Timing benchmark menunjukkan tidak ada variance signifikan antara tag valid dan invalid
- [ ] `docs/security/CRYPTOGRAPHY.md` mendokumentasikan constant-time paths
- [ ] Rekomendasi hardware AES (AES-NI) terdokumentasi untuk high-security deployments

### Formal Specification

- [ ] `docs/FORMAT_SPEC.md` menggunakan terminologi RFC 2119 (MUST/SHALL/SHOULD/MAY)
- [ ] Setiap field binary format terdokumentasi dengan offset, length, type, dan valid values
- [ ] Footer parsing protocol 7 langkah terdokumentasi secara normatif
- [ ] Semua MUST/SHALL requirement memiliki test case yang mengverifikasi compliance
- [ ] Spesifikasi dapat digunakan untuk membangun implementasi independen

### Ed25519 Schema Signing

- [ ] Flag `FLAGS.SCHEMA_SIGNED` diimplementasikan
- [ ] Signature disimpan dalam format yang benar: `[sig_algo][signature 64B][public_key 32B]`
- [ ] Verifikasi signature berfungsi di semua SDK
- [ ] `qrd-verify --signature` berfungsi
- [ ] `qrd-keygen signing` menghasilkan Ed25519 keypair dengan entropi yang tepat
- [ ] File yang gagal verifikasi signature menghasilkan error eksplisit

### Production CLI Tools

- [ ] `qrd-inspect` — schema, row count, stats, JSON output, exit codes benar
- [ ] `qrd-verify` — CRC32, ECC, signature, auth tag verification
- [ ] `qrd-convert` — CSV → QRD, Parquet → QRD, dengan opsi enkripsi
- [ ] `qrd-keygen` — master key, Ed25519 keypair, output env/JSON
- [ ] Semua tool mendukung `--help` dan `--version`
- [ ] Semua tool memiliki exit code yang tepat (0/1/2/3)
- [ ] Manual page atau dokumentasi lengkap tersedia untuk setiap tool

### Deployment Guides

- [ ] `docs/deployment/HIPAA.md` — PHI encryption, key management, audit logging
- [ ] `docs/deployment/SOC2.md` — Trust Service Criteria, audit trail, key rotation
- [ ] `docs/deployment/EDGE_TELEMETRY.md` — row group tuning, offline-first patterns
- [ ] Setiap panduan diverifikasi oleh domain expert
- [ ] Contoh konfigurasi kode tersedia di setiap panduan

### SDK Ekspansi

- [ ] Swift SDK — tersedia via Swift Package Manager, iOS 16+ support
- [ ] Swift SDK — lulus cross-language golden vector test
- [ ] Swift SDK — dokumentasi instalasi di `docs/sdk/SDKS.md`
- [ ] Kotlin/Android SDK — tersedia via Maven Central, API 26+ support
- [ ] Kotlin/Android SDK — lulus cross-language golden vector test
- [ ] .NET/C# SDK — tersedia via NuGet, .NET 6+ support
- [ ] .NET/C# SDK — lulus cross-language golden vector test
- [ ] Semua SDK baru memiliki CI pipeline

### Dokumentasi & Audit

- [ ] `docs/security/SECURITY_AUDIT.md` — hasil audit dengan semua temuan dan resolusi
- [ ] Tidak ada open security issue severity High atau Critical
- [ ] Semua Low/Medium security findings memiliki mitigasi atau timeline resolusi terdokumentasi

---

## Definisi Done (Definition of Done)

Phase 2 **SELESAI** ketika:

1. Semua 100% item checklist di atas tercentang.
2. Independent cryptographic audit Phase 1 telah selesai dengan tidak ada temuan kritis yang terbuka.
3. Semua 3 deployment guides tersedia dan telah diverifikasi oleh domain expert.
4. Semua 3 SDK baru (Swift, Kotlin, .NET) berstatus Stable.
5. `tests/compliance/` lulus 100% dengan NIST CAVP vectors.
6. Tidak ada known timing side-channel di jalur auth tag verification.

**Jika salah satu item di atas belum terpenuhi, Phase 3 TIDAK BOLEH dimulai.**

---

*QRD-SDK Phase 2 Implementation Guide · Production Readiness & Compliance*
