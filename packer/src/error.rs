//! Error handling for the Chronicle packer service
//!
//! This module provides comprehensive error types for all packer operations,
//! including ring buffer interactions, file I/O, encryption, and data processing.

use std::io;
use std::fmt;

use thiserror::Error;

/// The main error type for the packer service
#[derive(Error, Debug)]
pub enum PackerError {
    /// Ring buffer related errors
    #[error("Ring buffer error: {0}")]
    RingBuffer(#[from] RingBufferError),

    /// Storage related errors
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    /// Encryption related errors
    #[error("Encryption error: {0}")]
    Encryption(#[from] EncryptionError),

    /// Configuration related errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Data integrity errors
    #[error("Data integrity error: {0}")]
    Integrity(#[from] IntegrityError),

    /// Arrow/Parquet processing errors
    #[error("Arrow processing error: {0}")]
    Arrow(#[from] arrow::error::ArrowError),

    /// Parquet processing errors
    #[error("Parquet processing error: {0}")]
    Parquet(#[from] parquet::errors::ParquetError),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Scheduling errors
    #[error("Scheduling error: {0}")]
    Scheduling(String),

    /// Metrics errors
    #[error("Metrics error: {0}")]
    Metrics(#[from] MetricsError),

    /// Generic errors
    #[error("{0}")]
    Generic(String),

    /// Critical system errors that require immediate attention
    #[error("Critical system error: {0}")]
    Critical(String),
}

/// Ring buffer specific errors
#[derive(Error, Debug)]
pub enum RingBufferError {
    #[error("Ring buffer is full")]
    Full,

    #[error("Ring buffer is empty")]
    Empty,

    #[error("Ring buffer is corrupted")]
    Corrupted,

    #[error("Ring buffer backpressure detected")]
    Backpressure,

    #[error("Invalid ring buffer operation")]
    InvalidOperation,

    #[error("Ring buffer message too large: {size} bytes")]
    MessageTooLarge { size: usize },

    #[error("Ring buffer initialization failed: {reason}")]
    InitializationFailed { reason: String },

    #[error("Ring buffer read error: {reason}")]
    ReadError { reason: String },

    #[error("Ring buffer write error: {reason}")]
    WriteError { reason: String },

    #[error("Ring buffer memory mapping failed: {reason}")]
    MemoryMappingFailed { reason: String },

    #[error("Ring buffer FFI error: {code}")]
    FfiError { code: i32 },
}

/// Storage related errors
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Directory creation failed: {path}")]
    DirectoryCreationFailed { path: String },

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("File already exists: {path}")]
    FileExists { path: String },

    #[error("Invalid file format: {path}")]
    InvalidFormat { path: String },

    #[error("Insufficient disk space: required {required} bytes, available {available} bytes")]
    InsufficientSpace { required: u64, available: u64 },

    #[error("File permissions error: {path}")]
    PermissionDenied { path: String },

    #[error("Parquet file write error: {reason}")]
    ParquetWriteError { reason: String },

    #[error("HEIF processing error: {reason}")]
    HeifProcessingError { reason: String },

    #[error("Metadata update failed: {reason}")]
    MetadataUpdateFailed { reason: String },

    #[error("File compression failed: {reason}")]
    CompressionFailed { reason: String },

    #[error("File decompression failed: {reason}")]
    DecompressionFailed { reason: String },

    #[error("Storage quota exceeded: {used} bytes used, {limit} bytes limit")]
    QuotaExceeded { used: u64, limit: u64 },
}

/// Encryption related errors
#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Key derivation failed")]
    KeyDerivationFailed,

    #[error("Encryption failed: {reason}")]
    EncryptionFailed { reason: String },

    #[error("Decryption failed: {reason}")]
    DecryptionFailed { reason: String },

    #[error("Invalid key format")]
    InvalidKeyFormat,

    #[error("Key not found")]
    KeyNotFound,

    #[error("Keychain access denied")]
    KeychainAccessDenied,

    #[error("Invalid passphrase")]
    InvalidPassphrase,

    #[error("Nonce generation failed")]
    NonceGenerationFailed,

    #[error("Authentication tag verification failed")]
    AuthenticationFailed,

    #[error("Encryption key rotation failed")]
    KeyRotationFailed,

    #[error("Secure random generation failed")]
    SecureRandomFailed,
}

/// Configuration related errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },

    #[error("Invalid configuration format: {reason}")]
    InvalidFormat { reason: String },

    #[error("Missing required configuration field: {field}")]
    MissingField { field: String },

    #[error("Invalid configuration value: {field} = {value}")]
    InvalidValue { field: String, value: String },

    #[error("Configuration validation failed: {reason}")]
    ValidationFailed { reason: String },

    #[error("Configuration file permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Configuration parsing error: {reason}")]
    ParseError { reason: String },
}

/// Data integrity errors
#[derive(Error, Debug)]
pub enum IntegrityError {
    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Data corruption detected: {reason}")]
    DataCorruption { reason: String },

    #[error("Schema validation failed: {reason}")]
    SchemaValidation { reason: String },

    #[error("Data consistency check failed: {reason}")]
    ConsistencyCheck { reason: String },

    #[error("Timestamp validation failed: {reason}")]
    TimestampValidation { reason: String },

    #[error("Data format validation failed: {reason}")]
    FormatValidation { reason: String },

    #[error("Missing required data field: {field}")]
    MissingField { field: String },

    #[error("Data range validation failed: {field} = {value}")]
    RangeValidation { field: String, value: String },
}

/// Metrics related errors
#[derive(Error, Debug)]
pub enum MetricsError {
    #[error("Metrics collection failed: {reason}")]
    CollectionFailed { reason: String },

    #[error("Metrics export failed: {reason}")]
    ExportFailed { reason: String },

    #[error("Invalid metric name: {name}")]
    InvalidName { name: String },

    #[error("Metric registration failed: {name}")]
    RegistrationFailed { name: String },

    #[error("Metrics server start failed: {reason}")]
    ServerStartFailed { reason: String },
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, PackerError>;

/// A specialized result type for ring buffer operations
pub type RingBufferResult<T> = std::result::Result<T, RingBufferError>;

/// A specialized result type for storage operations
pub type StorageResult<T> = std::result::Result<T, StorageError>;

/// A specialized result type for encryption operations
pub type EncryptionResult<T> = std::result::Result<T, EncryptionError>;

/// A specialized result type for configuration operations
pub type ConfigResult<T> = std::result::Result<T, ConfigError>;

/// A specialized result type for integrity operations
pub type IntegrityResult<T> = std::result::Result<T, IntegrityError>;

/// A specialized result type for metrics operations
pub type MetricsResult<T> = std::result::Result<T, MetricsError>;

impl PackerError {
    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            PackerError::RingBuffer(RingBufferError::Full) => true,
            PackerError::RingBuffer(RingBufferError::Empty) => true,
            PackerError::RingBuffer(RingBufferError::Backpressure) => true,
            PackerError::Storage(StorageError::InsufficientSpace { .. }) => false,
            PackerError::Storage(StorageError::PermissionDenied { .. }) => false,
            PackerError::Encryption(EncryptionError::KeychainAccessDenied) => false,
            PackerError::Critical(_) => false,
            PackerError::Io(io_error) => {
                matches!(io_error.kind(), io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock)
            }
            _ => true,
        }
    }

    /// Check if this error requires immediate attention
    pub fn is_critical(&self) -> bool {
        match self {
            PackerError::Critical(_) => true,
            PackerError::RingBuffer(RingBufferError::Corrupted) => true,
            PackerError::Storage(StorageError::InsufficientSpace { .. }) => true,
            PackerError::Encryption(EncryptionError::KeychainAccessDenied) => true,
            PackerError::Integrity(IntegrityError::DataCorruption { .. }) => true,
            _ => false,
        }
    }

    /// Get the error category for logging and metrics
    pub fn category(&self) -> &'static str {
        match self {
            PackerError::RingBuffer(_) => "ring_buffer",
            PackerError::Storage(_) => "storage",
            PackerError::Encryption(_) => "encryption",
            PackerError::Config(_) => "config",
            PackerError::Integrity(_) => "integrity",
            PackerError::Arrow(_) => "arrow",
            PackerError::Parquet(_) => "parquet",
            PackerError::Io(_) => "io",
            PackerError::Serialization(_) => "serialization",
            PackerError::Scheduling(_) => "scheduling",
            PackerError::Metrics(_) => "metrics",
            PackerError::Generic(_) => "generic",
            PackerError::Critical(_) => "critical",
        }
    }
}

impl From<String> for PackerError {
    fn from(msg: String) -> Self {
        PackerError::Generic(msg)
    }
}

impl From<&str> for PackerError {
    fn from(msg: &str) -> Self {
        PackerError::Generic(msg.to_string())
    }
}

impl From<i32> for RingBufferError {
    fn from(code: i32) -> Self {
        match code {
            -1 => RingBufferError::InvalidOperation,
            -2 => RingBufferError::InitializationFailed { reason: "Memory allocation failed".to_string() },
            -3 => RingBufferError::Full,
            -4 => RingBufferError::Empty,
            -5 => RingBufferError::MessageTooLarge { size: 0 },
            -6 => RingBufferError::Corrupted,
            -7 => RingBufferError::Backpressure,
            _ => RingBufferError::FfiError { code },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categorization() {
        let ring_error = PackerError::RingBuffer(RingBufferError::Full);
        assert_eq!(ring_error.category(), "ring_buffer");
        assert!(ring_error.is_recoverable());
        assert!(!ring_error.is_critical());

        let storage_error = PackerError::Storage(StorageError::InsufficientSpace { required: 1000, available: 500 });
        assert_eq!(storage_error.category(), "storage");
        assert!(!storage_error.is_recoverable());
        assert!(storage_error.is_critical());

        let critical_error = PackerError::Critical("System failure".to_string());
        assert_eq!(critical_error.category(), "critical");
        assert!(!critical_error.is_recoverable());
        assert!(critical_error.is_critical());
    }

    #[test]
    fn test_ring_buffer_error_from_code() {
        let error = RingBufferError::from(-3);
        assert!(matches!(error, RingBufferError::Full));

        let error = RingBufferError::from(-6);
        assert!(matches!(error, RingBufferError::Corrupted));

        let error = RingBufferError::from(-99);
        assert!(matches!(error, RingBufferError::FfiError { code: -99 }));
    }

    #[test]
    fn test_error_conversion() {
        let string_error = "Test error".to_string();
        let packer_error = PackerError::from(string_error);
        assert!(matches!(packer_error, PackerError::Generic(_)));

        let str_error = "Test error";
        let packer_error = PackerError::from(str_error);
        assert!(matches!(packer_error, PackerError::Generic(_)));
    }
}