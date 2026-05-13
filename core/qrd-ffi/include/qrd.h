#ifndef QRD_H
#define QRD_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// TYPES AND CONSTANTS
// ============================================================================

typedef struct QrdHeaderC {
    uint16_t format_major;
    uint16_t format_minor;
    uint8_t schema_id[8];
    uint16_t flags;
    uint8_t writer_version[12];
} QrdHeaderC;

/// Opaque handle to FileReader
typedef struct QrdReaderHandle {
    void *inner;
} QrdReaderHandle;

/// Opaque handle to StreamingWriter
typedef struct QrdWriterHandle {
    void *inner;
} QrdWriterHandle;

enum {
    QRD_OK = 0,
    QRD_INVALID_ARGUMENT = 1,
    QRD_INVALID_FORMAT = 2,
    QRD_NOT_IMPLEMENTED = 3,
    QRD_ENCRYPTION_FAILED = 4,
};

// ============================================================================
// VERSION AND SIZE QUERIES
// ============================================================================

/// Returns the QRD core version as a null-terminated string.
const char *qrd_version(void);

/// Returns the canonical QRD file header size in bytes (32).
size_t qrd_header_size(void);

// ============================================================================
// HEADER PARSING
// ============================================================================

/// Parses a canonical QRD header from raw bytes.
/// Returns QRD_OK on success, error code otherwise.
int32_t qrd_parse_header(const uint8_t *bytes_ptr, size_t bytes_len, QrdHeaderC *out_header);

/// Parses the footer-length trailer (last 4 bytes of file).
int32_t qrd_parse_footer_length(
    const uint8_t *bytes_ptr,
    size_t bytes_len,
    uint32_t *out_footer_length
);

// ============================================================================
// FOOTER PARSING
// ============================================================================

/// Parses a canonical QRD footer from raw bytes.
/// Returns QRD_OK if footer is valid, error code otherwise.
int32_t qrd_parse_footer(const uint8_t *bytes_ptr, size_t bytes_len);

// ============================================================================
// COMPRESSION
// ============================================================================

/// Compresses a payload using Zstandard (compression level 3).
/// Caller must allocate out_compressed_ptr with size >= original payload * 1.1 + 18.
/// Sets out_compressed_len_ptr to actual compressed size on success.
int32_t qrd_compress_zstd(
    const uint8_t *payload_ptr,
    size_t payload_len,
    uint8_t *out_compressed_ptr,
    size_t *out_compressed_len_ptr
);

/// Decompresses a Zstandard-compressed payload.
/// Caller must allocate out_decompressed_ptr with size >= original uncompressed size.
/// Sets out_decompressed_len_ptr to actual decompressed size on success.
int32_t qrd_decompress_zstd(
    const uint8_t *compressed_ptr,
    size_t compressed_len,
    uint8_t *out_decompressed_ptr,
    size_t *out_decompressed_len_ptr
);

// ============================================================================
// ENCRYPTION
// ============================================================================

/// Derives a 32-byte column key using HKDF-SHA256.
/// column_name: null-terminated UTF-8 column name
/// schema_fingerprint: 8-byte array
/// out_key: pointer to 32-byte buffer (caller must allocate)
int32_t qrd_derive_column_key(
    const uint8_t *master_key_ptr,
    size_t master_key_len,
    const char *column_name,
    const uint8_t *schema_fingerprint,
    uint8_t *out_key
);

// ============================================================================
// UTILITY
// ============================================================================

/// Returns a human-readable error message for a status code.
/// The returned string must be freed with qrd_free_string.
char *qrd_error_message(int32_t code);

/// Frees a string allocated by QRD functions.
void qrd_free_string(char *ptr);

#ifdef __cplusplus
}
#endif

#endif