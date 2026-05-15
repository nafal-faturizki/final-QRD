use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use qrd_core::compression::{compress, decompress, CompressionKind};
use qrd_core::encoding::{decode, encode, EncodingId};
use qrd_core::encryption::{decrypt_payload, derive_column_key, encrypt_payload, EncryptionConfig};
use qrd_core::integrity::crc32;
use qrd_core::schema::FieldKind;
use qrd_core::schema::SchemaBuilder;
use qrd_core::writer::StreamingWriter;

fn benchmark_crc32(c: &mut Criterion) {
    c.bench_function("crc32_small_payload", |b| {
        b.iter(|| {
            let payload = black_box(b"123456789".to_vec());
            crc32(&payload)
        })
    });

    c.bench_function("crc32_large_payload", |b| {
        let payload = black_box(vec![0u8; 10_000]);
        b.iter(|| crc32(&payload))
    });
}

fn benchmark_schema_fingerprint(c: &mut Criterion) {
    c.bench_function("schema_fingerprint", |b| {
        b.iter(|| {
            let schema = qrd_core::schema::SchemaBuilder::new()
                .add_field("device_id", FieldKind::Utf8, true)
                .add_field("temperature", FieldKind::Float32, false)
                .build()
                .expect("schema should build");
            black_box(schema.fingerprint())
        })
    });
}

fn benchmark_encodings(c: &mut Criterion) {
    let payload = black_box(
        (0..1_000u16)
            .cycle()
            .take(10_000)
            .map(|x| (x % 64) as u8)
            .collect::<Vec<u8>>(),
    );

    let encodings = vec![
        ("PLAIN", EncodingId::Plain),
        ("RLE", EncodingId::Rle),
        ("BIT_PACKED", EncodingId::BitPacked),
        ("DELTA_BINARY", EncodingId::DeltaBinary),
        ("DELTA_BYTE_ARRAY", EncodingId::DeltaByteArray),
        ("BYTE_STREAM_SPLIT", EncodingId::ByteStreamSplit),
        ("DICT_RLE", EncodingId::DictRle),
    ];

    for (name, encoding_id) in encodings {
        c.bench_function(&format!("encode_{}", name), |b| {
            b.iter(|| encode(&payload, encoding_id))
        });

        // Encode once for decoding benchmark
        let encoded = encode(&payload, encoding_id).expect("encoding should work");

        c.bench_function(&format!("decode_{}", name), |b| {
            b.iter(|| decode(black_box(&encoded), encoding_id))
        });
    }
}

fn benchmark_compression(c: &mut Criterion) {
    let small_payload = black_box(b"hello world this is a test".to_vec());
    let medium_payload = black_box((0..100u8).cycle().take(5_000).collect::<Vec<u8>>());
    let large_payload = black_box((0..=255u8).cycle().take(100_000).collect::<Vec<u8>>());

    c.bench_function("compress_zstd_small", |b| {
        b.iter(|| compress(&small_payload, CompressionKind::Zstd))
    });

    c.bench_function("compress_zstd_medium", |b| {
        b.iter(|| compress(&medium_payload, CompressionKind::Zstd))
    });

    c.bench_function("compress_zstd_large", |b| {
        b.iter(|| compress(&large_payload, CompressionKind::Zstd))
    });

    c.bench_function("compress_lz4_small", |b| {
        b.iter(|| compress(&small_payload, CompressionKind::Lz4))
    });

    c.bench_function("compress_lz4_medium", |b| {
        b.iter(|| compress(&medium_payload, CompressionKind::Lz4))
    });

    c.bench_function("compress_lz4_large", |b| {
        b.iter(|| compress(&large_payload, CompressionKind::Lz4))
    });

    // Decompression benchmarks
    let zstd_compressed =
        compress(&medium_payload, CompressionKind::Zstd).expect("compression should work");
    let lz4_compressed =
        compress(&medium_payload, CompressionKind::Lz4).expect("compression should work");

    c.bench_function("decompress_zstd_medium", |b| {
        b.iter(|| decompress(black_box(&zstd_compressed), CompressionKind::Zstd))
    });

    c.bench_function("decompress_lz4_medium", |b| {
        b.iter(|| decompress(black_box(&lz4_compressed), CompressionKind::Lz4))
    });
}

fn benchmark_encryption(c: &mut Criterion) {
    let master_key = b"super-secret-key-for-testing";
    let config = EncryptionConfig {
        column_name: "test_column".to_string(),
        schema_fingerprint: [1, 2, 3, 4, 5, 6, 7, 8],
    };
    let key = derive_column_key(master_key, &config).expect("key derivation should work");

    let small_payload = black_box(b"hello".to_vec());
    let medium_payload = black_box((0..100u8).cycle().take(5_000).collect::<Vec<u8>>());

    c.bench_function("encrypt_aes256_gcm_small", |b| {
        b.iter(|| encrypt_payload(&small_payload, &key))
    });

    c.bench_function("encrypt_aes256_gcm_medium", |b| {
        b.iter(|| encrypt_payload(&medium_payload, &key))
    });

    c.bench_function("derive_column_key", |b| {
        b.iter(|| derive_column_key(master_key, &config))
    });

    // Decryption benchmark
    let encrypted = encrypt_payload(&medium_payload, &key).expect("encryption should work");

    c.bench_function("decrypt_aes256_gcm_medium", |b| {
        b.iter(|| {
            decrypt_payload(
                black_box(&encrypted.ciphertext),
                &key,
                &encrypted.nonce,
                &encrypted.auth_tag,
            )
        })
    });
}

fn benchmark_writer(c: &mut Criterion) {
    let schema = SchemaBuilder::new()
        .add_field("device_id", FieldKind::Utf8, true)
        .add_field("status", FieldKind::Int32, false)
        .build()
        .expect("schema should build");
    let rows = black_box(vec![vec![1, 2], vec![3, 4], vec![5, 6], vec![7, 8]]);

    c.bench_function("writer_write_row_group_small", |b| {
        b.iter_batched(
            || StreamingWriter::new(schema.clone()),
            |mut writer| {
                writer
                    .write_row_group(black_box(&rows))
                    .expect("row group write should work");
                black_box(writer)
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function("writer_finish_small", |b| {
        b.iter_batched(
            || {
                let mut writer = StreamingWriter::new(schema.clone());
                writer
                    .write_row_group(&rows)
                    .expect("row group write should work");
                writer
            },
            |mut writer| black_box(writer.finish().expect("writer should finish")),
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    benches,
    benchmark_crc32,
    benchmark_schema_fingerprint,
    benchmark_encodings,
    benchmark_compression,
    benchmark_encryption,
    benchmark_writer
);
criterion_main!(benches);
