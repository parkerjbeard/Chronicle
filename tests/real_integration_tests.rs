//! Comprehensive real integration tests that perform actual file operations

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
    collections::HashMap,
    process::Command,
    io::{Read, Write},
};
use tempfile::{TempDir, NamedTempFile};
use sha2::{Sha256, Digest};
use flate2::{write::GzEncoder, read::GzDecoder, Compression};

/// Real backup system that performs actual file operations
struct RealBackupSystem {
    backup_root: TempDir,
    config_file: PathBuf,
    backup_manifest: PathBuf,
}

impl RealBackupSystem {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let backup_root = TempDir::new()?;
        let config_file = backup_root.path().join("chronicle_config.toml");
        let backup_manifest = backup_root.path().join("backup_manifest.json");
        
        // Create default configuration
        let config = r#"
[auto_backup]
enabled = true
backup_destination_path = "auto_backups"
encryption_enabled = false
compression_enabled = true
verification_required = true
retry_attempts = 3
retry_delay_seconds = 30

[cloud_backup]
enabled = false
provider = "s3"
schedule = "daily"
encryption_enabled = true

[drive_monitoring]
enabled = true
monitor_all_drives = false
notify_on_connection = true
log_drive_events = true
"#;
        
        fs::write(&config_file, config)?;
        
        Ok(Self {
            backup_root,
            config_file,
            backup_manifest,
        })
    }
    
    fn get_backup_dir(&self) -> PathBuf {
        self.backup_root.path().join("auto_backups")
    }
    
    fn create_backup_directory(&self) -> Result<(), Box<dyn std::error::Error>> {
        let backup_dir = self.get_backup_dir();
        fs::create_dir_all(&backup_dir)?;
        Ok(())
    }
    
    fn backup_file(&self, source_path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
        self.create_backup_directory()?;
        
        let filename = source_path.file_name()
            .ok_or("Invalid source file name")?
            .to_string_lossy();
            
        let backup_path = self.get_backup_dir().join(&*filename);
        
        // Read source file
        let source_data = fs::read(source_path)?;
        
        // Compress if enabled
        let compressed_data = self.compress_data(&source_data)?;
        
        // Write backup
        fs::write(&backup_path, compressed_data)?;
        
        // Update manifest
        self.update_manifest(&backup_path, source_path, &source_data)?;
        
        // Verify backup
        self.verify_backup(&backup_path, &source_data)?;
        
        Ok(backup_path)
    }
    
    fn backup_directory(&self, source_dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        self.create_backup_directory()?;
        
        let mut backed_up_files = Vec::new();
        
        for entry in fs::read_dir(source_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                println!("  Backing up file: {:?}", path);
                let backup_path = self.backup_file(&path)?;
                backed_up_files.push(backup_path);
            }
        }
        
        println!("  Backed up {} files", backed_up_files.len());
        Ok(backed_up_files)
    }
    
    fn restore_file(&self, backup_path: &Path, restore_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        // Read compressed backup
        let compressed_data = fs::read(backup_path)?;
        println!("  Read {} bytes from backup file", compressed_data.len());
        
        // Decompress
        let original_data = self.decompress_data(&compressed_data)?;
        println!("  Decompressed to {} bytes", original_data.len());
        
        // Write restored file
        fs::write(restore_path, original_data)?;
        
        Ok(())
    }
    
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        let compressed = encoder.finish()?;
        Ok(compressed)
    }
    
    fn decompress_data(&self, compressed_data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut decoder = GzDecoder::new(compressed_data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }
    
    fn calculate_checksum(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }
    
    fn update_manifest(&self, backup_path: &Path, source_path: &Path, original_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let mut manifest = if self.backup_manifest.exists() {
            let manifest_content = fs::read_to_string(&self.backup_manifest)?;
            serde_json::from_str(&manifest_content).unwrap_or_else(|_| serde_json::Map::new())
        } else {
            serde_json::Map::new()
        };
        
        let entry = serde_json::json!({
            "source_path": source_path.to_string_lossy(),
            "backup_path": backup_path.to_string_lossy(),
            "checksum": self.calculate_checksum(original_data),
            "size": original_data.len(),
            "timestamp": SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs(),
        });
        
        manifest.insert(
            backup_path.file_name().unwrap().to_string_lossy().to_string(),
            entry
        );
        
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        fs::write(&self.backup_manifest, manifest_json)?;
        
        Ok(())
    }
    
    fn verify_backup(&self, backup_path: &Path, original_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        // Restore backup to temporary location
        let temp_file = NamedTempFile::new()?;
        self.restore_file(backup_path, temp_file.path())?;
        
        // Read restored data
        let restored_data = fs::read(temp_file.path())?;
        
        // Verify checksums match
        let original_checksum = self.calculate_checksum(original_data);
        let restored_checksum = self.calculate_checksum(&restored_data);
        
        if original_checksum != restored_checksum {
            return Err("Backup verification failed: checksums don't match".into());
        }
        
        // Verify data is identical
        if original_data != restored_data {
            return Err("Backup verification failed: data doesn't match".into());
        }
        
        Ok(())
    }
    
    fn list_backups(&self) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let backup_dir = self.get_backup_dir();
        
        if !backup_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut backups = Vec::new();
        for entry in fs::read_dir(&backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                backups.push(path);
            }
        }
        
        Ok(backups)
    }
    
    fn get_backup_stats(&self) -> Result<BackupStats, Box<dyn std::error::Error>> {
        let backups = self.list_backups()?;
        let mut total_size = 0;
        let mut total_original_size = 0;
        
        if self.backup_manifest.exists() {
            let manifest_content = fs::read_to_string(&self.backup_manifest)?;
            let manifest: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&manifest_content)?;
            
            for (_key, value) in manifest {
                if let Some(size) = value.get("size").and_then(|s| s.as_u64()) {
                    total_original_size += size;
                }
            }
        }
        
        for backup_path in &backups {
            let metadata = fs::metadata(backup_path)?;
            total_size += metadata.len();
        }
        
        let compression_ratio = if total_original_size > 0 {
            total_size as f64 / total_original_size as f64
        } else {
            1.0
        };
        
        Ok(BackupStats {
            total_backups: backups.len(),
            total_size_bytes: total_size,
            total_original_size_bytes: total_original_size,
            compression_ratio,
        })
    }
    
    fn cleanup(&self, backup_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if backup_path.exists() {
            fs::remove_file(backup_path)?;
            
            // Update manifest to remove entry
            if self.backup_manifest.exists() {
                let manifest_content = fs::read_to_string(&self.backup_manifest)?;
                let mut manifest: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&manifest_content)?;
                
                let filename = backup_path.file_name().unwrap().to_string_lossy().to_string();
                manifest.remove(&filename);
                
                let manifest_json = serde_json::to_string_pretty(&manifest)?;
                fs::write(&self.backup_manifest, manifest_json)?;
            }
        }
        
        Ok(())
    }
}

#[derive(Debug)]
struct BackupStats {
    total_backups: usize,
    total_size_bytes: u64,
    total_original_size_bytes: u64,
    compression_ratio: f64,
}

/// Create test files of different sizes and types
fn create_test_files(test_dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut test_files = Vec::new();
    
    // Small text file
    let text_file = test_dir.join("small_text.txt");
    fs::write(&text_file, "This is a small text file for backup testing.\nIt has multiple lines.\nAnd some content to compress.")?;
    test_files.push(text_file);
    
    // Medium binary file
    let binary_file = test_dir.join("medium_binary.dat");
    let binary_data: Vec<u8> = (0..10240).map(|i| (i % 256) as u8).collect();
    fs::write(&binary_file, binary_data)?;
    test_files.push(binary_file);
    
    // Large text file (highly compressible)
    let large_file = test_dir.join("large_text.log");
    let mut large_content = String::new();
    for i in 0..1000 {
        large_content.push_str(&format!("2024-01-{:02} 12:00:{:02} INFO [thread-{}] This is log entry number {}\n", 
                                       i % 28 + 1, i % 60, i % 10, i));
    }
    fs::write(&large_file, large_content)?;
    test_files.push(large_file);
    
    // Configuration file (JSON)
    let config_file = test_dir.join("config.json");
    let config_data = serde_json::json!({
        "database": {
            "host": "localhost",
            "port": 5432,
            "name": "test_db",
            "credentials": {
                "username": "test_user",
                "password": "secure_password_123"
            }
        },
        "backup": {
            "enabled": true,
            "schedule": "daily",
            "retention_days": 30,
            "compression": true,
            "encryption": false
        },
        "logging": {
            "level": "info",
            "output": "file",
            "path": "/var/log/app.log"
        }
    });
    fs::write(&config_file, serde_json::to_string_pretty(&config_data)?)?;
    test_files.push(config_file);
    
    // Empty file
    let empty_file = test_dir.join("empty.txt");
    fs::write(&empty_file, "")?;
    test_files.push(empty_file);
    
    Ok(test_files)
}

#[test]
fn test_real_single_file_backup_and_restore() {
    println!("🗂️ Testing real single file backup and restore");
    
    let test_dir = TempDir::new().expect("Failed to create test directory");
    let backup_system = RealBackupSystem::new().expect("Failed to create backup system");
    
    // Create a test file
    let test_file = test_dir.path().join("test_document.txt");
    let test_content = "This is important data that needs to be backed up!\nIt has multiple lines and should compress well.";
    fs::write(&test_file, test_content).expect("Failed to create test file");
    
    // Backup the file
    let backup_path = backup_system.backup_file(&test_file)
        .expect("Backup should succeed");
    
    assert!(backup_path.exists(), "Backup file should exist");
    
    // Verify backup is smaller due to compression
    let original_size = fs::metadata(&test_file).unwrap().len();
    let backup_size = fs::metadata(&backup_path).unwrap().len();
    println!("  Original size: {} bytes, Backup size: {} bytes", original_size, backup_size);
    
    // Restore to a new location
    let restore_file = test_dir.path().join("restored_document.txt");
    backup_system.restore_file(&backup_path, &restore_file)
        .expect("Restore should succeed");
    
    // Verify restored content matches original
    let restored_content = fs::read_to_string(&restore_file).expect("Should read restored file");
    assert_eq!(restored_content, test_content, "Restored content should match original");
    
    println!("✅ Single file backup and restore test passed");
}

#[test]
fn test_real_directory_backup() {
    println!("📁 Testing real directory backup");
    
    let test_dir = TempDir::new().expect("Failed to create test directory");
    let backup_system = RealBackupSystem::new().expect("Failed to create backup system");
    
    // Create test files
    let test_files = create_test_files(test_dir.path()).expect("Failed to create test files");
    
    println!("  Created {} test files", test_files.len());
    
    // Backup the entire directory
    let backup_paths = backup_system.backup_directory(test_dir.path())
        .expect("Directory backup should succeed");
    
    assert_eq!(backup_paths.len(), test_files.len(), "Should backup all files");
    
    // Verify all backup files exist
    for backup_path in &backup_paths {
        assert!(backup_path.exists(), "Backup file should exist: {:?}", backup_path);
    }
    
    // Test restoration of each file by matching backup names to original names
    let restore_dir = TempDir::new().expect("Failed to create restore directory");
    
    for backup_path in &backup_paths {
        let backup_filename = backup_path.file_name().unwrap().to_string_lossy();
        
        // Find matching original file
        let matching_original = test_files.iter().find(|test_file| {
            test_file.file_name().unwrap().to_string_lossy() == backup_filename
        }).expect("Should find matching original file");
        
        let restore_path = restore_dir.path().join(format!("restored_{}", backup_filename));
        backup_system.restore_file(backup_path, &restore_path)
            .expect("Restore should succeed");
        
        assert!(restore_path.exists(), "Restored file should exist");
        
        // Verify content matches original
        let original_content = fs::read(matching_original).expect("Should read original file");
        let restored_content = fs::read(&restore_path).expect("Should read restored file");
        assert_eq!(original_content, restored_content, "Content should match for file {}", backup_filename);
    }
    
    println!("✅ Directory backup test passed");
}

#[test]
fn test_real_backup_verification() {
    println!("✅ Testing real backup verification");
    
    let test_dir = TempDir::new().expect("Failed to create test directory");
    let backup_system = RealBackupSystem::new().expect("Failed to create backup system");
    
    // Create test file with specific content
    let test_file = test_dir.path().join("verify_test.txt");
    let test_content = b"This content will be verified through checksums and integrity checks.";
    fs::write(&test_file, test_content).expect("Failed to create test file");
    
    // Backup with verification
    let backup_path = backup_system.backup_file(&test_file)
        .expect("Backup with verification should succeed");
    
    // Manually verify the backup can be restored correctly
    let temp_restore = NamedTempFile::new().expect("Failed to create temp file");
    backup_system.restore_file(&backup_path, temp_restore.path())
        .expect("Restore should succeed");
    
    let restored_content = fs::read(temp_restore.path()).expect("Should read restored file");
    assert_eq!(restored_content, test_content, "Verification should confirm content integrity");
    
    println!("✅ Backup verification test passed");
}

#[test]
fn test_real_backup_manifest() {
    println!("📋 Testing real backup manifest");
    
    let test_dir = TempDir::new().expect("Failed to create test directory");
    let backup_system = RealBackupSystem::new().expect("Failed to create backup system");
    
    // Create multiple test files
    let test_files = create_test_files(test_dir.path()).expect("Failed to create test files");
    
    // Backup all files
    let _backup_paths = backup_system.backup_directory(test_dir.path())
        .expect("Directory backup should succeed");
    
    // Check manifest was created and contains correct entries
    assert!(backup_system.backup_manifest.exists(), "Manifest file should exist");
    
    let manifest_content = fs::read_to_string(&backup_system.backup_manifest)
        .expect("Should read manifest");
    
    let manifest: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&manifest_content)
        .expect("Manifest should be valid JSON");
    
    assert_eq!(manifest.len(), test_files.len(), "Manifest should contain entry for each file");
    
    // Verify manifest entries have required fields
    for (_key, entry) in manifest {
        assert!(entry.get("source_path").is_some(), "Entry should have source_path");
        assert!(entry.get("backup_path").is_some(), "Entry should have backup_path");
        assert!(entry.get("checksum").is_some(), "Entry should have checksum");
        assert!(entry.get("size").is_some(), "Entry should have size");
        assert!(entry.get("timestamp").is_some(), "Entry should have timestamp");
    }
    
    println!("✅ Backup manifest test passed");
}

#[test]
fn test_real_compression_effectiveness() {
    println!("🗜️ Testing real compression effectiveness");
    
    let test_dir = TempDir::new().expect("Failed to create test directory");
    let backup_system = RealBackupSystem::new().expect("Failed to create backup system");
    
    // Create highly compressible content
    let compressible_file = test_dir.path().join("compressible.txt");
    let mut compressible_content = String::new();
    for _ in 0..1000 {
        compressible_content.push_str("This line repeats many times and should compress very well.\n");
    }
    fs::write(&compressible_file, &compressible_content).expect("Failed to create compressible file");
    
    // Create less compressible content (pseudo-random)
    let random_file = test_dir.path().join("random.dat");
    let random_data: Vec<u8> = (0..10000).map(|i| ((i * 7919) % 256) as u8).collect();
    fs::write(&random_file, &random_data).expect("Failed to create random file");
    
    // Backup both files
    let compressible_backup = backup_system.backup_file(&compressible_file)
        .expect("Compressible backup should succeed");
    let random_backup = backup_system.backup_file(&random_file)
        .expect("Random backup should succeed");
    
    // Check compression ratios
    let compressible_original_size = fs::metadata(&compressible_file).unwrap().len();
    let compressible_backup_size = fs::metadata(&compressible_backup).unwrap().len();
    let compressible_ratio = compressible_backup_size as f64 / compressible_original_size as f64;
    
    let random_original_size = fs::metadata(&random_file).unwrap().len();
    let random_backup_size = fs::metadata(&random_backup).unwrap().len();
    let random_ratio = random_backup_size as f64 / random_original_size as f64;
    
    println!("  Compressible file: {} bytes -> {} bytes (ratio: {:.2})", 
             compressible_original_size, compressible_backup_size, compressible_ratio);
    println!("  Random file: {} bytes -> {} bytes (ratio: {:.2})", 
             random_original_size, random_backup_size, random_ratio);
    
    // Compressible content should compress well
    assert!(compressible_ratio < 0.1, "Highly compressible content should compress to <10% of original size");
    
    // Compressible content should compress significantly better than pseudo-random content
    assert!(compressible_ratio < random_ratio, "Highly compressible content should compress better than pseudo-random content");
    
    println!("✅ Compression effectiveness test passed");
}

#[test]
fn test_real_backup_statistics() {
    println!("📊 Testing real backup statistics");
    
    let test_dir = TempDir::new().expect("Failed to create test directory");
    let backup_system = RealBackupSystem::new().expect("Failed to create backup system");
    
    // Create test files
    let test_files = create_test_files(test_dir.path()).expect("Failed to create test files");
    
    // Backup all files
    let _backup_paths = backup_system.backup_directory(test_dir.path())
        .expect("Directory backup should succeed");
    
    // Get statistics
    let stats = backup_system.get_backup_stats()
        .expect("Should get backup statistics");
    
    println!("  Total backups: {}", stats.total_backups);
    println!("  Total backup size: {} bytes", stats.total_size_bytes);
    println!("  Total original size: {} bytes", stats.total_original_size_bytes);
    println!("  Overall compression ratio: {:.2}", stats.compression_ratio);
    
    assert_eq!(stats.total_backups, test_files.len(), "Should count all backup files");
    assert!(stats.total_size_bytes > 0, "Should have non-zero backup size");
    assert!(stats.total_original_size_bytes > 0, "Should have non-zero original size");
    assert!(stats.compression_ratio < 1.0, "Should show compression was effective");
    
    println!("✅ Backup statistics test passed");
}

#[test]
fn test_real_backup_cleanup() {
    println!("🧹 Testing real backup cleanup");
    
    let test_dir = TempDir::new().expect("Failed to create test directory");
    let backup_system = RealBackupSystem::new().expect("Failed to create backup system");
    
    // Create and backup a test file
    let test_file = test_dir.path().join("cleanup_test.txt");
    fs::write(&test_file, "This file will be backed up and then cleaned up").expect("Failed to create test file");
    
    let backup_path = backup_system.backup_file(&test_file)
        .expect("Backup should succeed");
    
    assert!(backup_path.exists(), "Backup should exist before cleanup");
    
    // Verify manifest contains the entry
    let manifest_content = fs::read_to_string(&backup_system.backup_manifest)
        .expect("Should read manifest");
    let manifest: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&manifest_content)
        .expect("Manifest should be valid JSON");
    
    let filename = backup_path.file_name().unwrap().to_string_lossy().to_string();
    assert!(manifest.contains_key(&filename), "Manifest should contain backup entry");
    
    // Clean up the backup
    backup_system.cleanup(&backup_path)
        .expect("Cleanup should succeed");
    
    assert!(!backup_path.exists(), "Backup should be deleted after cleanup");
    
    // Verify manifest was updated
    let updated_manifest_content = fs::read_to_string(&backup_system.backup_manifest)
        .expect("Should read updated manifest");
    let updated_manifest: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&updated_manifest_content)
        .expect("Updated manifest should be valid JSON");
    
    assert!(!updated_manifest.contains_key(&filename), "Manifest should not contain cleaned up entry");
    
    println!("✅ Backup cleanup test passed");
}

#[test]
fn test_real_large_file_backup() {
    println!("📦 Testing real large file backup");
    
    let test_dir = TempDir::new().expect("Failed to create test directory");
    let backup_system = RealBackupSystem::new().expect("Failed to create backup system");
    
    // Create a larger test file (1MB)
    let large_file = test_dir.path().join("large_file.dat");
    let large_data: Vec<u8> = (0..1024*1024).map(|i| (i % 256) as u8).collect();
    fs::write(&large_file, &large_data).expect("Failed to create large file");
    
    println!("  Created 1MB test file");
    
    // Backup the large file
    let start_time = std::time::Instant::now();
    let backup_path = backup_system.backup_file(&large_file)
        .expect("Large file backup should succeed");
    let backup_duration = start_time.elapsed();
    
    println!("  Backup completed in {:?}", backup_duration);
    
    assert!(backup_path.exists(), "Large file backup should exist");
    
    // Verify backup integrity by restoration
    let restore_file = test_dir.path().join("restored_large_file.dat");
    let start_time = std::time::Instant::now();
    backup_system.restore_file(&backup_path, &restore_file)
        .expect("Large file restore should succeed");
    let restore_duration = start_time.elapsed();
    
    println!("  Restore completed in {:?}", restore_duration);
    
    // Verify restored content
    let restored_data = fs::read(&restore_file).expect("Should read restored large file");
    assert_eq!(restored_data, large_data, "Restored large file should match original");
    
    // Performance check - should complete within reasonable time
    assert!(backup_duration < Duration::from_secs(10), "Large file backup should complete within 10 seconds");
    assert!(restore_duration < Duration::from_secs(10), "Large file restore should complete within 10 seconds");
    
    println!("✅ Large file backup test passed");
}

#[test]
fn test_real_concurrent_backups() {
    println!("⚡ Testing real concurrent backups");
    
    use std::sync::Arc;
    use std::thread;
    
    let test_dir = TempDir::new().expect("Failed to create test directory");
    let backup_system = Arc::new(RealBackupSystem::new().expect("Failed to create backup system"));
    
    // Create multiple test files
    let mut test_files = Vec::new();
    for i in 0..5 {
        let file_path = test_dir.path().join(format!("concurrent_test_{}.txt", i));
        let content = format!("This is test file number {} for concurrent backup testing.", i);
        fs::write(&file_path, content).expect("Failed to create test file");
        test_files.push(file_path);
    }
    
    // Launch concurrent backup threads
    let mut handles = Vec::new();
    
    for (i, test_file) in test_files.into_iter().enumerate() {
        let backup_system_clone = Arc::clone(&backup_system);
        
        let handle = thread::spawn(move || {
            println!("  Thread {} starting backup", i);
            let result = backup_system_clone.backup_file(&test_file);
            println!("  Thread {} backup result: {:?}", i, result.is_ok());
            // Convert to a Send-safe error type
            match result {
                Ok(path) => Ok(path),
                Err(e) => Err(e.to_string()),
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    let mut successful_backups = 0;
    
    for handle in handles {
        match handle.join().expect("Thread should complete") {
            Ok(_) => successful_backups += 1,
            Err(e) => println!("  Backup failed: {}", e),
        }
    }
    
    println!("  {} out of 5 concurrent backups succeeded", successful_backups);
    
    // All backups should succeed
    assert_eq!(successful_backups, 5, "All concurrent backups should succeed");
    
    // Verify all backup files exist
    let backups = backup_system.list_backups().expect("Should list backups");
    assert_eq!(backups.len(), 5, "Should have 5 backup files");
    
    println!("✅ Concurrent backups test passed");
}

#[test]
fn test_real_backup_configuration_loading() {
    println!("⚙️ Testing real backup configuration loading");
    
    let backup_system = RealBackupSystem::new().expect("Failed to create backup system");
    
    // Verify configuration file exists
    assert!(backup_system.config_file.exists(), "Configuration file should exist");
    
    // Read and parse configuration
    let config_content = fs::read_to_string(&backup_system.config_file)
        .expect("Should read configuration file");
    
    // Basic TOML parsing test
    let parsed_config: toml::Value = toml::from_str(&config_content)
        .expect("Configuration should be valid TOML");
    
    // Verify expected sections exist
    assert!(parsed_config.get("auto_backup").is_some(), "Should have auto_backup section");
    assert!(parsed_config.get("cloud_backup").is_some(), "Should have cloud_backup section");
    assert!(parsed_config.get("drive_monitoring").is_some(), "Should have drive_monitoring section");
    
    // Verify specific configuration values
    let auto_backup = parsed_config["auto_backup"].as_table().unwrap();
    assert_eq!(auto_backup["enabled"].as_bool().unwrap(), true);
    assert_eq!(auto_backup["compression_enabled"].as_bool().unwrap(), true);
    assert_eq!(auto_backup["verification_required"].as_bool().unwrap(), true);
    
    println!("✅ Configuration loading test passed");
}

#[test]
fn test_real_end_to_end_backup_workflow() {
    println!("🎯 Testing real end-to-end backup workflow");
    
    let test_dir = TempDir::new().expect("Failed to create test directory");
    let backup_system = RealBackupSystem::new().expect("Failed to create backup system");
    
    println!("  1. Creating test data...");
    let test_files = create_test_files(test_dir.path()).expect("Failed to create test files");
    
    println!("  2. Performing directory backup...");
    let backup_paths = backup_system.backup_directory(test_dir.path())
        .expect("Directory backup should succeed");
    
    println!("  3. Verifying all backups exist...");
    assert_eq!(backup_paths.len(), test_files.len());
    for backup_path in &backup_paths {
        assert!(backup_path.exists());
    }
    
    println!("  4. Checking backup statistics...");
    let stats = backup_system.get_backup_stats()
        .expect("Should get backup statistics");
    assert_eq!(stats.total_backups, test_files.len());
    assert!(stats.compression_ratio < 1.0);
    
    println!("  5. Testing restoration...");
    let restore_dir = TempDir::new().expect("Failed to create restore directory");
    for backup_path in &backup_paths {
        let backup_filename = backup_path.file_name().unwrap().to_string_lossy();
        
        // Find matching original file
        let matching_original = test_files.iter().find(|test_file| {
            test_file.file_name().unwrap().to_string_lossy() == backup_filename
        }).expect("Should find matching original file");
        
        let restore_path = restore_dir.path().join(format!("restored_{}", backup_filename));
        backup_system.restore_file(backup_path, &restore_path)
            .expect("Restore should succeed");
        
        let original_data = fs::read(matching_original).expect("Should read original");
        let restored_data = fs::read(&restore_path).expect("Should read restored");
        assert_eq!(original_data, restored_data);
    }
    
    println!("  6. Testing backup listing...");
    let listed_backups = backup_system.list_backups()
        .expect("Should list backups");
    assert_eq!(listed_backups.len(), backup_paths.len());
    
    println!("  7. Testing selective cleanup...");
    let cleanup_backup = &backup_paths[0];
    backup_system.cleanup(cleanup_backup)
        .expect("Cleanup should succeed");
    assert!(!cleanup_backup.exists());
    
    let remaining_backups = backup_system.list_backups()
        .expect("Should list remaining backups");
    assert_eq!(remaining_backups.len(), backup_paths.len() - 1);
    
    println!("✅ End-to-end workflow test completed successfully");
}