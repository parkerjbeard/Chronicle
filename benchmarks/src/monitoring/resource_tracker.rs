//! Resource usage tracking for Chronicle benchmarks

use crate::monitoring::{MonitoringComponent, MonitoringConfig};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub timestamp: u64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub network_usage: f64,
    pub process_count: u64,
    pub thread_count: u64,
    pub file_handle_count: u64,
}

pub struct ResourceTracker {
    config: MonitoringConfig,
    is_running: AtomicBool,
    current_usage: Arc<RwLock<Option<ResourceUsage>>>,
    usage_history: Arc<RwLock<Vec<ResourceUsage>>>,
}

impl ResourceTracker {
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            is_running: AtomicBool::new(false),
            current_usage: Arc::new(RwLock::new(None)),
            usage_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn collect_usage(&self) -> Result<ResourceUsage> {
        let mut system = sysinfo::System::new_all();
        system.refresh_all();

        let usage = ResourceUsage {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            cpu_usage: system.global_cpu_info().cpu_usage() as f64,
            memory_usage: system.used_memory() as f64 / 1024.0 / 1024.0,
            disk_usage: 0.0, // TODO: Implement disk usage tracking
            network_usage: 0.0, // TODO: Implement network usage tracking
            process_count: system.processes().len() as u64,
            thread_count: system.processes().values()
                .map(|p| 1) // TODO: Get actual thread count per process
                .sum(),
            file_handle_count: 0, // TODO: Implement file handle tracking
        };

        *self.current_usage.write().await = Some(usage.clone());

        let mut history = self.usage_history.write().await;
        history.push(usage.clone());

        // Cleanup old entries
        let retention_seconds = self.config.retention_duration_hours * 3600;
        let cutoff_time = usage.timestamp - retention_seconds;
        history.retain(|u| u.timestamp > cutoff_time);

        Ok(usage)
    }

    pub async fn get_current_usage(&self) -> Option<ResourceUsage> {
        self.current_usage.read().await.clone()
    }

    pub async fn get_usage_history(&self) -> Vec<ResourceUsage> {
        self.usage_history.read().await.clone()
    }
}

impl MonitoringComponent for ResourceTracker {
    async fn start(&self) -> Result<()> {
        self.is_running.store(true, Ordering::Relaxed);

        let tracker = Arc::new(self);
        let tracker_clone = tracker.clone();

        tokio::spawn(async move {
            while tracker_clone.is_running() {
                if let Err(e) = tracker_clone.collect_usage().await {
                    tracing::error!("Failed to collect resource usage: {}", e);
                }

                time::sleep(Duration::from_millis(tracker_clone.config.sample_interval_ms)).await;
            }
        });

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.is_running.store(false, Ordering::Relaxed);
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    async fn get_metrics(&self) -> Result<serde_json::Value> {
        let current = self.get_current_usage().await;
        Ok(serde_json::to_value(&current)?)
    }
}