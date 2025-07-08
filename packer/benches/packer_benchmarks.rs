//! Performance benchmarks for Chronicle packer service

use std::sync::Arc;
use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use tempfile::TempDir;
use tokio::runtime::Runtime;

use chronicle_packer::{
    config::{PackerConfig, StorageConfig},
    storage::{StorageManager, ChronicleEvent},
    encryption::EncryptionService,
    integrity::IntegrityService,
    metrics::MetricsCollector,
};

/// Create a benchmark configuration
fn create_benchmark_config() -> (PackerConfig, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let mut config = PackerConfig::default();
    
    config.storage.base_path = temp_dir.path().to_path_buf();
    config.storage.compression_level = 1; // Faster compression for benchmarks
    config.encryption.enabled = false; // Disable for pure performance testing
    config.metrics.enabled = false; // Disable to avoid overhead
    
    (config, temp_dir)
}

/// Create test events for benchmarking
fn create_benchmark_events(count: usize) -> Vec<ChronicleEvent> {
    let mut events = Vec::with_capacity(count);
    let base_timestamp = 1640995200000000000u64;
    
    for i in 0..count {
        events.push(ChronicleEvent {
            timestamp_ns: base_timestamp + (i as u64 * 1000000),
            event_type: match i % 4 {
                0 => "key",
                1 => "mouse", 
                2 => "window",
                _ => "network",
            }.to_string(),
            app_bundle_id: Some(format!("com.benchmark.app{}", i % 10)),
            window_title: Some(format!("Benchmark Window {}", i)),
            data: format!(
                r#"{{"index": {}, "type": "benchmark", "payload": "{}"}}"#, 
                i, 
                "x".repeat(100) // 100 character payload
            ),
            session_id: "benchmark_session".to_string(),
            event_id: format!("bench_event_{:08}", i),
        });
    }
    
    events
}

/// Benchmark event processing throughput
fn bench_event_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("event_processing");
    
    for size in &[100, 1000, 10000, 50000] {
        group.throughput(Throughput::Elements(*size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("process_events", size),
            size,
            |b, &size| {
                let (config, _temp_dir) = create_benchmark_config();
                let events = create_benchmark_events(size);
                
                b.to_async(&rt).iter(|| async {
                    let integrity = Arc::new(IntegrityService::new());
                    let mut storage = StorageManager::new(
                        config.storage.clone(),
                        None,
                        integrity,
                    ).unwrap();
                    
                    let date = chrono::Utc::now();
                    let result = storage.write_events_to_parquet(&events, &date).await;
                    black_box(result.unwrap());
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark Parquet file writing
fn bench_parquet_writing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("parquet_writing");
    
    for size in &[1000, 5000, 25000, 100000] {
        group.throughput(Throughput::Elements(*size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("write_parquet", size),
            size,
            |b, &size| {
                let (config, _temp_dir) = create_benchmark_config();
                let events = create_benchmark_events(size);
                
                b.to_async(&rt).iter(|| async {
                    let integrity = Arc::new(IntegrityService::new());
                    let mut storage = StorageManager::new(
                        config.storage.clone(),
                        None,
                        integrity,
                    ).unwrap();
                    
                    let date = chrono::Utc::now();
                    let result = storage.write_events_to_parquet(&events, &date).await;
                    black_box(result.unwrap());
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark encryption performance
fn bench_encryption(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("encryption");
    
    // Test different data sizes
    for size in &[1024, 10240, 102400, 1048576] { // 1KB to 1MB
        group.throughput(Throughput::Bytes(*size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("encrypt", size),
            size,
            |b, &size| {
                let data = vec![0u8; size];
                
                b.to_async(&rt).iter(|| async {
                    let (config, _temp_dir) = create_benchmark_config();
                    let mut encryption_config = config.encryption.clone();
                    encryption_config.enabled = true;
                    encryption_config.kdf_iterations = 1000; // Faster for benchmarks
                    
                    let mut encryption = EncryptionService::new(encryption_config).unwrap();
                    let result = encryption.encrypt(&data);
                    black_box(result.unwrap());
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("decrypt", size),
            size,
            |b, &size| {
                let data = vec![0u8; size];
                
                b.to_async(&rt).iter(|| async {
                    let (config, _temp_dir) = create_benchmark_config();
                    let mut encryption_config = config.encryption.clone();
                    encryption_config.enabled = true;
                    encryption_config.kdf_iterations = 1000;
                    
                    let mut encryption = EncryptionService::new(encryption_config).unwrap();
                    let encrypted = encryption.encrypt(&data).unwrap();
                    let result = encryption.decrypt(&encrypted);
                    black_box(result.unwrap());
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark data validation
fn bench_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation");
    
    for size in &[100, 1000, 10000, 50000] {
        group.throughput(Throughput::Elements(*size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("validate_events", size),
            size,
            |b, &size| {
                let events = create_benchmark_events(size);
                let integrity = IntegrityService::new();
                
                b.iter(|| {
                    let result = integrity.validate_chronicle_events(&events);
                    black_box(result.unwrap());
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("temporal_consistency", size),
            size,
            |b, &size| {
                let events = create_benchmark_events(size);
                let integrity = IntegrityService::new();
                
                b.iter(|| {
                    let result = integrity.check_temporal_consistency(&events);
                    black_box(result.unwrap());
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark checksum calculation
fn bench_checksums(c: &mut Criterion) {
    let mut group = c.benchmark_group("checksums");
    
    for size in &[1024, 10240, 102400, 1048576] { // 1KB to 1MB
        group.throughput(Throughput::Bytes(*size as u64));
        
        let data = vec![0u8; *size];
        
        group.bench_with_input(
            BenchmarkId::new("blake3", size),
            &data,
            |b, data| {
                let integrity = IntegrityService::new();
                
                b.iter(|| {
                    let result = integrity.calculate_checksum(data);
                    black_box(result.unwrap());
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("sha256", size),
            &data,
            |b, data| {
                let integrity = IntegrityService::with_algorithm(
                    chronicle_packer::integrity::ChecksumAlgorithm::Sha256
                );
                
                b.iter(|| {
                    let result = integrity.calculate_checksum(data);
                    black_box(result.unwrap());
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark metrics collection
fn bench_metrics(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("metrics");
    
    group.bench_function("record_metrics", |b| {
        b.to_async(&rt).iter(|| async {
            let (config, _temp_dir) = create_benchmark_config();
            let mut metrics_config = config.metrics.clone();
            metrics_config.enabled = true;
            metrics_config.port = 0; // Disable server
            
            let metrics = MetricsCollector::new(metrics_config).unwrap();
            
            // Record various metrics
            for i in 0..1000 {
                metrics.record_event_processed(1);
                if i % 10 == 0 {
                    metrics.record_event_failed(1);
                }
                if i % 100 == 0 {
                    metrics.record_file_created(1024);
                }
                metrics.record_processing_duration(Duration::from_millis(i % 100));
            }
            
            black_box(metrics.get_packer_stats());
        });
    });
    
    group.bench_function("export_prometheus", |b| {
        b.to_async(&rt).iter(|| async {
            let (config, _temp_dir) = create_benchmark_config();
            let mut metrics_config = config.metrics.clone();
            metrics_config.enabled = true;
            metrics_config.port = 0;
            
            let metrics = MetricsCollector::new(metrics_config).unwrap();
            
            // Add some data
            metrics.record_event_processed(1000);
            metrics.record_file_created(2048);
            
            let result = metrics.export_metrics("prometheus");
            black_box(result.unwrap());
        });
    });
    
    group.finish();
}

/// Benchmark memory usage patterns
fn bench_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("memory_usage");
    
    // Benchmark batch processing of different sizes
    for batch_size in &[1000, 5000, 10000, 25000] {
        group.bench_with_input(
            BenchmarkId::new("batch_processing", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let (config, _temp_dir) = create_benchmark_config();
                    let integrity = Arc::new(IntegrityService::new());
                    let mut storage = StorageManager::new(
                        config.storage.clone(),
                        None,
                        integrity,
                    ).unwrap();
                    
                    // Process events in batches
                    let total_events = 50000;
                    let num_batches = total_events / batch_size;
                    
                    for batch in 0..num_batches {
                        let events = create_benchmark_events(batch_size);
                        let date = chrono::Utc::now() + chrono::Duration::seconds(batch as i64);
                        
                        let result = storage.write_events_to_parquet(&events, &date).await;
                        black_box(result.unwrap());
                    }
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_operations");
    
    for num_tasks in &[2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("concurrent_writes", num_tasks),
            num_tasks,
            |b, &num_tasks| {
                b.to_async(&rt).iter(|| async {
                    let (config, _temp_dir) = create_benchmark_config();
                    let integrity = Arc::new(IntegrityService::new());
                    let storage = Arc::new(tokio::sync::RwLock::new(
                        StorageManager::new(config.storage.clone(), None, integrity).unwrap()
                    ));
                    
                    let mut handles = Vec::new();
                    
                    for i in 0..num_tasks {
                        let storage = storage.clone();
                        let events = create_benchmark_events(1000);
                        
                        let handle = tokio::spawn(async move {
                            let date = chrono::Utc::now() + chrono::Duration::seconds(i as i64);
                            let mut storage = storage.write().await;
                            storage.write_events_to_parquet(&events, &date).await
                        });
                        
                        handles.push(handle);
                    }
                    
                    let results = futures::future::join_all(handles).await;
                    for result in results {
                        black_box(result.unwrap().unwrap());
                    }
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_event_processing,
    bench_parquet_writing,
    bench_encryption,
    bench_validation,
    bench_checksums,
    bench_metrics,
    bench_memory_usage,
    bench_concurrent_operations
);

criterion_main!(benches);