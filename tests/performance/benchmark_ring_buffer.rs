use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::time::Duration;
use tokio::runtime::Runtime;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::json;

use crate::mocks::MockRingBuffer;
use crate::utils::TestEvent;

/// Ring buffer performance benchmarks
pub fn benchmark_ring_buffer_write(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("ring_buffer_write");
    
    // Test different buffer sizes
    for buffer_size in [64 * 1024, 256 * 1024, 1024 * 1024, 4 * 1024 * 1024].iter() {
        group.throughput(Throughput::Bytes(*buffer_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("single_write", buffer_size),
            buffer_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let ring_buffer = MockRingBuffer::new(size).unwrap();
                    let event = create_test_event(1, 100);
                    
                    black_box(ring_buffer.write_event(&event).await.unwrap());
                });
            },
        );
    }
    
    group.finish();
}

pub fn benchmark_ring_buffer_read(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("ring_buffer_read");
    
    // Test different read batch sizes
    for batch_size in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("batch_read", batch_size),
            batch_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let ring_buffer = MockRingBuffer::new(1024 * 1024).unwrap();
                    
                    // Pre-fill buffer
                    for i in 0..size {
                        let event = create_test_event(i as u64, 100);
                        ring_buffer.write_event(&event).await.unwrap();
                    }
                    
                    let events = black_box(ring_buffer.read_events(size).await.unwrap());
                    assert_eq!(events.len(), size);
                });
            },
        );
    }
    
    group.finish();
}

pub fn benchmark_ring_buffer_concurrent_access(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("ring_buffer_concurrent");
    
    // Test different numbers of concurrent operations
    for num_threads in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_write", num_threads),
            num_threads,
            |b, &threads| {
                b.to_async(&rt).iter(|| async {
                    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024).unwrap()));
                    
                    let handles = (0..threads).map(|i| {
                        let ring_buffer = ring_buffer.clone();
                        tokio::spawn(async move {
                            let event = create_test_event(i as u64, 100);
                            let mut buffer = ring_buffer.write().await;
                            buffer.write_event(&event).await.unwrap();
                        })
                    }).collect::<Vec<_>>();
                    
                    for handle in handles {
                        black_box(handle.await.unwrap());
                    }
                });
            },
        );
    }
    
    group.finish();
}

pub fn benchmark_ring_buffer_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("ring_buffer_memory");
    
    // Test different event sizes
    for event_size in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Bytes(*event_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("memory_efficiency", event_size),
            event_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let ring_buffer = MockRingBuffer::new(1024 * 1024).unwrap();
                    
                    // Fill buffer with events of specified size
                    for i in 0..100 {
                        let event = create_test_event(i, size);
                        ring_buffer.write_event(&event).await.unwrap();
                    }
                    
                    let memory_usage = black_box(ring_buffer.get_memory_usage());
                    assert!(memory_usage > 0);
                });
            },
        );
    }
    
    group.finish();
}

pub fn benchmark_ring_buffer_overflow_handling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("ring_buffer_overflow");
    
    group.bench_function("overflow_performance", |b| {
        b.to_async(&rt).iter(|| async {
            // Create small buffer to force overflow
            let ring_buffer = MockRingBuffer::new(1024).unwrap();
            
            // Write many events to trigger overflow
            for i in 0..1000 {
                let event = create_test_event(i, 100);
                black_box(ring_buffer.write_event(&event).await.unwrap());
            }
            
            let stats = ring_buffer.get_stats();
            assert!(stats.overflow_count > 0);
        });
    });
    
    group.finish();
}

pub fn benchmark_ring_buffer_persistence(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("ring_buffer_persistence");
    
    group.bench_function("persistence_write", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = tempfile::TempDir::new().unwrap();
            let persistence_path = temp_dir.path().join("benchmark_state");
            
            let ring_buffer = MockRingBuffer::with_persistence(
                1024 * 1024, 
                persistence_path.clone()
            ).unwrap();
            
            // Write events
            for i in 0..100 {
                let event = create_test_event(i, 100);
                ring_buffer.write_event(&event).await.unwrap();
            }
            
            // Benchmark persistence operation
            black_box(ring_buffer.persist().await.unwrap());
        });
    });
    
    group.bench_function("persistence_read", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = tempfile::TempDir::new().unwrap();
            let persistence_path = temp_dir.path().join("benchmark_state");
            
            // Create and populate buffer
            let ring_buffer = MockRingBuffer::with_persistence(
                1024 * 1024, 
                persistence_path.clone()
            ).unwrap();
            
            for i in 0..100 {
                let event = create_test_event(i, 100);
                ring_buffer.write_event(&event).await.unwrap();
            }
            ring_buffer.persist().await.unwrap();
            
            // Benchmark restoration
            black_box(MockRingBuffer::from_persistence(persistence_path).unwrap());
        });
    });
    
    group.finish();
}

pub fn benchmark_ring_buffer_search(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("ring_buffer_search");
    
    // Test different search patterns
    for num_events in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*num_events as u64));
        
        group.bench_with_input(
            BenchmarkId::new("linear_search", num_events),
            num_events,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let ring_buffer = MockRingBuffer::new(1024 * 1024).unwrap();
                    
                    // Populate buffer
                    for i in 0..count {
                        let event = create_test_event(i as u64, 100);
                        ring_buffer.write_event(&event).await.unwrap();
                    }
                    
                    // Search for specific event
                    let target_id = count / 2;
                    let result = black_box(ring_buffer.search_by_id(target_id as u64).await.unwrap());
                    assert!(result.is_some());
                });
            },
        );
    }
    
    group.finish();
}

pub fn benchmark_ring_buffer_compression(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("ring_buffer_compression");
    
    // Test different compression levels
    for compression_level in [1, 3, 6, 9].iter() {
        group.bench_with_input(
            BenchmarkId::new("compression", compression_level),
            compression_level,
            |b, &level| {
                b.to_async(&rt).iter(|| async {
                    let mut ring_buffer = MockRingBuffer::new(1024 * 1024).unwrap();
                    ring_buffer.set_compression_level(level);
                    
                    // Write compressible events
                    for i in 0..100 {
                        let event = create_compressible_event(i, 1000);
                        black_box(ring_buffer.write_event(&event).await.unwrap());
                    }
                    
                    let stats = ring_buffer.get_stats();
                    assert!(stats.compression_ratio > 0.0);
                });
            },
        );
    }
    
    group.finish();
}

fn create_test_event(id: u64, data_size: usize) -> TestEvent {
    TestEvent {
        id,
        timestamp: chrono::Utc::now(),
        event_type: "benchmark".to_string(),
        data: json!({
            "id": id,
            "data": "x".repeat(data_size),
            "benchmark": true
        }),
    }
}

fn create_compressible_event(id: u64, data_size: usize) -> TestEvent {
    TestEvent {
        id,
        timestamp: chrono::Utc::now(),
        event_type: "compression_benchmark".to_string(),
        data: json!({
            "id": id,
            "repeated_data": "ABCDEFGHIJKLMNOP".repeat(data_size / 16),
            "compression_test": true
        }),
    }
}

// Custom benchmark configuration
fn custom_criterion() -> Criterion {
    Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3))
        .with_plots()
}

criterion_group!(
    name = ring_buffer_benches;
    config = custom_criterion();
    targets = 
        benchmark_ring_buffer_write,
        benchmark_ring_buffer_read,
        benchmark_ring_buffer_concurrent_access,
        benchmark_ring_buffer_memory_usage,
        benchmark_ring_buffer_overflow_handling,
        benchmark_ring_buffer_persistence,
        benchmark_ring_buffer_search,
        benchmark_ring_buffer_compression
);

criterion_main!(ring_buffer_benches);

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_benchmark_helpers() {
        let event = create_test_event(1, 100);
        assert_eq!(event.id, 1);
        assert_eq!(event.event_type, "benchmark");
        
        let compressible = create_compressible_event(2, 160);
        assert_eq!(compressible.id, 2);
        assert_eq!(compressible.event_type, "compression_benchmark");
    }
}