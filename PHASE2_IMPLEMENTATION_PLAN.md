# PHASE 2 IMPLEMENTATION PLAN — Production Readiness & Compliance

**Start Date**: May 13, 2026  
**Target Completion**: 30 days  
**Status**: 🟢 **READY TO START**

---

## 📋 Phase 2 Scope Overview

Phase 2 transforms QRD from a functional format into production-ready system for regulated industries (HIPAA, SOC 2, FIPS 140-3).

### Seven Major Workstreams

| # | Workstream | Priority | Est. Days | Status |
|---|-----------|----------|-----------|--------|
| 1 | FIPS 140-3 Alignment | **CRITICAL** | 5 | 🟠 Starting |
| 2 | Constant-Time Crypto | **CRITICAL** | 4 | 🔴 Not Started |
| 3 | Formal Specification | **HIGH** | 6 | 🔴 Not Started |
| 4 | Ed25519 Schema Signing | **HIGH** | 5 | 🔴 Not Started |
| 5 | Production CLI Tools | **HIGH** | 5 | 🔴 Not Started |
| 6 | Deployment Guides | **HIGH** | 5 | 🔴 Not Started |
| 7 | SDK Expansion (Swift/Kotlin/.NET) | **HIGH** | 8 | 🔴 Not Started |
| 8 | Golden Vector Tests + Audit | **CRITICAL** | 4 | 🔴 Not Started |

**Total Estimated**: 42 days (planning for 30-day compressed timeline)

---

## 🎯 Phase 2 Exit Criteria (Success Metrics)

✅ All 7 workstreams 100% complete  
✅ 100% NIST CAVP test vectors passing  
✅ All production CLI tools passing integration tests  
✅ All 3 new SDKs at Stable status  
✅ All deployment guides verified by domain experts  
✅ Security audit completed with zero critical findings  
✅ 246+ tests (Phase 1) + 150+ new Phase 2 tests = 396+ total tests  

---

## 📊 Execution Strategy

### Sequential Dependency Chain

```
Phase 1 (COMPLETE) ✅
    ↓
1. FIPS 140-3 Alignment → 2. Constant-Time Crypto (parallel dependency)
    ↓
3. Formal Specification (can start with #1/#2)
    ↓
4. Ed25519 Schema Signing (depends on crypto #2)
    ↓
5. Production CLI Tools (depends on schema signing #4)
    ↓
6. Deployment Guides (depends on CLI #5)
    ↓
7. SDK Expansion (parallel with #6)
    ↓
8. Golden Vector Tests (final integration)
    ↓
PHASE 2 COMPLETE → Ready for Phase 3
```

### Resource Allocation

- **Security-Critical Path**: Workstreams 1, 2, 4 (cryptography)
- **Compliance Path**: Workstreams 3, 6 (documentation)
- **Tooling Path**: Workstreams 5, 7 (CLI + SDKs)
- **Verification**: Workstream 8 (testing + audit)

---

## 🔐 Workstream 1: FIPS 140-3 Alignment (Days 1-5)

### Components to Verify

| Algorithm | Standard | Test Vector Source | Status |
|-----------|----------|-------------------|--------|
| AES-256-GCM | NIST SP 800-38D | NIST CAVP | 🔴 TODO |
| HKDF-SHA256 | RFC 5869 | RFC Test Vector | 🔴 TODO |
| SHA-256 | FIPS 180-4 | NIST CAVP | 🔴 TODO |
| CSPRNG (OsRng) | NIST SP 800-90A | Entropy doc | 🔴 TODO |
| Ed25519 | RFC 8032 | RFC Test Vector | 🔴 TODO |

### Deliverables

- [ ] Create `tests/compliance/` directory structure
- [ ] Implement NIST CAVP test vectors for AES-256-GCM (20+ vectors)
- [ ] Implement RFC 5869 HKDF test vectors (10+ vectors)
- [ ] Implement NIST CAVP test vectors for SHA-256 (10+ vectors)
- [ ] Implement RFC 8032 Ed25519 test vectors (15+ vectors)
- [ ] Document entropy sources per platform in CRYPTOGRAPHY.md
- [ ] All test vectors passing (100% success rate)
- [ ] Create `docs/COMPLIANCE.md` with verification report

---

## ⏱️ Workstream 2: Constant-Time Crypto (Days 2-5)

### Security Audit Checklist

- [ ] Audit all `==` comparisons in encryption.rs
- [ ] Replace with `subtle::ConstantTimeEq` for auth tags
- [ ] Review key comparison paths
- [ ] Benchmark timing variance: valid vs invalid auth tags
- [ ] Document constant-time guarantees in CRYPTOGRAPHY.md

### Deliverables

- [ ] All auth tag comparisons using constant-time functions
- [ ] Zero timing side-channel vulnerabilities
- [ ] Benchmark results showing <5% timing variance
- [ ] Code review documentation
- [ ] Recommendation for AES-NI hardware acceleration

---

## 📖 Workstream 3: Formal Specification (Days 3-8)

### RFC-Style Specification Structure

```
docs/FORMAT_SPEC.md (Update/Expand)
├── 1. Introduction & Scope
├── 2. Terminology (RFC 2119: MUST/SHALL/SHOULD/MAY)
├── 3. File Structure (High-level)
├── 4. Header Specification (Byte-by-byte)
├── 5. Row Group Format
├── 6. Footer Specification
├── 7. Encoding Algorithms
├── 8. Compression Codecs
├── 9. Encryption Specification
├── 10. Error Handling
└── Appendix: Golden Vectors
```

### Deliverables

- [ ] RFC 2119 terminology throughout
- [ ] Every field documented: offset, length, type, valid values
- [ ] Footer parsing protocol 7-step process documented
- [ ] Every MUST/SHALL has corresponding test
- [ ] Specification self-contained for independent implementation

---

## 🔑 Workstream 4: Ed25519 Schema Signing (Days 5-9)

### Implementation Tasks

- [ ] Add `FLAGS.SCHEMA_SIGNED` flag (bit 0x02)
- [ ] Implement signature format: `[algo][sig64][pubkey32]`
- [ ] Implement Ed25519 signing in writer pipeline
- [ ] Implement signature verification in reader pipeline
- [ ] Add `qrd-keygen signing` command
- [ ] Add `qrd-verify --signature` command
- [ ] Integration tests for signing/verification

### File Format Change

```
Before: [footer_size: U64] [footer_data] [checksum_or_other]
After:  If FLAGS.SCHEMA_SIGNED:
          [sig_algo: U8] [signature: 64B] [pubkey: 32B]
        [footer_size: U64] [footer_data] [checksum]
```

### Deliverables

- [ ] Ed25519 signing working end-to-end
- [ ] All SDKs can verify signatures
- [ ] 15+ integration tests
- [ ] Non-repudiation verified

---

## 🛠️ Workstream 5: Production CLI Tools (Days 7-11)

### Enhanced CLI Commands

```
qrd-inspect:
  - Show schema, row count, column stats
  - JSON output mode
  - Exit codes: 0=OK, 1=user error, 2=file error, 3=integrity fail

qrd-verify:
  - CRC32 verification for all chunks
  - ECC status reporting
  - Schema signature verification
  - Auth tag verification (with key)
  - Detailed pass/fail report

qrd-convert:
  - CSV → QRD conversion
  - Parquet → QRD conversion
  - Column encryption support
  - Row group size control

qrd-keygen:
  - Master key generation (256-bit)
  - Ed25519 keypair generation
  - Environment variable output
  - JSON output
```

### Deliverables

- [ ] All 4 CLI tools production-ready
- [ ] Correct exit codes for all scenarios
- [ ] Help and version support
- [ ] 40+ integration tests
- [ ] Manual pages / comprehensive documentation

---

## 📋 Workstream 6: Deployment Guides (Days 9-13)

### Three Required Guides

1. **`docs/deployment/HIPAA.md`**
   - [ ] PHI encryption requirements
   - [ ] Key management patterns
   - [ ] Audit logging with schema signing
   - [ ] Data retention & secure deletion
   - [ ] Example code

2. **`docs/deployment/SOC2.md`**
   - [ ] Trust Service Criteria mapping
   - [ ] Encryption at-rest and at-transit
   - [ ] Audit trail with non-repudiation
   - [ ] Key rotation procedures
   - [ ] Incident response plan
   - [ ] Example code

3. **`docs/deployment/EDGE_TELEMETRY.md`**
   - [ ] Row group size tuning
   - [ ] LZ4 vs ZSTD selection
   - [ ] Batch write patterns
   - [ ] File rotation strategy
   - [ ] Offline-first sync patterns
   - [ ] Example code

### Deliverables

- [ ] All 3 guides complete with examples
- [ ] Each verified by domain expert
- [ ] 30+ code examples
- [ ] 50+ tests for example code

---

## 📱 Workstream 7: SDK Expansion (Days 10-17)

### Swift SDK (iOS/macOS Edge)

- [ ] Swift Package Manager structure
- [ ] C FFI bindings via Swift
- [ ] Async/await support
- [ ] iOS 16+, macOS 13+ support
- [ ] 30+ integration tests
- [ ] Documentation & examples

### Kotlin/Android SDK

- [ ] Android AAR packaging
- [ ] Maven Central publishing
- [ ] JNI bindings
- [ ] Coroutines support
- [ ] Android API 26+ support
- [ ] 30+ integration tests
- [ ] Documentation & examples

### C#/.NET SDK

- [ ] NuGet package structure
- [ ] P/Invoke bindings to C FFI
- [ ] Async/await support
- [ ] Span<T> for zero-copy
- [ ] .NET 6+ support (Windows/Linux/macOS)
- [ ] 30+ integration tests
- [ ] Documentation & examples

### Deliverables

- [ ] 3 new SDKs at Stable status
- [ ] 100+ new integration tests
- [ ] Cross-language golden vector tests passing
- [ ] CI pipelines for each SDK

---

## ✅ Workstream 8: Verification & Audit (Days 18-22)

### Testing & Validation

- [ ] Run all 396+ tests (Phase 1 + Phase 2)
- [ ] 100% pass rate
- [ ] Golden vector tests all passing
- [ ] Cross-language compatibility verified

### Security Audit

- [ ] Complete independent security audit
- [ ] Zero critical findings
- [ ] All Low/Medium findings have mitigations
- [ ] Audit report published

### Documentation Review

- [ ] All deployment guides verified by experts
- [ ] All compliance documentation complete
- [ ] Phase 2 completion checklist verified

### Deliverables

- [ ] Phase 2 Verification Report
- [ ] Security Audit Report
- [ ] Test Coverage Report (396+ tests)
- [ ] Phase 2 Sign-off Document

---

## 🎯 Success Criteria per Workstream

| Workstream | Success Metric | Target |
|-----------|----------------|--------|
| 1. FIPS | All NIST vectors passing | 100% |
| 2. Const-Time | Zero timing variance | <5% |
| 3. Spec | Independent implementation possible | Yes |
| 4. Ed25519 | Non-repudiation verified | Yes |
| 5. CLI Tools | All tools production-ready | Yes |
| 6. Deployment | All 3 guides + examples | 100% |
| 7. SDKs | 3 new SDKs stable | Stable |
| 8. Verification | 396+ tests passing | 100% |

---

## 📅 Timeline & Milestones

**Week 1 (Days 1-7)**
- ✅ FIPS 140-3 alignment complete
- ✅ Constant-time crypto verified
- ✅ Formal specification started
- ✅ Ed25519 schema signing started

**Week 2 (Days 8-14)**
- ✅ Formal specification complete
- ✅ Ed25519 schema signing complete
- ✅ Production CLI tools complete
- ✅ SDK expansion started

**Week 3 (Days 15-22)**
- ✅ SDK expansion complete
- ✅ Deployment guides complete
- ✅ Golden vector tests passing
- ✅ Security audit complete

**Week 4 (Days 23-30)**
- ✅ Full verification
- ✅ Phase 2 sign-off
- ✅ Ready for Phase 3

---

## 🚀 Next Steps

**Start immediately with Workstream 1:**

1. Create compliance test directory structure
2. Add NIST test vectors for AES-256-GCM
3. Verify all vectors passing
4. Document findings

---

**Phase 2 Implementation Plan Approved**  
**Start Date**: May 13, 2026  
**Target Completion**: June 12, 2026  
**Status**: 🟢 **READY TO EXECUTE**
