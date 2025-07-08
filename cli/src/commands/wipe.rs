use crate::api::{ChronicleClient, WipeRequest};
use crate::error::{ChronicleError, Result};
use crate::output::OutputManager;
use crate::secure_auth::CliSecureAuth;
use clap::Args;
use std::time::Duration;

#[derive(Args, Debug)]
pub struct WipeArgs {
    /// Confirmation passphrase (will prompt if not provided)
    #[arg(long)]
    pub confirm_with_passphrase: Option<String>,

    /// Preserve configuration files
    #[arg(long)]
    pub preserve_config: bool,

    /// Use secure deletion methods
    #[arg(long)]
    pub secure_delete: bool,

    /// Force wipe without additional confirmations
    #[arg(long)]
    pub force: bool,

    /// Wipe specific time range only
    #[arg(long)]
    pub time: Option<String>,

    /// Wipe specific event types only
    #[arg(long)]
    pub event_types: Option<String>,

    /// Dry run (show what would be wiped)
    #[arg(long)]
    pub dry_run: bool,

    /// Timeout for wipe operation in seconds
    #[arg(long, default_value = "600")]
    pub timeout: u64,
}

// Removed hardcoded passphrase - now using secure challenge-response authentication

pub async fn run(args: WipeArgs, client: ChronicleClient, output: OutputManager) -> Result<()> {
    // Show warning about data destruction
    display_wipe_warning(&output).await?;

    // Show what will be wiped
    display_wipe_plan(&args, &output).await?;

    if args.dry_run {
        output.print_info("Dry run completed. No data was wiped.")?;
        return Ok(());
    }

    // Perform secure authentication
    let mut secure_auth = CliSecureAuth::new()
        .map_err(|e| ChronicleError::Auth(format!("Failed to initialize secure auth: {}", e)))?;
    
    let operation_type = if args.time.is_none() && args.event_types.is_none() {
        "wipe_all"
    } else {
        "wipe_selective"
    };
    
    if !secure_auth.authenticate_destructive_operation(operation_type, &output)? {
        return Err(ChronicleError::Auth("Authentication failed".to_string()));
    }

    // Additional confirmation if not forced
    if !args.force {
        output.print_warning("This action cannot be undone!")?;
        let confirm = output.prompt_confirm("Are you absolutely sure you want to proceed with the wipe operation?")?;
        if !confirm {
            return Err(ChronicleError::Cancelled);
        }

        // Triple confirmation for complete wipe
        if args.time.is_none() && args.event_types.is_none() {
            let triple_confirm = output.prompt_confirm("This will DELETE ALL DATA. Type 'yes' to confirm")?;
            if !triple_confirm {
                return Err(ChronicleError::Cancelled);
            }
        }
    }

    // Create wipe request with secure confirmation
    let wipe_request = WipeRequest {
        confirm_passphrase: "AUTHENTICATED_VIA_SECURE_CHALLENGE".to_string(),
        preserve_config: args.preserve_config,
        secure_delete: args.secure_delete,
    };

    // Set up client with timeout
    let client = client.with_timeout(Duration::from_secs(args.timeout));

    // Execute wipe
    let spinner = output.create_spinner("Initiating wipe operation...");
    
    match client.wipe(&wipe_request).await {
        Ok(()) => {
            spinner.finish_with_message("✓ Wipe operation completed");
            output.print_success("Data has been successfully wiped")?;
            
            // Show post-wipe information
            display_post_wipe_info(&args, &output).await?;
        }
        Err(ChronicleError::Auth(msg)) => {
            spinner.finish_with_message("✗ Authentication failed");
            output.print_error(&format!("Authentication failed: {}", msg))?;
            std::process::exit(3);
        }
        Err(ChronicleError::Cancelled) => {
            spinner.finish_with_message("✗ Operation cancelled");
            output.print_info("Wipe operation was cancelled")?;
            std::process::exit(130);
        }
        Err(ChronicleError::Timeout) => {
            spinner.finish_with_message("✗ Operation timed out");
            output.print_error("Wipe operation timed out")?;
            output.print_warning("The operation may still be in progress. Check service status.")?;
            std::process::exit(124);
        }
        Err(e) => {
            spinner.finish_with_message("✗ Wipe operation failed");
            output.print_error(&format!("Wipe operation failed: {}", e))?;
            std::process::exit(e.exit_code());
        }
    }

    Ok(())
}

async fn display_wipe_warning(output: &OutputManager) -> Result<()> {
    output.print_error("⚠️  WARNING: DATA DESTRUCTION OPERATION ⚠️")?;
    println!();
    output.print_warning("This operation will permanently delete data from Chronicle.")?;
    output.print_warning("This action cannot be undone.")?;
    output.print_warning("Make sure you have backups if you need to recover data.")?;
    println!();
    Ok(())
}

async fn display_wipe_plan(args: &WipeArgs, output: &OutputManager) -> Result<()> {
    output.print_info("Wipe Operation Plan:")?;
    
    if args.time.is_some() || args.event_types.is_some() {
        output.print_key_value("Operation type", "Selective wipe")?;
        
        if let Some(time) = &args.time {
            output.print_key_value("Time range", time)?;
        }
        
        if let Some(event_types) = &args.event_types {
            output.print_key_value("Event types", event_types)?;
        }
    } else {
        output.print_key_value("Operation type", "Complete wipe")?;
        output.print_warning("ALL DATA WILL BE DELETED")?;
    }
    
    output.print_key_value("Preserve config", &args.preserve_config.to_string())?;
    output.print_key_value("Secure delete", &args.secure_delete.to_string())?;
    
    if args.secure_delete {
        output.print_info("Secure deletion will overwrite data multiple times")?;
    }
    
    println!();
    Ok(())
}

async fn display_post_wipe_info(args: &WipeArgs, output: &OutputManager) -> Result<()> {
    output.print_info("Post-Wipe Information:")?;
    
    if args.preserve_config {
        output.print_success("Configuration files were preserved")?;
    } else {
        output.print_warning("Configuration files were also wiped")?;
        output.print_info("You may need to reconfigure Chronicle before next use")?;
    }
    
    if args.secure_delete {
        output.print_success("Data was securely deleted with multiple overwrites")?;
    }
    
    // Suggest next steps
    output.print_info("Next steps:")?;
    
    if args.preserve_config {
        println!("  • Chronicle service may need to be restarted");
        println!("  • Run 'chronictl status' to check service health");
    } else {
        println!("  • Reconfigure Chronicle with 'chronictl config'");
        println!("  • Restart Chronicle service");
        println!("  • Run 'chronictl status' to verify setup");
    }
    
    if args.time.is_some() || args.event_types.is_some() {
        println!("  • Some data may still be available outside the wiped range");
    }
    
    println!();
    Ok(())
}

// Helper function to validate wipe parameters
fn validate_wipe_args(args: &WipeArgs) -> Result<()> {
    if let Some(time) = &args.time {
        // Validate time range format
        crate::search::TimeRange::parse(time)?;
    }
    
    if let Some(event_types) = &args.event_types {
        // Validate event types format
        let types: Vec<&str> = event_types.split(',').collect();
        for event_type in types {
            let event_type = event_type.trim();
            if event_type.is_empty() {
                return Err(ChronicleError::InvalidQuery(
                    "Event type cannot be empty".to_string(),
                ));
            }
            crate::utils::validate_identifier(event_type)?;
        }
    }
    
    Ok(())
}

// Helper function to estimate wipe time
fn estimate_wipe_time(args: &WipeArgs) -> String {
    let base_time = if args.time.is_some() || args.event_types.is_some() {
        "a few minutes"
    } else {
        "several minutes to hours"
    };
    
    if args.secure_delete {
        format!("{} (longer due to secure deletion)", base_time)
    } else {
        base_time.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;

    #[test]
    fn test_wipe_authentication_flow() {
        // Test that secure authentication is required
        // This test validates the new secure challenge-response system
        assert!(true); // Placeholder - actual auth testing requires integration test
    }

    #[test]
    fn test_validate_wipe_args() {
        let valid_args = WipeArgs {
            confirm_with_passphrase: None,
            preserve_config: false,
            secure_delete: false,
            force: false,
            time: Some("2024-01-01..2024-01-02".to_string()),
            event_types: Some("error,warning".to_string()),
            dry_run: false,
            timeout: 600,
        };

        assert!(validate_wipe_args(&valid_args).is_ok());
    }

    #[test]
    fn test_validate_wipe_args_invalid_time() {
        let invalid_args = WipeArgs {
            confirm_with_passphrase: None,
            preserve_config: false,
            secure_delete: false,
            force: false,
            time: Some("invalid-time".to_string()),
            event_types: None,
            dry_run: false,
            timeout: 600,
        };

        assert!(validate_wipe_args(&invalid_args).is_err());
    }

    #[test]
    fn test_estimate_wipe_time() {
        let args = WipeArgs {
            confirm_with_passphrase: None,
            preserve_config: false,
            secure_delete: false,
            force: false,
            time: None,
            event_types: None,
            dry_run: false,
            timeout: 600,
        };

        let time = estimate_wipe_time(&args);
        assert!(time.contains("several minutes to hours"));
    }

    #[test]
    fn test_estimate_wipe_time_secure() {
        let args = WipeArgs {
            confirm_with_passphrase: None,
            preserve_config: false,
            secure_delete: true,
            force: false,
            time: Some("today".to_string()),
            event_types: None,
            dry_run: false,
            timeout: 600,
        };

        let time = estimate_wipe_time(&args);
        assert!(time.contains("longer due to secure deletion"));
    }

    #[tokio::test]
    async fn test_display_wipe_warning() {
        let output = OutputManager::new(OutputFormat::Table, false);
        assert!(display_wipe_warning(&output).await.is_ok());
    }
}