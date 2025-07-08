use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::timeout;
use anyhow::Result;
use serde_json::json;

use crate::mocks::{MockCollector, MockRingBuffer};
use crate::utils::{TestHarness, TestEvent};

/// Ring buffer integration tests with multiple collectors
#[tokio::test]
async fn test_ring_buffer_multiple_collectors() -> Result<()> {
    let harness = TestHarness::new().await?;
    
    // Create ring buffer with moderate size
    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(2 * 1024 * 1024)?));
    
    // Create multiple collectors
    let collectors = (0..4).map(|i| {
        let mut collector = MockCollector::new(ring_buffer.clone()).unwrap();
        collector.set_id(format!("collector_{}", i));
        collector.set_event_rate(100); // 100 events per second per collector
        collector
    }).collect::<Vec<_>>();
    
    // Start all collectors
    let handles = collectors.into_iter().map(|collector| {
        tokio::spawn(async move {
            collector.start_collection().await
        })
    }).collect::<Vec<_>>();
    
    // Let collectors run for 10 seconds
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    // Stop all collectors
    for handle in handles {
        handle.abort();
    }
    
    // Validate ring buffer state
    let buffer = ring_buffer.read().await;
    let stats = buffer.get_stats();
    
    assert!(stats.total_writes >= 3800); // 4 collectors * 100 events/s * 10s * 0.95 (allow some variance)
    assert!(stats.total_reads >= 0);
    assert!(stats.buffer_utilization > 0.0);
    assert_eq!(stats.overflow_count, 0);
    
    Ok(())
}

#[tokio::test]
async fn test_ring_buffer_overflow_handling() -> Result<()> {
    let harness = TestHarness::new().await?;
    
    // Create small ring buffer to force overflow
    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(64 * 1024)?)); // 64KB
    
    // Create high-rate collector
    let mut collector = MockCollector::new(ring_buffer.clone())?;
    collector.set_event_rate(10000); // Very high rate to cause overflow
    collector.set_large_events(true); // Generate large events
    
    let collect_handle = tokio::spawn(async move {
        collector.start_collection().await
    });
    
    // Let it run until overflow occurs
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    collect_handle.abort();
    
    // Check overflow handling
    let buffer = ring_buffer.read().await;
    let stats = buffer.get_stats();
    
    assert!(stats.overflow_count > 0);
    assert!(stats.buffer_utilization > 0.8); // Should be nearly full
    assert!(stats.oldest_event_dropped); // Should have dropped old events
    
    Ok(())
}

#[tokio::test]
async fn test_ring_buffer_concurrent_read_write() -> Result<()> {
    let harness = TestHarness::new().await?;
    
    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024)?));
    
    // Create writers
    let writers = (0..3).map(|i| {
        let ring_buffer = ring_buffer.clone();
        tokio::spawn(async move {
            let mut events_written = 0;
            for j in 0..1000 {
                let event = TestEvent {
                    id: (i * 1000 + j) as u64,
                    timestamp: chrono::Utc::now(),
                    event_type: format!("writer_{}", i),
                    data: json!({
                        "writer_id": i,
                        "sequence": j,
                        "concurrent_test": true
                    }),
                };
                
                let mut buffer = ring_buffer.write().await;
                buffer.write_event(&event).await?;
                events_written += 1;
                
                // Small delay to allow readers
                if j % 10 == 0 {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }
            Ok::<usize, anyhow::Error>(events_written)
        })
    }).collect::<Vec<_>>();
    
    // Create readers
    let readers = (0..2).map(|i| {
        let ring_buffer = ring_buffer.clone();
        tokio::spawn(async move {
            let mut events_read = 0;
            let start_time = Instant::now();
            
            while start_time.elapsed() < Duration::from_secs(15) {
                let buffer = ring_buffer.read().await;
                if let Ok(events) = buffer.read_events(100).await {
                    events_read += events.len();
                }
                
                // Small delay between reads
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            
            Ok::<usize, anyhow::Error>(events_read)
        })
    }).collect::<Vec<_>>();
    
    // Wait for all writers to complete
    let mut total_written = 0;
    for writer in writers {
        total_written += writer.await??;
    }
    
    // Wait for readers to finish
    let mut total_read = 0;
    for reader in readers {
        total_read += reader.await??;
    }
    
    // Validate concurrent access
    assert_eq!(total_written, 3000); // 3 writers * 1000 events each
    assert!(total_read > 0);
    
    let buffer = ring_buffer.read().await;
    let stats = buffer.get_stats();
    
    assert!(stats.concurrent_reads > 0);
    assert!(stats.concurrent_writes > 0);
    assert_eq!(stats.deadlock_count, 0);
    
    Ok(())
}

#[tokio::test]
async fn test_ring_buffer_memory_management() -> Result<()> {
    let harness = TestHarness::new().await?;
    
    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024)?));
    
    // Record initial memory usage
    let initial_memory = ring_buffer.read().await.get_memory_usage();
    
    // Fill buffer with events
    let mut buffer = ring_buffer.write().await;
    for i in 0..10000 {
        let event = TestEvent {
            id: i as u64,
            timestamp: chrono::Utc::now(),
            event_type: "memory_test".to_string(),
            data: json!({
                "sequence": i,
                "large_data": "x".repeat(100), // 100 bytes per event
                "memory_test": true
            }),
        };
        
        buffer.write_event(&event).await?;
    }
    
    let filled_memory = buffer.get_memory_usage();
    assert!(filled_memory > initial_memory);
    
    // Read all events to test cleanup
    let events = buffer.read_events(10000).await?;
    assert_eq!(events.len(), 10000);
    
    // Force garbage collection
    buffer.compact().await?;
    
    let compacted_memory = buffer.get_memory_usage();
    assert!(compacted_memory <= filled_memory);
    
    // Verify memory is properly managed
    let stats = buffer.get_stats();
    assert!(stats.memory_efficiency > 0.8);
    assert_eq!(stats.memory_leaks, 0);
    
    Ok(())
}

#[tokio::test]
async fn test_ring_buffer_persistence() -> Result<()> {
    let harness = TestHarness::new().await?;
    
    let temp_dir = tempfile::TempDir::new()?;
    let persistence_path = temp_dir.path().join("ring_buffer_state");
    
    // Create ring buffer with persistence
    let ring_buffer = Arc::new(RwLock::new(
        MockRingBuffer::with_persistence(1024 * 1024, persistence_path.clone())?
    ));
    
    // Write some events
    let events = (0..100).map(|i| {
        TestEvent {
            id: i as u64,
            timestamp: chrono::Utc::now(),
            event_type: "persistence_test".to_string(),
            data: json!({
                "sequence": i,
                "persistence_test": true
            }),
        }
    }).collect::<Vec<_>>();
    
    {
        let mut buffer = ring_buffer.write().await;
        for event in &events {
            buffer.write_event(event).await?;
        }
        
        // Force persistence
        buffer.persist().await?;
    }
    
    // Create new ring buffer instance from persisted state
    let restored_buffer = Arc::new(RwLock::new(
        MockRingBuffer::from_persistence(persistence_path)?
    ));
    
    // Verify restored state
    let buffer = restored_buffer.read().await;
    let restored_events = buffer.read_events(100).await?;
    
    assert_eq!(restored_events.len(), 100);
    for (original, restored) in events.iter().zip(restored_events.iter()) {
        assert_eq!(original.id, restored.id);
        assert_eq!(original.event_type, restored.event_type);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_ring_buffer_event_ordering() -> Result<()> {
    let harness = TestHarness::new().await?;
    
    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024)?));
    
    // Write events with specific ordering
    let events = (0..1000).map(|i| {
        TestEvent {
            id: i as u64,
            timestamp: chrono::Utc::now() + chrono::Duration::milliseconds(i as i64),
            event_type: "ordering_test".to_string(),
            data: json!({
                "sequence": i,
                "ordering_test": true
            }),
        }
    }).collect::<Vec<_>>();
    
    // Write events in random order
    let mut shuffled_events = events.clone();
    use rand::seq::SliceRandom;
    shuffled_events.shuffle(&mut rand::thread_rng());
    
    {
        let mut buffer = ring_buffer.write().await;
        for event in shuffled_events {
            buffer.write_event(&event).await?;
        }
    }
    
    // Read events back and verify ordering
    let buffer = ring_buffer.read().await;
    let read_events = buffer.read_events_ordered(1000).await?;
    
    assert_eq!(read_events.len(), 1000);
    
    // Verify events are in chronological order
    for i in 1..read_events.len() {
        assert!(read_events[i].timestamp >= read_events[i-1].timestamp);
    }
    
    // Verify all events are present
    for i in 0..1000 {
        assert!(read_events.iter().any(|e| e.id == i as u64));
    }
    
    Ok(())
}

#[tokio::test]
async fn test_ring_buffer_performance_under_load() -> Result<()> {
    let harness = TestHarness::new().await?;
    
    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(10 * 1024 * 1024)?));
    
    // Performance test: measure throughput
    let start_time = Instant::now();
    let events_count = 50000;
    
    // Concurrent writers
    let writers = (0..4).map(|writer_id| {
        let ring_buffer = ring_buffer.clone();
        tokio::spawn(async move {
            let events_per_writer = events_count / 4;
            let mut write_times = Vec::new();
            
            for i in 0..events_per_writer {
                let event = TestEvent {
                    id: (writer_id * events_per_writer + i) as u64,
                    timestamp: chrono::Utc::now(),
                    event_type: "performance_test".to_string(),
                    data: json!({
                        "writer_id": writer_id,
                        "sequence": i,
                        "performance_test": true
                    }),
                };
                
                let write_start = Instant::now();
                {
                    let mut buffer = ring_buffer.write().await;
                    buffer.write_event(&event).await?;
                }
                write_times.push(write_start.elapsed());
            }
            
            Ok::<Vec<Duration>, anyhow::Error>(write_times)
        })
    }).collect::<Vec<_>>();
    
    // Concurrent readers
    let readers = (0..2).map(|_| {
        let ring_buffer = ring_buffer.clone();
        tokio::spawn(async move {
            let mut total_read = 0;
            let mut read_times = Vec::new();
            
            while total_read < events_count {
                let read_start = Instant::now();
                let buffer = ring_buffer.read().await;
                if let Ok(events) = buffer.read_events(1000).await {
                    total_read += events.len();
                    read_times.push(read_start.elapsed());
                }
                
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            
            Ok::<Vec<Duration>, anyhow::Error>(read_times)
        })
    }).collect::<Vec<_>>();
    
    // Wait for all operations to complete
    let mut all_write_times = Vec::new();
    for writer in writers {
        all_write_times.extend(writer.await??);
    }
    
    let mut all_read_times = Vec::new();
    for reader in readers {
        all_read_times.extend(reader.await??);
    }
    
    let total_time = start_time.elapsed();
    
    // Calculate performance metrics
    let avg_write_time = all_write_times.iter().sum::<Duration>() / all_write_times.len() as u32;
    let avg_read_time = all_read_times.iter().sum::<Duration>() / all_read_times.len() as u32;
    let throughput = events_count as f64 / total_time.as_secs_f64();
    
    println!("Ring Buffer Performance Test Results:");
    println!("Total events: {}", events_count);
    println!("Total time: {:?}", total_time);
    println!("Throughput: {:.2} events/sec", throughput);
    println!("Average write time: {:?}", avg_write_time);
    println!("Average read time: {:?}", avg_read_time);
    
    // Performance assertions
    assert!(avg_write_time < Duration::from_micros(100)); // Write should be fast
    assert!(avg_read_time < Duration::from_millis(1));    // Read should be fast
    assert!(throughput > 10000.0);                        // Should handle 10k+ events/sec
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_ring_buffer_basic_operations() -> Result<()> {
        let ring_buffer = MockRingBuffer::new(1024)?;
        
        // Test write
        let event = TestEvent {
            id: 1,
            timestamp: chrono::Utc::now(),
            event_type: "test".to_string(),
            data: json!({"test": true}),
        };
        
        ring_buffer.write_event(&event).await?;
        
        // Test read
        let events = ring_buffer.read_events(1).await?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, 1);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_ring_buffer_capacity() -> Result<()> {
        let ring_buffer = MockRingBuffer::new(1024)?;
        let capacity = ring_buffer.capacity();
        assert!(capacity > 0);
        assert!(capacity <= 1024);
        
        Ok(())
    }
}