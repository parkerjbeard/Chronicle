use crate::api::ChronicleClient;
use crate::error::{ChronicleError, Result};
use crate::output::OutputManager;
use clap::Args;
use std::time::Duration;

#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Show detailed system information
    #[arg(long, short)]
    pub detailed: bool,

    /// Check connectivity only (ping)
    #[arg(long)]
    pub ping: bool,

    /// Timeout for status check in seconds
    #[arg(long, default_value = "10")]
    pub timeout: u64,

    /// Show raw JSON output
    #[arg(long)]
    pub raw: bool,
}

pub async fn run(args: StatusArgs, client: ChronicleClient, output: OutputManager) -> Result<()> {
    let client = client.with_timeout(Duration::from_secs(args.timeout));

    // If ping mode, just check connectivity
    if args.ping {
        return check_ping(&client, &output).await;
    }

    // Get health status
    let spinner = output.create_spinner("Checking Chronicle service status...");
    
    match client.health().await {
        Ok(health) => {
            spinner.finish_with_message("✓ Status check completed");
            
            if args.raw {
                println!("{}", serde_json::to_string_pretty(&health)?);
            } else {
                output.print_health_status(&health)?;
            }

            // Additional detailed checks if requested
            if args.detailed {
                output.print_info("Running detailed system checks...")?;
                run_detailed_checks(&client, &output).await?;
            }

            // Exit with appropriate code based on health status
            match health.status.as_str() {
                "healthy" => {
                    output.print_success("Chronicle service is healthy")?;
                    Ok(())
                }
                "degraded" => {
                    output.print_warning("Chronicle service is degraded")?;
                    std::process::exit(1);
                }
                "unhealthy" => {
                    output.print_error("Chronicle service is unhealthy")?;
                    std::process::exit(2);
                }
                _ => {
                    output.print_warning(&format!("Unknown health status: {}", health.status))?;
                    std::process::exit(3);
                }
            }
        }
        Err(ChronicleError::ServiceUnavailable) => {
            spinner.finish_with_message("✗ Service unavailable");
            output.print_error("Chronicle service is not running or not accessible")?;
            output.print_info("Try starting the Chronicle service or check your configuration")?;
            std::process::exit(10);
        }
        Err(ChronicleError::Timeout) => {
            spinner.finish_with_message("✗ Timeout");
            output.print_error("Status check timed out")?;
            output.print_info("The service may be overloaded or network connectivity is poor")?;
            std::process::exit(124);
        }
        Err(e) => {
            spinner.finish_with_message("✗ Error");
            output.print_error(&format!("Failed to get status: {}", e))?;
            std::process::exit(e.exit_code());
        }
    }
}

async fn check_ping(client: &ChronicleClient, output: &OutputManager) -> Result<()> {
    let spinner = output.create_spinner("Pinging Chronicle service...");
    
    match client.ping().await {
        Ok(()) => {
            spinner.finish_with_message("✓ Ping successful");
            output.print_success("Chronicle service is reachable")?;
            Ok(())
        }
        Err(ChronicleError::ServiceUnavailable) => {
            spinner.finish_with_message("✗ Ping failed");
            output.print_error("Chronicle service is not reachable")?;
            std::process::exit(10);
        }
        Err(ChronicleError::Timeout) => {
            spinner.finish_with_message("✗ Ping timeout");
            output.print_error("Ping timed out")?;
            std::process::exit(124);
        }
        Err(e) => {
            spinner.finish_with_message("✗ Ping error");
            output.print_error(&format!("Ping failed: {}", e))?;
            std::process::exit(e.exit_code());
        }
    }
}

async fn run_detailed_checks(client: &ChronicleClient, output: &OutputManager) -> Result<()> {
    println!();
    output.print_info("Detailed System Checks:")?;
    
    // Check configuration
    let config_spinner = output.create_spinner("Checking configuration...");
    match client.config().await {
        Ok(config_info) => {
            config_spinner.finish_with_message("✓ Configuration accessible");
            output.print_key_value("Config file", &config_info.config_file)?;
            output.print_key_value("Last modified", &config_info.last_modified.to_string())?;
        }
        Err(e) => {
            config_spinner.finish_with_message("✗ Configuration check failed");
            output.print_warning(&format!("Configuration check failed: {}", e))?;
        }
    }

    // Test search functionality
    let search_spinner = output.create_spinner("Testing search functionality...");
    let test_query = crate::api::SearchQuery {
        query: "test".to_string(),
        start_time: None,
        end_time: None,
        limit: Some(1),
        offset: None,
        filters: None,
    };
    
    match client.search(&test_query).await {
        Ok(_) => {
            search_spinner.finish_with_message("✓ Search functionality working");
        }
        Err(e) => {
            search_spinner.finish_with_message("✗ Search test failed");
            output.print_warning(&format!("Search test failed: {}", e))?;
        }
    }

    // Check disk space (if we can access local file system)
    if let Ok(data_dir) = crate::utils::get_data_dir() {
        let disk_spinner = output.create_spinner("Checking disk space...");
        match crate::utils::get_available_space(&data_dir) {
            Ok(available) => {
                disk_spinner.finish_with_message("✓ Disk space check completed");
                output.print_key_value("Available space", &crate::utils::format_bytes(available))?;
            }
            Err(e) => {
                disk_spinner.finish_with_message("✗ Disk space check failed");
                output.print_warning(&format!("Disk space check failed: {}", e))?;
            }
        }
    }

    // Check permissions
    let perm_spinner = output.create_spinner("Checking permissions...");
    match crate::utils::check_permissions() {
        Ok(()) => {
            perm_spinner.finish_with_message("✓ Permissions OK");
        }
        Err(e) => {
            perm_spinner.finish_with_message("✗ Permission issues detected");
            output.print_warning(&format!("Permission issues: {}", e))?;
        }
    }

    println!();
    output.print_info("Detailed checks completed")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{HealthStatus, StorageInfo, MemoryInfo};
    use crate::output::OutputFormat;
    use chrono::Utc;

    #[tokio::test]
    async fn test_status_display() {
        let health = HealthStatus {
            status: "healthy".to_string(),
            version: "1.0.0".to_string(),
            uptime: 3600,
            storage_usage: StorageInfo {
                total_bytes: 1024 * 1024 * 1024,
                used_bytes: 512 * 1024 * 1024,
                available_bytes: 512 * 1024 * 1024,
                usage_percent: 50.0,
            },
            memory_usage: MemoryInfo {
                total_bytes: 8 * 1024 * 1024 * 1024,
                used_bytes: 4 * 1024 * 1024 * 1024,
                available_bytes: 4 * 1024 * 1024 * 1024,
                usage_percent: 50.0,
            },
            active_connections: 10,
            last_event_time: Some(Utc::now()),
        };

        let output = OutputManager::new(OutputFormat::Table, false);
        assert!(output.print_health_status(&health).is_ok());
    }
}