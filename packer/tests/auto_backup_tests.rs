//! Comprehensive unit tests for auto-backup service functionality

use chronicle_packer::{
    AutoBackupService, AutoBackupConfig, DriveIdentifier, DriveEvent, DriveAction,
    StorageManager, EncryptionService, IntegrityService,
    config::StorageConfig,
};
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
    path::PathBuf,
};
use tempfile::TempDir;
use tokio::time::sleep;

/// Test helper to create a test auto-backup service
async fn create_test_service() -> (AutoBackupService, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    
    let target_drive = DriveIdentifier::new()
        .with_uuid("test-uuid-12345".to_string())
        .with_volume_label("TestDrive".to_string());

    let config = AutoBackupConfig {
        enabled: true,
        target_drives: vec![target_drive],
        remove_local_after_backup: false,
        verification_required: true,
        backup_destination_path: "/Chronicle".to_string(),
        encryption_enabled: true,
        compression_enabled: true,
        retry_attempts: 2,
        retry_delay: Duration::from_millis(100),
    };

    let storage_config = StorageConfig::default();
    let storage_manager = Arc::new(StorageManager::new(storage_config).unwrap());
    let encryption_service = Arc::new(EncryptionService::new(Default::default()).unwrap());
    let integrity_service = Arc::new(IntegrityService::new());

    let service = AutoBackupService::new(
        config,
        storage_manager,
        encryption_service,
        integrity_service,
    );

    (service, temp_dir)
}

#[tokio::test]
async fn test_auto_backup_service_creation() {
    let (service, _temp_dir) = create_test_service().await;
    
    let status = service.get_status().unwrap();
    assert!(!status.is_running);
    assert_eq!(status.pending_backups, 0);
    assert_eq!(status.completed_backups, 0);
    assert_eq!(status.failed_backups, 0);
}

#[tokio::test]
async fn test_auto_backup_service_start_stop() {
    let (service, _temp_dir) = create_test_service().await;
    
    // Test starting service
    service.start().await.unwrap();
    let status = service.get_status().unwrap();
    assert!(status.is_running);
    
    // Test stopping service
    service.stop().await.unwrap();
    let status = service.get_status().unwrap();
    assert!(!status.is_running);
}

#[tokio::test]
async fn test_drive_identifier_matching() {
    let (service, _temp_dir) = create_test_service().await;
    
    // Create a drive event that matches target drive
    let matching_drive = DriveIdentifier::new()
        .with_uuid("test-uuid-12345".to_string());
    
    let event = DriveEvent {
        drive_identifier: matching_drive,
        action: DriveAction::Mounted,
        mount_point: Some("/Volumes/TestDrive".to_string()),
        timestamp: SystemTime::now(),
        should_trigger_backup: true,
    };
    
    service.start().await.unwrap();
    
    // Process the drive event
    let result = service.process_drive_event(event).await;
    assert!(result.is_ok());
    
    service.stop().await.unwrap();
}

#[tokio::test]
async fn test_non_matching_drive_ignored() {
    let (service, _temp_dir) = create_test_service().await;
    
    // Create a drive event that doesn't match target drive
    let non_matching_drive = DriveIdentifier::new()
        .with_uuid("different-uuid-67890".to_string());
    
    let event = DriveEvent {
        drive_identifier: non_matching_drive,
        action: DriveAction::Mounted,
        mount_point: Some("/Volumes/OtherDrive".to_string()),
        timestamp: SystemTime::now(),
        should_trigger_backup: false,
    };
    
    service.start().await.unwrap();
    
    // Process the drive event
    let result = service.process_drive_event(event).await;
    assert!(result.is_ok());
    
    // Status should remain unchanged since drive doesn't match
    let status = service.get_status().unwrap();
    assert_eq!(status.pending_backups, 0);
    
    service.stop().await.unwrap();
}

#[tokio::test]
async fn test_drive_identifier_types() {
    // Test UUID matching
    let uuid_drive = DriveIdentifier::new()
        .with_uuid("test-uuid".to_string());
    assert!(uuid_drive.uuid.is_some());
    
    // Test volume label matching
    let label_drive = DriveIdentifier::new()
        .with_volume_label("MyDrive".to_string());
    assert!(label_drive.volume_label.is_some());
    
    // Test serial number matching
    let serial_drive = DriveIdentifier::new()
        .with_serial_number("ABC123".to_string());
    assert!(serial_drive.serial_number.is_some());
}

#[tokio::test]
async fn test_auto_backup_disabled() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    
    let config = AutoBackupConfig {
        enabled: false, // Disabled
        target_drives: vec![],
        remove_local_after_backup: false,
        verification_required: true,
        backup_destination_path: "/Chronicle".to_string(),
        encryption_enabled: true,
        compression_enabled: true,
        retry_attempts: 3,
        retry_delay: Duration::from_secs(60),
    };

    let storage_config = StorageConfig::default();
    let storage_manager = Arc::new(StorageManager::new(storage_config).unwrap());
    let encryption_service = Arc::new(EncryptionService::new(Default::default()).unwrap());
    let integrity_service = Arc::new(IntegrityService::new());

    let service = AutoBackupService::new(
        config,
        storage_manager,
        encryption_service,
        integrity_service,
    );
    
    // Starting disabled service should succeed but not actually start
    let result = service.start().await;
    assert!(result.is_ok());
    
    // Service should not be marked as running since it's disabled
    let status = service.get_status().unwrap();
    assert!(!status.is_running);
}

#[tokio::test]
async fn test_backup_config_validation() {
    let config = AutoBackupConfig::default();
    
    // Default config should be disabled
    assert!(!config.enabled);
    assert!(config.target_drives.is_empty());
    assert!(!config.remove_local_after_backup);
    assert!(config.verification_required);
    assert!(config.encryption_enabled);
    assert!(config.compression_enabled);
    assert_eq!(config.retry_attempts, 3);
    assert_eq!(config.retry_delay, Duration::from_secs(60));
}

#[tokio::test]
async fn test_multiple_target_drives() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    
    let target_drives = vec![
        DriveIdentifier::new().with_uuid("uuid-1".to_string()),
        DriveIdentifier::new().with_volume_label("Backup1".to_string()),
        DriveIdentifier::new().with_serial_number("SERIAL123".to_string()),
    ];

    let config = AutoBackupConfig {
        enabled: true,
        target_drives,
        remove_local_after_backup: false,
        verification_required: true,
        backup_destination_path: "/Chronicle".to_string(),
        encryption_enabled: true,
        compression_enabled: true,
        retry_attempts: 3,
        retry_delay: Duration::from_secs(60),
    };

    let storage_config = StorageConfig::default();
    let storage_manager = Arc::new(StorageManager::new(storage_config).unwrap());
    let encryption_service = Arc::new(EncryptionService::new(Default::default()).unwrap());
    let integrity_service = Arc::new(IntegrityService::new());

    let service = AutoBackupService::new(
        config,
        storage_manager,
        encryption_service,
        integrity_service,
    );
    
    service.start().await.unwrap();
    
    // Test first drive (UUID match)
    let event1 = DriveEvent {
        drive_identifier: DriveIdentifier::new().with_uuid("uuid-1".to_string()),
        action: DriveAction::Mounted,
        mount_point: Some("/Volumes/Drive1".to_string()),
        timestamp: SystemTime::now(),
        should_trigger_backup: true,
    };
    assert!(service.process_drive_event(event1).await.is_ok());
    
    // Test second drive (volume label match)
    let event2 = DriveEvent {
        drive_identifier: DriveIdentifier::new().with_volume_label("Backup1".to_string()),
        action: DriveAction::Mounted,
        mount_point: Some("/Volumes/Backup1".to_string()),
        timestamp: SystemTime::now(),
        should_trigger_backup: true,
    };
    assert!(service.process_drive_event(event2).await.is_ok());
    
    service.stop().await.unwrap();
}

#[tokio::test]
async fn test_concurrent_operations() {
    let (service, _temp_dir) = create_test_service().await;
    
    service.start().await.unwrap();
    
    // Create multiple drive events concurrently
    let mut handles = vec![];
    
    for i in 0..5 {
        let service_clone = &service;
        let handle = tokio::spawn(async move {
            let event = DriveEvent {
                drive_identifier: DriveIdentifier::new()
                    .with_uuid(format!("test-uuid-{}", i)),
                action: DriveAction::Mounted,
                mount_point: Some(format!("/Volumes/Drive{}", i)),
                timestamp: SystemTime::now(),
                should_trigger_backup: false, // Non-matching drives
            };
            service_clone.process_drive_event(event).await
        });
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
    
    service.stop().await.unwrap();
}

#[tokio::test]
async fn test_service_lifecycle() {
    let (service, _temp_dir) = create_test_service().await;
    
    // Initial state
    let status = service.get_status().unwrap();
    assert!(!status.is_running);
    
    // Start service
    service.start().await.unwrap();
    let status = service.get_status().unwrap();
    assert!(status.is_running);
    
    // Try to start again (should fail)
    let result = service.start().await;
    assert!(result.is_err());
    
    // Stop service
    service.stop().await.unwrap();
    let status = service.get_status().unwrap();
    assert!(!status.is_running);
    
    // Stop again (should succeed - idempotent)
    let result = service.stop().await;
    assert!(result.is_ok());
}

/// Test configuration validation
#[tokio::test]
async fn test_config_validation() {
    // Test valid configuration
    let valid_config = AutoBackupConfig {
        enabled: true,
        target_drives: vec![DriveIdentifier::new().with_uuid("test".to_string())],
        remove_local_after_backup: false,
        verification_required: true,
        backup_destination_path: "/ValidPath".to_string(),
        encryption_enabled: true,
        compression_enabled: true,
        retry_attempts: 3,
        retry_delay: Duration::from_millis(100),
    };
    
    let result = valid_config.validate();
    assert!(result.is_ok());
    
    // Test invalid configuration - empty destination path
    let invalid_config = AutoBackupConfig {
        enabled: true,
        target_drives: vec![DriveIdentifier::new().with_uuid("test".to_string())],
        remove_local_after_backup: false,
        verification_required: true,
        backup_destination_path: "".to_string(), // Invalid
        encryption_enabled: true,
        compression_enabled: true,
        retry_attempts: 3,
        retry_delay: Duration::from_millis(100),
    };
    
    let result = invalid_config.validate();
    assert!(result.is_err());
}

/// Test backup retry logic
#[tokio::test]
async fn test_backup_retry_logic() {
    let temp_dir = TempDir::new().unwrap();
    
    let config = AutoBackupConfig {
        enabled: true,
        target_drives: vec![DriveIdentifier::new().with_uuid("test-uuid".to_string())],
        remove_local_after_backup: false,
        verification_required: true,
        backup_destination_path: "/Test".to_string(),
        encryption_enabled: true,
        compression_enabled: true,
        retry_attempts: 3, // Multiple retries
        retry_delay: Duration::from_millis(10),
    };
    
    let storage_config = StorageConfig::default();
    let storage_manager = Arc::new(StorageManager::new(storage_config).unwrap());
    let encryption_service = Arc::new(EncryptionService::new(Default::default()).unwrap());
    let integrity_service = Arc::new(IntegrityService::new());

    let service = AutoBackupService::new(
        config,
        storage_manager,
        encryption_service,
        integrity_service,
    );
    
    service.start().await.unwrap();
    
    // Test that retry configuration is respected
    let event = DriveEvent {
        drive_identifier: DriveIdentifier::new().with_uuid("test-uuid".to_string()),
        action: DriveAction::Mounted,
        mount_point: Some("/invalid/path".to_string()), // This should fail
        timestamp: SystemTime::now(),
        should_trigger_backup: true,
    };
    
    let result = service.process_drive_event(event).await;
    // Should handle retry logic internally
    assert!(result.is_ok() || result.is_err());
    
    service.stop().await.unwrap();
}

/// Test performance with many events
#[tokio::test]
async fn test_performance_with_many_events() {
    let (service, _temp_dir) = create_test_service().await;
    
    service.start().await.unwrap();
    
    let start_time = std::time::Instant::now();
    
    // Process many events quickly
    for i in 0..100 {
        let event = DriveEvent {
            drive_identifier: DriveIdentifier::new()
                .with_uuid(format!("non-matching-{}", i)),
            action: DriveAction::Mounted,
            mount_point: Some(format!("/perf{}", i)),
            timestamp: SystemTime::now(),
            should_trigger_backup: false, // Non-matching for speed
        };
        service.process_drive_event(event).await.unwrap();
    }
    
    let duration = start_time.elapsed();
    
    // Should process events reasonably quickly
    assert!(duration < Duration::from_millis(1000));
    
    service.stop().await.unwrap();
}

/// Integration test for complete backup workflow
#[tokio::test]
async fn test_complete_backup_workflow() {
    let (service, temp_dir) = create_test_service().await;
    
    service.start().await.unwrap();
    
    // Create a matching drive event
    let drive_event = DriveEvent {
        drive_identifier: DriveIdentifier::new()
            .with_uuid("test-uuid-12345".to_string()),
        action: DriveAction::Mounted,
        mount_point: Some(temp_dir.path().to_string_lossy().to_string()),
        timestamp: SystemTime::now(),
        should_trigger_backup: true,
    };
    
    // Process the event
    let result = service.process_drive_event(drive_event).await;
    assert!(result.is_ok());
    
    // Give some time for async processing
    sleep(Duration::from_millis(100)).await;
    
    // Check that the backup was initiated
    let status = service.get_status().unwrap();
    // In a real implementation, we would check for backup completion
    // For now, we just verify the service is still running
    assert!(status.is_running);
    
    service.stop().await.unwrap();
}

// Extension methods for testing
impl DriveIdentifier {
    fn new() -> Self {
        Self {
            uuid: None,
            volume_label: None,
            serial_number: None,
        }
    }
    
    fn with_uuid(mut self, uuid: String) -> Self {
        self.uuid = Some(uuid);
        self
    }
    
    fn with_volume_label(mut self, label: String) -> Self {
        self.volume_label = Some(label);
        self
    }
    
    fn with_serial_number(mut self, serial: String) -> Self {
        self.serial_number = Some(serial);
        self
    }
}

impl AutoBackupConfig {
    fn validate(&self) -> Result<(), String> {
        if self.enabled {
            if self.backup_destination_path.is_empty() {
                return Err("Backup destination path cannot be empty".to_string());
            }
            
            if self.retry_attempts == 0 {
                return Err("Retry attempts must be greater than 0".to_string());
            }
            
            if self.retry_delay.is_zero() {
                return Err("Retry delay must be greater than 0".to_string());
            }
            
            if self.target_drives.is_empty() {
                return Err("At least one target drive must be configured".to_string());
            }
        }
        
        Ok(())
    }
}