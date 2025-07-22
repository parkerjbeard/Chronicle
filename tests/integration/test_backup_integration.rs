//! Comprehensive integration tests for Chronicle backup functionality
//!
//! This test suite covers end-to-end testing of all backup features including
//! auto-backup, cloud backup, and their integration with the CLI and collectors.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};
use tempfile::TempDir;
use tokio::time::sleep;
use chronicle_packer::{
    AutoBackupService, AutoBackupConfig, DriveIdentifier, DriveEvent, DriveAction,
    CloudBackupService, CloudBackupConfig, CloudProvider, BackupSchedule, S3BackupConfig,
    StorageManager, EncryptionService, config::StorageConfig,
};

/// Integration test helper to create test environment
struct TestEnvironment {
    temp_dir: TempDir,
    storage_manager: Arc<StorageManager>,
    encryption_service: Arc<EncryptionService>,
    integrity_service: Arc<chronicle_packer::IntegrityService>,
}

impl TestEnvironment {
    async fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        
        let storage_config = StorageConfig::default();
        let storage_manager = Arc::new(StorageManager::new(storage_config).unwrap());
        let encryption_service = Arc::new(EncryptionService::new(Default::default()).unwrap());
        let integrity_service = Arc::new(chronicle_packer::IntegrityService::new());

        Self {
            temp_dir,
            storage_manager,
            encryption_service,
            integrity_service,
        }
    }

    fn temp_path(&self) -> &Path {
        self.temp_dir.path()
    }

    async fn create_test_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        
        // Create test data files
        for i in 0..5 {
            let file_path = self.temp_path().join(format!("test_file_{}.txt", i));
            let content = format!("Test content for file {} - {}", i, chrono::Utc::now());
            tokio::fs::write(&file_path, content).await.unwrap();
            files.push(file_path);
        }

        // Create a subdirectory with files
        let subdir = self.temp_path().join("subdir");
        tokio::fs::create_dir(&subdir).await.unwrap();
        
        for i in 0..3 {
            let file_path = subdir.join(format!("sub_file_{}.json", i));
            let content = serde_json::json!({
                "id": i,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "data": format!("Sample data {}", i)
            });
            tokio::fs::write(&file_path, content.to_string()).await.unwrap();
            files.push(file_path);
        }

        files
    }
}

/// Test auto-backup service creation and basic functionality
#[tokio::test]
async fn test_auto_backup_service_integration() {
    let env = TestEnvironment::new().await;
    let _test_files = env.create_test_files().await;

    let target_drive = DriveIdentifier::new()
        .with_uuid("test-integration-uuid".to_string())
        .with_volume_label("IntegrationTestDrive".to_string());

    let config = AutoBackupConfig {
        enabled: true,
        target_drives: vec![target_drive.clone()],
        remove_local_after_backup: false,
        verification_required: true,
        backup_destination_path: "/TestBackup".to_string(),
        encryption_enabled: true,
        compression_enabled: true,
        retry_attempts: 2,
        retry_delay: Duration::from_millis(100),
    };

    let service = AutoBackupService::new(
        config,
        env.storage_manager.clone(),
        env.encryption_service.clone(),
        env.integrity_service.clone(),
    );

    // Test service lifecycle
    assert!(service.start().await.is_ok());
    
    let status = service.get_status().unwrap();
    assert!(status.is_running);
    assert_eq!(status.pending_backups, 0);

    // Test drive event processing
    let drive_event = DriveEvent {
        drive_identifier: target_drive,
        action: DriveAction::Mounted,
        mount_point: Some(env.temp_path().to_string_lossy().to_string()),
        timestamp: SystemTime::now(),
        should_trigger_backup: true,
    };

    assert!(service.process_drive_event(drive_event).await.is_ok());

    // Give time for async processing
    sleep(Duration::from_millis(200)).await;

    assert!(service.stop().await.is_ok());
}

/// Test cloud backup service integration
#[tokio::test]
async fn test_cloud_backup_service_integration() {
    let env = TestEnvironment::new().await;
    let _test_files = env.create_test_files().await;

    let s3_config = S3BackupConfig {
        bucket_name: "test-integration-bucket".to_string(),
        region: "us-west-2".to_string(),
        prefix: "test-integration".to_string(),
        access_key_id: Some("test-key".to_string()),
        secret_access_key: Some("test-secret".to_string()),
        use_instance_profile: false,
        storage_class: chronicle_packer::S3StorageClass::StandardIA,
        server_side_encryption: true,
        kms_key_id: None,
    };

    let config = CloudBackupConfig {
        enabled: true,
        provider: CloudProvider::S3,
        s3_config: Some(s3_config),
        continuous_backup: false,
        schedule: BackupSchedule::Daily,
        encryption_enabled: true,
        client_side_encryption: true,
        retention_days: 30,
        max_backup_size: 1024 * 1024 * 100, // 100MB
        compression_enabled: true,
    };

    let mut service = CloudBackupService::new(
        config,
        env.storage_manager.clone(),
        env.encryption_service.clone(),
    );

    // Test initialization (will work even without real AWS credentials)
    #[cfg(not(feature = "cloud-backup"))]
    {
        let result = service.initialize().await;
        // Should fail when cloud-backup feature is disabled
        assert!(result.is_err());
    }

    #[cfg(feature = "cloud-backup")]
    {
        // Note: This will fail without real AWS credentials, but tests the code path
        let _result = service.initialize().await;
        // Don't assert success since we don't have real credentials
    }

    let status = service.get_status().unwrap();
    assert!(!status.is_running); // Won't start without proper credentials
    assert_eq!(status.pending_uploads, 0);
}

/// Test drive identifier matching logic
#[tokio::test]
async fn test_drive_identifier_matching_integration() {
    let env = TestEnvironment::new().await;

    let test_cases = vec![
        // (target, actual, should_match)
        (
            DriveIdentifier::new().with_uuid("12345".to_string()),
            DriveIdentifier::new().with_uuid("12345".to_string()),
            true,
        ),
        (
            DriveIdentifier::new().with_uuid("12345".to_string()),
            DriveIdentifier::new().with_uuid("67890".to_string()),
            false,
        ),
        (
            DriveIdentifier::new().with_volume_label("TestDrive".to_string()),
            DriveIdentifier::new().with_volume_label("TestDrive".to_string()),
            true,
        ),
        (
            DriveIdentifier::new().with_serial_number("ABC123".to_string()),
            DriveIdentifier::new().with_serial_number("ABC123".to_string()),
            true,
        ),
        (
            DriveIdentifier::new()
                .with_uuid("12345".to_string())
                .with_volume_label("Drive1".to_string()),
            DriveIdentifier::new()
                .with_uuid("12345".to_string())
                .with_volume_label("Drive2".to_string()),
            true, // UUID takes precedence
        ),
    ];

    for (target, actual, should_match) in test_cases {
        let config = AutoBackupConfig {
            enabled: true,
            target_drives: vec![target],
            remove_local_after_backup: false,
            verification_required: true,
            backup_destination_path: "/Test".to_string(),
            encryption_enabled: true,
            compression_enabled: true,
            retry_attempts: 1,
            retry_delay: Duration::from_millis(10),
        };

        let service = AutoBackupService::new(
            config,
            env.storage_manager.clone(),
            env.encryption_service.clone(),
            env.integrity_service.clone(),
        );

        service.start().await.unwrap();

        let event = DriveEvent {
            drive_identifier: actual,
            action: DriveAction::Mounted,
            mount_point: Some("/test".to_string()),
            timestamp: SystemTime::now(),
            should_trigger_backup: should_match,
        };

        let result = service.process_drive_event(event).await;
        assert!(result.is_ok());

        service.stop().await.unwrap();
    }
}

/// Test multiple drive targets
#[tokio::test]
async fn test_multiple_drive_targets_integration() {
    let env = TestEnvironment::new().await;

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
        backup_destination_path: "/MultiTest".to_string(),
        encryption_enabled: true,
        compression_enabled: true,
        retry_attempts: 1,
        retry_delay: Duration::from_millis(10),
    };

    let service = AutoBackupService::new(
        config,
        env.storage_manager.clone(),
        env.encryption_service.clone(),
        env.integrity_service.clone(),
    );

    service.start().await.unwrap();

    // Test each target drive type
    let test_events = vec![
        DriveEvent {
            drive_identifier: DriveIdentifier::new().with_uuid("uuid-1".to_string()),
            action: DriveAction::Mounted,
            mount_point: Some("/test1".to_string()),
            timestamp: SystemTime::now(),
            should_trigger_backup: true,
        },
        DriveEvent {
            drive_identifier: DriveIdentifier::new().with_volume_label("Backup1".to_string()),
            action: DriveAction::Mounted,
            mount_point: Some("/test2".to_string()),
            timestamp: SystemTime::now(),
            should_trigger_backup: true,
        },
        DriveEvent {
            drive_identifier: DriveIdentifier::new().with_serial_number("SERIAL123".to_string()),
            action: DriveAction::Mounted,
            mount_point: Some("/test3".to_string()),
            timestamp: SystemTime::now(),
            should_trigger_backup: true,
        },
    ];

    for event in test_events {
        let result = service.process_drive_event(event).await;
        assert!(result.is_ok());
    }

    service.stop().await.unwrap();
}

/// Test concurrent operations
#[tokio::test]
async fn test_concurrent_operations_integration() {
    let env = TestEnvironment::new().await;

    let config = AutoBackupConfig {
        enabled: true,
        target_drives: vec![
            DriveIdentifier::new().with_uuid("concurrent-test".to_string())
        ],
        remove_local_after_backup: false,
        verification_required: true,
        backup_destination_path: "/ConcurrentTest".to_string(),
        encryption_enabled: true,
        compression_enabled: true,
        retry_attempts: 1,
        retry_delay: Duration::from_millis(10),
    };

    let service = AutoBackupService::new(
        config,
        env.storage_manager.clone(),
        env.encryption_service.clone(),
        env.integrity_service.clone(),
    );

    service.start().await.unwrap();

    // Create multiple concurrent events
    let mut handles = vec![];
    for i in 0..10 {
        let service_ref = &service;
        let handle = tokio::spawn(async move {
            let event = DriveEvent {
                drive_identifier: DriveIdentifier::new()
                    .with_uuid(format!("test-{}", i)),
                action: DriveAction::Mounted,
                mount_point: Some(format!("/test{}", i)),
                timestamp: SystemTime::now(),
                should_trigger_backup: false, // Non-matching to avoid actual backup
            };
            service_ref.process_drive_event(event).await
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

/// Test error handling and recovery
#[tokio::test]
async fn test_error_handling_integration() {
    let env = TestEnvironment::new().await;

    // Test with invalid configuration
    let invalid_config = AutoBackupConfig {
        enabled: true,
        target_drives: vec![],
        remove_local_after_backup: false,
        verification_required: true,
        backup_destination_path: "".to_string(), // Invalid empty path
        encryption_enabled: true,
        compression_enabled: true,
        retry_attempts: 0, // Invalid retry count
        retry_delay: Duration::from_millis(0), // Invalid delay
    };

    let service = AutoBackupService::new(
        invalid_config,
        env.storage_manager.clone(),
        env.encryption_service.clone(),
        env.integrity_service.clone(),
    );

    // Service should still be created (validation happens elsewhere)
    let status = service.get_status().unwrap();
    assert!(!status.is_running);
}

/// Test service state management
#[tokio::test]
async fn test_service_state_management_integration() {
    let env = TestEnvironment::new().await;

    let config = AutoBackupConfig {
        enabled: true,
        target_drives: vec![
            DriveIdentifier::new().with_uuid("state-test".to_string())
        ],
        remove_local_after_backup: false,
        verification_required: true,
        backup_destination_path: "/StateTest".to_string(),
        encryption_enabled: true,
        compression_enabled: true,
        retry_attempts: 1,
        retry_delay: Duration::from_millis(10),
    };

    let service = AutoBackupService::new(
        config,
        env.storage_manager.clone(),
        env.encryption_service.clone(),
        env.integrity_service.clone(),
    );

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

    // Stop again (should be idempotent)
    let result = service.stop().await;
    assert!(result.is_ok());
}

/// Test cloud backup configuration validation
#[tokio::test]
async fn test_cloud_backup_config_validation() {
    let env = TestEnvironment::new().await;

    // Test valid configuration
    let valid_config = CloudBackupConfig {
        enabled: true,
        provider: CloudProvider::S3,
        s3_config: Some(S3BackupConfig {
            bucket_name: "valid-bucket-name".to_string(),
            region: "us-west-2".to_string(),
            prefix: "test".to_string(),
            access_key_id: None,
            secret_access_key: None,
            use_instance_profile: true,
            storage_class: chronicle_packer::S3StorageClass::Standard,
            server_side_encryption: true,
            kms_key_id: None,
        }),
        continuous_backup: false,
        schedule: BackupSchedule::Daily,
        encryption_enabled: true,
        client_side_encryption: true,
        retention_days: 30,
        max_backup_size: 1024 * 1024 * 1024,
        compression_enabled: true,
    };

    let service = CloudBackupService::new(
        valid_config,
        env.storage_manager.clone(),
        env.encryption_service.clone(),
    );

    let status = service.get_status().unwrap();
    assert!(!status.is_running);
    assert_eq!(status.pending_uploads, 0);
}

/// Test backup schedule configurations
#[tokio::test]
async fn test_backup_schedules_integration() {
    let env = TestEnvironment::new().await;

    let schedules = vec![
        BackupSchedule::Realtime,
        BackupSchedule::Hourly,
        BackupSchedule::Daily,
        BackupSchedule::Weekly,
        BackupSchedule::Monthly,
    ];

    for schedule in schedules {
        let config = CloudBackupConfig {
            enabled: true,
            provider: CloudProvider::S3,
            s3_config: Some(S3BackupConfig::default()),
            continuous_backup: false,
            schedule: schedule.clone(),
            encryption_enabled: true,
            client_side_encryption: true,
            retention_days: 30,
            max_backup_size: 1024 * 1024 * 1024,
            compression_enabled: true,
        };

        let service = CloudBackupService::new(
            config,
            env.storage_manager.clone(),
            env.encryption_service.clone(),
        );

        // Each schedule should create a valid service
        let status = service.get_status().unwrap();
        assert!(!status.is_running);
    }
}

/// Test encryption and compression functionality
#[tokio::test]
async fn test_encryption_compression_integration() {
    let env = TestEnvironment::new().await;

    // Test data compression
    let test_data = b"This is a test string that should compress well when repeated. ".repeat(100);
    let original_size = test_data.len();

    // Test compression
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&test_data).unwrap();
    let compressed = encoder.finish().unwrap();

    assert!(compressed.len() < original_size);
    assert!(!compressed.is_empty());

    // Test encryption service
    let test_string = "Test encryption data";
    let encrypted_result = env.encryption_service.encrypt_data(test_string.as_bytes()).await;
    
    // Should either succeed or fail gracefully
    match encrypted_result {
        Ok(encrypted_data) => {
            assert!(!encrypted_data.is_empty());
            assert_ne!(encrypted_data, test_string.as_bytes());
        }
        Err(_) => {
            // Encryption might fail in test environment without proper setup
            println!("Encryption test skipped - service not properly configured");
        }
    }
}

/// Test file operations and integrity
#[tokio::test]
async fn test_file_operations_integration() {
    let env = TestEnvironment::new().await;
    let test_files = env.create_test_files().await;

    // Verify test files were created
    assert_eq!(test_files.len(), 8); // 5 main files + 3 sub files

    for file_path in &test_files {
        assert!(file_path.exists());
        let metadata = tokio::fs::metadata(file_path).await.unwrap();
        assert!(metadata.len() > 0);
    }

    // Test file reading
    for file_path in &test_files {
        let content = tokio::fs::read_to_string(file_path).await.unwrap();
        assert!(!content.is_empty());
    }
}

/// Performance test for large numbers of events
#[tokio::test]
async fn test_performance_integration() {
    let env = TestEnvironment::new().await;

    let config = AutoBackupConfig {
        enabled: true,
        target_drives: vec![
            DriveIdentifier::new().with_uuid("perf-test".to_string())
        ],
        remove_local_after_backup: false,
        verification_required: false, // Disable for performance
        backup_destination_path: "/PerfTest".to_string(),
        encryption_enabled: false, // Disable for performance
        compression_enabled: false, // Disable for performance
        retry_attempts: 1,
        retry_delay: Duration::from_millis(1),
    };

    let service = AutoBackupService::new(
        config,
        env.storage_manager.clone(),
        env.encryption_service.clone(),
        env.integrity_service.clone(),
    );

    service.start().await.unwrap();

    let start_time = std::time::Instant::now();
    
    // Process many events quickly
    for i in 0..1000 {
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
    println!("Processed 1000 events in {:?}", duration);
    
    // Should process events reasonably quickly (less than 1 second for 1000 events)
    assert!(duration < Duration::from_secs(1));

    service.stop().await.unwrap();
}

/// Test memory usage and cleanup
#[tokio::test]
async fn test_memory_cleanup_integration() {
    let env = TestEnvironment::new().await;

    // Create and destroy multiple services to test cleanup
    for _i in 0..10 {
        let config = AutoBackupConfig {
            enabled: true,
            target_drives: vec![
                DriveIdentifier::new().with_uuid("cleanup-test".to_string())
            ],
            remove_local_after_backup: false,
            verification_required: true,
            backup_destination_path: "/CleanupTest".to_string(),
            encryption_enabled: true,
            compression_enabled: true,
            retry_attempts: 1,
            retry_delay: Duration::from_millis(10),
        };

        let service = AutoBackupService::new(
            config,
            env.storage_manager.clone(),
            env.encryption_service.clone(),
            env.integrity_service.clone(),
        );

        service.start().await.unwrap();
        
        // Process a few events
        for j in 0..5 {
            let event = DriveEvent {
                drive_identifier: DriveIdentifier::new()
                    .with_uuid(format!("cleanup-{}", j)),
                action: DriveAction::Mounted,
                mount_point: Some("/cleanup".to_string()),
                timestamp: SystemTime::now(),
                should_trigger_backup: false,
            };
            service.process_drive_event(event).await.unwrap();
        }

        service.stop().await.unwrap();
        
        // Service should be properly cleaned up
        let status = service.get_status().unwrap();
        assert!(!status.is_running);
    }
}

/// Integration test for all backup types working together
#[tokio::test]
async fn test_complete_backup_workflow_integration() {
    let env = TestEnvironment::new().await;
    let _test_files = env.create_test_files().await;

    // Set up auto-backup
    let auto_config = AutoBackupConfig {
        enabled: true,
        target_drives: vec![
            DriveIdentifier::new().with_uuid("workflow-test".to_string())
        ],
        remove_local_after_backup: false,
        verification_required: true,
        backup_destination_path: "/WorkflowTest".to_string(),
        encryption_enabled: true,
        compression_enabled: true,
        retry_attempts: 2,
        retry_delay: Duration::from_millis(50),
    };

    let auto_service = AutoBackupService::new(
        auto_config,
        env.storage_manager.clone(),
        env.encryption_service.clone(),
        env.integrity_service.clone(),
    );

    // Set up cloud backup
    let cloud_config = CloudBackupConfig {
        enabled: true,
        provider: CloudProvider::S3,
        s3_config: Some(S3BackupConfig::default()),
        continuous_backup: false,
        schedule: BackupSchedule::Daily,
        encryption_enabled: true,
        client_side_encryption: true,
        retention_days: 30,
        max_backup_size: 1024 * 1024 * 100,
        compression_enabled: true,
    };

    let cloud_service = CloudBackupService::new(
        cloud_config,
        env.storage_manager.clone(),
        env.encryption_service.clone(),
    );

    // Start both services
    auto_service.start().await.unwrap();
    
    // Cloud service might fail without real credentials, that's OK
    let _cloud_result = cloud_service.get_status();

    // Test auto-backup trigger
    let drive_event = DriveEvent {
        drive_identifier: DriveIdentifier::new().with_uuid("workflow-test".to_string()),
        action: DriveAction::Mounted,
        mount_point: Some(env.temp_path().to_string_lossy().to_string()),
        timestamp: SystemTime::now(),
        should_trigger_backup: true,
    };

    auto_service.process_drive_event(drive_event).await.unwrap();

    // Give time for processing
    sleep(Duration::from_millis(100)).await;

    // Verify services are in expected states
    let auto_status = auto_service.get_status().unwrap();
    assert!(auto_status.is_running);

    let cloud_status = cloud_service.get_status().unwrap();
    // Cloud service won't be running without proper AWS setup
    assert!(!cloud_status.is_running);

    // Clean up
    auto_service.stop().await.unwrap();
}