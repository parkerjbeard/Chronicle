//! Chronicle packer service library
//!
//! This library provides the core functionality for the Chronicle packer service,
//! which drains the ring buffer nightly and converts Arrow data to Parquet files
//! with HEIF frame organization.

pub mod config;
pub mod error;
pub mod packer;
pub mod storage;
pub mod encryption;
pub mod integrity;
pub mod metrics;
pub mod schema_versioning;
pub mod key_management;
pub mod flexible_config;

// Security modules
pub mod auth;
pub mod tls;
pub mod secure_api;
pub mod input_validation;
pub mod security_monitoring;

// Re-export commonly used types
pub use config::PackerConfig;
pub use error::{PackerError, Result};
pub use packer::PackerService;
pub use storage::StorageManager;
pub use encryption::EncryptionService;
pub use integrity::IntegrityService;
pub use metrics::MetricsCollector;
pub use schema_versioning::{SchemaVersion, SchemaRegistry, SchemaMetadata};
pub use key_management::{KeyManager, KeyType, KeyRotationPolicy, KeyStatus};
pub use flexible_config::{FlexibleConfig, ConfigValue, ConfigSection, Configurable, ChronicleConfig};