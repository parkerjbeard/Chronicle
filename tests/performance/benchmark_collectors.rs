use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::time::Duration;
use tokio::runtime::Runtime;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::json;

use crate::mocks::{MockCollector, MockRingBuffer};
use crate::utils::TestEvent;

/// Collector performance benchmarks
pub fn benchmark_collector_initialization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("collector_initialization");
    
    // Test different collector types
    let collector_types = [
        "KeyTapCollector",
        "PointerMonCollector", 
        "ScreenTapCollector",
        "WindowMonCollector",
        "ClipMonCollector",
        "FSMonCollector",
        "NetMonCollector",
        "AudioMonCollector"
    ];
    
    for collector_type in collector_types.iter() {
        group.bench_with_input(
            BenchmarkId::new("init", collector_type),
            collector_type,
            |b, &col_type| {
                b.to_async(&rt).iter(|| async {
                    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024).unwrap()));
                    let mut collector = MockCollector::new(ring_buffer).unwrap();
                    collector.set_collector_type(col_type.to_string());
                    
                    black_box(collector.initialize().await.unwrap());
                });
            },
        );
    }
    
    group.finish();
}

pub fn benchmark_collector_event_generation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("collector_event_generation");
    
    // Test different event generation rates
    for events_per_second in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*events_per_second as u64));
        
        group.bench_with_input(
            BenchmarkId::new("generation_rate", events_per_second),
            events_per_second,
            |b, &rate| {
                b.to_async(&rt).iter(|| async {
                    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024).unwrap()));
                    let mut collector = MockCollector::new(ring_buffer).unwrap();
                    collector.set_event_rate(rate);
                    
                    // Run for 1 second
                    let start = std::time::Instant::now();
                    let handle = tokio::spawn(async move {
                        collector.start_collection().await
                    });
                    
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    handle.abort();
                    
                    black_box(start.elapsed());
                });
            },
        );
    }
    
    group.finish();
}

pub fn benchmark_collector_concurrent_collection(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("collector_concurrent");
    
    // Test different numbers of concurrent collectors
    for num_collectors in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_collectors", num_collectors),
            num_collectors,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(10 * 1024 * 1024).unwrap()));
                    
                    let collectors = (0..count).map(|i| {
                        let mut collector = MockCollector::new(ring_buffer.clone()).unwrap();
                        collector.set_id(format!("collector_{}", i));
                        collector.set_event_rate(100);
                        collector
                    }).collect::<Vec<_>>();
                    
                    let handles = collectors.into_iter().map(|collector| {
                        tokio::spawn(async move {
                            collector.start_collection().await
                        })
                    }).collect::<Vec<_>>();
                    
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    
                    for handle in handles {
                        handle.abort();
                    }
                    
                    let stats = ring_buffer.read().await.get_stats();
                    black_box(stats.total_writes);
                });
            },
        );
    }
    
    group.finish();
}

pub fn benchmark_collector_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("collector_memory");
    
    // Test different event sizes
    for event_size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Bytes(*event_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("memory_usage", event_size),
            event_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024).unwrap()));
                    let mut collector = MockCollector::new(ring_buffer.clone()).unwrap();
                    collector.set_event_size(size);
                    collector.set_event_rate(1000);
                    
                    let initial_memory = collector.get_memory_usage();
                    
                    let handle = tokio::spawn(async move {
                        collector.start_collection().await
                    });
                    
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    handle.abort();
                    
                    let final_memory = ring_buffer.read().await.get_memory_usage();
                    black_box(final_memory - initial_memory);
                });
            },
        );
    }
    
    group.finish();
}

pub fn benchmark_collector_filtering(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("collector_filtering");
    
    // Test different filter complexity
    for filter_rules in [1, 5, 10, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("filter_rules", filter_rules),
            filter_rules,
            |b, &rules| {
                b.to_async(&rt).iter(|| async {
                    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024).unwrap()));
                    let mut collector = MockCollector::new(ring_buffer.clone()).unwrap();
                    
                    // Set up filtering rules
                    for i in 0..rules {
                        collector.add_filter_rule(format!("event_type == 'test_{}'", i));
                    }
                    
                    collector.set_event_rate(1000);
                    
                    let handle = tokio::spawn(async move {
                        collector.start_collection().await
                    });
                    
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    handle.abort();
                    
                    let stats = ring_buffer.read().await.get_stats();
                    black_box(stats.filtered_events);
                });
            },
        );
    }
    
    group.finish();
}

pub fn benchmark_collector_serialization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("collector_serialization");
    
    // Test different serialization formats
    let formats = ["json", "msgpack", "cbor", "bincode"];
    
    for format in formats.iter() {
        group.bench_with_input(
            BenchmarkId::new("serialization", format),
            format,
            |b, &fmt| {
                b.to_async(&rt).iter(|| async {
                    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024).unwrap()));
                    let mut collector = MockCollector::new(ring_buffer.clone()).unwrap();
                    collector.set_serialization_format(fmt.to_string());
                    collector.set_event_rate(100);
                    
                    let handle = tokio::spawn(async move {
                        collector.start_collection().await
                    });
                    
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    handle.abort();
                    
                    let stats = ring_buffer.read().await.get_stats();
                    black_box(stats.serialization_time);
                });
            },
        );
    }
    
    group.finish();
}

pub fn benchmark_collector_error_handling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("collector_error_handling");
    
    // Test different error rates
    for error_rate in [0.0, 0.01, 0.05, 0.1].iter() {
        group.bench_with_input(
            BenchmarkId::new("error_rate", (error_rate * 100.0) as u32),
            error_rate,
            |b, &rate| {
                b.to_async(&rt).iter(|| async {
                    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024).unwrap()));
                    let mut collector = MockCollector::new(ring_buffer.clone()).unwrap();
                    collector.enable_error_simulation(rate);
                    collector.set_event_rate(1000);
                    
                    let handle = tokio::spawn(async move {
                        collector.start_collection().await
                    });
                    
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    handle.abort();
                    
                    let stats = ring_buffer.read().await.get_stats();
                    black_box(stats.error_recovery_time);
                });
            },
        );
    }
    
    group.finish();
}

pub fn benchmark_collector_privacy_filtering(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("collector_privacy");
    
    // Test different privacy levels
    let privacy_levels = ["none", "basic", "enhanced", "strict"];
    
    for level in privacy_levels.iter() {
        group.bench_with_input(
            BenchmarkId::new("privacy_level", level),
            level,
            |b, &priv_level| {
                b.to_async(&rt).iter(|| async {
                    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024).unwrap()));
                    let mut collector = MockCollector::new(ring_buffer.clone()).unwrap();
                    collector.set_privacy_level(priv_level.to_string());
                    collector.set_event_rate(1000);
                    
                    let handle = tokio::spawn(async move {
                        collector.start_collection().await
                    });
                    
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    handle.abort();
                    
                    let stats = ring_buffer.read().await.get_stats();
                    black_box(stats.privacy_filtered_events);
                });
            },
        );
    }
    
    group.finish();
}

pub fn benchmark_collector_batching(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("collector_batching");
    
    // Test different batch sizes
    for batch_size in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("batch_size", batch_size),
            batch_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024).unwrap()));
                    let mut collector = MockCollector::new(ring_buffer.clone()).unwrap();
                    collector.set_batch_size(size);
                    collector.set_event_rate(1000);
                    
                    let handle = tokio::spawn(async move {
                        collector.start_collection().await
                    });
                    
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    handle.abort();
                    
                    let stats = ring_buffer.read().await.get_stats();
                    black_box(stats.batched_writes);
                });
            },
        );
    }
    
    group.finish();
}

pub fn benchmark_collector_compression(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("collector_compression");
    
    // Test different compression algorithms
    let algorithms = ["none", "gzip", "zstd", "lz4"];
    
    for algorithm in algorithms.iter() {
        group.bench_with_input(
            BenchmarkId::new("compression", algorithm),
            algorithm,
            |b, &algo| {
                b.to_async(&rt).iter(|| async {
                    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024).unwrap()));
                    let mut collector = MockCollector::new(ring_buffer.clone()).unwrap();
                    collector.set_compression_algorithm(algo.to_string());
                    collector.set_event_rate(100);
                    collector.set_large_events(true); // Generate compressible events
                    
                    let handle = tokio::spawn(async move {
                        collector.start_collection().await
                    });
                    
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    handle.abort();
                    
                    let stats = ring_buffer.read().await.get_stats();
                    black_box(stats.compression_ratio);
                });
            },
        );
    }
    
    group.finish();
}

fn custom_criterion() -> Criterion {
    Criterion::default()
        .sample_size(50)
        .measurement_time(Duration::from_secs(15))
        .warm_up_time(Duration::from_secs(5))
        .with_plots()
}

criterion_group!(
    name = collector_benches;
    config = custom_criterion();
    targets = 
        benchmark_collector_initialization,
        benchmark_collector_event_generation,
        benchmark_collector_concurrent_collection,
        benchmark_collector_memory_usage,
        benchmark_collector_filtering,
        benchmark_collector_serialization,
        benchmark_collector_error_handling,
        benchmark_collector_privacy_filtering,
        benchmark_collector_batching,
        benchmark_collector_compression
);

criterion_main!(collector_benches);

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;
    
    #[test]
    fn test_collector_benchmark_setup() {
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024).unwrap()));
            let collector = MockCollector::new(ring_buffer).unwrap();
            
            assert!(collector.is_initialized());
        });
    }
}