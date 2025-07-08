use chronictl::{
    api::ChronicleClient,
    error::Result,
    output::{OutputFormat, OutputManager},
    search::{SearchQueryBuilder, TimeRange, QueryValidator},
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = ChronicleClient::new("http://localhost:8080".to_string());
    let output = OutputManager::new(OutputFormat::Table, true);
    
    println!("Chronicle CLI Search Examples");
    println!("============================\n");
    
    // Example 1: Basic text search
    println!("1. Basic text search for 'error':");
    let basic_search = SearchQueryBuilder::new("error")
        .with_limit(5)
        .build();
    
    demonstrate_search(&client, &output, &basic_search, "Basic search").await?;
    
    // Example 2: Regex search
    println!("\n2. Regex search for errors with codes:");
    let regex_search = SearchQueryBuilder::new("/error.*[0-9]{3,4}/")
        .with_limit(5)
        .build();
    
    demonstrate_search(&client, &output, &regex_search, "Regex search").await?;
    
    // Example 3: Time-based searches
    println!("\n3. Time-based searches:");
    
    // Last hour
    let last_hour = SearchQueryBuilder::new("*")
        .with_time_str("last-hour")?
        .with_limit(3)
        .build();
    demonstrate_search(&client, &output, &last_hour, "Last hour").await?;
    
    // Today
    let today = SearchQueryBuilder::new("*")
        .with_time_str("today")?
        .with_limit(3)
        .build();
    demonstrate_search(&client, &output, &today, "Today").await?;
    
    // Custom time range
    let custom_range = SearchQueryBuilder::new("*")
        .with_time_str("2024-01-01..2024-01-02")?
        .with_limit(3)
        .build();
    demonstrate_search(&client, &output, &custom_range, "Custom range").await?;
    
    // Example 4: Filtered searches
    println!("\n4. Filtered searches:");
    
    // Search for authentication events
    let auth_search = SearchQueryBuilder::new("authentication")
        .with_filter("event_type", "security")
        .with_filter("result", "failed")
        .with_limit(5)
        .build();
    demonstrate_search(&client, &output, &auth_search, "Authentication failures").await?;
    
    // Search for high priority events
    let priority_search = SearchQueryBuilder::new("*")
        .with_filter("priority", "high")
        .with_filter("status", "active")
        .with_limit(5)
        .build();
    demonstrate_search(&client, &output, &priority_search, "High priority events").await?;
    
    // Example 5: Pagination
    println!("\n5. Pagination example:");
    demonstrate_pagination(&client, &output).await?;
    
    // Example 6: Complex queries
    println!("\n6. Complex queries:");
    
    // SQL injection attempts
    let sqli_search = SearchQueryBuilder::new("/(union|select|insert|drop|delete).*sql/i")
        .with_filter("event_type", "web_request")
        .with_limit(3)
        .build();
    demonstrate_search(&client, &output, &sqli_search, "SQL injection attempts").await?;
    
    // Failed login patterns
    let login_search = SearchQueryBuilder::new("/login.*fail/i")
        .with_filter("source", "authentication")
        .with_time_str("last-day")?
        .with_limit(5)
        .build();
    demonstrate_search(&client, &output, &login_search, "Failed logins (last day)").await?;
    
    // Example 7: Search validation
    println!("\n7. Search validation examples:");
    demonstrate_search_validation().await?;
    
    // Example 8: Performance considerations
    println!("\n8. Performance considerations:");
    demonstrate_performance_tips(&client, &output).await?;
    
    output.print_success("Search examples completed!")?;
    
    Ok(())
}

async fn demonstrate_search(
    client: &ChronicleClient,
    output: &OutputManager,
    query: &chronictl::api::SearchQuery,
    description: &str,
) -> Result<()> {
    output.print_info(&format!("Executing: {}", description))?;
    output.print_key_value("Query", &query.query)?;
    
    if let Some(start) = &query.start_time {
        output.print_key_value("Start time", &start.to_string())?;
    }
    if let Some(end) = &query.end_time {
        output.print_key_value("End time", &end.to_string())?;
    }
    if let Some(filters) = &query.filters {
        for (key, value) in filters {
            output.print_key_value(&format!("Filter {}", key), value)?;
        }
    }
    
    match client.search(query).await {
        Ok(results) => {
            output.print_key_value("Results found", &results.total_count.to_string())?;
            output.print_key_value("Query time", &format!("{}ms", results.query_time_ms))?;
            
            if !results.events.is_empty() {
                output.print_info("Sample events:")?;
                output.print_search_results(&results)?;
            }
        }
        Err(e) => {
            output.print_warning(&format!("Search failed: {}", e))?;
        }
    }
    
    println!();
    Ok(())
}

async fn demonstrate_pagination(
    client: &ChronicleClient,
    output: &OutputManager,
) -> Result<()> {
    output.print_info("Demonstrating pagination with multiple requests:")?;
    
    let page_size = 3;
    let mut offset = 0;
    let mut total_seen = 0;
    
    for page in 1..=3 {
        output.print_info(&format!("Page {}: offset {}, limit {}", page, offset, page_size))?;
        
        let page_query = SearchQueryBuilder::new("*")
            .with_limit(page_size)
            .with_offset(offset)
            .build();
        
        match client.search(&page_query).await {
            Ok(results) => {
                output.print_key_value("Events on this page", &results.events.len().to_string())?;
                total_seen += results.events.len();
                output.print_key_value("Total seen so far", &total_seen.to_string())?;
                output.print_key_value("Total available", &results.total_count.to_string())?;
                
                if results.events.is_empty() {
                    output.print_info("No more events available")?;
                    break;
                }
                
                // Show first event from this page
                if let Some(first_event) = results.events.first() {
                    output.print_key_value(
                        "First event on page", 
                        &format!("{} ({})", first_event.id, first_event.event_type)
                    )?;
                }
                
                offset += page_size;
            }
            Err(e) => {
                output.print_error(&format!("Page {} failed: {}", page, e))?;
                break;
            }
        }
        
        println!();
    }
    
    Ok(())
}

async fn demonstrate_search_validation() -> Result<()> {
    let output = OutputManager::new(OutputFormat::Table, true);
    let mut validator = QueryValidator::new();
    
    output.print_info("Search validation examples:")?;
    
    // Valid queries
    let valid_queries = [
        "simple search",
        "/regex.*/",
        "complex AND query",
        "event_type:error",
        "timestamp:[now-1h TO now]",
    ];
    
    for query in &valid_queries {
        match validator.validate_query(query) {
            Ok(()) => {
                output.print_success(&format!("Valid: '{}'", query))?;
            }
            Err(e) => {
                output.print_error(&format!("Unexpected validation error for '{}': {}", query, e))?;
            }
        }
    }
    
    // Invalid queries
    let invalid_queries = [
        "",  // Empty query
        "/[/",  // Invalid regex
        "a".repeat(1001),  // Too long
    ];
    
    for query in &invalid_queries {
        match validator.validate_query(query) {
            Ok(()) => {
                output.print_warning(&format!("Unexpectedly valid: '{}'", query))?;
            }
            Err(e) => {
                output.print_info(&format!("Correctly invalid: '{}' - {}", 
                    if query.len() > 50 { &query[..50] } else { query }, e))?;
            }
        }
    }
    
    println!();
    Ok(())
}

async fn demonstrate_performance_tips(
    client: &ChronicleClient,
    output: &OutputManager,
) -> Result<()> {
    output.print_info("Performance optimization examples:")?;
    
    // Tip 1: Use time ranges to limit search scope
    output.print_info("Tip 1: Use time ranges to limit search scope")?;
    let scoped_search = SearchQueryBuilder::new("error")
        .with_time_str("last-hour")?
        .with_limit(100)
        .build();
    
    let start_time = std::time::Instant::now();
    match client.search(&scoped_search).await {
        Ok(results) => {
            let elapsed = start_time.elapsed();
            output.print_key_value("Scoped search time", &format!("{:?}", elapsed))?;
            output.print_key_value("Results", &results.total_count.to_string())?;
        }
        Err(e) => {
            output.print_warning(&format!("Scoped search failed: {}", e))?;
        }
    }
    
    // Tip 2: Use specific filters instead of broad text search
    output.print_info("Tip 2: Use specific filters instead of broad text search")?;
    let filtered_search = SearchQueryBuilder::new("*")
        .with_filter("event_type", "error")
        .with_filter("severity", "high")
        .with_limit(100)
        .build();
    
    let start_time = std::time::Instant::now();
    match client.search(&filtered_search).await {
        Ok(results) => {
            let elapsed = start_time.elapsed();
            output.print_key_value("Filtered search time", &format!("{:?}", elapsed))?;
            output.print_key_value("Results", &results.total_count.to_string())?;
        }
        Err(e) => {
            output.print_warning(&format!("Filtered search failed: {}", e))?;
        }
    }
    
    // Tip 3: Use appropriate limit sizes
    output.print_info("Tip 3: Use appropriate limit sizes")?;
    output.print_info("  • Small limits (10-100) for interactive use")?;
    output.print_info("  • Larger limits (1000+) for batch processing")?;
    output.print_info("  • Use pagination for very large result sets")?;
    
    // Tip 4: Index-friendly query patterns
    output.print_info("Tip 4: Index-friendly query patterns")?;
    output.print_info("  • Prefer exact matches over wildcards when possible")?;
    output.print_info("  • Use field-specific filters: event_type:error")?;
    output.print_info("  • Combine multiple specific conditions")?;
    
    println!();
    Ok(())
}

/// Example of time range parsing
async fn demonstrate_time_ranges() -> Result<()> {
    let output = OutputManager::new(OutputFormat::Table, true);
    
    output.print_info("Time range parsing examples:")?;
    
    // Relative time ranges
    let relative_ranges = [
        "today",
        "yesterday", 
        "last-hour",
        "last-day",
        "last-week",
        "last-month",
    ];
    
    for range_str in &relative_ranges {
        match TimeRange::parse(range_str) {
            Ok(range) => {
                output.print_success(&format!(
                    "{}: {} to {}",
                    range_str,
                    range.start.format("%Y-%m-%d %H:%M:%S"),
                    range.end.format("%Y-%m-%d %H:%M:%S")
                ))?;
            }
            Err(e) => {
                output.print_error(&format!("Failed to parse '{}': {}", range_str, e))?;
            }
        }
    }
    
    // Absolute time ranges
    let absolute_ranges = [
        "2024-01-01..2024-01-02",
        "2024-01-01T10:00:00..2024-01-01T11:00:00",
        "2024-01-01 10:00:00 to 2024-01-01 11:00:00",
    ];
    
    println!();
    output.print_info("Absolute time range examples:")?;
    
    for range_str in &absolute_ranges {
        match TimeRange::parse(range_str) {
            Ok(range) => {
                output.print_success(&format!(
                    "{}: {} to {}",
                    range_str,
                    range.start.format("%Y-%m-%d %H:%M:%S"),
                    range.end.format("%Y-%m-%d %H:%M:%S")
                ))?;
            }
            Err(e) => {
                output.print_error(&format!("Failed to parse '{}': {}", range_str, e))?;
            }
        }
    }
    
    Ok(())
}