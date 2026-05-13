# QRD-SDK — Phase 5: Formal Verification & Post-Quantum

> **Prasyarat:** Phase 4 harus 100% selesai sebelum pekerjaan Phase 5 dimulai.
> **Fokus:** Jaminan keamanan jangka panjang — formal verification dan post-quantum cryptography.

---

## Ringkasan Fase

Phase 5 adalah fase kematangan tertinggi QRD-SDK. Fase ini memberikan jaminan keamanan yang tidak lagi hanya bergantung pada pengujian dan audit konvensional, melainkan pada bukti matematis formal dan algoritma kriptografi yang tahan terhadap serangan komputer kuantum. Tidak ada yang di fase ini bersifat "fitur" — semuanya adalah **jaminan keamanan jangka panjang**.

**Cakupan:** Formal Rust parser verification (Prusti/Kani) · ML-KEM post-quantum key encapsulation · Hybrid classical + post-quantum key derivation · HSM key derivation integration

---

## Panduan Implementasi

### 1. Formal Verification Rust Parser (Prusti / Kani)

**Tujuan:** Membuktikan secara matematis bahwa subset kritis parser QRD bebas dari kelas bug tertentu — bukan hanya mengujinya, tetapi memverifikasinya.

**Tool verifikasi:**

| Tool    | Pendekatan                    | Kekuatan                             | Keterbatasan                        |
|---------|-------------------------------|--------------------------------------|-------------------------------------|
| **Kani**  | Bounded model checking (AWS)  | Langsung pada Rust, integrasi `cargo` | Bounded (tidak infinite states)     |
| **Prusti**| Separation logic (ETH Zürich)  | Precondition/postcondition proofs    | Memerlukan anotasi manual (`#[requires]`) |

**Prioritas target verifikasi (critical path subset):**

```
Tier 1 — Wajib diverifikasi:
  parser::parse_header()
    Proof: tidak pernah membaca di luar buffer 32 bytes
    Proof: MAGIC bytes validation tidak dapat bypass
    Proof: RESERVED field validation tidak dapat bypass

  parser::parse_footer_length()
    Proof: selalu membaca tepat 4 bytes terakhir
    Proof: footer_len tidak pernah melebihi file_size - HEADER_SIZE - 4
    Proof: tidak ada integer overflow dalam kalkulasi offset

  parser::parse_column_chunk_header()
    Proof: COLUMN_INDEX tidak melebihi schema field count
    Proof: COMPRESSED_LEN tidak melebihi maksimum yang diizinkan
    Proof: CRC32 validation tidak dapat di-skip

Tier 2 — Target jika resources memungkinkan:
  encryption::derive_column_key()
    Proof: output selalu 32 bytes
    Proof: tidak ada plaintext key material di stack setelah fungsi return

  integrity::verify_crc32()
    Proof: tidak ada false positive dalam kondisi normal (non-adversarial)
```

**Contoh anotasi Kani:**

```rust
#[cfg(kani)]
#[kani::proof]
fn verify_parse_footer_length_no_overflow() {
    let file_size: u64 = kani::any();
    kani::assume(file_size >= HEADER_SIZE as u64 + 4);

    let footer_len: u32 = kani::any();
    // Verifikasi: kalkulasi ini tidak overflow
    let result = check_footer_length(footer_len, file_size);
    // Proof: jika file_size valid, tidak ada overflow
    assert!(result.is_ok() || result.is_err()); // tidak pernah panic
}
```

**Contoh anotasi Prusti:**

```rust
#[requires(buffer.len() == 32)]
#[ensures(result.is_ok() ==> result.unwrap().magic == MAGIC_BYTES)]
#[ensures(result.is_err() ==> true)]  // tidak pernah panic
fn parse_header(buffer: &[u8; 32]) -> Result<FileHeader, ParseError> {
    // ...
}
```

**Aturan formal verification:**
- Kode yang diverifikasi HARUS mempertahankan anotasi verifikasi selamanya. Penghapusan anotasi diperlakukan sebagai security regression.
- Setiap proof HARUS lulus dalam CI sebelum PR yang menyentuh kode terkait dapat di-merge.
- Proof yang gagal diperlakukan sebagai test failure, bukan warning.
- `docs/security/FORMAL_VERIFICATION.md` HARUS mendokumentasikan setiap proof, asumsi, dan batasannya.
- Formal verification TIDAK menggantikan fuzzing dan test suite — keduanya tetap berjalan.

---

### 2. Post-Quantum Key Encapsulation (ML-KEM / NIST FIPS 203)

**Latar belakang:**

Komputer kuantum yang cukup kuat dapat memecahkan RSA, ECDH, dan kurva eliptik menggunakan algoritma Shor. AES-256-GCM sendiri relatif aman terhadap komputer kuantum (Grover's algorithm hanya mengurangi efektif security dari 256-bit ke 128-bit, masih aman). Namun, key exchange dan key encapsulation saat ini (jika digunakan) rentan.

**Solusi: ML-KEM (Module Lattice-based Key Encapsulation Mechanism)**

ML-KEM adalah standar NIST FIPS 203 (finalized 2024) untuk post-quantum key encapsulation. QRD Phase 5 mengadopsi ML-KEM untuk key wrapping — cara master key didistribusikan ke authorized parties.

**Skema penggunaan ML-KEM dalam QRD:**

```
Skenario: Distribusi master key ke reader yang authorized

Tanpa post-quantum (vulnerable):
  Sender: encrypt(master_key, recipient_EC_public_key)  ← rentan quantum
  Recipient: decrypt(ciphertext, recipient_EC_private_key)

Dengan ML-KEM (post-quantum secure):
  Sender:
    1. (kem_ciphertext, shared_secret) = ML-KEM.Encapsulate(recipient_ML_KEM_public_key)
    2. derived_key = HKDF(shared_secret, context="qrd-key-wrap")
    3. wrapped_master_key = AES-256-GCM.Encrypt(master_key, derived_key)
    4. Store: kem_ciphertext + wrapped_master_key (dalam file metadata atau sidecar)

  Recipient:
    1. shared_secret = ML-KEM.Decapsulate(kem_ciphertext, recipient_ML_KEM_private_key)
    2. derived_key = HKDF(shared_secret, context="qrd-key-wrap")
    3. master_key = AES-256-GCM.Decrypt(wrapped_master_key, derived_key)
```

**Parameter ML-KEM yang didukung:**

| Variant         | Security Level | Public Key | Ciphertext | NIST Recommendation |
|-----------------|----------------|------------|------------|---------------------|
| ML-KEM-512      | Level 1 (~AES-128) | 800 bytes | 768 bytes  | Acceptable          |
| ML-KEM-768      | Level 3 (~AES-192) | 1184 bytes | 1088 bytes | Recommended         |
| **ML-KEM-1024** | Level 5 (~AES-256) | 1568 bytes | 1568 bytes | **Default untuk QRD** |

**Default: ML-KEM-1024** karena QRD adalah format long-term storage — data yang disimpan hari ini harus aman dalam 20+ tahun.

**Aturan ML-KEM:**
- ML-KEM hanya digunakan untuk key encapsulation (membungkus master key), bukan untuk enkripsi data.
- Data payload tetap dienkripsi dengan AES-256-GCM (tidak berubah).
- Library yang digunakan: `ml-kem` crate (implementasi pure Rust, side-channel resistant).
- ML-KEM keypair HARUS dapat di-generate oleh `qrd-keygen pq-kem`.
- Format penyimpanan key encapsulation (dalam file metadata atau sidecar) HARUS terdokumentasi dalam spesifikasi formal.

---

### 3. Hybrid Classical + Post-Quantum Key Derivation

**Tujuan:** Masa transisi — tidak semua sistem siap untuk ML-KEM sepenuhnya. Hybrid scheme memberikan keamanan minimum dari kedua dunia: jika satu sistem dikompromi, yang lain masih melindungi.

**Skema hybrid:**

```
Hybrid Key Derivation:

1. Classical component:
   classical_shared_secret = ECDH(sender_EC_privkey, recipient_EC_pubkey)
   ← menggunakan X25519 (Curve25519)

2. Post-quantum component:
   (ml_kem_ct, pq_shared_secret) = ML-KEM.Encapsulate(recipient_ml_kem_pubkey)

3. Hybrid key derivation (HKDF):
   hybrid_key = HKDF-SHA256(
     ikm = classical_shared_secret || pq_shared_secret,
     salt = random_salt_32_bytes,
     info = "qrd-hybrid-key-v1"
   )

4. Use hybrid_key untuk wrap master_key:
   wrapped = AES-256-GCM.Encrypt(master_key, hybrid_key)

Sidecar format:
  [EC_public_key_sender: 32B]      ← untuk ECDH key agreement
  [ML_KEM_ciphertext: 1568B]       ← untuk ML-KEM-1024
  [salt: 32B]
  [wrapped_master_key: 32B + overhead]

Security property:
  Aman jika DAN HANYA JIKA KEDUA classical DAN post-quantum dikompromi.
  Kompromi satu komponen tidak cukup untuk memulihkan master_key.
```

**Mode operasi hybrid yang didukung:**

| Mode            | Deskripsi                                    | Gunakan ketika                          |
|-----------------|----------------------------------------------|-----------------------------------------|
| `classical-only` | X25519 ECDH saja (backward compatible)      | Lingkungan legacy, < Phase 5           |
| `hybrid`        | X25519 + ML-KEM-1024 (default Phase 5)       | Deployment baru, transisi              |
| `pq-only`       | ML-KEM-1024 saja                             | Future-proof, post-transition           |

**Aturan hybrid:**
- Default untuk deployment baru di Phase 5 adalah `hybrid`.
- `classical-only` masih didukung untuk backward compatibility tetapi HARUS menghasilkan deprecation warning dalam log.
- Format sidecar hybrid HARUS backward compatible: reader yang tidak mendukung ML-KEM dapat membaca `classical-only` files.

---

### 4. Hardware Security Module (HSM) Key Derivation Integration

**Tujuan:** Memungkinkan master key atau key derivation dilakukan di dalam HSM, sehingga key material tidak pernah keluar dari hardware boundary.

**Model integrasi:**

```
Tanpa HSM:
  master_key → di aplikasi memory (vulnerable jika process compromised)

Dengan HSM:
  HSM holds:   master_key_handle (opaque, tidak pernah keluar HSM)
  HSM does:    HKDF(master_key_handle, column_info) → column_key
  App receives: column_key (ephemeral, hanya untuk satu operasi)
  column_key:  digunakan untuk AES-256-GCM, kemudian di-zeroize dari memory
```

**Interface yang harus didukung:**

```rust
// Trait yang harus diimplementasikan oleh HSM adapter
pub trait KeyDerivationProvider: Send + Sync {
    fn derive_column_key(
        &self,
        key_handle: &KeyHandle,
        column_name: &str,
        schema_id: &[u8; 8],
    ) -> Result<ZeroizeVec<u8>, KeyError>;

    fn wrap_key(
        &self,
        plaintext_key: &[u8; 32],
        wrapping_key_handle: &KeyHandle,
    ) -> Result<Vec<u8>, KeyError>;

    fn unwrap_key(
        &self,
        wrapped_key: &[u8],
        wrapping_key_handle: &KeyHandle,
    ) -> Result<ZeroizeVec<u8>, KeyError>;
}

// Implementasi software (default, untuk dev/test)
pub struct SoftwareKeyProvider { /* ... */ }

// Implementasi PKCS#11 (untuk HSM production)
pub struct Pkcs11KeyProvider { /* ... */ }
```

**HSM yang didukung dalam panduan:**
- AWS CloudHSM (via PKCS#11)
- Azure Dedicated HSM (via PKCS#11)
- Thales Luna (via PKCS#11)
- Software HSM: SoftHSM2 (untuk development dan testing)
- FIDO2 hardware keys (untuk personal use, terbatas)

**Aturan HSM integration:**
- Semua column key yang diturunkan dari HSM HARUS menggunakan `zeroize` crate untuk zero-memory setelah penggunaan.
- Implementasi PKCS#11 HARUS tersedia sebagai optional feature (`--features pkcs11`), bukan default.
- `docs/deployment/HSM.md` HARUS menyediakan panduan step-by-step untuk minimal dua HSM provider.
- Testing tanpa hardware HSM nyata HARUS menggunakan SoftHSM2 sebagai standar.
- Library: `cryptoki` crate untuk PKCS#11 binding.

---

### 5. Zeroize Policy

Phase 5 memperketat kebijakan memory zeroization:

**Kode yang harus menggunakan `zeroize`:**

```rust
// Setiap key material HARUS di-zeroize setelah penggunaan
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(ZeroizeOnDrop)]
struct ColumnKey([u8; 32]);

#[derive(ZeroizeOnDrop)]
struct MasterKey([u8; 32]);

// Semua buffer yang mengandung plaintext data yang sudah dienkripsi
// HARUS di-zeroize setelah operasi selesai
let mut plaintext_buffer: Vec<u8> = decrypt_chunk(&chunk)?;
process(&plaintext_buffer);
plaintext_buffer.zeroize(); // Manual jika tidak menggunakan ZeroizeOnDrop
```

**Aturan zeroize:**
- Semua struct yang mengandung key material HARUS derive `ZeroizeOnDrop`.
- Semua temporary buffer yang mengandung plaintext dari dekripsi HARUS di-zeroize setelah penggunaan.
- `clippy` custom lint HARUS ada untuk mendeteksi penggunaan `Vec<u8>` atau `[u8; N]` yang mengandung key material tanpa `ZeroizeOnDrop`.
- Dokumen `docs/security/MEMORY_SAFETY.md` HARUS mendokumentasikan kebijakan zeroize.

---

## Aturan Wajib (Rules)

1. **Formal verification proofs tidak boleh dihapus.** Penghapusan anotasi verifikasi dari kode yang telah diverifikasi adalah security regression.
2. **ML-KEM-1024 adalah default.** ML-KEM-512 boleh digunakan hanya jika ada justifikasi eksplisit dan terdokumentasi.
3. **Hybrid adalah mode default untuk Phase 5.** `classical-only` diperbolehkan untuk backward compat tetapi harus memberikan deprecation warning.
4. **Column key wajib di-zeroize.** Tidak ada column key yang boleh ada di memory setelah operasi selesai.
5. **HSM integration harus testable tanpa HSM hardware.** SoftHSM2 wajib didukung sebagai fallback testing.
6. **Post-quantum verification vectors wajib.** Semua ML-KEM operations HARUS diverifikasi terhadap NIST FIPS 203 test vectors.
7. **Tidak ada format change tanpa RFC.** Perubahan format untuk mendukung PQ key encapsulation HARUS melalui RFC proposal yang disetujui maintainer.

---

## Checklist Wajib — Phase 5 Exit Criteria

### Formal Verification

- [ ] Kani diintegrasikan ke CI pipeline (`cargo kani`)
- [ ] Prusti diintegrasikan ke CI pipeline (atau Kani-only jika Prusti tidak feasible)
- [ ] `parser::parse_header()` — proof: tidak membaca di luar buffer 32 bytes
- [ ] `parser::parse_header()` — proof: MAGIC bytes validation tidak dapat bypass
- [ ] `parser::parse_footer_length()` — proof: tidak ada integer overflow dalam offset kalkulasi
- [ ] `parser::parse_footer_length()` — proof: footer_len tidak melebihi batas valid
- [ ] `parser::parse_column_chunk_header()` — proof: COLUMN_INDEX tidak melebihi schema field count
- [ ] `parser::parse_column_chunk_header()` — proof: CRC32 validation tidak dapat di-skip
- [ ] Semua Tier 1 proof lulus di CI
- [ ] Penghapusan anotasi verifikasi dideteksi oleh CI sebagai failure
- [ ] `docs/security/FORMAL_VERIFICATION.md` — setiap proof, asumsi, dan batasan terdokumentasi

### Post-Quantum: ML-KEM

- [ ] `ml-kem` crate diintegrasikan sebagai dependency (`--features post-quantum`)
- [ ] ML-KEM-1024, ML-KEM-768, ML-KEM-512 semua tersedia
- [ ] ML-KEM-1024 adalah default
- [ ] `qrd-keygen pq-kem` menghasilkan ML-KEM keypair (1024 bit)
- [ ] Key encapsulation (`ML-KEM.Encapsulate`) berfungsi
- [ ] Key decapsulation (`ML-KEM.Decapsulate`) berfungsi
- [ ] NIST FIPS 203 test vectors lulus untuk semua tiga variant
- [ ] Format sidecar untuk wrapped master key terdokumentasi dalam spesifikasi
- [ ] ML-KEM operations bersifat constant-time (diverifikasi via benchmark atau library guarantees)

### Hybrid Key Derivation

- [ ] Mode `hybrid` (X25519 + ML-KEM-1024) diimplementasikan
- [ ] Mode `classical-only` (backward compat) diimplementasikan dengan deprecation warning
- [ ] Mode `pq-only` (ML-KEM-1024 saja) diimplementasikan
- [ ] Hybrid key derivation menggunakan HKDF-SHA256 dengan ikm = classical || pq
- [ ] Info string `"qrd-hybrid-key-v1"` digunakan secara konsisten
- [ ] Format sidecar hybrid terdokumentasi
- [ ] Reader yang tidak mendukung ML-KEM dapat membaca `classical-only` files
- [ ] Integration test: tulis dengan hybrid, baca dengan hybrid
- [ ] Security property terdokumentasi: aman jika DAN HANYA JIKA kedua komponen dikompromi

### HSM Integration

- [ ] Trait `KeyDerivationProvider` diimplementasikan
- [ ] `SoftwareKeyProvider` (dev/test) diimplementasikan
- [ ] `Pkcs11KeyProvider` (production) diimplementasikan sebagai optional feature
- [ ] `zeroize` digunakan untuk semua key material dalam HSM integration path
- [ ] Testing dengan SoftHSM2 berfungsi di CI
- [ ] `docs/deployment/HSM.md` — panduan AWS CloudHSM step-by-step
- [ ] `docs/deployment/HSM.md` — panduan Azure Dedicated HSM step-by-step
- [ ] `docs/deployment/HSM.md` — panduan SoftHSM2 untuk development

### Zeroize Policy

- [ ] Semua struct key material derive `ZeroizeOnDrop`
- [ ] Semua temporary plaintext buffer di-zeroize setelah penggunaan
- [ ] `docs/security/MEMORY_SAFETY.md` mendokumentasikan kebijakan zeroize
- [ ] Code review checklist mencakup verifikasi zeroize untuk setiap PR yang menyentuh key handling

### Dokumentasi & Format

- [ ] `docs/security/FORMAL_VERIFICATION.md` tersedia dan lengkap
- [ ] `docs/security/POST_QUANTUM.md` mendokumentasikan ML-KEM integration
- [ ] `docs/deployment/HSM.md` tersedia dengan panduan dua provider
- [ ] FORMAT_SPEC.md diperbarui dengan format sidecar PQ key encapsulation
- [ ] CHANGELOG.md diperbarui
- [ ] Semua perubahan format melalui RFC yang telah disetujui

### Regression & Compatibility

- [ ] Semua test suite Phase 1-4 masih lulus setelah Phase 5 changes
- [ ] Backward compatibility: file QRD v1.0 (classical-only) masih dapat dibaca
- [ ] Performance regression: overhead PQ operations terdokumentasi dalam benchmark
- [ ] Memory overhead PQ operations terdokumentasi (ML-KEM keypair + sidecar)

---

## Definisi Done (Definition of Done)

Phase 5 **SELESAI** ketika:

1. Semua 100% item checklist di atas tercentang.
2. Semua Tier 1 formal verification proof lulus di CI dan tidak dapat di-bypass.
3. ML-KEM-1024 lulus NIST FIPS 203 test vectors.
4. Hybrid key derivation berfungsi end-to-end (write + read).
5. HSM integration berfungsi dengan SoftHSM2 di CI.
6. Tidak ada regresi pada test suite Phase 1-4.
7. Semua key material menggunakan `ZeroizeOnDrop`.
8. Dokumen `docs/security/FORMAL_VERIFICATION.md` dan `docs/security/POST_QUANTUM.md` tersedia, lengkap, dan disetujui oleh security reviewer.

**Phase 5 adalah fase terakhir dalam roadmap QRD-SDK v1. Setelah Phase 5 selesai, QRD-SDK dinyatakan sebagai format yang memenuhi standar keamanan jangka panjang untuk lingkungan yang memerlukan privacy-native encrypted columnar storage.**

---

## Catatan Penting tentang Sertifikasi

Phase 5 **tidak menjamin** sertifikasi formal berikut secara otomatis, tetapi **mempersiapkan** jalan untuk sertifikasi tersebut:

| Sertifikasi       | Status setelah Phase 5            | Langkah tambahan yang diperlukan          |
|-------------------|-----------------------------------|-------------------------------------------|
| FIPS 140-3 Level 2| Aligned (bukan certified)         | Evaluasi pihak ketiga akreditasi NIST     |
| Common Criteria   | Kandidat                          | Security Target document + evaluasi       |
| ISO/IEC 27001     | Implementasi sesuai               | Audit organisasional (bukan teknis)       |
| SOC 2 Type II     | Teknis terpenuhi                  | Audit operasional 6-12 bulan             |

---

*QRD-SDK Phase 5 Implementation Guide · Formal Verification & Post-Quantum Security*
