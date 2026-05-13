#ifndef QRD_H
#define QRD_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct QrdHeaderC {
    uint16_t format_major;
    uint16_t format_minor;
    uint8_t schema_id[8];
    uint16_t flags;
    uint8_t writer_version[12];
} QrdHeaderC;

enum {
    QRD_OK = 0,
    QRD_INVALID_ARGUMENT = 1,
    QRD_INVALID_FORMAT = 2,
};

size_t qrd_header_size(void);
const char *qrd_version(void);
int32_t qrd_parse_header(const uint8_t *bytes_ptr, size_t bytes_len, QrdHeaderC *out_header);
int32_t qrd_parse_footer_length(const uint8_t *bytes_ptr, size_t bytes_len, uint32_t *out_footer_length);
bool qrd_init(void);

#ifdef __cplusplus
}
#endif

#endif