//! Data integrity verification for Chronicle packer service
//!
//! This module provides comprehensive data integrity verification
//! including checksums, schema validation, and consistency checks.

use std::path::Path;
use std::fs::File;
use std::io::{BufReader, Read};
use std::collections::HashMap;

use sha2::{Sha256, Digest};
use blake3::Hasher as Blake3Hasher;
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowReader;
use parquet::file::reader::{FileReader, SerializedFileReader};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::error::{IntegrityError, IntegrityResult};
use crate::storage::{ChronicleEvent, FileMetadata};

/// Integrity check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityCheckResult {
    /// File path checked
    pub file_path: String,
    
    /// Check timestamp
    pub checked_at: u64,
    
    /// Overall check result
    pub passed: bool,
    
    /// Individual check results
    pub checks: HashMap<String, CheckResult>,
    
    /// Error messages if any
    pub errors: Vec<String>,
    
    /// Warnings if any
    pub warnings: Vec<String>,
}

/// Individual check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Check name
    pub name: String,
    
    /// Check result
    pub passed: bool,
    
    /// Expected value
    pub expected: Option<String>,
    
    /// Actual value
    pub actual: Option<String>,
    
    /// Error message if failed
    pub error: Option<String>,
}

/// Integrity verification service
pub struct IntegrityService {
    /// Checksum algorithm to use
    checksum_algorithm: ChecksumAlgorithm,
    
    /// Schema cache for validation
    schema_cache: HashMap<String, Schema>,
    
    /// Integrity check history
    check_history: Vec<IntegrityCheckResult>,
}

/// Supported checksum algorithms
#[derive(Debug, Clone, Copy)]
pub enum ChecksumAlgorithm {
    Sha256,
    Blake3,
}

impl IntegrityService {
    /// Create a new integrity service
    pub fn new() -> Self {
        Self {
            checksum_algorithm: ChecksumAlgorithm::Blake3,
            schema_cache: HashMap::new(),
            check_history: Vec::new(),
        }
    }
    
    /// Create integrity service with specific algorithm
    pub fn with_algorithm(algorithm: ChecksumAlgorithm) -> Self {
        Self {
            checksum_algorithm: algorithm,
            schema_cache: HashMap::new(),
            check_history: Vec::new(),
        }
    }
    
    /// Calculate file checksum
    pub fn calculate_file_checksum<P: AsRef<Path>>(&self, path: P) -> IntegrityResult<String> {
        let path = path.as_ref();
        let mut file = File::open(path)
            .map_err(|_| IntegrityError::DataCorruption { 
                reason: format!("Cannot open file: {}", path.display()) 
            })?;
        
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|_| IntegrityError::DataCorruption { 
                reason: format!("Cannot read file: {}", path.display()) 
            })?;
        
        self.calculate_checksum(&buffer)
    }
    
    /// Calculate checksum for data
    pub fn calculate_checksum(&self, data: &[u8]) -> IntegrityResult<String> {
        match self.checksum_algorithm {
            ChecksumAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                Ok(format!("{:x}", hasher.finalize()))
            }
            ChecksumAlgorithm::Blake3 => {
                let mut hasher = Blake3Hasher::new();
                hasher.update(data);
                Ok(hasher.finalize().to_hex().to_string())
            }
        }
    }
    
    /// Verify file integrity
    pub fn verify_file_integrity<P: AsRef<Path>>(
        &mut self,
        path: P,
        expected_metadata: &FileMetadata,
    ) -> IntegrityResult<IntegrityCheckResult> {
        let path = path.as_ref();
        let mut result = IntegrityCheckResult {
            file_path: path.to_string_lossy().to_string(),
            checked_at: chrono::Utc::now().timestamp() as u64,
            passed: true,
            checks: HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        };
        
        // Check if file exists
        if !path.exists() {
            result.passed = false;
            result.errors.push("File does not exist".to_string());
            return Ok(result);
        }
        
        // Check file size
        let file_size_check = self.verify_file_size(path, expected_metadata.size)?;
        result.checks.insert("file_size".to_string(), file_size_check.clone());
        if !file_size_check.passed {
            result.passed = false;
            result.errors.push(file_size_check.error.unwrap_or_default());
        }
        
        // Check file checksum
        let checksum_check = self.verify_file_checksum(path, &expected_metadata.checksum)?;
        result.checks.insert("checksum".to_string(), checksum_check.clone());
        if !checksum_check.passed {
            result.passed = false;
            result.errors.push(checksum_check.error.unwrap_or_default());
        }
        
        // Format-specific checks
        match expected_metadata.format.as_str() {
            "parquet" => {
                let parquet_checks = self.verify_parquet_file(path, expected_metadata)?;
                for (name, check) in parquet_checks {
                    result.checks.insert(name.clone(), check.clone());
                    if !check.passed {
                        result.passed = false;
                        result.errors.push(check.error.unwrap_or_default());
                    }
                }
            }
            "heif" => {
                let heif_checks = self.verify_heif_file(path, expected_metadata)?;
                for (name, check) in heif_checks {
                    result.checks.insert(name.clone(), check.clone());
                    if !check.passed {
                        result.passed = false;
                        result.errors.push(check.error.unwrap_or_default());
                    }
                }
            }
            _ => {
                result.warnings.push(format!("Unknown format: {}", expected_metadata.format));
            }
        }
        
        // Store result in history
        self.check_history.push(result.clone());
        
        // Keep only last 1000 checks
        if self.check_history.len() > 1000 {
            self.check_history.drain(0..self.check_history.len() - 1000);
        }
        
        Ok(result)
    }
    
    /// Verify file size
    fn verify_file_size<P: AsRef<Path>>(
        &self,
        path: P,
        expected_size: u64,
    ) -> IntegrityResult<CheckResult> {
        let path = path.as_ref();
        let metadata = std::fs::metadata(path)
            .map_err(|_| IntegrityError::DataCorruption { 
                reason: format!("Cannot get metadata for: {}", path.display()) 
            })?;
        
        let actual_size = metadata.len();
        let passed = actual_size == expected_size;
        
        Ok(CheckResult {
            name: "file_size".to_string(),
            passed,
            expected: Some(expected_size.to_string()),
            actual: Some(actual_size.to_string()),
            error: if passed {
                None
            } else {
                Some(format!("File size mismatch: expected {}, got {}", expected_size, actual_size))
            },
        })
    }
    
    /// Verify file checksum
    fn verify_file_checksum<P: AsRef<Path>>(
        &self,
        path: P,
        expected_checksum: &str,
    ) -> IntegrityResult<CheckResult> {
        let actual_checksum = self.calculate_file_checksum(path)?;
        let passed = actual_checksum == expected_checksum;
        
        Ok(CheckResult {
            name: "checksum".to_string(),
            passed,
            expected: Some(expected_checksum.to_string()),
            actual: Some(actual_checksum.clone()),
            error: if passed {
                None
            } else {
                Some(format!("Checksum mismatch: expected {}, got {}", expected_checksum, actual_checksum))
            },
        })
    }
    
    /// Verify Parquet file integrity
    fn verify_parquet_file<P: AsRef<Path>>(
        &self,
        path: P,
        expected_metadata: &FileMetadata,
    ) -> IntegrityResult<HashMap<String, CheckResult>> {
        let path = path.as_ref();
        let mut checks = HashMap::new();
        
        // Try to open and read the Parquet file
        let file = File::open(path)
            .map_err(|_| IntegrityError::DataCorruption { 
                reason: format!("Cannot open Parquet file: {}", path.display()) 
            })?;
        
        let reader = SerializedFileReader::new(file)
            .map_err(|_| IntegrityError::DataCorruption { 
                reason: format!("Cannot create Parquet reader: {}", path.display()) 
            })?;
        
        // Check metadata
        let file_metadata = reader.metadata();
        
        // Verify record count if available
        if let Some(expected_count) = expected_metadata.record_count {
            let actual_count = file_metadata.file_metadata().num_rows();
            let passed = actual_count == expected_count as i64;
            
            checks.insert("record_count".to_string(), CheckResult {
                name: "record_count".to_string(),
                passed,
                expected: Some(expected_count.to_string()),
                actual: Some(actual_count.to_string()),
                error: if passed {
                    None
                } else {
                    Some(format!("Record count mismatch: expected {}, got {}", expected_count, actual_count))
                },
            });
        }
        
        // Verify schema
        let schema_check = self.verify_parquet_schema(&reader)?;
        checks.insert("schema".to_string(), schema_check);
        
        // Verify data consistency
        let consistency_check = self.verify_parquet_data_consistency(&reader)?;
        checks.insert("data_consistency".to_string(), consistency_check);
        
        Ok(checks)
    }
    
    /// Verify Parquet schema
    fn verify_parquet_schema(
        &self,
        reader: &SerializedFileReader<File>,
    ) -> IntegrityResult<CheckResult> {
        let file_metadata = reader.metadata();
        let schema = file_metadata.file_metadata().schema();
        
        // Check for expected columns
        let expected_columns = vec![
            "timestamp_ns",
            "event_type",
            "app_bundle_id",
            "window_title",
            "data",
            "session_id",
            "event_id",
        ];
        
        let mut missing_columns = Vec::new();
        for expected_col in &expected_columns {
            if !schema.get_fields().iter().any(|field| field.name() == expected_col) {
                missing_columns.push(expected_col.to_string());
            }
        }
        
        let passed = missing_columns.is_empty();
        
        Ok(CheckResult {
            name: "schema".to_string(),
            passed,
            expected: Some(expected_columns.join(", ")),
            actual: Some(
                schema.get_fields()
                    .iter()
                    .map(|f| f.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            error: if passed {
                None
            } else {
                Some(format!("Missing columns: {}", missing_columns.join(", ")))
            },
        })
    }
    
    /// Verify Parquet data consistency
    fn verify_parquet_data_consistency(
        &self,
        reader: &SerializedFileReader<File>,
    ) -> IntegrityResult<CheckResult> {
        let file_metadata = reader.metadata();
        let mut errors = Vec::new();
        
        // Check row group consistency
        let num_row_groups = file_metadata.num_row_groups();
        for i in 0..num_row_groups {
            let row_group = file_metadata.row_group(i);
            
            // Check that all columns have the same number of rows
            let mut row_counts = Vec::new();
            for j in 0..row_group.num_columns() {
                let column = row_group.column(j);
                row_counts.push(column.num_values());
            }
            
            if !row_counts.iter().all(|&count| count == row_counts[0]) {
                errors.push(format!("Row group {} has inconsistent column row counts", i));
            }
        }
        
        let passed = errors.is_empty();
        
        Ok(CheckResult {
            name: "data_consistency".to_string(),
            passed,
            expected: Some("All row groups have consistent column counts".to_string()),
            actual: Some(if passed {
                "Consistent".to_string()
            } else {
                "Inconsistent".to_string()
            }),
            error: if passed {
                None
            } else {
                Some(errors.join("; "))
            },
        })
    }
    
    /// Verify HEIF file integrity
    fn verify_heif_file<P: AsRef<Path>>(
        &self,
        path: P,
        _expected_metadata: &FileMetadata,
    ) -> IntegrityResult<HashMap<String, CheckResult>> {
        let path = path.as_ref();
        let mut checks = HashMap::new();
        
        // Try to load the image file
        let load_result = image::open(path);
        
        let passed = load_result.is_ok();
        
        checks.insert("image_format".to_string(), CheckResult {
            name: "image_format".to_string(),
            passed,
            expected: Some("Valid image format".to_string()),
            actual: Some(if passed {
                "Valid".to_string()
            } else {
                "Invalid".to_string()
            }),
            error: if passed {
                None
            } else {
                Some(format!("Cannot load image: {}", load_result.err().unwrap()))
            },
        });
        
        // If we can load the image, perform additional checks
        if let Ok(image) = load_result {
            // Check image dimensions
            let width = image.width();
            let height = image.height();
            
            // Basic sanity checks
            let dimensions_valid = width > 0 && height > 0 && width <= 8192 && height <= 8192;
            
            checks.insert("dimensions".to_string(), CheckResult {
                name: "dimensions".to_string(),
                passed: dimensions_valid,
                expected: Some("Valid dimensions (1-8192)".to_string()),
                actual: Some(format!("{}x{}", width, height)),
                error: if dimensions_valid {
                    None
                } else {
                    Some(format!("Invalid dimensions: {}x{}", width, height))
                },
            });
        }
        
        Ok(checks)
    }
    
    /// Validate Chronicle event data
    pub fn validate_chronicle_event(&self, event: &ChronicleEvent) -> IntegrityResult<CheckResult> {
        let mut errors = Vec::new();
        
        // Check timestamp
        if event.timestamp_ns == 0 {
            errors.push("Timestamp cannot be zero".to_string());
        }
        
        // Check event type
        if event.event_type.is_empty() {
            errors.push("Event type cannot be empty".to_string());
        }
        
        // Check event ID
        if event.event_id.is_empty() {
            errors.push("Event ID cannot be empty".to_string());
        }
        
        // Check session ID
        if event.session_id.is_empty() {
            errors.push("Session ID cannot be empty".to_string());
        }
        
        // Validate JSON data
        if serde_json::from_str::<serde_json::Value>(&event.data).is_err() {
            errors.push("Event data is not valid JSON".to_string());
        }
        
        let passed = errors.is_empty();
        
        Ok(CheckResult {
            name: "event_validation".to_string(),
            passed,
            expected: Some("Valid Chronicle event".to_string()),
            actual: Some(if passed {
                "Valid".to_string()
            } else {
                "Invalid".to_string()
            }),
            error: if passed {
                None
            } else {
                Some(errors.join("; "))
            },
        })
    }
    
    /// Validate batch of Chronicle events
    pub fn validate_chronicle_events(&self, events: &[ChronicleEvent]) -> IntegrityResult<Vec<CheckResult>> {
        let mut results = Vec::new();
        
        for (i, event) in events.iter().enumerate() {
            let mut check = self.validate_chronicle_event(event)?;
            check.name = format!("event_{}", i);
            results.push(check);
        }
        
        Ok(results)
    }
    
    /// Check for data consistency across time range
    pub fn check_temporal_consistency(
        &self,
        events: &[ChronicleEvent],
    ) -> IntegrityResult<CheckResult> {
        let mut errors = Vec::new();
        
        // Check timestamp ordering
        for i in 1..events.len() {
            if events[i].timestamp_ns < events[i-1].timestamp_ns {
                errors.push(format!("Timestamp out of order at index {}", i));
            }
        }
        
        // Check for duplicate event IDs
        let mut event_ids = std::collections::HashSet::new();
        for (i, event) in events.iter().enumerate() {
            if !event_ids.insert(&event.event_id) {
                errors.push(format!("Duplicate event ID at index {}: {}", i, event.event_id));
            }
        }
        
        // Check for reasonable timestamp gaps
        let mut large_gaps = Vec::new();
        for i in 1..events.len() {
            let gap = events[i].timestamp_ns - events[i-1].timestamp_ns;
            // Flag gaps larger than 1 hour
            if gap > 3_600_000_000_000 {
                large_gaps.push(format!("Large time gap at index {}: {} ns", i, gap));
            }
        }
        
        if !large_gaps.is_empty() {
            errors.push(format!("Large time gaps detected: {}", large_gaps.join(", ")));
        }
        
        let passed = errors.is_empty();
        
        Ok(CheckResult {
            name: "temporal_consistency".to_string(),
            passed,
            expected: Some("Consistent timestamps and no duplicates".to_string()),
            actual: Some(if passed {
                "Consistent".to_string()
            } else {
                "Inconsistent".to_string()
            }),
            error: if passed {
                None
            } else {
                Some(errors.join("; "))
            },
        })
    }
    
    /// Get integrity check history
    pub fn get_check_history(&self) -> &[IntegrityCheckResult] {
        &self.check_history
    }
    
    /// Get integrity statistics
    pub fn get_integrity_stats(&self) -> IntegrityStats {
        let total_checks = self.check_history.len();
        let passed_checks = self.check_history.iter().filter(|r| r.passed).count();
        let failed_checks = total_checks - passed_checks;
        
        let mut error_counts = HashMap::new();
        for result in &self.check_history {
            for error in &result.errors {
                *error_counts.entry(error.clone()).or_insert(0) += 1;
            }
        }
        
        IntegrityStats {
            total_checks,
            passed_checks,
            failed_checks,
            error_counts,
        }
    }
}

/// Integrity statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityStats {
    /// Total number of checks performed
    pub total_checks: usize,
    
    /// Number of checks that passed
    pub passed_checks: usize,
    
    /// Number of checks that failed
    pub failed_checks: usize,
    
    /// Error counts by type
    pub error_counts: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_integrity_service_creation() {
        let service = IntegrityService::new();
        assert!(matches!(service.checksum_algorithm, ChecksumAlgorithm::Blake3));
    }
    
    #[test]
    fn test_checksum_calculation() {
        let service = IntegrityService::new();
        let data = b"Hello, Chronicle!";
        
        let checksum = service.calculate_checksum(data);
        assert!(checksum.is_ok());
        
        let checksum = checksum.unwrap();
        assert!(!checksum.is_empty());
        assert_eq!(checksum.len(), 64); // Blake3 produces 64-character hex strings
    }
    
    #[test]
    fn test_file_checksum_calculation() {
        let service = IntegrityService::new();
        let temp_file = NamedTempFile::new().unwrap();
        
        // Write test data
        std::fs::write(temp_file.path(), b"Test data").unwrap();
        
        let checksum = service.calculate_file_checksum(temp_file.path());
        assert!(checksum.is_ok());
        
        let checksum = checksum.unwrap();
        assert!(!checksum.is_empty());
    }
    
    #[test]
    fn test_chronicle_event_validation() {
        let service = IntegrityService::new();
        
        // Valid event
        let valid_event = ChronicleEvent {
            timestamp_ns: 1234567890000000000,
            event_type: "key".to_string(),
            app_bundle_id: Some("com.example.app".to_string()),
            window_title: Some("Test Window".to_string()),
            data: r#"{"key": "a"}"#.to_string(),
            session_id: "session123".to_string(),
            event_id: "event123".to_string(),
        };
        
        let result = service.validate_chronicle_event(&valid_event);
        assert!(result.is_ok());
        assert!(result.unwrap().passed);
        
        // Invalid event (empty event type)
        let invalid_event = ChronicleEvent {
            timestamp_ns: 1234567890000000000,
            event_type: "".to_string(),
            app_bundle_id: Some("com.example.app".to_string()),
            window_title: Some("Test Window".to_string()),
            data: r#"{"key": "a"}"#.to_string(),
            session_id: "session123".to_string(),
            event_id: "event123".to_string(),
        };
        
        let result = service.validate_chronicle_event(&invalid_event);
        assert!(result.is_ok());
        assert!(!result.unwrap().passed);
    }
    
    #[test]
    fn test_temporal_consistency_check() {
        let service = IntegrityService::new();
        
        // Events in correct order
        let events = vec![
            ChronicleEvent {
                timestamp_ns: 1000000000000000000,
                event_type: "key".to_string(),
                app_bundle_id: None,
                window_title: None,
                data: "{}".to_string(),
                session_id: "session1".to_string(),
                event_id: "event1".to_string(),
            },
            ChronicleEvent {
                timestamp_ns: 2000000000000000000,
                event_type: "key".to_string(),
                app_bundle_id: None,
                window_title: None,
                data: "{}".to_string(),
                session_id: "session1".to_string(),
                event_id: "event2".to_string(),
            },
        ];
        
        let result = service.check_temporal_consistency(&events);
        assert!(result.is_ok());
        assert!(result.unwrap().passed);
        
        // Events out of order
        let events = vec![
            ChronicleEvent {
                timestamp_ns: 2000000000000000000,
                event_type: "key".to_string(),
                app_bundle_id: None,
                window_title: None,
                data: "{}".to_string(),
                session_id: "session1".to_string(),
                event_id: "event1".to_string(),
            },
            ChronicleEvent {
                timestamp_ns: 1000000000000000000,
                event_type: "key".to_string(),
                app_bundle_id: None,
                window_title: None,
                data: "{}".to_string(),
                session_id: "session1".to_string(),
                event_id: "event2".to_string(),
            },
        ];
        
        let result = service.check_temporal_consistency(&events);
        assert!(result.is_ok());
        assert!(!result.unwrap().passed);
    }
}