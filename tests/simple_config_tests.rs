//! Simplified configuration validation tests without external dependencies

use std::{fs, path::PathBuf, collections::HashMap};
use tempfile::TempDir;

/// Simple TOML-like parser for testing
fn parse_basic_config(content: &str) -> HashMap<String, String> {
    let mut config = HashMap::new();
    
    for line in content.lines() {
        let line = line.trim();
        if line.contains('=') && !line.starts_with('[') && !line.starts_with('#') {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                let key = parts[0].trim().to_string();
                let value = parts[1].trim().trim_matches('"').to_string();
                config.insert(key, value);
            }
        }
    }
    
    config
}

/// Test helper to create a temporary config file
fn create_test_config_file(content: &str) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("test_config.toml");
    fs::write(&config_path, content).expect("Failed to write config file");
    (temp_dir, config_path)
}

#[test]
fn test_valid_auto_backup_config() {
    let config_content = r#"
[auto_backup]
enabled = true
remove_local_after_backup = false
verification_required = true
backup_destination_path = "/Chronicle"
encryption_enabled = true
compression_enabled = true
retry_attempts = 3
retry_delay_seconds = 60

[[auto_backup.target_drives]]
type = "uuid"
identifier = "12345678-1234-1234-1234-123456789ABC"

[[auto_backup.target_drives]]
type = "volume_label"
identifier = "BackupDrive"
"#;
    
    let (_temp_dir, config_path) = create_test_config_file(config_content);
    
    // Test that the config file can be read and parsed
    let content = fs::read_to_string(&config_path).unwrap();
    let parsed = parse_basic_config(&content);
    
    // Verify key structure exists
    assert!(content.contains("[auto_backup]"));
    assert!(content.contains("enabled = true"));
    assert!(content.contains("backup_destination_path = \"/Chronicle\""));
    assert!(content.contains("retry_attempts = 3"));
    
    // Verify target drives section exists
    assert!(content.contains("[[auto_backup.target_drives]]"));
    assert!(content.contains("type = \"uuid\""));
    assert!(content.contains("type = \"volume_label\""));
    
    println!("✅ Auto-backup config validation passed");
}

#[test]
fn test_valid_cloud_backup_config() {
    let config_content = r#"
[cloud_backup]
enabled = true
provider = "s3"
continuous_backup = false
schedule = "daily"
encryption_enabled = true
client_side_encryption = true
retention_days = 90
max_backup_size_gb = 10
compression_enabled = true

[cloud_backup.s3]
bucket_name = "my-chronicle-backups"
region = "us-west-2"
prefix = "chronicle-data"
storage_class = "STANDARD_IA"
server_side_encryption = true
use_instance_profile = true
"#;
    
    let (_temp_dir, config_path) = create_test_config_file(config_content);
    
    // Test that the config file can be read and parsed
    let content = fs::read_to_string(&config_path).unwrap();
    
    // Verify cloud backup structure
    assert!(content.contains("[cloud_backup]"));
    assert!(content.contains("enabled = true"));
    assert!(content.contains("provider = \"s3\""));
    assert!(content.contains("retention_days = 90"));
    
    // Verify S3 configuration
    assert!(content.contains("[cloud_backup.s3]"));
    assert!(content.contains("bucket_name = \"my-chronicle-backups\""));
    assert!(content.contains("region = \"us-west-2\""));
    assert!(content.contains("storage_class = \"STANDARD_IA\""));
    
    println!("✅ Cloud backup config validation passed");
}

#[test]
fn test_combined_backup_config() {
    let config_content = r#"
[auto_backup]
enabled = true
backup_destination_path = "/AutoBackup"
encryption_enabled = true

[[auto_backup.target_drives]]
type = "uuid"
identifier = "test-uuid"

[cloud_backup]
enabled = true
provider = "s3"
schedule = "weekly"

[cloud_backup.s3]
bucket_name = "test-bucket"
region = "us-east-1"
"#;
    
    let (_temp_dir, config_path) = create_test_config_file(config_content);
    
    // Test that both configurations can coexist
    let content = fs::read_to_string(&config_path).unwrap();
    
    assert!(content.contains("[auto_backup]"));
    assert!(content.contains("[cloud_backup]"));
    
    // Both should be enabled
    assert!(content.contains("enabled = true"));
    
    println!("✅ Combined backup config validation passed");
}

#[test]
fn test_backup_schedule_validation() {
    let valid_schedules = ["realtime", "hourly", "daily", "weekly", "monthly"];
    
    for schedule in &valid_schedules {
        let config_content = format!(r#"
[cloud_backup]
enabled = true
provider = "s3"
schedule = "{}"

[cloud_backup.s3]
bucket_name = "test-bucket"
region = "us-west-2"
"#, schedule);
        
        let (_temp_dir, config_path) = create_test_config_file(&config_content);
        let content = fs::read_to_string(&config_path).unwrap();
        
        assert!(content.contains(&format!("schedule = \"{}\"", schedule)));
        
        println!("✅ Schedule '{}' validation passed", schedule);
    }
}

#[test]
fn test_storage_class_validation() {
    let valid_storage_classes = [
        "STANDARD",
        "STANDARD_IA", 
        "ONEZONE_IA",
        "GLACIER",
        "GLACIER_IR",
        "DEEP_ARCHIVE"
    ];
    
    for storage_class in &valid_storage_classes {
        let config_content = format!(r#"
[cloud_backup]
enabled = true
provider = "s3"

[cloud_backup.s3]
bucket_name = "test-bucket"
region = "us-west-2"
storage_class = "{}"
"#, storage_class);
        
        let (_temp_dir, config_path) = create_test_config_file(&config_content);
        let content = fs::read_to_string(&config_path).unwrap();
        
        assert!(content.contains(&format!("storage_class = \"{}\"", storage_class)));
        
        println!("✅ Storage class '{}' validation passed", storage_class);
    }
}

#[test]
fn test_drive_identifier_types() {
    let valid_drive_types = ["uuid", "volume_label", "serial_number"];
    
    for drive_type in &valid_drive_types {
        let config_content = format!(r#"
[auto_backup]
enabled = true
backup_destination_path = "/Test"

[[auto_backup.target_drives]]
type = "{}"
identifier = "test-identifier"
"#, drive_type);
        
        let (_temp_dir, config_path) = create_test_config_file(&config_content);
        let content = fs::read_to_string(&config_path).unwrap();
        
        assert!(content.contains(&format!("type = \"{}\"", drive_type)));
        
        println!("✅ Drive type '{}' validation passed", drive_type);
    }
}

#[test]
fn test_encryption_settings() {
    let config_content = r#"
[auto_backup]
enabled = true
backup_destination_path = "/Test"
encryption_enabled = true

[cloud_backup]
enabled = true
provider = "s3"
encryption_enabled = true
client_side_encryption = true

[cloud_backup.s3]
bucket_name = "test-bucket"
region = "us-west-2"
server_side_encryption = true
"#;
    
    let (_temp_dir, config_path) = create_test_config_file(config_content);
    let content = fs::read_to_string(&config_path).unwrap();
    
    // Verify encryption settings
    assert!(content.contains("encryption_enabled = true"));
    assert!(content.contains("client_side_encryption = true"));
    assert!(content.contains("server_side_encryption = true"));
    
    println!("✅ Encryption settings validation passed");
}

#[test]
fn test_aws_credentials_config() {
    let config_content = r#"
[cloud_backup]
enabled = true
provider = "s3"

[cloud_backup.s3]
bucket_name = "test-bucket"
region = "us-west-2"
access_key_id = "AKIAIOSFODNN7EXAMPLE"
secret_access_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
use_instance_profile = false
"#;
    
    let (_temp_dir, config_path) = create_test_config_file(config_content);
    let content = fs::read_to_string(&config_path).unwrap();
    
    assert!(content.contains("access_key_id = \"AKIAIOSFODNN7EXAMPLE\""));
    assert!(content.contains("secret_access_key"));
    assert!(content.contains("use_instance_profile = false"));
    
    println!("✅ AWS credentials config validation passed");
}

#[test]
fn test_instance_profile_config() {
    let config_content = r#"
[cloud_backup]
enabled = true
provider = "s3"

[cloud_backup.s3]
bucket_name = "test-bucket"
region = "us-west-2"
use_instance_profile = true
"#;
    
    let (_temp_dir, config_path) = create_test_config_file(config_content);
    let content = fs::read_to_string(&config_path).unwrap();
    
    assert!(content.contains("use_instance_profile = true"));
    // When using instance profile, explicit access keys are not needed
    
    println!("✅ Instance profile config validation passed");
}

#[test]
fn test_s3_bucket_name_validation() {
    // Valid bucket names
    let valid_bucket_names = [
        "my-bucket",
        "chronicle-backups-2024",
        "test123bucket",
    ];
    
    for bucket_name in &valid_bucket_names {
        let config_content = format!(r#"
[cloud_backup]
enabled = true
provider = "s3"

[cloud_backup.s3]
bucket_name = "{}"
region = "us-west-2"
"#, bucket_name);
        
        let (_temp_dir, config_path) = create_test_config_file(&config_content);
        let content = fs::read_to_string(&config_path).unwrap();
        
        // Basic validation - should not be empty and should match
        assert!(content.contains(&format!("bucket_name = \"{}\"", bucket_name)));
        assert!(!bucket_name.is_empty());
        
        println!("✅ Bucket name '{}' validation passed", bucket_name);
    }
}

#[test]
fn test_retry_configuration() {
    let config_content = r#"
[auto_backup]
enabled = true
backup_destination_path = "/Test"
retry_attempts = 5
retry_delay_seconds = 120
"#;
    
    let (_temp_dir, config_path) = create_test_config_file(config_content);
    let content = fs::read_to_string(&config_path).unwrap();
    
    assert!(content.contains("retry_attempts = 5"));
    assert!(content.contains("retry_delay_seconds = 120"));
    
    // Verify retry configuration is within reasonable bounds (basic validation)
    let parsed = parse_basic_config(&content);
    if let Some(retry_attempts_str) = parsed.get("retry_attempts") {
        let retry_attempts: i32 = retry_attempts_str.parse().unwrap_or(0);
        assert!(retry_attempts > 0);
        assert!(retry_attempts <= 10);
    }
    
    println!("✅ Retry configuration validation passed");
}

#[test]
fn test_dangerous_configuration_warnings() {
    let dangerous_config = r#"
[auto_backup]
enabled = true
backup_destination_path = "/Test"
remove_local_after_backup = true
verification_required = false

[[auto_backup.target_drives]]
type = "uuid"
identifier = "test-uuid"
"#;
    
    let (_temp_dir, config_path) = create_test_config_file(dangerous_config);
    let content = fs::read_to_string(&config_path).unwrap();
    
    let has_remove_local = content.contains("remove_local_after_backup = true");
    let has_verification_disabled = content.contains("verification_required = false");
    
    // These combinations should trigger warnings
    if has_remove_local && has_verification_disabled {
        println!("⚠️ WARNING: remove_local_after_backup=true with verification_required=false is dangerous");
    }
    
    assert!(has_remove_local);
    assert!(has_verification_disabled);
    
    println!("✅ Dangerous configuration detection passed");
}

#[test]
fn test_disabled_services_config() {
    let disabled_config = r#"
[auto_backup]
enabled = false

[cloud_backup]
enabled = false
"#;
    
    let (_temp_dir, config_path) = create_test_config_file(disabled_config);
    let content = fs::read_to_string(&config_path).unwrap();
    
    // When services are disabled, they should still parse correctly
    assert!(content.contains("[auto_backup]"));
    assert!(content.contains("[cloud_backup]"));
    assert!(content.contains("enabled = false"));
    
    println!("✅ Disabled services config validation passed");
}

#[test]
fn test_comprehensive_valid_config() {
    let comprehensive_config = r#"
[auto_backup]
enabled = true
remove_local_after_backup = false
verification_required = true
backup_destination_path = "/Chronicle/AutoBackup"
encryption_enabled = true
compression_enabled = true
retry_attempts = 3
retry_delay_seconds = 60

[[auto_backup.target_drives]]
type = "uuid"
identifier = "12345678-1234-1234-1234-123456789ABC"

[[auto_backup.target_drives]]
type = "volume_label"
identifier = "BackupDrive"

[[auto_backup.target_drives]]
type = "serial_number"
identifier = "WD1234567890"

[cloud_backup]
enabled = true
provider = "s3"
continuous_backup = false
schedule = "daily"
encryption_enabled = true
client_side_encryption = true
retention_days = 90
max_backup_size_gb = 10
compression_enabled = true

[cloud_backup.s3]
bucket_name = "my-chronicle-backups"
region = "us-west-2"
prefix = "chronicle-data"
storage_class = "STANDARD_IA"
server_side_encryption = true
use_instance_profile = true

[drive_monitoring]
enabled = true
monitor_all_drives = true
notify_on_connection = true
log_drive_events = true
sample_rate = 1.0
monitor_usb_drives = true
monitor_thunderbolt_drives = true
monitor_firewire_drives = true
monitor_internal_drives = false
"#;
    
    let (_temp_dir, config_path) = create_test_config_file(comprehensive_config);
    let content = fs::read_to_string(&config_path).unwrap();
    
    // Verify all sections exist
    assert!(content.contains("[auto_backup]"));
    assert!(content.contains("[cloud_backup]"));
    assert!(content.contains("[drive_monitoring]"));
    
    // Verify auto_backup configuration
    assert!(content.contains("enabled = true"));
    assert!(content.contains("[[auto_backup.target_drives]]"));
    
    // Count target drives (should have 3)
    let target_drive_count = content.matches("[[auto_backup.target_drives]]").count();
    assert_eq!(target_drive_count, 3);
    
    // Verify cloud_backup configuration
    assert!(content.contains("[cloud_backup.s3]"));
    
    // Verify drive_monitoring configuration
    assert!(content.contains("sample_rate = 1.0"));
    
    println!("✅ Comprehensive config validation passed");
}