//! Metrics collection and management for Chronicle benchmarks

use anyhow::Result;
use prometheus::{Counter, Gauge, Histogram, Registry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global metrics registry
static mut METRICS_REGISTRY: Option<Arc<MetricsRegistry>> = None;

/// Initialize the global metrics registry
pub fn init() -> Result<()> {
    unsafe {
        if METRICS_REGISTRY.is_none() {
            METRICS_REGISTRY = Some(Arc::new(MetricsRegistry::new()?));
        }
    }
    Ok(())
}

/// Get the global metrics registry
pub fn registry() -> &'static Arc<MetricsRegistry> {
    unsafe {
        METRICS_REGISTRY.as_ref().expect("Metrics registry not initialized")
    }
}

/// Chronicle metrics registry
pub struct MetricsRegistry {
    prometheus_registry: Registry,
    
    // Benchmark metrics
    benchmark_duration: Histogram,
    benchmark_iterations: Counter,
    benchmark_errors: Counter,
    
    // Performance metrics
    throughput_events_per_second: Gauge,
    throughput_bytes_per_second: Gauge,
    latency_p50: Gauge,
    latency_p95: Gauge,
    latency_p99: Gauge,
    
    // System metrics
    cpu_usage_percent: Gauge,
    memory_usage_mb: Gauge,
    disk_io_bytes_per_second: Gauge,
    network_io_bytes_per_second: Gauge,
    
    // Component-specific metrics
    ring_buffer_events_per_second: Gauge,
    collectors_cpu_usage: Gauge,
    packer_throughput_mb_per_hour: Gauge,
    search_latency_ms: Gauge,
    storage_overhead_mb: Gauge,
    
    // Custom metrics
    custom_metrics: Arc<RwLock<HashMap<String, CustomMetric>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CustomMetric {
    Counter(f64),
    Gauge(f64),
    Histogram(Vec<f64>),
}

impl MetricsRegistry {
    pub fn new() -> Result<Self> {
        let prometheus_registry = Registry::new();
        
        // Create benchmark metrics
        let benchmark_duration = Histogram::new(
            "benchmark_duration_seconds",
            "Duration of benchmark execution in seconds"
        )?;
        prometheus_registry.register(Box::new(benchmark_duration.clone()))?;
        
        let benchmark_iterations = Counter::new(
            "benchmark_iterations_total",
            "Total number of benchmark iterations"
        )?;
        prometheus_registry.register(Box::new(benchmark_iterations.clone()))?;
        
        let benchmark_errors = Counter::new(
            "benchmark_errors_total",
            "Total number of benchmark errors"
        )?;
        prometheus_registry.register(Box::new(benchmark_errors.clone()))?;
        
        // Create performance metrics
        let throughput_events_per_second = Gauge::new(
            "throughput_events_per_second",
            "Events processed per second"
        )?;
        prometheus_registry.register(Box::new(throughput_events_per_second.clone()))?;
        
        let throughput_bytes_per_second = Gauge::new(
            "throughput_bytes_per_second",
            "Bytes processed per second"
        )?;
        prometheus_registry.register(Box::new(throughput_bytes_per_second.clone()))?;
        
        let latency_p50 = Gauge::new(
            "latency_p50_milliseconds",
            "50th percentile latency in milliseconds"
        )?;
        prometheus_registry.register(Box::new(latency_p50.clone()))?;
        
        let latency_p95 = Gauge::new(
            "latency_p95_milliseconds",
            "95th percentile latency in milliseconds"
        )?;
        prometheus_registry.register(Box::new(latency_p95.clone()))?;
        
        let latency_p99 = Gauge::new(
            "latency_p99_milliseconds",
            "99th percentile latency in milliseconds"
        )?;
        prometheus_registry.register(Box::new(latency_p99.clone()))?;
        
        // Create system metrics
        let cpu_usage_percent = Gauge::new(
            "system_cpu_usage_percent",
            "System CPU usage percentage"
        )?;
        prometheus_registry.register(Box::new(cpu_usage_percent.clone()))?;
        
        let memory_usage_mb = Gauge::new(
            "system_memory_usage_mb",
            "System memory usage in megabytes"
        )?;
        prometheus_registry.register(Box::new(memory_usage_mb.clone()))?;
        
        let disk_io_bytes_per_second = Gauge::new(
            "system_disk_io_bytes_per_second",
            "System disk I/O in bytes per second"
        )?;
        prometheus_registry.register(Box::new(disk_io_bytes_per_second.clone()))?;
        
        let network_io_bytes_per_second = Gauge::new(
            "system_network_io_bytes_per_second",
            "System network I/O in bytes per second"
        )?;
        prometheus_registry.register(Box::new(network_io_bytes_per_second.clone()))?;
        
        // Create component-specific metrics
        let ring_buffer_events_per_second = Gauge::new(
            "ring_buffer_events_per_second",
            "Ring buffer events processed per second"
        )?;
        prometheus_registry.register(Box::new(ring_buffer_events_per_second.clone()))?;
        
        let collectors_cpu_usage = Gauge::new(
            "collectors_cpu_usage_percent",
            "Collectors CPU usage percentage"
        )?;
        prometheus_registry.register(Box::new(collectors_cpu_usage.clone()))?;
        
        let packer_throughput_mb_per_hour = Gauge::new(
            "packer_throughput_mb_per_hour",
            "Packer throughput in megabytes per hour"
        )?;
        prometheus_registry.register(Box::new(packer_throughput_mb_per_hour.clone()))?;
        
        let search_latency_ms = Gauge::new(
            "search_latency_milliseconds",
            "Search query latency in milliseconds"
        )?;
        prometheus_registry.register(Box::new(search_latency_ms.clone()))?;
        
        let storage_overhead_mb = Gauge::new(
            "storage_overhead_mb",
            "Storage overhead in megabytes"
        )?;
        prometheus_registry.register(Box::new(storage_overhead_mb.clone()))?;
        
        Ok(Self {
            prometheus_registry,
            benchmark_duration,
            benchmark_iterations,
            benchmark_errors,
            throughput_events_per_second,
            throughput_bytes_per_second,
            latency_p50,
            latency_p95,
            latency_p99,
            cpu_usage_percent,
            memory_usage_mb,
            disk_io_bytes_per_second,
            network_io_bytes_per_second,
            ring_buffer_events_per_second,
            collectors_cpu_usage,
            packer_throughput_mb_per_hour,
            search_latency_ms,
            storage_overhead_mb,
            custom_metrics: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Record benchmark metrics
    pub fn record_benchmark_metrics(&self, metrics: &crate::PerformanceMetrics) {
        // Throughput metrics
        self.throughput_events_per_second.set(metrics.throughput.events_per_second);
        self.throughput_bytes_per_second.set(metrics.throughput.bytes_per_second);
        
        // Latency metrics
        self.latency_p50.set(metrics.latency.p50_ms);
        self.latency_p95.set(metrics.latency.p95_ms);
        self.latency_p99.set(metrics.latency.p99_ms);
        
        // Resource metrics
        self.cpu_usage_percent.set(metrics.resources.cpu_usage_percent);
        self.memory_usage_mb.set(metrics.resources.memory_usage_mb);
        self.disk_io_bytes_per_second.set(metrics.resources.disk_io_bytes_per_second);
        self.network_io_bytes_per_second.set(metrics.resources.network_io_bytes_per_second);
        
        // Error metrics
        if metrics.errors.total_errors > 0 {
            self.benchmark_errors.inc_by(metrics.errors.total_errors as f64);
        }
    }
    
    /// Record component-specific metrics
    pub fn record_component_metrics(&self, component: crate::BenchmarkComponent, metrics: &crate::PerformanceMetrics) {
        match component {
            crate::BenchmarkComponent::RingBuffer => {
                self.ring_buffer_events_per_second.set(metrics.throughput.events_per_second);
            }
            crate::BenchmarkComponent::Collectors => {
                self.collectors_cpu_usage.set(metrics.resources.cpu_usage_percent);
            }
            crate::BenchmarkComponent::Packer => {
                let mb_per_hour = metrics.throughput.bytes_per_second * 3600.0 / (1024.0 * 1024.0);
                self.packer_throughput_mb_per_hour.set(mb_per_hour);
            }
            crate::BenchmarkComponent::Search => {
                self.search_latency_ms.set(metrics.latency.p95_ms);
            }
            crate::BenchmarkComponent::Storage => {
                let overhead_mb = metrics.throughput.bytes_per_second * 86400.0 / (1024.0 * 1024.0);
                self.storage_overhead_mb.set(overhead_mb);
            }
            crate::BenchmarkComponent::System => {
                // System metrics are already recorded in record_benchmark_metrics
            }
        }
    }
    
    /// Add custom metric
    pub async fn add_custom_metric(&self, name: String, metric: CustomMetric) {
        let mut custom_metrics = self.custom_metrics.write().await;
        custom_metrics.insert(name, metric);
    }
    
    /// Get custom metric
    pub async fn get_custom_metric(&self, name: &str) -> Option<CustomMetric> {
        let custom_metrics = self.custom_metrics.read().await;
        custom_metrics.get(name).cloned()
    }
    
    /// Get all custom metrics
    pub async fn get_all_custom_metrics(&self) -> HashMap<String, CustomMetric> {
        let custom_metrics = self.custom_metrics.read().await;
        custom_metrics.clone()
    }
    
    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> Result<String> {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.prometheus_registry.gather();
        Ok(encoder.encode_to_string(&metric_families)?)
    }
    
    /// Export metrics as JSON
    pub async fn export_json(&self) -> Result<String> {
        let custom_metrics = self.get_all_custom_metrics().await;
        
        let export = MetricsExport {
            benchmark: BenchmarkMetricsExport {
                iterations: self.benchmark_iterations.get(),
                errors: self.benchmark_errors.get(),
            },
            performance: PerformanceMetricsExport {
                throughput_events_per_second: self.throughput_events_per_second.get(),
                throughput_bytes_per_second: self.throughput_bytes_per_second.get(),
                latency_p50_ms: self.latency_p50.get(),
                latency_p95_ms: self.latency_p95.get(),
                latency_p99_ms: self.latency_p99.get(),
            },
            system: SystemMetricsExport {
                cpu_usage_percent: self.cpu_usage_percent.get(),
                memory_usage_mb: self.memory_usage_mb.get(),
                disk_io_bytes_per_second: self.disk_io_bytes_per_second.get(),
                network_io_bytes_per_second: self.network_io_bytes_per_second.get(),
            },
            components: ComponentMetricsExport {
                ring_buffer_events_per_second: self.ring_buffer_events_per_second.get(),
                collectors_cpu_usage_percent: self.collectors_cpu_usage.get(),
                packer_throughput_mb_per_hour: self.packer_throughput_mb_per_hour.get(),
                search_latency_ms: self.search_latency_ms.get(),
                storage_overhead_mb: self.storage_overhead_mb.get(),
            },
            custom: custom_metrics,
        };
        
        Ok(serde_json::to_string_pretty(&export)?)
    }
    
    /// Reset all metrics
    pub async fn reset(&self) {
        // Reset Prometheus metrics
        self.benchmark_iterations.reset();
        self.benchmark_errors.reset();
        
        self.throughput_events_per_second.set(0.0);
        self.throughput_bytes_per_second.set(0.0);
        self.latency_p50.set(0.0);
        self.latency_p95.set(0.0);
        self.latency_p99.set(0.0);
        
        self.cpu_usage_percent.set(0.0);
        self.memory_usage_mb.set(0.0);
        self.disk_io_bytes_per_second.set(0.0);
        self.network_io_bytes_per_second.set(0.0);
        
        self.ring_buffer_events_per_second.set(0.0);
        self.collectors_cpu_usage.set(0.0);
        self.packer_throughput_mb_per_hour.set(0.0);
        self.search_latency_ms.set(0.0);
        self.storage_overhead_mb.set(0.0);
        
        // Reset custom metrics
        let mut custom_metrics = self.custom_metrics.write().await;
        custom_metrics.clear();
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct MetricsExport {
    benchmark: BenchmarkMetricsExport,
    performance: PerformanceMetricsExport,
    system: SystemMetricsExport,
    components: ComponentMetricsExport,
    custom: HashMap<String, CustomMetric>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BenchmarkMetricsExport {
    iterations: f64,
    errors: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct PerformanceMetricsExport {
    throughput_events_per_second: f64,
    throughput_bytes_per_second: f64,
    latency_p50_ms: f64,
    latency_p95_ms: f64,
    latency_p99_ms: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SystemMetricsExport {
    cpu_usage_percent: f64,
    memory_usage_mb: f64,
    disk_io_bytes_per_second: f64,
    network_io_bytes_per_second: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ComponentMetricsExport {
    ring_buffer_events_per_second: f64,
    collectors_cpu_usage_percent: f64,
    packer_throughput_mb_per_hour: f64,
    search_latency_ms: f64,
    storage_overhead_mb: f64,
}

/// Convenience macros for metrics recording
#[macro_export]
macro_rules! record_benchmark_metric {
    ($metrics:expr) => {
        $crate::metrics::registry().record_benchmark_metrics($metrics);
    };
}

#[macro_export]
macro_rules! record_component_metric {
    ($component:expr, $metrics:expr) => {
        $crate::metrics::registry().record_component_metrics($component, $metrics);
    };
}

#[macro_export]
macro_rules! add_custom_metric {
    ($name:expr, $metric:expr) => {
        $crate::metrics::registry().add_custom_metric($name.to_string(), $metric).await;
    };
}