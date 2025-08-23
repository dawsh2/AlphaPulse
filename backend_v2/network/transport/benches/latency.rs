//! Latency Benchmarks for Transport System
//!
//! Measures end-to-end latency for different transport configurations
//! and protocols to validate performance requirements.

use alphapulse_transport::{
    CompressionEngine, CompressionType, EncryptionType, NetworkConfig, NetworkEnvelope,
    NetworkTransport, SecurityLayer,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;
use tokio::runtime::Runtime;

/// Benchmark message envelope serialization/deserialization
fn bench_envelope_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("envelope_serialization");

    let small_payload = vec![0u8; 64]; // 64 bytes
    let medium_payload = vec![0u8; 1024]; // 1KB
    let large_payload = vec![0u8; 8192]; // 8KB

    for (size_name, payload) in [
        ("64B", &small_payload),
        ("1KB", &medium_payload),
        ("8KB", &large_payload),
    ] {
        let envelope = NetworkEnvelope::new(
            "node1".to_string(),
            "node2".to_string(),
            "test_actor".to_string(),
            payload.clone(),
            CompressionType::None,
            EncryptionType::None,
        );

        group.benchmark_with_input(
            BenchmarkId::new("serialize", size_name),
            &envelope,
            |b, envelope| b.iter(|| black_box(envelope.to_bytes().unwrap())),
        );

        let serialized = envelope.to_bytes().unwrap();
        group.benchmark_with_input(
            BenchmarkId::new("deserialize", size_name),
            &serialized,
            |b, data| b.iter(|| black_box(NetworkEnvelope::from_bytes(data).unwrap())),
        );
    }

    group.finish();
}

/// Benchmark compression algorithms
#[cfg(feature = "compression")]
fn bench_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");

    // Test data - repetitive data that compresses well
    let test_data = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(100);
    let test_bytes = test_data.as_bytes();

    let compression_types = [
        CompressionType::None,
        CompressionType::Lz4,
        CompressionType::Zstd,
        CompressionType::Snappy,
    ];

    for compression_type in compression_types {
        let engine = CompressionEngine::new(compression_type);
        let type_name = format!("{:?}", compression_type);

        group.benchmark_with_input(
            BenchmarkId::new("compress", &type_name),
            &engine,
            |b, engine| b.iter(|| black_box(engine.compress(test_bytes).unwrap())),
        );

        if !matches!(compression_type, CompressionType::None) {
            let compressed = engine.compress(test_bytes).unwrap();
            group.benchmark_with_input(
                BenchmarkId::new("decompress", &type_name),
                &(engine, compressed),
                |b, (engine, compressed)| {
                    b.iter(|| black_box(engine.decompress(compressed).unwrap()))
                },
            );
        }
    }

    group.finish();
}

/// Benchmark encryption algorithms
#[cfg(feature = "encryption")]
fn bench_encryption(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("encryption");

    let test_data = vec![0u8; 1024]; // 1KB of test data

    // Test ChaCha20Poly1305 encryption
    let key = alphapulse_transport::SecurityLayer::generate_chacha_key();
    let encryption_type = EncryptionType::ChaCha20Poly1305 { key };

    let security_layer = rt.block_on(async { SecurityLayer::new(encryption_type).await.unwrap() });

    group.benchmark_function("chacha20poly1305_encrypt", |b| {
        b.to_async(&rt)
            .iter(|| async { black_box(security_layer.encrypt(&test_data).await.unwrap()) })
    });

    let encrypted_data = rt.block_on(async { security_layer.encrypt(&test_data).await.unwrap() });

    group.benchmark_function("chacha20poly1305_decrypt", |b| {
        b.to_async(&rt)
            .iter(|| async { black_box(security_layer.decrypt(&encrypted_data).await.unwrap()) })
    });

    group.finish();
}

/// Benchmark end-to-end message processing
fn bench_end_to_end_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("end_to_end");
    group.measurement_time(Duration::from_secs(10));

    let test_data = vec![0u8; 1024];

    // Test different configurations
    let configs = [
        (
            "no_compression_no_encryption",
            CompressionType::None,
            EncryptionType::None,
        ),
        #[cfg(feature = "compression")]
        (
            "lz4_no_encryption",
            CompressionType::Lz4,
            EncryptionType::None,
        ),
        #[cfg(all(feature = "compression", feature = "encryption"))]
        (
            "lz4_with_encryption",
            CompressionType::Lz4,
            EncryptionType::ChaCha20Poly1305 {
                key: alphapulse_transport::SecurityLayer::generate_chacha_key(),
            },
        ),
    ];

    for (config_name, compression, encryption) in configs {
        group.benchmark_function(config_name, |b| {
            b.to_async(&rt).iter(|| async {
                // Create envelope
                let envelope = NetworkEnvelope::new(
                    "node1".to_string(),
                    "node2".to_string(),
                    "test_actor".to_string(),
                    test_data.clone(),
                    compression,
                    encryption.clone(),
                );

                // Create compression and security layers
                let compression_engine = CompressionEngine::new(compression);
                let security_layer = SecurityLayer::new(encryption.clone()).await.unwrap();

                // Simulate full processing pipeline

                // 1. Compress
                let compressed = compression_engine.compress(&test_data).unwrap();

                // 2. Encrypt
                let encrypted = security_layer.encrypt(&compressed).await.unwrap();

                // 3. Serialize envelope
                let mut envelope_with_encrypted = envelope.clone();
                envelope_with_encrypted.payload = encrypted;
                let serialized = envelope_with_encrypted.to_bytes().unwrap();

                // 4. Deserialize envelope
                let deserialized = NetworkEnvelope::from_bytes(&serialized).unwrap();

                // 5. Decrypt
                let decrypted = security_layer.decrypt(&deserialized.payload).await.unwrap();

                // 6. Decompress
                let decompressed = compression_engine.decompress(&decrypted).unwrap();

                black_box(decompressed)
            })
        });
    }

    group.finish();
}

/// Benchmark message throughput
fn bench_message_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("throughput");
    group.measurement_time(Duration::from_secs(15));

    let message_sizes = [64, 256, 1024, 4096]; // Different message sizes
    let batch_sizes = [1, 10, 100]; // Different batch sizes

    for message_size in message_sizes {
        for batch_size in batch_sizes {
            let test_data = vec![0u8; message_size];
            let batch_id = format!("{}B_batch_{}", message_size, batch_size);

            group.benchmark_function(&batch_id, |b| {
                b.to_async(&rt).iter(|| async {
                    let mut results = Vec::with_capacity(batch_size);

                    for _ in 0..batch_size {
                        let envelope = NetworkEnvelope::new(
                            "node1".to_string(),
                            "node2".to_string(),
                            "test_actor".to_string(),
                            test_data.clone(),
                            CompressionType::None,
                            EncryptionType::None,
                        );

                        let serialized = envelope.to_bytes().unwrap();
                        let deserialized = NetworkEnvelope::from_bytes(&serialized).unwrap();
                        results.push(deserialized);
                    }

                    black_box(results)
                })
            });
        }
    }

    group.finish();
}

/// Benchmark network configuration creation
fn bench_config_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("config_creation");

    group.benchmark_function("default_config", |b| {
        b.iter(|| black_box(NetworkConfig::default()))
    });

    group.benchmark_function("ultra_low_latency_config", |b| {
        b.iter(|| black_box(NetworkConfig::ultra_low_latency()))
    });

    group.benchmark_function("high_throughput_config", |b| {
        b.iter(|| black_box(NetworkConfig::high_throughput()))
    });

    group.benchmark_function("secure_config", |b| {
        b.iter(|| black_box(NetworkConfig::secure()))
    });

    group.benchmark_function("network_transport_creation", |b| {
        b.to_async(&rt).iter(|| async {
            let config = NetworkConfig::default();
            black_box(NetworkTransport::new(config).await.unwrap())
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_envelope_serialization,
    bench_end_to_end_processing,
    bench_message_throughput,
    bench_config_creation
);

#[cfg(feature = "compression")]
criterion_group!(compression_benches, bench_compression);

#[cfg(feature = "encryption")]
criterion_group!(encryption_benches, bench_encryption);

// Conditional compilation for feature-specific benchmarks
#[cfg(all(feature = "compression", feature = "encryption"))]
criterion_main!(benches, compression_benches, encryption_benches);

#[cfg(all(feature = "compression", not(feature = "encryption")))]
criterion_main!(benches, compression_benches);

#[cfg(all(not(feature = "compression"), feature = "encryption"))]
criterion_main!(benches, encryption_benches);

#[cfg(all(not(feature = "compression"), not(feature = "encryption")))]
criterion_main!(benches);
