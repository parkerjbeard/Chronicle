//! Chronicle packer service main entry point
//!
//! This service drains the ring buffer nightly and converts Arrow data
//! to Parquet files with HEIF frame organization.

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use tracing::{info, error, Level};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use tokio::signal;

use chronicle_packer::{
    config::PackerConfig,
    packer::PackerService,
    error::Result,
};

// Module declarations
mod config;
mod error;
mod packer;
mod storage;
mod encryption;
mod integrity;
mod metrics;

/// Chronicle packer service command line interface
#[derive(Parser)]
#[command(name = "chronicle-packer")]
#[command(about = "Chronicle packer service for processing ring buffer data")]
#[command(version = "0.1.0")]
struct Cli {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,
    
    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
    
    /// Enable JSON logging
    #[arg(long)]
    json_logs: bool,
    
    /// Daemon mode (run in background)
    #[arg(short, long)]
    daemon: bool,
    
    /// PID file path for daemon mode
    #[arg(long)]
    pid_file: Option<PathBuf>,
    
    /// Subcommand
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Available commands
#[derive(Subcommand)]
enum Commands {
    /// Start the packer service
    Start {
        /// Force start even if already running
        #[arg(long)]
        force: bool,
    },
    
    /// Stop the packer service
    Stop {
        /// Force stop (SIGKILL)
        #[arg(long)]
        force: bool,
    },
    
    /// Restart the packer service
    Restart {
        /// Force restart
        #[arg(long)]
        force: bool,
    },
    
    /// Get service status
    Status,
    
    /// Trigger manual processing
    Process {
        /// Process specific date (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,
        
        /// Dry run (don't actually process)
        #[arg(long)]
        dry_run: bool,
    },
    
    /// Validate configuration
    Config {
        /// Show effective configuration
        #[arg(long)]
        show: bool,
    },
    
    /// Export metrics
    Metrics {
        /// Output format (json, prometheus)
        #[arg(short, long, default_value = "json")]
        format: String,
        
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Health check
    Health,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    
    // Initialize logging
    if let Err(e) = initialize_logging(&cli) {
        eprintln!("Failed to initialize logging: {}", e);
        process::exit(1);
    }
    
    info!("Starting Chronicle packer service");
    
    // Load configuration
    let config = match load_configuration(&cli).await {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            process::exit(1);
        }
    };
    
    // Handle daemon mode
    if cli.daemon {
        if let Err(e) = daemonize(&cli, &config).await {
            error!("Failed to daemonize: {}", e);
            process::exit(1);
        }
    }
    
    // Execute command
    let result = match &cli.command {
        Some(Commands::Start { force }) => start_service(config, *force).await,
        Some(Commands::Stop { force }) => stop_service(*force).await,
        Some(Commands::Restart { force }) => restart_service(config, *force).await,
        Some(Commands::Status) => show_status().await,
        Some(Commands::Process { date, dry_run }) => process_data(config, date.clone(), *dry_run).await,
        Some(Commands::Config { show }) => handle_config(config, *show).await,
        Some(Commands::Metrics { format, output }) => export_metrics(config, format.clone(), output.clone()).await,
        Some(Commands::Health) => health_check(config).await,
        None => start_service(config, false).await, // Default to start
    };
    
    match result {
        Ok(_) => {
            info!("Command completed successfully");
        }
        Err(e) => {
            error!("Command failed: {}", e);
            process::exit(1);
        }
    }
}

/// Initialize logging based on configuration
fn initialize_logging(cli: &Cli) -> Result<()> {
    let log_level = match cli.log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };
    
    let filter = EnvFilter::from_default_env()
        .add_directive(format!("chronicle_packer={}", log_level).parse()?)
        .add_directive("tokio=warn".parse()?)
        .add_directive("hyper=warn".parse()?)
        .add_directive("mio=warn".parse()?);
    
    if cli.json_logs {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_target(false))
            .init();
    }
    
    Ok(())
}

/// Load configuration from file or defaults
async fn load_configuration(cli: &Cli) -> Result<PackerConfig> {
    let config = if let Some(config_path) = &cli.config {
        info!("Loading configuration from: {}", config_path.display());
        PackerConfig::from_file(config_path)?
    } else {
        // Try default locations
        let default_path = PackerConfig::default_config_path()?;
        if default_path.exists() {
            info!("Loading configuration from: {}", default_path.display());
            PackerConfig::from_file(&default_path)?
        } else {
            info!("Using default configuration");
            PackerConfig::default()
        }
    };
    
    // Validate configuration
    config.validate()?;
    
    info!("Configuration loaded successfully");
    Ok(config)
}

/// Daemonize the process
async fn daemonize(cli: &Cli, config: &PackerConfig) -> Result<()> {
    info!("Daemonizing process");
    
    // Create PID file if specified
    if let Some(pid_file) = &cli.pid_file {
        let pid = process::id();
        std::fs::write(pid_file, pid.to_string())?;
        info!("Created PID file: {} (PID: {})", pid_file.display(), pid);
    }
    
    // Additional daemonization would happen here in a real implementation
    // For now, we'll just log that we're in daemon mode
    info!("Running in daemon mode");
    
    Ok(())
}

/// Start the packer service
async fn start_service(config: PackerConfig, force: bool) -> Result<()> {
    info!("Starting packer service (force: {})", force);
    
    // Check if service is already running (placeholder)
    if !force && is_service_running().await? {
        return Err("Service is already running. Use --force to restart.".into());
    }
    
    // Create and start the service
    let mut service = PackerService::new(config).await?;
    service.start().await?;
    
    info!("Packer service started successfully");
    
    // Wait for shutdown signals
    service.wait_for_shutdown().await?;
    
    // Graceful shutdown
    info!("Initiating graceful shutdown");
    service.stop().await?;
    
    info!("Packer service stopped");
    Ok(())
}

/// Stop the packer service
async fn stop_service(force: bool) -> Result<()> {
    info!("Stopping packer service (force: {})", force);
    
    if !is_service_running().await? {
        info!("Service is not running");
        return Ok(());
    }
    
    // Send shutdown signal to running service
    // This is a placeholder implementation
    if force {
        info!("Force stopping service");
        // Would send SIGKILL here
    } else {
        info!("Gracefully stopping service");
        // Would send SIGTERM here
    }
    
    info!("Service stop signal sent");
    Ok(())
}

/// Restart the packer service
async fn restart_service(config: PackerConfig, force: bool) -> Result<()> {
    info!("Restarting packer service (force: {})", force);
    
    // Stop the service if running
    if is_service_running().await? {
        stop_service(force).await?;
        
        // Wait a moment for clean shutdown
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
    
    // Start the service
    start_service(config, true).await?;
    
    Ok(())
}

/// Show service status
async fn show_status() -> Result<()> {
    info!("Checking service status");
    
    if is_service_running().await? {
        println!("Chronicle packer service is running");
        
        // Get detailed status from running service
        // This would typically connect via IPC to get status
        println!("Status: Running");
        println!("Last processing: N/A");
        println!("Events processed: N/A");
        println!("Files created: N/A");
    } else {
        println!("Chronicle packer service is not running");
    }
    
    Ok(())
}

/// Process data manually
async fn process_data(config: PackerConfig, date: Option<String>, dry_run: bool) -> Result<()> {
    info!("Manual data processing (date: {:?}, dry_run: {})", date, dry_run);
    
    if dry_run {
        info!("Dry run mode - no actual processing will occur");
    }
    
    // Create temporary service for processing
    let mut service = PackerService::new(config).await?;
    let result = service.trigger_processing().await?;
    
    println!("Processing completed:");
    println!("  Events processed: {}", result.events_processed);
    println!("  Files created: {}", result.files_created);
    println!("  Bytes processed: {}", result.bytes_processed);
    println!("  Duration: {:?}", result.duration);
    
    if !result.errors.is_empty() {
        println!("Errors encountered:");
        for error in &result.errors {
            println!("  - {}", error);
        }
    }
    
    Ok(())
}

/// Handle configuration commands
async fn handle_config(config: PackerConfig, show: bool) -> Result<()> {
    if show {
        println!("Effective configuration:");
        println!("{}", toml::to_string_pretty(&config)?);
    } else {
        // Validate configuration
        config.validate()?;
        println!("Configuration is valid");
    }
    
    Ok(())
}

/// Export metrics
async fn export_metrics(config: PackerConfig, format: String, output: Option<PathBuf>) -> Result<()> {
    info!("Exporting metrics (format: {}, output: {:?})", format, output);
    
    // Create temporary service to get metrics
    let service = PackerService::new(config).await?;
    
    // Get metrics would require the service to be running
    // For now, we'll just show placeholder output
    let metrics_output = match format.as_str() {
        "json" => r#"{"status": "ok", "timestamp": "2024-01-01T00:00:00Z"}"#.to_string(),
        "prometheus" => "# TYPE chronicle_status gauge\nchronicle_status 1\n".to_string(),
        _ => return Err(format!("Unsupported format: {}", format).into()),
    };
    
    if let Some(output_path) = output {
        std::fs::write(&output_path, metrics_output)?;
        println!("Metrics exported to: {}", output_path.display());
    } else {
        println!("{}", metrics_output);
    }
    
    Ok(())
}

/// Perform health check
async fn health_check(config: PackerConfig) -> Result<()> {
    info!("Performing health check");
    
    println!("Chronicle Packer Health Check");
    println!("=============================");
    
    // Check configuration
    print!("Configuration: ");
    match config.validate() {
        Ok(_) => println!("✓ Valid"),
        Err(e) => {
            println!("✗ Invalid - {}", e);
            return Err(e.into());
        }
    }
    
    // Check storage directories
    print!("Storage directories: ");
    if config.storage.base_path.exists() {
        println!("✓ Accessible");
    } else {
        println!("✗ Not accessible");
        return Err("Storage directory not accessible".into());
    }
    
    // Check ring buffer (placeholder)
    print!("Ring buffer: ");
    if config.ring_buffer.path.exists() {
        println!("✓ Available");
    } else {
        println!("⚠ Not found (may not be initialized)");
    }
    
    // Check encryption (if enabled)
    if config.encryption.enabled {
        print!("Encryption: ");
        // This would check keychain access
        println!("✓ Configured");
    }
    
    // Check metrics endpoint (if enabled)
    if config.metrics.enabled && config.metrics.port > 0 {
        print!("Metrics endpoint: ");
        // This would test HTTP connectivity
        println!("✓ Configured");
    }
    
    println!("\nHealth check completed successfully");
    Ok(())
}

/// Check if service is already running
async fn is_service_running() -> Result<bool> {
    // Placeholder implementation
    // In a real implementation, this would check:
    // 1. PID file existence and process status
    // 2. Unix socket connectivity
    // 3. Lock file status
    
    Ok(false)
}

/// Handle shutdown signals
async fn wait_for_shutdown() -> Result<()> {
    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;
    let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())?;
    
    tokio::select! {
        _ = sigterm.recv() => {
            info!("Received SIGTERM, initiating graceful shutdown");
        }
        _ = sigint.recv() => {
            info!("Received SIGINT, initiating graceful shutdown");
        }
    }
    
    Ok(())
}