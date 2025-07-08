//! Storage management for Chronicle packer service
//!
//! This module handles writing Parquet files, organizing HEIF frames,
//! and managing the directory structure for Chronicle data.

use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::collections::HashMap;
use std::sync::Arc;

use arrow::array::*;
use arrow::datatypes::*;
use arrow::record_batch::RecordBatch;
use parquet::arrow::{ArrowWriter, ProjectionMask};
use parquet::file::properties::WriterProperties;
use parquet::basic::{Compression, Encoding};
use parquet::schema::types::ColumnPath;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, NaiveDateTime};
use uuid::Uuid;

use crate::config::{StorageConfig, ParquetConfig, HeifConfig};
use crate::error::{StorageError, StorageResult};
use crate::encryption::EncryptionService;
use crate::integrity::IntegrityService;

/// Metadata for a storage file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File path relative to base directory
    pub path: PathBuf,
    
    /// File size in bytes
    pub size: u64,
    
    /// File creation timestamp
    pub created_at: u64,
    
    /// File modification timestamp
    pub modified_at: u64,
    
    /// File format
    pub format: String,
    
    /// Compression algorithm used
    pub compression: Option<String>,
    
    /// Encryption status
    pub encrypted: bool,
    
    /// Checksum for integrity verification
    pub checksum: String,
    
    /// Schema version
    pub schema_version: u32,
    
    /// Record count (for Parquet files)
    pub record_count: Option<u64>,
    
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

/// Chronicle event data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChronicleEvent {
    /// Event timestamp in nanoseconds
    pub timestamp_ns: u64,
    
    /// Event type
    pub event_type: String,
    
    /// Application bundle ID
    pub app_bundle_id: Option<String>,
    
    /// Window title
    pub window_title: Option<String>,
    
    /// Event data (JSON)
    pub data: String,
    
    /// Event session ID
    pub session_id: String,
    
    /// Event ID
    pub event_id: String,
}

/// Storage manager for Chronicle data
pub struct StorageManager {
    /// Configuration
    config: StorageConfig,
    
    /// Encryption service
    encryption: Option<Arc<EncryptionService>>,
    
    /// Integrity service
    integrity: Arc<IntegrityService>,
    
    /// Arrow schema for Chronicle events
    schema: Schema,
    
    /// Parquet writer properties
    writer_properties: WriterProperties,
    
    /// File metadata cache
    metadata_cache: HashMap<PathBuf, FileMetadata>,
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new(
        config: StorageConfig,
        encryption: Option<Arc<EncryptionService>>,
        integrity: Arc<IntegrityService>,
    ) -> StorageResult<Self> {
        let schema = Self::create_arrow_schema();
        let writer_properties = Self::create_writer_properties(&config.parquet);
        
        let mut manager = Self {
            config,
            encryption,
            integrity,
            schema,
            writer_properties,
            metadata_cache: HashMap::new(),
        };
        
        // Initialize storage directories
        manager.initialize_directories()?;
        
        // Load existing metadata
        manager.load_metadata_cache()?;
        
        Ok(manager)
    }
    
    /// Create Arrow schema for Chronicle events
    fn create_arrow_schema() -> Schema {
        Schema::new(vec![
            Field::new("timestamp_ns", DataType::UInt64, false),
            Field::new("event_type", DataType::Utf8, false),
            Field::new("app_bundle_id", DataType::Utf8, true),
            Field::new("window_title", DataType::Utf8, true),
            Field::new("data", DataType::Utf8, false),
            Field::new("session_id", DataType::Utf8, false),
            Field::new("event_id", DataType::Utf8, false),
        ])
    }
    
    /// Create Parquet writer properties
    fn create_writer_properties(config: &ParquetConfig) -> WriterProperties {
        let compression = match config.compression.as_str() {
            "SNAPPY" => Compression::SNAPPY,
            "GZIP" => Compression::GZIP,
            "LZ4" => Compression::LZ4,
            "ZSTD" => Compression::ZSTD,
            _ => Compression::SNAPPY,
        };
        
        let mut builder = WriterProperties::builder()
            .set_compression(compression)
            .set_dictionary_enabled(config.dictionary_encoding)
            .set_statistics_enabled(config.statistics);
        
        // Set row group size
        if config.row_group_size > 0 {
            builder = builder.set_max_row_group_size(config.row_group_size);
        }
        
        // Set page size
        if config.page_size > 0 {
            builder = builder.set_data_page_size_limit(config.page_size);
        }
        
        // Configure bloom filters
        if config.bloom_filter.enabled {
            for column in &config.bloom_filter.columns {
                builder = builder.set_bloom_filter_enabled(
                    ColumnPath::from(column.as_str()),
                    true,
                );
            }
        }
        
        builder.build()
    }
    
    /// Initialize storage directories
    fn initialize_directories(&self) -> StorageResult<()> {
        // Create base directory
        fs::create_dir_all(&self.config.base_path)
            .map_err(|_| StorageError::DirectoryCreationFailed { 
                path: self.config.base_path.to_string_lossy().to_string() 
            })?;
        
        // Create subdirectories
        for subdir in &["parquet", "heif", "metadata", "temp"] {
            let path = self.config.base_path.join(subdir);
            fs::create_dir_all(&path)
                .map_err(|_| StorageError::DirectoryCreationFailed { 
                    path: path.to_string_lossy().to_string() 
                })?;
        }
        
        tracing::info!("Initialized storage directories at: {}", self.config.base_path.display());
        Ok(())
    }
    
    /// Load metadata cache from storage
    fn load_metadata_cache(&mut self) -> StorageResult<()> {
        let metadata_path = self.config.base_path.join("metadata").join("files.json");
        
        if metadata_path.exists() {
            let content = fs::read_to_string(&metadata_path)
                .map_err(|_| StorageError::FileNotFound { 
                    path: metadata_path.to_string_lossy().to_string() 
                })?;
            
            self.metadata_cache = serde_json::from_str(&content)
                .map_err(|_| StorageError::InvalidFormat { 
                    path: metadata_path.to_string_lossy().to_string() 
                })?;
            
            tracing::info!("Loaded {} files from metadata cache", self.metadata_cache.len());
        }
        
        Ok(())
    }
    
    /// Save metadata cache to storage
    fn save_metadata_cache(&self) -> StorageResult<()> {
        let metadata_path = self.config.base_path.join("metadata").join("files.json");
        
        let content = serde_json::to_string_pretty(&self.metadata_cache)
            .map_err(|_| StorageError::MetadataUpdateFailed { 
                reason: "Failed to serialize metadata".to_string() 
            })?;
        
        fs::write(&metadata_path, content)
            .map_err(|_| StorageError::MetadataUpdateFailed { 
                reason: format!("Failed to write metadata to {}", metadata_path.display()) 
            })?;
        
        Ok(())
    }
    
    /// Get directory path for a given date
    pub fn get_date_directory(&self, date: &DateTime<Utc>) -> PathBuf {
        let formatted = date.format(&self.config.directory_format).to_string();
        self.config.base_path.join("parquet").join(formatted)
    }
    
    /// Get HEIF directory path for a given date
    pub fn get_heif_directory(&self, date: &DateTime<Utc>) -> PathBuf {
        let formatted = date.format(&self.config.directory_format).to_string();
        self.config.base_path.join("heif").join(formatted)
    }
    
    /// Write events to a Parquet file
    pub async fn write_events_to_parquet(
        &mut self,
        events: &[ChronicleEvent],
        date: &DateTime<Utc>,
    ) -> StorageResult<PathBuf> {
        if events.is_empty() {
            return Err(StorageError::InvalidFormat { 
                path: "empty events array".to_string() 
            });
        }
        
        // Create date directory
        let date_dir = self.get_date_directory(date);
        fs::create_dir_all(&date_dir)
            .map_err(|_| StorageError::DirectoryCreationFailed { 
                path: date_dir.to_string_lossy().to_string() 
            })?;
        
        // Create file path
        let file_name = format!("events_{}.parquet", date.format("%Y%m%d_%H%M%S"));
        let file_path = date_dir.join(&file_name);
        
        // Convert events to Arrow record batch
        let record_batch = self.events_to_record_batch(events)?;
        
        // Write to temporary file first
        let temp_file_path = file_path.with_extension("tmp");
        self.write_parquet_file(&record_batch, &temp_file_path).await?;
        
        // Encrypt if configured
        if let Some(encryption) = &self.encryption {
            let mut encryption_service = encryption.as_ref().clone();
            encryption_service.encrypt_file(&temp_file_path)
                .map_err(|e| StorageError::InvalidFormat { 
                    path: format!("Encryption failed: {}", e) 
                })?;
        }
        
        // Move to final location
        fs::rename(&temp_file_path, &file_path)
            .map_err(|_| StorageError::ParquetWriteError { 
                reason: format!("Failed to rename {} to {}", temp_file_path.display(), file_path.display()) 
            })?;
        
        // Calculate file size and checksum
        let file_size = fs::metadata(&file_path)
            .map_err(|_| StorageError::FileNotFound { 
                path: file_path.to_string_lossy().to_string() 
            })?
            .len();
        
        let checksum = self.integrity.calculate_file_checksum(&file_path)?;
        
        // Create metadata
        let metadata = FileMetadata {
            path: file_path.strip_prefix(&self.config.base_path)
                .unwrap_or(&file_path)
                .to_path_buf(),
            size: file_size,
            created_at: chrono::Utc::now().timestamp() as u64,
            modified_at: chrono::Utc::now().timestamp() as u64,
            format: "parquet".to_string(),
            compression: Some(self.config.parquet.compression.clone()),
            encrypted: self.encryption.is_some(),
            checksum,
            schema_version: 1,
            record_count: Some(events.len() as u64),
            metadata: HashMap::new(),
        };
        
        // Update metadata cache
        self.metadata_cache.insert(file_path.clone(), metadata);
        self.save_metadata_cache()?;
        
        tracing::info!("Wrote {} events to {}", events.len(), file_path.display());
        Ok(file_path)
    }
    
    /// Convert events to Arrow record batch
    fn events_to_record_batch(&self, events: &[ChronicleEvent]) -> StorageResult<RecordBatch> {
        let mut timestamp_builder = UInt64Builder::new();
        let mut event_type_builder = StringBuilder::new();
        let mut app_bundle_id_builder = StringBuilder::new();
        let mut window_title_builder = StringBuilder::new();
        let mut data_builder = StringBuilder::new();
        let mut session_id_builder = StringBuilder::new();
        let mut event_id_builder = StringBuilder::new();
        
        for event in events {
            timestamp_builder.append_value(event.timestamp_ns);
            event_type_builder.append_value(&event.event_type);
            app_bundle_id_builder.append_option(event.app_bundle_id.as_deref());
            window_title_builder.append_option(event.window_title.as_deref());
            data_builder.append_value(&event.data);
            session_id_builder.append_value(&event.session_id);
            event_id_builder.append_value(&event.event_id);
        }
        
        let arrays: Vec<Arc<dyn Array>> = vec![
            Arc::new(timestamp_builder.finish()),
            Arc::new(event_type_builder.finish()),
            Arc::new(app_bundle_id_builder.finish()),
            Arc::new(window_title_builder.finish()),
            Arc::new(data_builder.finish()),
            Arc::new(session_id_builder.finish()),
            Arc::new(event_id_builder.finish()),
        ];
        
        RecordBatch::try_new(Arc::new(self.schema.clone()), arrays)
            .map_err(|e| StorageError::ParquetWriteError { 
                reason: format!("Failed to create record batch: {}", e) 
            })
    }
    
    /// Write record batch to Parquet file
    async fn write_parquet_file(
        &self,
        record_batch: &RecordBatch,
        file_path: &Path,
    ) -> StorageResult<()> {
        let file = File::create(file_path)
            .map_err(|_| StorageError::ParquetWriteError { 
                reason: format!("Failed to create file: {}", file_path.display()) 
            })?;
        
        let mut writer = ArrowWriter::try_new(file, record_batch.schema(), Some(self.writer_properties.clone()))
            .map_err(|e| StorageError::ParquetWriteError { 
                reason: format!("Failed to create Arrow writer: {}", e) 
            })?;
        
        writer.write(record_batch)
            .map_err(|e| StorageError::ParquetWriteError { 
                reason: format!("Failed to write record batch: {}", e) 
            })?;
        
        writer.close()
            .map_err(|e| StorageError::ParquetWriteError { 
                reason: format!("Failed to close writer: {}", e) 
            })?;
        
        Ok(())
    }
    
    /// Process and store HEIF frames
    pub async fn process_heif_frames(
        &mut self,
        frames: &[HeifFrame],
        date: &DateTime<Utc>,
    ) -> StorageResult<Vec<PathBuf>> {
        if frames.is_empty() {
            return Ok(Vec::new());
        }
        
        // Create HEIF directory
        let heif_dir = self.get_heif_directory(date);
        fs::create_dir_all(&heif_dir)
            .map_err(|_| StorageError::DirectoryCreationFailed { 
                path: heif_dir.to_string_lossy().to_string() 
            })?;
        
        let mut processed_files = Vec::new();
        
        for (i, frame) in frames.iter().enumerate() {
            let file_name = format!("frame_{}_{:06}.heif", date.format("%Y%m%d_%H%M%S"), i);
            let file_path = heif_dir.join(&file_name);
            
            self.process_single_heif_frame(frame, &file_path).await?;
            processed_files.push(file_path);
        }
        
        tracing::info!("Processed {} HEIF frames to {}", frames.len(), heif_dir.display());
        Ok(processed_files)
    }
    
    /// Process a single HEIF frame
    async fn process_single_heif_frame(
        &mut self,
        frame: &HeifFrame,
        file_path: &Path,
    ) -> StorageResult<()> {
        // Load image
        let image = image::load_from_memory(&frame.data)
            .map_err(|_| StorageError::HeifProcessingError { 
                reason: "Failed to load image from memory".to_string() 
            })?;
        
        // Resize if needed
        let processed_image = if image.width() > self.config.heif.max_dimension || 
                                image.height() > self.config.heif.max_dimension {
            image.resize(
                self.config.heif.max_dimension,
                self.config.heif.max_dimension,
                image::imageops::FilterType::Lanczos3,
            )
        } else {
            image
        };
        
        // Convert to HEIF format (placeholder - actual HEIF encoding would need libheif)
        // For now, we'll use JPEG as a placeholder
        let temp_path = file_path.with_extension("jpg");
        processed_image.save(&temp_path)
            .map_err(|_| StorageError::HeifProcessingError { 
                reason: "Failed to save processed image".to_string() 
            })?;
        
        // Encrypt if configured
        if let Some(encryption) = &self.encryption {
            let mut encryption_service = encryption.as_ref().clone();
            encryption_service.encrypt_file(&temp_path)
                .map_err(|e| StorageError::HeifProcessingError { 
                    reason: format!("Encryption failed: {}", e) 
                })?;
        }
        
        // Move to final location
        fs::rename(&temp_path, file_path)
            .map_err(|_| StorageError::HeifProcessingError { 
                reason: format!("Failed to rename {} to {}", temp_path.display(), file_path.display()) 
            })?;
        
        // Generate thumbnail if configured
        if self.config.heif.generate_thumbnails {
            self.generate_thumbnail(&processed_image, file_path).await?;
        }
        
        // Create metadata
        let file_size = fs::metadata(file_path)
            .map_err(|_| StorageError::FileNotFound { 
                path: file_path.to_string_lossy().to_string() 
            })?
            .len();
        
        let checksum = self.integrity.calculate_file_checksum(file_path)?;
        
        let metadata = FileMetadata {
            path: file_path.strip_prefix(&self.config.base_path)
                .unwrap_or(file_path)
                .to_path_buf(),
            size: file_size,
            created_at: chrono::Utc::now().timestamp() as u64,
            modified_at: chrono::Utc::now().timestamp() as u64,
            format: "heif".to_string(),
            compression: None,
            encrypted: self.encryption.is_some(),
            checksum,
            schema_version: 1,
            record_count: None,
            metadata: HashMap::from([
                ("width".to_string(), processed_image.width().to_string()),
                ("height".to_string(), processed_image.height().to_string()),
                ("original_timestamp".to_string(), frame.timestamp.to_string()),
            ]),
        };
        
        // Update metadata cache
        self.metadata_cache.insert(file_path.to_path_buf(), metadata);
        
        Ok(())
    }
    
    /// Generate thumbnail for an image
    async fn generate_thumbnail(
        &self,
        image: &image::DynamicImage,
        original_path: &Path,
    ) -> StorageResult<()> {
        let thumbnail = image.resize(
            self.config.heif.thumbnail_size,
            self.config.heif.thumbnail_size,
            image::imageops::FilterType::Lanczos3,
        );
        
        let thumbnail_path = original_path.with_extension("thumb.jpg");
        thumbnail.save(&thumbnail_path)
            .map_err(|_| StorageError::HeifProcessingError { 
                reason: "Failed to save thumbnail".to_string() 
            })?;
        
        Ok(())
    }
    
    /// Get file metadata
    pub fn get_file_metadata(&self, path: &Path) -> Option<&FileMetadata> {
        self.metadata_cache.get(path)
    }
    
    /// List files in date range
    pub fn list_files_in_date_range(
        &self,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
    ) -> Vec<&FileMetadata> {
        self.metadata_cache
            .values()
            .filter(|metadata| {
                let file_date = chrono::DateTime::from_timestamp(metadata.created_at as i64, 0)
                    .unwrap_or_else(|| chrono::Utc::now());
                file_date >= *start && file_date <= *end
            })
            .collect()
    }
    
    /// Clean up old files based on retention policy
    pub async fn cleanup_old_files(&mut self) -> StorageResult<u64> {
        let cutoff_time = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::days(self.config.retention_days as i64))
            .unwrap_or_else(|| chrono::Utc::now());
        
        let cutoff_timestamp = cutoff_time.timestamp() as u64;
        let mut deleted_count = 0;
        let mut files_to_remove = Vec::new();
        
        for (path, metadata) in &self.metadata_cache {
            if metadata.created_at < cutoff_timestamp {
                let full_path = self.config.base_path.join(&metadata.path);
                if full_path.exists() {
                    fs::remove_file(&full_path)
                        .map_err(|_| StorageError::PermissionDenied { 
                            path: full_path.to_string_lossy().to_string() 
                        })?;
                    deleted_count += 1;
                }
                files_to_remove.push(path.clone());
            }
        }
        
        // Remove from metadata cache
        for path in files_to_remove {
            self.metadata_cache.remove(&path);
        }
        
        // Save updated metadata
        self.save_metadata_cache()?;
        
        tracing::info!("Cleaned up {} old files", deleted_count);
        Ok(deleted_count)
    }
    
    /// Get storage statistics
    pub fn get_storage_stats(&self) -> StorageStats {
        let total_files = self.metadata_cache.len();
        let total_size: u64 = self.metadata_cache.values().map(|m| m.size).sum();
        
        let mut by_format = HashMap::new();
        for metadata in self.metadata_cache.values() {
            *by_format.entry(metadata.format.clone()).or_insert(0) += 1;
        }
        
        StorageStats {
            total_files,
            total_size,
            by_format,
        }
    }
}

/// HEIF frame data structure
#[derive(Debug, Clone)]
pub struct HeifFrame {
    /// Frame timestamp
    pub timestamp: u64,
    
    /// Frame data
    pub data: Vec<u8>,
    
    /// Frame metadata
    pub metadata: HashMap<String, String>,
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total number of files
    pub total_files: usize,
    
    /// Total size in bytes
    pub total_size: u64,
    
    /// Files by format
    pub by_format: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    fn create_test_config() -> StorageConfig {
        let temp_dir = TempDir::new().unwrap();
        let mut config = StorageConfig::default();
        config.base_path = temp_dir.path().to_path_buf();
        config
    }
    
    #[tokio::test]
    async fn test_storage_manager_creation() {
        let config = create_test_config();
        let integrity = Arc::new(IntegrityService::new());
        
        let manager = StorageManager::new(config, None, integrity);
        assert!(manager.is_ok());
    }
    
    #[tokio::test]
    async fn test_write_events_to_parquet() {
        let config = create_test_config();
        let integrity = Arc::new(IntegrityService::new());
        let mut manager = StorageManager::new(config, None, integrity).unwrap();
        
        let events = vec![
            ChronicleEvent {
                timestamp_ns: 1234567890000000000,
                event_type: "key".to_string(),
                app_bundle_id: Some("com.example.app".to_string()),
                window_title: Some("Test Window".to_string()),
                data: r#"{"key": "a", "modifiers": []}"#.to_string(),
                session_id: "session123".to_string(),
                event_id: "event123".to_string(),
            },
        ];
        
        let date = chrono::Utc::now();
        let result = manager.write_events_to_parquet(&events, &date).await;
        assert!(result.is_ok());
        
        let file_path = result.unwrap();
        assert!(file_path.exists());
    }
    
    #[test]
    fn test_events_to_record_batch() {
        let config = create_test_config();
        let integrity = Arc::new(IntegrityService::new());
        let manager = StorageManager::new(config, None, integrity).unwrap();
        
        let events = vec![
            ChronicleEvent {
                timestamp_ns: 1234567890000000000,
                event_type: "key".to_string(),
                app_bundle_id: Some("com.example.app".to_string()),
                window_title: Some("Test Window".to_string()),
                data: r#"{"key": "a"}"#.to_string(),
                session_id: "session123".to_string(),
                event_id: "event123".to_string(),
            },
        ];
        
        let batch = manager.events_to_record_batch(&events);
        assert!(batch.is_ok());
        
        let batch = batch.unwrap();
        assert_eq!(batch.num_rows(), 1);
        assert_eq!(batch.num_columns(), 7);
    }
}