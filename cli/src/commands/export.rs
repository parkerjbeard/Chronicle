use crate::api::{ChronicleClient, ExportRequest, ExportResponse};
use crate::error::{ChronicleError, Result};
use crate::output::OutputManager;
use crate::search::{SearchQueryBuilder, parse_filters, validate_limit, validate_offset};
use crate::utils;
use clap::Args;
use std::path::Path;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[derive(Args, Debug)]
pub struct ExportArgs {
    /// Export format (json, csv, parquet, arrow)
    #[arg(short, long, default_value = "json")]
    pub format: String,

    /// Search query for data to export
    #[arg(short, long)]
    pub query: Option<String>,

    /// Time range for export (e.g., "2024-01-01..2024-01-02", "last-week", "today")
    #[arg(short, long)]
    pub time: Option<String>,

    /// Maximum number of records to export
    #[arg(short, long)]
    pub limit: Option<usize>,

    /// Additional filters (key=value,key2=value2)
    #[arg(long)]
    pub filters: Option<String>,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Compression format (gzip, bzip2, lz4)
    #[arg(short, long)]
    pub compression: Option<String>,

    /// Overwrite existing files
    #[arg(long)]
    pub overwrite: bool,

    /// Export in streaming mode (for large datasets)
    #[arg(long)]
    pub stream: bool,

    /// Timeout for export operation in seconds
    #[arg(long, default_value = "300")]
    pub timeout: u64,

    /// Show progress during export
    #[arg(long)]
    pub progress: bool,

    /// Validate exported data
    #[arg(long)]
    pub validate: bool,
}

pub async fn run(args: ExportArgs, client: ChronicleClient, output: OutputManager) -> Result<()> {
    // Validate arguments
    if let Some(limit) = args.limit {
        validate_limit(limit)?;
    }

    if !matches!(args.format.as_str(), "json" | "csv" | "parquet" | "arrow") {
        return Err(ChronicleError::InvalidQuery(
            "Format must be one of: json, csv, parquet, arrow".to_string(),
        ));
    }

    if let Some(compression) = &args.compression {
        if !matches!(compression.as_str(), "gzip" | "bzip2" | "lz4") {
            return Err(ChronicleError::InvalidQuery(
                "Compression must be one of: gzip, bzip2, lz4".to_string(),
            ));
        }
    }

    // Build search query
    let search_query = build_search_query(&args)?;

    // Determine output path
    let output_path = determine_output_path(&args)?;

    // Check if output file exists and handle overwrite
    if output_path.exists() && !args.overwrite {
        let overwrite = output.prompt_confirm(&format!(
            "File {} already exists. Overwrite?",
            output_path.display()
        ))?;
        if !overwrite {
            return Err(ChronicleError::Cancelled);
        }
    }

    // Create export request
    let export_request = ExportRequest {
        format: args.format.clone(),
        query: search_query,
        destination: Some(output_path.to_string_lossy().to_string()),
        compression: args.compression.clone(),
    };

    // Set up client with timeout
    let client = client.with_timeout(Duration::from_secs(args.timeout));

    // Execute export
    if args.stream {
        export_streaming(&export_request, &client, &output_path, &output, args.progress).await
    } else {
        export_batch(&export_request, &client, &output_path, &output, args.progress, args.validate).await
    }
}

fn build_search_query(args: &ExportArgs) -> Result<crate::api::SearchQuery> {
    let query_str = args.query.as_deref().unwrap_or("*");
    let mut query_builder = SearchQueryBuilder::new(query_str);

    if let Some(time_str) = &args.time {
        query_builder = query_builder.with_time_str(time_str)?;
    }

    if let Some(limit) = args.limit {
        query_builder = query_builder.with_limit(limit);
    }

    if let Some(filters_str) = &args.filters {
        let filters = parse_filters(filters_str)?;
        for (key, value) in filters {
            query_builder = query_builder.with_filter(&key, &value);
        }
    }

    Ok(query_builder.build())
}

fn determine_output_path(args: &ExportArgs) -> Result<std::path::PathBuf> {
    if let Some(output_path) = &args.output {
        return Ok(std::path::PathBuf::from(output_path));
    }

    // Generate default filename
    let timestamp = chrono::Utc::now();
    let filename = format!(
        "chronicle_export_{}.{}",
        utils::format_timestamp_filename(&timestamp),
        args.format
    );

    // Add compression extension if specified
    let filename = if let Some(compression) = &args.compression {
        format!("{}.{}", filename, compression)
    } else {
        filename
    };

    Ok(std::path::PathBuf::from(filename))
}

async fn export_batch(
    request: &ExportRequest,
    client: &ChronicleClient,
    output_path: &Path,
    output: &OutputManager,
    show_progress: bool,
    validate: bool,
) -> Result<()> {
    let spinner = output.create_spinner("Starting export...");
    
    // Initiate export
    let export_response = client.export(request).await?;
    spinner.finish_with_message("✓ Export initiated");

    output.print_key_value("Export ID", &export_response.export_id)?;
    output.print_key_value("Status", &export_response.status)?;

    // Poll for completion
    let mut progress_bar = None;
    if show_progress {
        progress_bar = Some(output.create_spinner("Exporting data..."));
    }

    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        let status = client.export_status(&export_response.export_id).await?;
        
        match status.status.as_str() {
            "completed" => {
                if let Some(pb) = progress_bar {
                    pb.finish_with_message("✓ Export completed");
                }
                
                // Download the exported file
                let download_spinner = output.create_spinner("Downloading export...");
                let mut response = client.download_export(&export_response.export_id).await?;
                
                let mut file = File::create(output_path).await?;
                
                while let Some(chunk) = response.chunk().await? {
                    file.write_all(&chunk).await?;
                }
                
                file.flush().await?;
                download_spinner.finish_with_message("✓ Download completed");
                
                // Validate if requested
                if validate {
                    validate_export(output_path, &request.format, output).await?;
                }
                
                let file_size = tokio::fs::metadata(output_path).await?.len();
                output.print_success(&format!(
                    "Export completed: {} ({})",
                    output_path.display(),
                    utils::format_bytes(file_size)
                ))?;
                
                break;
            }
            "failed" => {
                if let Some(pb) = progress_bar {
                    pb.finish_with_message("✗ Export failed");
                }
                return Err(ChronicleError::Export("Export failed".to_string()));
            }
            "cancelled" => {
                if let Some(pb) = progress_bar {
                    pb.finish_with_message("✗ Export cancelled");
                }
                return Err(ChronicleError::Cancelled);
            }
            _ => {
                // Still in progress
                if let Some(pb) = &progress_bar {
                    pb.set_message(format!("Exporting... ({})", status.status));
                }
            }
        }
    }

    Ok(())
}

async fn export_streaming(
    request: &ExportRequest,
    client: &ChronicleClient,
    output_path: &Path,
    output: &OutputManager,
    show_progress: bool,
) -> Result<()> {
    output.print_info("Streaming export not yet implemented")?;
    output.print_info("Falling back to batch export...")?;
    
    // For now, fall back to batch export
    export_batch(request, client, output_path, output, show_progress, false).await
}

async fn validate_export(
    output_path: &Path,
    format: &str,
    output: &OutputManager,
) -> Result<()> {
    let spinner = output.create_spinner("Validating export...");
    
    match format {
        "json" => {
            let content = tokio::fs::read_to_string(output_path).await?;
            serde_json::from_str::<serde_json::Value>(&content)?;
            spinner.finish_with_message("✓ JSON validation passed");
        }
        "csv" => {
            let mut reader = csv::Reader::from_path(output_path)?;
            let mut record_count = 0;
            for result in reader.records() {
                result?;
                record_count += 1;
            }
            spinner.finish_with_message(&format!("✓ CSV validation passed ({} records)", record_count));
        }
        "parquet" => {
            // For parquet validation, we'd need to read the file with arrow/parquet
            // For now, just check that the file exists and is not empty
            let metadata = tokio::fs::metadata(output_path).await?;
            if metadata.len() == 0 {
                return Err(ChronicleError::Validation("Parquet file is empty".to_string()));
            }
            spinner.finish_with_message("✓ Parquet validation passed");
        }
        "arrow" => {
            // Similar to parquet, we'd need arrow libraries for proper validation
            let metadata = tokio::fs::metadata(output_path).await?;
            if metadata.len() == 0 {
                return Err(ChronicleError::Validation("Arrow file is empty".to_string()));
            }
            spinner.finish_with_message("✓ Arrow validation passed");
        }
        _ => {
            spinner.finish_with_message("✓ Basic validation passed");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;
    use tempfile::NamedTempFile;

    #[test]
    fn test_determine_output_path() {
        let args = ExportArgs {
            format: "json".to_string(),
            query: None,
            time: None,
            limit: None,
            filters: None,
            output: Some("test.json".to_string()),
            compression: None,
            overwrite: false,
            stream: false,
            timeout: 300,
            progress: false,
            validate: false,
        };

        let path = determine_output_path(&args).unwrap();
        assert_eq!(path.to_string_lossy(), "test.json");
    }

    #[test]
    fn test_build_search_query() {
        let args = ExportArgs {
            format: "json".to_string(),
            query: Some("test".to_string()),
            time: None,
            limit: Some(100),
            filters: Some("type=error".to_string()),
            output: None,
            compression: None,
            overwrite: false,
            stream: false,
            timeout: 300,
            progress: false,
            validate: false,
        };

        let query = build_search_query(&args).unwrap();
        assert_eq!(query.query, "test");
        assert_eq!(query.limit, Some(100));
        assert!(query.filters.is_some());
    }

    #[tokio::test]
    async fn test_validate_json_export() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        // Write valid JSON
        tokio::fs::write(path, r#"{"test": "value"}"#).await.unwrap();
        
        let output = OutputManager::new(OutputFormat::Table, false);
        assert!(validate_export(path, "json", &output).await.is_ok());
    }

    #[tokio::test]
    async fn test_validate_csv_export() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        // Write valid CSV
        tokio::fs::write(path, "header1,header2\nvalue1,value2\n").await.unwrap();
        
        let output = OutputManager::new(OutputFormat::Table, false);
        assert!(validate_export(path, "csv", &output).await.is_ok());
    }
}