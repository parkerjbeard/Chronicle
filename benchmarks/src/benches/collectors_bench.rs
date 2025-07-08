//! Collectors performance benchmarks
//!
//! Tests the performance of various Chronicle collectors including CPU usage,
//! memory consumption, and event processing throughput.

use crate::{
    BenchmarkComponent, BenchmarkConfig, BenchmarkResult, ErrorMetrics, LatencyMetrics,
    PerformanceMetrics, ResourceMetrics, ThroughputMetrics,
};
use anyhow::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time;

/// Collector benchmark test cases
const BENCHMARK_TESTS: &[&str] = &[
    "keyboard_collector_performance",
    "mouse_collector_performance",
    "window_collector_performance",
    "filesystem_collector_performance",
    "network_collector_performance",
    "audio_collector_performance",
    "screen_collector_performance",
    "all_collectors_combined",
    "collector_startup_time",
    "collector_memory_usage",
    "collector_cpu_usage",
    "collector_error_handling",
];

/// Simulated collector types
#[derive(Debug, Clone, Copy)]
enum CollectorType {
    Keyboard,
    Mouse,
    Window,
    Filesystem,
    Network,
    Audio,
    Screen,
}

impl CollectorType {
    fn name(&self) -> &'static str {
        match self {
            CollectorType::Keyboard => "keyboard",
            CollectorType::Mouse => "mouse",
            CollectorType::Window => "window",
            CollectorType::Filesystem => "filesystem",
            CollectorType::Network => "network",
            CollectorType::Audio => "audio",
            CollectorType::Screen => "screen",
        }
    }
    
    fn expected_cpu_usage(&self) -> f64 {
        match self {
            CollectorType::Keyboard => 0.1,
            CollectorType::Mouse => 0.1,
            CollectorType::Window => 0.2,
            CollectorType::Filesystem => 0.5,
            CollectorType::Network => 0.3,
            CollectorType::Audio => 0.8,
            CollectorType::Screen => 1.0,
        }
    }
    
    fn expected_memory_usage(&self) -> f64 {
        match self {
            CollectorType::Keyboard => 2.0,
            CollectorType::Mouse => 2.0,
            CollectorType::Window => 5.0,
            CollectorType::Filesystem => 10.0,
            CollectorType::Network => 8.0,
            CollectorType::Audio => 15.0,
            CollectorType::Screen => 20.0,
        }
    }
}

/// Simulated collector implementation
struct CollectorSimulator {
    collector_type: CollectorType,
    events_collected: AtomicU64,
    bytes_processed: AtomicU64,
    errors: AtomicU64,
    is_running: AtomicU64,
    start_time: Instant,
}

impl CollectorSimulator {
    fn new(collector_type: CollectorType) -> Self {
        Self {
            collector_type,
            events_collected: AtomicU64::new(0),
            bytes_processed: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            is_running: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }
    
    async fn start(&self) -> Result<()> {
        // Simulate collector startup time
        let startup_delay = match self.collector_type {
            CollectorType::Keyboard => Duration::from_millis(10),
            CollectorType::Mouse => Duration::from_millis(10),
            CollectorType::Window => Duration::from_millis(50),
            CollectorType::Filesystem => Duration::from_millis(100),
            CollectorType::Network => Duration::from_millis(200),
            CollectorType::Audio => Duration::from_millis(300),
            CollectorType::Screen => Duration::from_millis(500),
        };
        
        time::sleep(startup_delay).await;
        self.is_running.store(1, Ordering::Relaxed);
        Ok(())
    }
    
    async fn stop(&self) -> Result<()> {
        self.is_running.store(0, Ordering::Relaxed);
        Ok(())
    }
    
    async fn collect_events(&self, duration: Duration) -> Result<Vec<f64>> {
        let mut latencies = Vec::new();
        let start_time = Instant::now();
        
        while start_time.elapsed() < duration {
            let event_start = Instant::now();
            
            // Simulate event collection based on collector type
            let event_delay = match self.collector_type {
                CollectorType::Keyboard => Duration::from_millis(1),
                CollectorType::Mouse => Duration::from_millis(1),
                CollectorType::Window => Duration::from_millis(100),
                CollectorType::Filesystem => Duration::from_millis(50),
                CollectorType::Network => Duration::from_millis(10),
                CollectorType::Audio => Duration::from_millis(20),
                CollectorType::Screen => Duration::from_millis(33), // ~30 FPS
            };
            
            time::sleep(event_delay).await;
            
            // Simulate event processing
            let event_size = match self.collector_type {
                CollectorType::Keyboard => 64,
                CollectorType::Mouse => 32,
                CollectorType::Window => 256,
                CollectorType::Filesystem => 512,
                CollectorType::Network => 1024,
                CollectorType::Audio => 4096,
                CollectorType::Screen => 65536, // 64KB for screen data
            };
            
            self.events_collected.fetch_add(1, Ordering::Relaxed);
            self.bytes_processed.fetch_add(event_size, Ordering::Relaxed);
            
            // Simulate occasional errors
            if rand::random::<f64>() < 0.001 {
                self.errors.fetch_add(1, Ordering::Relaxed);
            }
            
            latencies.push(event_start.elapsed().as_nanos() as f64 / 1_000_000.0);
        }
        
        Ok(latencies)
    }
    
    fn get_stats(&self) -> (u64, u64, u64) {
        (
            self.events_collected.load(Ordering::Relaxed),
            self.bytes_processed.load(Ordering::Relaxed),
            self.errors.load(Ordering::Relaxed),
        )
    }
    
    fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed) == 1
    }
}

/// Run a specific collector benchmark
pub async fn run_benchmark(test_name: &str, config: &BenchmarkConfig) -> Result<BenchmarkResult> {
    let start_time = Instant::now();
    
    let result = match test_name {
        "keyboard_collector_performance" => {
            single_collector_benchmark(CollectorType::Keyboard, config).await
        }
        "mouse_collector_performance" => {
            single_collector_benchmark(CollectorType::Mouse, config).await
        }
        "window_collector_performance" => {
            single_collector_benchmark(CollectorType::Window, config).await
        }
        "filesystem_collector_performance" => {
            single_collector_benchmark(CollectorType::Filesystem, config).await
        }
        "network_collector_performance" => {
            single_collector_benchmark(CollectorType::Network, config).await
        }
        "audio_collector_performance" => {
            single_collector_benchmark(CollectorType::Audio, config).await
        }
        "screen_collector_performance" => {
            single_collector_benchmark(CollectorType::Screen, config).await
        }
        "all_collectors_combined" => all_collectors_combined_benchmark(config).await,
        "collector_startup_time" => collector_startup_time_benchmark(config).await,
        "collector_memory_usage" => collector_memory_usage_benchmark(config).await,
        "collector_cpu_usage" => collector_cpu_usage_benchmark(config).await,
        "collector_error_handling" => collector_error_handling_benchmark(config).await,
        _ => return Err(anyhow::anyhow!("Unknown benchmark test: {}", test_name)),
    };

    let duration = start_time.elapsed();
    
    match result {
        Ok(mut metrics) => {
            metrics.timestamp = chrono::Utc::now();
            
            // Check if performance targets are met
            let passed = metrics.resources.cpu_usage_percent <= config.targets.collectors_cpu_usage_percent;
            
            Ok(BenchmarkResult {
                component: BenchmarkComponent::Collectors,
                test_name: test_name.to_string(),
                metrics,
                passed,
                notes: Some(format!("Completed in {:.2?}", duration)),
            })
        }
        Err(e) => {
            let metrics = create_error_metrics();
            
            Ok(BenchmarkResult {
                component: BenchmarkComponent::Collectors,
                test_name: test_name.to_string(),
                metrics,
                passed: false,
                notes: Some(format!("Failed: {}", e)),
            })
        }
    }
}

/// Run all collector benchmarks
pub async fn run_all_benchmarks(config: &BenchmarkConfig) -> Result<Vec<BenchmarkResult>> {
    let mut results = Vec::new();
    
    for test_name in BENCHMARK_TESTS {
        let result = run_benchmark(test_name, config).await?;
        results.push(result);
    }
    
    Ok(results)
}

/// Single collector performance benchmark
async fn single_collector_benchmark(
    collector_type: CollectorType,
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let collector = Arc::new(CollectorSimulator::new(collector_type));
    
    // Warmup
    time::sleep(config.warmup_duration).await;
    
    // Start collector
    collector.start().await?;
    
    let start_time = Instant::now();
    let system_start = get_system_snapshot();
    
    // Collect events for the specified duration
    let latencies = collector.collect_events(config.duration).await?;
    
    let duration = start_time.elapsed();
    let system_end = get_system_snapshot();
    
    // Stop collector
    collector.stop().await?;
    
    let (events, bytes, errors) = collector.get_stats();
    
    // Calculate metrics
    let events_per_second = events as f64 / duration.as_secs_f64();
    let bytes_per_second = bytes as f64 / duration.as_secs_f64();
    
    let cpu_usage = calculate_cpu_usage(&system_start, &system_end);
    let memory_usage = calculate_memory_usage(&system_start, &system_end);
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second,
            bytes_per_second,
            operations_per_second: events_per_second,
        },
        latency: calculate_latency_metrics(&latencies),
        resources: ResourceMetrics {
            cpu_usage_percent: cpu_usage,
            memory_usage_mb: memory_usage,
            disk_io_bytes_per_second: 0.0,
            network_io_bytes_per_second: 0.0,
            file_handles: 0,
            thread_count: 1,
        },
        errors: ErrorMetrics {
            error_rate: errors as f64 / events as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// All collectors combined benchmark
async fn all_collectors_combined_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let collector_types = [
        CollectorType::Keyboard,
        CollectorType::Mouse,
        CollectorType::Window,
        CollectorType::Filesystem,
        CollectorType::Network,
        CollectorType::Audio,
        CollectorType::Screen,
    ];
    
    let mut collectors = Vec::new();
    for collector_type in collector_types {
        collectors.push(Arc::new(CollectorSimulator::new(collector_type)));
    }
    
    // Warmup
    time::sleep(config.warmup_duration).await;
    
    // Start all collectors
    for collector in &collectors {
        collector.start().await?;
    }
    
    let start_time = Instant::now();
    let system_start = get_system_snapshot();
    
    // Run all collectors concurrently
    let mut tasks = Vec::new();
    for collector in &collectors {
        let collector_clone = collector.clone();
        let task = tokio::spawn(async move {
            collector_clone.collect_events(config.duration).await
        });
        tasks.push(task);
    }
    
    let results = futures::future::join_all(tasks).await;
    
    let duration = start_time.elapsed();
    let system_end = get_system_snapshot();
    
    // Stop all collectors
    for collector in &collectors {
        collector.stop().await?;
    }
    
    // Aggregate results
    let mut total_events = 0;
    let mut total_bytes = 0;
    let mut total_errors = 0;
    let mut all_latencies = Vec::new();
    
    for (i, result) in results.into_iter().enumerate() {
        let latencies = result??;
        all_latencies.extend(latencies);
        
        let (events, bytes, errors) = collectors[i].get_stats();
        total_events += events;
        total_bytes += bytes;
        total_errors += errors;
    }
    
    let events_per_second = total_events as f64 / duration.as_secs_f64();
    let bytes_per_second = total_bytes as f64 / duration.as_secs_f64();
    
    let cpu_usage = calculate_cpu_usage(&system_start, &system_end);
    let memory_usage = calculate_memory_usage(&system_start, &system_end);
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second,
            bytes_per_second,
            operations_per_second: events_per_second,
        },
        latency: calculate_latency_metrics(&all_latencies),
        resources: ResourceMetrics {
            cpu_usage_percent: cpu_usage,
            memory_usage_mb: memory_usage,
            disk_io_bytes_per_second: 0.0,
            network_io_bytes_per_second: 0.0,
            file_handles: 0,
            thread_count: collectors.len() as u64,
        },
        errors: ErrorMetrics {
            error_rate: total_errors as f64 / total_events as f64,
            recovery_time_ms: 0.0,
            total_errors: total_errors,
        },
    })
}

/// Collector startup time benchmark
async fn collector_startup_time_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let collector_types = [
        CollectorType::Keyboard,
        CollectorType::Mouse,
        CollectorType::Window,
        CollectorType::Filesystem,
        CollectorType::Network,
        CollectorType::Audio,
        CollectorType::Screen,
    ];
    
    let mut startup_times = Vec::new();
    
    for collector_type in collector_types {
        let collector = CollectorSimulator::new(collector_type);
        
        let start_time = Instant::now();
        collector.start().await?;
        let startup_time = start_time.elapsed();
        
        startup_times.push(startup_time.as_nanos() as f64 / 1_000_000.0);
        
        collector.stop().await?;
    }
    
    let mean_startup_time = startup_times.iter().sum::<f64>() / startup_times.len() as f64;
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: 0.0,
            bytes_per_second: 0.0,
            operations_per_second: 1.0 / (mean_startup_time / 1000.0), // ops per second
        },
        latency: calculate_latency_metrics(&startup_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: 0.0,
            recovery_time_ms: 0.0,
            total_errors: 0,
        },
    })
}

/// Collector memory usage benchmark
async fn collector_memory_usage_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let collector_types = [
        CollectorType::Keyboard,
        CollectorType::Mouse,
        CollectorType::Window,
        CollectorType::Filesystem,
        CollectorType::Network,
        CollectorType::Audio,
        CollectorType::Screen,
    ];
    
    let mut memory_usage_samples = Vec::new();
    
    for collector_type in collector_types {
        let collector = Arc::new(CollectorSimulator::new(collector_type));
        
        collector.start().await?;
        
        // Monitor memory usage over time
        let monitoring_duration = Duration::from_secs(5);
        let start_time = Instant::now();
        
        while start_time.elapsed() < monitoring_duration {
            let system_snapshot = get_system_snapshot();
            memory_usage_samples.push(system_snapshot.memory_usage_mb);
            
            // Simulate collector activity
            let _ = collector.collect_events(Duration::from_millis(100)).await;
            
            time::sleep(Duration::from_millis(100)).await;
        }
        
        collector.stop().await?;
    }
    
    let mean_memory_usage = memory_usage_samples.iter().sum::<f64>() / memory_usage_samples.len() as f64;
    let max_memory_usage = memory_usage_samples.iter().fold(0.0, |max, &val| max.max(val));
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: 0.0,
            bytes_per_second: 0.0,
            operations_per_second: 0.0,
        },
        latency: LatencyMetrics {
            p50_ms: mean_memory_usage,
            p95_ms: max_memory_usage,
            p99_ms: max_memory_usage,
            max_ms: max_memory_usage,
            mean_ms: mean_memory_usage,
        },
        resources: ResourceMetrics {
            cpu_usage_percent: 0.0,
            memory_usage_mb: mean_memory_usage,
            disk_io_bytes_per_second: 0.0,
            network_io_bytes_per_second: 0.0,
            file_handles: 0,
            thread_count: collector_types.len() as u64,
        },
        errors: ErrorMetrics {
            error_rate: 0.0,
            recovery_time_ms: 0.0,
            total_errors: 0,
        },
    })
}

/// Collector CPU usage benchmark
async fn collector_cpu_usage_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let collector_types = [
        CollectorType::Keyboard,
        CollectorType::Mouse,
        CollectorType::Window,
        CollectorType::Filesystem,
        CollectorType::Network,
        CollectorType::Audio,
        CollectorType::Screen,
    ];
    
    let mut cpu_usage_samples = Vec::new();
    
    for collector_type in collector_types {
        let collector = Arc::new(CollectorSimulator::new(collector_type));
        
        collector.start().await?;
        
        // Monitor CPU usage over time
        let monitoring_duration = Duration::from_secs(5);
        let start_time = Instant::now();
        
        while start_time.elapsed() < monitoring_duration {
            let system_snapshot = get_system_snapshot();
            cpu_usage_samples.push(system_snapshot.cpu_usage_percent);
            
            // Simulate collector activity
            let _ = collector.collect_events(Duration::from_millis(100)).await;
            
            time::sleep(Duration::from_millis(100)).await;
        }
        
        collector.stop().await?;
    }
    
    let mean_cpu_usage = cpu_usage_samples.iter().sum::<f64>() / cpu_usage_samples.len() as f64;
    let max_cpu_usage = cpu_usage_samples.iter().fold(0.0, |max, &val| max.max(val));
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: 0.0,
            bytes_per_second: 0.0,
            operations_per_second: 0.0,
        },
        latency: LatencyMetrics {
            p50_ms: mean_cpu_usage,
            p95_ms: max_cpu_usage,
            p99_ms: max_cpu_usage,
            max_ms: max_cpu_usage,
            mean_ms: mean_cpu_usage,
        },
        resources: ResourceMetrics {
            cpu_usage_percent: mean_cpu_usage,
            memory_usage_mb: 0.0,
            disk_io_bytes_per_second: 0.0,
            network_io_bytes_per_second: 0.0,
            file_handles: 0,
            thread_count: collector_types.len() as u64,
        },
        errors: ErrorMetrics {
            error_rate: 0.0,
            recovery_time_ms: 0.0,
            total_errors: 0,
        },
    })
}

/// Collector error handling benchmark
async fn collector_error_handling_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let collector = Arc::new(CollectorSimulator::new(CollectorType::Keyboard));
    
    collector.start().await?;
    
    let start_time = Instant::now();
    let mut recovery_times = Vec::new();
    
    // Simulate error conditions and recovery
    for _ in 0..10 {
        let error_start = Instant::now();
        
        // Simulate error by stopping collector
        collector.stop().await?;
        
        // Simulate recovery by restarting
        collector.start().await?;
        
        recovery_times.push(error_start.elapsed().as_nanos() as f64 / 1_000_000.0);
        
        time::sleep(Duration::from_millis(100)).await;
    }
    
    let duration = start_time.elapsed();
    let (events, bytes, errors) = collector.get_stats();
    
    collector.stop().await?;
    
    let mean_recovery_time = recovery_times.iter().sum::<f64>() / recovery_times.len() as f64;
    let max_recovery_time = recovery_times.iter().fold(0.0, |max, &val| max.max(val));
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: events as f64 / duration.as_secs_f64(),
            bytes_per_second: bytes as f64 / duration.as_secs_f64(),
            operations_per_second: 10.0 / duration.as_secs_f64(), // recovery operations
        },
        latency: calculate_latency_metrics(&recovery_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: 10.0 / (events as f64 + 10.0), // 10 simulated errors
            recovery_time_ms: mean_recovery_time,
            total_errors: 10,
        },
    })
}

/// System snapshot for monitoring
#[derive(Debug, Clone)]
struct SystemSnapshot {
    timestamp: Instant,
    cpu_usage_percent: f64,
    memory_usage_mb: f64,
}

/// Get a system snapshot
fn get_system_snapshot() -> SystemSnapshot {
    let mut system = sysinfo::System::new_all();
    system.refresh_all();
    
    SystemSnapshot {
        timestamp: Instant::now(),
        cpu_usage_percent: system.global_cpu_info().cpu_usage() as f64,
        memory_usage_mb: system.used_memory() as f64 / 1024.0 / 1024.0,
    }
}

/// Calculate CPU usage between two snapshots
fn calculate_cpu_usage(start: &SystemSnapshot, end: &SystemSnapshot) -> f64 {
    (start.cpu_usage_percent + end.cpu_usage_percent) / 2.0
}

/// Calculate memory usage between two snapshots
fn calculate_memory_usage(start: &SystemSnapshot, end: &SystemSnapshot) -> f64 {
    end.memory_usage_mb - start.memory_usage_mb
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
    let memory_usage = system.used_memory() as f64 / 1024.0 / 1024.0;
    
    ResourceMetrics {
        cpu_usage_percent: cpu_usage,
        memory_usage_mb: memory_usage,
        disk_io_bytes_per_second: 0.0,
        network_io_bytes_per_second: 0.0,
        file_handles: 0,
        thread_count: system.processes().len() as u64,
    }
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