//! Basic usage example for Chronicle packer service
//!
//! This example demonstrates how to create and use the packer service
//! for processing Chronicle events.

use std::time::Duration;

use chronicle_packer::{
    config::PackerConfig,
    packer::PackerService,
    storage::ChronicleEvent,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("Chronicle Packer Service - Basic Usage Example");
    println!("=============================================");
    
    // Create configuration
    let mut config = PackerConfig::default();
    
    // Use a temporary directory for this example
    let temp_dir = tempfile::TempDir::new()?;
    config.storage.base_path = temp_dir.path().to_path_buf();
    
    // Disable encryption for simplicity
    config.encryption.enabled = false;
    
    // Disable metrics server
    config.metrics.enabled = true;
    config.metrics.port = 0;
    
    println!("Configuration:");
    println!("  Storage path: {}", config.storage.base_path.display());
    println!("  Encryption: {}", config.encryption.enabled);
    println!("  Retention: {} days", config.storage.retention_days);
    
    // Create the packer service
    println!("\nCreating packer service...");
    let mut service = PackerService::new(config).await?;
    
    // Start the service
    println!("Starting packer service...");
    service.start().await?;
    
    // Check initial status
    let status = service.get_status().await;
    println!("Service status: {:?}", status.status);
    
    // Trigger manual processing (since ring buffer is empty, this will be quick)
    println!("\nTriggering manual processing...");
    let start_time = std::time::Instant::now();
    let result = service.trigger_processing().await?;
    let processing_time = start_time.elapsed();
    
    println!("Processing completed in {:?}", processing_time);
    println!("  Events processed: {}", result.events_processed);
    println!("  Files created: {}", result.files_created);
    println!("  Bytes processed: {}", result.bytes_processed);
    
    if !result.errors.is_empty() {
        println!("  Errors:");
        for error in &result.errors {
            println!("    - {}", error);
        }
    }
    
    // Get updated status
    let updated_status = service.get_status().await;
    println!("\nUpdated service statistics:");
    println!("  Total runs: {}", updated_status.stats.total_runs);
    println!("  Successful runs: {}", updated_status.stats.successful_runs);
    println!("  Failed runs: {}", updated_status.stats.failed_runs);
    println!("  Average processing time: {:?}", updated_status.stats.avg_processing_time);
    
    // Stop the service
    println!("\nStopping packer service...");
    service.stop().await?;
    
    let final_status = service.get_status().await;
    println!("Final status: {:?}", final_status.status);
    
    println!("\nExample completed successfully!");
    
    Ok(())
}