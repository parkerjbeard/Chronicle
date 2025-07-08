use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use anyhow::{Result, anyhow};
use aes_gcm::{Aes256Gcm, Key, Nonce, NewAead, AeadInPlace};
use argon2::{Argon2, PasswordHasher, PasswordHash, PasswordVerifier, password_hash::{SaltString, rand_core::OsRng}};
use rand::RngCore;
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationPolicy {
    pub master_key_rotation_days: u32,
    pub data_key_rotation_days: u32,
    pub automatic_rotation: bool,
    pub backup_old_keys: bool,
    pub max_key_history: u32,
}

impl Default for KeyRotationPolicy {
    fn default() -> Self {
        Self {
            master_key_rotation_days: 30,  // Monthly master key rotation
            data_key_rotation_days: 7,     // Weekly data key rotation
            automatic_rotation: true,
            backup_old_keys: true,
            max_key_history: 12,           // Keep 12 generations
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    pub key_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub key_type: KeyType,
    pub generation: u32,
    pub is_active: bool,
    pub algorithm: String,
    pub usage_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyType {
    Master,
    DataKey(String), // Data type name
    Archive,         // For old data access
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedKey {
    pub metadata: KeyMetadata,
    pub encrypted_key: Vec<u8>,
    pub salt: Vec<u8>,
    pub nonce: Vec<u8>,
    pub checksum: String,
}

impl EncryptedKey {
    pub fn new(key_data: &[u8], metadata: KeyMetadata, master_password: &str) -> Result<Self> {
        // Generate salt and nonce
        let mut salt = vec![0u8; 32];
        let mut nonce_bytes = vec![0u8; 12];
        OsRng.fill_bytes(&mut salt);
        OsRng.fill_bytes(&mut nonce_bytes);
        
        // Derive encryption key from password
        let argon2 = Argon2::default();
        let salt_string = SaltString::encode_b64(&salt).map_err(|e| anyhow!("Salt encoding error: {}", e))?;
        let password_hash = argon2.hash_password(master_password.as_bytes(), &salt_string)
            .map_err(|e| anyhow!("Password hashing error: {}", e))?;
        
        // Extract the hash as the encryption key
        let derived_key = password_hash.hash.ok_or_else(|| anyhow!("No hash in password hash"))?;
        let key = Key::from_slice(&derived_key.as_bytes()[..32]);
        
        // Encrypt the key data
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let mut encrypted_data = key_data.to_vec();
        cipher.encrypt_in_place(nonce, b"", &mut encrypted_data)
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;
        
        // Calculate checksum
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&encrypted_data);
        let checksum = format!("{:x}", hasher.finalize());
        
        Ok(Self {
            metadata,
            encrypted_key: encrypted_data,
            salt,
            nonce: nonce_bytes,
            checksum,
        })
    }
    
    pub fn decrypt(&self, master_password: &str) -> Result<Vec<u8>> {
        // Derive decryption key
        let argon2 = Argon2::default();
        let salt_string = SaltString::encode_b64(&self.salt).map_err(|e| anyhow!("Salt encoding error: {}", e))?;
        let password_hash = argon2.hash_password(master_password.as_bytes(), &salt_string)
            .map_err(|e| anyhow!("Password hashing error: {}", e))?;
        
        let derived_key = password_hash.hash.ok_or_else(|| anyhow!("No hash in password hash"))?;
        let key = Key::from_slice(&derived_key.as_bytes()[..32]);
        
        // Verify checksum first
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&self.encrypted_key);
        let calculated_checksum = format!("{:x}", hasher.finalize());
        
        if calculated_checksum != self.checksum {
            return Err(anyhow!("Key checksum verification failed"));
        }
        
        // Decrypt
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&self.nonce);
        
        let mut decrypted_data = self.encrypted_key.clone();
        cipher.decrypt_in_place(nonce, b"", &mut decrypted_data)
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;
        
        Ok(decrypted_data)
    }
    
    pub fn verify_integrity(&self) -> bool {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&self.encrypted_key);
        let calculated_checksum = format!("{:x}", hasher.finalize());
        calculated_checksum == self.checksum
    }
}

pub struct KeyManager {
    key_store_path: PathBuf,
    policy: KeyRotationPolicy,
    active_keys: HashMap<String, EncryptedKey>,
    key_history: HashMap<String, Vec<EncryptedKey>>,
    master_password: Option<String>,
}

impl KeyManager {
    pub fn new(key_store_path: PathBuf, policy: KeyRotationPolicy) -> Self {
        Self {
            key_store_path,
            policy,
            active_keys: HashMap::new(),
            key_history: HashMap::new(),
            master_password: None,
        }
    }
    
    pub fn unlock(&mut self, master_password: String) -> Result<()> {
        self.master_password = Some(master_password);
        self.load_keys()?;
        Ok(())
    }
    
    pub fn lock(&mut self) {
        self.master_password = None;
        self.active_keys.clear();
        self.key_history.clear();
    }
    
    pub fn is_unlocked(&self) -> bool {
        self.master_password.is_some()
    }
    
    fn ensure_unlocked(&self) -> Result<&str> {
        self.master_password.as_ref()
            .map(|s| s.as_str())
            .ok_or_else(|| anyhow!("Key manager is locked"))
    }
    
    pub fn generate_master_key(&mut self) -> Result<String> {
        let password = self.ensure_unlocked()?;
        
        // Generate new master key
        let mut key_data = vec![0u8; 32];
        OsRng.fill_bytes(&mut key_data);
        
        let key_id = format!("master_{}", Utc::now().timestamp());
        let metadata = KeyMetadata {
            key_id: key_id.clone(),
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + Duration::days(self.policy.master_key_rotation_days as i64)),
            key_type: KeyType::Master,
            generation: self.get_next_generation(&KeyType::Master),
            is_active: true,
            algorithm: "AES-256-GCM".to_string(),
            usage_count: 0,
        };
        
        let encrypted_key = EncryptedKey::new(&key_data, metadata, password)?;
        
        // Deactivate old master key
        if let Some(old_key) = self.active_keys.get_mut("master") {
            old_key.metadata.is_active = false;
            
            // Move to history if backup is enabled
            if self.policy.backup_old_keys {
                self.add_to_history("master", old_key.clone());
            }
        }
        
        self.active_keys.insert("master".to_string(), encrypted_key);
        self.save_keys()?;
        
        Ok(key_id)
    }
    
    pub fn generate_data_key(&mut self, data_type: &str) -> Result<String> {
        let password = self.ensure_unlocked()?;
        
        // Generate new data key
        let mut key_data = vec![0u8; 32];
        OsRng.fill_bytes(&mut key_data);
        
        let key_id = format!("{}_{}", data_type, Utc::now().timestamp());
        let metadata = KeyMetadata {
            key_id: key_id.clone(),
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + Duration::days(self.policy.data_key_rotation_days as i64)),
            key_type: KeyType::DataKey(data_type.to_string()),
            generation: self.get_next_generation(&KeyType::DataKey(data_type.to_string())),
            is_active: true,
            algorithm: "AES-256-GCM".to_string(),
            usage_count: 0,
        };
        
        let encrypted_key = EncryptedKey::new(&key_data, metadata, password)?;
        
        // Handle old key
        let key_name = format!("data_{}", data_type);
        if let Some(old_key) = self.active_keys.get_mut(&key_name) {
            old_key.metadata.is_active = false;
            
            if self.policy.backup_old_keys {
                self.add_to_history(&key_name, old_key.clone());
            }
        }
        
        self.active_keys.insert(key_name, encrypted_key);
        self.save_keys()?;
        
        Ok(key_id)
    }
    
    pub fn get_active_key(&mut self, key_type: &KeyType) -> Result<Vec<u8>> {
        let password = self.ensure_unlocked()?;
        
        let key_name = match key_type {
            KeyType::Master => "master".to_string(),
            KeyType::DataKey(data_type) => format!("data_{}", data_type),
            KeyType::Archive => return Err(anyhow!("Cannot get active archive key")),
        };
        
        let encrypted_key = self.active_keys.get_mut(&key_name)
            .ok_or_else(|| anyhow!("No active key found for type: {:?}", key_type))?;
        
        // Check if key needs rotation
        if self.needs_rotation(&encrypted_key.metadata) {
            match key_type {
                KeyType::Master => { self.generate_master_key()?; }
                KeyType::DataKey(data_type) => { self.generate_data_key(data_type)?; }
                KeyType::Archive => {}
            }
            
            // Get the new key
            let encrypted_key = self.active_keys.get_mut(&key_name)
                .ok_or_else(|| anyhow!("Failed to get rotated key"))?;
        }
        
        // Increment usage count
        encrypted_key.metadata.usage_count += 1;
        
        encrypted_key.decrypt(password)
    }
    
    pub fn get_historical_key(&self, key_id: &str) -> Result<Vec<u8>> {
        let password = self.ensure_unlocked()?;
        
        // Check active keys first
        for encrypted_key in self.active_keys.values() {
            if encrypted_key.metadata.key_id == key_id {
                return encrypted_key.decrypt(password);
            }
        }
        
        // Check history
        for history in self.key_history.values() {
            for encrypted_key in history {
                if encrypted_key.metadata.key_id == key_id {
                    return encrypted_key.decrypt(password);
                }
            }
        }
        
        Err(anyhow!("Key not found: {}", key_id))
    }
    
    pub fn rotate_all_keys(&mut self) -> Result<Vec<String>> {
        let mut rotated_keys = Vec::new();
        
        // Rotate master key
        let master_id = self.generate_master_key()?;
        rotated_keys.push(master_id);
        
        // Rotate all data keys
        let data_key_names: Vec<String> = self.active_keys.keys()
            .filter(|k| k.starts_with("data_"))
            .map(|k| k.strip_prefix("data_").unwrap().to_string())
            .collect();
        
        for data_type in data_key_names {
            let data_id = self.generate_data_key(&data_type)?;
            rotated_keys.push(data_id);
        }
        
        Ok(rotated_keys)
    }
    
    pub fn emergency_key_rotation(&mut self) -> Result<Vec<String>> {
        // Immediately rotate all keys regardless of schedule
        self.rotate_all_keys()
    }
    
    pub fn backup_keys(&self, backup_path: &Path) -> Result<()> {
        let password = self.ensure_unlocked()?;
        
        #[derive(Serialize)]
        struct KeyBackup {
            timestamp: DateTime<Utc>,
            active_keys: HashMap<String, EncryptedKey>,
            key_history: HashMap<String, Vec<EncryptedKey>>,
            policy: KeyRotationPolicy,
        }
        
        let backup = KeyBackup {
            timestamp: Utc::now(),
            active_keys: self.active_keys.clone(),
            key_history: self.key_history.clone(),
            policy: self.policy.clone(),
        };
        
        let backup_json = serde_json::to_string_pretty(&backup)?;
        
        // Encrypt the backup
        let mut backup_data = backup_json.into_bytes();
        let salt = self.generate_backup_salt()?;
        let backup_key = self.derive_backup_key(password, &salt)?;
        let cipher = Aes256Gcm::new(Key::from_slice(&backup_key));
        
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        cipher.encrypt_in_place(nonce, b"", &mut backup_data)
            .map_err(|e| anyhow!("Backup encryption failed: {}", e))?;
        
        // Write backup file with salt + nonce + encrypted data
        let mut final_backup = salt;
        final_backup.extend_from_slice(&nonce_bytes);
        final_backup.extend_from_slice(&backup_data);
        
        fs::write(backup_path, final_backup)?;
        
        Ok(())
    }
    
    pub fn restore_from_backup(&mut self, backup_path: &Path) -> Result<()> {
        let password = self.ensure_unlocked()?;
        
        let backup_data = fs::read(backup_path)?;
        
        if backup_data.len() < 44 { // 32 bytes salt + 12 bytes nonce minimum
            return Err(anyhow!("Invalid backup file: too small"));
        }
        
        // Extract salt (first 32 bytes), nonce (next 12 bytes), and encrypted data
        let (salt, remainder) = backup_data.split_at(32);
        let (nonce_bytes, encrypted_data) = remainder.split_at(12);
        let backup_key = self.derive_backup_key(password, salt)?;
        let cipher = Aes256Gcm::new(Key::from_slice(&backup_key));
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let mut decrypted_data = encrypted_data.to_vec();
        cipher.decrypt_in_place(nonce, b"", &mut decrypted_data)
            .map_err(|e| anyhow!("Backup decryption failed: {}", e))?;
        
        let backup_json = String::from_utf8(decrypted_data)?;
        
        #[derive(Deserialize)]
        struct KeyBackup {
            active_keys: HashMap<String, EncryptedKey>,
            key_history: HashMap<String, Vec<EncryptedKey>>,
            policy: KeyRotationPolicy,
        }
        
        let backup: KeyBackup = serde_json::from_str(&backup_json)?;
        
        self.active_keys = backup.active_keys;
        self.key_history = backup.key_history;
        self.policy = backup.policy;
        
        Ok(())
    }
    
    pub fn verify_all_keys(&self) -> Result<HashMap<String, bool>> {
        let mut verification_results = HashMap::new();
        
        // Verify active keys
        for (key_name, encrypted_key) in &self.active_keys {
            verification_results.insert(
                format!("active_{}", key_name),
                encrypted_key.verify_integrity()
            );
        }
        
        // Verify historical keys
        for (key_name, history) in &self.key_history {
            for (index, encrypted_key) in history.iter().enumerate() {
                verification_results.insert(
                    format!("history_{}_{}", key_name, index),
                    encrypted_key.verify_integrity()
                );
            }
        }
        
        Ok(verification_results)
    }
    
    pub fn get_key_status(&self) -> HashMap<String, KeyStatus> {
        let mut status = HashMap::new();
        
        for (key_name, encrypted_key) in &self.active_keys {
            status.insert(key_name.clone(), KeyStatus {
                key_id: encrypted_key.metadata.key_id.clone(),
                created_at: encrypted_key.metadata.created_at,
                expires_at: encrypted_key.metadata.expires_at,
                is_active: encrypted_key.metadata.is_active,
                needs_rotation: self.needs_rotation(&encrypted_key.metadata),
                usage_count: encrypted_key.metadata.usage_count,
                integrity_ok: encrypted_key.verify_integrity(),
            });
        }
        
        status
    }
    
    fn needs_rotation(&self, metadata: &KeyMetadata) -> bool {
        if !self.policy.automatic_rotation {
            return false;
        }
        
        if let Some(expires_at) = metadata.expires_at {
            return Utc::now() >= expires_at;
        }
        
        false
    }
    
    fn get_next_generation(&self, key_type: &KeyType) -> u32 {
        let key_name = match key_type {
            KeyType::Master => "master".to_string(),
            KeyType::DataKey(data_type) => format!("data_{}", data_type),
            KeyType::Archive => "archive".to_string(),
        };
        
        let current_gen = self.active_keys.get(&key_name)
            .map(|k| k.metadata.generation)
            .unwrap_or(0);
        
        let history_gen = self.key_history.get(&key_name)
            .and_then(|h| h.iter().map(|k| k.metadata.generation).max())
            .unwrap_or(0);
        
        std::cmp::max(current_gen, history_gen) + 1
    }
    
    fn add_to_history(&mut self, key_name: &str, encrypted_key: EncryptedKey) {
        let history = self.key_history.entry(key_name.to_string()).or_insert_with(Vec::new);
        history.push(encrypted_key);
        
        // Limit history size
        if history.len() > self.policy.max_key_history as usize {
            history.remove(0);
        }
    }
    
    /// Derive backup key using secure random salt (FIXED: no longer uses hardcoded salt)
    fn derive_backup_key(&self, password: &str, salt: &[u8]) -> Result<Vec<u8>> {
        let argon2 = Argon2::default();
        let salt_string = SaltString::encode_b64(salt).map_err(|e| anyhow!("Salt encoding error: {}", e))?;
        let password_hash = argon2.hash_password(password.as_bytes(), &salt_string)
            .map_err(|e| anyhow!("Password hashing error: {}", e))?;
        
        let derived_key = password_hash.hash.ok_or_else(|| anyhow!("No hash in password hash"))?;
        Ok(derived_key.as_bytes()[..32].to_vec())
    }
    
    /// Generate cryptographically secure random salt for backup key derivation
    fn generate_backup_salt(&self) -> Result<Vec<u8>> {
        let mut salt = vec![0u8; 32]; // 256-bit salt
        OsRng.fill_bytes(&mut salt);
        Ok(salt)
    }
    
    fn load_keys(&mut self) -> Result<()> {
        let keys_file = self.key_store_path.join("keys.json");
        
        if !keys_file.exists() {
            // No existing keys, start fresh
            return Ok(());
        }
        
        let data = fs::read_to_string(keys_file)?;
        
        #[derive(Deserialize)]
        struct KeyStore {
            active_keys: HashMap<String, EncryptedKey>,
            key_history: HashMap<String, Vec<EncryptedKey>>,
        }
        
        let store: KeyStore = serde_json::from_str(&data)?;
        self.active_keys = store.active_keys;
        self.key_history = store.key_history;
        
        Ok(())
    }
    
    fn save_keys(&self) -> Result<()> {
        fs::create_dir_all(&self.key_store_path)?;
        
        #[derive(Serialize)]
        struct KeyStore {
            active_keys: HashMap<String, EncryptedKey>,
            key_history: HashMap<String, Vec<EncryptedKey>>,
        }
        
        let store = KeyStore {
            active_keys: self.active_keys.clone(),
            key_history: self.key_history.clone(),
        };
        
        let data = serde_json::to_string_pretty(&store)?;
        let keys_file = self.key_store_path.join("keys.json");
        
        fs::write(keys_file, data)?;
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyStatus {
    pub key_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub needs_rotation: bool,
    pub usage_count: u64,
    pub integrity_ok: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_key_encryption_decryption() {
        let key_data = b"test_key_data_32_bytes_exactly!";
        let password = "test_password";
        
        let metadata = KeyMetadata {
            key_id: "test_key".to_string(),
            created_at: Utc::now(),
            expires_at: None,
            key_type: KeyType::Master,
            generation: 1,
            is_active: true,
            algorithm: "AES-256-GCM".to_string(),
            usage_count: 0,
        };
        
        let encrypted_key = EncryptedKey::new(key_data, metadata, password).unwrap();
        assert!(encrypted_key.verify_integrity());
        
        let decrypted = encrypted_key.decrypt(password).unwrap();
        assert_eq!(decrypted, key_data);
        
        // Test wrong password
        assert!(encrypted_key.decrypt("wrong_password").is_err());
    }
    
    #[test]
    fn test_key_manager_basic_operations() {
        let temp_dir = TempDir::new().unwrap();
        let policy = KeyRotationPolicy::default();
        let mut manager = KeyManager::new(temp_dir.path().to_path_buf(), policy);
        
        // Unlock
        manager.unlock("test_password".to_string()).unwrap();
        assert!(manager.is_unlocked());
        
        // Generate master key
        let master_id = manager.generate_master_key().unwrap();
        assert!(!master_id.is_empty());
        
        // Generate data key
        let data_id = manager.generate_data_key("events").unwrap();
        assert!(!data_id.is_empty());
        
        // Get keys
        let master_key = manager.get_active_key(&KeyType::Master).unwrap();
        assert_eq!(master_key.len(), 32);
        
        let data_key = manager.get_active_key(&KeyType::DataKey("events".to_string())).unwrap();
        assert_eq!(data_key.len(), 32);
        
        // Lock and verify
        manager.lock();
        assert!(!manager.is_unlocked());
        assert!(manager.get_active_key(&KeyType::Master).is_err());
    }
    
    #[test]
    fn test_key_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let mut policy = KeyRotationPolicy::default();
        policy.master_key_rotation_days = 0; // Force immediate rotation
        
        let mut manager = KeyManager::new(temp_dir.path().to_path_buf(), policy);
        manager.unlock("test_password".to_string()).unwrap();
        
        // Generate initial key
        let initial_id = manager.generate_master_key().unwrap();
        
        // Force rotation by getting key (should auto-rotate due to 0-day policy)
        let _key1 = manager.get_active_key(&KeyType::Master).unwrap();
        let status = manager.get_key_status();
        
        // Should have rotated automatically
        let master_status = status.get("master").unwrap();
        assert_ne!(master_status.key_id, initial_id);
    }
    
    #[test]
    fn test_key_backup_restore() {
        let temp_dir = TempDir::new().unwrap();
        let policy = KeyRotationPolicy::default();
        let mut manager1 = KeyManager::new(temp_dir.path().to_path_buf(), policy.clone());
        
        manager1.unlock("test_password".to_string()).unwrap();
        manager1.generate_master_key().unwrap();
        manager1.generate_data_key("events").unwrap();
        
        // Backup keys
        let backup_path = temp_dir.path().join("backup.enc");
        manager1.backup_keys(&backup_path).unwrap();
        
        // Create new manager and restore
        let temp_dir2 = TempDir::new().unwrap();
        let mut manager2 = KeyManager::new(temp_dir2.path().to_path_buf(), policy);
        manager2.unlock("test_password".to_string()).unwrap();
        manager2.restore_from_backup(&backup_path).unwrap();
        
        // Verify keys match
        let key1 = manager1.get_active_key(&KeyType::Master).unwrap();
        let key2 = manager2.get_active_key(&KeyType::Master).unwrap();
        assert_eq!(key1, key2);
    }
    
    #[test]
    fn test_key_verification() {
        let temp_dir = TempDir::new().unwrap();
        let policy = KeyRotationPolicy::default();
        let mut manager = KeyManager::new(temp_dir.path().to_path_buf(), policy);
        
        manager.unlock("test_password".to_string()).unwrap();
        manager.generate_master_key().unwrap();
        manager.generate_data_key("events").unwrap();
        
        let verification_results = manager.verify_all_keys().unwrap();
        
        // All keys should verify successfully
        for (key_name, is_valid) in verification_results {
            assert!(is_valid, "Key {} failed verification", key_name);
        }
    }
}