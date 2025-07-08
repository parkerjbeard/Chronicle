use std::time::{Duration, Instant};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use anyhow::Result;

/// Performance measurement utilities for Chronicle tests
pub struct PerformanceMeasurer {
    measurements: HashMap<String, Vec<Duration>>,
    baselines: HashMap<String, Duration>,
    start_times: HashMap<String, Instant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub metric_name: String,
    pub total_measurements: usize,
    pub average_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub median_duration: Duration,
    pub percentile_95: Duration,
    pub percentile_99: Duration,
    pub standard_deviation: f64,
    pub baseline_comparison: Option<BaselineComparison>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineComparison {
    pub baseline_duration: Duration,
    pub current_average: Duration,
    pub change_percent: f64,
    pub is_regression: bool,
    pub is_improvement: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub disk_io_mb_per_sec: f64,
    pub network_io_mb_per_sec: f64,
    pub open_file_descriptors: u64,
    pub thread_count: u64,
}

impl PerformanceMeasurer {
    pub fn new() -> Self {
        Self {
            measurements: HashMap::new(),
            baselines: HashMap::new(),
            start_times: HashMap::new(),
        }
    }

    /// Start measuring a metric
    pub fn start_measurement(&mut self, metric_name: &str) {
        self.start_times.insert(metric_name.to_string(), Instant::now());
    }

    /// Stop measuring a metric and record the duration
    pub fn stop_measurement(&mut self, metric_name: &str) -> Option<Duration> {
        if let Some(start_time) = self.start_times.remove(metric_name) {
            let duration = start_time.elapsed();
            self.measurements
                .entry(metric_name.to_string())
                .or_insert_with(Vec::new)
                .push(duration);
            Some(duration)
        } else {
            None
        }
    }

    /// Measure a single operation
    pub async fn measure_async<F, T>(&mut self, metric_name: &str, operation: F) -> Result<(T, Duration)>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let start = Instant::now();
        let result = operation.await?;
        let duration = start.elapsed();
        
        self.measurements
            .entry(metric_name.to_string())
            .or_insert_with(Vec::new)
            .push(duration);
        
        Ok((result, duration))
    }

    /// Measure a synchronous operation
    pub fn measure_sync<F, T>(&mut self, metric_name: &str, operation: F) -> Result<(T, Duration)>
    where
        F: FnOnce() -> Result<T>,
    {
        let start = Instant::now();
        let result = operation()?;
        let duration = start.elapsed();
        
        self.measurements
            .entry(metric_name.to_string())
            .or_insert_with(Vec::new)
            .push(duration);
        
        Ok((result, duration))
    }

    /// Set a baseline for a metric
    pub fn set_baseline(&mut self, metric_name: &str, baseline: Duration) {
        self.baselines.insert(metric_name.to_string(), baseline);
    }

    /// Load baselines from a file
    pub fn load_baselines(&mut self, baselines: HashMap<String, Duration>) {
        self.baselines.extend(baselines);
    }

    /// Generate a performance report for a metric
    pub fn generate_report(&self, metric_name: &str) -> Option<PerformanceReport> {
        let measurements = self.measurements.get(metric_name)?;
        
        if measurements.is_empty() {
            return None;
        }

        let mut sorted_measurements = measurements.clone();
        sorted_measurements.sort();

        let total = measurements.len();
        let sum: Duration = measurements.iter().sum();
        let average = sum / total as u32;
        let min = *sorted_measurements.first().unwrap();
        let max = *sorted_measurements.last().unwrap();
        
        let median = if total % 2 == 0 {
            let mid1 = sorted_measurements[total / 2 - 1];
            let mid2 = sorted_measurements[total / 2];
            Duration::from_nanos((mid1.as_nanos() + mid2.as_nanos()) / 2)
        } else {
            sorted_measurements[total / 2]
        };

        let percentile_95_idx = (total as f64 * 0.95) as usize;
        let percentile_95 = sorted_measurements[percentile_95_idx.min(total - 1)];
        
        let percentile_99_idx = (total as f64 * 0.99) as usize;
        let percentile_99 = sorted_measurements[percentile_99_idx.min(total - 1)];

        // Calculate standard deviation
        let mean_nanos = average.as_nanos() as f64;
        let variance: f64 = measurements
            .iter()
            .map(|d| {
                let diff = d.as_nanos() as f64 - mean_nanos;
                diff * diff
            })
            .sum::<f64>() / total as f64;
        let standard_deviation = variance.sqrt();

        // Baseline comparison
        let baseline_comparison = self.baselines.get(metric_name).map(|baseline| {
            let change_percent = ((average.as_nanos() as f64 - baseline.as_nanos() as f64) 
                / baseline.as_nanos() as f64) * 100.0;
            
            BaselineComparison {
                baseline_duration: *baseline,
                current_average: average,
                change_percent,
                is_regression: change_percent > 5.0, // >5% slower is regression
                is_improvement: change_percent < -5.0, // >5% faster is improvement
            }
        });

        Some(PerformanceReport {
            metric_name: metric_name.to_string(),
            total_measurements: total,
            average_duration: average,
            min_duration: min,
            max_duration: max,
            median_duration: median,
            percentile_95,
            percentile_99,
            standard_deviation,
            baseline_comparison,
        })
    }

    /// Generate reports for all metrics
    pub fn generate_all_reports(&self) -> Vec<PerformanceReport> {
        self.measurements
            .keys()
            .filter_map(|metric_name| self.generate_report(metric_name))
            .collect()
    }

    /// Check if any metrics show performance regression
    pub fn has_regressions(&self) -> bool {
        self.generate_all_reports()
            .iter()
            .any(|report| {
                report.baseline_comparison
                    .as_ref()
                    .map_or(false, |comp| comp.is_regression)
            })
    }

    /// Clear all measurements
    pub fn clear(&mut self) {
        self.measurements.clear();
        self.start_times.clear();
    }

    /// Get raw measurements for a metric
    pub fn get_measurements(&self, metric_name: &str) -> Option<&Vec<Duration>> {
        self.measurements.get(metric_name)
    }
}

/// System metrics collector
pub struct SystemMetricsCollector {
    start_time: Instant,
    samples: Vec<SystemMetrics>,
}

impl SystemMetricsCollector {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            samples: Vec::new(),
        }
    }

    /// Collect current system metrics
    pub fn collect(&mut self) -> Result<SystemMetrics> {
        let metrics = SystemMetrics {
            timestamp: chrono::Utc::now(),
            memory_usage_mb: self.get_memory_usage()?,
            cpu_usage_percent: self.get_cpu_usage()?,
            disk_io_mb_per_sec: self.get_disk_io()?,
            network_io_mb_per_sec: self.get_network_io()?,
            open_file_descriptors: self.get_open_file_descriptors()?,
            thread_count: self.get_thread_count()?,
        };

        self.samples.push(metrics.clone());
        Ok(metrics)
    }

    /// Start continuous monitoring
    pub async fn start_monitoring(&mut self, interval: Duration) -> Result<()> {
        loop {
            self.collect()?;
            tokio::time::sleep(interval).await;
        }
    }

    /// Stop monitoring and return collected samples
    pub fn stop_monitoring(self) -> Vec<SystemMetrics> {
        self.samples
    }

    fn get_memory_usage(&self) -> Result<f64> {
        // Platform-specific memory usage collection
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let output = Command::new("vm_stat").output()?;
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            // Parse vm_stat output (simplified)
            // In a real implementation, this would be more robust
            Ok(100.0) // Placeholder
        }
        
        #[cfg(target_os = "linux")]
        {
            let meminfo = std::fs::read_to_string("/proc/meminfo")?;
            // Parse /proc/meminfo (simplified)
            Ok(100.0) // Placeholder
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Ok(100.0) // Placeholder for other platforms
        }
    }

    fn get_cpu_usage(&self) -> Result<f64> {
        // Platform-specific CPU usage collection
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let output = Command::new("top")
                .args(&["-l", "1", "-n", "0"])
                .output()?;
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            // Parse top output (simplified)
            Ok(25.0) // Placeholder
        }
        
        #[cfg(target_os = "linux")]
        {
            let stat = std::fs::read_to_string("/proc/stat")?;
            // Parse /proc/stat (simplified)
            Ok(25.0) // Placeholder
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Ok(25.0) // Placeholder for other platforms
        }
    }

    fn get_disk_io(&self) -> Result<f64> {
        // Platform-specific disk I/O monitoring
        Ok(10.0) // Placeholder
    }

    fn get_network_io(&self) -> Result<f64> {
        // Platform-specific network I/O monitoring
        Ok(5.0) // Placeholder
    }

    fn get_open_file_descriptors(&self) -> Result<u64> {
        // Count open file descriptors
        #[cfg(unix)]
        {
            use std::fs;
            let fd_dir = format!("/proc/{}/fd", std::process::id());
            if let Ok(entries) = fs::read_dir(&fd_dir) {
                Ok(entries.count() as u64)
            } else {
                Ok(0)
            }
        }
        
        #[cfg(not(unix))]
        {
            Ok(0)
        }
    }

    fn get_thread_count(&self) -> Result<u64> {
        // Get thread count for current process
        #[cfg(target_os = "linux")]
        {
            let status = std::fs::read_to_string("/proc/self/status")?;
            for line in status.lines() {
                if line.starts_with("Threads:") {
                    if let Some(count_str) = line.split_whitespace().nth(1) {
                        return Ok(count_str.parse().unwrap_or(1));
                    }
                }
            }
            Ok(1)
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            Ok(1) // Placeholder
        }
    }
}

/// Benchmark utilities
pub struct BenchmarkRunner {
    warmup_iterations: usize,
    measurement_iterations: usize,
    measurer: PerformanceMeasurer,
}

impl BenchmarkRunner {
    pub fn new(warmup_iterations: usize, measurement_iterations: usize) -> Self {
        Self {
            warmup_iterations,
            measurement_iterations,
            measurer: PerformanceMeasurer::new(),
        }
    }

    /// Run a benchmark with warmup
    pub async fn benchmark_async<F, T>(
        &mut self,
        name: &str,
        mut operation: F,
    ) -> Result<PerformanceReport>
    where
        F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
        T: Send,
    {
        // Warmup phase
        for _ in 0..self.warmup_iterations {
            operation().await?;
        }

        // Measurement phase
        for _ in 0..self.measurement_iterations {
            self.measurer.measure_async(name, operation()).await?;
        }

        self.measurer.generate_report(name)
            .ok_or_else(|| anyhow::anyhow!("Failed to generate benchmark report"))
    }

    /// Run a synchronous benchmark with warmup
    pub fn benchmark_sync<F, T>(
        &mut self,
        name: &str,
        mut operation: F,
    ) -> Result<PerformanceReport>
    where
        F: FnMut() -> Result<T>,
    {
        // Warmup phase
        for _ in 0..self.warmup_iterations {
            operation()?;
        }

        // Measurement phase
        for _ in 0..self.measurement_iterations {
            self.measurer.measure_sync(name, &mut operation)?;
        }

        self.measurer.generate_report(name)
            .ok_or_else(|| anyhow::anyhow!("Failed to generate benchmark report"))
    }

    /// Run throughput benchmark (operations per second)
    pub async fn benchmark_throughput<F, T>(
        &mut self,
        name: &str,
        mut operation: F,
        duration: Duration,
    ) -> Result<ThroughputReport>
    where
        F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
        T: Send,
    {
        let start = Instant::now();
        let mut operations = 0;
        let mut errors = 0;

        while start.elapsed() < duration {
            match operation().await {
                Ok(_) => operations += 1,
                Err(_) => errors += 1,
            }
        }

        let actual_duration = start.elapsed();
        let throughput = operations as f64 / actual_duration.as_secs_f64();
        let error_rate = errors as f64 / (operations + errors) as f64;

        Ok(ThroughputReport {
            name: name.to_string(),
            duration: actual_duration,
            total_operations: operations,
            total_errors: errors,
            throughput_ops_per_sec: throughput,
            error_rate,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputReport {
    pub name: String,
    pub duration: Duration,
    pub total_operations: u64,
    pub total_errors: u64,
    pub throughput_ops_per_sec: f64,
    pub error_rate: f64,
}

impl Default for PerformanceMeasurer {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SystemMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_performance_measurer() {
        let mut measurer = PerformanceMeasurer::new();
        
        // Test sync measurement
        let (result, duration) = measurer.measure_sync("test_op", || {
            std::thread::sleep(Duration::from_millis(10));
            Ok::<i32, anyhow::Error>(42)
        }).unwrap();
        
        assert_eq!(result, 42);
        assert!(duration >= Duration::from_millis(10));
        
        // Test report generation
        let report = measurer.generate_report("test_op").unwrap();
        assert_eq!(report.metric_name, "test_op");
        assert_eq!(report.total_measurements, 1);
    }

    #[tokio::test]
    async fn test_async_measurement() {
        let mut measurer = PerformanceMeasurer::new();
        
        let (result, duration) = measurer.measure_async("async_op", async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok::<i32, anyhow::Error>(100)
        }).await.unwrap();
        
        assert_eq!(result, 100);
        assert!(duration >= Duration::from_millis(10));
    }

    #[test]
    fn test_baseline_comparison() {
        let mut measurer = PerformanceMeasurer::new();
        
        // Set baseline
        measurer.set_baseline("baseline_test", Duration::from_millis(100));
        
        // Add measurement
        measurer.measure_sync("baseline_test", || {
            std::thread::sleep(Duration::from_millis(110)); // 10% slower
            Ok::<(), anyhow::Error>(())
        }).unwrap();
        
        let report = measurer.generate_report("baseline_test").unwrap();
        let comparison = report.baseline_comparison.unwrap();
        
        assert!(comparison.change_percent > 0.0);
        assert!(comparison.is_regression);
    }

    #[test]
    fn test_system_metrics_collector() {
        let mut collector = SystemMetricsCollector::new();
        let metrics = collector.collect().unwrap();
        
        assert!(metrics.memory_usage_mb >= 0.0);
        assert!(metrics.cpu_usage_percent >= 0.0);
        assert!(metrics.thread_count >= 1);
    }
}