//! Memory usage analysis for Chronicle benchmarks

use crate::monitoring::{MonitoringComponent, MonitoringConfig};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub timestamp: u64,
    pub total_memory_mb: f64,
    pub used_memory_mb: f64,
    pub free_memory_mb: f64,
    pub available_memory_mb: f64,
    pub swap_total_mb: f64,
    pub swap_used_mb: f64,
    pub process_memory_mb: f64,
    pub heap_size_mb: f64,
    pub memory_fragmentation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAnalysis {
    pub peak_usage_mb: f64,
    pub average_usage_mb: f64,
    pub memory_growth_rate_mb_per_sec: f64,
    pub potential_leaks: Vec<MemoryLeak>,
    pub memory_efficiency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLeak {
    pub component: String,
    pub growth_rate_mb_per_sec: f64,
    pub confidence: f64,
}

pub struct MemoryAnalyzer {
    config: MonitoringConfig,
    is_running: AtomicBool,
    snapshots: Arc<RwLock<Vec<MemorySnapshot>>>,
    current_analysis: Arc<RwLock<Option<MemoryAnalysis>>>,
}

impl MemoryAnalyzer {
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            is_running: AtomicBool::new(false),
            snapshots: Arc::new(RwLock::new(Vec::new())),
            current_analysis: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn take_snapshot(&self) -> Result<MemorySnapshot> {
        let mut system = sysinfo::System::new_all();
        system.refresh_memory();

        // Get current process memory usage
        let current_pid = sysinfo::Pid::from(std::process::id() as usize);
        let process_memory = system
            .process(current_pid)
            .map(|p| p.memory() as f64 / 1024.0 / 1024.0)
            .unwrap_or(0.0);

        let snapshot = MemorySnapshot {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            total_memory_mb: system.total_memory() as f64 / 1024.0 / 1024.0,
            used_memory_mb: system.used_memory() as f64 / 1024.0 / 1024.0,
            free_memory_mb: system.free_memory() as f64 / 1024.0 / 1024.0,
            available_memory_mb: system.available_memory() as f64 / 1024.0 / 1024.0,
            swap_total_mb: system.total_swap() as f64 / 1024.0 / 1024.0,
            swap_used_mb: system.used_swap() as f64 / 1024.0 / 1024.0,
            process_memory_mb: process_memory,
            heap_size_mb: self.estimate_heap_size(),
            memory_fragmentation: self.calculate_fragmentation(&system),
        };

        let mut snapshots = self.snapshots.write().await;
        snapshots.push(snapshot.clone());

        // Cleanup old snapshots
        let retention_seconds = self.config.retention_duration_hours * 3600;
        let cutoff_time = snapshot.timestamp - retention_seconds;
        snapshots.retain(|s| s.timestamp > cutoff_time);

        Ok(snapshot)
    }

    pub async fn analyze_memory_usage(&self) -> Result<MemoryAnalysis> {
        let snapshots = self.snapshots.read().await;
        
        if snapshots.len() < 2 {
            return Ok(MemoryAnalysis {
                peak_usage_mb: 0.0,
                average_usage_mb: 0.0,
                memory_growth_rate_mb_per_sec: 0.0,
                potential_leaks: Vec::new(),
                memory_efficiency: 100.0,
            });
        }

        let memory_values: Vec<f64> = snapshots.iter()
            .map(|s| s.process_memory_mb)
            .collect();

        let peak_usage = memory_values.iter()
            .fold(0.0, |max, &val| max.max(val));
        
        let average_usage = memory_values.iter().sum::<f64>() / memory_values.len() as f64;

        // Calculate growth rate
        let time_span = snapshots.last().unwrap().timestamp - snapshots.first().unwrap().timestamp;
        let memory_growth = snapshots.last().unwrap().process_memory_mb - snapshots.first().unwrap().process_memory_mb;
        let growth_rate = if time_span > 0 {
            memory_growth / time_span as f64
        } else {
            0.0
        };

        // Detect potential memory leaks
        let potential_leaks = self.detect_memory_leaks(&snapshots).await;

        // Calculate memory efficiency
        let allocated_memory = snapshots.last().unwrap().process_memory_mb;
        let used_memory = allocated_memory * 0.8; // Estimate 80% actual usage
        let efficiency = if allocated_memory > 0.0 {
            (used_memory / allocated_memory) * 100.0
        } else {
            100.0
        };

        let analysis = MemoryAnalysis {
            peak_usage_mb: peak_usage,
            average_usage_mb: average_usage,
            memory_growth_rate_mb_per_sec: growth_rate,
            potential_leaks,
            memory_efficiency: efficiency,
        };

        *self.current_analysis.write().await = Some(analysis.clone());

        Ok(analysis)
    }

    async fn detect_memory_leaks(&self, snapshots: &[MemorySnapshot]) -> Vec<MemoryLeak> {
        let mut leaks = Vec::new();

        // Simple leak detection based on memory growth patterns
        if snapshots.len() >= 10 {
            let recent_snapshots = &snapshots[snapshots.len()-10..];
            
            let memory_values: Vec<f64> = recent_snapshots.iter()
                .map(|s| s.process_memory_mb)
                .collect();

            // Check for consistent upward trend
            let mut increasing_count = 0;
            for i in 1..memory_values.len() {
                if memory_values[i] > memory_values[i-1] {
                    increasing_count += 1;
                }
            }

            let growth_ratio = increasing_count as f64 / (memory_values.len() - 1) as f64;
            
            if growth_ratio > 0.7 { // 70% of samples show growth
                let time_span = recent_snapshots.last().unwrap().timestamp - recent_snapshots.first().unwrap().timestamp;
                let memory_growth = recent_snapshots.last().unwrap().process_memory_mb - recent_snapshots.first().unwrap().process_memory_mb;
                let leak_rate = if time_span > 0 {
                    memory_growth / time_span as f64
                } else {
                    0.0
                };

                if leak_rate > 0.01 { // Growing by more than 0.01 MB/sec
                    leaks.push(MemoryLeak {
                        component: "unknown".to_string(),
                        growth_rate_mb_per_sec: leak_rate,
                        confidence: growth_ratio,
                    });
                }
            }
        }

        leaks
    }

    fn estimate_heap_size(&self) -> f64 {
        // TODO: Implement platform-specific heap size estimation
        0.0
    }

    fn calculate_fragmentation(&self, system: &sysinfo::System) -> f64 {
        // Simple fragmentation estimation
        let used = system.used_memory() as f64;
        let available = system.available_memory() as f64;
        let total = system.total_memory() as f64;
        
        let expected_free = total - used;
        let actual_free = available;
        
        if expected_free > 0.0 {
            ((expected_free - actual_free) / expected_free * 100.0).max(0.0)
        } else {
            0.0
        }
    }

    pub async fn get_current_analysis(&self) -> Option<MemoryAnalysis> {
        self.current_analysis.read().await.clone()
    }

    pub async fn get_memory_snapshots(&self) -> Vec<MemorySnapshot> {
        self.snapshots.read().await.clone()
    }
}

impl MonitoringComponent for MemoryAnalyzer {
    async fn start(&self) -> Result<()> {
        self.is_running.store(true, Ordering::Relaxed);

        let analyzer = Arc::new(self);
        let analyzer_clone = analyzer.clone();

        tokio::spawn(async move {
            while analyzer_clone.is_running() {
                if let Err(e) = analyzer_clone.take_snapshot().await {
                    tracing::error!("Failed to take memory snapshot: {}", e);
                }

                // Run analysis every 10 snapshots
                let snapshot_count = analyzer_clone.snapshots.read().await.len();
                if snapshot_count % 10 == 0 && snapshot_count > 0 {
                    if let Err(e) = analyzer_clone.analyze_memory_usage().await {
                        tracing::error!("Failed to analyze memory usage: {}", e);
                    }
                }

                time::sleep(Duration::from_millis(analyzer_clone.config.sample_interval_ms)).await;
            }
        });

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.is_running.store(false, Ordering::Relaxed);
        
        // Perform final analysis
        let _ = self.analyze_memory_usage().await;
        
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    async fn get_metrics(&self) -> Result<serde_json::Value> {
        let analysis = self.get_current_analysis().await;
        Ok(serde_json::to_value(&analysis)?)
    }
}