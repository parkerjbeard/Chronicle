//! Storage demonstration for Chronicle packer service
//!
//! This example shows how to use the storage manager to write
//! and manage Chronicle events in Parquet format.

use std::collections::HashMap;
use std::sync::Arc;

use chronicle_packer::{
    config::StorageConfig,
    storage::{StorageManager, ChronicleEvent, HeifFrame},
    integrity::IntegrityService,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("Chronicle Storage Manager - Demo");
    println!("===============================");
    
    // Create storage configuration
    let temp_dir = tempfile::TempDir::new()?;
    let mut config = StorageConfig::default();
    config.base_path = temp_dir.path().to_path_buf();
    config.compression_level = 6;
    
    println!("Storage configuration:");
    println!("  Base path: {}", config.base_path.display());
    println!("  Compression level: {}", config.compression_level);
    println!("  Retention days: {}", config.retention_days);
    
    // Create storage manager
    let integrity = Arc::new(IntegrityService::new());
    let mut storage = StorageManager::new(config, None, integrity)?;
    
    // Create sample events
    println!("\nCreating sample events...");
    let events = create_sample_events(1000);
    println!("Created {} events", events.len());
    
    // Write events to Parquet
    println!("\nWriting events to Parquet file...");
    let start_time = std::time::Instant::now();
    let date = chrono::Utc::now();
    let parquet_path = storage.write_events_to_parquet(&events, &date).await?;
    let write_time = start_time.elapsed();
    
    println!("Parquet file created: {}", parquet_path.display());
    println!("Write time: {:?}", write_time);
    
    // Check file metadata
    let metadata = storage.get_file_metadata(&parquet_path);
    if let Some(metadata) = metadata {
        println!("\nFile metadata:");
        println!("  Size: {} bytes", metadata.size);
        println!("  Format: {}", metadata.format);
        println!("  Records: {:?}", metadata.record_count);
        println!("  Checksum: {}", metadata.checksum);
        println!("  Encrypted: {}", metadata.encrypted);
        
        if let Some(compression) = &metadata.compression {
            println!("  Compression: {}", compression);
        }
    }
    
    // Create and process HEIF frames
    println!("\nProcessing HEIF frames...");
    let frames = create_sample_heif_frames(5);
    let frame_paths = storage.process_heif_frames(&frames, &date).await?;
    
    println!("Processed {} HEIF frames:", frame_paths.len());
    for (i, path) in frame_paths.iter().enumerate() {
        let file_size = std::fs::metadata(path)?.len();
        println!("  Frame {}: {} ({} bytes)", i + 1, path.display(), file_size);
    }
    
    // Get storage statistics
    let stats = storage.get_storage_stats();
    println!("\nStorage statistics:");
    println!("  Total files: {}", stats.total_files);
    println!("  Total size: {} bytes", stats.total_size);
    println!("  Files by format:");
    for (format, count) in &stats.by_format {
        println!("    {}: {}", format, count);
    }
    
    // Demonstrate file cleanup
    println!("\nTesting file cleanup...");
    
    // Create more files with different dates
    for i in 1..=5 {
        let old_date = chrono::Utc::now() - chrono::Duration::days(i);
        let old_events = create_sample_events(100);
        let _old_path = storage.write_events_to_parquet(&old_events, &old_date).await?;
        println!("Created file for {} days ago", i);
    }
    
    let stats_before = storage.get_storage_stats();
    println!("Files before cleanup: {}", stats_before.total_files);
    
    // Clean up old files (retention is 60 days by default, so nothing should be deleted)
    let deleted_count = storage.cleanup_old_files().await?;
    println!("Deleted {} old files", deleted_count);
    
    let stats_after = storage.get_storage_stats();
    println!("Files after cleanup: {}", stats_after.total_files);
    
    // List files in date range
    println!("\nListing files in date range...");
    let start_date = chrono::Utc::now() - chrono::Duration::days(2);
    let end_date = chrono::Utc::now() + chrono::Duration::days(1);
    
    let files_in_range = storage.list_files_in_date_range(&start_date, &end_date);
    println!("Found {} files in date range", files_in_range.len());
    
    for file_metadata in files_in_range {
        let created_date = chrono::DateTime::from_timestamp(file_metadata.created_at as i64, 0)
            .unwrap_or_else(|| chrono::Utc::now());
        println!("  {}: {} ({} bytes, created: {})", 
            file_metadata.path.display(),
            file_metadata.format,
            file_metadata.size,
            created_date.format("%Y-%m-%d %H:%M:%S")
        );
    }
    
    println!("\nStorage demo completed successfully!");
    
    Ok(())
}

/// Create sample Chronicle events for testing
fn create_sample_events(count: usize) -> Vec<ChronicleEvent> {
    let mut events = Vec::with_capacity(count);
    let base_timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
    
    let apps = vec![
        "com.apple.Safari",
        "com.apple.Terminal", 
        "com.microsoft.VSCode",
        "com.slack.Slack",
        "com.apple.mail",
    ];
    
    let event_types = vec!["key", "mouse", "window", "network", "clipboard"];
    
    for i in 0..count {
        let event_type = &event_types[i % event_types.len()];
        let app = &apps[i % apps.len()];
        
        let data = match event_type {
            "key" => format!(r#"{{"key": "{}", "modifiers": ["ctrl"], "app": "{}"}}"#, 
                           char::from(b'a' + (i as u8 % 26)), app),
            "mouse" => format!(r#"{{"x": {}, "y": {}, "button": "left", "app": "{}"}}"#, 
                             i % 1920, i % 1080, app),
            "window" => format!(r#"{{"action": "focus", "title": "Window {}", "app": "{}"}}"#, 
                              i, app),
            "network" => format!(r#"{{"host": "example{}.com", "port": {}, "protocol": "https"}}"#, 
                               i % 10, 443 + (i % 100)),
            "clipboard" => format!(r#"{{"content_type": "text", "length": {}}}"#, i % 1000),
            _ => "{}".to_string(),
        };
        
        events.push(ChronicleEvent {
            timestamp_ns: base_timestamp + (i as u64 * 1000000), // 1ms apart
            event_type: event_type.to_string(),
            app_bundle_id: Some(app.to_string()),
            window_title: Some(format!("{} - Window {}", 
                match app {
                    "com.apple.Safari" => "Safari",
                    "com.apple.Terminal" => "Terminal",
                    "com.microsoft.VSCode" => "Visual Studio Code",
                    "com.slack.Slack" => "Slack",
                    "com.apple.mail" => "Mail",
                    _ => "Unknown App",
                }, i)),
            data,
            session_id: format!("session_{}", i / 100), // Group events into sessions
            event_id: format!("event_{:08}", i),
        });
    }
    
    events
}

/// Create sample HEIF frames for testing
fn create_sample_heif_frames(count: usize) -> Vec<HeifFrame> {
    let mut frames = Vec::with_capacity(count);
    let base_timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
    
    for i in 0..count {
        // Create a simple test image (placeholder)
        // In reality, this would be actual image data
        let mut image_data = Vec::new();
        
        // Create a minimal JPEG header (just for demo - not a real image)
        image_data.extend_from_slice(&[0xFF, 0xD8, 0xFF, 0xE0]); // JPEG SOI + APP0
        
        // Add some dummy data
        for j in 0..1000 {
            image_data.push(((i + j) % 256) as u8);
        }
        
        // JPEG EOI
        image_data.extend_from_slice(&[0xFF, 0xD9]);
        
        let metadata = HashMap::from([
            ("width".to_string(), "1920".to_string()),
            ("height".to_string(), "1080".to_string()),
            ("quality".to_string(), "80".to_string()),
            ("frame_id".to_string(), format!("frame_{:04}", i)),
        ]);
        
        frames.push(HeifFrame {
            timestamp: base_timestamp + (i as u64 * 5000000000), // 5 seconds apart
            data: image_data,
            metadata,
        });
    }
    
    frames
}