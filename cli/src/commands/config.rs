use crate::api::ChronicleClient;
use crate::error::{ChronicleError, Result};
use crate::output::OutputManager;
use crate::utils;
use clap::Args;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

#[derive(Args, Debug)]
pub struct ConfigArgs {
    /// Configuration action
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(clap::Subcommand, Debug)]
pub enum ConfigAction {
    /// Show current configuration
    Show {
        /// Show only specific key
        #[arg(short, long)]
        key: Option<String>,
        
        /// Show configuration file path
        #[arg(long)]
        path: bool,
    },
    
    /// Set configuration value
    Set {
        /// Configuration key
        key: String,
        
        /// Configuration value
        value: String,
        
        /// Value type (string, number, boolean, json)
        #[arg(long, default_value = "string")]
        value_type: String,
    },
    
    /// Get configuration value
    Get {
        /// Configuration key
        key: String,
    },
    
    /// Delete configuration key
    Delete {
        /// Configuration key
        key: String,
        
        /// Skip confirmation
        #[arg(long)]
        force: bool,
    },
    
    /// Validate configuration
    Validate {
        /// Configuration file path (optional)
        #[arg(short, long)]
        file: Option<String>,
    },
    
    /// Reset configuration to defaults
    Reset {
        /// Skip confirmation
        #[arg(long)]
        force: bool,
        
        /// Keep current values for specified keys
        #[arg(long)]
        keep: Option<String>,
    },
    
    /// Import configuration from file
    Import {
        /// Configuration file path
        file: String,
        
        /// Merge with existing configuration
        #[arg(long)]
        merge: bool,
    },
    
    /// Export configuration to file
    Export {
        /// Output file path
        file: String,
        
        /// Export format (json, toml, yaml)
        #[arg(long, default_value = "json")]
        format: String,
    },
}

pub async fn run(args: ConfigArgs, client: ChronicleClient, output: OutputManager) -> Result<()> {
    let client = client.with_timeout(Duration::from_secs(30));

    match args.action {
        ConfigAction::Show { key, path } => {
            show_config(&client, &output, key.as_deref(), path).await
        }
        ConfigAction::Set { key, value, value_type } => {
            set_config(&client, &output, &key, &value, &value_type).await
        }
        ConfigAction::Get { key } => {
            get_config(&client, &output, &key).await
        }
        ConfigAction::Delete { key, force } => {
            delete_config(&client, &output, &key, force).await
        }
        ConfigAction::Validate { file } => {
            validate_config(&client, &output, file.as_deref()).await
        }
        ConfigAction::Reset { force, keep } => {
            reset_config(&client, &output, force, keep.as_deref()).await
        }
        ConfigAction::Import { file, merge } => {
            import_config(&client, &output, &file, merge).await
        }
        ConfigAction::Export { file, format } => {
            export_config(&client, &output, &file, &format).await
        }
    }
}

async fn show_config(
    client: &ChronicleClient,
    output: &OutputManager,
    key: Option<&str>,
    show_path: bool,
) -> Result<()> {
    let spinner = output.create_spinner("Loading configuration...");
    
    match client.config().await {
        Ok(config_info) => {
            spinner.finish_with_message("✓ Configuration loaded");
            
            if show_path {
                output.print_key_value("Configuration file", &config_info.config_file)?;
                output.print_key_value("Last modified", &config_info.last_modified.to_string())?;
                println!();
            }
            
            if let Some(key) = key {
                // Show specific key
                if let Some(value) = config_info.config.get(key) {
                    output.print_key_value(key, &value.to_string())?;
                } else {
                    output.print_warning(&format!("Configuration key '{}' not found", key))?;
                }
            } else {
                // Show all configuration
                output.print_config(&config_info.config)?;
            }
        }
        Err(e) => {
            spinner.finish_with_message("✗ Failed to load configuration");
            output.print_error(&format!("Failed to load configuration: {}", e))?;
            std::process::exit(e.exit_code());
        }
    }
    
    Ok(())
}

async fn set_config(
    client: &ChronicleClient,
    output: &OutputManager,
    key: &str,
    value: &str,
    value_type: &str,
) -> Result<()> {
    // Validate key
    utils::validate_identifier(key)?;
    
    // Parse value based on type
    let parsed_value = parse_config_value(value, value_type)?;
    
    // Show what will be set
    output.print_info("Setting configuration:")?;
    output.print_key_value("Key", key)?;
    output.print_key_value("Value", &parsed_value.to_string())?;
    output.print_key_value("Type", value_type)?;
    
    // Get current config
    let spinner = output.create_spinner("Loading current configuration...");
    let mut current_config = match client.config().await {
        Ok(config_info) => {
            spinner.finish_with_message("✓ Configuration loaded");
            config_info.config
        }
        Err(e) => {
            spinner.finish_with_message("✗ Failed to load configuration");
            return Err(e);
        }
    };
    
    // Update configuration
    current_config.insert(key.to_string(), parsed_value);
    
    // Save configuration
    let save_spinner = output.create_spinner("Saving configuration...");
    match client.update_config(&current_config).await {
        Ok(_) => {
            save_spinner.finish_with_message("✓ Configuration saved");
            output.print_success(&format!("Configuration key '{}' set successfully", key))?;
        }
        Err(e) => {
            save_spinner.finish_with_message("✗ Failed to save configuration");
            output.print_error(&format!("Failed to save configuration: {}", e))?;
            std::process::exit(e.exit_code());
        }
    }
    
    Ok(())
}

async fn get_config(
    client: &ChronicleClient,
    output: &OutputManager,
    key: &str,
) -> Result<()> {
    let spinner = output.create_spinner("Loading configuration...");
    
    match client.config().await {
        Ok(config_info) => {
            spinner.finish_with_message("✓ Configuration loaded");
            
            if let Some(value) = config_info.config.get(key) {
                println!("{}", value);
            } else {
                output.print_error(&format!("Configuration key '{}' not found", key))?;
                std::process::exit(5);
            }
        }
        Err(e) => {
            spinner.finish_with_message("✗ Failed to load configuration");
            output.print_error(&format!("Failed to load configuration: {}", e))?;
            std::process::exit(e.exit_code());
        }
    }
    
    Ok(())
}

async fn delete_config(
    client: &ChronicleClient,
    output: &OutputManager,
    key: &str,
    force: bool,
) -> Result<()> {
    // Get current config
    let spinner = output.create_spinner("Loading current configuration...");
    let mut current_config = match client.config().await {
        Ok(config_info) => {
            spinner.finish_with_message("✓ Configuration loaded");
            config_info.config
        }
        Err(e) => {
            spinner.finish_with_message("✗ Failed to load configuration");
            return Err(e);
        }
    };
    
    // Check if key exists
    if !current_config.contains_key(key) {
        output.print_warning(&format!("Configuration key '{}' not found", key))?;
        return Ok(());
    }
    
    // Show current value
    if let Some(value) = current_config.get(key) {
        output.print_info(&format!("Current value of '{}': {}", key, value))?;
    }
    
    // Confirm deletion
    if !force {
        let confirm = output.prompt_confirm(&format!("Delete configuration key '{}'?", key))?;
        if !confirm {
            return Err(ChronicleError::Cancelled);
        }
    }
    
    // Remove key
    current_config.remove(key);
    
    // Save configuration
    let save_spinner = output.create_spinner("Saving configuration...");
    match client.update_config(&current_config).await {
        Ok(_) => {
            save_spinner.finish_with_message("✓ Configuration saved");
            output.print_success(&format!("Configuration key '{}' deleted successfully", key))?;
        }
        Err(e) => {
            save_spinner.finish_with_message("✗ Failed to save configuration");
            output.print_error(&format!("Failed to save configuration: {}", e))?;
            std::process::exit(e.exit_code());
        }
    }
    
    Ok(())
}

async fn validate_config(
    client: &ChronicleClient,
    output: &OutputManager,
    file: Option<&str>,
) -> Result<()> {
    let spinner = output.create_spinner("Validating configuration...");
    
    let config = if let Some(file_path) = file {
        // Validate local file
        let path = Path::new(file_path);
        utils::check_file_readable(path)?;
        
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str::<HashMap<String, Value>>(&content)?
    } else {
        // Validate current config
        match client.config().await {
            Ok(config_info) => config_info.config,
            Err(e) => {
                spinner.finish_with_message("✗ Failed to load configuration");
                return Err(e);
            }
        }
    };
    
    // Perform validation checks
    let mut issues = Vec::new();
    
    // Check for required keys
    let required_keys = ["service_url", "timeout", "max_results"];
    for &key in &required_keys {
        if !config.contains_key(key) {
            issues.push(format!("Missing required key: {}", key));
        }
    }
    
    // Validate specific values
    if let Some(timeout) = config.get("timeout") {
        if let Some(timeout_num) = timeout.as_u64() {
            if timeout_num == 0 || timeout_num > 3600 {
                issues.push("Timeout should be between 1 and 3600 seconds".to_string());
            }
        }
    }
    
    if let Some(max_results) = config.get("max_results") {
        if let Some(max_num) = max_results.as_u64() {
            if max_num == 0 || max_num > 100000 {
                issues.push("Max results should be between 1 and 100000".to_string());
            }
        }
    }
    
    spinner.finish_with_message("✓ Validation completed");
    
    if issues.is_empty() {
        output.print_success("Configuration is valid")?;
    } else {
        output.print_warning("Configuration has issues:")?;
        for issue in issues {
            println!("  • {}", issue);
        }
    }
    
    Ok(())
}

async fn reset_config(
    client: &ChronicleClient,
    output: &OutputManager,
    force: bool,
    keep: Option<&str>,
) -> Result<()> {
    if !force {
        output.print_warning("This will reset all configuration to default values")?;
        let confirm = output.prompt_confirm("Are you sure you want to reset the configuration?")?;
        if !confirm {
            return Err(ChronicleError::Cancelled);
        }
    }
    
    // Get current config to preserve specified keys
    let current_config = if let Some(keep_keys) = keep {
        let spinner = output.create_spinner("Loading current configuration...");
        match client.config().await {
            Ok(config_info) => {
                spinner.finish_with_message("✓ Configuration loaded");
                Some(config_info.config)
            }
            Err(_) => {
                spinner.finish_with_message("✗ Failed to load configuration");
                None
            }
        }
    } else {
        None
    };
    
    // Create default configuration
    let mut default_config = create_default_config();
    
    // Preserve specified keys
    if let (Some(keep_keys), Some(current)) = (keep, current_config) {
        for key in keep_keys.split(',') {
            let key = key.trim();
            if let Some(value) = current.get(key) {
                default_config.insert(key.to_string(), value.clone());
            }
        }
    }
    
    // Save default configuration
    let save_spinner = output.create_spinner("Resetting configuration...");
    match client.update_config(&default_config).await {
        Ok(_) => {
            save_spinner.finish_with_message("✓ Configuration reset");
            output.print_success("Configuration reset to defaults")?;
        }
        Err(e) => {
            save_spinner.finish_with_message("✗ Failed to reset configuration");
            output.print_error(&format!("Failed to reset configuration: {}", e))?;
            std::process::exit(e.exit_code());
        }
    }
    
    Ok(())
}

async fn import_config(
    client: &ChronicleClient,
    output: &OutputManager,
    file: &str,
    merge: bool,
) -> Result<()> {
    let path = Path::new(file);
    utils::check_file_readable(path)?;
    
    let content = std::fs::read_to_string(path)?;
    let import_config: HashMap<String, Value> = serde_json::from_str(&content)?;
    
    let final_config = if merge {
        // Merge with existing config
        let spinner = output.create_spinner("Loading current configuration...");
        let mut current_config = match client.config().await {
            Ok(config_info) => {
                spinner.finish_with_message("✓ Configuration loaded");
                config_info.config
            }
            Err(e) => {
                spinner.finish_with_message("✗ Failed to load configuration");
                return Err(e);
            }
        };
        
        // Merge imported config
        for (key, value) in import_config {
            current_config.insert(key, value);
        }
        
        current_config
    } else {
        import_config
    };
    
    // Save configuration
    let save_spinner = output.create_spinner("Importing configuration...");
    match client.update_config(&final_config).await {
        Ok(_) => {
            save_spinner.finish_with_message("✓ Configuration imported");
            output.print_success("Configuration imported successfully")?;
        }
        Err(e) => {
            save_spinner.finish_with_message("✗ Failed to import configuration");
            output.print_error(&format!("Failed to import configuration: {}", e))?;
            std::process::exit(e.exit_code());
        }
    }
    
    Ok(())
}

async fn export_config(
    client: &ChronicleClient,
    output: &OutputManager,
    file: &str,
    format: &str,
) -> Result<()> {
    if !matches!(format, "json" | "toml" | "yaml") {
        return Err(ChronicleError::InvalidQuery(
            "Export format must be json, toml, or yaml".to_string(),
        ));
    }
    
    let spinner = output.create_spinner("Loading configuration...");
    let config = match client.config().await {
        Ok(config_info) => {
            spinner.finish_with_message("✓ Configuration loaded");
            config_info.config
        }
        Err(e) => {
            spinner.finish_with_message("✗ Failed to load configuration");
            return Err(e);
        }
    };
    
    let content = match format {
        "json" => serde_json::to_string_pretty(&config)?,
        "toml" => return Err(ChronicleError::Export("TOML export not yet implemented".to_string())),
        "yaml" => serde_yaml::to_string(&config).map_err(|e| ChronicleError::Export(e.to_string()))?,
        _ => unreachable!(),
    };
    
    std::fs::write(file, content)?;
    
    output.print_success(&format!("Configuration exported to {}", file))?;
    Ok(())
}

fn parse_config_value(value: &str, value_type: &str) -> Result<Value> {
    match value_type {
        "string" => Ok(Value::String(value.to_string())),
        "number" => {
            if let Ok(int_val) = value.parse::<i64>() {
                Ok(Value::Number(serde_json::Number::from(int_val)))
            } else if let Ok(float_val) = value.parse::<f64>() {
                Ok(Value::Number(serde_json::Number::from_f64(float_val).unwrap()))
            } else {
                Err(ChronicleError::Parse(format!("Invalid number: {}", value)))
            }
        }
        "boolean" => {
            match value.to_lowercase().as_str() {
                "true" | "yes" | "1" => Ok(Value::Bool(true)),
                "false" | "no" | "0" => Ok(Value::Bool(false)),
                _ => Err(ChronicleError::Parse(format!("Invalid boolean: {}", value))),
            }
        }
        "json" => {
            serde_json::from_str(value).map_err(|e| ChronicleError::Parse(format!("Invalid JSON: {}", e)))
        }
        _ => Err(ChronicleError::Parse(format!("Unknown value type: {}", value_type))),
    }
}

fn create_default_config() -> HashMap<String, Value> {
    let mut config = HashMap::new();
    
    config.insert("service_url".to_string(), Value::String("http://localhost:8080".to_string()));
    config.insert("timeout".to_string(), Value::Number(serde_json::Number::from(30)));
    config.insert("max_results".to_string(), Value::Number(serde_json::Number::from(1000)));
    config.insert("default_format".to_string(), Value::String("table".to_string()));
    config.insert("colored_output".to_string(), Value::Bool(true));
    config.insert("auto_paging".to_string(), Value::Bool(true));
    
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config_value() {
        assert_eq!(parse_config_value("hello", "string").unwrap(), Value::String("hello".to_string()));
        assert_eq!(parse_config_value("42", "number").unwrap(), Value::Number(serde_json::Number::from(42)));
        assert_eq!(parse_config_value("true", "boolean").unwrap(), Value::Bool(true));
        assert_eq!(parse_config_value("false", "boolean").unwrap(), Value::Bool(false));
        
        let json_val = parse_config_value(r#"{"key": "value"}"#, "json").unwrap();
        assert!(json_val.is_object());
    }

    #[test]
    fn test_create_default_config() {
        let config = create_default_config();
        assert!(config.contains_key("service_url"));
        assert!(config.contains_key("timeout"));
        assert!(config.contains_key("max_results"));
    }

    #[test]
    fn test_invalid_config_values() {
        assert!(parse_config_value("invalid", "number").is_err());
        assert!(parse_config_value("invalid", "boolean").is_err());
        assert!(parse_config_value("invalid json", "json").is_err());
        assert!(parse_config_value("value", "unknown_type").is_err());
    }
}