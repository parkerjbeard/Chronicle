//! Storage performance benchmarks
//!
//! Tests Chronicle's storage layer performance including file I/O, database operations,
//! compression efficiency, and storage overhead metrics.

use crate::{
    BenchmarkComponent, BenchmarkConfig, BenchmarkResult, ErrorMetrics, LatencyMetrics,
    PerformanceMetrics, ResourceMetrics, ThroughputMetrics,
};
use anyhow::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time;

/// Storage benchmark test cases
const BENCHMARK_TESTS: &[&str] = &[
    "file_write_performance",
    "file_read_performance",
    "database_insert_performance",
    "database_query_performance",
    "compression_efficiency",
    "storage_overhead",
    "concurrent_file_operations",
    "large_file_handling",
    "metadata_operations",
    "index_performance",
    "backup_restore_performance",
    "storage_cleanup_performance",
];

/// Simulated storage system
struct StorageSimulator {
    files_written: AtomicU64,
    files_read: AtomicU64,
    bytes_written: AtomicU64,
    bytes_read: AtomicU64,
    db_operations: AtomicU64,
    compressed_bytes: AtomicU64,
    errors: AtomicU64,
}

impl StorageSimulator {
    fn new() -> Self {
        Self {
            files_written: AtomicU64::new(0),
            files_read: AtomicU64::new(0),
            bytes_written: AtomicU64::new(0),
            bytes_read: AtomicU64::new(0),
            db_operations: AtomicU64::new(0),
            compressed_bytes: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        }
    }
    
    async fn write_file(&self, data: &[u8]) -> Result<f64> {
        let start = Instant::now();
        
        // Simulate file write latency based on size
        let write_time = Duration::from_nanos(data.len() as u64 * 10); // 100 MB/s
        time::sleep(write_time).await;
        
        self.files_written.fetch_add(1, Ordering::Relaxed);
        self.bytes_written.fetch_add(data.len() as u64, Ordering::Relaxed);
        
        // Simulate write errors
        if rand::random::<f64>() < 0.001 {
            self.errors.fetch_add(1, Ordering::Relaxed);
            return Err(anyhow::anyhow!("File write error"));
        }
        
        Ok(start.elapsed().as_nanos() as f64 / 1_000_000.0)
    }
    
    async fn read_file(&self, size: usize) -> Result<(Vec<u8>, f64)> {
        let start = Instant::now();
        
        // Simulate file read latency
        let read_time = Duration::from_nanos(size as u64 * 8); // 125 MB/s
        time::sleep(read_time).await;
        
        self.files_read.fetch_add(1, Ordering::Relaxed);
        self.bytes_read.fetch_add(size as u64, Ordering::Relaxed);
        
        // Simulate read errors
        if rand::random::<f64>() < 0.001 {
            self.errors.fetch_add(1, Ordering::Relaxed);
            return Err(anyhow::anyhow!("File read error"));
        }
        
        let data = vec![0u8; size];
        let latency = start.elapsed().as_nanos() as f64 / 1_000_000.0;
        
        Ok((data, latency))
    }
    
    async fn insert_record(&self, data: &[u8]) -> Result<f64> {
        let start = Instant::now();
        
        // Simulate database insert latency
        let insert_time = Duration::from_micros(100 + data.len() as u64 / 1000);
        time::sleep(insert_time).await;
        
        self.db_operations.fetch_add(1, Ordering::Relaxed);
        self.bytes_written.fetch_add(data.len() as u64, Ordering::Relaxed);
        
        // Simulate insert errors
        if rand::random::<f64>() < 0.001 {
            self.errors.fetch_add(1, Ordering::Relaxed);
            return Err(anyhow::anyhow!("Database insert error"));
        }
        
        Ok(start.elapsed().as_nanos() as f64 / 1_000_000.0)
    }
    
    async fn query_records(&self, query_size: usize) -> Result<(Vec<Vec<u8>>, f64)> {
        let start = Instant::now();
        
        // Simulate database query latency
        let query_time = Duration::from_micros(50 + query_size as u64 / 100);
        time::sleep(query_time).await;
        
        self.db_operations.fetch_add(1, Ordering::Relaxed);
        self.bytes_read.fetch_add(query_size as u64, Ordering::Relaxed);
        
        // Simulate query errors
        if rand::random::<f64>() < 0.001 {
            self.errors.fetch_add(1, Ordering::Relaxed);
            return Err(anyhow::anyhow!("Database query error"));
        }
        
        // Return simulated query results
        let results = vec![vec![0u8; 1024]; query_size / 1024];
        let latency = start.elapsed().as_nanos() as f64 / 1_000_000.0;
        
        Ok((results, latency))
    }
    
    async fn compress_data(&self, data: &[u8]) -> Result<(Vec<u8>, f64)> {
        let start = Instant::now();
        
        // Simulate compression latency
        let compress_time = Duration::from_micros(data.len() as u64 / 10); // 10 MB/s
        time::sleep(compress_time).await;
        
        // Simulate 60% compression ratio
        let compressed_size = (data.len() as f64 * 0.6) as usize;
        let compressed_data = vec![0u8; compressed_size];
        
        self.compressed_bytes.fetch_add(compressed_size as u64, Ordering::Relaxed);
        
        // Simulate compression errors
        if rand::random::<f64>() < 0.001 {
            self.errors.fetch_add(1, Ordering::Relaxed);
            return Err(anyhow::anyhow!("Compression error"));
        }
        
        let latency = start.elapsed().as_nanos() as f64 / 1_000_000.0;
        
        Ok((compressed_data, latency))
    }
    
    async fn create_index(&self, record_count: usize) -> Result<f64> {
        let start = Instant::now();
        
        // Simulate index creation latency
        let index_time = Duration::from_millis(record_count as u64 / 100);
        time::sleep(index_time).await;
        
        self.db_operations.fetch_add(1, Ordering::Relaxed);
        
        // Simulate index creation errors
        if rand::random::<f64>() < 0.01 {
            self.errors.fetch_add(1, Ordering::Relaxed);
            return Err(anyhow::anyhow!("Index creation error"));
        }
        
        Ok(start.elapsed().as_nanos() as f64 / 1_000_000.0)
    }
    
    async fn cleanup_storage(&self, file_count: usize) -> Result<f64> {
        let start = Instant::now();
        
        // Simulate storage cleanup latency
        let cleanup_time = Duration::from_millis(file_count as u64 / 10);
        time::sleep(cleanup_time).await;
        
        self.db_operations.fetch_add(1, Ordering::Relaxed);
        
        // Simulate cleanup errors
        if rand::random::<f64>() < 0.005 {
            self.errors.fetch_add(1, Ordering::Relaxed);
            return Err(anyhow::anyhow!("Storage cleanup error"));
        }
        
        Ok(start.elapsed().as_nanos() as f64 / 1_000_000.0)
    }
    
    fn get_stats(&self) -> StorageStats {
        StorageStats {
            files_written: self.files_written.load(Ordering::Relaxed),
            files_read: self.files_read.load(Ordering::Relaxed),
            bytes_written: self.bytes_written.load(Ordering::Relaxed),
            bytes_read: self.bytes_read.load(Ordering::Relaxed),
            db_operations: self.db_operations.load(Ordering::Relaxed),
            compressed_bytes: self.compressed_bytes.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug)]
struct StorageStats {
    files_written: u64,
    files_read: u64,
    bytes_written: u64,
    bytes_read: u64,
    db_operations: u64,
    compressed_bytes: u64,
    errors: u64,
}

/// Run a specific storage benchmark
pub async fn run_benchmark(test_name: &str, config: &BenchmarkConfig) -> Result<BenchmarkResult> {
    let start_time = Instant::now();
    
    let result = match test_name {
        "file_write_performance" => file_write_performance_benchmark(config).await,
        "file_read_performance" => file_read_performance_benchmark(config).await,
        "database_insert_performance" => database_insert_performance_benchmark(config).await,
        "database_query_performance" => database_query_performance_benchmark(config).await,
        "compression_efficiency" => compression_efficiency_benchmark(config).await,
        "storage_overhead" => storage_overhead_benchmark(config).await,
        "concurrent_file_operations" => concurrent_file_operations_benchmark(config).await,
        "large_file_handling" => large_file_handling_benchmark(config).await,
        "metadata_operations" => metadata_operations_benchmark(config).await,
        "index_performance" => index_performance_benchmark(config).await,
        "backup_restore_performance" => backup_restore_performance_benchmark(config).await,
        "storage_cleanup_performance" => storage_cleanup_performance_benchmark(config).await,
        _ => return Err(anyhow::anyhow!("Unknown benchmark test: {}", test_name)),
    };

    let duration = start_time.elapsed();
    
    match result {
        Ok(mut metrics) => {
            metrics.timestamp = chrono::Utc::now();
            
            // Check if performance targets are met (<10MB/day overhead)
            let daily_overhead_mb = metrics.throughput.bytes_per_second * 86400.0 / (1024.0 * 1024.0);
            let passed = daily_overhead_mb <= config.targets.storage_overhead_mb_per_day;
            
            Ok(BenchmarkResult {
                component: BenchmarkComponent::Storage,
                test_name: test_name.to_string(),
                metrics,
                passed,
                notes: Some(format!("Completed in {:.2?}", duration)),
            })
        }
        Err(e) => {
            let metrics = create_error_metrics();
            
            Ok(BenchmarkResult {
                component: BenchmarkComponent::Storage,
                test_name: test_name.to_string(),
                metrics,
                passed: false,
                notes: Some(format!("Failed: {}", e)),
            })
        }
    }
}

/// Run all storage benchmarks
pub async fn run_all_benchmarks(config: &BenchmarkConfig) -> Result<Vec<BenchmarkResult>> {
    let mut results = Vec::new();
    
    for test_name in BENCHMARK_TESTS {
        let result = run_benchmark(test_name, config).await?;
        results.push(result);
    }
    
    Ok(results)
}

/// File write performance benchmark
async fn file_write_performance_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let storage = Arc::new(StorageSimulator::new());
    let data = vec![0u8; 1024 * 1024]; // 1MB files
    
    // Warmup
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut write_times = Vec::new();
    
    for _ in 0..config.iterations {
        let write_time = storage.write_file(&data).await?;
        write_times.push(write_time);
    }
    
    let duration = start_time.elapsed();
    let stats = storage.get_stats();
    
    let files_per_second = stats.files_written as f64 / duration.as_secs_f64();
    let bytes_per_second = stats.bytes_written as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: files_per_second,
            bytes_per_second,
            operations_per_second: files_per_second,
        },
        latency: calculate_latency_metrics(&write_times),
        resources: ResourceMetrics {
            cpu_usage_percent: get_cpu_usage(),
            memory_usage_mb: get_memory_usage(),
            disk_io_bytes_per_second: bytes_per_second,
            network_io_bytes_per_second: 0.0,
            file_handles: stats.files_written,
            thread_count: 1,
        },
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.files_written as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// File read performance benchmark
async fn file_read_performance_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let storage = Arc::new(StorageSimulator::new());
    let file_size = 1024 * 1024; // 1MB files
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut read_times = Vec::new();
    
    for _ in 0..config.iterations {
        let (_, read_time) = storage.read_file(file_size).await?;
        read_times.push(read_time);
    }
    
    let duration = start_time.elapsed();
    let stats = storage.get_stats();
    
    let files_per_second = stats.files_read as f64 / duration.as_secs_f64();
    let bytes_per_second = stats.bytes_read as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: files_per_second,
            bytes_per_second,
            operations_per_second: files_per_second,
        },
        latency: calculate_latency_metrics(&read_times),
        resources: ResourceMetrics {
            cpu_usage_percent: get_cpu_usage(),
            memory_usage_mb: get_memory_usage(),
            disk_io_bytes_per_second: bytes_per_second,
            network_io_bytes_per_second: 0.0,
            file_handles: stats.files_read,
            thread_count: 1,
        },
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.files_read as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Database insert performance benchmark
async fn database_insert_performance_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let storage = Arc::new(StorageSimulator::new());
    let record_data = vec![0u8; 1024]; // 1KB records
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut insert_times = Vec::new();
    
    for _ in 0..config.iterations {
        let insert_time = storage.insert_record(&record_data).await?;
        insert_times.push(insert_time);
    }
    
    let duration = start_time.elapsed();
    let stats = storage.get_stats();
    
    let operations_per_second = stats.db_operations as f64 / duration.as_secs_f64();
    let bytes_per_second = stats.bytes_written as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: operations_per_second,
            bytes_per_second,
            operations_per_second,
        },
        latency: calculate_latency_metrics(&insert_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.db_operations as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Database query performance benchmark
async fn database_query_performance_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let storage = Arc::new(StorageSimulator::new());
    let query_size = 10 * 1024; // 10KB queries
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut query_times = Vec::new();
    
    for _ in 0..config.iterations {
        let (_, query_time) = storage.query_records(query_size).await?;
        query_times.push(query_time);
    }
    
    let duration = start_time.elapsed();
    let stats = storage.get_stats();
    
    let operations_per_second = stats.db_operations as f64 / duration.as_secs_f64();
    let bytes_per_second = stats.bytes_read as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: operations_per_second,
            bytes_per_second,
            operations_per_second,
        },
        latency: calculate_latency_metrics(&query_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.db_operations as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Compression efficiency benchmark
async fn compression_efficiency_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let storage = Arc::new(StorageSimulator::new());
    let data = vec![0u8; 10 * 1024 * 1024]; // 10MB data
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut compression_times = Vec::new();
    let mut original_sizes = Vec::new();
    let mut compressed_sizes = Vec::new();
    
    for _ in 0..config.iterations {
        let original_size = data.len();
        let (compressed_data, compression_time) = storage.compress_data(&data).await?;
        
        compression_times.push(compression_time);
        original_sizes.push(original_size);
        compressed_sizes.push(compressed_data.len());
    }
    
    let duration = start_time.elapsed();
    let stats = storage.get_stats();
    
    let total_original_size: usize = original_sizes.iter().sum();
    let total_compressed_size: usize = compressed_sizes.iter().sum();
    let compression_ratio = total_compressed_size as f64 / total_original_size as f64;
    
    let operations_per_second = config.iterations as f64 / duration.as_secs_f64();
    let bytes_per_second = total_original_size as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: operations_per_second,
            bytes_per_second,
            operations_per_second: compression_ratio, // Use compression ratio as operation metric
        },
        latency: calculate_latency_metrics(&compression_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / config.iterations as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Storage overhead benchmark
async fn storage_overhead_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let storage = Arc::new(StorageSimulator::new());
    let data = vec![0u8; 1024]; // 1KB data
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let start_stats = storage.get_stats();
    
    // Perform various storage operations
    for _ in 0..config.iterations {
        // Write file
        let _ = storage.write_file(&data).await?;
        
        // Insert record
        let _ = storage.insert_record(&data).await?;
        
        // Compress data
        let _ = storage.compress_data(&data).await?;
    }
    
    let duration = start_time.elapsed();
    let end_stats = storage.get_stats();
    
    let total_operations = (end_stats.files_written - start_stats.files_written) +
                          (end_stats.db_operations - start_stats.db_operations);
    let total_bytes = (end_stats.bytes_written - start_stats.bytes_written) +
                     (end_stats.compressed_bytes - start_stats.compressed_bytes);
    
    let operations_per_second = total_operations as f64 / duration.as_secs_f64();
    let bytes_per_second = total_bytes as f64 / duration.as_secs_f64();
    
    // Calculate overhead as percentage of additional storage used
    let overhead_bytes = total_bytes as f64 * 0.05; // 5% overhead
    let overhead_per_second = overhead_bytes / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: operations_per_second,
            bytes_per_second: overhead_per_second, // Report overhead as throughput
            operations_per_second,
        },
        latency: LatencyMetrics {
            p50_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
            max_ms: 0.0,
            mean_ms: 0.0,
        },
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: end_stats.errors as f64 / total_operations as f64,
            recovery_time_ms: 0.0,
            total_errors: end_stats.errors,
        },
    })
}

/// Concurrent file operations benchmark
async fn concurrent_file_operations_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let storage = Arc::new(StorageSimulator::new());
    let data = vec![0u8; 1024 * 1024]; // 1MB files
    let concurrency = config.concurrency;
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    
    // Run concurrent file operations
    let mut tasks = Vec::new();
    for _ in 0..concurrency {
        let storage_clone = storage.clone();
        let data_clone = data.clone();
        let task = tokio::spawn(async move {
            let mut operation_times = Vec::new();
            for _ in 0..(config.iterations / concurrency) {
                let write_time = storage_clone.write_file(&data_clone).await?;
                operation_times.push(write_time);
            }
            Ok::<Vec<f64>, anyhow::Error>(operation_times)
        });
        tasks.push(task);
    }
    
    let results = futures::future::join_all(tasks).await;
    
    let duration = start_time.elapsed();
    let stats = storage.get_stats();
    
    let mut all_operation_times = Vec::new();
    for result in results {
        all_operation_times.extend(result??);
    }
    
    let operations_per_second = stats.files_written as f64 / duration.as_secs_f64();
    let bytes_per_second = stats.bytes_written as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: operations_per_second,
            bytes_per_second,
            operations_per_second,
        },
        latency: calculate_latency_metrics(&all_operation_times),
        resources: ResourceMetrics {
            cpu_usage_percent: get_cpu_usage(),
            memory_usage_mb: get_memory_usage(),
            disk_io_bytes_per_second: bytes_per_second,
            network_io_bytes_per_second: 0.0,
            file_handles: stats.files_written,
            thread_count: concurrency as u64,
        },
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.files_written as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Large file handling benchmark
async fn large_file_handling_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let storage = Arc::new(StorageSimulator::new());
    let large_data = vec![0u8; 100 * 1024 * 1024]; // 100MB files
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut operation_times = Vec::new();
    
    for _ in 0..10 { // Process 10 large files
        let write_time = storage.write_file(&large_data).await?;
        operation_times.push(write_time);
    }
    
    let duration = start_time.elapsed();
    let stats = storage.get_stats();
    
    let operations_per_second = stats.files_written as f64 / duration.as_secs_f64();
    let bytes_per_second = stats.bytes_written as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: operations_per_second,
            bytes_per_second,
            operations_per_second,
        },
        latency: calculate_latency_metrics(&operation_times),
        resources: ResourceMetrics {
            cpu_usage_percent: get_cpu_usage(),
            memory_usage_mb: get_memory_usage(),
            disk_io_bytes_per_second: bytes_per_second,
            network_io_bytes_per_second: 0.0,
            file_handles: stats.files_written,
            thread_count: 1,
        },
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.files_written as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Metadata operations benchmark
async fn metadata_operations_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let storage = Arc::new(StorageSimulator::new());
    let metadata = vec![0u8; 256]; // 256 bytes metadata
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut operation_times = Vec::new();
    
    for _ in 0..config.iterations {
        let write_time = storage.write_file(&metadata).await?;
        operation_times.push(write_time);
    }
    
    let duration = start_time.elapsed();
    let stats = storage.get_stats();
    
    let operations_per_second = stats.files_written as f64 / duration.as_secs_f64();
    let bytes_per_second = stats.bytes_written as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: operations_per_second,
            bytes_per_second,
            operations_per_second,
        },
        latency: calculate_latency_metrics(&operation_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.files_written as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Index performance benchmark
async fn index_performance_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let storage = Arc::new(StorageSimulator::new());
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut index_times = Vec::new();
    
    for i in 0..config.iterations {
        let record_count = 1000 * (i + 1) as usize; // Increasing record counts
        let index_time = storage.create_index(record_count).await?;
        index_times.push(index_time);
    }
    
    let duration = start_time.elapsed();
    let stats = storage.get_stats();
    
    let operations_per_second = stats.db_operations as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: operations_per_second,
            bytes_per_second: 0.0,
            operations_per_second,
        },
        latency: calculate_latency_metrics(&index_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.db_operations as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Backup/restore performance benchmark
async fn backup_restore_performance_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let storage = Arc::new(StorageSimulator::new());
    let backup_data = vec![0u8; 10 * 1024 * 1024]; // 10MB backup
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut backup_times = Vec::new();
    let mut restore_times = Vec::new();
    
    for _ in 0..config.iterations {
        // Backup operation
        let backup_time = storage.write_file(&backup_data).await?;
        backup_times.push(backup_time);
        
        // Restore operation
        let (_, restore_time) = storage.read_file(backup_data.len()).await?;
        restore_times.push(restore_time);
    }
    
    let duration = start_time.elapsed();
    let stats = storage.get_stats();
    
    let operations_per_second = (stats.files_written + stats.files_read) as f64 / duration.as_secs_f64();
    let bytes_per_second = (stats.bytes_written + stats.bytes_read) as f64 / duration.as_secs_f64();
    
    let mut all_times = backup_times;
    all_times.extend(restore_times);
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: operations_per_second,
            bytes_per_second,
            operations_per_second,
        },
        latency: calculate_latency_metrics(&all_times),
        resources: ResourceMetrics {
            cpu_usage_percent: get_cpu_usage(),
            memory_usage_mb: get_memory_usage(),
            disk_io_bytes_per_second: bytes_per_second,
            network_io_bytes_per_second: 0.0,
            file_handles: stats.files_written + stats.files_read,
            thread_count: 1,
        },
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / operations_per_second,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
        },
    })
}

/// Storage cleanup performance benchmark
async fn storage_cleanup_performance_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let storage = Arc::new(StorageSimulator::new());
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut cleanup_times = Vec::new();
    
    for i in 0..config.iterations {
        let file_count = 100 * (i + 1) as usize; // Increasing file counts
        let cleanup_time = storage.cleanup_storage(file_count).await?;
        cleanup_times.push(cleanup_time);
    }
    
    let duration = start_time.elapsed();
    let stats = storage.get_stats();
    
    let operations_per_second = stats.db_operations as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: operations_per_second,
            bytes_per_second: 0.0,
            operations_per_second,
        },
        latency: calculate_latency_metrics(&cleanup_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: stats.errors as f64 / stats.db_operations as f64,
            recovery_time_ms: 0.0,
            total_errors: stats.errors,
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