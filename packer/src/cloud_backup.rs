//! Cloud backup service for Chronicle
//!
//! This module provides cloud backup functionality with client-side encryption,
//! supporting AWS S3 and other cloud providers while maintaining privacy.

#[cfg(feature = "cloud-backup")]
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
#[cfg(feature = "cloud-backup")]
use aws_sdk_s3::{Client as S3Client, types::ObjectStorageClass};
#[cfg(feature = "cloud-backup")]
use aws_types::region::Region;

use crate::{
    config::PackerConfig,
    error::{PackerError, Result},
    storage::StorageManager,
    encryption::EncryptionService,
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
    io::AsyncWriteExt,
};
use tracing::{debug, error, info, warn};

/// Cloud provider enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CloudProvider {
    S3,
    #[allow(dead_code)]
    Gcp,
    #[allow(dead_code)]
    Azure,
}

/// Backup schedule enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackupSchedule {
    Realtime,
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

/// S3 storage class enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum S3StorageClass {
    Standard,
    StandardIA,
    OneZoneIA,
    Glacier,
    GlacierInstantRetrieval,
    DeepArchive,
}

impl From<S3StorageClass> for ObjectStorageClass {
    fn from(class: S3StorageClass) -> Self {
        match class {
            S3StorageClass::Standard => ObjectStorageClass::Standard,
            S3StorageClass::StandardIA => ObjectStorageClass::StandardIa,
            S3StorageClass::OneZoneIA => ObjectStorageClass::OnezoneIa,
            S3StorageClass::Glacier => ObjectStorageClass::Glacier,
            S3StorageClass::GlacierInstantRetrieval => ObjectStorageClass::GlacierIr,
            S3StorageClass::DeepArchive => ObjectStorageClass::DeepArchive,
        }
    }
}

/// S3 backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3BackupConfig {
    pub bucket_name: String,
    pub region: String,
    pub prefix: String,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub use_instance_profile: bool,
    pub storage_class: S3StorageClass,
    pub server_side_encryption: bool,
    pub kms_key_id: Option<String>,
}

impl Default for S3BackupConfig {
    fn default() -> Self {
        Self {
            bucket_name: "chronicle-backups".to_string(),
            region: "us-west-2".to_string(),
            prefix: "chronicle-data".to_string(),
            access_key_id: None,
            secret_access_key: None,
            use_instance_profile: false,
            storage_class: S3StorageClass::StandardIA,
            server_side_encryption: true,
            kms_key_id: None,
        }
    }
}

/// Cloud backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudBackupConfig {
    pub enabled: bool,
    pub provider: CloudProvider,
    pub s3_config: Option<S3BackupConfig>,
    pub continuous_backup: bool,
    pub schedule: BackupSchedule,
    pub encryption_enabled: bool,
    pub client_side_encryption: bool,
    pub retention_days: u32,
    pub max_backup_size: u64,
    pub compression_enabled: bool,
}

impl Default for CloudBackupConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: CloudProvider::S3,
            s3_config: Some(S3BackupConfig::default()),
            continuous_backup: false,
            schedule: BackupSchedule::Daily,
            encryption_enabled: true,
            client_side_encryption: true,
            retention_days: 90,
            max_backup_size: 1024 * 1024 * 1024 * 10, // 10GB
            compression_enabled: true,
        }
    }
}

/// Cloud backup status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudBackupStatus {
    pub is_running: bool,
    pub last_backup_time: Option<SystemTime>,
    pub next_scheduled_backup: Option<SystemTime>,
    pub pending_uploads: u32,
    pub completed_uploads: u64,
    pub failed_uploads: u64,
    pub bytes_uploaded_total: u64,
    pub current_upload: Option<UploadJobStatus>,
}

/// Upload job status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadJobStatus {
    pub job_id: String,
    pub file_path: String,
    pub started_at: SystemTime,
    pub progress: f64,
    pub status: String,
    pub bytes_uploaded: u64,
    pub total_bytes: u64,
}

/// Privacy-preserving cloud backup service
pub struct CloudBackupService {
    config: CloudBackupConfig,
    storage_manager: Arc<StorageManager>,
    encryption_service: Arc<EncryptionService>,
    #[cfg(feature = "cloud-backup")]
    s3_client: Option<Arc<S3Client>>,
    status: Arc<Mutex<CloudBackupStatus>>,
    upload_queue_tx: mpsc::UnboundedSender<UploadJob>,
    upload_queue_rx: Arc<Mutex<mpsc::UnboundedReceiver<UploadJob>>>,
    shutdown_tx: broadcast::Sender<()>,
    is_running: Arc<Mutex<bool>>,
}

/// Upload job definition
#[derive(Debug, Clone)]
struct UploadJob {
    job_id: String,
    file_path: PathBuf,
    cloud_key: String,
    priority: UploadPriority,
    retry_count: u32,
}

/// Upload priority
#[derive(Debug, Clone, PartialEq, PartialOrd)]
enum UploadPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl CloudBackupService {
    /// Create a new cloud backup service
    pub fn new(
        config: CloudBackupConfig,
        storage_manager: Arc<StorageManager>,
        encryption_service: Arc<EncryptionService>,
    ) -> Self {
        let (upload_queue_tx, upload_queue_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, _) = broadcast::channel(1);

        let status = CloudBackupStatus {
            is_running: false,
            last_backup_time: None,
            next_scheduled_backup: None,
            pending_uploads: 0,
            completed_uploads: 0,
            failed_uploads: 0,
            bytes_uploaded_total: 0,
            current_upload: None,
        };

        Self {
            config,
            storage_manager,
            encryption_service,
            #[cfg(feature = "cloud-backup")]
            s3_client: None,
            status: Arc::new(Mutex::new(status)),
            upload_queue_tx,
            upload_queue_rx: Arc::new(Mutex::new(upload_queue_rx)),
            shutdown_tx,
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    /// Initialize cloud provider client
    #[cfg(feature = "cloud-backup")]
    pub async fn initialize(&mut self) -> Result<()> {
        if !self.config.enabled {
            info!("Cloud backup is disabled");
            return Ok(());
        }

        match self.config.provider {
            CloudProvider::S3 => {
                if let Some(s3_config) = &self.config.s3_config {
                    self.s3_client = Some(Arc::new(self.create_s3_client(s3_config).await?));
                    info!("S3 client initialized for bucket: {}", s3_config.bucket_name);
                }
            }
            CloudProvider::Gcp | CloudProvider::Azure => {
                return Err(PackerError::UnsupportedOperation(
                    "GCP and Azure providers not yet implemented".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Initialize cloud provider client (no-op when cloud-backup feature is disabled)
    #[cfg(not(feature = "cloud-backup"))]
    pub async fn initialize(&mut self) -> Result<()> {
        if self.config.enabled {
            return Err(PackerError::UnsupportedOperation(
                "Cloud backup feature is not enabled in this build".to_string()
            ));
        }
        Ok(())
    }

    /// Start the cloud backup service
    pub async fn start(&self) -> Result<()> {
        if !self.config.enabled {
            info!("Cloud backup service is disabled");
            return Ok(());
        }

        let mut is_running = self.is_running.lock().map_err(|_| {
            PackerError::SystemError("Failed to acquire is_running lock".to_string())
        })?;
        
        if *is_running {
            return Err(PackerError::SystemError("Cloud backup service is already running".to_string()));
        }

        *is_running = true;
        drop(is_running);

        {
            let mut status = self.status.lock().map_err(|_| {
                PackerError::SystemError("Failed to acquire status lock".to_string())
            })?;
            status.is_running = true;
        }

        info!("Starting cloud backup service with {} provider", 
              match self.config.provider {
                  CloudProvider::S3 => "S3",
                  CloudProvider::Gcp => "GCP",
                  CloudProvider::Azure => "Azure",
              });

        // Start the upload processing loop
        self.start_upload_processing_loop().await?;

        // Start the scheduled backup loop if not continuous
        if !self.config.continuous_backup {
            self.start_scheduled_backup_loop().await?;
        }

        Ok(())
    }

    /// Stop the cloud backup service
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping cloud backup service");

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

        info!("Cloud backup service stopped");
        Ok(())
    }

    /// Upload a file to cloud storage
    pub async fn upload_file(&self, file_path: &Path, priority: UploadPriority) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let job_id = uuid::Uuid::new_v4().to_string();
        let cloud_key = self.generate_cloud_key(file_path)?;

        let upload_job = UploadJob {
            job_id,
            file_path: file_path.to_path_buf(),
            cloud_key,
            priority,
            retry_count: 0,
        };

        self.upload_queue_tx.send(upload_job).map_err(|e| {
            PackerError::SystemError(format!("Failed to queue upload job: {}", e))
        })?;

        // Update pending uploads counter
        {
            let mut status = self.status.lock().map_err(|_| {
                PackerError::SystemError("Failed to acquire status lock".to_string())
            })?;
            status.pending_uploads += 1;
        }

        Ok(())
    }

    /// Get current cloud backup status
    pub fn get_status(&self) -> Result<CloudBackupStatus> {
        let status = self.status.lock().map_err(|_| {
            PackerError::SystemError("Failed to acquire status lock".to_string())
        })?;
        Ok(status.clone())
    }

    /// Update configuration
    pub async fn update_config(&mut self, config: CloudBackupConfig) -> Result<()> {
        info!("Updating cloud backup configuration");
        let was_running = {
            let is_running = self.is_running.lock().map_err(|_| {
                PackerError::SystemError("Failed to acquire is_running lock".to_string())
            })?;
            *is_running
        };

        if was_running {
            self.stop().await?;
        }

        self.config = config;

        #[cfg(feature = "cloud-backup")]
        {
            self.initialize().await?;
        }

        if was_running && self.config.enabled {
            self.start().await?;
        }

        Ok(())
    }

    /// Create S3 client with configuration
    #[cfg(feature = "cloud-backup")]
    async fn create_s3_client(&self, s3_config: &S3BackupConfig) -> Result<S3Client> {
        let region_provider = RegionProviderChain::default_provider()
            .or_else(Region::new(s3_config.region.clone()));

        let mut config_builder = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider);

        // Set credentials if provided
        if let (Some(access_key), Some(secret_key)) = (&s3_config.access_key_id, &s3_config.secret_access_key) {
            let credentials = aws_credential_types::Credentials::new(
                access_key,
                secret_key,
                None,
                None,
                "chronicle-cloud-backup",
            );
            config_builder = config_builder.credentials_provider(credentials);
        }

        let aws_config = config_builder.load().await;
        Ok(S3Client::new(&aws_config))
    }

    /// Generate cloud storage key for file
    fn generate_cloud_key(&self, file_path: &Path) -> Result<String> {
        let prefix = match &self.config.provider {
            CloudProvider::S3 => {
                self.config.s3_config.as_ref()
                    .map(|c| c.prefix.clone())
                    .unwrap_or_else(|| "chronicle-data".to_string())
            }
            _ => "chronicle-data".to_string(),
        };

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| PackerError::SystemError(format!("Time error: {}", e)))?
            .as_secs();

        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        Ok(format!("{}/year={}/month={}/day={}/{}",
            prefix,
            chrono::DateTime::from_timestamp(timestamp as i64, 0)
                .unwrap_or_default()
                .format("%Y"),
            chrono::DateTime::from_timestamp(timestamp as i64, 0)
                .unwrap_or_default()
                .format("%m"),
            chrono::DateTime::from_timestamp(timestamp as i64, 0)
                .unwrap_or_default()
                .format("%d"),
            file_name
        ))
    }

    /// Start the upload processing loop
    async fn start_upload_processing_loop(&self) -> Result<()> {
        let upload_queue_rx = Arc::clone(&self.upload_queue_rx);
        let status = Arc::clone(&self.status);
        let encryption_service = Arc::clone(&self.encryption_service);
        let config = self.config.clone();
        #[cfg(feature = "cloud-backup")]
        let s3_client = self.s3_client.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let mut queue_rx = match upload_queue_rx.lock() {
                Ok(rx) => rx,
                Err(e) => {
                    error!("Failed to acquire upload queue receiver lock: {}", e);
                    return;
                }
            };

            loop {
                tokio::select! {
                    // Process upload jobs
                    job = queue_rx.recv() => {
                        match job {
                            Some(upload_job) => {
                                #[cfg(feature = "cloud-backup")]
                                {
                                    if let Err(e) = Self::handle_upload_job(
                                        upload_job,
                                        &config,
                                        &encryption_service,
                                        &s3_client,
                                        &status,
                                    ).await {
                                        error!("Failed to handle upload job: {}", e);
                                    }
                                }
                                #[cfg(not(feature = "cloud-backup"))]
                                {
                                    error!("Upload job received but cloud-backup feature is disabled");
                                }
                            }
                            None => {
                                warn!("Upload queue channel closed");
                                break;
                            }
                        }
                    }
                    // Handle shutdown signal
                    _ = shutdown_rx.recv() => {
                        info!("Cloud backup upload processor received shutdown signal");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Start the scheduled backup loop
    async fn start_scheduled_backup_loop(&self) -> Result<()> {
        let storage_manager = Arc::clone(&self.storage_manager);
        let schedule = self.config.schedule.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let interval_duration = match schedule {
                BackupSchedule::Hourly => Duration::from_secs(3600),
                BackupSchedule::Daily => Duration::from_secs(86400),
                BackupSchedule::Weekly => Duration::from_secs(604800),
                BackupSchedule::Monthly => Duration::from_secs(2592000),
                BackupSchedule::Realtime => Duration::from_secs(60), // Check every minute for new files
            };

            let mut interval_timer = interval(interval_duration);

            loop {
                tokio::select! {
                    _ = interval_timer.tick() => {
                        info!("Running scheduled cloud backup");
                        // In a real implementation, this would trigger a full backup
                        // For now, it's a placeholder
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Scheduled backup loop received shutdown signal");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Handle an upload job
    #[cfg(feature = "cloud-backup")]
    async fn handle_upload_job(
        job: UploadJob,
        config: &CloudBackupConfig,
        encryption_service: &EncryptionService,
        s3_client: &Option<Arc<S3Client>>,
        status: &Arc<Mutex<CloudBackupStatus>>,
    ) -> Result<()> {
        let job_start = SystemTime::now();

        info!("Processing upload job {} for file: {}", job.job_id, job.file_path.display());

        // Update status with current job
        {
            let mut status_guard = status.lock().map_err(|_| {
                PackerError::SystemError("Failed to acquire status lock".to_string())
            })?;
            
            let file_size = tokio::fs::metadata(&job.file_path).await
                .map(|m| m.len())
                .unwrap_or(0);

            status_guard.current_upload = Some(UploadJobStatus {
                job_id: job.job_id.clone(),
                file_path: job.file_path.to_string_lossy().to_string(),
                started_at: job_start,
                progress: 0.0,
                status: "Starting".to_string(),
                bytes_uploaded: 0,
                total_bytes: file_size,
            });
        }

        match config.provider {
            CloudProvider::S3 => {
                if let Some(client) = s3_client {
                    Self::upload_to_s3(&job, config, encryption_service, client, status).await?;
                } else {
                    return Err(PackerError::SystemError("S3 client not initialized".to_string()));
                }
            }
            _ => {
                return Err(PackerError::UnsupportedOperation(
                    "Only S3 provider is currently supported".to_string()
                ));
            }
        }

        // Update completion status
        {
            let mut status_guard = status.lock().map_err(|_| {
                PackerError::SystemError("Failed to acquire status lock".to_string())
            })?;
            
            status_guard.completed_uploads += 1;
            status_guard.pending_uploads = status_guard.pending_uploads.saturating_sub(1);
            
            if let Some(ref upload) = status_guard.current_upload {
                status_guard.bytes_uploaded_total += upload.total_bytes;
            }
            
            status_guard.current_upload = None;
            status_guard.last_backup_time = Some(SystemTime::now());
        }

        info!("Upload job {} completed successfully", job.job_id);
        Ok(())
    }

    /// Upload file to S3
    #[cfg(feature = "cloud-backup")]
    async fn upload_to_s3(
        job: &UploadJob,
        config: &CloudBackupConfig,
        encryption_service: &EncryptionService,
        s3_client: &Arc<S3Client>,
        status: &Arc<Mutex<CloudBackupStatus>>,
    ) -> Result<()> {
        let s3_config = config.s3_config.as_ref()
            .ok_or_else(|| PackerError::ConfigurationError("S3 config not found".to_string()))?;

        // Update status
        Self::update_upload_status(status, "Reading file", 0.1)?;

        // Read and process file
        let file_data = tokio::fs::read(&job.file_path).await.map_err(|e| {
            PackerError::IOError(format!("Failed to read file: {}", e))
        })?;

        Self::update_upload_status(status, "Processing data", 0.3)?;

        let mut processed_data = file_data;

        // Apply compression if enabled
        if config.compression_enabled {
            processed_data = Self::compress_data(processed_data)?;
        }

        // Apply client-side encryption if enabled
        if config.client_side_encryption && config.encryption_enabled {
            processed_data = encryption_service.encrypt_data(&processed_data).await?;
        }

        Self::update_upload_status(status, "Uploading to S3", 0.5)?;

        // Create S3 put request
        let mut put_request = s3_client
            .put_object()
            .bucket(&s3_config.bucket_name)
            .key(&job.cloud_key)
            .body(processed_data.into())
            .storage_class(s3_config.storage_class.clone().into());

        // Add server-side encryption if enabled
        if s3_config.server_side_encryption {
            put_request = put_request.server_side_encryption(
                aws_sdk_s3::types::ServerSideEncryption::Aes256
            );

            if let Some(kms_key_id) = &s3_config.kms_key_id {
                put_request = put_request
                    .server_side_encryption(aws_sdk_s3::types::ServerSideEncryption::AwsKms)
                    .ssekms_key_id(kms_key_id);
            }
        }

        // Add metadata
        put_request = put_request
            .metadata("chronicle-version", "1.0")
            .metadata("upload-job-id", &job.job_id)
            .metadata("client-side-encrypted", &config.client_side_encryption.to_string())
            .metadata("compressed", &config.compression_enabled.to_string());

        Self::update_upload_status(status, "Finalizing upload", 0.9)?;

        // Execute upload
        put_request.send().await.map_err(|e| {
            PackerError::CloudError(format!("S3 upload failed: {}", e))
        })?;

        Self::update_upload_status(status, "Completed", 1.0)?;

        info!("Successfully uploaded {} to S3 key: {}", job.file_path.display(), job.cloud_key);
        Ok(())
    }

    /// Update upload status
    fn update_upload_status(
        status: &Arc<Mutex<CloudBackupStatus>>,
        status_text: &str,
        progress: f64,
    ) -> Result<()> {
        let mut status_guard = status.lock().map_err(|_| {
            PackerError::SystemError("Failed to acquire status lock".to_string())
        })?;

        if let Some(ref mut current_upload) = status_guard.current_upload {
            current_upload.status = status_text.to_string();
            current_upload.progress = progress;
            current_upload.bytes_uploaded = (current_upload.total_bytes as f64 * progress) as u64;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_s3_storage_class_conversion() {
        assert_eq!(
            ObjectStorageClass::from(S3StorageClass::Standard),
            ObjectStorageClass::Standard
        );
        assert_eq!(
            ObjectStorageClass::from(S3StorageClass::StandardIA),
            ObjectStorageClass::StandardIa
        );
    }

    #[test]
    fn test_cloud_backup_config_default() {
        let config = CloudBackupConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.provider, CloudProvider::S3);
        assert!(config.encryption_enabled);
        assert!(config.client_side_encryption);
    }

    #[tokio::test]
    async fn test_cloud_backup_service_creation() {
        let config = CloudBackupConfig::default();
        let storage_manager = Arc::new(StorageManager::new(crate::config::StorageConfig::default()).unwrap());
        let encryption_service = Arc::new(EncryptionService::new(Default::default()).unwrap());

        let service = CloudBackupService::new(config, storage_manager, encryption_service);
        let status = service.get_status().unwrap();
        assert!(!status.is_running);
    }

    #[test]
    fn test_generate_cloud_key() {
        let config = CloudBackupConfig::default();
        let storage_manager = Arc::new(StorageManager::new(crate::config::StorageConfig::default()).unwrap());
        let encryption_service = Arc::new(EncryptionService::new(Default::default()).unwrap());

        let service = CloudBackupService::new(config, storage_manager, encryption_service);
        let path = Path::new("/test/file.parquet");
        let key = service.generate_cloud_key(path).unwrap();
        
        assert!(key.starts_with("chronicle-data/year="));
        assert!(key.ends_with("/file.parquet"));
    }
}