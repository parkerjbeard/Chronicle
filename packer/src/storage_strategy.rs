// Future-proof storage partitioning strategy
use chrono::{DateTime, Utc, Datelike};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum PartitionStrategy {
    Daily,           // Current: /YYYY/MM/DD/
    Hourly,          // /YYYY/MM/DD/HH/
    EventType,       // /event_type/YYYY/MM/DD/
    Hybrid,          // /event_type/YYYY/MM/DD/HH/
    ShardBased,      // /shard_XXX/YYYY/MM/DD/
}

pub struct StorageLayout {
    pub base_path: PathBuf,
    pub strategy: PartitionStrategy,
    pub max_files_per_dir: usize,
    pub compression_enabled: bool,
}

impl StorageLayout {
    pub fn new(base_path: PathBuf, strategy: PartitionStrategy) -> Self {
        Self {
            base_path,
            strategy,
            max_files_per_dir: 1000, // Prevent filesystem performance issues
            compression_enabled: true,
        }
    }
    
    pub fn get_storage_path(&self, timestamp: DateTime<Utc>, event_type: &str) -> PathBuf {
        match self.strategy {
            PartitionStrategy::Daily => {
                self.base_path
                    .join(format!("{:04}", timestamp.year()))
                    .join(format!("{:02}", timestamp.month()))
                    .join(format!("{:02}", timestamp.day()))
            }
            PartitionStrategy::Hourly => {
                self.base_path
                    .join(format!("{:04}", timestamp.year()))
                    .join(format!("{:02}", timestamp.month()))
                    .join(format!("{:02}", timestamp.day()))
                    .join(format!("{:02}", timestamp.hour()))
            }
            PartitionStrategy::EventType => {
                self.base_path
                    .join(event_type)
                    .join(format!("{:04}", timestamp.year()))
                    .join(format!("{:02}", timestamp.month()))
                    .join(format!("{:02}", timestamp.day()))
            }
            PartitionStrategy::Hybrid => {
                self.base_path
                    .join(event_type)
                    .join(format!("{:04}", timestamp.year()))
                    .join(format!("{:02}", timestamp.month()))
                    .join(format!("{:02}", timestamp.day()))
                    .join(format!("{:02}", timestamp.hour()))
            }
            PartitionStrategy::ShardBased => {
                let shard_id = self.calculate_shard(timestamp, event_type);
                self.base_path
                    .join(format!("shard_{:03}", shard_id))
                    .join(format!("{:04}", timestamp.year()))
                    .join(format!("{:02}", timestamp.month()))
                    .join(format!("{:02}", timestamp.day()))
            }
        }
    }
    
    fn calculate_shard(&self, timestamp: DateTime<Utc>, event_type: &str) -> u32 {
        // Simple hash-based sharding
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        
        event_type.hash(&mut hasher);
        timestamp.timestamp().hash(&mut hasher);
        
        (hasher.finish() % 256) as u32 // 256 shards
    }
    
    pub fn should_rotate(&self, current_dir: &PathBuf) -> bool {
        // Check if we need to rotate based on file count
        if let Ok(entries) = std::fs::read_dir(current_dir) {
            let file_count = entries.count();
            file_count > self.max_files_per_dir
        } else {
            false
        }
    }
}