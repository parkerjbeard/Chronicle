use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tokio::sync::RwLock;
use tempfile::TempDir;
use chrono::{DateTime, Utc};
use serde_json::json;
use anyhow::Result;

use crate::mocks::{MockCollector, MockRingBuffer, MockPacker};
use crate::utils::{TestHarness, TestEvent, ValidationResult};

/// Full pipeline integration test
/// Tests the complete flow: collectors → ring buffer → packer → storage
#[tokio::test]
async fn test_full_pipeline_integration() -> Result<()> {
    let harness = TestHarness::new().await?;
    
    // Setup test environment
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_path_buf();
    
    // Initialize components
    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024)?));
    let collector = MockCollector::new(ring_buffer.clone())?;
    let packer = MockPacker::new(ring_buffer.clone(), storage_path.clone())?;
    
    // Start collecting events
    let collect_handle = tokio::spawn(async move {
        collector.start_collection().await
    });
    
    // Start packer
    let pack_handle = tokio::spawn(async move {
        packer.start_packing().await
    });
    
    // Generate test events
    let events = generate_test_events(1000);
    
    // Write events to ring buffer
    let write_handle = tokio::spawn({
        let ring_buffer = ring_buffer.clone();
        async move {
            for event in events {
                let mut buffer = ring_buffer.write().await;
                buffer.write_event(&event).await?;
            }
            Ok::<(), anyhow::Error>(())
        }
    });
    
    // Wait for events to be processed
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Stop components
    collect_handle.abort();
    pack_handle.abort();
    write_handle.await??;
    
    // Validate results
    let validation = harness.validate_pipeline_output(&storage_path).await?;
    assert!(validation.is_valid());
    assert_eq!(validation.processed_events, 1000);
    assert!(validation.data_integrity_check);
    
    Ok(())
}

#[tokio::test]
async fn test_pipeline_with_high_load() -> Result<()> {
    let harness = TestHarness::new().await?;
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_path_buf();
    
    // Create larger ring buffer for high load
    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(10 * 1024 * 1024)?));
    let collector = MockCollector::new(ring_buffer.clone())?;
    let packer = MockPacker::new(ring_buffer.clone(), storage_path.clone())?;
    
    // High load test: 10,000 events per second for 10 seconds
    let events_per_second = 10000;
    let duration_seconds = 10;
    let total_events = events_per_second * duration_seconds;
    
    let collect_handle = tokio::spawn(async move {
        collector.start_collection().await
    });
    
    let pack_handle = tokio::spawn(async move {
        packer.start_packing().await
    });
    
    // Generate high-frequency events
    let write_handle = tokio::spawn({
        let ring_buffer = ring_buffer.clone();
        async move {
            for i in 0..total_events {
                let event = TestEvent {
                    id: i as u64,
                    timestamp: Utc::now(),
                    event_type: "high_load_test".to_string(),
                    data: json!({
                        "sequence": i,
                        "batch": i / 1000,
                        "load_test": true
                    }),
                };
                
                let mut buffer = ring_buffer.write().await;
                buffer.write_event(&event).await?;
                
                // Maintain 10k events per second
                if i % 100 == 0 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
            Ok::<(), anyhow::Error>(())
        }
    });
    
    // Wait for processing
    tokio::time::sleep(Duration::from_secs(15)).await;
    
    collect_handle.abort();
    pack_handle.abort();
    write_handle.await??;
    
    // Validate high load processing
    let validation = harness.validate_pipeline_output(&storage_path).await?;
    assert!(validation.is_valid());
    assert!(validation.processed_events >= total_events as usize * 95 / 100); // Allow 5% loss
    assert!(validation.data_integrity_check);
    
    Ok(())
}

#[tokio::test]
async fn test_pipeline_error_recovery() -> Result<()> {
    let harness = TestHarness::new().await?;
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_path_buf();
    
    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024)?));
    let mut collector = MockCollector::new(ring_buffer.clone())?;
    let mut packer = MockPacker::new(ring_buffer.clone(), storage_path.clone())?;
    
    // Enable error simulation
    collector.enable_error_simulation(0.1); // 10% error rate
    packer.enable_error_simulation(0.05);   // 5% error rate
    
    let collect_handle = tokio::spawn(async move {
        collector.start_collection().await
    });
    
    let pack_handle = tokio::spawn(async move {
        packer.start_packing().await
    });
    
    // Generate events with simulated errors
    let events = generate_test_events(1000);
    let write_handle = tokio::spawn({
        let ring_buffer = ring_buffer.clone();
        async move {
            for event in events {
                let mut buffer = ring_buffer.write().await;
                // Simulate occasional write failures
                if rand::random::<f64>() < 0.02 {
                    // Skip this event to simulate error
                    continue;
                }
                buffer.write_event(&event).await?;
            }
            Ok::<(), anyhow::Error>(())
        }
    });
    
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    collect_handle.abort();
    pack_handle.abort();
    write_handle.await??;
    
    // Validate error recovery
    let validation = harness.validate_pipeline_output(&storage_path).await?;
    assert!(validation.is_valid());
    // Should have processed most events despite errors
    assert!(validation.processed_events >= 850); // Allow for some loss due to errors
    assert!(validation.error_recovery_successful);
    
    Ok(())
}

#[tokio::test]
async fn test_pipeline_concurrent_access() -> Result<()> {
    let harness = TestHarness::new().await?;
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_path_buf();
    
    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(2 * 1024 * 1024)?));
    
    // Create multiple collectors and packers
    let collectors = (0..3).map(|i| {
        let mut collector = MockCollector::new(ring_buffer.clone()).unwrap();
        collector.set_id(format!("collector_{}", i));
        collector
    }).collect::<Vec<_>>();
    
    let packers = (0..2).map(|i| {
        let mut packer = MockPacker::new(ring_buffer.clone(), storage_path.clone()).unwrap();
        packer.set_id(format!("packer_{}", i));
        packer
    }).collect::<Vec<_>>();
    
    // Start all collectors
    let collect_handles = collectors.into_iter().map(|collector| {
        tokio::spawn(async move {
            collector.start_collection().await
        })
    }).collect::<Vec<_>>();
    
    // Start all packers
    let pack_handles = packers.into_iter().map(|packer| {
        tokio::spawn(async move {
            packer.start_packing().await
        })
    }).collect::<Vec<_>>();
    
    // Generate events from multiple sources
    let writer_handles = (0..5).map(|writer_id| {
        let ring_buffer = ring_buffer.clone();
        tokio::spawn(async move {
            for i in 0..200 {
                let event = TestEvent {
                    id: (writer_id * 1000 + i) as u64,
                    timestamp: Utc::now(),
                    event_type: format!("writer_{}", writer_id),
                    data: json!({
                        "writer_id": writer_id,
                        "sequence": i,
                        "concurrent_test": true
                    }),
                };
                
                let mut buffer = ring_buffer.write().await;
                buffer.write_event(&event).await?;
                
                // Add small delay to simulate real-world timing
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            Ok::<(), anyhow::Error>(())
        })
    }).collect::<Vec<_>>();
    
    // Wait for all writers to complete
    for handle in writer_handles {
        handle.await??;
    }
    
    // Let processing continue
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Stop all components
    for handle in collect_handles {
        handle.abort();
    }
    for handle in pack_handles {
        handle.abort();
    }
    
    // Validate concurrent processing
    let validation = harness.validate_pipeline_output(&storage_path).await?;
    assert!(validation.is_valid());
    assert_eq!(validation.processed_events, 1000); // 5 writers * 200 events each
    assert!(validation.data_integrity_check);
    assert!(validation.concurrent_access_safe);
    
    Ok(())
}

#[tokio::test]
async fn test_pipeline_data_integrity() -> Result<()> {
    let harness = TestHarness::new().await?;
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_path_buf();
    
    let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024 * 1024)?));
    let mut collector = MockCollector::new(ring_buffer.clone())?;
    let mut packer = MockPacker::new(ring_buffer.clone(), storage_path.clone())?;
    
    // Enable integrity checks
    collector.enable_integrity_checks(true);
    packer.enable_integrity_checks(true);
    
    let collect_handle = tokio::spawn(async move {
        collector.start_collection().await
    });
    
    let pack_handle = tokio::spawn(async move {
        packer.start_packing().await
    });
    
    // Generate events with known checksums
    let events = generate_test_events_with_checksums(500);
    let expected_checksums = events.iter().map(|e| e.checksum()).collect::<Vec<_>>();
    
    let write_handle = tokio::spawn({
        let ring_buffer = ring_buffer.clone();
        async move {
            for event in events {
                let mut buffer = ring_buffer.write().await;
                buffer.write_event(&event).await?;
            }
            Ok::<(), anyhow::Error>(())
        }
    });
    
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    collect_handle.abort();
    pack_handle.abort();
    write_handle.await??;
    
    // Validate data integrity
    let validation = harness.validate_pipeline_output(&storage_path).await?;
    assert!(validation.is_valid());
    assert_eq!(validation.processed_events, 500);
    assert!(validation.data_integrity_check);
    
    // Check individual event integrity
    let stored_checksums = harness.extract_event_checksums(&storage_path).await?;
    assert_eq!(stored_checksums.len(), expected_checksums.len());
    
    for (expected, actual) in expected_checksums.iter().zip(stored_checksums.iter()) {
        assert_eq!(expected, actual, "Checksum mismatch detected");
    }
    
    Ok(())
}

fn generate_test_events(count: usize) -> Vec<TestEvent> {
    (0..count).map(|i| {
        TestEvent {
            id: i as u64,
            timestamp: Utc::now(),
            event_type: match i % 4 {
                0 => "keypress".to_string(),
                1 => "mouse_click".to_string(),
                2 => "window_focus".to_string(),
                _ => "clipboard_change".to_string(),
            },
            data: json!({
                "sequence": i,
                "test_data": format!("test_event_{}", i),
                "metadata": {
                    "source": "integration_test",
                    "version": "1.0"
                }
            }),
        }
    }).collect()
}

fn generate_test_events_with_checksums(count: usize) -> Vec<TestEvent> {
    (0..count).map(|i| {
        let mut event = TestEvent {
            id: i as u64,
            timestamp: Utc::now(),
            event_type: "integrity_test".to_string(),
            data: json!({
                "sequence": i,
                "test_data": format!("integrity_test_event_{}", i),
                "checksum_test": true
            }),
        };
        event.calculate_checksum();
        event
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_pipeline_components_initialization() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage_path = temp_dir.path().to_path_buf();
        
        let ring_buffer = Arc::new(RwLock::new(MockRingBuffer::new(1024)?));
        let collector = MockCollector::new(ring_buffer.clone())?;
        let packer = MockPacker::new(ring_buffer.clone(), storage_path)?;
        
        assert!(collector.is_initialized());
        assert!(packer.is_initialized());
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_event_generation() -> Result<()> {
        let events = generate_test_events(10);
        assert_eq!(events.len(), 10);
        
        for (i, event) in events.iter().enumerate() {
            assert_eq!(event.id, i as u64);
            assert!(!event.event_type.is_empty());
            assert!(!event.data.is_null());
        }
        
        Ok(())
    }
}