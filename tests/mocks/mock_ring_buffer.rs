use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use std::path::PathBuf;

use crate::utils::TestEvent;

/// Mock ring buffer for testing
pub struct MockRingBuffer {
    capacity: usize,
    buffer: Arc<Mutex<VecDeque<TestEvent>>>,
    stats: Arc<Mutex<RingBufferStats>>,
    config: RingBufferConfig,
    persistence_path: Option<PathBuf>,
}

#[derive(Clone)]
pub struct RingBufferConfig {
    pub compression_level: u8,
    pub enable_persistence: bool,
    pub persistence_interval: Duration,
    pub memory_limit: usize,
    pub overflow_strategy: OverflowStrategy,
}

#[derive(Clone, Debug)]
pub enum OverflowStrategy {
    DropOldest,
    DropNewest,
    Block,
}

#[derive(Default, Clone, Debug)]
pub struct RingBufferStats {
    pub total_writes: u64,
    pub total_reads: u64,
    pub overflow_count: u64,
    pub buffer_utilization: f64,
    pub memory_usage: u64,
    pub compression_ratio: f64,
    pub concurrent_reads: u64,
    pub concurrent_writes: u64,
    pub deadlock_count: u64,
    pub memory_efficiency: f64,
    pub memory_leaks: u64,
    pub filtered_events: u64,
    pub oldest_event_dropped: bool,
    pub error_recovery_successful: bool,
    pub last_write_time: Option<Instant>,
    pub last_read_time: Option<Instant>,
}

#[derive(Serialize, Deserialize)]
struct PersistentState {
    events: Vec<TestEvent>,
    stats: RingBufferStats,
    timestamp: i64,
}

impl MockRingBuffer {
    pub fn new(capacity: usize) -> Result<Self> {
        Ok(Self {
            capacity,
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            stats: Arc::new(Mutex::new(RingBufferStats::default())),
            config: RingBufferConfig::default(),
            persistence_path: None,
        })
    }

    pub fn with_persistence(capacity: usize, persistence_path: PathBuf) -> Result<Self> {
        Ok(Self {
            capacity,
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            stats: Arc::new(Mutex::new(RingBufferStats::default())),
            config: RingBufferConfig {
                enable_persistence: true,
                ..Default::default()
            },
            persistence_path: Some(persistence_path),
        })
    }

    pub fn from_persistence(persistence_path: PathBuf) -> Result<Self> {
        let data = std::fs::read_to_string(&persistence_path)?;
        let state: PersistentState = serde_json::from_str(&data)?;
        
        let mut buffer = VecDeque::with_capacity(state.events.len());
        for event in state.events {
            buffer.push_back(event);
        }

        Ok(Self {
            capacity: buffer.capacity(),
            buffer: Arc::new(Mutex::new(buffer)),
            stats: Arc::new(Mutex::new(state.stats)),
            config: RingBufferConfig {
                enable_persistence: true,
                ..Default::default()
            },
            persistence_path: Some(persistence_path),
        })
    }

    pub async fn write_event(&self, event: &TestEvent) -> Result<()> {
        let mut buffer = self.buffer.lock().await;
        let mut stats = self.stats.lock().await;

        // Check if buffer is full
        if buffer.len() >= self.capacity {
            match self.config.overflow_strategy {
                OverflowStrategy::DropOldest => {
                    buffer.pop_front();
                    stats.overflow_count += 1;
                    stats.oldest_event_dropped = true;
                },
                OverflowStrategy::DropNewest => {
                    // Don't add new event
                    stats.overflow_count += 1;
                    return Ok(());
                },
                OverflowStrategy::Block => {
                    // In real implementation, this would block
                    // For testing, we'll just drop oldest
                    buffer.pop_front();
                    stats.overflow_count += 1;
                }
            }
        }

        buffer.push_back(event.clone());
        stats.total_writes += 1;
        stats.last_write_time = Some(Instant::now());
        stats.buffer_utilization = buffer.len() as f64 / self.capacity as f64;
        stats.memory_usage = self.calculate_memory_usage(&buffer);

        Ok(())
    }

    pub async fn read_events(&self, count: usize) -> Result<Vec<TestEvent>> {
        let mut buffer = self.buffer.lock().await;
        let mut stats = self.stats.lock().await;

        let mut events = Vec::new();
        for _ in 0..count {
            if let Some(event) = buffer.pop_front() {
                events.push(event);
            } else {
                break;
            }
        }

        stats.total_reads += events.len() as u64;
        stats.last_read_time = Some(Instant::now());
        stats.buffer_utilization = buffer.len() as f64 / self.capacity as f64;

        Ok(events)
    }

    pub async fn read_events_ordered(&self, count: usize) -> Result<Vec<TestEvent>> {
        let mut events = self.read_events(count).await?;
        events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(events)
    }

    pub async fn search_by_id(&self, id: u64) -> Result<Option<TestEvent>> {
        let buffer = self.buffer.lock().await;
        Ok(buffer.iter().find(|e| e.id == id).cloned())
    }

    pub async fn persist(&self) -> Result<()> {
        if let Some(path) = &self.persistence_path {
            let buffer = self.buffer.lock().await;
            let stats = self.stats.lock().await;

            let state = PersistentState {
                events: buffer.iter().cloned().collect(),
                stats: stats.clone(),
                timestamp: chrono::Utc::now().timestamp(),
            };

            let data = serde_json::to_string_pretty(&state)?;
            std::fs::write(path, data)?;
        }
        Ok(())
    }

    pub async fn compact(&self) -> Result<()> {
        let mut buffer = self.buffer.lock().await;
        let mut stats = self.stats.lock().await;

        // Simulate compaction by removing duplicate events
        let mut seen_ids = std::collections::HashSet::new();
        let mut compacted = VecDeque::new();

        for event in buffer.drain(..) {
            if seen_ids.insert(event.id) {
                compacted.push_back(event);
            }
        }

        *buffer = compacted;
        stats.memory_usage = self.calculate_memory_usage(&buffer);
        stats.memory_efficiency = 0.9; // Simulate good efficiency after compaction

        Ok(())
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub async fn get_stats(&self) -> RingBufferStats {
        self.stats.lock().await.clone()
    }

    pub fn get_memory_usage(&self) -> u64 {
        // Simulate memory usage calculation
        let base_usage = 1024 * 1024; // 1MB base
        let per_event = 100; // 100 bytes per event
        base_usage + (self.capacity * per_event) as u64
    }

    pub fn set_compression_level(&mut self, level: u8) {
        self.config.compression_level = level;
    }

    fn calculate_memory_usage(&self, buffer: &VecDeque<TestEvent>) -> u64 {
        let base_size = std::mem::size_of::<TestEvent>() * buffer.len();
        let data_size: usize = buffer.iter()
            .map(|e| e.data.to_string().len())
            .sum();
        (base_size + data_size) as u64
    }

    // Test helper methods
    pub async fn fill_to_capacity(&self) -> Result<()> {
        for i in 0..self.capacity {
            let event = TestEvent {
                id: i as u64,
                timestamp: chrono::Utc::now(),
                event_type: "fill_test".to_string(),
                data: serde_json::json!({"index": i}),
            };
            self.write_event(&event).await?;
        }
        Ok(())
    }

    pub async fn clear(&self) -> Result<()> {
        let mut buffer = self.buffer.lock().await;
        buffer.clear();
        Ok(())
    }

    pub async fn size(&self) -> usize {
        let buffer = self.buffer.lock().await;
        buffer.len()
    }

    pub async fn is_empty(&self) -> bool {
        let buffer = self.buffer.lock().await;
        buffer.is_empty()
    }

    pub async fn is_full(&self) -> bool {
        let buffer = self.buffer.lock().await;
        buffer.len() >= self.capacity
    }

    // Simulate concurrent access for testing
    pub async fn concurrent_write_test(&self, num_writers: usize, events_per_writer: usize) -> Result<()> {
        let handles = (0..num_writers).map(|writer_id| {
            let buffer = self.buffer.clone();
            let stats = self.stats.clone();
            let capacity = self.capacity;
            let config = self.config.clone();

            tokio::spawn(async move {
                for i in 0..events_per_writer {
                    let event = TestEvent {
                        id: (writer_id * events_per_writer + i) as u64,
                        timestamp: chrono::Utc::now(),
                        event_type: "concurrent_test".to_string(),
                        data: serde_json::json!({
                            "writer_id": writer_id,
                            "sequence": i
                        }),
                    };

                    let mut buffer = buffer.lock().await;
                    let mut stats = stats.lock().await;

                    if buffer.len() >= capacity {
                        match config.overflow_strategy {
                            OverflowStrategy::DropOldest => {
                                buffer.pop_front();
                                stats.overflow_count += 1;
                            },
                            _ => {}
                        }
                    }

                    buffer.push_back(event);
                    stats.total_writes += 1;
                    stats.concurrent_writes += 1;
                }
            })
        }).collect::<Vec<_>>();

        for handle in handles {
            handle.await?;
        }

        Ok(())
    }

    pub async fn concurrent_read_test(&self, num_readers: usize, reads_per_reader: usize) -> Result<Vec<usize>> {
        let mut results = Vec::new();

        let handles = (0..num_readers).map(|_| {
            let buffer = self.buffer.clone();
            let stats = self.stats.clone();

            tokio::spawn(async move {
                let mut total_read = 0;
                for _ in 0..reads_per_reader {
                    let mut buffer = buffer.lock().await;
                    let mut stats = stats.lock().await;

                    if let Some(_event) = buffer.pop_front() {
                        total_read += 1;
                        stats.total_reads += 1;
                        stats.concurrent_reads += 1;
                    }
                }
                total_read
            })
        }).collect::<Vec<_>>();

        for handle in handles {
            results.push(handle.await?);
        }

        Ok(results)
    }

    // Performance testing methods
    pub async fn benchmark_write_performance(&self, num_events: usize) -> Result<Duration> {
        let start = Instant::now();

        for i in 0..num_events {
            let event = TestEvent {
                id: i as u64,
                timestamp: chrono::Utc::now(),
                event_type: "benchmark".to_string(),
                data: serde_json::json!({"index": i}),
            };
            self.write_event(&event).await?;
        }

        Ok(start.elapsed())
    }

    pub async fn benchmark_read_performance(&self, num_reads: usize) -> Result<Duration> {
        // Fill buffer first
        self.fill_to_capacity().await?;

        let start = Instant::now();
        let mut total_read = 0;

        while total_read < num_reads {
            let events = self.read_events(100).await?;
            total_read += events.len();
            
            if events.is_empty() {
                break;
            }
        }

        Ok(start.elapsed())
    }
}

impl Default for RingBufferConfig {
    fn default() -> Self {
        Self {
            compression_level: 3,
            enable_persistence: false,
            persistence_interval: Duration::from_secs(60),
            memory_limit: 100 * 1024 * 1024, // 100MB
            overflow_strategy: OverflowStrategy::DropOldest,
        }
    }
}

/// Factory for creating different types of ring buffers for testing
pub struct MockRingBufferFactory;

impl MockRingBufferFactory {
    pub fn create_small_buffer() -> Result<MockRingBuffer> {
        MockRingBuffer::new(100)
    }

    pub fn create_medium_buffer() -> Result<MockRingBuffer> {
        MockRingBuffer::new(10000)
    }

    pub fn create_large_buffer() -> Result<MockRingBuffer> {
        MockRingBuffer::new(1000000)
    }

    pub fn create_persistent_buffer(path: PathBuf) -> Result<MockRingBuffer> {
        MockRingBuffer::with_persistence(50000, path)
    }

    pub fn create_overflow_test_buffer() -> Result<MockRingBuffer> {
        let mut buffer = MockRingBuffer::new(10)?;
        buffer.config.overflow_strategy = OverflowStrategy::DropOldest;
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_ring_buffer_basic_operations() -> Result<()> {
        let buffer = MockRingBuffer::new(100)?;
        
        let event = TestEvent {
            id: 1,
            timestamp: chrono::Utc::now(),
            event_type: "test".to_string(),
            data: serde_json::json!({"test": true}),
        };

        buffer.write_event(&event).await?;
        let events = buffer.read_events(1).await?;
        
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, 1);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_ring_buffer_overflow() -> Result<()> {
        let buffer = MockRingBuffer::new(2)?;
        
        // Fill buffer beyond capacity
        for i in 0..5 {
            let event = TestEvent {
                id: i,
                timestamp: chrono::Utc::now(),
                event_type: "test".to_string(),
                data: serde_json::json!({"index": i}),
            };
            buffer.write_event(&event).await?;
        }

        let stats = buffer.get_stats().await;
        assert!(stats.overflow_count > 0);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_ring_buffer_persistence() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let persistence_path = temp_dir.path().join("test_buffer");
        
        let buffer = MockRingBuffer::with_persistence(100, persistence_path.clone())?;
        
        let event = TestEvent {
            id: 1,
            timestamp: chrono::Utc::now(),
            event_type: "test".to_string(),
            data: serde_json::json!({"test": true}),
        };

        buffer.write_event(&event).await?;
        buffer.persist().await?;
        
        let restored_buffer = MockRingBuffer::from_persistence(persistence_path)?;
        let events = restored_buffer.read_events(1).await?;
        
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, 1);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_ring_buffer_concurrent_access() -> Result<()> {
        let buffer = MockRingBuffer::new(1000)?;
        
        // Test concurrent writes
        buffer.concurrent_write_test(4, 100).await?;
        
        let stats = buffer.get_stats().await;
        assert_eq!(stats.total_writes, 400);
        assert!(stats.concurrent_writes > 0);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_ring_buffer_factory() -> Result<()> {
        let small = MockRingBufferFactory::create_small_buffer()?;
        assert_eq!(small.capacity(), 100);
        
        let medium = MockRingBufferFactory::create_medium_buffer()?;
        assert_eq!(medium.capacity(), 10000);
        
        let large = MockRingBufferFactory::create_large_buffer()?;
        assert_eq!(large.capacity(), 1000000);
        
        Ok(())
    }
}