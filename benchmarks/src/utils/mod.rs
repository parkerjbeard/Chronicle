//! Utility modules for Chronicle benchmarks

pub mod benchmark_harness;
pub mod data_generator;
pub mod metrics_collector;
pub mod report_generator;

use anyhow::Result;
use std::time::{Duration, Instant};

/// Timer utility for measuring execution time
pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }
    
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
    
    pub fn elapsed_ms(&self) -> f64 {
        self.elapsed().as_nanos() as f64 / 1_000_000.0
    }
    
    pub fn restart(&mut self) {
        self.start = Instant::now();
    }
}

/// Statistical utilities
pub mod stats {
    pub fn mean(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        values.iter().sum::<f64>() / values.len() as f64
    }
    
    pub fn median(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let len = sorted.len();
        if len % 2 == 0 {
            (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
        } else {
            sorted[len / 2]
        }
    }
    
    pub fn percentile(values: &[f64], p: f64) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let len = sorted.len();
        let index = ((len as f64 - 1.0) * p / 100.0).round() as usize;
        sorted[index.min(len - 1)]
    }
    
    pub fn standard_deviation(values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }
        let mean_val = mean(values);
        let variance = values.iter()
            .map(|x| (x - mean_val).powi(2))
            .sum::<f64>() / (values.len() - 1) as f64;
        variance.sqrt()
    }
}

/// Format utilities
pub mod format {
    use std::time::Duration;
    
    pub fn duration_human(duration: Duration) -> String {
        let total_secs = duration.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        let millis = duration.subsec_millis();
        
        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else if seconds > 0 {
            format!("{}.{:03}s", seconds, millis)
        } else {
            format!("{}ms", duration.as_millis())
        }
    }
    
    pub fn bytes_human(bytes: f64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        
        if bytes == 0.0 {
            return "0 B".to_string();
        }
        
        let i = (bytes.log10() / 3.0).floor() as usize;
        let size = bytes / 1000_f64.powi(i as i32);
        
        if i < UNITS.len() {
            format!("{:.2} {}", size, UNITS[i])
        } else {
            format!("{:.2} PB", bytes / 1000_f64.powi(5))
        }
    }
    
    pub fn rate_human(rate: f64, unit: &str) -> String {
        if rate >= 1_000_000.0 {
            format!("{:.2}M {}/s", rate / 1_000_000.0, unit)
        } else if rate >= 1_000.0 {
            format!("{:.2}K {}/s", rate / 1_000.0, unit)
        } else {
            format!("{:.2} {}/s", rate, unit)
        }
    }
}

/// System utilities
pub mod system {
    use std::process::Command;
    use anyhow::Result;
    
    pub fn get_system_info() -> Result<SystemInfo> {
        Ok(SystemInfo {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            cpu_count: num_cpus::get(),
            hostname: hostname::get()?.to_string_lossy().to_string(),
        })
    }
    
    pub fn get_process_id() -> u32 {
        std::process::id()
    }
    
    pub fn get_memory_info() -> Result<MemoryInfo> {
        let mut system = sysinfo::System::new_all();
        system.refresh_memory();
        
        Ok(MemoryInfo {
            total_mb: system.total_memory() as f64 / 1024.0 / 1024.0,
            used_mb: system.used_memory() as f64 / 1024.0 / 1024.0,
            free_mb: system.free_memory() as f64 / 1024.0 / 1024.0,
        })
    }
    
    #[derive(Debug, Clone)]
    pub struct SystemInfo {
        pub os: String,
        pub arch: String,
        pub cpu_count: usize,
        pub hostname: String,
    }
    
    #[derive(Debug, Clone)]
    pub struct MemoryInfo {
        pub total_mb: f64,
        pub used_mb: f64,
        pub free_mb: f64,
    }
}

/// Test data generation utilities
pub mod test_data {
    use rand::{Rng, SeedableRng};
    use rand::rngs::StdRng;
    
    pub struct DataGenerator {
        rng: StdRng,
    }
    
    impl DataGenerator {
        pub fn new(seed: u64) -> Self {
            Self {
                rng: StdRng::seed_from_u64(seed),
            }
        }
        
        pub fn random_bytes(&mut self, size: usize) -> Vec<u8> {
            (0..size).map(|_| self.rng.gen()).collect()
        }
        
        pub fn random_string(&mut self, length: usize) -> String {
            const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            (0..length)
                .map(|_| {
                    let idx = self.rng.gen_range(0..CHARSET.len());
                    CHARSET[idx] as char
                })
                .collect()
        }
        
        pub fn random_json(&mut self, complexity: usize) -> serde_json::Value {
            use serde_json::{Map, Value};
            
            let mut map = Map::new();
            for i in 0..complexity {
                let key = format!("field_{}", i);
                let value = match self.rng.gen_range(0..4) {
                    0 => Value::String(self.random_string(10)),
                    1 => Value::Number(serde_json::Number::from(self.rng.gen::<i32>())),
                    2 => Value::Bool(self.rng.gen()),
                    _ => Value::Array(vec![
                        Value::String(self.random_string(5)),
                        Value::Number(serde_json::Number::from(self.rng.gen::<i32>())),
                    ]),
                };
                map.insert(key, value);
            }
            Value::Object(map)
        }
    }
    
    impl Default for DataGenerator {
        fn default() -> Self {
            Self::new(42) // Deterministic seed for reproducible tests
        }
    }
}

/// Async utilities
pub mod async_utils {
    use std::future::Future;
    use std::time::Duration;
    use tokio::time;
    
    /// Run a future with a timeout
    pub async fn with_timeout<F, T>(
        future: F,
        timeout: Duration,
    ) -> Result<T, tokio::time::error::Elapsed>
    where
        F: Future<Output = T>,
    {
        time::timeout(timeout, future).await
    }
    
    /// Run multiple futures concurrently with a limit
    pub async fn run_concurrent<F, T>(
        futures: Vec<F>,
        concurrency_limit: usize,
    ) -> Vec<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        use futures::stream::{FuturesUnordered, StreamExt};
        
        let mut futures_unordered = FuturesUnordered::new();
        let mut results = Vec::new();
        let mut futures_iter = futures.into_iter();
        
        // Start initial batch
        for _ in 0..concurrency_limit.min(futures_iter.len()) {
            if let Some(future) = futures_iter.next() {
                futures_unordered.push(tokio::spawn(future));
            }
        }
        
        // Process results and start new futures
        while let Some(result) = futures_unordered.next().await {
            if let Ok(value) = result {
                results.push(value);
            }
            
            // Start next future if available
            if let Some(future) = futures_iter.next() {
                futures_unordered.push(tokio::spawn(future));
            }
        }
        
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_timer() {
        let timer = Timer::start();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(timer.elapsed_ms() >= 10.0);
    }
    
    #[test]
    fn test_stats() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(stats::mean(&values), 3.0);
        assert_eq!(stats::median(&values), 3.0);
        assert_eq!(stats::percentile(&values, 50.0), 3.0);
        assert!(stats::standard_deviation(&values) > 0.0);
    }
    
    #[test]
    fn test_format() {
        assert_eq!(format::bytes_human(1024.0), "1.02 KB");
        assert_eq!(format::rate_human(1500.0, "ops"), "1.50K ops/s");
    }
    
    #[test]
    fn test_data_generator() {
        let mut gen = test_data::DataGenerator::new(42);
        let bytes = gen.random_bytes(10);
        assert_eq!(bytes.len(), 10);
        
        let string = gen.random_string(5);
        assert_eq!(string.len(), 5);
        
        let json = gen.random_json(3);
        assert!(json.is_object());
    }
}