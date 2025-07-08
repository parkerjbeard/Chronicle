//! System monitoring components for Chronicle

pub mod memory_analyzer;
pub mod performance_profiler;
pub mod resource_tracker;
pub mod system_monitor;

use crate::{BenchmarkConfig, BenchmarkResult};
use anyhow::Result;

/// Trait for monitoring components
pub trait MonitoringComponent {
    /// Start monitoring
    async fn start(&self) -> Result<()>;
    
    /// Stop monitoring
    async fn stop(&self) -> Result<()>;
    
    /// Get current status
    fn is_running(&self) -> bool;
    
    /// Get collected metrics
    async fn get_metrics(&self) -> Result<serde_json::Value>;
}

/// System monitoring configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonitoringConfig {
    pub sample_interval_ms: u64,
    pub retention_duration_hours: u64,
    pub alert_thresholds: AlertThresholds,
    pub enabled_monitors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AlertThresholds {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub error_rate_percent: f64,
    pub response_time_ms: f64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            sample_interval_ms: 1000,
            retention_duration_hours: 24,
            alert_thresholds: AlertThresholds {
                cpu_usage_percent: 80.0,
                memory_usage_percent: 85.0,
                disk_usage_percent: 90.0,
                error_rate_percent: 5.0,
                response_time_ms: 1000.0,
            },
            enabled_monitors: vec![
                "system".to_string(),
                "resource".to_string(),
                "performance".to_string(),
                "memory".to_string(),
            ],
        }
    }
}