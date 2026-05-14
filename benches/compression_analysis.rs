/// Comprehensive compression analysis for optimization
/// Tests various ZSTD levels and LZ4 variants to find optimal settings

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Instant;

/// Analyzes compression with different ZSTD levels
fn analyze_zstd_levels(data: &[u8]) {
    println!("\n=== ZSTD Level Analysis ===");
    println!("{:<8} {:<12} {:<12} {:<15}", "Level", "Ratio %", "Time (ms)", "Speed (MB/s)");
    println!("{:-<47}", "");
    
    for level in [1, 2, 3, 4, 5, 6, 10, 15, 19] {
        let start = Instant::now();
        match zstd::encode_all(data, level) {
            Ok(compressed) => {
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                let ratio = (compressed.len() as f64 / data.len() as f64) * 100.0;
                let speed = (data.len() as f64 / elapsed / 1_000_000.0) * 1000.0; // MB/s
                println!(
                    "{:<8} {:<12.2}% {:<12.4} {:<15.2}",
                    level, ratio, elapsed, speed
                );
            }
            Err(e) => {
                println!("{:<8} {:<12} {:<12} Error: {}", level, "N/A", "N/A", e);
            }
        }
    }
}

/// Test with various data sizes
fn analyze_by_data_size() {
    println!("\n=== Compression by Data Size ===");
    
    let sizes = vec![
        (1024, "1 KB"),
        (10 * 1024, "10 KB"),
        (100 * 1024, "100 KB"),
        (1024 * 1024, "1 MB"),
        (10 * 1024 * 1024, "10 MB"),
    ];
    
    for (size, label) in sizes {
        let data = vec![42u8; size]; // Repetitive data for good compression
        println!("\nData size: {}", label);
        analyze_zstd_levels(&data);
    }
}

/// Benchmark ZSTD compression throughput at different levels
fn benchmark_zstd(c: &mut Criterion) {
    let payload_1mb = black_box(vec![42u8; 1_024 * 1024]);
    let payload_10mb = black_box(vec![42u8; 10 * 1024 * 1024]);
    
    let mut group = c.benchmark_group("zstd_levels");
    group.sample_size(10); // Reduce samples for large data
    
    for level in [1, 3, 5, 10, 19].iter() {
        // 1MB benchmark
        group.bench_with_input(BenchmarkId::new("1mb", level), level, |b, &level| {
            b.iter(|| {
                zstd::encode_all(&payload_1mb, level as i32)
                    .expect("compression should work")
            })
        });
        
        // 10MB benchmark (sample_size=1)
        if *level == 3 || *level == 10 {
            group.sample_size(1);
            group.bench_with_input(BenchmarkId::new("10mb", level), level, |b, &level| {
                b.iter(|| {
                    zstd::encode_all(&payload_10mb, level as i32)
                        .expect("compression should work")
                })
            });
            group.sample_size(10);
        }
    }
    
    group.finish();
}

/// Analyze LZ4 performance
fn analyze_lz4(data: &[u8]) {
    println!("\n=== LZ4 Compression Analysis ===");
    
    let start = Instant::now();
    let compressed = lz4::block::compress(
        data,
        Some(lz4::block::CompressionMode::DEFAULT),
        true
    ).expect("LZ4 compression should work");
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    let ratio = (compressed.len() as f64 / data.len() as f64) * 100.0;
    let speed = (data.len() as f64 / elapsed / 1_000_000.0) * 1000.0;
    
    println!("Size: {} bytes", data.len());
    println!("Compressed: {} bytes", compressed.len());
    println!("Ratio: {:.2}%", ratio);
    println!("Time: {:.4} ms", elapsed);
    println!("Speed: {:.2} MB/s", speed);
}

/// Compare current (level 3) with alternatives
fn benchmark_current_vs_alternatives(c: &mut Criterion) {
    let payload = black_box(vec![42u8; 1_024 * 1024]);
    
    let mut group = c.benchmark_group("compression_comparison");
    
    group.bench_function("zstd_level_3_current", |b| {
        b.iter(|| {
            zstd::encode_all(&payload, 3)
                .expect("compression should work")
        })
    });
    
    group.bench_function("zstd_level_5_alternative", |b| {
        b.iter(|| {
            zstd::encode_all(&payload, 5)
                .expect("compression should work")
        })
    });
    
    group.bench_function("lz4_current", |b| {
        b.iter(|| {
            lz4::block::compress(&payload, Some(lz4::block::CompressionMode::DEFAULT), true)
                .expect("LZ4 compression should work")
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_zstd,
    benchmark_current_vs_alternatives
);
criterion_main!(benches);
