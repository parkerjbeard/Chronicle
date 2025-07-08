//! Configuration management for Chronicle benchmarks

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Main benchmark configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub benchmarks: BenchmarkSettings,
    pub monitoring: MonitoringSettings,
    pub dashboard: DashboardSettings,
    pub storage: StorageSettings,
    pub alerts: AlertSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSettings {
    pub enabled_suites: Vec<String>,
    pub default_duration_seconds: u64,
    pub default_iterations: u32,
    pub default_concurrency: u32,
    pub warmup_duration_seconds: u64,
    pub output_format: OutputFormat,
    pub output_directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringSettings {
    pub enabled: bool,
    pub sample_interval_ms: u64,
    pub retention_duration_hours: u64,
    pub metrics_export_enabled: bool,
    pub metrics_export_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSettings {
    pub enabled: bool,
    pub port: u16,
    pub host: String,
    pub auto_refresh_seconds: u64,
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSettings {
    pub database_path: String,
    pub backup_enabled: bool,
    pub backup_interval_hours: u64,
    pub compression_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSettings {
    pub enabled: bool,
    pub email_notifications: bool,
    pub webhook_url: Option<String>,
    pub thresholds: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Json,
    Csv,
    Html,
    Prometheus,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            benchmarks: BenchmarkSettings {
                enabled_suites: vec![
                    "ring_buffer".to_string(),
                    "collectors".to_string(),
                    "packer".to_string(),
                    "search".to_string(),
                    "storage".to_string(),
                ],
                default_duration_seconds: 10,
                default_iterations: 100,
                default_concurrency: 1,
                warmup_duration_seconds: 2,
                output_format: OutputFormat::Json,
                output_directory: "./benchmark_results".to_string(),
            },
            monitoring: MonitoringSettings {
                enabled: true,
                sample_interval_ms: 1000,
                retention_duration_hours: 24,
                metrics_export_enabled: true,
                metrics_export_port: 9090,
            },
            dashboard: DashboardSettings {
                enabled: true,
                port: 8080,
                host: "127.0.0.1".to_string(),
                auto_refresh_seconds: 5,
                theme: "dark".to_string(),
            },
            storage: StorageSettings {
                database_path: "./benchmark_data.db".to_string(),
                backup_enabled: true,
                backup_interval_hours: 24,
                compression_enabled: true,
            },
            alerts: AlertSettings {
                enabled: true,
                email_notifications: false,
                webhook_url: None,
                thresholds: [
                    ("cpu_usage_percent".to_string(), 80.0),
                    ("memory_usage_percent".to_string(), 85.0),
                    ("error_rate_percent".to_string(), 5.0),
                    ("response_time_ms".to_string(), 1000.0),
                ].iter().cloned().collect(),
            },
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Load configuration from environment variables
    pub fn load_from_env() -> Result<Self> {
        let mut config = Config::default();
        
        if let Ok(duration) = std::env::var("BENCHMARK_DURATION") {
            config.benchmarks.default_duration_seconds = duration.parse()?;
        }
        
        if let Ok(iterations) = std::env::var("BENCHMARK_ITERATIONS") {
            config.benchmarks.default_iterations = iterations.parse()?;
        }
        
        if let Ok(concurrency) = std::env::var("BENCHMARK_CONCURRENCY") {
            config.benchmarks.default_concurrency = concurrency.parse()?;
        }
        
        if let Ok(port) = std::env::var("DASHBOARD_PORT") {
            config.dashboard.port = port.parse()?;
        }
        
        if let Ok(host) = std::env::var("DASHBOARD_HOST") {
            config.dashboard.host = host;
        }
        
        Ok(config)
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.benchmarks.default_duration_seconds == 0 {
            return Err(anyhow::anyhow!("Benchmark duration must be greater than 0"));
        }
        
        if self.benchmarks.default_iterations == 0 {
            return Err(anyhow::anyhow!("Benchmark iterations must be greater than 0"));
        }
        
        if self.monitoring.sample_interval_ms == 0 {
            return Err(anyhow::anyhow!("Monitoring sample interval must be greater than 0"));
        }
        
        if self.dashboard.port == 0 {
            return Err(anyhow::anyhow!("Dashboard port must be valid"));
        }
        
        Ok(())
    }
}