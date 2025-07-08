//! Chronicle Benchmarking and Performance Monitoring
//!
//! This crate provides comprehensive benchmarking and performance monitoring capabilities
//! for the Chronicle project, including:
//!
//! - Core component benchmarks (ring buffer, collectors, packer, search, storage)
//! - Real-time system monitoring
//! - Performance profiling and analysis
//! - Web-based monitoring dashboard
//! - Performance regression detection
//! - Optimization recommendations

pub mod analysis;
pub mod benches;
pub mod config;
pub mod dashboard;
pub mod metrics;
pub mod monitoring;
pub mod utils;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Performance targets for Chronicle components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTargets {
    /// Ring buffer target: >100,000 events/second
    pub ring_buffer_events_per_second: u64,
    /// Collectors target: <3% CPU usage total
    pub collectors_cpu_usage_percent: f64,
    /// Packer target: Process 1GB/hour
    pub packer_throughput_gb_per_hour: f64,
    /// Search target: <100ms for typical queries
    pub search_latency_ms: u64,
    /// Storage target: <10MB/day overhead
    pub storage_overhead_mb_per_day: f64,
}

impl Default for PerformanceTargets {
    fn default() -> Self {
        Self {
            ring_buffer_events_per_second: 100_000,
            collectors_cpu_usage_percent: 3.0,
            packer_throughput_gb_per_hour: 1.0,
            search_latency_ms: 100,
            storage_overhead_mb_per_day: 10.0,
        }
    }
}

/// Key performance metrics tracked by Chronicle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Timestamp of measurement
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Throughput metrics
    pub throughput: ThroughputMetrics,
    /// Latency metrics
    pub latency: LatencyMetrics,
    /// Resource usage metrics
    pub resources: ResourceMetrics,
    /// Error metrics
    pub errors: ErrorMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    pub events_per_second: f64,
    pub bytes_per_second: f64,
    pub operations_per_second: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyMetrics {
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub disk_io_bytes_per_second: f64,
    pub network_io_bytes_per_second: f64,
    pub file_handles: u64,
    pub thread_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    pub error_rate: f64,
    pub recovery_time_ms: f64,
    pub total_errors: u64,
}

/// Component being benchmarked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BenchmarkComponent {
    RingBuffer,
    Collectors,
    Packer,
    Search,
    Storage,
    System,
}

impl std::fmt::Display for BenchmarkComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BenchmarkComponent::RingBuffer => write!(f, "ring_buffer"),
            BenchmarkComponent::Collectors => write!(f, "collectors"),
            BenchmarkComponent::Packer => write!(f, "packer"),
            BenchmarkComponent::Search => write!(f, "search"),
            BenchmarkComponent::Storage => write!(f, "storage"),
            BenchmarkComponent::System => write!(f, "system"),
        }
    }
}

/// Benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub component: BenchmarkComponent,
    pub test_name: String,
    pub metrics: PerformanceMetrics,
    pub passed: bool,
    pub notes: Option<String>,
}

/// Performance test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    pub duration: Duration,
    pub warmup_duration: Duration,
    pub iterations: u32,
    pub concurrency: u32,
    pub data_size: u64,
    pub targets: PerformanceTargets,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(10),
            warmup_duration: Duration::from_secs(2),
            iterations: 100,
            concurrency: 1,
            data_size: 1024 * 1024, // 1MB
            targets: PerformanceTargets::default(),
        }
    }
}

/// Initialize the benchmarking system
pub fn init() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("chronicle_benchmarks=info")
        .init();

    // Initialize metrics collection
    metrics::init()?;

    Ok(())
}

/// Run a single benchmark
pub async fn run_benchmark(
    component: BenchmarkComponent,
    test_name: &str,
    config: &BenchmarkConfig,
) -> Result<BenchmarkResult> {
    match component {
        BenchmarkComponent::RingBuffer => {
            benches::ring_buffer_bench::run_benchmark(test_name, config).await
        }
        BenchmarkComponent::Collectors => {
            benches::collectors_bench::run_benchmark(test_name, config).await
        }
        BenchmarkComponent::Packer => {
            benches::packer_bench::run_benchmark(test_name, config).await
        }
        BenchmarkComponent::Search => {
            benches::search_bench::run_benchmark(test_name, config).await
        }
        BenchmarkComponent::Storage => {
            benches::storage_bench::run_benchmark(test_name, config).await
        }
        BenchmarkComponent::System => {
            monitoring::system_monitor::run_benchmark(test_name, config).await
        }
    }
}

/// Run all benchmarks
pub async fn run_all_benchmarks(config: &BenchmarkConfig) -> Result<Vec<BenchmarkResult>> {
    let components = [
        BenchmarkComponent::RingBuffer,
        BenchmarkComponent::Collectors,
        BenchmarkComponent::Packer,
        BenchmarkComponent::Search,
        BenchmarkComponent::Storage,
        BenchmarkComponent::System,
    ];

    let mut results = Vec::new();
    
    for component in components {
        let component_results = match component {
            BenchmarkComponent::RingBuffer => {
                benches::ring_buffer_bench::run_all_benchmarks(config).await?
            }
            BenchmarkComponent::Collectors => {
                benches::collectors_bench::run_all_benchmarks(config).await?
            }
            BenchmarkComponent::Packer => {
                benches::packer_bench::run_all_benchmarks(config).await?
            }
            BenchmarkComponent::Search => {
                benches::search_bench::run_all_benchmarks(config).await?
            }
            BenchmarkComponent::Storage => {
                benches::storage_bench::run_all_benchmarks(config).await?
            }
            BenchmarkComponent::System => {
                monitoring::system_monitor::run_all_benchmarks(config).await?
            }
        };
        
        results.extend(component_results);
    }

    Ok(results)
}