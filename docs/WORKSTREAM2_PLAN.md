# Workstream 2: Constant-Time Cryptography - Implementation Plan

**Status**: In Progress  
**Duration**: Days 2-5 (4 days)  
**Parallel With**: Workstream 3 (Formal Specification)  
**Depends On**: Workstream 1 ✅ (FIPS 140-3 Alignment)

---

## Objective

Verify and strengthen QRD's cryptographic implementations against timing attacks. Ensure that all secret-dependent operations (key comparisons, authentication tag verification, nonce handling) execute in constant time, regardless of input values.

---

## Why Constant-Time Crypto Matters

**Threat**: Timing Side-Channel Attacks
- Attacker measures operation duration
- Derives secret information from timing differences
- Example: Different decryption time for valid vs invalid auth tags can leak key material

**NIST Requirement**: SP 800-38D (AES-GCM standard) mandates constant-time authentication tag verification

**Phase 2 Impact**: HIPAA/SOC 2 deployments require certified constant-time implementation

---

## Technical Approach

### Task 1: Audit encryption.rs for Timing Vulnerabilities

**Files to Audit**:
- `core/qrd-core/src/encryption.rs` (primary crypto layer)
- `core/qrd-core/src/integrity.rs` (if contains secret comparisons)
- `core/qrd-core/src/parser.rs` (if contains authentication logic)

**Specific Items to Check**:

1. **Authentication Tag Verification**
   - Current: Handled by `aes_gcm` crate (RustCrypto)
   - Status: ✅ RustCrypto uses constant-time by default
   - Action: Verify no manual comparisons exist

2. **Key Comparisons**
   - Search for: `key ==` or `key.eq()`
   - Replace with: `subtle::ConstantTimeEq::ct_eq()`
   - Current findings: None found in encryption.rs (good)

3. **Nonce Handling**
   - Current: Nonces are 12-byte random values
   - Status: ✅ No secret-dependent comparisons
   - Action: Verify nonce is not used in branch conditions

4. **Password/Master Key Processing**
   - Search for: Early returns based on key length
   - Current: Line 37 - `if master_key.is_empty()` - early return
   - Action: Consider timing implications and document as acceptable

5. **HKDF Operations**
   - Status: ✅ RFC 5869 HKDF uses HMAC which is constant-time
   - Action: Verify `hkdf` crate uses constant-time operations

### Task 2: Add `subtle` Crate for Constant-Time Operations

**Changes**:

1. Update `core/qrd-core/Cargo.toml`:
   ```toml
   [dependencies]
   subtle = "2.5"  # Constant-time utilities for cryptography
   ```

2. Import in encryption.rs:
   ```rust
   use subtle::ConstantTimeEq;
   ```

### Task 3: Implement Constant-Time Wrappers

**New Functions** (if needed):

```rust
/// Constant-time tag verification wrapper
pub fn verify_auth_tag_constant_time(
    computed: &[u8; 16],
    received: &[u8; 16],
) -> bool {
    computed.ct_eq(received).into()
}

/// Constant-time key comparison (for key rotation scenarios)
pub fn compare_keys_constant_time(key1: &[u8; 32], key2: &[u8; 32]) -> bool {
    key1.ct_eq(key2).into()
}
```

### Task 4: Add Timing Attack Resistance Tests

**New Test File**: `core/qrd-core/tests/timing_resistance.rs`

**Test Cases**:

1. **Valid vs Invalid Auth Tag Timing** (600 iterations)
   ```rust
   #[test]
   fn timing_valid_vs_invalid_auth_tags() {
       // Encrypt 600 payloads with valid tags
       // Decrypt 600 payloads with invalid tags (single bit flip)
       // Measure time difference
       // Assert: time_diff < 1% variance (threshold TBD)
   }
   ```

2. **Correct vs Incorrect Key Timing**
   ```rust
   #[test]
   fn timing_correct_vs_incorrect_key() {
       // Decrypt with correct key
       // Decrypt with wrong key (single bit difference)
       // Measure time difference
       // Assert: time_diff < 1% variance
   }
   ```

3. **Early vs Late Message Tampering**
   ```rust
   #[test]
   fn timing_tamper_position_independent() {
       // Tamper first byte of ciphertext
       // Tamper last byte of ciphertext
       // Measure decryption failure time
       // Assert: times are equal (within variance)
   }
   ```

4. **Key Length Invariance**
   ```rust
   #[test]
   fn timing_key_length_independent() {
       // Test with keys of different effective lengths
       // Assert: decryption time unchanged
   }
   ```

### Task 5: Benchmark Timing Characteristics

**Tools**:
- `criterion` crate for precise benchmarking
- Sample 1000 operations per configuration

**Configurations to Benchmark**:
- Small payload (10 bytes)
- Medium payload (1KB)
- Large payload (1MB)
- With valid auth tag
- With invalid auth tag (1-bit flip)

---

## Implementation Checklist

- [ ] **Phase 1: Audit**
  - [ ] Review encryption.rs for manual comparisons
  - [ ] Check integrity.rs for secret-dependent branches
  - [ ] Review parser.rs for authentication logic
  - [ ] Document findings in CONSTANT_TIME_AUDIT.md

- [ ] **Phase 2: Implementation**
  - [ ] Add `subtle` to Cargo.toml
  - [ ] Add ConstantTimeEq wrapper functions (if needed)
  - [ ] Document constant-time guarantees

- [ ] **Phase 3: Testing**
  - [ ] Create timing_resistance.rs with 4+ test cases
  - [ ] Verify test execution passes
  - [ ] Document timing variance thresholds

- [ ] **Phase 4: Verification**
  - [ ] Run timing tests 10+ times
  - [ ] Collect statistics on timing variance
  - [ ] Verify all tests pass consistently

- [ ] **Phase 5: Documentation**
  - [ ] Update CRYPTOGRAPHY.md with constant-time section
  - [ ] Create TIMING_ANALYSIS.md with measurements
  - [ ] Add constant-time guarantees to API docs

---

## Success Criteria

| Criterion | Target | Pass/Fail |
|-----------|--------|-----------|
| No manual key comparisons | 0 vulnerabilities | TBD |
| All auth tag checks constant-time | 100% coverage | TBD |
| Timing variance < 3% | <3% | TBD |
| All 600 test iterations pass | 100% | TBD |
| Documentation complete | Yes/No | TBD |

---

## Dependencies & Risks

**Dependencies**:
- Workstream 1 ✅ (completed)
- Access to encryption.rs, integrity.rs, parser.rs
- `subtle` crate availability

**Risks**:
1. **Timing measurements may be noisy** on shared systems
   - Mitigation: Run tests multiple times, use statistical variance
   
2. **Some timing attacks may be undetectable** at Rust level
   - Mitigation: This is acceptable for FIPS Level 1; more advanced attacks (CPU cache, speculative execution) are Level 3+

3. **RustCrypto libraries may have undocumented timing variations**
   - Mitigation: Use latest versions (aes-gcm 0.10+, sha2 0.10+)

---

## Deliverables

1. **Audit Report**: `docs/CONSTANT_TIME_AUDIT.md`
   - Findings from code review
   - Justification for any timing-dependent code
   - RustCrypto library version verification

2. **Timing Resistance Tests**: `core/qrd-core/tests/timing_resistance.rs`
   - 4+ test cases
   - 600+ test iterations
   - Variance measurements

3. **Implementation Changes**: `core/qrd-core/src/encryption.rs` (if needed)
   - Add `subtle` dependency
   - Add constant-time wrappers
   - Add test documentation

4. **Verification Report**: `docs/TIMING_RESISTANCE_REPORT.md`
   - Test results with statistics
   - Timing variance analysis
   - Compliance with NIST SP 800-38D

---

## Timeline

- **Day 2**: Audit & planning (4 hours)
- **Day 3**: Implementation & testing (6 hours)  
- **Day 4**: Verification & documentation (4 hours)
- **Day 5**: Final review & sign-off (2 hours)

**Total**: ~16 hours

---

## Handoff to Workstream 3

**Completed**: Constant-time crypto verified ✅  
**Ready for**: Formal specification (docs/FORMAT_SPEC.md expansion)

---

## References

- NIST SP 800-38D: Recommendation for Block Cipher Modes of Operation: GCM and GMAC
- NIST SP 800-38B: Recommendation for Block Cipher Modes of Operation: The CMAC Mode for Authentication
- subtle crate: https://docs.rs/subtle/
- RustCrypto documentation: https://docs.rs/aes-gcm/
