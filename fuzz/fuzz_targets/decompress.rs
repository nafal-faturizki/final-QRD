#![no_main]

use libfuzzer_sys::fuzz_target;
use qrd_core::compression::{decompress, CompressionKind};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let kind = match data[0] % 3 {
        0 => CompressionKind::Zstd,
        1 => CompressionKind::Lz4,
        _ => CompressionKind::Adaptive,
    };

    let _ = decompress(&data[1..], kind);
});