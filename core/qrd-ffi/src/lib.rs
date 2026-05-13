use qrd_core::parser::{parse_footer_length, parse_header, FileHeader};
use std::ffi::CStr;
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
