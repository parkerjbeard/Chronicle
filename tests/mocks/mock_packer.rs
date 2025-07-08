use std::sync::Arc;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex};
use tokio::time::interval;
use anyhow::Result;
use serde_json::Value;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

use crate::mocks::MockRingBuffer;
use crate::utils::TestEvent;

/// Mock packer for testing data processing pipeline
pub struct MockPacker {
    id: String,
    ring_buffer: Arc<RwLock<MockRingBuffer>>,
    storage_path: PathBuf,
    config: PackerConfig,
    stats: Arc<Mutex<PackerStats>>,
    running: Arc<Mutex<bool>>,
}

#[derive(Clone)]
pub struct PackerConfig {
    pub batch_size: usize,
    pub processing_interval: Duration,
    pub compression_enabled: bool,
    pub compression_level: u32,
    pub encryption_enabled: bool,
    pub integrity_checks: bool,
    pub max_file_size_mb: usize,
    pub error_rate: f64,
    pub processing_delay: Duration,
}

#[derive(Default, Clone)]
pub struct PackerStats {
    pub batches_processed: u64,
    pub events_processed: u64,
    pub files_created: u64,
    pub compression_ratio: f64,
    pub processing_errors: u64,
    pub total_processing_time: Duration,
    pub average_batch_time: Duration,
    pub bytes_written: u64,
    pub integrity_checks_passed: u64,
    pub integrity_checks_failed: u64,
}

impl MockPacker {
    pub fn new(
        ring_buffer: Arc<RwLock<MockRingBuffer>>,
        storage_path: PathBuf,
    ) -> Result<Self> {
        // Create storage directory if it doesn't exist
        std::fs::create_dir_all(&storage_path)?;
        
        Ok(Self {
            id: "mock_packer".to_string(),
            ring_buffer,
            storage_path,
            config: PackerConfig::default(),
            stats: Arc::new(Mutex::new(PackerStats::default())),
            running: Arc::new(Mutex::new(false)),
        })
    }

    pub fn set_id(&mut self, id: String) {
        self.id = id;
    }

    pub fn set_batch_size(&mut self, size: usize) {
        self.config.batch_size = size;
    }

    pub fn enable_compression(&mut self, enabled: bool) {
        self.config.compression_enabled = enabled;
    }

    pub fn set_compression_level(&mut self, level: u32) {
        self.config.compression_level = level;
    }

    pub fn enable_encryption(&mut self, enabled: bool) {
        self.config.encryption_enabled = enabled;
    }

    pub fn enable_integrity_checks(&mut self, enabled: bool) {
        self.config.integrity_checks = enabled;
    }

    pub fn enable_error_simulation(&mut self, error_rate: f64) {
        self.config.error_rate = error_rate;
    }

    pub fn set_processing_delay(&mut self, delay: Duration) {
        self.config.processing_delay = delay;
    }

    pub fn is_initialized(&self) -> bool {
        self.storage_path.exists()
    }

    pub async fn start_packing(&self) -> Result<()> {
        {
            let mut running = self.running.lock().await;
            *running = true;
        }

        let mut interval = interval(self.config.processing_interval);

        while *self.running.lock().await {
            interval.tick().await;

            // Read batch from ring buffer
            let events = {
                let mut buffer = self.ring_buffer.write().await;
                buffer.read_events(self.config.batch_size).await?
            };

            if !events.is_empty() {
                self.process_batch(events).await?;
            }

            // Simulate processing delay
            if !self.config.processing_delay.is_zero() {
                tokio::time::sleep(self.config.processing_delay).await;
            }
        }

        Ok(())
    }

    pub async fn stop_packing(&self) -> Result<()> {
        let mut running = self.running.lock().await;
        *running = false;
        Ok(())
    }

    async fn process_batch(&self, events: Vec<TestEvent>) -> Result<()> {
        let start_time = Instant::now();

        // Simulate error conditions
        if self.should_simulate_error().await {
            let mut stats = self.stats.lock().await;
            stats.processing_errors += 1;
            return Err(anyhow::anyhow!("Simulated processing error"));
        }

        // Convert events to processable format
        let serialized_data = self.serialize_events(&events)?;

        // Apply compression if enabled
        let processed_data = if self.config.compression_enabled {
            self.compress_data(&serialized_data)?
        } else {
            serialized_data
        };

        // Apply encryption if enabled
        let final_data = if self.config.encryption_enabled {
            self.encrypt_data(&processed_data)?
        } else {
            processed_data
        };

        // Validate integrity if enabled
        if self.config.integrity_checks {
            self.validate_integrity(&events, &final_data).await?;
        }

        // Write to storage
        let file_path = self.generate_file_path().await;
        self.write_to_storage(&file_path, &final_data).await?;

        // Create metadata file
        self.create_metadata_file(&file_path, &events, &final_data).await?;

        // Update statistics
        let processing_time = start_time.elapsed();
        self.update_stats(&events, &serialized_data, &final_data, processing_time).await;

        Ok(())
    }

    fn serialize_events(&self, events: &[TestEvent]) -> Result<Vec<u8>> {
        // Simulate different serialization formats
        let serialized = serde_json::to_vec(events)?;
        Ok(serialized)
    }

    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.config.compression_level));
        encoder.write_all(data)?;
        let compressed = encoder.finish()?;
        Ok(compressed)
    }

    fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Simulate encryption (in real implementation, would use actual encryption)
        let mut encrypted = data.to_vec();
        
        // Simple XOR "encryption" for testing
        let key = 0x42u8;
        for byte in &mut encrypted {
            *byte ^= key;
        }
        
        Ok(encrypted)
    }

    async fn validate_integrity(&self, events: &[TestEvent], processed_data: &[u8]) -> Result<()> {
        // Simulate integrity validation
        let expected_checksum = self.calculate_checksum(events);
        let actual_checksum = self.calculate_data_checksum(processed_data);

        let mut stats = self.stats.lock().await;
        
        if expected_checksum == actual_checksum {
            stats.integrity_checks_passed += 1;
        } else {
            stats.integrity_checks_failed += 1;
            return Err(anyhow::anyhow!("Integrity check failed"));
        }

        Ok(())
    }

    fn calculate_checksum(&self, events: &[TestEvent]) -> String {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        for event in events {
            hasher.update(event.id.to_string().as_bytes());
            hasher.update(event.event_type.as_bytes());
        }
        
        format!("{:x}", hasher.finalize())
    }

    fn calculate_data_checksum(&self, data: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    async fn generate_file_path(&self) -> PathBuf {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f");
        let filename = format!("chronicle_{}_{}.dat", self.id, timestamp);
        self.storage_path.join(filename)
    }

    async fn write_to_storage(&self, file_path: &PathBuf, data: &[u8]) -> Result<()> {
        tokio::fs::write(file_path, data).await?;
        Ok(())
    }

    async fn create_metadata_file(
        &self,
        data_file_path: &PathBuf,
        events: &[TestEvent],
        processed_data: &[u8],
    ) -> Result<()> {
        let metadata = serde_json::json!({
            "data_file": data_file_path.file_name().unwrap().to_string_lossy(),
            "event_count": events.len(),
            "original_size": self.calculate_original_size(events),
            "compressed_size": processed_data.len(),
            "compression_ratio": if self.config.compression_enabled {
                processed_data.len() as f64 / self.calculate_original_size(events) as f64
            } else {
                1.0
            },
            "encrypted": self.config.encryption_enabled,
            "integrity_verified": self.config.integrity_checks,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "packer_id": self.id,
            "checksum": self.calculate_data_checksum(processed_data)
        });

        let metadata_path = data_file_path.with_extension("json");
        let metadata_content = serde_json::to_string_pretty(&metadata)?;
        tokio::fs::write(&metadata_path, metadata_content).await?;

        Ok(())
    }

    fn calculate_original_size(&self, events: &[TestEvent]) -> usize {
        events.iter().map(|e| e.size_bytes()).sum()
    }

    async fn update_stats(
        &self,
        events: &[TestEvent],
        original_data: &[u8],
        final_data: &[u8],
        processing_time: Duration,
    ) {
        let mut stats = self.stats.lock().await;
        
        stats.batches_processed += 1;
        stats.events_processed += events.len() as u64;
        stats.files_created += 1;
        stats.bytes_written += final_data.len() as u64;
        stats.total_processing_time += processing_time;
        stats.average_batch_time = stats.total_processing_time / stats.batches_processed as u32;

        if self.config.compression_enabled && !original_data.is_empty() {
            stats.compression_ratio = final_data.len() as f64 / original_data.len() as f64;
        }
    }

    async fn should_simulate_error(&self) -> bool {
        rand::random::<f64>() < self.config.error_rate
    }

    pub async fn get_stats(&self) -> PackerStats {
        self.stats.lock().await.clone()
    }

    /// Force process all remaining events in ring buffer
    pub async fn flush(&self) -> Result<()> {
        loop {
            let events = {
                let mut buffer = self.ring_buffer.write().await;
                buffer.read_events(self.config.batch_size).await?
            };

            if events.is_empty() {
                break;
            }

            self.process_batch(events).await?;
        }

        Ok(())
    }

    /// Get list of created files
    pub async fn get_created_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.storage_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "dat") {
                files.push(path);
            }
        }

        files.sort();
        Ok(files)
    }

    /// Validate stored data integrity
    pub async fn validate_stored_data(&self) -> Result<ValidationReport> {
        let data_files = self.get_created_files().await?;
        let mut report = ValidationReport {
            total_files: data_files.len(),
            valid_files: 0,
            corrupted_files: 0,
            total_events: 0,
            validation_errors: Vec::new(),
        };

        for data_file in &data_files {
            match self.validate_single_file(data_file).await {
                Ok(event_count) => {
                    report.valid_files += 1;
                    report.total_events += event_count;
                }
                Err(e) => {
                    report.corrupted_files += 1;
                    report.validation_errors.push(format!(
                        "File {}: {}",
                        data_file.display(),
                        e
                    ));
                }
            }
        }

        Ok(report)
    }

    async fn validate_single_file(&self, file_path: &PathBuf) -> Result<usize> {
        // Read metadata
        let metadata_path = file_path.with_extension("json");
        let metadata_content = tokio::fs::read_to_string(&metadata_path).await?;
        let metadata: Value = serde_json::from_str(&metadata_content)?;

        // Read data file
        let data = tokio::fs::read(file_path).await?;

        // Verify checksum
        let expected_checksum = metadata["checksum"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing checksum in metadata"))?;
        let actual_checksum = self.calculate_data_checksum(&data);

        if expected_checksum != actual_checksum {
            return Err(anyhow::anyhow!("Checksum mismatch"));
        }

        // Return event count
        let event_count = metadata["event_count"].as_u64()
            .ok_or_else(|| anyhow::anyhow!("Missing event count in metadata"))? as usize;

        Ok(event_count)
    }
}

#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub total_files: usize,
    pub valid_files: usize,
    pub corrupted_files: usize,
    pub total_events: usize,
    pub validation_errors: Vec<String>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.corrupted_files == 0 && self.validation_errors.is_empty()
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_files == 0 {
            1.0
        } else {
            self.valid_files as f64 / self.total_files as f64
        }
    }
}

impl Default for PackerConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            processing_interval: Duration::from_millis(100),
            compression_enabled: true,
            compression_level: 6,
            encryption_enabled: false,
            integrity_checks: true,
            max_file_size_mb: 10,
            error_rate: 0.0,
            processing_delay: Duration::from_millis(0),
        }
    }
}

/// Factory for creating different types of mock packers
pub struct MockPackerFactory;

impl MockPackerFactory {
    pub fn create_basic_packer(
        ring_buffer: Arc<RwLock<MockRingBuffer>>,
        storage_path: PathBuf,
    ) -> Result<MockPacker> {
        MockPacker::new(ring_buffer, storage_path)
    }

    pub fn create_high_performance_packer(
        ring_buffer: Arc<RwLock<MockRingBuffer>>,
        storage_path: PathBuf,
    ) -> Result<MockPacker> {
        let mut packer = MockPacker::new(ring_buffer, storage_path)?;
        packer.set_batch_size(1000);
        packer.config.processing_interval = Duration::from_millis(10);
        packer.enable_compression(true);
        packer.set_compression_level(1); // Fast compression
        Ok(packer)
    }

    pub fn create_secure_packer(
        ring_buffer: Arc<RwLock<MockRingBuffer>>,
        storage_path: PathBuf,
    ) -> Result<MockPacker> {
        let mut packer = MockPacker::new(ring_buffer, storage_path)?;
        packer.enable_encryption(true);
        packer.enable_integrity_checks(true);
        packer.set_compression_level(9); // Best compression
        Ok(packer)
    }

    pub fn create_error_prone_packer(
        ring_buffer: Arc<RwLock<MockRingBuffer>>,
        storage_path: PathBuf,
        error_rate: f64,
    ) -> Result<MockPacker> {
        let mut packer = MockPacker::new(ring_buffer, storage_path)?;
        packer.enable_error_simulation(error_rate);
        Ok(packer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_mock_packer_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024)?));
        
        let packer = MockPacker::new(ring_buffer, temp_dir.path().to_path_buf())?;
        assert!(packer.is_initialized());
        
        Ok(())
    }

    #[tokio::test]
    async fn test_packer_processing() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024)?));
        let packer = MockPacker::new(ring_buffer.clone(), temp_dir.path().to_path_buf())?;

        // Add test events to ring buffer
        {
            let mut buffer = ring_buffer.write().await;
            for i in 0..10 {
                let event = TestEvent::new(
                    i,
                    "test_event",
                    serde_json::json!({"data": format!("test_{}", i)})
                );
                buffer.write_event(&event).await?;
            }
        }

        // Process events
        packer.flush().await?;

        // Verify files were created
        let files = packer.get_created_files().await?;
        assert!(!files.is_empty());

        // Validate stored data
        let validation = packer.validate_stored_data().await?;
        assert!(validation.is_valid());
        assert_eq!(validation.total_events, 10);

        Ok(())
    }

    #[tokio::test]
    async fn test_packer_factory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024)?));

        let basic = MockPackerFactory::create_basic_packer(
            ring_buffer.clone(),
            temp_dir.path().join("basic")
        )?;
        assert_eq!(basic.config.batch_size, 100);

        let high_perf = MockPackerFactory::create_high_performance_packer(
            ring_buffer.clone(),
            temp_dir.path().join("high_perf")
        )?;
        assert_eq!(high_perf.config.batch_size, 1000);

        let secure = MockPackerFactory::create_secure_packer(
            ring_buffer.clone(),
            temp_dir.path().join("secure")
        )?;
        assert!(secure.config.encryption_enabled);
        assert!(secure.config.integrity_checks);

        Ok(())
    }
}