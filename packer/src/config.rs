//! Configuration management for the Chronicle packer service
//!
//! This module handles loading, parsing, and validating configuration
//! from various sources including TOML files, environment variables,
//! and command line arguments.

use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::error::{ConfigError, ConfigResult};

/// Main configuration structure for the packer service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackerConfig {
    /// Storage configuration
    pub storage: StorageConfig,
    
    /// Encryption configuration
    pub encryption: EncryptionConfig,
    
    /// Scheduling configuration
    pub scheduling: SchedulingConfig,
    
    /// Ring buffer configuration
    pub ring_buffer: RingBufferConfig,
    
    /// Metrics configuration
    pub metrics: MetricsConfig,
    
    /// Logging configuration
    pub logging: LoggingConfig,
    
    /// Performance tuning configuration
    pub performance: PerformanceConfig,
}

/// Storage related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Base path for Chronicle data storage
    pub base_path: PathBuf,
    
    /// Retention policy in days
    pub retention_days: u32,
    
    /// Compression level (0-9)
    pub compression_level: u8,
    
    /// Maximum file size in bytes before rotation
    pub max_file_size: u64,
    
    /// Directory structure format
    pub directory_format: String,
    
    /// Parquet configuration
    pub parquet: ParquetConfig,
    
    /// HEIF configuration
    pub heif: HeifConfig,
    
    /// Backup configuration
    pub backup: BackupConfig,
}

/// Parquet-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParquetConfig {
    /// Row group size
    pub row_group_size: usize,
    
    /// Page size
    pub page_size: usize,
    
    /// Compression algorithm
    pub compression: String,
    
    /// Enable dictionary encoding
    pub dictionary_encoding: bool,
    
    /// Enable statistics
    pub statistics: bool,
    
    /// Bloom filter configuration
    pub bloom_filter: BloomFilterConfig,
}

/// Bloom filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloomFilterConfig {
    /// Enable bloom filters
    pub enabled: bool,
    
    /// False positive probability
    pub fpp: f64,
    
    /// Columns to create bloom filters for
    pub columns: Vec<String>,
}

/// HEIF-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeifConfig {
    /// Quality level (0-100)
    pub quality: u8,
    
    /// Enable lossless compression
    pub lossless: bool,
    
    /// Maximum image dimension
    pub max_dimension: u32,
    
    /// Thumbnail generation
    pub generate_thumbnails: bool,
    
    /// Thumbnail size
    pub thumbnail_size: u32,
    
    /// Metadata preservation
    pub preserve_metadata: bool,
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Enable automatic backup
    pub enabled: bool,
    
    /// Backup destination path
    pub destination: Option<PathBuf>,
    
    /// Backup schedule
    pub schedule: String,
    
    /// Remove files after backup
    pub remove_after_backup: bool,
    
    /// Retention policy for backups
    pub backup_retention_days: u32,
    
    /// Compression for backups
    pub compress_backups: bool,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Enable encryption
    pub enabled: bool,
    
    /// Encryption algorithm
    pub algorithm: String,
    
    /// Key derivation function
    pub kdf: String,
    
    /// Key derivation iterations
    pub kdf_iterations: u32,
    
    /// Salt size in bytes
    pub salt_size: usize,
    
    /// Nonce size in bytes
    pub nonce_size: usize,
    
    /// Key rotation interval in days
    pub key_rotation_days: u32,
    
    /// Keychain service identifier
    pub keychain_service: String,
    
    /// Keychain account identifier
    pub keychain_account: String,
}

/// Scheduling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingConfig {
    /// Daily processing time (HH:MM format)
    pub daily_time: String,
    
    /// Timezone for scheduling
    pub timezone: String,
    
    /// Backup trigger threshold (bytes)
    pub backup_threshold: u64,
    
    /// Maximum processing time in seconds
    pub max_processing_time: u32,
    
    /// Retry configuration
    pub retry: RetryConfig,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    
    /// Base delay between retries in seconds
    pub base_delay: u32,
    
    /// Maximum delay between retries in seconds
    pub max_delay: u32,
    
    /// Backoff multiplier
    pub backoff_multiplier: f64,
}

/// Ring buffer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RingBufferConfig {
    /// Ring buffer path
    pub path: PathBuf,
    
    /// Ring buffer size in bytes
    pub size: usize,
    
    /// Backpressure threshold (0.0-1.0)
    pub backpressure_threshold: f64,
    
    /// Maximum message size in bytes
    pub max_message_size: usize,
    
    /// Read timeout in milliseconds
    pub read_timeout: u32,
    
    /// Write timeout in milliseconds
    pub write_timeout: u32,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,
    
    /// Metrics server bind address
    pub bind_address: String,
    
    /// Metrics server port
    pub port: u16,
    
    /// Metrics collection interval in seconds
    pub collection_interval: u32,
    
    /// Metrics export format
    pub export_format: String,
    
    /// Custom metrics configuration
    pub custom_metrics: HashMap<String, String>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    
    /// Log format
    pub format: String,
    
    /// Log file path
    pub file_path: Option<PathBuf>,
    
    /// Log rotation configuration
    pub rotation: LogRotationConfig,
    
    /// Enable structured logging
    pub structured: bool,
    
    /// Enable console logging
    pub console: bool,
}

/// Log rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    /// Maximum log file size in bytes
    pub max_size: u64,
    
    /// Maximum number of log files to keep
    pub max_files: u32,
    
    /// Compress rotated logs
    pub compress: bool,
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Number of worker threads
    pub worker_threads: usize,
    
    /// Buffer size for I/O operations
    pub io_buffer_size: usize,
    
    /// Batch size for processing
    pub batch_size: usize,
    
    /// Memory limit in bytes
    pub memory_limit: u64,
    
    /// CPU usage limit (0.0-1.0)
    pub cpu_limit: f64,
    
    /// Enable parallel processing
    pub parallel_processing: bool,
    
    /// Prefetch configuration
    pub prefetch: PrefetchConfig,
}

/// Prefetch configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefetchConfig {
    /// Enable prefetching
    pub enabled: bool,
    
    /// Prefetch buffer size
    pub buffer_size: usize,
    
    /// Prefetch read-ahead size
    pub read_ahead: usize,
}

impl Default for PackerConfig {
    fn default() -> Self {
        Self {
            storage: StorageConfig::default(),
            encryption: EncryptionConfig::default(),
            scheduling: SchedulingConfig::default(),
            ring_buffer: RingBufferConfig::default(),
            metrics: MetricsConfig::default(),
            logging: LoggingConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            base_path: PathBuf::from("/ChronicleRaw"),
            retention_days: 60,
            compression_level: 6,
            max_file_size: 1024 * 1024 * 1024, // 1GB
            directory_format: "%Y/%m/%d".to_string(),
            parquet: ParquetConfig::default(),
            heif: HeifConfig::default(),
            backup: BackupConfig::default(),
        }
    }
}

impl Default for ParquetConfig {
    fn default() -> Self {
        Self {
            row_group_size: 65536,
            page_size: 8192,
            compression: "SNAPPY".to_string(),
            dictionary_encoding: true,
            statistics: true,
            bloom_filter: BloomFilterConfig::default(),
        }
    }
}

impl Default for BloomFilterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fpp: 0.1,
            columns: vec!["app_bundle_id".to_string(), "window_title".to_string()],
        }
    }
}

impl Default for HeifConfig {
    fn default() -> Self {
        Self {
            quality: 80,
            lossless: false,
            max_dimension: 4096,
            generate_thumbnails: true,
            thumbnail_size: 256,
            preserve_metadata: false,
        }
    }
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            destination: None,
            schedule: "daily".to_string(),
            remove_after_backup: false,
            backup_retention_days: 90,
            compress_backups: true,
        }
    }
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: "AES-256-GCM".to_string(),
            kdf: "Argon2id".to_string(),
            kdf_iterations: 100000,
            salt_size: 32,
            nonce_size: 12,
            key_rotation_days: 30,
            keychain_service: "com.chronicle.packer".to_string(),
            keychain_account: "encryption-key".to_string(),
        }
    }
}

impl Default for SchedulingConfig {
    fn default() -> Self {
        Self {
            daily_time: "03:00".to_string(),
            timezone: "UTC".to_string(),
            backup_threshold: 50 * 1024 * 1024, // 50MB
            max_processing_time: 3600, // 1 hour
            retry: RetryConfig::default(),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: 1,
            max_delay: 60,
            backoff_multiplier: 2.0,
        }
    }
}

impl Default for RingBufferConfig {
    fn default() -> Self {
        // Use user's cache directory for security
        let default_path = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("chronicle")
            .join("ring_buffer");
        
        Self {
            path: default_path,
            size: 64 * 1024 * 1024, // 64MB
            backpressure_threshold: 0.8,
            max_message_size: 16 * 1024 * 1024, // 16MB
            read_timeout: 5000, // 5 seconds
            write_timeout: 1000, // 1 second
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bind_address: "127.0.0.1".to_string(),
            port: 9090,
            collection_interval: 60,
            export_format: "prometheus".to_string(),
            custom_metrics: HashMap::new(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        // Use user's local data directory for logs
        let default_log_path = dirs::data_local_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp")))
            .join("chronicle")
            .join("logs")
            .join("packer.log");
            
        Self {
            level: "INFO".to_string(),
            format: "json".to_string(),
            file_path: Some(default_log_path),
            rotation: LogRotationConfig::default(),
            structured: true,
            console: true,
        }
    }
}

impl Default for LogRotationConfig {
    fn default() -> Self {
        Self {
            max_size: 100 * 1024 * 1024, // 100MB
            max_files: 10,
            compress: true,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get(),
            io_buffer_size: 64 * 1024, // 64KB
            batch_size: 1000,
            memory_limit: 1024 * 1024 * 1024, // 1GB
            cpu_limit: 0.8,
            parallel_processing: true,
            prefetch: PrefetchConfig::default(),
        }
    }
}

impl Default for PrefetchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            buffer_size: 1024 * 1024, // 1MB
            read_ahead: 4096,
        }
    }
}

impl PackerConfig {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .map_err(|_| ConfigError::FileNotFound { path: path.to_string_lossy().to_string() })?;
        
        let config: PackerConfig = toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError { reason: e.to_string() })?;
        
        config.validate()?;
        Ok(config)
    }
    
    /// Load configuration from environment variables
    pub fn from_env() -> ConfigResult<Self> {
        let mut config = PackerConfig::default();
        
        // Override with environment variables
        if let Ok(base_path) = std::env::var("CHRONICLE_BASE_PATH") {
            config.storage.base_path = PathBuf::from(base_path);
        }
        
        if let Ok(retention_days) = std::env::var("CHRONICLE_RETENTION_DAYS") {
            config.storage.retention_days = retention_days.parse()
                .map_err(|_| ConfigError::InvalidValue { 
                    field: "CHRONICLE_RETENTION_DAYS".to_string(), 
                    value: retention_days 
                })?;
        }
        
        if let Ok(daily_time) = std::env::var("CHRONICLE_DAILY_TIME") {
            config.scheduling.daily_time = daily_time;
        }
        
        if let Ok(encryption_enabled) = std::env::var("CHRONICLE_ENCRYPTION_ENABLED") {
            config.encryption.enabled = encryption_enabled.parse()
                .map_err(|_| ConfigError::InvalidValue { 
                    field: "CHRONICLE_ENCRYPTION_ENABLED".to_string(), 
                    value: encryption_enabled 
                })?;
        }
        
        if let Ok(log_level) = std::env::var("CHRONICLE_LOG_LEVEL") {
            config.logging.level = log_level;
        }
        
        config.validate()?;
        Ok(config)
    }
    
    /// Load configuration with fallback order: file -> env -> defaults
    pub fn load_with_fallback<P: AsRef<Path>>(config_path: Option<P>) -> ConfigResult<Self> {
        // Start with defaults
        let mut config = PackerConfig::default();
        
        // Try to load from file if path is provided
        if let Some(path) = config_path {
            if path.as_ref().exists() {
                config = PackerConfig::from_file(path)?;
            }
        }
        
        // Override with environment variables
        if let Ok(env_config) = PackerConfig::from_env() {
            config = config.merge_with(env_config);
        }
        
        config.validate()?;
        Ok(config)
    }
    
    /// Merge this configuration with another, preferring values from other
    pub fn merge_with(mut self, other: PackerConfig) -> Self {
        // Simple merge - in practice, you'd want more sophisticated merging
        self.storage = other.storage;
        self.encryption = other.encryption;
        self.scheduling = other.scheduling;
        self.ring_buffer = other.ring_buffer;
        self.metrics = other.metrics;
        self.logging = other.logging;
        self.performance = other.performance;
        self
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> ConfigResult<()> {
        // Validate storage configuration
        if self.storage.retention_days == 0 {
            return Err(ConfigError::InvalidValue { 
                field: "storage.retention_days".to_string(), 
                value: "0".to_string() 
            });
        }
        
        if self.storage.compression_level > 9 {
            return Err(ConfigError::InvalidValue { 
                field: "storage.compression_level".to_string(), 
                value: self.storage.compression_level.to_string() 
            });
        }
        
        // Validate encryption configuration
        if self.encryption.enabled {
            if self.encryption.kdf_iterations < 10000 {
                return Err(ConfigError::InvalidValue { 
                    field: "encryption.kdf_iterations".to_string(), 
                    value: self.encryption.kdf_iterations.to_string() 
                });
            }
        }
        
        // Validate scheduling configuration
        if !self.scheduling.daily_time.contains(':') {
            return Err(ConfigError::InvalidValue { 
                field: "scheduling.daily_time".to_string(), 
                value: self.scheduling.daily_time.clone() 
            });
        }
        
        // Validate performance configuration
        if self.performance.worker_threads == 0 {
            return Err(ConfigError::InvalidValue { 
                field: "performance.worker_threads".to_string(), 
                value: "0".to_string() 
            });
        }
        
        if self.performance.cpu_limit <= 0.0 || self.performance.cpu_limit > 1.0 {
            return Err(ConfigError::InvalidValue { 
                field: "performance.cpu_limit".to_string(), 
                value: self.performance.cpu_limit.to_string() 
            });
        }
        
        // Validate ring buffer configuration
        if self.ring_buffer.backpressure_threshold <= 0.0 || self.ring_buffer.backpressure_threshold > 1.0 {
            return Err(ConfigError::InvalidValue { 
                field: "ring_buffer.backpressure_threshold".to_string(), 
                value: self.ring_buffer.backpressure_threshold.to_string() 
            });
        }
        
        Ok(())
    }
    
    /// Get the default configuration file path
    pub fn default_config_path() -> ConfigResult<PathBuf> {
        dirs::config_dir()
            .map(|dir| dir.join("chronicle").join("packer.toml"))
            .ok_or_else(|| ConfigError::ValidationFailed { 
                reason: "Unable to determine config directory".to_string() 
            })
    }
    
    /// Save configuration to a TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> ConfigResult<()> {
        let path = path.as_ref();
        
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|_| ConfigError::ValidationFailed { 
                    reason: format!("Unable to create config directory: {}", parent.display()) 
                })?;
        }
        
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::ValidationFailed { reason: e.to_string() })?;
        
        fs::write(path, content)
            .map_err(|_| ConfigError::PermissionDenied { path: path.to_string_lossy().to_string() })?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_default_config() {
        let config = PackerConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = PackerConfig::default();
        
        // Test invalid retention days
        config.storage.retention_days = 0;
        assert!(config.validate().is_err());
        
        // Test invalid compression level
        config.storage.retention_days = 30;
        config.storage.compression_level = 15;
        assert!(config.validate().is_err());
        
        // Test invalid CPU limit
        config.storage.compression_level = 6;
        config.performance.cpu_limit = 1.5;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_file_operations() {
        let config = PackerConfig::default();
        let temp_file = NamedTempFile::new().unwrap();
        
        // Save config
        config.save_to_file(temp_file.path()).unwrap();
        
        // Load config
        let loaded_config = PackerConfig::from_file(temp_file.path()).unwrap();
        
        // Compare (in practice you'd implement PartialEq)
        assert_eq!(config.storage.retention_days, loaded_config.storage.retention_days);
        assert_eq!(config.encryption.enabled, loaded_config.encryption.enabled);
    }
    
    #[test]
    fn test_config_merge() {
        let mut config1 = PackerConfig::default();
        config1.storage.retention_days = 30;
        
        let mut config2 = PackerConfig::default();
        config2.storage.retention_days = 60;
        
        let merged = config1.merge_with(config2);
        assert_eq!(merged.storage.retention_days, 60);
    }
}