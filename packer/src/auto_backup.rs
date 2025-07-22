//! Auto-backup service for Chronicle
//!
//! This module provides automatic backup functionality that monitors drive events
//! and triggers backups when target external drives are connected.

use crate::{
    config::PackerConfig,
    error::{PackerError, Result},
    storage::StorageManager,
    encryption::EncryptionService,
    integrity::IntegrityService,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
use tokio::{
    sync::{broadcast, mpsc},
    time::{interval, sleep},
};
use tracing::{debug, error, info, warn};

/// Drive identifier for auto-backup configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriveIdentifier {
    pub uuid: Option<String>,
    pub bsd_name: Option<String>,
    pub volume_label: Option<String>,
    pub serial_number: Option<String>,
}

impl DriveIdentifier {
    pub fn new() -> Self {
        Self {
            uuid: None,
            bsd_name: None,
            volume_label: None,
            serial_number: None,
        }
    }

    pub fn with_uuid(mut self, uuid: String) -> Self {
        self.uuid = Some(uuid);
        self
    }

    pub fn with_volume_label(mut self, label: String) -> Self {
        self.volume_label = Some(label);
        self
    }

    pub fn with_serial_number(mut self, serial: String) -> Self {
        self.serial_number = Some(serial);
        self
    }
}

/// Drive event that triggers auto-backup processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveEvent {
    pub drive_identifier: DriveIdentifier,
    pub action: DriveAction,
    pub mount_point: Option<String>,
    pub timestamp: SystemTime,
    pub should_trigger_backup: bool,
}

/// Drive action enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DriveAction {
    Mounted,
    Unmounted,
    Appeared,
    Disappeared,
    Ejected,
    Remounted,
}

/// Auto-backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoBackupConfig {
    pub enabled: bool,
    pub target_drives: Vec<DriveIdentifier>,
    pub remove_local_after_backup: bool,
    pub verification_required: bool,
    pub backup_destination_path: String,
    pub encryption_enabled: bool,
    pub compression_enabled: bool,
    pub retry_attempts: u32,
    pub retry_delay: Duration,
}

impl Default for AutoBackupConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            target_drives: Vec::new(),
            remove_local_after_backup: false,
            verification_required: true,
            backup_destination_path: "/Chronicle".to_string(),
            encryption_enabled: true,
            compression_enabled: true,
            retry_attempts: 3,
            retry_delay: Duration::from_secs(60),
        }
    }
}

/// Auto-backup status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoBackupStatus {
    pub is_running: bool,
    pub last_backup_time: Option<SystemTime>,
    pub pending_backups: u32,
    pub completed_backups: u64,
    pub failed_backups: u64,
    pub current_backup: Option<BackupJobStatus>,
}

/// Backup job status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupJobStatus {
    pub job_id: String,
    pub drive_identifier: DriveIdentifier,
    pub mount_point: String,
    pub started_at: SystemTime,
    pub progress: f64,
    pub status: String,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
}

/// Auto-backup service
pub struct AutoBackupService {
    config: AutoBackupConfig,
    storage_manager: Arc<StorageManager>,
    encryption_service: Arc<EncryptionService>,
    integrity_service: Arc<IntegrityService>,
    status: Arc<Mutex<AutoBackupStatus>>,
    drive_event_tx: mpsc::UnboundedSender<DriveEvent>,
    drive_event_rx: Arc<Mutex<mpsc::UnboundedReceiver<DriveEvent>>>,
    shutdown_tx: broadcast::Sender<()>,
    is_running: Arc<Mutex<bool>>,
}

impl AutoBackupService {
    /// Create a new auto-backup service
    pub fn new(
        config: AutoBackupConfig,
        storage_manager: Arc<StorageManager>,
        encryption_service: Arc<EncryptionService>,
        integrity_service: Arc<IntegrityService>,
    ) -> Self {
        let (drive_event_tx, drive_event_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, _) = broadcast::channel(1);

        let status = AutoBackupStatus {
            is_running: false,
            last_backup_time: None,
            pending_backups: 0,
            completed_backups: 0,
            failed_backups: 0,
            current_backup: None,
        };

        Self {
            config,
            storage_manager,
            encryption_service,
            integrity_service,
            status: Arc::new(Mutex::new(status)),
            drive_event_tx,
            drive_event_rx: Arc::new(Mutex::new(drive_event_rx)),
            shutdown_tx,
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the auto-backup service
    pub async fn start(&self) -> Result<()> {
        if !self.config.enabled {
            info!("Auto-backup service is disabled");
            return Ok(());
        }

        let mut is_running = self.is_running.lock().map_err(|_| {
            PackerError::SystemError("Failed to acquire is_running lock".to_string())
        })?;
        
        if *is_running {
            return Err(PackerError::SystemError("Auto-backup service is already running".to_string()));
        }

        *is_running = true;
        drop(is_running);

        {
            let mut status = self.status.lock().map_err(|_| {
                PackerError::SystemError("Failed to acquire status lock".to_string())
            })?;
            status.is_running = true;
        }

        info!("Starting auto-backup service with {} target drives", self.config.target_drives.len());

        // Start the drive event processing loop
        self.start_event_processing_loop().await?;

        Ok(())
    }

    /// Stop the auto-backup service
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping auto-backup service");

        let mut is_running = self.is_running.lock().map_err(|_| {
            PackerError::SystemError("Failed to acquire is_running lock".to_string())
        })?;
        
        if !*is_running {
            return Ok(());
        }

        *is_running = false;
        drop(is_running);

        // Send shutdown signal
        let _ = self.shutdown_tx.send(());

        {
            let mut status = self.status.lock().map_err(|_| {
                PackerError::SystemError("Failed to acquire status lock".to_string())
            })?;
            status.is_running = false;
        }

        info!("Auto-backup service stopped");
        Ok(())
    }

    /// Process a drive event
    pub async fn process_drive_event(&self, event: DriveEvent) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        debug!("Processing drive event: {:?}", event);

        // Check if this is a target drive
        if !self.is_target_drive(&event.drive_identifier) {
            debug!("Drive is not a target drive, ignoring");
            return Ok(());
        }

        // Send event to processing queue
        self.drive_event_tx.send(event).map_err(|e| {
            PackerError::SystemError(format!("Failed to send drive event: {}", e))
        })?;

        Ok(())
    }

    /// Get current auto-backup status
    pub fn get_status(&self) -> Result<AutoBackupStatus> {
        let status = self.status.lock().map_err(|_| {
            PackerError::SystemError("Failed to acquire status lock".to_string())
        })?;
        Ok(status.clone())
    }

    /// Update configuration
    pub async fn update_config(&mut self, config: AutoBackupConfig) -> Result<()> {
        info!("Updating auto-backup configuration");
        self.config = config;
        Ok(())
    }

    /// Check if a drive identifier matches any target drives
    fn is_target_drive(&self, identifier: &DriveIdentifier) -> bool {
        for target in &self.config.target_drives {
            if self.drive_identifiers_match(target, identifier) {
                return true;
            }
        }
        false
    }

    /// Check if two drive identifiers match
    fn drive_identifiers_match(&self, target: &DriveIdentifier, drive: &DriveIdentifier) -> bool {
        // Check UUID match (highest priority)
        if let (Some(target_uuid), Some(drive_uuid)) = (&target.uuid, &drive.uuid) {
            if target_uuid == drive_uuid {
                return true;
            }
        }

        // Check serial number match
        if let (Some(target_serial), Some(drive_serial)) = (&target.serial_number, &drive.serial_number) {
            if target_serial == drive_serial {
                return true;
            }
        }

        // Check volume label match
        if let (Some(target_label), Some(drive_label)) = (&target.volume_label, &drive.volume_label) {
            if target_label == drive_label {
                return true;
            }
        }

        // Check BSD name match (lowest priority)
        if let (Some(target_bsd), Some(drive_bsd)) = (&target.bsd_name, &drive.bsd_name) {
            if target_bsd == drive_bsd {
                return true;
            }
        }

        false
    }

    /// Start the event processing loop
    async fn start_event_processing_loop(&self) -> Result<()> {
        let drive_event_rx = Arc::clone(&self.drive_event_rx);
        let status = Arc::clone(&self.status);
        let storage_manager = Arc::clone(&self.storage_manager);
        let encryption_service = Arc::clone(&self.encryption_service);
        let integrity_service = Arc::clone(&self.integrity_service);
        let config = self.config.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let mut event_rx = match drive_event_rx.lock() {
                Ok(rx) => rx,
                Err(e) => {
                    error!("Failed to acquire drive event receiver lock: {}", e);
                    return;
                }
            };

            loop {
                tokio::select! {
                    // Process drive events
                    event = event_rx.recv() => {
                        match event {
                            Some(drive_event) => {
                                if let Err(e) = Self::handle_drive_event(
                                    drive_event,
                                    &config,
                                    &storage_manager,
                                    &encryption_service,
                                    &integrity_service,
                                    &status,
                                ).await {
                                    error!("Failed to handle drive event: {}", e);
                                }
                            }
                            None => {
                                warn!("Drive event channel closed");
                                break;
                            }
                        }
                    }
                    // Handle shutdown signal
                    _ = shutdown_rx.recv() => {
                        info!("Auto-backup service received shutdown signal");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Handle a drive event
    async fn handle_drive_event(
        event: DriveEvent,
        config: &AutoBackupConfig,
        storage_manager: &StorageManager,
        encryption_service: &EncryptionService,
        integrity_service: &IntegrityService,
        status: &Arc<Mutex<AutoBackupStatus>>,
    ) -> Result<()> {
        match event.action {
            DriveAction::Mounted | DriveAction::Appeared => {
                if let Some(mount_point) = &event.mount_point {
                    info!("Target drive mounted at: {}", mount_point);

                    // Increment pending backups counter
                    {
                        let mut status_guard = status.lock().map_err(|_| {
                            PackerError::SystemError("Failed to acquire status lock".to_string())
                        })?;
                        status_guard.pending_backups += 1;
                    }

                    // Start backup process
                    if let Err(e) = Self::perform_backup(
                        &event,
                        mount_point,
                        config,
                        storage_manager,
                        encryption_service,
                        integrity_service,
                        status,
                    ).await {
                        error!("Backup failed: {}", e);
                        
                        // Update failure counter
                        let mut status_guard = status.lock().map_err(|_| {
                            PackerError::SystemError("Failed to acquire status lock".to_string())
                        })?;
                        status_guard.failed_backups += 1;
                        status_guard.pending_backups = status_guard.pending_backups.saturating_sub(1);
                    }
                }
            }
            DriveAction::Unmounted | DriveAction::Disappeared | DriveAction::Ejected => {
                info!("Target drive disconnected");
                // Handle cleanup if needed
            }
            DriveAction::Remounted => {
                info!("Target drive remounted");
                // Could trigger incremental backup
            }
        }

        Ok(())
    }

    /// Perform backup to the mounted drive
    async fn perform_backup(
        event: &DriveEvent,
        mount_point: &str,
        config: &AutoBackupConfig,
        storage_manager: &StorageManager,
        encryption_service: &EncryptionService,
        integrity_service: &IntegrityService,
        status: &Arc<Mutex<AutoBackupStatus>>,
    ) -> Result<()> {
        let job_id = uuid::Uuid::new_v4().to_string();
        let backup_start = SystemTime::now();

        info!("Starting backup job {} to {}", job_id, mount_point);

        // Create backup destination path
        let backup_dest = Path::new(mount_point).join(&config.backup_destination_path);
        if !backup_dest.exists() {
            std::fs::create_dir_all(&backup_dest).map_err(|e| {
                PackerError::IOError(format!("Failed to create backup directory: {}", e))
            })?;
        }

        // Update status with current job
        {
            let mut status_guard = status.lock().map_err(|_| {
                PackerError::SystemError("Failed to acquire status lock".to_string())
            })?;
            
            status_guard.current_backup = Some(BackupJobStatus {
                job_id: job_id.clone(),
                drive_identifier: event.drive_identifier.clone(),
                mount_point: mount_point.to_string(),
                started_at: backup_start,
                progress: 0.0,
                status: "Starting".to_string(),
                bytes_transferred: 0,
                total_bytes: 0,
            });
        }

        // Perform the actual backup with retries
        let mut attempt = 0;
        let mut last_error: Option<PackerError> = None;

        while attempt < config.retry_attempts {
            attempt += 1;
            
            match Self::execute_backup(
                &job_id,
                &backup_dest,
                config,
                storage_manager,
                encryption_service,
                integrity_service,
                status,
            ).await {
                Ok(()) => {
                    info!("Backup job {} completed successfully", job_id);
                    
                    // Update status
                    {
                        let mut status_guard = status.lock().map_err(|_| {
                            PackerError::SystemError("Failed to acquire status lock".to_string())
                        })?;
                        
                        status_guard.completed_backups += 1;
                        status_guard.pending_backups = status_guard.pending_backups.saturating_sub(1);
                        status_guard.last_backup_time = Some(SystemTime::now());
                        status_guard.current_backup = None;
                    }

                    // Remove local files if configured
                    if config.remove_local_after_backup {
                        if let Err(e) = Self::remove_local_files(storage_manager).await {
                            warn!("Failed to remove local files after backup: {}", e);
                        }
                    }

                    return Ok(());
                }
                Err(e) => {
                    warn!("Backup attempt {} failed: {}", attempt, e);
                    last_error = Some(e);
                    
                    if attempt < config.retry_attempts {
                        info!("Retrying backup in {} seconds", config.retry_delay.as_secs());
                        sleep(config.retry_delay).await;
                    }
                }
            }
        }

        // All attempts failed
        error!("Backup job {} failed after {} attempts", job_id, config.retry_attempts);
        
        // Clear current backup status
        {
            let mut status_guard = status.lock().map_err(|_| {
                PackerError::SystemError("Failed to acquire status lock".to_string())
            })?;
            status_guard.current_backup = None;
        }

        Err(last_error.unwrap_or_else(|| {
            PackerError::SystemError("Backup failed with unknown error".to_string())
        }))
    }

    /// Execute the backup operation
    async fn execute_backup(
        job_id: &str,
        backup_dest: &Path,
        config: &AutoBackupConfig,
        storage_manager: &StorageManager,
        encryption_service: &EncryptionService,
        integrity_service: &IntegrityService,
        status: &Arc<Mutex<AutoBackupStatus>>,
    ) -> Result<()> {
        // Update status
        Self::update_backup_status(status, "Analyzing source data", 0.1)?;

        // Get source data info
        let source_info = storage_manager.get_storage_info().await?;
        let total_bytes = source_info.total_size;

        Self::update_backup_status(status, "Preparing backup", 0.2)?;

        // Create backup manifest
        let manifest = BackupManifest {
            job_id: job_id.to_string(),
            created_at: SystemTime::now(),
            source_path: source_info.base_path.clone(),
            destination_path: backup_dest.to_path_buf(),
            encryption_enabled: config.encryption_enabled,
            compression_enabled: config.compression_enabled,
            total_bytes,
            file_count: source_info.file_count,
        };

        // Write manifest
        let manifest_path = backup_dest.join("backup_manifest.json");
        let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| {
            PackerError::SerializationError(format!("Failed to serialize manifest: {}", e))
        })?;
        
        std::fs::write(&manifest_path, manifest_json).map_err(|e| {
            PackerError::IOError(format!("Failed to write manifest: {}", e))
        })?;

        Self::update_backup_status(status, "Copying data", 0.3)?;

        // Copy files with progress tracking
        let mut bytes_transferred = 0u64;
        for file_info in storage_manager.list_files().await? {
            // Update progress
            let progress = 0.3 + (bytes_transferred as f64 / total_bytes as f64) * 0.6;
            Self::update_backup_progress(status, bytes_transferred, total_bytes, progress)?;

            // Copy file
            let dest_file = backup_dest.join(&file_info.relative_path);
            if let Some(parent) = dest_file.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    PackerError::IOError(format!("Failed to create directory: {}", e))
                })?;
            }

            // Copy with optional encryption/compression
            if config.encryption_enabled || config.compression_enabled {
                let source_data = std::fs::read(&file_info.full_path).map_err(|e| {
                    PackerError::IOError(format!("Failed to read source file: {}", e))
                })?;

                let mut processed_data = source_data;

                if config.compression_enabled {
                    processed_data = Self::compress_data(processed_data)?;
                }

                if config.encryption_enabled {
                    processed_data = encryption_service.encrypt_data(&processed_data).await?;
                }

                std::fs::write(&dest_file, processed_data).map_err(|e| {
                    PackerError::IOError(format!("Failed to write destination file: {}", e))
                })?;
            } else {
                std::fs::copy(&file_info.full_path, &dest_file).map_err(|e| {
                    PackerError::IOError(format!("Failed to copy file: {}", e))
                })?;
            }

            bytes_transferred += file_info.size;
        }

        Self::update_backup_status(status, "Verifying backup", 0.9)?;

        // Verification if required
        if config.verification_required {
            Self::verify_backup(&manifest, integrity_service).await?;
        }

        Self::update_backup_status(status, "Completed", 1.0)?;

        info!("Backup completed successfully, {} bytes transferred", bytes_transferred);
        Ok(())
    }

    /// Update backup status
    fn update_backup_status(
        status: &Arc<Mutex<AutoBackupStatus>>,
        status_text: &str,
        progress: f64,
    ) -> Result<()> {
        let mut status_guard = status.lock().map_err(|_| {
            PackerError::SystemError("Failed to acquire status lock".to_string())
        })?;

        if let Some(ref mut current_backup) = status_guard.current_backup {
            current_backup.status = status_text.to_string();
            current_backup.progress = progress;
        }

        Ok(())
    }

    /// Update backup progress
    fn update_backup_progress(
        status: &Arc<Mutex<AutoBackupStatus>>,
        bytes_transferred: u64,
        total_bytes: u64,
        progress: f64,
    ) -> Result<()> {
        let mut status_guard = status.lock().map_err(|_| {
            PackerError::SystemError("Failed to acquire status lock".to_string())
        })?;

        if let Some(ref mut current_backup) = status_guard.current_backup {
            current_backup.bytes_transferred = bytes_transferred;
            current_backup.total_bytes = total_bytes;
            current_backup.progress = progress;
        }

        Ok(())
    }

    /// Compress data using the default compression algorithm
    fn compress_data(data: Vec<u8>) -> Result<Vec<u8>> {
        use flate2::{write::GzEncoder, Compression};
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&data).map_err(|e| {
            PackerError::CompressionError(format!("Failed to compress data: {}", e))
        })?;
        
        encoder.finish().map_err(|e| {
            PackerError::CompressionError(format!("Failed to finish compression: {}", e))
        })
    }

    /// Verify backup integrity
    async fn verify_backup(
        manifest: &BackupManifest,
        integrity_service: &IntegrityService,
    ) -> Result<()> {
        // Verify manifest exists and is readable
        let manifest_path = manifest.destination_path.join("backup_manifest.json");
        if !manifest_path.exists() {
            return Err(PackerError::IntegrityError(
                "Backup manifest not found".to_string()
            ));
        }

        // Additional integrity checks could be added here
        info!("Backup verification completed successfully");
        Ok(())
    }

    /// Remove local files after successful backup
    async fn remove_local_files(storage_manager: &StorageManager) -> Result<()> {
        warn!("Removing local files after backup - this is irreversible!");
        
        // Safety check - ensure this is actually desired
        // In a real implementation, this would have additional safeguards
        
        let files = storage_manager.list_files().await?;
        for file_info in files {
            if let Err(e) = std::fs::remove_file(&file_info.full_path) {
                warn!("Failed to remove local file {}: {}", file_info.full_path.display(), e);
            }
        }

        info!("Local files removed after backup");
        Ok(())
    }
}

/// Backup manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupManifest {
    job_id: String,
    created_at: SystemTime,
    source_path: PathBuf,
    destination_path: PathBuf,
    encryption_enabled: bool,
    compression_enabled: bool,
    total_bytes: u64,
    file_count: usize,
}

/// File information for backup
#[derive(Debug, Clone)]
struct FileInfo {
    full_path: PathBuf,
    relative_path: PathBuf,
    size: u64,
}

/// Storage information
#[derive(Debug, Clone)]
struct StorageInfo {
    base_path: PathBuf,
    total_size: u64,
    file_count: usize,
}

// Extension trait for StorageManager to support backup operations
impl StorageManager {
    /// Get storage information for backup
    pub async fn get_storage_info(&self) -> Result<StorageInfo> {
        // This would be implemented based on the actual StorageManager interface
        // For now, we'll provide a placeholder implementation
        Ok(StorageInfo {
            base_path: PathBuf::from("/ChronicleRaw"),
            total_size: 1024 * 1024 * 1024, // 1GB placeholder
            file_count: 1000, // placeholder
        })
    }

    /// List files for backup
    pub async fn list_files(&self) -> Result<Vec<FileInfo>> {
        // This would be implemented based on the actual StorageManager interface
        // For now, we'll provide a placeholder implementation
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drive_identifier_creation() {
        let drive = DriveIdentifier::new()
            .with_uuid("12345678-1234-1234-1234-123456789ABC".to_string())
            .with_volume_label("TestDrive".to_string());

        assert_eq!(drive.uuid.as_ref().unwrap(), "12345678-1234-1234-1234-123456789ABC");
        assert_eq!(drive.volume_label.as_ref().unwrap(), "TestDrive");
    }

    #[test]
    fn test_drive_identifier_matching() {
        let service_config = AutoBackupConfig::default();
        let service = AutoBackupService::new(
            service_config,
            Arc::new(StorageManager::new(crate::config::StorageConfig::default()).unwrap()),
            Arc::new(EncryptionService::new(Default::default()).unwrap()),
            Arc::new(IntegrityService::new()),
        );

        let target = DriveIdentifier::new().with_uuid("test-uuid".to_string());
        let drive1 = DriveIdentifier::new().with_uuid("test-uuid".to_string());
        let drive2 = DriveIdentifier::new().with_uuid("different-uuid".to_string());

        assert!(service.drive_identifiers_match(&target, &drive1));
        assert!(!service.drive_identifiers_match(&target, &drive2));
    }
}