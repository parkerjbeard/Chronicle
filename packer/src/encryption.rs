//! Encryption module for the Chronicle packer service
//!
//! This module provides AES-256-GCM encryption with Argon2 key derivation
//! for securing Parquet files and HEIF frames at rest.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use aes_gcm::{Aes256Gcm, Key, Nonce, aead::{Aead, AeadCore, KeyInit, OsRng}};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::{SaltString, rand_core::OsRng as ArgonRng}};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::EncryptionConfig;
use crate::error::{EncryptionError, EncryptionResult};

/// Encryption header that is prepended to encrypted files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionHeader {
    /// Version of the encryption format
    pub version: u8,
    
    /// Algorithm used for encryption
    pub algorithm: String,
    
    /// Key derivation function used
    pub kdf: String,
    
    /// Salt used for key derivation
    pub salt: Vec<u8>,
    
    /// Nonce used for encryption
    pub nonce: Vec<u8>,
    
    /// Key identifier
    pub key_id: String,
    
    /// Additional authenticated data
    pub aad: Option<Vec<u8>>,
    
    /// Timestamp when encrypted
    pub timestamp: u64,
}

/// Encryption key metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    /// Key identifier
    pub key_id: String,
    
    /// Key creation timestamp
    pub created_at: u64,
    
    /// Key expiration timestamp
    pub expires_at: Option<u64>,
    
    /// Key usage count
    pub usage_count: u64,
    
    /// Key derivation parameters
    pub kdf_params: KdfParams,
}

/// Key derivation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdfParams {
    /// Number of iterations
    pub iterations: u32,
    
    /// Memory cost in KB
    pub memory_cost: u32,
    
    /// Parallelism factor
    pub parallelism: u32,
    
    /// Salt used for derivation
    pub salt: Vec<u8>,
}

/// Encryption service for managing keys and encrypting/decrypting data
pub struct EncryptionService {
    /// Configuration
    config: EncryptionConfig,
    
    /// Active encryption keys
    keys: HashMap<String, Aes256Gcm>,
    
    /// Key metadata
    key_metadata: HashMap<String, KeyMetadata>,
    
    /// Current key ID
    current_key_id: String,
}

impl EncryptionService {
    /// Create a new encryption service
    pub fn new(config: EncryptionConfig) -> EncryptionResult<Self> {
        let mut service = Self {
            config,
            keys: HashMap::new(),
            key_metadata: HashMap::new(),
            current_key_id: String::new(),
        };
        
        // Initialize with current key
        service.initialize_keys()?;
        
        Ok(service)
    }
    
    /// Initialize encryption keys
    fn initialize_keys(&mut self) -> EncryptionResult<()> {
        // Try to load existing keys from keychain
        match self.load_keys_from_keychain() {
            Ok(_) => {
                tracing::info!("Loaded existing encryption keys from keychain");
            }
            Err(_) => {
                // Create new keys if none exist
                tracing::info!("Creating new encryption keys");
                self.create_new_key()?;
            }
        }
        
        Ok(())
    }
    
    /// Create a new encryption key
    fn create_new_key(&mut self) -> EncryptionResult<()> {
        let key_id = Uuid::new_v4().to_string();
        let passphrase = self.get_or_create_passphrase()?;
        
        // Generate salt for key derivation
        let mut salt = vec![0u8; self.config.salt_size];
        OsRng.fill_bytes(&mut salt);
        
        // Derive key using Argon2
        let kdf_params = KdfParams {
            iterations: self.config.kdf_iterations,
            memory_cost: 65536, // 64MB
            parallelism: 4,
            salt: salt.clone(),
        };
        
        let key = self.derive_key(&passphrase, &kdf_params)?;
        let cipher = Aes256Gcm::new(&key);
        
        // Create key metadata
        let metadata = KeyMetadata {
            key_id: key_id.clone(),
            created_at: chrono::Utc::now().timestamp() as u64,
            expires_at: if self.config.key_rotation_days > 0 {
                Some(chrono::Utc::now().timestamp() as u64 + (self.config.key_rotation_days as u64 * 24 * 60 * 60))
            } else {
                None
            },
            usage_count: 0,
            kdf_params,
        };
        
        // Store key and metadata
        self.keys.insert(key_id.clone(), cipher);
        self.key_metadata.insert(key_id.clone(), metadata);
        self.current_key_id = key_id.clone();
        
        // Save to keychain
        self.save_key_to_keychain(&key_id, &key)?;
        
        tracing::info!("Created new encryption key: {}", key_id);
        Ok(())
    }
    
    /// Derive encryption key from passphrase
    fn derive_key(&self, passphrase: &str, params: &KdfParams) -> EncryptionResult<Key<Aes256Gcm>> {
        let salt = SaltString::from_b64(&base64::encode(&params.salt))
            .map_err(|_| EncryptionError::KeyDerivationFailed)?;
        
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(passphrase.as_bytes(), &salt)
            .map_err(|_| EncryptionError::KeyDerivationFailed)?;
        
        let key_bytes = password_hash.hash
            .ok_or(EncryptionError::KeyDerivationFailed)?;
        
        if key_bytes.len() != 32 {
            return Err(EncryptionError::InvalidKeyFormat);
        }
        
        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&key_bytes.as_bytes()[..32]);
        
        Ok(Key::<Aes256Gcm>::from(key_array))
    }
    
    /// Get or create master passphrase
    fn get_or_create_passphrase(&self) -> EncryptionResult<String> {
        #[cfg(target_os = "macos")]
        {
            use security_framework::passwords::*;
            
            // Try to get existing passphrase from keychain
            match SecPasswordRef::find_internet_password(
                None,
                Some(&self.config.keychain_service),
                Some(&self.config.keychain_account),
                None,
                None,
            ) {
                Ok(password) => {
                    let passphrase = String::from_utf8_lossy(password.password()).to_string();
                    return Ok(passphrase);
                }
                Err(_) => {
                    // Create new passphrase
                    let passphrase = self.generate_passphrase()?;
                    
                    // Save to keychain
                    let password = SecPasswordRef::new(&passphrase, &self.config.keychain_service)?;
                    password.set_internet_password(
                        None,
                        Some(&self.config.keychain_service),
                        Some(&self.config.keychain_account),
                        None,
                        None,
                    )?;
                    
                    return Ok(passphrase);
                }
            }
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            // For non-macOS platforms, generate a new passphrase
            // In production, you'd want to implement proper keyring integration
            self.generate_passphrase()
        }
    }
    
    /// Generate a secure passphrase
    fn generate_passphrase(&self) -> EncryptionResult<String> {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        Ok(base64::encode(&bytes))
    }
    
    /// Load keys from keychain
    fn load_keys_from_keychain(&mut self) -> EncryptionResult<()> {
        // Implementation depends on keychain format
        // This is a placeholder for the actual keychain integration
        Err(EncryptionError::KeyNotFound)
    }
    
    /// Save key to keychain
    fn save_key_to_keychain(&self, key_id: &str, key: &Key<Aes256Gcm>) -> EncryptionResult<()> {
        #[cfg(target_os = "macos")]
        {
            use security_framework::passwords::*;
            
            let service = format!("{}.{}", self.config.keychain_service, key_id);
            let password = SecPasswordRef::new(&base64::encode(key.as_slice()), &service)
                .map_err(|_| EncryptionError::KeychainAccessDenied)?;
            
            password.set_internet_password(
                None,
                Some(&service),
                Some(&self.config.keychain_account),
                None,
                None,
            ).map_err(|_| EncryptionError::KeychainAccessDenied)?;
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            // For non-macOS platforms, implement alternative secure storage
            tracing::warn!("Keychain not available on this platform");
        }
        
        Ok(())
    }
    
    /// Encrypt data with current key
    pub fn encrypt(&mut self, data: &[u8]) -> EncryptionResult<Vec<u8>> {
        let key_id = self.current_key_id.clone();
        self.encrypt_with_key(&key_id, data)
    }
    
    /// Encrypt data with specific key
    pub fn encrypt_with_key(&mut self, key_id: &str, data: &[u8]) -> EncryptionResult<Vec<u8>> {
        let cipher = self.keys.get(key_id)
            .ok_or(EncryptionError::KeyNotFound)?;
        
        // Generate nonce
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        
        // Encrypt data
        let ciphertext = cipher.encrypt(&nonce, data)
            .map_err(|_| EncryptionError::EncryptionFailed { 
                reason: "AES-GCM encryption failed".to_string() 
            })?;
        
        // Update key usage count
        if let Some(metadata) = self.key_metadata.get_mut(key_id) {
            metadata.usage_count += 1;
        }
        
        // Create encryption header
        let header = EncryptionHeader {
            version: 1,
            algorithm: self.config.algorithm.clone(),
            kdf: self.config.kdf.clone(),
            salt: self.key_metadata.get(key_id)
                .map(|m| m.kdf_params.salt.clone())
                .unwrap_or_default(),
            nonce: nonce.to_vec(),
            key_id: key_id.to_string(),
            aad: None,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };
        
        // Serialize header
        let header_bytes = serde_json::to_vec(&header)
            .map_err(|_| EncryptionError::EncryptionFailed { 
                reason: "Header serialization failed".to_string() 
            })?;
        
        // Combine header length + header + ciphertext
        let mut result = Vec::new();
        result.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
        result.extend_from_slice(&header_bytes);
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }
    
    /// Decrypt data
    pub fn decrypt(&self, encrypted_data: &[u8]) -> EncryptionResult<Vec<u8>> {
        if encrypted_data.len() < 4 {
            return Err(EncryptionError::DecryptionFailed { 
                reason: "Invalid encrypted data format".to_string() 
            });
        }
        
        // Read header length
        let header_len = u32::from_le_bytes([
            encrypted_data[0],
            encrypted_data[1],
            encrypted_data[2],
            encrypted_data[3],
        ]) as usize;
        
        if encrypted_data.len() < 4 + header_len {
            return Err(EncryptionError::DecryptionFailed { 
                reason: "Invalid encrypted data format".to_string() 
            });
        }
        
        // Parse header
        let header_bytes = &encrypted_data[4..4 + header_len];
        let header: EncryptionHeader = serde_json::from_slice(header_bytes)
            .map_err(|_| EncryptionError::DecryptionFailed { 
                reason: "Header deserialization failed".to_string() 
            })?;
        
        // Get cipher for the key
        let cipher = self.keys.get(&header.key_id)
            .ok_or(EncryptionError::KeyNotFound)?;
        
        // Extract ciphertext
        let ciphertext = &encrypted_data[4 + header_len..];
        
        // Create nonce
        let nonce = Nonce::from_slice(&header.nonce);
        
        // Decrypt
        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|_| EncryptionError::DecryptionFailed { 
                reason: "AES-GCM decryption failed".to_string() 
            })?;
        
        Ok(plaintext)
    }
    
    /// Encrypt file in place
    pub fn encrypt_file<P: AsRef<Path>>(&mut self, file_path: P) -> EncryptionResult<()> {
        let file_path = file_path.as_ref();
        
        // Read file contents
        let data = fs::read(file_path)
            .map_err(|_| EncryptionError::EncryptionFailed { 
                reason: format!("Failed to read file: {}", file_path.display()) 
            })?;
        
        // Encrypt data
        let encrypted_data = self.encrypt(&data)?;
        
        // Write encrypted data back
        fs::write(file_path, encrypted_data)
            .map_err(|_| EncryptionError::EncryptionFailed { 
                reason: format!("Failed to write encrypted file: {}", file_path.display()) 
            })?;
        
        Ok(())
    }
    
    /// Decrypt file in place
    pub fn decrypt_file<P: AsRef<Path>>(&self, file_path: P) -> EncryptionResult<()> {
        let file_path = file_path.as_ref();
        
        // Read encrypted file contents
        let encrypted_data = fs::read(file_path)
            .map_err(|_| EncryptionError::DecryptionFailed { 
                reason: format!("Failed to read encrypted file: {}", file_path.display()) 
            })?;
        
        // Decrypt data
        let decrypted_data = self.decrypt(&encrypted_data)?;
        
        // Write decrypted data back
        fs::write(file_path, decrypted_data)
            .map_err(|_| EncryptionError::DecryptionFailed { 
                reason: format!("Failed to write decrypted file: {}", file_path.display()) 
            })?;
        
        Ok(())
    }
    
    /// Rotate encryption keys
    pub fn rotate_keys(&mut self) -> EncryptionResult<()> {
        tracing::info!("Rotating encryption keys");
        
        // Create new key
        self.create_new_key()?;
        
        // Clean up old keys if needed
        self.cleanup_old_keys()?;
        
        Ok(())
    }
    
    /// Clean up old or expired keys
    fn cleanup_old_keys(&mut self) -> EncryptionResult<()> {
        let now = chrono::Utc::now().timestamp() as u64;
        let mut expired_keys = Vec::new();
        
        // Find expired keys
        for (key_id, metadata) in &self.key_metadata {
            if let Some(expires_at) = metadata.expires_at {
                if expires_at < now {
                    expired_keys.push(key_id.clone());
                }
            }
        }
        
        // Remove expired keys
        for key_id in expired_keys {
            self.keys.remove(&key_id);
            self.key_metadata.remove(&key_id);
            tracing::info!("Removed expired key: {}", key_id);
        }
        
        Ok(())
    }
    
    /// Get current key ID
    pub fn current_key_id(&self) -> &str {
        &self.current_key_id
    }
    
    /// Get key metadata
    pub fn get_key_metadata(&self, key_id: &str) -> Option<&KeyMetadata> {
        self.key_metadata.get(key_id)
    }
    
    /// List all key IDs
    pub fn list_keys(&self) -> Vec<String> {
        self.keys.keys().cloned().collect()
    }
    
    /// Check if key rotation is needed
    pub fn needs_key_rotation(&self) -> bool {
        if self.config.key_rotation_days == 0 {
            return false;
        }
        
        if let Some(metadata) = self.key_metadata.get(&self.current_key_id) {
            let now = chrono::Utc::now().timestamp() as u64;
            let rotation_time = metadata.created_at + (self.config.key_rotation_days as u64 * 24 * 60 * 60);
            return now >= rotation_time;
        }
        
        false
    }
}

// Add base64 dependency
extern crate base64;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    fn test_config() -> EncryptionConfig {
        EncryptionConfig {
            enabled: true,
            algorithm: "AES-256-GCM".to_string(),
            kdf: "Argon2id".to_string(),
            kdf_iterations: 1000, // Lower for tests
            salt_size: 32,
            nonce_size: 12,
            key_rotation_days: 1,
            keychain_service: "com.chronicle.test".to_string(),
            keychain_account: "test-key".to_string(),
        }
    }
    
    #[test]
    fn test_encryption_service_creation() {
        let config = test_config();
        let service = EncryptionService::new(config);
        assert!(service.is_ok());
    }
    
    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let config = test_config();
        let mut service = EncryptionService::new(config).unwrap();
        
        let plaintext = b"Hello, Chronicle!";
        let encrypted = service.encrypt(plaintext).unwrap();
        let decrypted = service.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext, decrypted.as_slice());
    }
    
    #[test]
    fn test_file_encryption() {
        let config = test_config();
        let mut service = EncryptionService::new(config).unwrap();
        
        let temp_file = NamedTempFile::new().unwrap();
        let original_data = b"Test file content";
        
        // Write original data
        std::fs::write(temp_file.path(), original_data).unwrap();
        
        // Encrypt file
        service.encrypt_file(temp_file.path()).unwrap();
        
        // Verify file is encrypted (different from original)
        let encrypted_data = std::fs::read(temp_file.path()).unwrap();
        assert_ne!(original_data.as_slice(), encrypted_data.as_slice());
        
        // Decrypt file
        service.decrypt_file(temp_file.path()).unwrap();
        
        // Verify file is decrypted correctly
        let decrypted_data = std::fs::read(temp_file.path()).unwrap();
        assert_eq!(original_data.as_slice(), decrypted_data.as_slice());
    }
    
    #[test]
    fn test_key_rotation() {
        let config = test_config();
        let mut service = EncryptionService::new(config).unwrap();
        
        let original_key_id = service.current_key_id().to_string();
        
        // Rotate keys
        service.rotate_keys().unwrap();
        
        // Verify new key is different
        assert_ne!(original_key_id, service.current_key_id());
        
        // Verify we can still encrypt/decrypt
        let plaintext = b"Test after rotation";
        let encrypted = service.encrypt(plaintext).unwrap();
        let decrypted = service.decrypt(&encrypted).unwrap();
        assert_eq!(plaintext, decrypted.as_slice());
    }
}