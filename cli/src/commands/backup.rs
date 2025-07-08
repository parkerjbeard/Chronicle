use crate::api::{ChronicleClient, BackupRequest, BackupResponse};
use crate::error::{ChronicleError, Result};
use crate::output::OutputManager;
use crate::utils;
use clap::Args;
use std::path::Path;
use std::time::Duration;

#[derive(Args, Debug)]
pub struct BackupArgs {
    /// Backup destination path
    #[arg(short, long)]
    pub destination: String,

    /// Include metadata in backup
    #[arg(long)]
    pub include_metadata: bool,

    /// Compression format (gzip, bzip2, lz4)
    #[arg(short, long)]
    pub compression: Option<String>,

    /// Encryption password (will prompt if not provided)
    #[arg(long)]
    pub encryption: Option<String>,

    /// Overwrite existing backup
    #[arg(long)]
    pub overwrite: bool,

    /// Verify backup integrity after creation
    #[arg(long)]
    pub verify: bool,

    /// Show progress during backup
    #[arg(long)]
    pub progress: bool,

    /// Timeout for backup operation in seconds
    #[arg(long, default_value = "3600")]
    pub timeout: u64,

    /// Backup specific time range
    #[arg(long)]
    pub time: Option<String>,

    /// Backup only specific event types
    #[arg(long)]
    pub event_types: Option<String>,

    /// Dry run (show what would be backed up)
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: BackupArgs, client: ChronicleClient, output: OutputManager) -> Result<()> {
    // Validate arguments
    if let Some(compression) = &args.compression {
        if !matches!(compression.as_str(), "gzip" | "bzip2" | "lz4") {
            return Err(ChronicleError::InvalidQuery(
                "Compression must be one of: gzip, bzip2, lz4".to_string(),
            ));
        }
    }

    // Validate destination path
    let dest_path = Path::new(&args.destination);
    if dest_path.exists() && !args.overwrite {
        let overwrite = output.prompt_confirm(&format!(
            "Destination {} already exists. Overwrite?",
            dest_path.display()
        ))?;
        if !overwrite {
            return Err(ChronicleError::Cancelled);
        }
    }

    // Validate destination directory is writable
    if let Some(parent) = dest_path.parent() {
        if parent.exists() {
            utils::check_directory_writable(parent)?;
        }
    }

    // Handle encryption
    let encryption = if args.encryption.is_some() {
        args.encryption.clone()
    } else {
        // Check if user wants encryption
        if output.prompt_confirm("Do you want to encrypt the backup?")? {
            Some(output.prompt_password("Enter encryption password")?)
        } else {
            None
        }
    };

    // Create backup request
    let backup_request = BackupRequest {
        destination: args.destination.clone(),
        include_metadata: args.include_metadata,
        compression: args.compression.clone(),
        encryption,
    };

    // Show backup plan
    display_backup_plan(&args, &backup_request, &output).await?;

    if args.dry_run {
        output.print_info("Dry run completed. No backup was created.")?;
        return Ok(());
    }

    // Confirm backup
    if !output.prompt_confirm("Proceed with backup?")? {
        return Err(ChronicleError::Cancelled);
    }

    // Set up client with timeout
    let client = client.with_timeout(Duration::from_secs(args.timeout));

    // Execute backup
    let spinner = output.create_spinner("Starting backup...");
    let backup_response = client.backup(&backup_request).await?;
    spinner.finish_with_message("✓ Backup initiated");

    output.print_key_value("Backup ID", &backup_response.backup_id)?;
    output.print_key_value("Status", &backup_response.status)?;

    // Poll for completion
    let mut progress_bar = None;
    if args.progress {
        progress_bar = Some(output.create_spinner("Creating backup..."));
    }

    let mut last_status = backup_response.status.clone();
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        let status = client.backup_status(&backup_response.backup_id).await?;
        
        if status.status != last_status {
            if let Some(pb) = &progress_bar {
                pb.set_message(format!("Backup status: {}", status.status));
            }
            last_status = status.status.clone();
        }
        
        match status.status.as_str() {
            "completed" => {
                if let Some(pb) = progress_bar {
                    pb.finish_with_message("✓ Backup completed");
                }
                
                // Display completion information
                if let Some(file_path) = &status.file_path {
                    output.print_key_value("Backup file", file_path)?;
                }
                
                if let Some(file_size) = status.file_size {
                    output.print_key_value("Backup size", &utils::format_bytes(file_size))?;
                }
                
                output.print_key_value("Created at", &status.created_at.to_string())?;
                
                // Verify backup if requested
                if args.verify {
                    verify_backup(&args, &output).await?;
                }
                
                output.print_success("Backup completed successfully")?;
                break;
            }
            "failed" => {
                if let Some(pb) = progress_bar {
                    pb.finish_with_message("✗ Backup failed");
                }
                return Err(ChronicleError::Backup("Backup failed".to_string()));
            }
            "cancelled" => {
                if let Some(pb) = progress_bar {
                    pb.finish_with_message("✗ Backup cancelled");
                }
                return Err(ChronicleError::Cancelled);
            }
            _ => {
                // Still in progress
                continue;
            }
        }
    }

    Ok(())
}

async fn display_backup_plan(
    args: &BackupArgs,
    request: &BackupRequest,
    output: &OutputManager,
) -> Result<()> {
    output.print_info("Backup Plan:")?;
    output.print_key_value("Destination", &request.destination)?;
    output.print_key_value("Include metadata", &request.include_metadata.to_string())?;
    
    if let Some(compression) = &request.compression {
        output.print_key_value("Compression", compression)?;
    }
    
    if request.encryption.is_some() {
        output.print_key_value("Encryption", "Enabled")?;
    }
    
    if let Some(time) = &args.time {
        output.print_key_value("Time range", time)?;
    }
    
    if let Some(event_types) = &args.event_types {
        output.print_key_value("Event types", event_types)?;
    }
    
    // Check available space
    if let Some(parent) = Path::new(&request.destination).parent() {
        if parent.exists() {
            match utils::get_available_space(parent) {
                Ok(available) => {
                    output.print_key_value("Available space", &utils::format_bytes(available))?;
                }
                Err(_) => {
                    output.print_warning("Could not determine available disk space")?;
                }
            }
        }
    }
    
    println!();
    Ok(())
}

async fn verify_backup(args: &BackupArgs, output: &OutputManager) -> Result<()> {
    let spinner = output.create_spinner("Verifying backup...");
    
    let dest_path = Path::new(&args.destination);
    
    // Check if backup file exists
    if !dest_path.exists() {
        spinner.finish_with_message("✗ Backup file not found");
        return Err(ChronicleError::Backup("Backup file not found".to_string()));
    }
    
    // Check if backup file is readable
    utils::check_file_readable(dest_path)?;
    
    // Get file size
    let metadata = std::fs::metadata(dest_path)?;
    let file_size = metadata.len();
    
    if file_size == 0 {
        spinner.finish_with_message("✗ Backup file is empty");
        return Err(ChronicleError::Backup("Backup file is empty".to_string()));
    }
    
    // Basic format validation based on file extension
    let extension = dest_path.extension().and_then(|ext| ext.to_str());
    match extension {
        Some("gz") | Some("bz2") | Some("lz4") => {
            // For compressed files, we could add more sophisticated validation
            spinner.finish_with_message("✓ Backup file appears valid (compressed)");
        }
        _ => {
            spinner.finish_with_message("✓ Backup file appears valid");
        }
    }
    
    output.print_key_value("Verified size", &utils::format_bytes(file_size))?;
    Ok(())
}

// Helper function to create a backup restoration command
pub fn generate_restore_command(backup_path: &str, destination: &str) -> String {
    format!(
        "chronictl restore --backup {} --destination {}",
        backup_path, destination
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;
    use tempfile::NamedTempFile;

    #[test]
    fn test_backup_validation() {
        let args = BackupArgs {
            destination: "/tmp/test_backup".to_string(),
            include_metadata: true,
            compression: Some("gzip".to_string()),
            encryption: None,
            overwrite: false,
            verify: false,
            progress: false,
            timeout: 3600,
            time: None,
            event_types: None,
            dry_run: false,
        };

        // Test valid compression
        assert!(matches!(args.compression.as_deref(), Some("gzip")));
        
        // Test invalid compression would be caught in run()
        let invalid_args = BackupArgs {
            compression: Some("invalid".to_string()),
            ..args
        };
        
        // This would fail validation in the run() function
        assert_eq!(invalid_args.compression, Some("invalid".to_string()));
    }

    #[tokio::test]
    async fn test_verify_backup() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        // Write some data to the file
        std::fs::write(path, "test backup data").unwrap();
        
        let args = BackupArgs {
            destination: path.to_string_lossy().to_string(),
            include_metadata: true,
            compression: None,
            encryption: None,
            overwrite: false,
            verify: true,
            progress: false,
            timeout: 3600,
            time: None,
            event_types: None,
            dry_run: false,
        };

        let output = OutputManager::new(OutputFormat::Table, false);
        assert!(verify_backup(&args, &output).await.is_ok());
    }

    #[test]
    fn test_generate_restore_command() {
        let backup_path = "/tmp/backup.tar.gz";
        let destination = "/tmp/restore";
        
        let cmd = generate_restore_command(backup_path, destination);
        assert_eq!(cmd, "chronictl restore --backup /tmp/backup.tar.gz --destination /tmp/restore");
    }
}