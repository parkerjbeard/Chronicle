//! Integration tests for the Chronicle packer service

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tempfile::TempDir;
use tokio::time::timeout;

use chronicle_packer::{
    config::{PackerConfig, StorageConfig, EncryptionConfig, MetricsConfig},
    packer::{PackerService, ServiceStatus},
    storage::{StorageManager, ChronicleEvent},
    encryption::EncryptionService,
    integrity::IntegrityService,
    metrics::MetricsCollector,
};

/// Create a test configuration with temporary directories
fn create_test_config() -> (PackerConfig, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let mut config = PackerConfig::default();
    
    // Configure storage to use temp directory
    config.storage.base_path = temp_dir.path().to_path_buf();
    config.storage.retention_days = 1; // Short retention for tests
    
    // Disable encryption for tests
    config.encryption.enabled = false;
    
    // Disable metrics server for tests
    config.metrics.enabled = true;
    config.metrics.port = 0;
    
    // Configure ring buffer to use temp path
    config.ring_buffer.path = temp_dir.path().join("ring_buffer");
    config.ring_buffer.size = 1024 * 1024; // 1MB for tests
    
    (config, temp_dir)
}

/// Create test Chronicle events
fn create_test_events(count: usize) -> Vec<ChronicleEvent> {
    let mut events = Vec::new();
    let base_timestamp = 1640995200000000000u64; // 2022-01-01T00:00:00Z
    
    for i in 0..count {
        events.push(ChronicleEvent {
            timestamp_ns: base_timestamp + (i as u64 * 1000000000), // 1 second apart
            event_type: match i % 3 {
                0 => "key".to_string(),
                1 => "mouse".to_string(),
                _ => "window".to_string(),
            },
            app_bundle_id: Some(format!("com.example.app{}", i % 5)),
            window_title: Some(format!("Test Window {}", i)),
            data: format!(r#"{{"index": {}, "test": true}}"#, i),
            session_id: "test_session_123".to_string(),
            event_id: format!("event_{:06}", i),
        });
    }
    
    events
}

#[tokio::test]
async fn test_packer_service_lifecycle() {
    let (config, _temp_dir) = create_test_config();
    
    // Create service
    let mut service = PackerService::new(config).await.unwrap();
    
    // Check initial status
    let status = service.get_status().await;
    assert_eq!(status.status, ServiceStatus::Starting);
    
    // Start service
    service.start().await.unwrap();
    
    // Check running status
    let status = service.get_status().await;
    assert_eq!(status.status, ServiceStatus::Running);
    
    // Stop service
    service.stop().await.unwrap();
    
    // Check stopped status
    let status = service.get_status().await;
    assert_eq!(status.status, ServiceStatus::Stopped);
}

#[tokio::test]
async fn test_manual_processing() {
    let (config, _temp_dir) = create_test_config();
    
    let mut service = PackerService::new(config).await.unwrap();
    service.start().await.unwrap();
    
    // Trigger manual processing
    let result = service.trigger_processing().await.unwrap();
    
    // Should process 0 events since ring buffer is empty
    assert_eq!(result.events_processed, 0);
    assert_eq!(result.files_created, 0);
    assert_eq!(result.bytes_processed, 0);
    assert!(result.duration < Duration::from_secs(1));
    
    service.stop().await.unwrap();
}

#[tokio::test]
async fn test_storage_manager_operations() {
    let (config, _temp_dir) = create_test_config();
    
    let integrity = Arc::new(IntegrityService::new());
    let mut storage = StorageManager::new(
        config.storage.clone(),
        None, // No encryption
        integrity,
    ).unwrap();
    
    // Create test events
    let events = create_test_events(10);
    let date = chrono::Utc::now();
    
    // Write events to Parquet
    let parquet_path = storage.write_events_to_parquet(&events, &date).await.unwrap();
    
    // Verify file was created
    assert!(parquet_path.exists());
    
    // Check file metadata
    let metadata = storage.get_file_metadata(&parquet_path);
    assert!(metadata.is_some());
    
    let metadata = metadata.unwrap();
    assert_eq!(metadata.format, "parquet");
    assert_eq!(metadata.record_count, Some(10));
    assert!(metadata.size > 0);
}

#[tokio::test]
async fn test_encryption_roundtrip() {
    let (config, _temp_dir) = create_test_config();
    
    let mut encryption_config = config.encryption.clone();
    encryption_config.enabled = true;
    encryption_config.kdf_iterations = 1000; // Faster for tests
    
    let mut encryption = EncryptionService::new(encryption_config).unwrap();
    
    let test_data = b"Hello, Chronicle! This is test data for encryption.";
    
    // Encrypt data
    let encrypted = encryption.encrypt(test_data).unwrap();
    assert_ne!(encrypted.as_slice(), test_data);
    
    // Decrypt data
    let decrypted = encryption.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted.as_slice(), test_data);
}

#[tokio::test]
async fn test_file_encryption() {
    let (config, temp_dir) = create_test_config();
    
    let mut encryption_config = config.encryption.clone();
    encryption_config.enabled = true;
    encryption_config.kdf_iterations = 1000;
    
    let mut encryption = EncryptionService::new(encryption_config).unwrap();
    
    // Create test file
    let test_file = temp_dir.path().join("test.txt");
    let original_data = b"Test file content for encryption";
    std::fs::write(&test_file, original_data).unwrap();
    
    // Encrypt file
    encryption.encrypt_file(&test_file).unwrap();
    
    // Verify file is different
    let encrypted_data = std::fs::read(&test_file).unwrap();
    assert_ne!(encrypted_data.as_slice(), original_data);
    
    // Decrypt file
    encryption.decrypt_file(&test_file).unwrap();
    
    // Verify file is restored
    let decrypted_data = std::fs::read(&test_file).unwrap();
    assert_eq!(decrypted_data.as_slice(), original_data);
}

#[tokio::test]
async fn test_integrity_verification() {
    let integrity = IntegrityService::new();
    
    // Test valid events
    let events = create_test_events(5);
    let validation_results = integrity.validate_chronicle_events(&events).unwrap();
    
    assert_eq!(validation_results.len(), 5);
    for result in &validation_results {
        assert!(result.passed, "Event validation failed: {:?}", result.error);
    }
    
    // Test temporal consistency
    let consistency_result = integrity.check_temporal_consistency(&events).unwrap();
    assert!(consistency_result.passed, "Temporal consistency failed: {:?}", consistency_result.error);
}

#[tokio::test]
async fn test_metrics_collection() {
    let (config, _temp_dir) = create_test_config();
    
    let metrics = MetricsCollector::new(config.metrics.clone()).unwrap();
    
    // Record some metrics
    metrics.record_event_processed(100);
    metrics.record_event_failed(5);
    metrics.record_file_created(1024);
    metrics.record_processing_duration(Duration::from_millis(500));
    
    // Get stats
    let stats = metrics.get_packer_stats();
    assert_eq!(stats.events_processed, 100);
    assert_eq!(stats.events_failed, 5);
    assert_eq!(stats.files_created, 1);
    assert_eq!(stats.bytes_processed, 1024);
    assert!(stats.success_rate > 0.0);
    
    // Test metrics export
    let prometheus_output = metrics.export_metrics("prometheus").unwrap();
    assert!(prometheus_output.contains("events_processed_total"));
    
    let json_output = metrics.export_metrics("json").unwrap();
    assert!(json_output.contains("events_processed"));
}

#[tokio::test]
async fn test_storage_cleanup() {
    let (mut config, _temp_dir) = create_test_config();
    config.storage.retention_days = 0; // Immediate cleanup
    
    let integrity = Arc::new(IntegrityService::new());
    let mut storage = StorageManager::new(
        config.storage.clone(),
        None,
        integrity,
    ).unwrap();
    
    // Create some test files
    let events = create_test_events(5);
    let date = chrono::Utc::now();
    
    let _parquet_path = storage.write_events_to_parquet(&events, &date).await.unwrap();
    
    // Wait a moment to ensure file is older than retention
    tokio::time::sleep(Duration::from_millis(10)).await;
    
    // Cleanup old files
    let deleted_count = storage.cleanup_old_files().await.unwrap();
    assert!(deleted_count > 0);
}

#[tokio::test]
async fn test_key_rotation() {
    let (config, _temp_dir) = create_test_config();
    
    let mut encryption_config = config.encryption.clone();
    encryption_config.enabled = true;
    encryption_config.key_rotation_days = 0; // Immediate rotation needed
    encryption_config.kdf_iterations = 1000;
    
    let mut encryption = EncryptionService::new(encryption_config).unwrap();
    
    let original_key_id = encryption.current_key_id().to_string();
    
    // Rotate keys
    encryption.rotate_keys().unwrap();
    
    // Verify new key is different
    assert_ne!(original_key_id, encryption.current_key_id());
    
    // Verify we can still encrypt/decrypt
    let test_data = b"Test after key rotation";
    let encrypted = encryption.encrypt(test_data).unwrap();
    let decrypted = encryption.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted.as_slice(), test_data);
}

#[tokio::test]
async fn test_service_with_timeout() {
    let (config, _temp_dir) = create_test_config();
    
    // Test that service operations complete within reasonable time
    let service_creation = timeout(
        Duration::from_secs(5),
        PackerService::new(config)
    ).await;
    
    assert!(service_creation.is_ok(), "Service creation timed out");
    let mut service = service_creation.unwrap().unwrap();
    
    let service_start = timeout(
        Duration::from_secs(5),
        service.start()
    ).await;
    
    assert!(service_start.is_ok(), "Service start timed out");
    service_start.unwrap().unwrap();
    
    let processing = timeout(
        Duration::from_secs(5),
        service.trigger_processing()
    ).await;
    
    assert!(processing.is_ok(), "Processing timed out");
    processing.unwrap().unwrap();
    
    let service_stop = timeout(
        Duration::from_secs(5),
        service.stop()
    ).await;
    
    assert!(service_stop.is_ok(), "Service stop timed out");
    service_stop.unwrap().unwrap();
}

#[tokio::test]
async fn test_concurrent_operations() {
    let (config, _temp_dir) = create_test_config();
    
    let integrity = Arc::new(IntegrityService::new());
    let storage = Arc::new(tokio::sync::RwLock::new(
        StorageManager::new(config.storage.clone(), None, integrity).unwrap()
    ));
    
    // Create multiple concurrent write operations
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let storage = storage.clone();
        let events = create_test_events(10);
        
        let handle = tokio::spawn(async move {
            let date = chrono::Utc::now() + chrono::Duration::seconds(i);
            let mut storage = storage.write().await;
            storage.write_events_to_parquet(&events, &date).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // Verify all operations succeeded
    for result in results {
        let path_result = result.unwrap();
        assert!(path_result.is_ok(), "Concurrent write failed: {:?}", path_result.err());
        
        let path = path_result.unwrap();
        assert!(path.exists());
    }
}

#[tokio::test]
async fn test_error_handling() {
    let (mut config, _temp_dir) = create_test_config();
    
    // Configure invalid path to trigger errors
    config.storage.base_path = PathBuf::from("/invalid/path/that/does/not/exist");
    
    // This should fail
    let result = PackerService::new(config).await;
    assert!(result.is_err(), "Expected service creation to fail with invalid path");
}

// Helper function for async testing with futures
use futures;

#[tokio::test]
async fn test_full_workflow() {
    let (config, _temp_dir) = create_test_config();
    
    // Create and start service
    let mut service = PackerService::new(config).await.unwrap();
    service.start().await.unwrap();
    
    // Get initial status
    let initial_status = service.get_status().await;
    assert_eq!(initial_status.status, ServiceStatus::Running);
    assert_eq!(initial_status.stats.total_runs, 0);
    
    // Trigger processing
    let result = service.trigger_processing().await.unwrap();
    assert_eq!(result.events_processed, 0); // No events in ring buffer
    
    // Check updated status
    let updated_status = service.get_status().await;
    assert_eq!(updated_status.stats.total_runs, 1);
    assert_eq!(updated_status.stats.successful_runs, 1);
    
    // Stop service
    service.stop().await.unwrap();
    
    let final_status = service.get_status().await;
    assert_eq!(final_status.status, ServiceStatus::Stopped);
}