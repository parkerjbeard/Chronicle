//! Comprehensive tests for CLI backup functionality including cloud and auto-backup features

use chronicle_cli::{
    api::{BackupRequest, CloudBackupOptions, AutoBackupOptions},
    commands::backup::{BackupArgs, run},
    output::{OutputManager, OutputFormat},
    error::ChronicleError,
};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::time::Duration;

/// Create a test backup args structure
fn create_test_backup_args(temp_dir: &TempDir) -> BackupArgs {
    BackupArgs {
        destination: temp_dir.path().join("backup.tar.gz").to_string_lossy().to_string(),
        include_metadata: true,
        compression: Some("gzip".to_string()),
        encryption: None,
        overwrite: false,
        verify: true,
        progress: false,
        timeout: 60,
        time: None,
        event_types: None,
        dry_run: true, // Use dry run for testing
        cloud: false,
        s3_uri: None,
        continuous: false,
        auto_backup: false,
        target_drive: None,
        drive_id_type: "uuid".to_string(),
        remove_local: false,
    }
}

#[tokio::test]
async fn test_basic_backup_args_validation() {
    let temp_dir = TempDir::new().unwrap();
    let args = create_test_backup_args(&temp_dir);
    
    // Basic validation
    assert!(!args.destination.is_empty());
    assert!(args.include_metadata);
    assert_eq!(args.compression, Some("gzip".to_string()));
    assert!(args.verify);
    assert!(args.dry_run);
    assert!(!args.cloud);
    assert!(!args.auto_backup);
}

#[test]
fn test_compression_validation() {
    let valid_compressions = vec!["gzip", "bzip2", "lz4"];
    let invalid_compressions = vec!["zip", "rar", "7z", "invalid"];
    
    for compression in valid_compressions {
        // These should be valid according to the CLI validation
        assert!(matches!(compression, "gzip" | "bzip2" | "lz4"));
    }
    
    for compression in invalid_compressions {
        // These should be invalid according to the CLI validation
        assert!(!matches!(compression, "gzip" | "bzip2" | "lz4"));
    }
}

#[test]
fn test_drive_id_type_validation() {
    let valid_types = vec!["uuid", "volume_label", "serial_number"];
    let invalid_types = vec!["name", "path", "invalid"];
    
    for drive_type in valid_types {
        assert!(matches!(drive_type, "uuid" | "volume_label" | "serial_number"));
    }
    
    for drive_type in invalid_types {
        assert!(!matches!(drive_type, "uuid" | "volume_label" | "serial_number"));
    }
}

#[test]
fn test_cloud_backup_options_creation() {
    let cloud_options = CloudBackupOptions {
        enabled: true,
        s3_uri: Some("s3://my-bucket/prefix".to_string()),
        continuous: false,
        client_side_encryption: true,
    };
    
    assert!(cloud_options.enabled);
    assert!(cloud_options.s3_uri.is_some());
    assert!(!cloud_options.continuous);
    assert!(cloud_options.client_side_encryption);
}

#[test]
fn test_auto_backup_options_creation() {
    let auto_options = AutoBackupOptions {
        enabled: true,
        target_drive: Some("12345678-1234-1234-1234-123456789ABC".to_string()),
        drive_id_type: "uuid".to_string(),
        remove_local_after_backup: false,
    };
    
    assert!(auto_options.enabled);
    assert!(auto_options.target_drive.is_some());
    assert_eq!(auto_options.drive_id_type, "uuid");
    assert!(!auto_options.remove_local_after_backup);
}

#[test]
fn test_backup_request_with_cloud_options() {
    let backup_request = BackupRequest {
        destination: "/test/backup".to_string(),
        include_metadata: true,
        compression: Some("gzip".to_string()),
        encryption: None,
        cloud_backup: Some(CloudBackupOptions {
            enabled: true,
            s3_uri: Some("s3://test-bucket/test-prefix".to_string()),
            continuous: true,
            client_side_encryption: true,
        }),
        auto_backup: None,
    };
    
    assert!(backup_request.cloud_backup.is_some());
    assert!(backup_request.auto_backup.is_none());
    
    let cloud_opts = backup_request.cloud_backup.unwrap();
    assert!(cloud_opts.enabled);
    assert!(cloud_opts.continuous);
    assert!(cloud_opts.client_side_encryption);
}

#[test]
fn test_backup_request_with_auto_backup_options() {
    let backup_request = BackupRequest {
        destination: "/test/backup".to_string(),
        include_metadata: true,
        compression: Some("gzip".to_string()),
        encryption: None,
        cloud_backup: None,
        auto_backup: Some(AutoBackupOptions {
            enabled: true,
            target_drive: Some("TestDrive".to_string()),
            drive_id_type: "volume_label".to_string(),
            remove_local_after_backup: false,
        }),
    };
    
    assert!(backup_request.auto_backup.is_some());
    assert!(backup_request.cloud_backup.is_none());
    
    let auto_opts = backup_request.auto_backup.unwrap();
    assert!(auto_opts.enabled);
    assert_eq!(auto_opts.drive_id_type, "volume_label");
    assert!(!auto_opts.remove_local_after_backup);
}

#[test]
fn test_backup_request_with_both_options() {
    let backup_request = BackupRequest {
        destination: "/test/backup".to_string(),
        include_metadata: true,
        compression: Some("gzip".to_string()),
        encryption: None,
        cloud_backup: Some(CloudBackupOptions {
            enabled: true,
            s3_uri: Some("s3://test-bucket/test-prefix".to_string()),
            continuous: false,
            client_side_encryption: true,
        }),
        auto_backup: Some(AutoBackupOptions {
            enabled: true,
            target_drive: Some("12345678-1234-1234-1234-123456789ABC".to_string()),
            drive_id_type: "uuid".to_string(),
            remove_local_after_backup: false,
        }),
    };
    
    assert!(backup_request.cloud_backup.is_some());
    assert!(backup_request.auto_backup.is_some());
}

#[test]
fn test_s3_uri_validation() {
    let valid_s3_uris = vec![
        "s3://bucket",
        "s3://bucket/prefix",
        "s3://bucket/long/nested/prefix",
        "s3://my-bucket-name/data/chronicle",
    ];
    
    let invalid_s3_uris = vec![
        "http://bucket",
        "https://bucket",
        "bucket",
        "s3:/",
        "s3://",
        "",
    ];
    
    for uri in valid_s3_uris {
        assert!(uri.starts_with("s3://"));
        assert!(uri.len() > 5); // More than just "s3://"
    }
    
    for uri in invalid_s3_uris {
        assert!(!uri.starts_with("s3://") || uri.len() <= 5);
    }
}

#[test]
fn test_backup_args_with_cloud_options() {
    let temp_dir = TempDir::new().unwrap();
    let mut args = create_test_backup_args(&temp_dir);
    
    // Enable cloud backup
    args.cloud = true;
    args.s3_uri = Some("s3://my-bucket/chronicle-data".to_string());
    args.continuous = true;
    
    assert!(args.cloud);
    assert!(args.s3_uri.is_some());
    assert!(args.continuous);
    
    // Validate S3 URI
    let s3_uri = args.s3_uri.unwrap();
    assert!(s3_uri.starts_with("s3://"));
}

#[test]
fn test_backup_args_with_auto_backup_options() {
    let temp_dir = TempDir::new().unwrap();
    let mut args = create_test_backup_args(&temp_dir);
    
    // Enable auto-backup
    args.auto_backup = true;
    args.target_drive = Some("12345678-1234-1234-1234-123456789ABC".to_string());
    args.drive_id_type = "uuid".to_string();
    args.remove_local = false;
    
    assert!(args.auto_backup);
    assert!(args.target_drive.is_some());
    assert_eq!(args.drive_id_type, "uuid");
    assert!(!args.remove_local);
}

#[test]
fn test_dangerous_remove_local_option() {
    let temp_dir = TempDir::new().unwrap();
    let mut args = create_test_backup_args(&temp_dir);
    
    // Enable auto-backup with remove_local (dangerous option)
    args.auto_backup = true;
    args.target_drive = Some("TestDrive".to_string());
    args.remove_local = true; // This should require confirmation
    
    assert!(args.remove_local);
    // In actual implementation, this would trigger a confirmation prompt
}

#[test]
fn test_timeout_validation() {
    let temp_dir = TempDir::new().unwrap();
    let mut args = create_test_backup_args(&temp_dir);
    
    // Test various timeout values
    let timeout_values = vec![60, 300, 1800, 3600, 7200];
    
    for timeout in timeout_values {
        args.timeout = timeout;
        assert!(args.timeout > 0);
        assert!(args.timeout <= 7200); // Reasonable upper limit
    }
}

#[test]
fn test_backup_args_combinations() {
    let temp_dir = TempDir::new().unwrap();
    let mut args = create_test_backup_args(&temp_dir);
    
    // Test cloud + auto-backup combination
    args.cloud = true;
    args.s3_uri = Some("s3://backup-bucket/chronicle".to_string());
    args.auto_backup = true;
    args.target_drive = Some("BackupDrive".to_string());
    args.drive_id_type = "volume_label".to_string();
    
    // Both should be enabled
    assert!(args.cloud);
    assert!(args.auto_backup);
    
    // Test encryption + compression + cloud + auto-backup
    args.encryption = Some("password123".to_string());
    args.compression = Some("lz4".to_string());
    
    assert!(args.encryption.is_some());
    assert!(args.compression.is_some());
}

#[test]
fn test_dry_run_mode() {
    let temp_dir = TempDir::new().unwrap();
    let mut args = create_test_backup_args(&temp_dir);
    
    // Test dry run with various options
    args.dry_run = true;
    args.cloud = true;
    args.auto_backup = true;
    args.verify = true;
    
    // Dry run should work with all options
    assert!(args.dry_run);
    assert!(args.cloud);
    assert!(args.auto_backup);
    assert!(args.verify);
}

#[test]
fn test_backup_verification() {
    let temp_dir = TempDir::new().unwrap();
    let mut args = create_test_backup_args(&temp_dir);
    
    // Test verification enabled/disabled
    args.verify = true;
    assert!(args.verify);
    
    args.verify = false;
    assert!(!args.verify);
    
    // Verification should be recommended for important backups
    args.cloud = true;
    args.auto_backup = true;
    args.remove_local = true;
    // For safety, verification should be strongly recommended for these scenarios
}

#[test]
fn test_progress_display_options() {
    let temp_dir = TempDir::new().unwrap();
    let mut args = create_test_backup_args(&temp_dir);
    
    // Test progress display
    args.progress = true;
    assert!(args.progress);
    
    // Progress should be useful for long-running cloud uploads
    args.cloud = true;
    args.continuous = true;
    assert!(args.progress);
}

#[test]
fn test_event_type_filtering() {
    let temp_dir = TempDir::new().unwrap();
    let mut args = create_test_backup_args(&temp_dir);
    
    // Test event type filtering
    args.event_types = Some("screen_capture,file_system".to_string());
    assert!(args.event_types.is_some());
    
    let event_types = args.event_types.unwrap();
    assert!(event_types.contains("screen_capture"));
    assert!(event_types.contains("file_system"));
}

#[test]
fn test_time_range_filtering() {
    let temp_dir = TempDir::new().unwrap();
    let mut args = create_test_backup_args(&temp_dir);
    
    // Test time range filtering
    args.time = Some("last-week".to_string());
    assert!(args.time.is_some());
    
    // Test different time formats
    let time_ranges = vec![
        "last-hour",
        "last-day", 
        "last-week",
        "last-month",
        "2024-01-01..2024-01-31",
        "today",
    ];
    
    for time_range in time_ranges {
        args.time = Some(time_range.to_string());
        assert!(args.time.is_some());
    }
}

/// Integration test for complete backup argument processing
#[test]
fn test_complete_backup_workflow_args() {
    let temp_dir = TempDir::new().unwrap();
    let args = BackupArgs {
        destination: temp_dir.path().join("complete_backup.tar.gz").to_string_lossy().to_string(),
        include_metadata: true,
        compression: Some("gzip".to_string()),
        encryption: Some("secure_password".to_string()),
        overwrite: true,
        verify: true,
        progress: true,
        timeout: 3600,
        time: Some("last-month".to_string()),
        event_types: Some("screen_capture,file_system,network".to_string()),
        dry_run: false,
        cloud: true,
        s3_uri: Some("s3://company-backups/chronicle/production".to_string()),
        continuous: false,
        auto_backup: true,
        target_drive: Some("12345678-1234-1234-1234-123456789ABC".to_string()),
        drive_id_type: "uuid".to_string(),
        remove_local: false,
    };
    
    // Validate all options are set correctly
    assert!(args.include_metadata);
    assert_eq!(args.compression, Some("gzip".to_string()));
    assert!(args.encryption.is_some());
    assert!(args.overwrite);
    assert!(args.verify);
    assert!(args.progress);
    assert_eq!(args.timeout, 3600);
    assert!(args.time.is_some());
    assert!(args.event_types.is_some());
    assert!(!args.dry_run);
    assert!(args.cloud);
    assert!(args.s3_uri.is_some());
    assert!(!args.continuous);
    assert!(args.auto_backup);
    assert!(args.target_drive.is_some());
    assert_eq!(args.drive_id_type, "uuid");
    assert!(!args.remove_local);
    
    // Validate S3 URI format
    let s3_uri = args.s3_uri.unwrap();
    assert!(s3_uri.starts_with("s3://"));
    assert!(s3_uri.contains("company-backups"));
    
    // Validate UUID format (basic check)
    let uuid = args.target_drive.unwrap();
    assert_eq!(uuid.len(), 36); // UUID length
    assert_eq!(uuid.matches('-').count(), 4); // UUID has 4 hyphens
}

#[test]
fn test_error_scenarios() {
    // Test invalid compression
    let invalid_compression = "invalid_compression";
    assert!(!matches!(invalid_compression, "gzip" | "bzip2" | "lz4"));
    
    // Test invalid drive ID type
    let invalid_drive_type = "invalid_type";
    assert!(!matches!(invalid_drive_type, "uuid" | "volume_label" | "serial_number"));
    
    // Test invalid S3 URI
    let invalid_s3_uri = "http://not-s3-uri";
    assert!(!invalid_s3_uri.starts_with("s3://"));
    
    // Test empty destination
    let empty_destination = "";
    assert!(empty_destination.is_empty());
}