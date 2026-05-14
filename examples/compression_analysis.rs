#!/usr/bin/env rust-script
// Quick compression analysis script
// Run with: cargo run --release --example compression_analysis

use std::time::Instant;

fn main() {
    // Test with different data patterns
    analyze_random_data();
    analyze_repetitive_data();
    analyze_mixed_data();
}

fn analyze_random_data() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║           Random Data Compression Analysis (1MB)              ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    
    let data: Vec<u8> = (0..1024*1024).map(|i| ((i ^ 0xAAAA) % 256) as u8).collect();
    analyze_compression(&data);
}

fn analyze_repetitive_data() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║         Repetitive Data Compression Analysis (1MB)            ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    
    let data = vec![42u8; 1024*1024];
    analyze_compression(&data);
}

fn analyze_mixed_data() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║            Mixed Data Compression Analysis (1MB)              ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    
    let mut data = Vec::new();
    // Mix of repetitive and random patterns
    for i in 0..256 {
        data.extend_from_slice(&[i as u8; 256]);
        data.extend_from_slice(&(0..256).map(|j| ((j ^ i) % 256) as u8).collect::<Vec<_>>());
    }
    analyze_compression(&data);
}

fn analyze_compression(data: &[u8]) {
    println!("\nData size: {} bytes ({} MB)\n", data.len(), data.len() / 1024 / 1024);
    println!("{:<8} {:<12} {:<12} {:<15}", "Level", "Ratio %", "Time (ms)", "Speed (MB/s)");
    println!("{:-<47}", "");
    
    let levels = [1, 2, 3, 4, 5, 6, 8, 10, 15, 19];
    
    for level in levels.iter() {
        let start = Instant::now();
        
        match zstd::encode_all(data, *level as i32) {
            Ok(compressed) => {
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                let ratio = (compressed.len() as f64 / data.len() as f64) * 100.0;
                let speed = if elapsed > 0.0 {
                    (data.len() as f64 / (elapsed / 1000.0)) / 1_000_000.0
                } else {
                    0.0
                };
                
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
    
    // LZ4 comparison
    let start = Instant::now();
    match lz4::block::compress(data, Some(lz4::block::CompressionMode::DEFAULT), true) {
        Ok(compressed) => {
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            let ratio = (compressed.len() as f64 / data.len() as f64) * 100.0;
            let speed = if elapsed > 0.0 {
                (data.len() as f64 / (elapsed / 1000.0)) / 1_000_000.0
            } else {
                0.0
            };
            println!(
                "{:<8} {:<12.2}% {:<12.4} {:<15.2}",
                "LZ4", ratio, elapsed, speed
            );
        }
        Err(e) => {
            println!("{:<8} {:<12} {:<12} Error: {}", "LZ4", "N/A", "N/A", e);
        }
    }
}
