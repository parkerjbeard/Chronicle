use crate::api::ChronicleClient;
use crate::error::{ChronicleError, Result};
use crate::output::OutputManager;
use crate::search::{SearchQueryBuilder, QueryValidator, parse_filters, validate_limit, validate_offset};
use clap::Args;
use std::time::Duration;

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search query (supports regex with /pattern/ syntax)
    #[arg(short, long)]
    pub query: String,

    /// Time range for search (e.g., "2024-01-01..2024-01-02", "last-week", "today")
    #[arg(short, long)]
    pub time: Option<String>,

    /// Maximum number of results to return
    #[arg(short, long, default_value = "100")]
    pub limit: usize,

    /// Number of results to skip (for pagination)
    #[arg(long, default_value = "0")]
    pub offset: usize,

    /// Additional filters (key=value,key2=value2)
    #[arg(short, long)]
    pub filters: Option<String>,

    /// Sort order (newest, oldest)
    #[arg(long, default_value = "newest")]
    pub sort: String,

    /// Show search statistics
    #[arg(long)]
    pub stats: bool,

    /// Interactive mode for large result sets
    #[arg(short, long)]
    pub interactive: bool,

    /// Save results to file
    #[arg(long)]
    pub save: Option<String>,

    /// Timeout for search in seconds
    #[arg(long, default_value = "30")]
    pub timeout: u64,

    /// Show only event IDs
    #[arg(long)]
    pub ids_only: bool,

    /// Count only (don't return events)
    #[arg(short, long)]
    pub count: bool,
}

pub async fn run(args: SearchArgs, client: ChronicleClient, output: OutputManager) -> Result<()> {
    // Validate arguments
    validate_limit(args.limit)?;
    validate_offset(args.offset)?;
    
    if !matches!(args.sort.as_str(), "newest" | "oldest") {
        return Err(ChronicleError::InvalidQuery(
            "Sort order must be 'newest' or 'oldest'".to_string(),
        ));
    }

    // Validate query
    let mut query_validator = QueryValidator::new();
    query_validator.validate_query(&args.query)?;

    // Build search query
    let mut query_builder = SearchQueryBuilder::new(&args.query)
        .with_limit(args.limit)
        .with_offset(args.offset);

    // Add time range if specified
    if let Some(time_str) = &args.time {
        query_builder = query_builder.with_time_str(time_str)?;
    }

    // Add filters if specified
    if let Some(filters_str) = &args.filters {
        let filters = parse_filters(filters_str)?;
        for (key, value) in filters {
            query_builder = query_builder.with_filter(&key, &value);
        }
    }

    let search_query = query_builder.build();

    // Set up client with timeout
    let client = client.with_timeout(Duration::from_secs(args.timeout));

    // Show search parameters
    if args.stats {
        output.print_info("Search Parameters:")?;
        output.print_key_value("Query", &search_query.query)?;
        if let Some(start_time) = &search_query.start_time {
            output.print_key_value("Start time", &start_time.to_string())?;
        }
        if let Some(end_time) = &search_query.end_time {
            output.print_key_value("End time", &end_time.to_string())?;
        }
        output.print_key_value("Limit", &args.limit.to_string())?;
        output.print_key_value("Offset", &args.offset.to_string())?;
        println!();
    }

    // Execute search
    let spinner = output.create_spinner("Searching...");
    
    match client.search(&search_query).await {
        Ok(results) => {
            spinner.finish_with_message("✓ Search completed");
            
            // Handle count-only mode
            if args.count {
                output.print_key_value("Total events", &results.total_count.to_string())?;
                output.print_key_value("Query time", &format!("{}ms", results.query_time_ms))?;
                return Ok(());
            }

            // Handle IDs-only mode
            if args.ids_only {
                for event in &results.events {
                    println!("{}", event.id);
                }
                if args.stats {
                    println!();
                    output.print_key_value("Total events", &results.total_count.to_string())?;
                    output.print_key_value("Query time", &format!("{}ms", results.query_time_ms))?;
                }
                return Ok(());
            }

            // Handle interactive mode for large result sets
            if args.interactive && results.events.len() > 20 {
                return handle_interactive_results(&results, &output, &client, &search_query).await;
            }

            // Save results if requested
            if let Some(save_path) = &args.save {
                save_search_results(&results, save_path, &output).await?;
            }

            // Display results
            output.print_search_results(&results)?;

            // Show pagination info
            if results.has_more {
                output.print_info(&format!(
                    "Showing {} of {} results. Use --offset {} to see more.",
                    results.events.len(),
                    results.total_count,
                    args.offset + args.limit
                ))?;
            }

            Ok(())
        }
        Err(ChronicleError::InvalidQuery(msg)) => {
            spinner.finish_with_message("✗ Invalid query");
            output.print_error(&format!("Invalid query: {}", msg))?;
            output.print_info("Try 'chronictl search --help' for query syntax examples")?;
            std::process::exit(6);
        }
        Err(ChronicleError::Timeout) => {
            spinner.finish_with_message("✗ Search timeout");
            output.print_error("Search timed out")?;
            output.print_info("Try narrowing your search criteria or increasing the timeout")?;
            std::process::exit(124);
        }
        Err(e) => {
            spinner.finish_with_message("✗ Search failed");
            output.print_error(&format!("Search failed: {}", e))?;
            std::process::exit(e.exit_code());
        }
    }
}

async fn handle_interactive_results(
    results: &crate::api::SearchResult,
    output: &OutputManager,
    client: &ChronicleClient,
    original_query: &crate::api::SearchQuery,
) -> Result<()> {
    output.print_info(&format!(
        "Found {} results. Entering interactive mode.",
        results.total_count
    ))?;

    let mut current_offset = 0;
    let page_size = 20;

    loop {
        // Show current page
        let page_results = crate::api::SearchResult {
            events: results.events
                .iter()
                .skip(current_offset)
                .take(page_size)
                .cloned()
                .collect(),
            total_count: results.total_count,
            query_time_ms: results.query_time_ms,
            has_more: current_offset + page_size < results.events.len(),
        };

        output.print_search_results(&page_results)?;

        // Show navigation options
        println!();
        output.print_info("Navigation:")?;
        println!("  [n] Next page");
        println!("  [p] Previous page");
        println!("  [f] First page");
        println!("  [l] Last page");
        println!("  [s] Save results");
        println!("  [q] Quit");
        println!();

        let choice = output.prompt_input("Choose an option")?;
        
        match choice.to_lowercase().as_str() {
            "n" | "next" => {
                if current_offset + page_size < results.events.len() {
                    current_offset += page_size;
                } else {
                    output.print_warning("Already at the last page")?;
                }
            }
            "p" | "prev" | "previous" => {
                if current_offset >= page_size {
                    current_offset -= page_size;
                } else {
                    output.print_warning("Already at the first page")?;
                }
            }
            "f" | "first" => {
                current_offset = 0;
            }
            "l" | "last" => {
                let last_page_offset = (results.events.len() / page_size) * page_size;
                current_offset = if last_page_offset >= results.events.len() {
                    last_page_offset - page_size
                } else {
                    last_page_offset
                };
            }
            "s" | "save" => {
                let save_path = output.prompt_input("Enter file path to save results")?;
                save_search_results(results, &save_path, output).await?;
            }
            "q" | "quit" | "exit" => {
                break;
            }
            _ => {
                output.print_warning("Invalid option. Use n/p/f/l/s/q")?;
            }
        }
    }

    Ok(())
}

async fn save_search_results(
    results: &crate::api::SearchResult,
    save_path: &str,
    output: &OutputManager,
) -> Result<()> {
    let spinner = output.create_spinner("Saving results...");
    
    let path = std::path::Path::new(save_path);
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    
    match extension {
        "json" => {
            let json = serde_json::to_string_pretty(results)?;
            tokio::fs::write(path, json).await?;
        }
        "csv" => {
            let mut wtr = csv::Writer::from_path(path)?;
            wtr.write_record(&["timestamp", "type", "id", "data"])?;
            for event in &results.events {
                wtr.write_record(&[
                    event.timestamp.to_rfc3339(),
                    event.event_type.clone(),
                    event.id.clone(),
                    serde_json::to_string(&event.data)?,
                ])?;
            }
            wtr.flush()?;
        }
        _ => {
            // Default to JSON
            let json = serde_json::to_string_pretty(results)?;
            tokio::fs::write(path, json).await?;
        }
    }
    
    spinner.finish_with_message("✓ Results saved");
    output.print_success(&format!("Results saved to {}", save_path))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;

    #[test]
    fn test_search_args_validation() {
        assert!(validate_limit(100).is_ok());
        assert!(validate_limit(0).is_err());
        assert!(validate_limit(10001).is_err());
        
        assert!(validate_offset(100).is_ok());
        assert!(validate_offset(1000001).is_err());
    }

    #[test]
    fn test_query_validation() {
        let mut validator = QueryValidator::new();
        assert!(validator.validate_query("test").is_ok());
        assert!(validator.validate_query("/test.*/").is_ok());
        assert!(validator.validate_query("").is_err());
    }

    #[test]
    fn test_filter_parsing() {
        let filters = parse_filters("type=error,level=critical").unwrap();
        assert_eq!(filters.len(), 2);
        assert_eq!(filters.get("type"), Some(&"error".to_string()));
        assert_eq!(filters.get("level"), Some(&"critical".to_string()));
    }
}