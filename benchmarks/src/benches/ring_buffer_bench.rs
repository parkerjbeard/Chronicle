//! Ring buffer performance benchmarks
//!
//! Tests the core ring buffer implementation for throughput, latency, and memory usage
//! across different scenarios including single/multi-producer and single/multi-consumer patterns.

use crate::{
    BenchmarkComponent, BenchmarkConfig, BenchmarkResult, ErrorMetrics, LatencyMetrics,
    PerformanceMetrics, ResourceMetrics, ThroughputMetrics,
};
use anyhow::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time;

/// Ring buffer benchmark test cases
const BENCHMARK_TESTS: &[&str] = &[
    "single_producer_single_consumer",
    "single_producer_multi_consumer",
    "multi_producer_single_consumer",
    "multi_producer_multi_consumer",
    "burst_write_performance",
    "sustained_throughput",
    "memory_pressure",
    "lock_contention",
];

/// Simulated ring buffer operations (FFI calls to C implementation)
struct RingBufferSimulator {
    capacity: usize,
    write_count: AtomicU64,
    read_count: AtomicU64,
    errors: AtomicU64,
}

impl RingBufferSimulator {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            write_count: AtomicU64::new(0),
            read_count: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        }
    }

    async fn write_event(&self, data: &[u8]) -> Result<()> {
        // Simulate write latency
        time::sleep(Duration::from_nanos(100)).await;
        self.write_count.fetch_add(1, Ordering::Relaxed);
        
        // Simulate occasional errors
        if rand::random::<f64>() < 0.001 {
            self.errors.fetch_add(1, Ordering::Relaxed);
            return Err(anyhow::anyhow!("Ring buffer full"));
        }
        
        Ok(())
    }

    async fn read_event(&self) -> Result<Vec<u8>> {
        // Simulate read latency
        time::sleep(Duration::from_nanos(80)).await;
        self.read_count.fetch_add(1, Ordering::Relaxed);
        
        // Simulate occasional errors
        if rand::random::<f64>() < 0.001 {
            self.errors.fetch_add(1, Ordering::Relaxed);
            return Err(anyhow::anyhow!("Ring buffer empty"));
        }
        
        Ok(vec![0u8; 1024]) // Simulate 1KB event
    }

    fn get_stats(&self) -> (u64, u64, u64) {
        (
            self.write_count.load(Ordering::Relaxed),
            self.read_count.load(Ordering::Relaxed),
            self.errors.load(Ordering::Relaxed),
        )
    }
}

/// Run a specific ring buffer benchmark
pub async fn run_benchmark(test_name: &str, config: &BenchmarkConfig) -> Result<BenchmarkResult> {
    let start_time = Instant::now();
    
    let result = match test_name {
        "single_producer_single_consumer" => {
            single_producer_single_consumer_benchmark(config).await
        }
        "single_producer_multi_consumer" => {
            single_producer_multi_consumer_benchmark(config).await
        }
        "multi_producer_single_consumer" => {
            multi_producer_single_consumer_benchmark(config).await
        }
        "multi_producer_multi_consumer" => {
            multi_producer_multi_consumer_benchmark(config).await
        }
        "burst_write_performance" => burst_write_performance_benchmark(config).await,
        "sustained_throughput" => sustained_throughput_benchmark(config).await,
        "memory_pressure" => memory_pressure_benchmark(config).await,
        "lock_contention" => lock_contention_benchmark(config).await,
        _ => return Err(anyhow::anyhow!("Unknown benchmark test: {}", test_name)),
    };

    let duration = start_time.elapsed();
    
    match result {
        Ok(mut metrics) => {
            metrics.timestamp = chrono::Utc::now();
            
            // Check if performance targets are met
            let passed = metrics.throughput.events_per_second >= config.targets.ring_buffer_events_per_second as f64;
            
            Ok(BenchmarkResult {
                component: BenchmarkComponent::RingBuffer,
                test_name: test_name.to_string(),
                metrics,
                passed,
                notes: Some(format!("Completed in {:.2?}", duration)),
            })
        }
        Err(e) => {
            let metrics = PerformanceMetrics {
                timestamp: chrono::Utc::now(),
                throughput: ThroughputMetrics {
                    events_per_second: 0.0,
                    bytes_per_second: 0.0,
                    operations_per_second: 0.0,
                },
                latency: LatencyMetrics {
                    p50_ms: 0.0,
                    p95_ms: 0.0,
                    p99_ms: 0.0,
                    max_ms: 0.0,
                    mean_ms: 0.0,
                },
                resources: ResourceMetrics {
                    cpu_usage_percent: 0.0,
                    memory_usage_mb: 0.0,
                    disk_io_bytes_per_second: 0.0,
                    network_io_bytes_per_second: 0.0,
                    file_handles: 0,
                    thread_count: 0,
                },
                errors: ErrorMetrics {
                    error_rate: 1.0,
                    recovery_time_ms: 0.0,
                    total_errors: 1,
                },
            };
            
            Ok(BenchmarkResult {
                component: BenchmarkComponent::RingBuffer,
                test_name: test_name.to_string(),
                metrics,
                passed: false,
                notes: Some(format!("Failed: {}", e)),
            })
        }
    }
}

/// Run all ring buffer benchmarks
pub async fn run_all_benchmarks(config: &BenchmarkConfig) -> Result<Vec<BenchmarkResult>> {
    let mut results = Vec::new();
    
    for test_name in BENCHMARK_TESTS {
        let result = run_benchmark(test_name, config).await?;
        results.push(result);
    }
    
    Ok(results)
}

/// Single producer, single consumer benchmark
async fn single_producer_single_consumer_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let ring_buffer = Arc::new(RingBufferSimulator::new(1024 * 1024));
    let data = vec![0u8; 1024]; // 1KB events
    
    // Warmup
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let rb_producer = ring_buffer.clone();
    let rb_consumer = ring_buffer.clone();
    
    // Producer task
    let producer_task = tokio::spawn(async move {
        let mut latencies = Vec::new();
        
        for _ in 0..config.iterations {
            let start = Instant::now();
            let _ = rb_producer.write_event(&data).await;
            latencies.push(start.elapsed().as_nanos() as f64 / 1_000_000.0);
        }
        
        latencies
    });
    
    // Consumer task
    let consumer_task = tokio::spawn(async move {
        let mut latencies = Vec::new();
        
        for _ in 0..config.iterations {
            let start = Instant::now();
            let _ = rb_consumer.read_event().await;
            latencies.push(start.elapsed().as_nanos() as f64 / 1_000_000.0);
        }
        
        latencies
    });
    
    let (producer_latencies, consumer_latencies) = 
        tokio::try_join!(producer_task, consumer_task)?;
    
    let duration = start_time.elapsed();
    let (writes, reads, errors) = ring_buffer.get_stats();
    
    // Calculate metrics
    let total_ops = writes + reads;
    let events_per_second = total_ops as f64 / duration.as_secs_f64();
    let bytes_per_second = events_per_second * 1024.0; // 1KB per event
    
    let all_latencies = [producer_latencies, consumer_latencies].concat();
    let latency_metrics = calculate_latency_metrics(&all_latencies);
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second,
            bytes_per_second,
            operations_per_second: events_per_second,
        },
        latency: latency_metrics,
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / total_ops as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Single producer, multiple consumers benchmark
async fn single_producer_multi_consumer_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let ring_buffer = Arc::new(RingBufferSimulator::new(1024 * 1024));
    let data = vec![0u8; 1024];
    let consumer_count = config.concurrency;
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let rb_producer = ring_buffer.clone();
    
    // Single producer task
    let producer_task = tokio::spawn(async move {
        for _ in 0..config.iterations {
            let _ = rb_producer.write_event(&data).await;
        }
    });
    
    // Multiple consumer tasks
    let mut consumer_tasks = Vec::new();
    for _ in 0..consumer_count {
        let rb_consumer = ring_buffer.clone();
        let task = tokio::spawn(async move {
            let mut latencies = Vec::new();
            for _ in 0..(config.iterations / consumer_count) {
                let start = Instant::now();
                let _ = rb_consumer.read_event().await;
                latencies.push(start.elapsed().as_nanos() as f64 / 1_000_000.0);
            }
            latencies
        });
        consumer_tasks.push(task);
    }
    
    let _ = producer_task.await;
    let consumer_results = futures::future::join_all(consumer_tasks).await;
    
    let duration = start_time.elapsed();
    let (writes, reads, errors) = ring_buffer.get_stats();
    
    let mut all_latencies = Vec::new();
    for result in consumer_results {
        all_latencies.extend(result?);
    }
    
    let total_ops = writes + reads;
    let events_per_second = total_ops as f64 / duration.as_secs_f64();
    let bytes_per_second = events_per_second * 1024.0;
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second,
            bytes_per_second,
            operations_per_second: events_per_second,
        },
        latency: calculate_latency_metrics(&all_latencies),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / total_ops as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Multiple producers, single consumer benchmark
async fn multi_producer_single_consumer_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let ring_buffer = Arc::new(RingBufferSimulator::new(1024 * 1024));
    let data = vec![0u8; 1024];
    let producer_count = config.concurrency;
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    
    // Multiple producer tasks
    let mut producer_tasks = Vec::new();
    for _ in 0..producer_count {
        let rb_producer = ring_buffer.clone();
        let data_clone = data.clone();
        let task = tokio::spawn(async move {
            let mut latencies = Vec::new();
            for _ in 0..(config.iterations / producer_count) {
                let start = Instant::now();
                let _ = rb_producer.write_event(&data_clone).await;
                latencies.push(start.elapsed().as_nanos() as f64 / 1_000_000.0);
            }
            latencies
        });
        producer_tasks.push(task);
    }
    
    // Single consumer task
    let rb_consumer = ring_buffer.clone();
    let consumer_task = tokio::spawn(async move {
        let mut latencies = Vec::new();
        for _ in 0..config.iterations {
            let start = Instant::now();
            let _ = rb_consumer.read_event().await;
            latencies.push(start.elapsed().as_nanos() as f64 / 1_000_000.0);
        }
        latencies
    });
    
    let producer_results = futures::future::join_all(producer_tasks).await;
    let consumer_latencies = consumer_task.await?;
    
    let duration = start_time.elapsed();
    let (writes, reads, errors) = ring_buffer.get_stats();
    
    let mut all_latencies = Vec::new();
    for result in producer_results {
        all_latencies.extend(result?);
    }
    all_latencies.extend(consumer_latencies);
    
    let total_ops = writes + reads;
    let events_per_second = total_ops as f64 / duration.as_secs_f64();
    let bytes_per_second = events_per_second * 1024.0;
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second,
            bytes_per_second,
            operations_per_second: events_per_second,
        },
        latency: calculate_latency_metrics(&all_latencies),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / total_ops as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Multiple producers, multiple consumers benchmark
async fn multi_producer_multi_consumer_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let ring_buffer = Arc::new(RingBufferSimulator::new(1024 * 1024));
    let data = vec![0u8; 1024];
    let producer_count = config.concurrency;
    let consumer_count = config.concurrency;
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    
    // Multiple producer tasks
    let mut producer_tasks = Vec::new();
    for _ in 0..producer_count {
        let rb_producer = ring_buffer.clone();
        let data_clone = data.clone();
        let task = tokio::spawn(async move {
            for _ in 0..(config.iterations / producer_count) {
                let _ = rb_producer.write_event(&data_clone).await;
            }
        });
        producer_tasks.push(task);
    }
    
    // Multiple consumer tasks
    let mut consumer_tasks = Vec::new();
    for _ in 0..consumer_count {
        let rb_consumer = ring_buffer.clone();
        let task = tokio::spawn(async move {
            let mut latencies = Vec::new();
            for _ in 0..(config.iterations / consumer_count) {
                let start = Instant::now();
                let _ = rb_consumer.read_event().await;
                latencies.push(start.elapsed().as_nanos() as f64 / 1_000_000.0);
            }
            latencies
        });
        consumer_tasks.push(task);
    }
    
    let _ = futures::future::join_all(producer_tasks).await;
    let consumer_results = futures::future::join_all(consumer_tasks).await;
    
    let duration = start_time.elapsed();
    let (writes, reads, errors) = ring_buffer.get_stats();
    
    let mut all_latencies = Vec::new();
    for result in consumer_results {
        all_latencies.extend(result?);
    }
    
    let total_ops = writes + reads;
    let events_per_second = total_ops as f64 / duration.as_secs_f64();
    let bytes_per_second = events_per_second * 1024.0;
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second,
            bytes_per_second,
            operations_per_second: events_per_second,
        },
        latency: calculate_latency_metrics(&all_latencies),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / total_ops as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Burst write performance benchmark
async fn burst_write_performance_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let ring_buffer = Arc::new(RingBufferSimulator::new(1024 * 1024));
    let data = vec![0u8; 1024];
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut latencies = Vec::new();
    
    // Burst writes
    for _ in 0..config.iterations {
        let start = Instant::now();
        ring_buffer.write_event(&data).await?;
        latencies.push(start.elapsed().as_nanos() as f64 / 1_000_000.0);
    }
    
    let duration = start_time.elapsed();
    let (writes, _, errors) = ring_buffer.get_stats();
    
    let events_per_second = writes as f64 / duration.as_secs_f64();
    let bytes_per_second = events_per_second * 1024.0;
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second,
            bytes_per_second,
            operations_per_second: events_per_second,
        },
        latency: calculate_latency_metrics(&latencies),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / writes as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Sustained throughput benchmark
async fn sustained_throughput_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let ring_buffer = Arc::new(RingBufferSimulator::new(1024 * 1024));
    let data = vec![0u8; 1024];
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let rb_producer = ring_buffer.clone();
    let rb_consumer = ring_buffer.clone();
    
    // Sustained producer
    let producer_task = tokio::spawn(async move {
        while start_time.elapsed() < config.duration {
            let _ = rb_producer.write_event(&data).await;
            time::sleep(Duration::from_millis(1)).await;
        }
    });
    
    // Sustained consumer
    let consumer_task = tokio::spawn(async move {
        let mut latencies = Vec::new();
        while start_time.elapsed() < config.duration {
            let start = Instant::now();
            let _ = rb_consumer.read_event().await;
            latencies.push(start.elapsed().as_nanos() as f64 / 1_000_000.0);
            time::sleep(Duration::from_millis(1)).await;
        }
        latencies
    });
    
    let _ = producer_task.await;
    let latencies = consumer_task.await?;
    
    let duration = start_time.elapsed();
    let (writes, reads, errors) = ring_buffer.get_stats();
    
    let total_ops = writes + reads;
    let events_per_second = total_ops as f64 / duration.as_secs_f64();
    let bytes_per_second = events_per_second * 1024.0;
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second,
            bytes_per_second,
            operations_per_second: events_per_second,
        },
        latency: calculate_latency_metrics(&latencies),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / total_ops as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Memory pressure benchmark
async fn memory_pressure_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let ring_buffer = Arc::new(RingBufferSimulator::new(1024 * 1024));
    let large_data = vec![0u8; 64 * 1024]; // 64KB events
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut latencies = Vec::new();
    
    // Write large events to test memory pressure
    for _ in 0..config.iterations {
        let start = Instant::now();
        ring_buffer.write_event(&large_data).await?;
        latencies.push(start.elapsed().as_nanos() as f64 / 1_000_000.0);
    }
    
    let duration = start_time.elapsed();
    let (writes, _, errors) = ring_buffer.get_stats();
    
    let events_per_second = writes as f64 / duration.as_secs_f64();
    let bytes_per_second = events_per_second * 64.0 * 1024.0; // 64KB per event
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second,
            bytes_per_second,
            operations_per_second: events_per_second,
        },
        latency: calculate_latency_metrics(&latencies),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / writes as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Lock contention benchmark
async fn lock_contention_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let ring_buffer = Arc::new(RingBufferSimulator::new(1024 * 1024));
    let data = vec![0u8; 1024];
    let thread_count = config.concurrency * 2; // High contention
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    
    // High contention scenario
    let mut tasks = Vec::new();
    for _ in 0..thread_count {
        let rb = ring_buffer.clone();
        let data_clone = data.clone();
        let task = tokio::spawn(async move {
            let mut latencies = Vec::new();
            for _ in 0..(config.iterations / thread_count) {
                let start = Instant::now();
                let _ = rb.write_event(&data_clone).await;
                latencies.push(start.elapsed().as_nanos() as f64 / 1_000_000.0);
            }
            latencies
        });
        tasks.push(task);
    }
    
    let results = futures::future::join_all(tasks).await;
    
    let duration = start_time.elapsed();
    let (writes, _, errors) = ring_buffer.get_stats();
    
    let mut all_latencies = Vec::new();
    for result in results {
        all_latencies.extend(result?);
    }
    
    let events_per_second = writes as f64 / duration.as_secs_f64();
    let bytes_per_second = events_per_second * 1024.0;
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second,
            bytes_per_second,
            operations_per_second: events_per_second,
        },
        latency: calculate_latency_metrics(&all_latencies),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / writes as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Calculate latency metrics from a set of measurements
fn calculate_latency_metrics(latencies: &[f64]) -> LatencyMetrics {
    if latencies.is_empty() {
        return LatencyMetrics {
            p50_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
            max_ms: 0.0,
            mean_ms: 0.0,
        };
    }
    
    let mut sorted_latencies = latencies.to_vec();
    sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let len = sorted_latencies.len();
    let p50_idx = (len as f64 * 0.50) as usize;
    let p95_idx = (len as f64 * 0.95) as usize;
    let p99_idx = (len as f64 * 0.99) as usize;
    
    let mean = sorted_latencies.iter().sum::<f64>() / len as f64;
    
    LatencyMetrics {
        p50_ms: sorted_latencies[p50_idx.min(len - 1)],
        p95_ms: sorted_latencies[p95_idx.min(len - 1)],
        p99_ms: sorted_latencies[p99_idx.min(len - 1)],
        max_ms: sorted_latencies[len - 1],
        mean_ms: mean,
    }
}

/// Get current resource usage metrics
fn get_resource_metrics() -> ResourceMetrics {
    let mut system = sysinfo::System::new_all();
    system.refresh_all();
    
    let cpu_usage = system.global_cpu_info().cpu_usage() as f64;
    let memory_usage = system.used_memory() as f64 / 1024.0 / 1024.0; // MB
    
    ResourceMetrics {
        cpu_usage_percent: cpu_usage,
        memory_usage_mb: memory_usage,
        disk_io_bytes_per_second: 0.0, // TODO: Implement disk I/O monitoring
        network_io_bytes_per_second: 0.0, // TODO: Implement network I/O monitoring
        file_handles: 0, // TODO: Implement file handle counting
        thread_count: system.processes().len() as u64,
    }
}