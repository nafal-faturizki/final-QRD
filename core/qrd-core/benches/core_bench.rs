use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_crc32(c: &mut Criterion) {
    c.bench_function("crc32_small_payload", |b| {
        b.iter(|| {
            let payload = black_box(b"123456789".to_vec());
            qrd_core::integrity::crc32(&payload)
        })
    });
}

fn benchmark_schema_fingerprint(c: &mut Criterion) {
    c.bench_function("schema_fingerprint", |b| {
        b.iter(|| {
            let schema = qrd_core::schema::SchemaBuilder::new()
                .add_field("device_id", qrd_core::schema::FieldKind::Utf8, true)
                .add_field("temperature", qrd_core::schema::FieldKind::Float32, false)
                .build()
                .expect("schema should build");
            black_box(schema.fingerprint())
        })
    });
}

criterion_group!(benches, benchmark_crc32, benchmark_schema_fingerprint);
criterion_main!(benches);