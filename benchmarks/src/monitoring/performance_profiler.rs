//! Performance profiler for Chronicle benchmarks

use crate::monitoring::{MonitoringComponent, MonitoringConfig};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileData {
    pub function_name: String,
    pub call_count: u64,
    pub total_time_ms: f64,
    pub avg_time_ms: f64,
    pub min_time_ms: f64,
    pub max_time_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingSession {
    pub session_id: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub profiles: Vec<ProfileData>,
}

pub struct PerformanceProfiler {
    config: MonitoringConfig,
    is_running: AtomicBool,
    current_session: Arc<RwLock<Option<ProfilingSession>>>,
    profile_data: Arc<RwLock<HashMap<String, Vec<f64>>>>,
}

impl PerformanceProfiler {
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            is_running: AtomicBool::new(false),
            current_session: Arc::new(RwLock::new(None)),
            profile_data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_session(&self, session_id: String) -> Result<()> {
        let session = ProfilingSession {
            session_id,
            start_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            end_time: None,
            profiles: Vec::new(),
        };

        *self.current_session.write().await = Some(session);
        self.profile_data.write().await.clear();

        Ok(())
    }

    pub async fn end_session(&self) -> Result<Option<ProfilingSession>> {
        let mut current = self.current_session.write().await;
        
        if let Some(mut session) = current.take() {
            session.end_time = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs(),
            );

            // Generate profile data
            let profile_data = self.profile_data.read().await;
            session.profiles = profile_data
                .iter()
                .map(|(name, times)| {
                    let call_count = times.len() as u64;
                    let total_time = times.iter().sum::<f64>();
                    let avg_time = total_time / call_count as f64;
                    let min_time = times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                    let max_time = times.iter().fold(0.0, |a, &b| a.max(b));

                    ProfileData {
                        function_name: name.clone(),
                        call_count,
                        total_time_ms: total_time,
                        avg_time_ms: avg_time,
                        min_time_ms: min_time,
                        max_time_ms: max_time,
                    }
                })
                .collect();

            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    pub async fn record_function_call(&self, function_name: &str, duration_ms: f64) {
        let mut profile_data = self.profile_data.write().await;
        profile_data
            .entry(function_name.to_string())
            .or_insert_with(Vec::new)
            .push(duration_ms);
    }

    pub async fn get_current_session(&self) -> Option<ProfilingSession> {
        self.current_session.read().await.clone()
    }
}

impl MonitoringComponent for PerformanceProfiler {
    async fn start(&self) -> Result<()> {
        self.is_running.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.is_running.store(false, Ordering::Relaxed);
        let _ = self.end_session().await;
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    async fn get_metrics(&self) -> Result<serde_json::Value> {
        let session = self.get_current_session().await;
        Ok(serde_json::to_value(&session)?)
    }
}

/// Macro for easy function profiling
#[macro_export]
macro_rules! profile_function {
    ($profiler:expr, $func_name:expr, $code:block) => {{
        let start = std::time::Instant::now();
        let result = $code;
        let duration = start.elapsed().as_nanos() as f64 / 1_000_000.0;
        $profiler.record_function_call($func_name, duration).await;
        result
    }};
}