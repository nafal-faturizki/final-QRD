use qrd_core::parser::{parse_footer, parse_footer_length, parse_header, FileHeader};
use qrd_core::reader::FileReader;
use qrd_core::writer::StreamingWriter;
use qrd_core::compression::{compress, decompress, CompressionKind};
use qrd_core::encryption::{derive_column_key, EncryptionConfig};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Canonical C ABI representation of the QRD file header.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QrdHeaderC {
    pub format_major: u16,
    pub format_minor: u16,
    pub schema_id: [u8; 8],
    pub flags: u16,
    pub writer_version: [u8; 12],
}

impl From<FileHeader> for QrdHeaderC {
    fn from(header: FileHeader) -> Self {
        Self {
            format_major: header.format_major,
            format_minor: header.format_minor,
            schema_id: header.schema_id,
            flags: header.flags,
            writer_version: header.writer_version,
        }
    }
}

/// C ABI status codes.
pub const QRD_OK: i32 = 0;
pub const QRD_INVALID_ARGUMENT: i32 = 1;
pub const QRD_INVALID_FORMAT: i32 = 2;
pub const QRD_NOT_IMPLEMENTED: i32 = 3;
pub const QRD_ENCRYPTION_FAILED: i32 = 4;

/// Opaque pointer to FileReader (for C callers)
#[repr(transparent)]
pub struct QrdReaderHandle {
    inner: *mut FileReader,
}

/// Opaque pointer to StreamingWriter (for C callers)
#[repr(transparent)]
pub struct QrdWriterHandle {
    inner: *mut StreamingWriter,
}

// ============================================================================
// VERSION AND SIZE QUERIES
// ============================================================================

/// Returns the QRD core version as a null-terminated string pointer.
#[no_mangle]
pub extern "C" fn qrd_version() -> *const c_char {
    static VERSION: &[u8] = b"0.1.0\0";
    VERSION.as_ptr().cast()
}

/// Returns the canonical QRD header size.
#[no_mangle]
pub extern "C" fn qrd_header_size() -> usize {
    qrd_core::parser::HEADER_SIZE
}

// ============================================================================
// HEADER PARSING
// ============================================================================

/// Parses a canonical QRD header into a C ABI structure.
#[no_mangle]
pub extern "C" fn qrd_parse_header(
    bytes_ptr: *const u8,
    bytes_len: usize,
    out_header: *mut QrdHeaderC,
) -> i32 {
    if bytes_ptr.is_null() || out_header.is_null() {
        return QRD_INVALID_ARGUMENT;
    }

    let bytes = unsafe {
        // SAFETY: The caller guarantees bytes_ptr is valid for bytes_len reads.
        std::slice::from_raw_parts(bytes_ptr, bytes_len)
    };

    match parse_header(bytes) {
        Ok(header) => {
            unsafe {
                // SAFETY: out_header is checked non-null and points to writable memory.
                *out_header = header.into();
            }
            QRD_OK
        }
        Err(_) => QRD_INVALID_FORMAT,
    }
}

/// Parses a footer-length trailer into a native `u32`.
#[no_mangle]
pub extern "C" fn qrd_parse_footer_length(
    bytes_ptr: *const u8,
    bytes_len: usize,
    out_footer_length: *mut u32,
) -> i32 {
    if bytes_ptr.is_null() || out_footer_length.is_null() {
        return QRD_INVALID_ARGUMENT;
    }

    let bytes = unsafe {
        // SAFETY: The caller guarantees bytes_ptr is valid for bytes_len reads.
        std::slice::from_raw_parts(bytes_ptr, bytes_len)
    };

    match parse_footer_length(bytes) {
        Ok(length) => {
            unsafe {
                // SAFETY: out_footer_length is checked non-null and points to writable memory.
                *out_footer_length = length;
            }
            QRD_OK
        }
        Err(_) => QRD_INVALID_FORMAT,
    }
}

// ============================================================================
// FOOTER PARSING
// ============================================================================

/// Parses a canonical QRD footer from raw bytes.
/// Returns QRD_OK on success, QRD_INVALID_FORMAT on failure.
#[no_mangle]
pub extern "C" fn qrd_parse_footer(
    bytes_ptr: *const u8,
    bytes_len: usize,
) -> i32 {
    if bytes_ptr.is_null() {
        return QRD_INVALID_ARGUMENT;
    }

    let bytes = unsafe {
        // SAFETY: The caller guarantees bytes_ptr is valid for bytes_len reads.
        std::slice::from_raw_parts(bytes_ptr, bytes_len)
    };

    match parse_footer(bytes) {
        Ok(_) => QRD_OK,
        Err(_) => QRD_INVALID_FORMAT,
    }
}

// ============================================================================
// COMPRESSION
// ============================================================================

/// Compresses a payload using Zstandard (compression level 3).
/// Returns QRD_OK on success, error code otherwise.
/// On success, writes compressed data to out_compressed_ptr (caller must allocate).
#[no_mangle]
pub extern "C" fn qrd_compress_zstd(
    payload_ptr: *const u8,
    payload_len: usize,
    out_compressed_ptr: *mut u8,
    out_compressed_len_ptr: *mut usize,
) -> i32 {
    if payload_ptr.is_null() || out_compressed_ptr.is_null() || out_compressed_len_ptr.is_null() {
        return QRD_INVALID_ARGUMENT;
    }

    let payload = unsafe {
        // SAFETY: The caller guarantees payload_ptr is valid for payload_len reads.
        std::slice::from_raw_parts(payload_ptr, payload_len)
    };

    match compress(payload, CompressionKind::Zstd) {
        Ok(compressed) => {
            if compressed.len() > unsafe { *out_compressed_len_ptr } {
                return QRD_INVALID_ARGUMENT; // Output buffer too small
            }
            unsafe {
                // SAFETY: out_compressed_ptr is checked non-null and caller guarantees size.
                std::ptr::copy_nonoverlapping(
                    compressed.as_ptr(),
                    out_compressed_ptr,
                    compressed.len(),
                );
                *out_compressed_len_ptr = compressed.len();
            }
            QRD_OK
        }
        Err(_) => QRD_INVALID_FORMAT,
    }
}

/// Decompresses a payload using Zstandard.
#[no_mangle]
pub extern "C" fn qrd_decompress_zstd(
    compressed_ptr: *const u8,
    compressed_len: usize,
    out_decompressed_ptr: *mut u8,
    out_decompressed_len_ptr: *mut usize,
) -> i32 {
    if compressed_ptr.is_null() || out_decompressed_ptr.is_null() || out_decompressed_len_ptr.is_null()
    {
        return QRD_INVALID_ARGUMENT;
    }

    let compressed = unsafe {
        // SAFETY: The caller guarantees compressed_ptr is valid for compressed_len reads.
        std::slice::from_raw_parts(compressed_ptr, compressed_len)
    };

    match decompress(compressed, CompressionKind::Zstd) {
        Ok(decompressed) => {
            if decompressed.len() > unsafe { *out_decompressed_len_ptr } {
                return QRD_INVALID_ARGUMENT; // Output buffer too small
            }
            unsafe {
                // SAFETY: out_decompressed_ptr is checked non-null and caller guarantees size.
                std::ptr::copy_nonoverlapping(
                    decompressed.as_ptr(),
                    out_decompressed_ptr,
                    decompressed.len(),
                );
                *out_decompressed_len_ptr = decompressed.len();
            }
            QRD_OK
        }
        Err(_) => QRD_INVALID_FORMAT,
    }
}

// ============================================================================
// ENCRYPTION
// ============================================================================

/// Derives a 32-byte column key using HKDF-SHA256.
/// master_key_ptr: pointer to master key bytes
/// master_key_len: length of master key
/// column_name: null-terminated column name string
/// schema_fingerprint: 8-byte schema fingerprint
/// out_key: pointer to 32-byte buffer (caller must allocate)
#[no_mangle]
pub extern "C" fn qrd_derive_column_key(
    master_key_ptr: *const u8,
    master_key_len: usize,
    column_name: *const c_char,
    schema_fingerprint: *const u8,
    out_key: *mut u8,
) -> i32 {
    if master_key_ptr.is_null() || column_name.is_null() || schema_fingerprint.is_null()
        || out_key.is_null()
    {
        return QRD_INVALID_ARGUMENT;
    }

    let master_key = unsafe {
        // SAFETY: The caller guarantees master_key_ptr is valid for master_key_len reads.
        std::slice::from_raw_parts(master_key_ptr, master_key_len)
    };

    let col_name_c = unsafe {
        // SAFETY: The caller guarantees column_name is a valid null-terminated C string.
        CStr::from_ptr(column_name)
    };
    let col_name = match col_name_c.to_str() {
        Ok(s) => s,
        Err(_) => return QRD_INVALID_ARGUMENT,
    };

    let mut schema_fp = [0u8; 8];
    unsafe {
        // SAFETY: The caller guarantees schema_fingerprint is valid for 8 reads.
        std::ptr::copy_nonoverlapping(schema_fingerprint, schema_fp.as_mut_ptr(), 8);
    }

    let config = EncryptionConfig {
        column_name: col_name.to_string(),
        schema_fingerprint: schema_fp,
    };

    match derive_column_key(master_key, &config) {
        Ok(key) => {
            unsafe {
                // SAFETY: out_key is checked non-null and caller guarantees 32 bytes.
                std::ptr::copy_nonoverlapping(key.as_ptr(), out_key, 32);
            }
            QRD_OK
        }
        Err(_) => QRD_ENCRYPTION_FAILED,
    }
}

// ============================================================================
// UTILITY: String conversion helpers
// ============================================================================

/// Converts a Rust string to a C-allocated null-terminated string.
/// The caller must free the returned pointer with qrd_free_string.
#[no_mangle]
pub extern "C" fn qrd_error_message(code: i32) -> *mut c_char {
    let message = match code {
        QRD_OK => "success",
        QRD_INVALID_ARGUMENT => "invalid argument",
        QRD_INVALID_FORMAT => "invalid format",
        QRD_NOT_IMPLEMENTED => "not implemented",
        QRD_ENCRYPTION_FAILED => "encryption failed",
        _ => "unknown error",
    };

    match CString::new(message) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a string previously allocated by qrd_error_message or similar.
#[no_mangle]
pub extern "C" fn qrd_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            // SAFETY: ptr must have been allocated by CString::into_raw.
            let _ = CString::from_raw(ptr);
        }
    }
}

/// Returns whether the FFI layer has been initialized.
#[no_mangle]
pub extern "C" fn qrd_init() -> bool {
    let _ = CStr::from_bytes_with_nul(b"qrd\0");
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_size_matches_core_contract() {
        assert_eq!(qrd_header_size(), qrd_core::parser::HEADER_SIZE);
    }

    #[test]
    fn ffi_header_parser_roundtrips() {
        let header = qrd_core::parser::FileHeader::new(
            1,
            0,
            [1, 2, 3, 4, 5, 6, 7, 8],
            0b11,
            *b"qrd-0.1.0\0\0\0",
        );
        let bytes = header.serialize();
        let mut out = QrdHeaderC {
            format_major: 0,
            format_minor: 0,
            schema_id: [0; 8],
            flags: 0,
            writer_version: [0; 12],
        };

        let status = qrd_parse_header(bytes.as_ptr(), bytes.len(), &mut out);
        assert_eq!(status, QRD_OK);
        assert_eq!(out.format_major, 1);
        assert_eq!(out.schema_id, [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn ffi_footer_length_parser_roundtrips() {
        let mut bytes = vec![1u8, 2, 3, 4];
        bytes.extend_from_slice(&0xAABB_CCDDu32.to_le_bytes());

        let mut out = 0u32;
        let status = qrd_parse_footer_length(bytes.as_ptr(), bytes.len(), &mut out);
        assert_eq!(status, QRD_OK);
        assert_eq!(out, 0xAABB_CCDD);
    }
}
