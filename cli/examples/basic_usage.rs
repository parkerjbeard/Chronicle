// Basic usage example - this would be implemented as a separate binary
// For now, this serves as documentation of the API usage patterns
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the Chronicle client
    let client = ChronicleClient::new("http://localhost:8080".to_string());
    
    // Create output manager with table format and colors
    let output = OutputManager::new(OutputFormat::Table, true);
    
    println!("Chronicle CLI Basic Usage Examples");
    println!("=================================\n");
    
    // Example 1: Check service health
    println!("1. Checking service health...");
    match client.health().await {
        Ok(health) => {
            output.print_health_status(&health)?;
        }
        Err(e) => {
            output.print_error(&format!("Failed to get health status: {}", e))?;
        }
    }
    
    println!("\n" + &"-".repeat(50) + "\n");
    
    // Example 2: Simple search
    println!("2. Performing a simple search...");
    let search_query = SearchQueryBuilder::new("*")
        .with_limit(5)
        .build();
    
    match client.search(&search_query).await {
        Ok(results) => {
            if results.events.is_empty() {
                output.print_info("No events found")?;
            } else {
                output.print_search_results(&results)?;
            }
        }
        Err(e) => {
            output.print_error(&format!("Search failed: {}", e))?;
        }
    }
    
    println!("\n" + &"-".repeat(50) + "\n");
    
    // Example 3: Search with time range
    println!("3. Searching with time range (last hour)...");
    let time_search = SearchQueryBuilder::new("*")
        .with_time_str("last-hour")?
        .with_limit(3)
        .build();
    
    match client.search(&time_search).await {
        Ok(results) => {
            output.print_info(&format!("Found {} events in the last hour", results.total_count))?;
            output.print_search_results(&results)?;
        }
        Err(e) => {
            output.print_warning(&format!("Time-based search failed: {}", e))?;
        }
    }
    
    println!("\n" + &"-".repeat(50) + "\n");
    
    // Example 4: Search with filters
    println!("4. Searching with filters...");
    let filtered_search = SearchQueryBuilder::new("error")
        .with_filter("level", "critical")
        .with_limit(3)
        .build();
    
    match client.search(&filtered_search).await {
        Ok(results) => {
            output.print_info(&format!("Found {} critical error events", results.total_count))?;
            output.print_search_results(&results)?;
        }
        Err(e) => {
            output.print_warning(&format!("Filtered search failed: {}", e))?;
        }
    }
    
    println!("\n" + &"-".repeat(50) + "\n");
    
    // Example 5: Get configuration
    println!("5. Getting configuration...");
    match client.config().await {
        Ok(config_info) => {
            output.print_key_value("Config file", &config_info.config_file)?;
            output.print_key_value("Last modified", &config_info.last_modified.to_string())?;
            
            // Show a few config values
            for (key, value) in config_info.config.iter().take(3) {
                output.print_key_value(key, &value.to_string())?;
            }
        }
        Err(e) => {
            output.print_warning(&format!("Failed to get configuration: {}", e))?;
        }
    }
    
    println!("\n" + &"-".repeat(50) + "\n");
    
    // Example 6: Different output formats
    println!("6. Demonstrating different output formats...");
    
    let sample_search = SearchQueryBuilder::new("*")
        .with_limit(2)
        .build();
    
    if let Ok(results) = client.search(&sample_search).await {
        if !results.events.is_empty() {
            println!("\nJSON format:");
            let json_output = OutputManager::new(OutputFormat::Json, false);
            json_output.print_search_results(&results)?;
            
            println!("\nCSV format:");
            let csv_output = OutputManager::new(OutputFormat::Csv, false);
            csv_output.print_search_results(&results)?;
        }
    }
    
    println!("\n" + &"-".repeat(50) + "\n");
    
    output.print_success("Basic usage examples completed!")?;
    output.print_info("For more advanced usage, see the CLI help:")?;
    println!("  chronictl --help");
    println!("  chronictl search --help");
    println!("  chronictl export --help");
    
    Ok(())
}

/// Example of error handling patterns
async fn demonstrate_error_handling() -> Result<()> {
    let client = ChronicleClient::new("http://invalid-url:9999".to_string());
    let output = OutputManager::new(OutputFormat::Table, true);
    
    match client.health().await {
        Ok(_) => {
            output.print_success("Service is healthy")?;
        }
        Err(e) => {
            // Different error handling strategies
            match &e {
                chronictl::error::ChronicleError::ServiceUnavailable => {
                    output.print_error("Chronicle service is not available")?;
                    output.print_info("Try starting the service or check your configuration")?;
                }
                chronictl::error::ChronicleError::Network(_) => {
                    output.print_error("Network connection failed")?;
                    output.print_info("Check your network connection and service URL")?;
                }
                chronictl::error::ChronicleError::Timeout => {
                    output.print_error("Request timed out")?;
                    output.print_info("Try increasing the timeout or check service performance")?;
                }
                _ => {
                    output.print_error(&format!("Unexpected error: {}", e))?;
                }
            }
        }
    }
    
    Ok(())
}

/// Example of configuration management
async fn demonstrate_config_management() -> Result<()> {
    let client = ChronicleClient::new("http://localhost:8080".to_string());
    let output = OutputManager::new(OutputFormat::Table, true);
    
    // Get current configuration
    match client.config().await {
        Ok(config_info) => {
            output.print_info("Current configuration:")?;
            output.print_config(&config_info.config)?;
            
            // Example of updating configuration
            let mut new_config = config_info.config;
            new_config.insert(
                "example_setting".to_string(),
                serde_json::Value::String("example_value".to_string()),
            );
            
            match client.update_config(&new_config).await {
                Ok(_) => {
                    output.print_success("Configuration updated successfully")?;
                }
                Err(e) => {
                    output.print_error(&format!("Failed to update configuration: {}", e))?;
                }
            }
        }
        Err(e) => {
            output.print_error(&format!("Failed to get configuration: {}", e))?;
        }
    }
    
    Ok(())
}

/// Example of advanced search patterns
async fn demonstrate_advanced_search() -> Result<()> {
    let client = ChronicleClient::new("http://localhost:8080".to_string());
    let output = OutputManager::new(OutputFormat::Table, true);
    
    // Regex search example
    output.print_info("Performing regex search...")?;
    let regex_search = SearchQueryBuilder::new("/error.*critical/")
        .with_limit(5)
        .build();
    
    match client.search(&regex_search).await {
        Ok(results) => {
            output.print_info(&format!("Regex search found {} events", results.total_count))?;
        }
        Err(e) => {
            output.print_warning(&format!("Regex search failed: {}", e))?;
        }
    }
    
    // Time range search with custom format
    output.print_info("Searching specific time range...")?;
    let time_range_search = SearchQueryBuilder::new("*")
        .with_time_str("2024-01-01T10:00:00..2024-01-01T11:00:00")?
        .with_limit(10)
        .build();
    
    match client.search(&time_range_search).await {
        Ok(results) => {
            output.print_info(&format!("Time range search found {} events", results.total_count))?;
        }
        Err(e) => {
            output.print_warning(&format!("Time range search failed: {}", e))?;
        }
    }
    
    // Complex filter search
    output.print_info("Searching with multiple filters...")?;
    let complex_search = SearchQueryBuilder::new("authentication")
        .with_filter("event_type", "security")
        .with_filter("status", "failed")
        .with_filter("severity", "high")
        .with_limit(5)
        .build();
    
    match client.search(&complex_search).await {
        Ok(results) => {
            output.print_info(&format!("Complex search found {} events", results.total_count))?;
            if !results.events.is_empty() {
                output.print_events(&results.events)?;
            }
        }
        Err(e) => {
            output.print_warning(&format!("Complex search failed: {}", e))?;
        }
    }
    
    Ok(())
}