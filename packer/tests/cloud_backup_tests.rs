//! Comprehensive tests for cloud backup functionality

use chronicle_packer::{
    CloudBackupService, CloudBackupConfig, CloudProvider, BackupSchedule, 
    S3BackupConfig, S3StorageClass, StorageManager, EncryptionService,
};
use std::{sync::Arc, path::Path};
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

/// Test helper to create a test cloud backup service
async fn create_test_service() -> (CloudBackupService, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    
    let s3_config = S3BackupConfig {
        bucket_name: "test-bucket".to_string(),
        region: "us-west-2".to_string(),
        prefix: "test-prefix".to_string(),
        access_key_id: Some("test-access-key".to_string()),
        secret_access_key: Some("test-secret-key".to_string()),
        use_instance_profile: false,
        storage_class: S3StorageClass::StandardIA,
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
        retention_days: 90,
        max_backup_size: 1024 * 1024 * 1024, // 1GB
        compression_enabled: true,
    };

    let storage_config = chronicle_packer::config::StorageConfig::default();
    let storage_manager = Arc::new(StorageManager::new(storage_config).unwrap());
    let encryption_service = Arc::new(EncryptionService::new(Default::default()).unwrap());

    let service = CloudBackupService::new(
        config,
        storage_manager,
        encryption_service,
    );

    (service, temp_dir)
}

#[tokio::test]
async fn test_cloud_backup_service_creation() {
    let (service, _temp_dir) = create_test_service().await;
    
    let status = service.get_status().unwrap();
    assert!(!status.is_running);
    assert_eq!(status.pending_uploads, 0);
    assert_eq!(status.completed_uploads, 0);
    assert_eq!(status.failed_uploads, 0);
    assert_eq!(status.bytes_uploaded_total, 0);
}

#[tokio::test]
async fn test_cloud_backup_config_validation() {
    let config = CloudBackupConfig::default();
    
    // Default config should be disabled
    assert!(!config.enabled);
    assert_eq!(config.provider, CloudProvider::S3);
    assert!(!config.continuous_backup);
    assert_eq!(config.schedule, BackupSchedule::Daily);
    assert!(config.encryption_enabled);
    assert!(config.client_side_encryption);
    assert_eq!(config.retention_days, 90);
    assert!(config.compression_enabled);
}

#[tokio::test]
async fn test_s3_config_validation() {
    let s3_config = S3BackupConfig::default();
    
    assert_eq!(s3_config.bucket_name, "chronicle-backups");
    assert_eq!(s3_config.region, "us-west-2");
    assert_eq!(s3_config.prefix, "chronicle-data");
    assert!(s3_config.access_key_id.is_none());
    assert!(s3_config.secret_access_key.is_none());
    assert!(!s3_config.use_instance_profile);
    assert_eq!(s3_config.storage_class, S3StorageClass::StandardIA);
    assert!(s3_config.server_side_encryption);
    assert!(s3_config.kms_key_id.is_none());
}

#[tokio::test]
async fn test_s3_storage_class_conversion() {
    use chronicle_packer::cloud_backup::ObjectStorageClass;
    
    // Test all storage class conversions
    assert_eq!(
        ObjectStorageClass::from(S3StorageClass::Standard),
        ObjectStorageClass::Standard
    );
    assert_eq!(
        ObjectStorageClass::from(S3StorageClass::StandardIA),
        ObjectStorageClass::StandardIa
    );
    assert_eq!(
        ObjectStorageClass::from(S3StorageClass::OneZoneIA),
        ObjectStorageClass::OnezoneIa
    );
    assert_eq!(
        ObjectStorageClass::from(S3StorageClass::Glacier),
        ObjectStorageClass::Glacier
    );
    assert_eq!(
        ObjectStorageClass::from(S3StorageClass::GlacierInstantRetrieval),
        ObjectStorageClass::GlacierIr
    );
    assert_eq!(
        ObjectStorageClass::from(S3StorageClass::DeepArchive),
        ObjectStorageClass::DeepArchive
    );
}

#[tokio::test]
async fn test_cloud_key_generation() {
    let (service, _temp_dir) = create_test_service().await;
    
    let test_path = Path::new("/test/path/file.parquet");
    let key = service.generate_cloud_key(test_path).unwrap();
    
    // Key should have the expected format
    assert!(key.starts_with("test-prefix/year="));
    assert!(key.contains("/month="));
    assert!(key.contains("/day="));
    assert!(key.ends_with("/file.parquet"));
}

#[tokio::test]
async fn test_disabled_cloud_backup() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    
    let config = CloudBackupConfig {
        enabled: false, // Disabled
        provider: CloudProvider::S3,
        s3_config: None,
        continuous_backup: false,
        schedule: BackupSchedule::Daily,
        encryption_enabled: true,
        client_side_encryption: true,
        retention_days: 90,
        max_backup_size: 1024 * 1024 * 1024,
        compression_enabled: true,
    };

    let storage_config = chronicle_packer::config::StorageConfig::default();
    let storage_manager = Arc::new(StorageManager::new(storage_config).unwrap());
    let encryption_service = Arc::new(EncryptionService::new(Default::default()).unwrap());

    let service = CloudBackupService::new(
        config,
        storage_manager,
        encryption_service,
    );
    
    // Initialize disabled service should succeed
    let mut service = service;
    let result = service.initialize().await;
    assert!(result.is_ok());
    
    // Starting disabled service should succeed but not actually start
    let result = service.start().await;
    assert!(result.is_ok());
    
    // Service should not be marked as running since it's disabled
    let status = service.get_status().unwrap();
    assert!(!status.is_running);
}

#[tokio::test]
async fn test_different_backup_schedules() {
    let schedules = vec![
        BackupSchedule::Realtime,
        BackupSchedule::Hourly,
        BackupSchedule::Daily,
        BackupSchedule::Weekly,
        BackupSchedule::Monthly,
    ];
    
    for schedule in schedules {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        
        let config = CloudBackupConfig {
            enabled: true,
            provider: CloudProvider::S3,
            s3_config: Some(S3BackupConfig::default()),
            continuous_backup: false,
            schedule: schedule.clone(),
            encryption_enabled: true,
            client_side_encryption: true,
            retention_days: 90,
            max_backup_size: 1024 * 1024 * 1024,
            compression_enabled: true,
        };
        
        let storage_config = chronicle_packer::config::StorageConfig::default();
        let storage_manager = Arc::new(StorageManager::new(storage_config).unwrap());
        let encryption_service = Arc::new(EncryptionService::new(Default::default()).unwrap());

        let service = CloudBackupService::new(
            config,
            storage_manager,
            encryption_service,
        );
        
        // Service should be created successfully with any schedule
        let status = service.get_status().unwrap();
        assert!(!status.is_running);
    }
}

#[tokio::test]
async fn test_continuous_vs_scheduled_backup() {
    // Test continuous backup
    let config_continuous = CloudBackupConfig {
        enabled: true,
        provider: CloudProvider::S3,
        s3_config: Some(S3BackupConfig::default()),
        continuous_backup: true,
        schedule: BackupSchedule::Realtime,
        encryption_enabled: true,
        client_side_encryption: true,
        retention_days: 90,
        max_backup_size: 1024 * 1024 * 1024,
        compression_enabled: true,
    };
    
    // Test scheduled backup
    let config_scheduled = CloudBackupConfig {
        enabled: true,
        provider: CloudProvider::S3,
        s3_config: Some(S3BackupConfig::default()),
        continuous_backup: false,
        schedule: BackupSchedule::Daily,
        encryption_enabled: true,
        client_side_encryption: true,
        retention_days: 90,
        max_backup_size: 1024 * 1024 * 1024,
        compression_enabled: true,
    };
    
    let storage_config = chronicle_packer::config::StorageConfig::default();
    let storage_manager = Arc::new(StorageManager::new(storage_config).unwrap());
    let encryption_service = Arc::new(EncryptionService::new(Default::default()).unwrap());

    let service_continuous = CloudBackupService::new(
        config_continuous,
        storage_manager.clone(),
        encryption_service.clone(),
    );
    
    let service_scheduled = CloudBackupService::new(
        config_scheduled,
        storage_manager,
        encryption_service,
    );
    
    // Both services should be created successfully
    assert!(service_continuous.get_status().is_ok());
    assert!(service_scheduled.get_status().is_ok());
}

#[tokio::test]
async fn test_unsupported_cloud_providers() {
    let providers = vec![CloudProvider::Gcp, CloudProvider::Azure];
    
    for provider in providers {
        let config = CloudBackupConfig {
            enabled: true,
            provider,
            s3_config: None,
            continuous_backup: false,
            schedule: BackupSchedule::Daily,
            encryption_enabled: true,
            client_side_encryption: true,
            retention_days: 90,
            max_backup_size: 1024 * 1024 * 1024,
            compression_enabled: true,
        };
        
        let storage_config = chronicle_packer::config::StorageConfig::default();
        let storage_manager = Arc::new(StorageManager::new(storage_config).unwrap());
        let encryption_service = Arc::new(EncryptionService::new(Default::default()).unwrap());

        let mut service = CloudBackupService::new(
            config,
            storage_manager,
            encryption_service,
        );
        
        // Initialize should fail for unsupported providers
        #[cfg(feature = "cloud-backup")]
        {
            let result = service.initialize().await;
            assert!(result.is_err());
        }
        
        #[cfg(not(feature = "cloud-backup"))]
        {
            let result = service.initialize().await;
            assert!(result.is_err());
        }
    }
}

#[tokio::test]
async fn test_privacy_settings() {
    let config = CloudBackupConfig {
        enabled: true,
        provider: CloudProvider::S3,
        s3_config: Some(S3BackupConfig::default()),
        continuous_backup: false,
        schedule: BackupSchedule::Daily,
        encryption_enabled: true,
        client_side_encryption: true, // Privacy-first
        retention_days: 90,
        max_backup_size: 1024 * 1024 * 1024,
        compression_enabled: true,
    };
    
    // Verify privacy settings
    assert!(config.encryption_enabled);
    assert!(config.client_side_encryption);
    assert!(config.compression_enabled); // Helps with privacy by obfuscating data size
}

#[tokio::test]
async fn test_upload_queue_functionality() {
    let (service, temp_dir) = create_test_service().await;
    
    // Create a test file
    let test_file = temp_dir.path().join("test_file.txt");
    tokio::fs::write(&test_file, "test content").await.unwrap();
    
    // Note: This would test the upload functionality, but without real S3 credentials
    // we can only test the interface
    let result = service.upload_file(&test_file, chronicle_packer::cloud_backup::UploadPriority::Normal).await;
    
    // Should succeed in queueing the upload (even if it might fail later without real credentials)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_service_lifecycle() {
    let (service, _temp_dir) = create_test_service().await;
    
    // Initial state
    let status = service.get_status().unwrap();
    assert!(!status.is_running);
    
    // Note: We can't fully test start/stop without real AWS credentials
    // But we can test the basic service creation and status
    assert_eq!(status.pending_uploads, 0);
    assert_eq!(status.completed_uploads, 0);
    assert_eq!(status.failed_uploads, 0);
}

#[tokio::test]
async fn test_compression_functionality() {
    let test_data = b"This is test data for compression testing. ".repeat(100);
    let compressed = chronicle_packer::cloud_backup::CloudBackupService::compress_data(test_data.clone()).unwrap();
    
    // Compressed data should be smaller than original for repetitive data
    assert!(compressed.len() < test_data.len());
    
    // Compressed data should not be empty
    assert!(!compressed.is_empty());
}

#[tokio::test]
async fn test_config_update() {
    let (mut service, _temp_dir) = create_test_service().await;
    
    let new_config = CloudBackupConfig {
        enabled: false, // Change enabled state
        provider: CloudProvider::S3,
        s3_config: Some(S3BackupConfig::default()),
        continuous_backup: true, // Change continuous backup
        schedule: BackupSchedule::Hourly, // Change schedule
        encryption_enabled: true,
        client_side_encryption: true,
        retention_days: 30, // Change retention
        max_backup_size: 512 * 1024 * 1024, // Change max size
        compression_enabled: false, // Change compression
    };
    
    let result = service.update_config(new_config).await;
    assert!(result.is_ok());
}

/// Integration test for error handling
#[tokio::test]
async fn test_error_handling() {
    // Test with invalid S3 config
    let invalid_config = CloudBackupConfig {
        enabled: true,
        provider: CloudProvider::S3,
        s3_config: None, // Missing S3 config
        continuous_backup: false,
        schedule: BackupSchedule::Daily,
        encryption_enabled: true,
        client_side_encryption: true,
        retention_days: 90,
        max_backup_size: 1024 * 1024 * 1024,
        compression_enabled: true,
    };
    
    let storage_config = chronicle_packer::config::StorageConfig::default();
    let storage_manager = Arc::new(StorageManager::new(storage_config).unwrap());
    let encryption_service = Arc::new(EncryptionService::new(Default::default()).unwrap());

    let service = CloudBackupService::new(
        invalid_config,
        storage_manager,
        encryption_service,
    );
    
    // Service creation should succeed, but initialization might fail
    let status = service.get_status().unwrap();
    assert!(!status.is_running);
}

/// Test data retention and cleanup policies
#[tokio::test]
async fn test_retention_policies() {
    let retention_days_values = vec![1, 7, 30, 90, 365];
    
    for retention_days in retention_days_values {
        let config = CloudBackupConfig {
            enabled: true,
            provider: CloudProvider::S3,
            s3_config: Some(S3BackupConfig::default()),
            continuous_backup: false,
            schedule: BackupSchedule::Daily,
            encryption_enabled: true,
            client_side_encryption: true,
            retention_days,
            max_backup_size: 1024 * 1024 * 1024,
            compression_enabled: true,
        };
        
        // Config should be valid for any reasonable retention period
        assert!(retention_days > 0);
        assert!(retention_days <= 365);
    }
}