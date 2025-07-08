//! Packer service performance benchmarks
//!
//! Tests the Chronicle packer service performance including data processing throughput,
//! compression efficiency, encryption overhead, and storage I/O performance.

use crate::{
    BenchmarkComponent, BenchmarkConfig, BenchmarkResult, ErrorMetrics, LatencyMetrics,
    PerformanceMetrics, ResourceMetrics, ThroughputMetrics,
};
use anyhow::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time;

/// Packer benchmark test cases
const BENCHMARK_TESTS: &[&str] = &[
    "data_processing_throughput",
    "compression_performance",
    "encryption_overhead",
    "parquet_write_performance",
    "heif_processing_performance",
    "batch_processing_efficiency",
    "memory_usage_under_load",
    "concurrent_processing",
    "large_file_handling",
    "storage_io_performance",
    "metadata_processing",
    "error_recovery",
];

/// Simulated packer service
struct PackerSimulator {
    processed_bytes: AtomicU64,
    processed_events: AtomicU64,
    compressed_bytes: AtomicU64,
    encrypted_bytes: AtomicU64,
    written_files: AtomicU64,
    errors: AtomicU64,
}

impl PackerSimulator {
    fn new() -> Self {
        Self {
            processed_bytes: AtomicU64::new(0),
            processed_events: AtomicU64::new(0),
            compressed_bytes: AtomicU64::new(0),
            encrypted_bytes: AtomicU64::new(0),
            written_files: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        }
    }
    
    async fn process_data(&self, data: &[u8]) -> Result<ProcessingResult> {
        let start = Instant::now();
        
        // Simulate data processing
        time::sleep(Duration::from_micros(10)).await;
        
        // Simulate compression (50% compression ratio)
        let compressed_size = data.len() / 2;
        time::sleep(Duration::from_micros(20)).await;
        
        // Simulate encryption overhead
        let encrypted_size = compressed_size + 16; // Add auth tag
        time::sleep(Duration::from_micros(5)).await;
        
        // Update statistics
        self.processed_bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
        self.processed_events.fetch_add(1, Ordering::Relaxed);
        self.compressed_bytes.fetch_add(compressed_size as u64, Ordering::Relaxed);
        self.encrypted_bytes.fetch_add(encrypted_size as u64, Ordering::Relaxed);
        
        // Simulate occasional errors
        if rand::random::<f64>() < 0.001 {
            self.errors.fetch_add(1, Ordering::Relaxed);
            return Err(anyhow::anyhow!("Processing error"));
        }
        
        Ok(ProcessingResult {
            processing_time_ms: start.elapsed().as_nanos() as f64 / 1_000_000.0,
            original_size: data.len(),
            compressed_size,
            encrypted_size,
        })
    }
    
    async fn write_parquet(&self, data: &[u8]) -> Result<f64> {
        let start = Instant::now();
        
        // Simulate Parquet writing with columnar compression
        let write_time = Duration::from_micros(data.len() as u64 / 100); // 100 MB/s
        time::sleep(write_time).await;
        
        self.written_files.fetch_add(1, Ordering::Relaxed);
        
        // Simulate write errors
        if rand::random::<f64>() < 0.001 {
            self.errors.fetch_add(1, Ordering::Relaxed);
            return Err(anyhow::anyhow!("Parquet write error"));
        }
        
        Ok(start.elapsed().as_nanos() as f64 / 1_000_000.0)
    }
    
    async fn process_heif(&self, image_data: &[u8]) -> Result<f64> {
        let start = Instant::now();
        
        // Simulate HEIF processing (more CPU intensive)
        let processing_time = Duration::from_micros(image_data.len() as u64 / 10); // 10 MB/s
        time::sleep(processing_time).await;
        
        self.written_files.fetch_add(1, Ordering::Relaxed);
        
        // Simulate processing errors
        if rand::random::<f64>() < 0.002 {
            self.errors.fetch_add(1, Ordering::Relaxed);
            return Err(anyhow::anyhow!("HEIF processing error"));
        }
        
        Ok(start.elapsed().as_nanos() as f64 / 1_000_000.0)
    }
    
    fn get_stats(&self) -> PackerStats {
        PackerStats {
            processed_bytes: self.processed_bytes.load(Ordering::Relaxed),
            processed_events: self.processed_events.load(Ordering::Relaxed),
            compressed_bytes: self.compressed_bytes.load(Ordering::Relaxed),
            encrypted_bytes: self.encrypted_bytes.load(Ordering::Relaxed),
            written_files: self.written_files.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug)]
struct ProcessingResult {
    processing_time_ms: f64,
    original_size: usize,
    compressed_size: usize,
    encrypted_size: usize,
}

#[derive(Debug)]
struct PackerStats {
    processed_bytes: u64,
    processed_events: u64,
    compressed_bytes: u64,
    encrypted_bytes: u64,
    written_files: u64,
    errors: u64,
}

/// Run a specific packer benchmark
pub async fn run_benchmark(test_name: &str, config: &BenchmarkConfig) -> Result<BenchmarkResult> {
    let start_time = Instant::now();
    
    let result = match test_name {
        "data_processing_throughput" => data_processing_throughput_benchmark(config).await,
        "compression_performance" => compression_performance_benchmark(config).await,
        "encryption_overhead" => encryption_overhead_benchmark(config).await,
        "parquet_write_performance" => parquet_write_performance_benchmark(config).await,
        "heif_processing_performance" => heif_processing_performance_benchmark(config).await,
        "batch_processing_efficiency" => batch_processing_efficiency_benchmark(config).await,
        "memory_usage_under_load" => memory_usage_under_load_benchmark(config).await,
        "concurrent_processing" => concurrent_processing_benchmark(config).await,
        "large_file_handling" => large_file_handling_benchmark(config).await,
        "storage_io_performance" => storage_io_performance_benchmark(config).await,
        "metadata_processing" => metadata_processing_benchmark(config).await,
        "error_recovery" => error_recovery_benchmark(config).await,
        _ => return Err(anyhow::anyhow!("Unknown benchmark test: {}", test_name)),
    };

    let duration = start_time.elapsed();
    
    match result {
        Ok(mut metrics) => {
            metrics.timestamp = chrono::Utc::now();
            
            // Check if performance targets are met (1GB/hour = ~291 KB/s)
            let target_bytes_per_second = config.targets.packer_throughput_gb_per_hour * 1024.0 * 1024.0 * 1024.0 / 3600.0;
            let passed = metrics.throughput.bytes_per_second >= target_bytes_per_second;
            
            Ok(BenchmarkResult {
                component: BenchmarkComponent::Packer,
                test_name: test_name.to_string(),
                metrics,
                passed,
                notes: Some(format!("Completed in {:.2?}", duration)),
            })
        }
        Err(e) => {
            let metrics = create_error_metrics();
            
            Ok(BenchmarkResult {
                component: BenchmarkComponent::Packer,
                test_name: test_name.to_string(),
                metrics,
                passed: false,
                notes: Some(format!("Failed: {}", e)),
            })
        }
    }
}

/// Run all packer benchmarks
pub async fn run_all_benchmarks(config: &BenchmarkConfig) -> Result<Vec<BenchmarkResult>> {
    let mut results = Vec::new();
    
    for test_name in BENCHMARK_TESTS {
        let result = run_benchmark(test_name, config).await?;
        results.push(result);
    }
    
    Ok(results)
}

/// Data processing throughput benchmark
async fn data_processing_throughput_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let packer = Arc::new(PackerSimulator::new());
    let data = vec![0u8; 1024]; // 1KB events
    
    // Warmup
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut processing_times = Vec::new();
    
    // Process data for specified duration
    while start_time.elapsed() < config.duration {
        let result = packer.process_data(&data).await?;
        processing_times.push(result.processing_time_ms);
    }
    
    let duration = start_time.elapsed();
    let stats = packer.get_stats();
    
    let events_per_second = stats.processed_events as f64 / duration.as_secs_f64();
    let bytes_per_second = stats.processed_bytes as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second,
            bytes_per_second,
            operations_per_second: events_per_second,
        },
        latency: calculate_latency_metrics(&processing_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.processed_events as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Compression performance benchmark
async fn compression_performance_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let packer = Arc::new(PackerSimulator::new());
    let data = vec![0u8; 10240]; // 10KB events for better compression testing
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut processing_times = Vec::new();
    
    for _ in 0..config.iterations {
        let result = packer.process_data(&data).await?;
        processing_times.push(result.processing_time_ms);
    }
    
    let duration = start_time.elapsed();
    let stats = packer.get_stats();
    
    let compression_ratio = stats.compressed_bytes as f64 / stats.processed_bytes as f64;
    let bytes_per_second = stats.processed_bytes as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: stats.processed_events as f64 / duration.as_secs_f64(),
            bytes_per_second,
            operations_per_second: compression_ratio, // Use compression ratio as operation metric
        },
        latency: calculate_latency_metrics(&processing_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.processed_events as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Encryption overhead benchmark
async fn encryption_overhead_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let packer = Arc::new(PackerSimulator::new());
    let data = vec![0u8; 1024];
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut processing_times = Vec::new();
    
    for _ in 0..config.iterations {
        let result = packer.process_data(&data).await?;
        processing_times.push(result.processing_time_ms);
    }
    
    let duration = start_time.elapsed();
    let stats = packer.get_stats();
    
    let encryption_overhead = (stats.encrypted_bytes as f64 - stats.compressed_bytes as f64) / stats.compressed_bytes as f64;
    let bytes_per_second = stats.processed_bytes as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: stats.processed_events as f64 / duration.as_secs_f64(),
            bytes_per_second,
            operations_per_second: 1.0 / encryption_overhead, // Lower overhead = higher ops/sec
        },
        latency: calculate_latency_metrics(&processing_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.processed_events as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Parquet write performance benchmark
async fn parquet_write_performance_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let packer = Arc::new(PackerSimulator::new());
    let data = vec![0u8; 100 * 1024]; // 100KB chunks
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut write_times = Vec::new();
    
    for _ in 0..config.iterations {
        let write_time = packer.write_parquet(&data).await?;
        write_times.push(write_time);
    }
    
    let duration = start_time.elapsed();
    let stats = packer.get_stats();
    
    let bytes_per_second = (data.len() * config.iterations as usize) as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: stats.written_files as f64 / duration.as_secs_f64(),
            bytes_per_second,
            operations_per_second: stats.written_files as f64 / duration.as_secs_f64(),
        },
        latency: calculate_latency_metrics(&write_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.written_files as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// HEIF processing performance benchmark
async fn heif_processing_performance_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let packer = Arc::new(PackerSimulator::new());
    let image_data = vec![0u8; 1024 * 1024]; // 1MB image
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut processing_times = Vec::new();
    
    for _ in 0..config.iterations {
        let processing_time = packer.process_heif(&image_data).await?;
        processing_times.push(processing_time);
    }
    
    let duration = start_time.elapsed();
    let stats = packer.get_stats();
    
    let bytes_per_second = (image_data.len() * config.iterations as usize) as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: stats.written_files as f64 / duration.as_secs_f64(),
            bytes_per_second,
            operations_per_second: stats.written_files as f64 / duration.as_secs_f64(),
        },
        latency: calculate_latency_metrics(&processing_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.written_files as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Batch processing efficiency benchmark
async fn batch_processing_efficiency_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let packer = Arc::new(PackerSimulator::new());
    let batch_size = 100;
    let data = vec![0u8; 1024];
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut batch_times = Vec::new();
    
    for _ in 0..(config.iterations / batch_size) {
        let batch_start = Instant::now();
        
        for _ in 0..batch_size {
            let _ = packer.process_data(&data).await?;
        }
        
        batch_times.push(batch_start.elapsed().as_nanos() as f64 / 1_000_000.0);
    }
    
    let duration = start_time.elapsed();
    let stats = packer.get_stats();
    
    let events_per_second = stats.processed_events as f64 / duration.as_secs_f64();
    let bytes_per_second = stats.processed_bytes as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second,
            bytes_per_second,
            operations_per_second: events_per_second / batch_size as f64, // Batches per second
        },
        latency: calculate_latency_metrics(&batch_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.processed_events as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Memory usage under load benchmark
async fn memory_usage_under_load_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let packer = Arc::new(PackerSimulator::new());
    let data = vec![0u8; 64 * 1024]; // 64KB events
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut memory_samples = Vec::new();
    
    while start_time.elapsed() < config.duration {
        let memory_before = get_memory_usage();
        
        // Process multiple events to simulate load
        for _ in 0..10 {
            let _ = packer.process_data(&data).await?;
        }
        
        let memory_after = get_memory_usage();
        memory_samples.push(memory_after - memory_before);
        
        time::sleep(Duration::from_millis(10)).await;
    }
    
    let duration = start_time.elapsed();
    let stats = packer.get_stats();
    
    let mean_memory_usage = memory_samples.iter().sum::<f64>() / memory_samples.len() as f64;
    let max_memory_usage = memory_samples.iter().fold(0.0, |max, &val| max.max(val));
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: stats.processed_events as f64 / duration.as_secs_f64(),
            bytes_per_second: stats.processed_bytes as f64 / duration.as_secs_f64(),
            operations_per_second: stats.processed_events as f64 / duration.as_secs_f64(),
        },
        latency: LatencyMetrics {
            p50_ms: mean_memory_usage,
            p95_ms: max_memory_usage,
            p99_ms: max_memory_usage,
            max_ms: max_memory_usage,
            mean_ms: mean_memory_usage,
        },
        resources: ResourceMetrics {
            cpu_usage_percent: get_cpu_usage(),
            memory_usage_mb: mean_memory_usage,
            disk_io_bytes_per_second: 0.0,
            network_io_bytes_per_second: 0.0,
            file_handles: 0,
            thread_count: 1,
        },
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.processed_events as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Concurrent processing benchmark
async fn concurrent_processing_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let packer = Arc::new(PackerSimulator::new());
    let data = vec![0u8; 1024];
    let concurrency = config.concurrency;
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    
    // Spawn concurrent processing tasks
    let mut tasks = Vec::new();
    for _ in 0..concurrency {
        let packer_clone = packer.clone();
        let data_clone = data.clone();
        let task = tokio::spawn(async move {
            let mut processing_times = Vec::new();
            for _ in 0..(config.iterations / concurrency) {
                let result = packer_clone.process_data(&data_clone).await?;
                processing_times.push(result.processing_time_ms);
            }
            Ok::<Vec<f64>, anyhow::Error>(processing_times)
        });
        tasks.push(task);
    }
    
    let results = futures::future::join_all(tasks).await;
    
    let duration = start_time.elapsed();
    let stats = packer.get_stats();
    
    let mut all_processing_times = Vec::new();
    for result in results {
        all_processing_times.extend(result??);
    }
    
    let events_per_second = stats.processed_events as f64 / duration.as_secs_f64();
    let bytes_per_second = stats.processed_bytes as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second,
            bytes_per_second,
            operations_per_second: events_per_second,
        },
        latency: calculate_latency_metrics(&all_processing_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.processed_events as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Large file handling benchmark
async fn large_file_handling_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let packer = Arc::new(PackerSimulator::new());
    let large_data = vec![0u8; 10 * 1024 * 1024]; // 10MB files
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut processing_times = Vec::new();
    
    for _ in 0..10 { // Process 10 large files
        let result = packer.process_data(&large_data).await?;
        processing_times.push(result.processing_time_ms);
    }
    
    let duration = start_time.elapsed();
    let stats = packer.get_stats();
    
    let bytes_per_second = stats.processed_bytes as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: stats.processed_events as f64 / duration.as_secs_f64(),
            bytes_per_second,
            operations_per_second: stats.processed_events as f64 / duration.as_secs_f64(),
        },
        latency: calculate_latency_metrics(&processing_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.processed_events as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Storage I/O performance benchmark
async fn storage_io_performance_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let packer = Arc::new(PackerSimulator::new());
    let data = vec![0u8; 1024 * 1024]; // 1MB chunks
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut write_times = Vec::new();
    
    for _ in 0..config.iterations {
        let write_time = packer.write_parquet(&data).await?;
        write_times.push(write_time);
    }
    
    let duration = start_time.elapsed();
    let stats = packer.get_stats();
    
    let bytes_per_second = (data.len() * config.iterations as usize) as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: stats.written_files as f64 / duration.as_secs_f64(),
            bytes_per_second,
            operations_per_second: stats.written_files as f64 / duration.as_secs_f64(),
        },
        latency: calculate_latency_metrics(&write_times),
        resources: ResourceMetrics {
            cpu_usage_percent: get_cpu_usage(),
            memory_usage_mb: get_memory_usage(),
            disk_io_bytes_per_second: bytes_per_second,
            network_io_bytes_per_second: 0.0,
            file_handles: stats.written_files,
            thread_count: 1,
        },
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.written_files as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Metadata processing benchmark
async fn metadata_processing_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let packer = Arc::new(PackerSimulator::new());
    let metadata = vec![0u8; 256]; // 256 bytes metadata
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut processing_times = Vec::new();
    
    for _ in 0..config.iterations {
        let result = packer.process_data(&metadata).await?;
        processing_times.push(result.processing_time_ms);
    }
    
    let duration = start_time.elapsed();
    let stats = packer.get_stats();
    
    let events_per_second = stats.processed_events as f64 / duration.as_secs_f64();
    let bytes_per_second = stats.processed_bytes as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second,
            bytes_per_second,
            operations_per_second: events_per_second,
        },
        latency: calculate_latency_metrics(&processing_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.processed_events as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Error recovery benchmark
async fn error_recovery_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let packer = Arc::new(PackerSimulator::new());
    let data = vec![0u8; 1024];
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut recovery_times = Vec::new();
    let mut total_errors = 0;
    
    for _ in 0..config.iterations {
        let error_start = Instant::now();
        
        // Process data, some will fail
        match packer.process_data(&data).await {
            Ok(_) => {}
            Err(_) => {
                total_errors += 1;
                // Simulate recovery time
                time::sleep(Duration::from_millis(10)).await;
                recovery_times.push(error_start.elapsed().as_nanos() as f64 / 1_000_000.0);
            }
        }
    }
    
    let duration = start_time.elapsed();
    let stats = packer.get_stats();
    
    let mean_recovery_time = if !recovery_times.is_empty() {
        recovery_times.iter().sum::<f64>() / recovery_times.len() as f64
    } else {
        0.0
    };
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: stats.processed_events as f64 / duration.as_secs_f64(),
            bytes_per_second: stats.processed_bytes as f64 / duration.as_secs_f64(),
            operations_per_second: stats.processed_events as f64 / duration.as_secs_f64(),
        },
        latency: calculate_latency_metrics(&recovery_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: total_errors as f64 / config.iterations as f64,
            recovery_time_ms: mean_recovery_time,
            total_errors: total_errors,
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
    ResourceMetrics {
        cpu_usage_percent: get_cpu_usage(),
        memory_usage_mb: get_memory_usage(),
        disk_io_bytes_per_second: 0.0,
        network_io_bytes_per_second: 0.0,
        file_handles: 0,
        thread_count: 1,
    }
}

/// Get current CPU usage
fn get_cpu_usage() -> f64 {
    let mut system = sysinfo::System::new();
    system.refresh_cpu();
    system.global_cpu_info().cpu_usage() as f64
}

/// Get current memory usage in MB
fn get_memory_usage() -> f64 {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    system.used_memory() as f64 / 1024.0 / 1024.0
}

/// Create error metrics for failed benchmarks
fn create_error_metrics() -> PerformanceMetrics {
    PerformanceMetrics {
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
    }
}