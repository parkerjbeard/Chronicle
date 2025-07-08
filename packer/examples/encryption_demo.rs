//! Encryption demonstration for Chronicle packer service
//!
//! This example shows how to use the encryption service for
//! securing data at rest.

use std::fs;

use chronicle_packer::{
    config::EncryptionConfig,
    encryption::EncryptionService,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("Chronicle Encryption Service - Demo");
    println!("==================================");
    
    // Create encryption configuration
    let mut config = EncryptionConfig::default();
    config.enabled = true;
    config.kdf_iterations = 10000; // Reasonable for demo
    
    println!("Encryption configuration:");
    println!("  Algorithm: {}", config.algorithm);
    println!("  KDF: {}", config.kdf);
    println!("  KDF iterations: {}", config.kdf_iterations);
    println!("  Key rotation days: {}", config.key_rotation_days);
    
    // Create encryption service
    println!("\nInitializing encryption service...");
    let mut encryption = EncryptionService::new(config)?;
    
    println!("Current key ID: {}", encryption.current_key_id());
    
    // Test data encryption/decryption
    println!("\nTesting data encryption/decryption...");
    
    let test_data = b"Hello, Chronicle! This is sensitive data that needs to be encrypted.";
    println!("Original data: {}", String::from_utf8_lossy(test_data));
    println!("Original size: {} bytes", test_data.len());
    
    // Encrypt data
    let start_time = std::time::Instant::now();
    let encrypted_data = encryption.encrypt(test_data)?;
    let encrypt_time = start_time.elapsed();
    
    println!("\nEncryption completed in {:?}", encrypt_time);
    println!("Encrypted size: {} bytes", encrypted_data.len());
    println!("Size overhead: {} bytes ({:.1}%)", 
        encrypted_data.len() - test_data.len(),
        ((encrypted_data.len() - test_data.len()) as f64 / test_data.len() as f64) * 100.0
    );
    
    // Decrypt data
    let start_time = std::time::Instant::now();
    let decrypted_data = encryption.decrypt(&encrypted_data)?;
    let decrypt_time = start_time.elapsed();
    
    println!("\nDecryption completed in {:?}", decrypt_time);
    println!("Decrypted data: {}", String::from_utf8_lossy(&decrypted_data));
    println!("Data integrity: {}", if decrypted_data == test_data { "✓ PASSED" } else { "✗ FAILED" });
    
    // Test file encryption
    println!("\nTesting file encryption...");
    
    let temp_dir = tempfile::TempDir::new()?;
    let test_file = temp_dir.path().join("test_document.txt");
    
    // Create test file
    let file_content = "This is a test document with sensitive information.\n\
                       It contains multiple lines of text.\n\
                       Chronicle packer will encrypt this file for secure storage.\n\
                       The encryption is transparent to the user.";
    
    fs::write(&test_file, file_content)?;
    let original_size = fs::metadata(&test_file)?.len();
    
    println!("Created test file: {}", test_file.display());
    println!("Original file size: {} bytes", original_size);
    
    // Encrypt file
    let start_time = std::time::Instant::now();
    encryption.encrypt_file(&test_file)?;
    let encrypt_time = start_time.elapsed();
    
    let encrypted_size = fs::metadata(&test_file)?.len();
    println!("\nFile encryption completed in {:?}", encrypt_time);
    println!("Encrypted file size: {} bytes", encrypted_size);
    
    // Verify file is encrypted (content should be different)
    let encrypted_content = fs::read(&test_file)?;
    let is_encrypted = encrypted_content != file_content.as_bytes();
    println!("File encrypted: {}", if is_encrypted { "✓ YES" } else { "✗ NO" });
    
    // Decrypt file
    let start_time = std::time::Instant::now();
    encryption.decrypt_file(&test_file)?;
    let decrypt_time = start_time.elapsed();
    
    println!("\nFile decryption completed in {:?}", decrypt_time);
    
    // Verify file is restored
    let restored_content = fs::read_to_string(&test_file)?;
    let is_restored = restored_content == file_content;
    println!("File restored: {}", if is_restored { "✓ YES" } else { "✗ NO" });
    
    if is_restored {
        println!("Restored content preview:");
        for (i, line) in restored_content.lines().enumerate() {
            if i < 2 {
                println!("  {}", line);
            } else {
                println!("  ... ({} more lines)", restored_content.lines().count() - 2);
                break;
            }
        }
    }
    
    // Test key rotation
    println!("\nTesting key rotation...");
    
    let original_key_id = encryption.current_key_id().to_string();
    println!("Original key ID: {}", original_key_id);
    
    // Check if rotation is needed
    let needs_rotation = encryption.needs_key_rotation();
    println!("Needs rotation: {}", needs_rotation);
    
    // Force key rotation
    let start_time = std::time::Instant::now();
    encryption.rotate_keys()?;
    let rotation_time = start_time.elapsed();
    
    let new_key_id = encryption.current_key_id();
    println!("\nKey rotation completed in {:?}", rotation_time);
    println!("New key ID: {}", new_key_id);
    println!("Key changed: {}", if new_key_id != original_key_id { "✓ YES" } else { "✗ NO" });
    
    // Test encryption with new key
    println!("\nTesting encryption with new key...");
    
    let new_test_data = b"This data is encrypted with the rotated key.";
    let new_encrypted = encryption.encrypt(new_test_data)?;
    let new_decrypted = encryption.decrypt(&new_encrypted)?;
    
    println!("New key encryption test: {}", 
        if new_decrypted == new_test_data { "✓ PASSED" } else { "✗ FAILED" });
    
    // Test decryption of old data (should still work if old key is available)
    println!("\nTesting decryption of data encrypted with old key...");
    match encryption.decrypt(&encrypted_data) {
        Ok(old_decrypted) => {
            let old_data_ok = old_decrypted == test_data;
            println!("Old data decryption: {}", 
                if old_data_ok { "✓ PASSED" } else { "✗ FAILED" });
        }
        Err(e) => {
            println!("Old data decryption: ✗ FAILED ({})", e);
            println!("Note: This may be expected if old keys are not retained");
        }
    }
    
    // Show key metadata
    if let Some(metadata) = encryption.get_key_metadata(new_key_id) {
        println!("\nCurrent key metadata:");
        println!("  Key ID: {}", metadata.key_id);
        println!("  Created: {}", chrono::DateTime::from_timestamp(metadata.created_at as i64, 0)
            .unwrap_or_else(|| chrono::Utc::now()).format("%Y-%m-%d %H:%M:%S"));
        println!("  Usage count: {}", metadata.usage_count);
        
        if let Some(expires_at) = metadata.expires_at {
            println!("  Expires: {}", chrono::DateTime::from_timestamp(expires_at as i64, 0)
                .unwrap_or_else(|| chrono::Utc::now()).format("%Y-%m-%d %H:%M:%S"));
        }
    }
    
    // List all available keys
    let all_keys = encryption.list_keys();
    println!("\nAvailable keys: {}", all_keys.len());
    for (i, key_id) in all_keys.iter().enumerate() {
        println!("  {}: {}", i + 1, key_id);
    }
    
    // Performance test
    println!("\nPerformance test...");
    
    let test_sizes = vec![1024, 10240, 102400, 1048576]; // 1KB, 10KB, 100KB, 1MB
    
    for size in test_sizes {
        let test_data = vec![0u8; size];
        
        // Encryption
        let start = std::time::Instant::now();
        let encrypted = encryption.encrypt(&test_data)?;
        let encrypt_time = start.elapsed();
        
        // Decryption
        let start = std::time::Instant::now();
        let _decrypted = encryption.decrypt(&encrypted)?;
        let decrypt_time = start.elapsed();
        
        let throughput_mb_s = (size as f64) / (1024.0 * 1024.0) / encrypt_time.as_secs_f64();
        
        println!("  {} bytes: encrypt {:?}, decrypt {:?} ({:.1} MB/s)", 
            size, encrypt_time, decrypt_time, throughput_mb_s);
    }
    
    println!("\nEncryption demo completed successfully!");
    
    Ok(())
}