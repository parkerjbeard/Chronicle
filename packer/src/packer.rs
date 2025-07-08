//! Core packer service for Chronicle
//!
//! This module implements the main packer service that drains the ring buffer
//! nightly and converts Arrow data to Parquet files with HEIF frame organization.

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;

use tokio::signal;
use tokio::sync::{mpsc, RwLock};
use tokio_cron_scheduler::{JobScheduler, Job};
use chrono::{DateTime, Utc, TimeZone};
use arrow::ipc::{reader::StreamReader, writer::StreamWriter};
use arrow::record_batch::RecordBatch;

use crate::config::PackerConfig;
use crate::storage::{StorageManager, ChronicleEvent, HeifFrame};
use crate::encryption::EncryptionService;
use crate::integrity::IntegrityService;
use crate::metrics::MetricsCollector;
use crate::error::{PackerError, Result};

/// Ring buffer interface for FFI operations
#[repr(C)]
pub struct RingBufferHandle {
    ptr: *mut std::ffi::c_void,
}

unsafe impl Send for RingBufferHandle {}
unsafe impl Sync for RingBufferHandle {}

/// Chronicle packer service
pub struct PackerService {
    /// Configuration
    config: PackerConfig,
    
    /// Storage manager
    storage: Arc<RwLock<StorageManager>>,
    
    /// Encryption service
    encryption: Option<Arc<RwLock<EncryptionService>>>,
    
    /// Integrity service
    integrity: Arc<IntegrityService>,
    
    /// Metrics collector
    metrics: Arc<MetricsCollector>,
    
    /// Job scheduler
    scheduler: JobScheduler,
    
    /// Ring buffer handle (now safe)
    ring_buffer: Option<SafeRingBufferHandle>,
    
    /// Service state
    state: Arc<RwLock<ServiceState>>,
    
    /// Shutdown channel
    shutdown_tx: Option<mpsc::Sender<()>>,
}

/// Service state
#[derive(Debug, Clone)]
pub struct ServiceState {
    /// Service status
    pub status: ServiceStatus,
    
    /// Last processing time
    pub last_processing: Option<DateTime<Utc>>,
    
    /// Last error
    pub last_error: Option<String>,
    
    /// Processing statistics
    pub stats: ProcessingStats,
}

/// Service status
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Starting,
    Running,
    Processing,
    Stopping,
    Stopped,
    Error(String),
}

/// Processing statistics
#[derive(Debug, Clone, Default)]
pub struct ProcessingStats {
    /// Total runs
    pub total_runs: u64,
    
    /// Successful runs
    pub successful_runs: u64,
    
    /// Failed runs
    pub failed_runs: u64,
    
    /// Total events processed
    pub total_events: u64,
    
    /// Total files created
    pub total_files: u64,
    
    /// Total bytes processed
    pub total_bytes: u64,
    
    /// Average processing time
    pub avg_processing_time: Duration,
}

/// Processing result
#[derive(Debug)]
pub struct ProcessingResult {
    /// Number of events processed
    pub events_processed: usize,
    
    /// Number of files created
    pub files_created: usize,
    
    /// Total bytes processed
    pub bytes_processed: u64,
    
    /// Processing duration
    pub duration: Duration,
    
    /// Errors encountered
    pub errors: Vec<String>,
}

impl PackerService {
    /// Create a new packer service
    pub async fn new(config: PackerConfig) -> Result<Self> {
        tracing::info!("Initializing Chronicle packer service");
        
        // Initialize integrity service
        let integrity = Arc::new(IntegrityService::new());
        
        // Initialize encryption service if enabled
        let encryption = if config.encryption.enabled {
            let encryption_service = EncryptionService::new(config.encryption.clone())?;
            Some(Arc::new(RwLock::new(encryption_service)))
        } else {
            None
        };
        
        // Initialize storage manager
        let storage = StorageManager::new(
            config.storage.clone(),
            encryption.clone().map(|e| e.clone() as Arc<_>),
            integrity.clone(),
        )?;
        let storage = Arc::new(RwLock::new(storage));
        
        // Initialize metrics collector
        let metrics = Arc::new(MetricsCollector::new(config.metrics.clone())?);
        
        // Initialize job scheduler
        let scheduler = JobScheduler::new().await?;
        
        // Initialize service state
        let state = Arc::new(RwLock::new(ServiceState {
            status: ServiceStatus::Starting,
            last_processing: None,
            last_error: None,
            stats: ProcessingStats::default(),
        }));
        
        let service = Self {
            config,
            storage,
            encryption,
            integrity,
            metrics,
            scheduler,
            ring_buffer: None,
            state,
            shutdown_tx: None,
        };
        
        tracing::info!("Chronicle packer service initialized");
        Ok(service)
    }
    
    /// Start the packer service
    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting Chronicle packer service");
        
        // Update status
        {
            let mut state = self.state.write().await;
            state.status = ServiceStatus::Running;
        }
        
        // Initialize ring buffer connection
        self.initialize_ring_buffer().await?;
        
        // Start metrics collection
        self.metrics.start().await?;
        
        // Schedule daily processing job
        self.schedule_daily_processing().await?;
        
        // Schedule backup trigger monitoring
        self.schedule_backup_monitoring().await?;
        
        // Start the scheduler
        self.scheduler.start().await?;
        
        // Set up shutdown channel
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);
        
        // Handle graceful shutdown
        let state = self.state.clone();
        tokio::spawn(async move {
            shutdown_rx.recv().await;
            let mut state = state.write().await;
            state.status = ServiceStatus::Stopping;
            tracing::info!("Received shutdown signal");
        });
        
        tracing::info!("Chronicle packer service started successfully");
        Ok(())
    }
    
    /// Initialize ring buffer connection
    async fn initialize_ring_buffer(&mut self) -> Result<()> {
        tracing::info!("Initializing ring buffer connection");
        
        // For now, we'll use a placeholder for the ring buffer initialization
        // In a real implementation, this would call the C FFI functions
        // to connect to the shared memory ring buffer
        
        // Placeholder implementation
        self.ring_buffer = Some(RingBufferHandle {
            ptr: std::ptr::null_mut(),
        });
        
        tracing::info!("Ring buffer connection established");
        Ok(())
    }
    
    /// Schedule daily processing job
    async fn schedule_daily_processing(&mut self) -> Result<()> {
        let daily_time = &self.config.scheduling.daily_time;
        let cron_expr = format!("0 {} * * *", daily_time.replace(':', " "));
        
        let storage = self.storage.clone();
        let encryption = self.encryption.clone();
        let integrity = self.integrity.clone();
        let metrics = self.metrics.clone();
        let state = self.state.clone();
        let config = self.config.clone();
        
        let job = Job::new_async(cron_expr.as_str(), move |_uuid, _l| {
            let storage = storage.clone();
            let encryption = encryption.clone();
            let integrity = integrity.clone();
            let metrics = metrics.clone();
            let state = state.clone();
            let config = config.clone();
            
            Box::pin(async move {
                tracing::info!("Starting scheduled daily processing");
                
                // Update status
                {
                    let mut state = state.write().await;
                    state.status = ServiceStatus::Processing;
                }
                
                let result = Self::process_daily_data_static(
                    storage,
                    encryption,
                    integrity,
                    metrics.clone(),
                    &config,
                ).await;
                
                // Update state based on result
                {
                    let mut state = state.write().await;
                    match result {
                        Ok(processing_result) => {
                            state.status = ServiceStatus::Running;
                            state.last_processing = Some(Utc::now());
                            state.last_error = None;
                            state.stats.total_runs += 1;
                            state.stats.successful_runs += 1;
                            state.stats.total_events += processing_result.events_processed as u64;
                            state.stats.total_files += processing_result.files_created as u64;
                            state.stats.total_bytes += processing_result.bytes_processed;
                            
                            // Update average processing time
                            let total_time = state.stats.avg_processing_time.as_secs_f64() * 
                                            (state.stats.successful_runs - 1) as f64 + 
                                            processing_result.duration.as_secs_f64();
                            state.stats.avg_processing_time = Duration::from_secs_f64(
                                total_time / state.stats.successful_runs as f64
                            );
                            
                            metrics.record_daily_processing_time(processing_result.duration);
                            
                            tracing::info!("Daily processing completed successfully: {} events, {} files, {} bytes",
                                processing_result.events_processed,
                                processing_result.files_created,
                                processing_result.bytes_processed);
                        }
                        Err(e) => {
                            state.status = ServiceStatus::Running;
                            state.last_error = Some(e.to_string());
                            state.stats.total_runs += 1;
                            state.stats.failed_runs += 1;
                            
                            metrics.record_error("processing");
                            
                            tracing::error!("Daily processing failed: {}", e);
                        }
                    }
                }
            })
        })?;
        
        self.scheduler.add(job).await?;
        tracing::info!("Scheduled daily processing at {}", daily_time);
        Ok(())
    }
    
    /// Schedule backup monitoring
    async fn schedule_backup_monitoring(&mut self) -> Result<()> {
        // Check backup threshold every 5 minutes
        let cron_expr = "0 */5 * * * *";
        
        let config = self.config.clone();
        let state = self.state.clone();
        let metrics = self.metrics.clone();
        
        let job = Job::new_async(cron_expr, move |_uuid, _l| {
            let config = config.clone();
            let state = state.clone();
            let metrics = metrics.clone();
            
            Box::pin(async move {
                if let Err(e) = Self::check_backup_threshold(&config, &metrics).await {
                    tracing::error!("Backup threshold check failed: {}", e);
                    
                    let mut state = state.write().await;
                    state.last_error = Some(format!("Backup check failed: {}", e));
                }
            })
        })?;
        
        self.scheduler.add(job).await?;
        tracing::info!("Scheduled backup monitoring every 5 minutes");
        Ok(())
    }
    
    /// Process daily data (static version for async closure)
    async fn process_daily_data_static(
        storage: Arc<RwLock<StorageManager>>,
        encryption: Option<Arc<RwLock<EncryptionService>>>,
        integrity: Arc<IntegrityService>,
        metrics: Arc<MetricsCollector>,
        config: &PackerConfig,
    ) -> Result<ProcessingResult> {
        let start_time = Instant::now();
        let mut events_processed = 0;
        let mut files_created = 0;
        let mut bytes_processed = 0;
        let mut errors = Vec::new();
        
        // Drain ring buffer
        let events = Self::drain_ring_buffer_static(config, &metrics).await?;
        events_processed = events.len();
        
        if events.is_empty() {
            tracing::info!("No events to process");
            return Ok(ProcessingResult {
                events_processed: 0,
                files_created: 0,
                bytes_processed: 0,
                duration: start_time.elapsed(),
                errors: Vec::new(),
            });
        }
        
        // Group events by date
        let mut events_by_date = HashMap::new();
        for event in events {
            let date = Utc.timestamp_nanos(event.timestamp_ns as i64).date_naive();
            events_by_date.entry(date).or_insert_with(Vec::new).push(event);
        }
        
        // Process each date group
        for (date, date_events) in events_by_date {
            let date_time = Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap());
            
            match Self::process_date_events(
                &storage,
                &encryption,
                &integrity,
                &metrics,
                &date_events,
                &date_time,
            ).await {
                Ok((file_count, byte_count)) => {
                    files_created += file_count;
                    bytes_processed += byte_count;
                }
                Err(e) => {
                    errors.push(format!("Failed to process events for {}: {}", date, e));
                    metrics.record_error("storage");
                }
            }
        }
        
        // Perform maintenance tasks
        if let Err(e) = Self::perform_maintenance(&storage, &encryption, &metrics, config).await {
            errors.push(format!("Maintenance failed: {}", e));
        }
        
        // Record metrics
        metrics.record_event_processed(events_processed as u64);
        if !errors.is_empty() {
            metrics.record_event_failed(errors.len() as u64);
        }
        
        Ok(ProcessingResult {
            events_processed,
            files_created,
            bytes_processed,
            duration: start_time.elapsed(),
            errors,
        })
    }
    
    /// Drain ring buffer
    async fn drain_ring_buffer_static(
        config: &PackerConfig,
        metrics: &Arc<MetricsCollector>,
    ) -> Result<Vec<ChronicleEvent>> {
        tracing::info!("Draining ring buffer");
        
        // Placeholder implementation
        // In a real implementation, this would:
        // 1. Connect to the shared memory ring buffer
        // 2. Read all available Arrow IPC messages
        // 3. Parse them into ChronicleEvent structures
        // 4. Clear the ring buffer
        
        // For now, return empty vector
        let events = Vec::new();
        
        metrics.record_ring_buffer_utilization(0, 0.0);
        
        tracing::info!("Drained {} events from ring buffer", events.len());
        Ok(events)
    }
    
    /// Process events for a specific date
    async fn process_date_events(
        storage: &Arc<RwLock<StorageManager>>,
        encryption: &Option<Arc<RwLock<EncryptionService>>>,
        integrity: &Arc<IntegrityService>,
        metrics: &Arc<MetricsCollector>,
        events: &[ChronicleEvent],
        date: &DateTime<Utc>,
    ) -> Result<(usize, u64)> {
        let mut file_count = 0;
        let mut byte_count = 0;
        
        // Validate events
        let validation_start = Instant::now();
        let validation_results = integrity.validate_chronicle_events(events)?;
        let invalid_events: Vec<_> = validation_results.iter()
            .enumerate()
            .filter(|(_, result)| !result.passed)
            .collect();
        
        if !invalid_events.is_empty() {
            tracing::warn!("Found {} invalid events for {}", invalid_events.len(), date);
            for (i, result) in invalid_events {
                tracing::warn!("Invalid event {}: {}", i, result.error.as_ref().unwrap_or(&"Unknown error".to_string()));
            }
        }
        
        metrics.record_integrity_check_duration(validation_start.elapsed());
        
        // Check temporal consistency
        let consistency_check = integrity.check_temporal_consistency(events)?;
        if !consistency_check.passed {
            tracing::warn!("Temporal consistency issues for {}: {}", date, 
                consistency_check.error.as_ref().unwrap_or(&"Unknown error".to_string()));
        }
        
        // Separate events and frames
        let mut regular_events = Vec::new();
        let mut heif_frames = Vec::new();
        
        for event in events {
            match event.event_type.as_str() {
                "frame" => {
                    // Parse frame data
                    if let Ok(frame_data) = serde_json::from_str::<serde_json::Value>(&event.data) {
                        if let Some(data_str) = frame_data.get("data").and_then(|v| v.as_str()) {
                            if let Ok(data) = base64::decode(data_str) {
                                let frame = HeifFrame {
                                    timestamp: event.timestamp_ns,
                                    data,
                                    metadata: HashMap::new(),
                                };
                                heif_frames.push(frame);
                            }
                        }
                    }
                }
                _ => {
                    regular_events.push(event.clone());
                }
            }
        }
        
        // Write Parquet file if we have regular events
        if !regular_events.is_empty() {
            let storage_start = Instant::now();
            let mut storage_manager = storage.write().await;
            
            let parquet_path = storage_manager.write_events_to_parquet(&regular_events, date).await?;
            let file_size = std::fs::metadata(&parquet_path)?.len();
            
            file_count += 1;
            byte_count += file_size;
            
            metrics.record_storage_duration(storage_start.elapsed());
            metrics.record_file_created(file_size);
            
            tracing::info!("Created Parquet file: {} ({} bytes)", parquet_path.display(), file_size);
        }
        
        // Process HEIF frames if we have any
        if !heif_frames.is_empty() {
            let storage_start = Instant::now();
            let mut storage_manager = storage.write().await;
            
            let frame_paths = storage_manager.process_heif_frames(&heif_frames, date).await?;
            
            for path in &frame_paths {
                if let Ok(metadata) = std::fs::metadata(path) {
                    file_count += 1;
                    byte_count += metadata.len();
                }
            }
            
            metrics.record_storage_duration(storage_start.elapsed());
            
            tracing::info!("Processed {} HEIF frames for {}", heif_frames.len(), date);
        }
        
        Ok((file_count, byte_count))
    }
    
    /// Perform maintenance tasks
    async fn perform_maintenance(
        storage: &Arc<RwLock<StorageManager>>,
        encryption: &Option<Arc<RwLock<EncryptionService>>>,
        metrics: &Arc<MetricsCollector>,
        config: &PackerConfig,
    ) -> Result<()> {
        tracing::info!("Performing maintenance tasks");
        
        // Clean up old files
        let mut storage_manager = storage.write().await;
        let deleted_count = storage_manager.cleanup_old_files().await?;
        
        if deleted_count > 0 {
            tracing::info!("Cleaned up {} old files", deleted_count);
        }
        
        // Rotate encryption keys if needed
        if let Some(encryption_service) = encryption {
            let mut encryption = encryption_service.write().await;
            if encryption.needs_key_rotation() {
                encryption.rotate_keys()?;
                metrics.record_key_rotation();
                tracing::info!("Rotated encryption keys");
            }
        }
        
        tracing::info!("Maintenance tasks completed");
        Ok(())
    }
    
    /// Check if backup threshold is exceeded
    async fn check_backup_threshold(
        config: &PackerConfig,
        metrics: &Arc<MetricsCollector>,
    ) -> Result<()> {
        // Placeholder implementation
        // In a real implementation, this would check the ring buffer size
        // and trigger backup if threshold is exceeded
        
        let ring_buffer_size = 0u64; // Placeholder
        
        if ring_buffer_size > config.scheduling.backup_threshold {
            tracing::warn!("Ring buffer size ({} bytes) exceeds backup threshold ({} bytes)",
                ring_buffer_size, config.scheduling.backup_threshold);
            
            // Trigger immediate processing
            // This would typically send a message to trigger processing
        }
        
        metrics.record_ring_buffer_utilization(
            ring_buffer_size,
            ring_buffer_size as f64 / config.ring_buffer.size as f64 * 100.0,
        );
        
        Ok(())
    }
    
    /// Stop the service
    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping Chronicle packer service");
        
        // Update status
        {
            let mut state = self.state.write().await;
            state.status = ServiceStatus::Stopping;
        }
        
        // Send shutdown signal
        if let Some(shutdown_tx) = &self.shutdown_tx {
            let _ = shutdown_tx.send(()).await;
        }
        
        // Stop scheduler
        self.scheduler.shutdown().await?;
        
        // Update final status
        {
            let mut state = self.state.write().await;
            state.status = ServiceStatus::Stopped;
        }
        
        tracing::info!("Chronicle packer service stopped");
        Ok(())
    }
    
    /// Get service status
    pub async fn get_status(&self) -> ServiceState {
        self.state.read().await.clone()
    }
    
    /// Trigger manual processing
    pub async fn trigger_processing(&self) -> Result<ProcessingResult> {
        tracing::info!("Triggering manual processing");
        
        {
            let mut state = self.state.write().await;
            state.status = ServiceStatus::Processing;
        }
        
        let result = Self::process_daily_data_static(
            self.storage.clone(),
            self.encryption.clone(),
            self.integrity.clone(),
            self.metrics.clone(),
            &self.config,
        ).await;
        
        // Update state
        {
            let mut state = self.state.write().await;
            match &result {
                Ok(_) => {
                    state.status = ServiceStatus::Running;
                    state.last_processing = Some(Utc::now());
                    state.last_error = None;
                }
                Err(e) => {
                    state.status = ServiceStatus::Running;
                    state.last_error = Some(e.to_string());
                }
            }
        }
        
        result
    }
    
    /// Wait for shutdown signal
    pub async fn wait_for_shutdown(&self) -> Result<()> {
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())?;
        
        tokio::select! {
            _ = sigterm.recv() => {
                tracing::info!("Received SIGTERM");
            }
            _ = sigint.recv() => {
                tracing::info!("Received SIGINT");
            }
        }
        
        Ok(())
    }
}

impl Drop for PackerService {
    fn drop(&mut self) {
        // Clean up ring buffer handle if needed
        if let Some(_handle) = &self.ring_buffer {
            // In a real implementation, this would call the C FFI function
            // to clean up the ring buffer connection
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    async fn create_test_service() -> PackerService {
        let temp_dir = TempDir::new().unwrap();
        let mut config = PackerConfig::default();
        config.storage.base_path = temp_dir.path().to_path_buf();
        config.metrics.port = 0; // Disable metrics server
        
        PackerService::new(config).await.unwrap()
    }
    
    #[tokio::test]
    async fn test_service_creation() {
        let service = create_test_service().await;
        let status = service.get_status().await;
        assert_eq!(status.status, ServiceStatus::Starting);
    }
    
    #[tokio::test]
    async fn test_service_start_stop() {
        let mut service = create_test_service().await;
        
        service.start().await.unwrap();
        let status = service.get_status().await;
        assert_eq!(status.status, ServiceStatus::Running);
        
        service.stop().await.unwrap();
        let status = service.get_status().await;
        assert_eq!(status.status, ServiceStatus::Stopped);
    }
    
    #[tokio::test]
    async fn test_manual_processing() {
        let mut service = create_test_service().await;
        service.start().await.unwrap();
        
        let result = service.trigger_processing().await.unwrap();
        
        // Should process 0 events since ring buffer is empty
        assert_eq!(result.events_processed, 0);
        assert_eq!(result.files_created, 0);
        
        service.stop().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_drain_ring_buffer() {
        let config = PackerConfig::default();
        let metrics = Arc::new(MetricsCollector::new(config.metrics.clone()).unwrap());
        
        let events = PackerService::drain_ring_buffer_static(&config, &metrics).await.unwrap();
        
        // Should return empty vector since we don't have a real ring buffer
        assert_eq!(events.len(), 0);
    }
}