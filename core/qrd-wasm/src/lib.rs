use qrd_core::parser::{parse_footer, parse_footer_length, parse_header, FileFooter, FileHeader};

/// Initializes the WASM layer scaffold.
pub fn init_wasm() -> bool {
    true
}

/// Inspects a raw header buffer without mutating it.
pub fn inspect_header(bytes: &[u8]) -> Option<FileHeader> {
    parse_header(bytes).ok()
}

/// Inspects a raw footer-length trailer without touching payload bytes.
pub fn inspect_footer_length(bytes: &[u8]) -> Option<u32> {
    parse_footer_length(bytes).ok()
}

/// Inspects a canonical footer without touching payload bytes.
pub fn inspect_footer_bytes(bytes: &[u8]) -> Option<FileFooter> {
    parse_footer(bytes).ok()
}

/// Returns a footer inspection placeholder without touching payload bytes.
pub fn inspect_footer() -> &'static str {
    "footer-inspection-placeholder"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_layer_initializes() {
        assert!(init_wasm());
    }

    #[test]
    fn wasm_header_inspection_roundtrips() {
        let header = qrd_core::parser::FileHeader::new(
            1,
            0,
            [9, 8, 7, 6, 5, 4, 3, 2],
            0b1010,
            *b"qrd-0.1.0\0\0\0",
        );
        let bytes = header.serialize();

        let parsed = inspect_header(&bytes).expect("header should parse");
        assert_eq!(parsed, header);
    }

    #[test]
    fn wasm_footer_length_inspection_roundtrips() {
        let mut bytes = vec![1u8, 2, 3, 4];
        bytes.extend_from_slice(&0x1122_3344u32.to_le_bytes());

        let parsed = inspect_footer_length(&bytes).expect("footer length should parse");
        assert_eq!(parsed, 0x1122_3344);
    }
}
