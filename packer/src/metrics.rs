//! Metrics and performance monitoring for Chronicle packer service
//!
//! This module provides comprehensive metrics collection and monitoring
//! for the packer service, including system metrics, performance counters,
//! and custom business metrics.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use prometheus::{
    Counter, Gauge, Histogram, HistogramOpts, IntCounter, IntGauge, Opts, Registry,
    TextEncoder, Encoder,
};
use serde::{Deserialize, Serialize};
use sysinfo::{System, SystemExt, ProcessExt, CpuExt, DiskExt, NetworkExt, NetworksExt};
use tokio::time::interval;

use crate::config::MetricsConfig;
use crate::error::{MetricsError, MetricsResult};

/// Metrics collector for Chronicle packer service
pub struct MetricsCollector {
    /// Configuration
    config: MetricsConfig,
    
    /// Prometheus registry
    registry: Registry,
    
    /// System information
    system: Arc<Mutex<System>>,
    
    /// Performance counters
    counters: PerformanceCounters,
    
    /// Custom metrics
    custom_metrics: HashMap<String, Box<dyn CustomMetric>>,
    
    /// Metrics collection start time
    start_time: Instant,
}

/// Performance counters for the packer service
#[derive(Clone)]
pub struct PerformanceCounters {
    // Processing metrics
    pub events_processed: IntCounter,
    pub events_failed: IntCounter,
    pub files_created: IntCounter,
    pub files_encrypted: IntCounter,
    pub bytes_processed: IntCounter,
    
    // Timing metrics
    pub processing_duration: Histogram,
    pub encryption_duration: Histogram,
    pub storage_duration: Histogram,
    pub integrity_check_duration: Histogram,
    
    // System metrics
    pub memory_usage: IntGauge,
    pub cpu_usage: Gauge,
    pub disk_usage: IntGauge,
    pub ring_buffer_size: IntGauge,
    pub ring_buffer_utilization: Gauge,
    
    // Error metrics
    pub ring_buffer_errors: IntCounter,
    pub storage_errors: IntCounter,
    pub encryption_errors: IntCounter,
    pub integrity_errors: IntCounter,
    
    // Business metrics
    pub daily_processing_time: Histogram,
    pub data_retention_violations: IntCounter,
    pub key_rotation_events: IntCounter,
}

/// Custom metric trait
pub trait CustomMetric: Send + Sync {
    fn collect(&self) -> MetricsResult<f64>;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
}

/// System performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// CPU usage percentage
    pub cpu_usage: f64,
    
    /// Memory usage in bytes
    pub memory_usage: u64,
    
    /// Available memory in bytes
    pub memory_available: u64,
    
    /// Disk usage in bytes
    pub disk_usage: u64,
    
    /// Available disk space in bytes
    pub disk_available: u64,
    
    /// Network bytes received
    pub network_rx: u64,
    
    /// Network bytes transmitted
    pub network_tx: u64,
    
    /// System load average
    pub load_average: f64,
    
    /// Number of processes
    pub process_count: usize,
    
    /// System uptime in seconds
    pub uptime: u64,
}

/// Performance statistics for the packer service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackerStats {
    /// Number of events processed
    pub events_processed: u64,
    
    /// Number of failed events
    pub events_failed: u64,
    
    /// Number of files created
    pub files_created: u64,
    
    /// Total bytes processed
    pub bytes_processed: u64,
    
    /// Average processing time in milliseconds
    pub avg_processing_time: f64,
    
    /// Success rate as percentage
    pub success_rate: f64,
    
    /// Service uptime in seconds
    pub uptime: u64,
    
    /// Last processing timestamp
    pub last_processing: u64,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(config: MetricsConfig) -> MetricsResult<Self> {
        let registry = Registry::new();
        let system = Arc::new(Mutex::new(System::new_all()));
        let counters = PerformanceCounters::new(&registry)?;
        
        let collector = Self {
            config,
            registry,
            system,
            counters,
            custom_metrics: HashMap::new(),
            start_time: Instant::now(),
        };
        
        Ok(collector)
    }
    
    /// Start the metrics collection service
    pub async fn start(&self) -> MetricsResult<()> {
        if !self.config.enabled {
            tracing::info!("Metrics collection disabled");
            return Ok(());
        }
        
        tracing::info!("Starting metrics collection service");
        
        // Start periodic collection
        self.start_periodic_collection().await;
        
        // Start metrics server if configured
        if self.config.port > 0 {
            self.start_metrics_server().await?;
        }
        
        Ok(())
    }
    
    /// Start periodic metrics collection
    async fn start_periodic_collection(&self) {
        let mut interval_timer = interval(Duration::from_secs(self.config.collection_interval as u64));
        let system = self.system.clone();
        let counters = self.counters.clone();
        
        tokio::spawn(async move {
            loop {
                interval_timer.tick().await;
                
                if let Err(e) = Self::collect_system_metrics(&system, &counters).await {
                    tracing::error!("Failed to collect system metrics: {}", e);
                }
            }
        });
    }
    
    /// Start metrics HTTP server
    async fn start_metrics_server(&self) -> MetricsResult<()> {
        let bind_addr = format!("{}:{}", self.config.bind_address, self.config.port);
        let registry = self.registry.clone();
        
        tokio::spawn(async move {
            let make_svc = hyper::service::make_service_fn(move |_conn| {
                let registry = registry.clone();
                async move {
                    Ok::<_, hyper::Error>(hyper::service::service_fn(move |req| {
                        let registry = registry.clone();
                        async move {
                            match req.uri().path() {
                                "/metrics" => {
                                    let encoder = TextEncoder::new();
                                    let metric_families = registry.gather();
                                    
                                    match encoder.encode_to_string(&metric_families) {
                                        Ok(output) => {
                                            Ok(hyper::Response::builder()
                                                .status(200)
                                                .header("Content-Type", "text/plain")
                                                .body(hyper::Body::from(output))
                                                .unwrap())
                                        }
                                        Err(_) => {
                                            Ok(hyper::Response::builder()
                                                .status(500)
                                                .body(hyper::Body::from("Internal Server Error"))
                                                .unwrap())
                                        }
                                    }
                                }
                                "/health" => {
                                    Ok(hyper::Response::builder()
                                        .status(200)
                                        .body(hyper::Body::from("OK"))
                                        .unwrap())
                                }
                                _ => {
                                    Ok(hyper::Response::builder()
                                        .status(404)
                                        .body(hyper::Body::from("Not Found"))
                                        .unwrap())
                                }
                            }
                        }
                    }))
                }
            });
            
            let server = hyper::Server::bind(&bind_addr.parse().unwrap())
                .serve(make_svc);
            
            tracing::info!("Metrics server listening on {}", bind_addr);
            
            if let Err(e) = server.await {
                tracing::error!("Metrics server error: {}", e);
            }
        });
        
        Ok(())
    }
    
    /// Collect system metrics
    async fn collect_system_metrics(
        system: &Arc<Mutex<System>>,
        counters: &PerformanceCounters,
    ) -> MetricsResult<()> {
        let mut sys = system.lock().unwrap();
        sys.refresh_all();
        
        // CPU usage
        let cpu_usage = sys.global_cpu_info().cpu_usage();
        counters.cpu_usage.set(cpu_usage as f64);
        
        // Memory usage
        let memory_usage = sys.used_memory();
        counters.memory_usage.set(memory_usage as i64);
        
        // Disk usage
        let disk_usage: u64 = sys.disks().iter().map(|disk| disk.total_space() - disk.available_space()).sum();
        counters.disk_usage.set(disk_usage as i64);
        
        // Process count
        let process_count = sys.processes().len();
        
        tracing::debug!("System metrics - CPU: {:.2}%, Memory: {} bytes, Disk: {} bytes, Processes: {}",
            cpu_usage, memory_usage, disk_usage, process_count);
        
        Ok(())
    }
    
    /// Record event processing
    pub fn record_event_processed(&self, count: u64) {
        self.counters.events_processed.inc_by(count);
    }
    
    /// Record event processing failure
    pub fn record_event_failed(&self, count: u64) {
        self.counters.events_failed.inc_by(count);
    }
    
    /// Record file creation
    pub fn record_file_created(&self, size: u64) {
        self.counters.files_created.inc();
        self.counters.bytes_processed.inc_by(size);
    }
    
    /// Record processing duration
    pub fn record_processing_duration(&self, duration: Duration) {
        self.counters.processing_duration.observe(duration.as_secs_f64());
    }
    
    /// Record encryption duration
    pub fn record_encryption_duration(&self, duration: Duration) {
        self.counters.encryption_duration.observe(duration.as_secs_f64());
    }
    
    /// Record storage duration
    pub fn record_storage_duration(&self, duration: Duration) {
        self.counters.storage_duration.observe(duration.as_secs_f64());
    }
    
    /// Record integrity check duration
    pub fn record_integrity_check_duration(&self, duration: Duration) {
        self.counters.integrity_check_duration.observe(duration.as_secs_f64());
    }
    
    /// Record ring buffer utilization
    pub fn record_ring_buffer_utilization(&self, size: u64, utilization: f64) {
        self.counters.ring_buffer_size.set(size as i64);
        self.counters.ring_buffer_utilization.set(utilization);
    }
    
    /// Record error by category
    pub fn record_error(&self, category: &str) {
        match category {
            "ring_buffer" => self.counters.ring_buffer_errors.inc(),
            "storage" => self.counters.storage_errors.inc(),
            "encryption" => self.counters.encryption_errors.inc(),
            "integrity" => self.counters.integrity_errors.inc(),
            _ => tracing::warn!("Unknown error category: {}", category),
        }
    }
    
    /// Record daily processing time
    pub fn record_daily_processing_time(&self, duration: Duration) {
        self.counters.daily_processing_time.observe(duration.as_secs_f64());
    }
    
    /// Record key rotation event
    pub fn record_key_rotation(&self) {
        self.counters.key_rotation_events.inc();
    }
    
    /// Get current packer statistics
    pub fn get_packer_stats(&self) -> PackerStats {
        let events_processed = self.counters.events_processed.get();
        let events_failed = self.counters.events_failed.get();
        let files_created = self.counters.files_created.get();
        let bytes_processed = self.counters.bytes_processed.get();
        
        let success_rate = if events_processed + events_failed > 0 {
            (events_processed as f64 / (events_processed + events_failed) as f64) * 100.0
        } else {
            0.0
        };
        
        // Calculate average processing time from histogram
        let avg_processing_time = self.counters.processing_duration.get_sample_sum() / 
                                 self.counters.processing_duration.get_sample_count() as f64;
        
        PackerStats {
            events_processed: events_processed as u64,
            events_failed: events_failed as u64,
            files_created: files_created as u64,
            bytes_processed: bytes_processed as u64,
            avg_processing_time: avg_processing_time * 1000.0, // Convert to milliseconds
            success_rate,
            uptime: self.start_time.elapsed().as_secs(),
            last_processing: chrono::Utc::now().timestamp() as u64,
        }
    }
    
    /// Get system metrics
    pub fn get_system_metrics(&self) -> SystemMetrics {
        let sys = self.system.lock().unwrap();
        
        SystemMetrics {
            cpu_usage: sys.global_cpu_info().cpu_usage() as f64,
            memory_usage: sys.used_memory(),
            memory_available: sys.available_memory(),
            disk_usage: sys.disks().iter().map(|d| d.total_space() - d.available_space()).sum(),
            disk_available: sys.disks().iter().map(|d| d.available_space()).sum(),
            network_rx: sys.networks().iter().map(|(_, net)| net.received()).sum(),
            network_tx: sys.networks().iter().map(|(_, net)| net.transmitted()).sum(),
            load_average: sys.load_average().one,
            process_count: sys.processes().len(),
            uptime: sys.uptime(),
        }
    }
    
    /// Register custom metric
    pub fn register_custom_metric(&mut self, metric: Box<dyn CustomMetric>) -> MetricsResult<()> {
        let name = metric.name().to_string();
        self.custom_metrics.insert(name, metric);
        Ok(())
    }
    
    /// Export metrics in specified format
    pub fn export_metrics(&self, format: &str) -> MetricsResult<String> {
        match format {
            "prometheus" => {
                let encoder = TextEncoder::new();
                let metric_families = self.registry.gather();
                encoder.encode_to_string(&metric_families)
                    .map_err(|e| MetricsError::ExportFailed { reason: e.to_string() })
            }
            "json" => {
                let stats = self.get_packer_stats();
                let system_metrics = self.get_system_metrics();
                
                let combined = serde_json::json!({
                    "packer_stats": stats,
                    "system_metrics": system_metrics,
                    "timestamp": chrono::Utc::now().timestamp()
                });
                
                serde_json::to_string_pretty(&combined)
                    .map_err(|e| MetricsError::ExportFailed { reason: e.to_string() })
            }
            _ => Err(MetricsError::ExportFailed { 
                reason: format!("Unsupported format: {}", format) 
            })
        }
    }
}

impl PerformanceCounters {
    /// Create new performance counters
    fn new(registry: &Registry) -> MetricsResult<Self> {
        let events_processed = IntCounter::new("events_processed_total", "Total number of events processed")
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(events_processed.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let events_failed = IntCounter::new("events_failed_total", "Total number of failed events")
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(events_failed.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let files_created = IntCounter::new("files_created_total", "Total number of files created")
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(files_created.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let files_encrypted = IntCounter::new("files_encrypted_total", "Total number of files encrypted")
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(files_encrypted.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let bytes_processed = IntCounter::new("bytes_processed_total", "Total bytes processed")
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(bytes_processed.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let processing_duration = Histogram::with_opts(
            HistogramOpts::new("processing_duration_seconds", "Event processing duration")
                .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0])
        ).map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(processing_duration.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let encryption_duration = Histogram::with_opts(
            HistogramOpts::new("encryption_duration_seconds", "Encryption duration")
                .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0])
        ).map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(encryption_duration.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let storage_duration = Histogram::with_opts(
            HistogramOpts::new("storage_duration_seconds", "Storage operation duration")
                .buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0])
        ).map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(storage_duration.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let integrity_check_duration = Histogram::with_opts(
            HistogramOpts::new("integrity_check_duration_seconds", "Integrity check duration")
                .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0])
        ).map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(integrity_check_duration.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let memory_usage = IntGauge::new("memory_usage_bytes", "Memory usage in bytes")
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(memory_usage.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let cpu_usage = Gauge::new("cpu_usage_percent", "CPU usage percentage")
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(cpu_usage.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let disk_usage = IntGauge::new("disk_usage_bytes", "Disk usage in bytes")
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(disk_usage.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let ring_buffer_size = IntGauge::new("ring_buffer_size_bytes", "Ring buffer size in bytes")
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(ring_buffer_size.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let ring_buffer_utilization = Gauge::new("ring_buffer_utilization_percent", "Ring buffer utilization percentage")
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(ring_buffer_utilization.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let ring_buffer_errors = IntCounter::new("ring_buffer_errors_total", "Ring buffer errors")
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(ring_buffer_errors.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let storage_errors = IntCounter::new("storage_errors_total", "Storage errors")
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(storage_errors.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let encryption_errors = IntCounter::new("encryption_errors_total", "Encryption errors")
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(encryption_errors.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let integrity_errors = IntCounter::new("integrity_errors_total", "Integrity check errors")
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(integrity_errors.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let daily_processing_time = Histogram::with_opts(
            HistogramOpts::new("daily_processing_time_seconds", "Daily processing time")
                .buckets(vec![60.0, 300.0, 600.0, 1800.0, 3600.0, 7200.0])
        ).map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(daily_processing_time.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let data_retention_violations = IntCounter::new("data_retention_violations_total", "Data retention violations")
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(data_retention_violations.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        let key_rotation_events = IntCounter::new("key_rotation_events_total", "Key rotation events")
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        registry.register(Box::new(key_rotation_events.clone()))
            .map_err(|e| MetricsError::RegistrationFailed { name: e.to_string() })?;
        
        Ok(Self {
            events_processed,
            events_failed,
            files_created,
            files_encrypted,
            bytes_processed,
            processing_duration,
            encryption_duration,
            storage_duration,
            integrity_check_duration,
            memory_usage,
            cpu_usage,
            disk_usage,
            ring_buffer_size,
            ring_buffer_utilization,
            ring_buffer_errors,
            storage_errors,
            encryption_errors,
            integrity_errors,
            daily_processing_time,
            data_retention_violations,
            key_rotation_events,
        })
    }
}

/// Example custom metric implementation
pub struct RingBufferHealthMetric {
    name: String,
    description: String,
}

impl RingBufferHealthMetric {
    pub fn new() -> Self {
        Self {
            name: "ring_buffer_health".to_string(),
            description: "Ring buffer health score (0-100)".to_string(),
        }
    }
}

impl CustomMetric for RingBufferHealthMetric {
    fn collect(&self) -> MetricsResult<f64> {
        // Placeholder implementation
        // In reality, this would check ring buffer health
        Ok(95.0)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        &self.description
    }
}

// Add hyper dependency for metrics server
extern crate hyper;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    fn test_config() -> MetricsConfig {
        MetricsConfig {
            enabled: true,
            bind_address: "127.0.0.1".to_string(),
            port: 0, // Use 0 to disable server in tests
            collection_interval: 1,
            export_format: "prometheus".to_string(),
            custom_metrics: HashMap::new(),
        }
    }
    
    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let config = test_config();
        let collector = MetricsCollector::new(config);
        assert!(collector.is_ok());
    }
    
    #[test]
    fn test_performance_counters() {
        let registry = Registry::new();
        let counters = PerformanceCounters::new(&registry);
        assert!(counters.is_ok());
        
        let counters = counters.unwrap();
        
        // Test counter operations
        counters.events_processed.inc();
        assert_eq!(counters.events_processed.get(), 1);
        
        counters.events_processed.inc_by(5);
        assert_eq!(counters.events_processed.get(), 6);
        
        // Test gauge operations
        counters.cpu_usage.set(75.5);
        assert_eq!(counters.cpu_usage.get(), 75.5);
        
        // Test histogram operations
        counters.processing_duration.observe(0.1);
        assert_eq!(counters.processing_duration.get_sample_count(), 1);
    }
    
    #[tokio::test]
    async fn test_metrics_recording() {
        let config = test_config();
        let collector = MetricsCollector::new(config).unwrap();
        
        // Record some metrics
        collector.record_event_processed(10);
        collector.record_event_failed(2);
        collector.record_file_created(1024);
        collector.record_processing_duration(Duration::from_millis(100));
        
        // Get stats
        let stats = collector.get_packer_stats();
        assert_eq!(stats.events_processed, 10);
        assert_eq!(stats.events_failed, 2);
        assert_eq!(stats.files_created, 1);
        assert_eq!(stats.bytes_processed, 1024);
        assert!(stats.success_rate > 0.0);
    }
    
    #[test]
    fn test_custom_metric() {
        let metric = RingBufferHealthMetric::new();
        assert_eq!(metric.name(), "ring_buffer_health");
        assert_eq!(metric.description(), "Ring buffer health score (0-100)");
        
        let value = metric.collect().unwrap();
        assert!(value >= 0.0 && value <= 100.0);
    }
    
    #[tokio::test]
    async fn test_metrics_export() {
        let config = test_config();
        let collector = MetricsCollector::new(config).unwrap();
        
        // Record some data
        collector.record_event_processed(5);
        collector.record_file_created(512);
        
        // Test Prometheus export
        let prometheus_output = collector.export_metrics("prometheus");
        assert!(prometheus_output.is_ok());
        let output = prometheus_output.unwrap();
        assert!(output.contains("events_processed_total"));
        
        // Test JSON export
        let json_output = collector.export_metrics("json");
        assert!(json_output.is_ok());
        let output = json_output.unwrap();
        assert!(output.contains("events_processed"));
    }
}