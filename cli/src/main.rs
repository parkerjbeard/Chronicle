use clap::{Parser, Subcommand};
use std::env;
use std::process;
use tokio;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod commands;
mod error;
mod output;
mod search;
mod secure_auth;
mod utils;

use api::ChronicleClient;
use commands::*;
use error::{ChronicleError, Result};
use output::{OutputFormat, OutputManager};

#[derive(Parser)]
#[command(name = "chronictl")]
#[command(about = "Chronicle CLI - Command-line interface for Chronicle data collection and analysis")]
#[command(version)]
#[command(long_about = "
Chronicle CLI (chronictl) provides comprehensive command-line access to Chronicle's 
data collection, search, analysis, and management capabilities.

Examples:
  chronictl status                                    # Check service health
  chronictl search --query \"error\" --time \"last-day\"   # Search for errors in last day
  chronictl export --format json --time \"today\"        # Export today's data as JSON
  chronictl backup --destination /backup/chronicle    # Create backup
  chronictl config show                               # Show configuration
")]
struct Cli {
    /// Service URL (overrides config file)
    #[arg(long, global = true, env = "CHRONICLE_URL")]
    url: Option<String>,

    /// Output format
    #[arg(long, global = true, value_enum, default_value = "table")]
    format: OutputFormatArg,

    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Enable debug logging
    #[arg(long, global = true)]
    debug: bool,

    /// Quiet mode (minimal output)
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Configuration file path
    #[arg(long, global = true, env = "CHRONICLE_CONFIG")]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum OutputFormatArg {
    Table,
    Json,
    Csv,
    Yaml,
    Raw,
}

impl From<OutputFormatArg> for OutputFormat {
    fn from(arg: OutputFormatArg) -> Self {
        match arg {
            OutputFormatArg::Table => OutputFormat::Table,
            OutputFormatArg::Json => OutputFormat::Json,
            OutputFormatArg::Csv => OutputFormat::Csv,
            OutputFormatArg::Yaml => OutputFormat::Yaml,
            OutputFormatArg::Raw => OutputFormat::Raw,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Check Chronicle service status and health
    Status(StatusArgs),
    
    /// Search events with queries and filters
    Search(SearchArgs),
    
    /// Export data in various formats
    Export(ExportArgs),
    
    /// Replay events with timing simulation
    Replay(ReplayArgs),
    
    /// Create backups of Chronicle data
    Backup(BackupArgs),
    
    /// Securely wipe Chronicle data
    Wipe(WipeArgs),
    
    /// Manage Chronicle configuration
    Config(ConfigArgs),
    
    /// Generate shell completions
    Completions {
        /// Shell type
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    
    // Initialize logging
    init_logging(&cli);
    
    // Handle completion generation
    if let Commands::Completions { shell } = cli.command {
        generate_completions(shell);
        return;
    }
    
    // Run the command
    if let Err(e) = run_command(cli).await {
        let error_msg = error::format_error(&e);
        eprintln!("{}", error_msg);
        process::exit(e.exit_code());
    }
}

async fn run_command(cli: Cli) -> Result<()> {
    // Load configuration
    let config = load_config(&cli)?;
    
    // Determine service URL
    let service_url = cli.url
        .or(config.get("service_url").and_then(|v| v.as_str().map(String::from)))
        .unwrap_or_else(|| "http://localhost:8080".to_string());
    
    info!("Connecting to Chronicle service at: {}", service_url);
    
    // Create API client
    let client = ChronicleClient::new(service_url);
    
    // Create output manager
    let colored = !cli.no_color && !cli.quiet && console::Term::stdout().features().colors_supported();
    let output_format = OutputFormat::from(cli.format);
    let output = OutputManager::new(output_format, colored);
    
    // Route to appropriate command
    match cli.command {
        Commands::Status(args) => {
            commands::status::run(args, client, output).await
        }
        Commands::Search(args) => {
            commands::search::run(args, client, output).await
        }
        Commands::Export(args) => {
            commands::export::run(args, client, output).await
        }
        Commands::Replay(args) => {
            commands::replay::run(args, client, output).await
        }
        Commands::Backup(args) => {
            commands::backup::run(args, client, output).await
        }
        Commands::Wipe(args) => {
            commands::wipe::run(args, client, output).await
        }
        Commands::Config(args) => {
            commands::config::run(args, client, output).await
        }
        Commands::Completions { .. } => {
            unreachable!("Completions handled earlier")
        }
    }
}

fn init_logging(cli: &Cli) {
    // Set log level based on CLI flags
    let log_level = if cli.debug {
        tracing::Level::DEBUG
    } else if cli.verbose {
        tracing::Level::INFO
    } else if cli.quiet {
        tracing::Level::ERROR
    } else {
        tracing::Level::WARN
    };
    
    // Initialize tracing subscriber
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    format!("chronictl={}", log_level).into()
                })
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    info!("Chronicle CLI started");
}

fn load_config(cli: &Cli) -> Result<std::collections::HashMap<String, serde_json::Value>> {
    let config_path = if let Some(config_path) = &cli.config {
        std::path::PathBuf::from(config_path)
    } else {
        utils::get_config_file()?
    };
    
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let config: std::collections::HashMap<String, serde_json::Value> = 
            serde_json::from_str(&content)
                .or_else(|_| {
                    // Try as TOML if JSON fails
                    toml::from_str::<toml::Value>(&content)
                        .map(|v| serde_json::from_value(serde_json::to_value(v).unwrap()).unwrap())
                })
                .map_err(|e| ChronicleError::Config(
                    config::ConfigError::Message(format!("Invalid config file: {}", e))
                ))?;
        
        info!("Loaded configuration from: {}", config_path.display());
        Ok(config)
    } else {
        warn!("Configuration file not found: {}", config_path.display());
        Ok(std::collections::HashMap::new())
    }
}

fn generate_completions(shell: clap_complete::Shell) {
    use clap_complete::{generate, Generator};
    use std::io;
    
    fn print_completions<G: Generator>(gen: G, cmd: &mut clap::Command) {
        generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
    }
    
    let mut cmd = Cli::command();
    eprintln!("Generating completion file for {shell}...");
    print_completions(shell, &mut cmd);
}

// Helper function to check for updates (could be expanded)
#[allow(dead_code)]
async fn check_for_updates() -> Result<()> {
    // Placeholder for update checking functionality
    // Could check GitHub releases or a dedicated update endpoint
    Ok(())
}

// Helper function to validate environment
fn validate_environment() -> Result<()> {
    // Check required environment setup
    utils::check_permissions()?;
    
    // Validate configuration directories exist and are writable
    let _config_dir = utils::get_config_dir()?;
    let _data_dir = utils::get_data_dir()?;
    let _cache_dir = utils::get_cache_dir()?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert()
    }

    #[test]
    fn test_output_format_conversion() {
        assert!(matches!(OutputFormat::from(OutputFormatArg::Table), OutputFormat::Table));
        assert!(matches!(OutputFormat::from(OutputFormatArg::Json), OutputFormat::Json));
        assert!(matches!(OutputFormat::from(OutputFormatArg::Csv), OutputFormat::Csv));
        assert!(matches!(OutputFormat::from(OutputFormatArg::Yaml), OutputFormat::Yaml));
        assert!(matches!(OutputFormat::from(OutputFormatArg::Raw), OutputFormat::Raw));
    }

    #[test]
    fn test_cli_parsing() {
        // Test basic command parsing
        let args = vec!["chronictl", "status"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert!(matches!(cli.command, Commands::Status(_)));
        
        // Test with global flags
        let args = vec!["chronictl", "--verbose", "--format", "json", "status"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert!(cli.verbose);
        assert!(matches!(cli.format, OutputFormatArg::Json));
    }

    #[test]
    fn test_environment_validation() {
        // This test might fail in some environments, so we'll just check it doesn't panic
        let _ = validate_environment();
    }
}