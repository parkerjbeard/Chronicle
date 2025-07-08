use crate::api::{Event, HealthStatus, SearchResult};
use crate::error::Result;
use chrono::{DateTime, Utc};
use console::{style, Style, Term};
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, Write};

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Json,
    Table,
    Csv,
    Yaml,
    Raw,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "table" => Ok(OutputFormat::Table),
            "csv" => Ok(OutputFormat::Csv),
            "yaml" => Ok(OutputFormat::Yaml),
            "raw" => Ok(OutputFormat::Raw),
            _ => Err(format!("Unknown output format: {}", s)),
        }
    }
}

pub struct OutputManager {
    format: OutputFormat,
    colored: bool,
    term: Term,
}

impl OutputManager {
    pub fn new(format: OutputFormat, colored: bool) -> Self {
        Self {
            format,
            colored,
            term: Term::stdout(),
        }
    }

    pub fn print_health_status(&self, status: &HealthStatus) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(status)?);
            }
            OutputFormat::Table => {
                self.print_health_table(status)?;
            }
            OutputFormat::Csv => {
                self.print_health_csv(status)?;
            }
            OutputFormat::Yaml => {
                println!("{}", serde_yaml::to_string(status).unwrap_or_else(|_| "Error serializing to YAML".to_string()));
            }
            OutputFormat::Raw => {
                println!("Status: {}", status.status);
                println!("Version: {}", status.version);
                println!("Uptime: {}s", status.uptime);
                println!("Storage: {:.1}% used", status.storage_usage.usage_percent);
                println!("Memory: {:.1}% used", status.memory_usage.usage_percent);
                println!("Active connections: {}", status.active_connections);
                if let Some(last_event) = &status.last_event_time {
                    println!("Last event: {}", last_event.format("%Y-%m-%d %H:%M:%S UTC"));
                }
            }
        }
        Ok(())
    }

    pub fn print_search_results(&self, results: &SearchResult) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(results)?);
            }
            OutputFormat::Table => {
                self.print_search_table(results)?;
            }
            OutputFormat::Csv => {
                self.print_search_csv(results)?;
            }
            OutputFormat::Yaml => {
                println!("{}", serde_yaml::to_string(results).unwrap_or_else(|_| "Error serializing to YAML".to_string()));
            }
            OutputFormat::Raw => {
                for event in &results.events {
                    println!("{} [{}] {}: {}", 
                        event.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                        event.event_type,
                        event.id,
                        event.data
                    );
                }
                println!("\nTotal: {} events, Query time: {}ms", results.total_count, results.query_time_ms);
            }
        }
        Ok(())
    }

    pub fn print_events(&self, events: &[Event]) -> Result<()> {
        let results = SearchResult {
            events: events.to_vec(),
            total_count: events.len(),
            query_time_ms: 0,
            has_more: false,
        };
        self.print_search_results(&results)
    }

    pub fn print_key_value(&self, key: &str, value: &str) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                let mut map = HashMap::new();
                map.insert(key, value);
                println!("{}", serde_json::to_string_pretty(&map)?);
            }
            OutputFormat::Table => {
                if self.colored {
                    println!("{}: {}", style(key).bold().blue(), style(value).green());
                } else {
                    println!("{}: {}", key, value);
                }
            }
            OutputFormat::Csv => {
                println!("{},{}", key, value);
            }
            OutputFormat::Yaml => {
                println!("{}: {}", key, value);
            }
            OutputFormat::Raw => {
                println!("{}: {}", key, value);
            }
        }
        Ok(())
    }

    pub fn print_config(&self, config: &HashMap<String, Value>) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(config)?);
            }
            OutputFormat::Table => {
                self.print_config_table(config)?;
            }
            OutputFormat::Csv => {
                self.print_config_csv(config)?;
            }
            OutputFormat::Yaml => {
                println!("{}", serde_yaml::to_string(config).unwrap_or_else(|_| "Error serializing to YAML".to_string()));
            }
            OutputFormat::Raw => {
                for (key, value) in config {
                    println!("{}: {}", key, value);
                }
            }
        }
        Ok(())
    }

    pub fn print_success(&self, message: &str) -> Result<()> {
        if self.colored {
            println!("{} {}", style("✓").green().bold(), message);
        } else {
            println!("✓ {}", message);
        }
        Ok(())
    }

    pub fn print_warning(&self, message: &str) -> Result<()> {
        if self.colored {
            println!("{} {}", style("⚠").yellow().bold(), message);
        } else {
            println!("⚠ {}", message);
        }
        Ok(())
    }

    pub fn print_error(&self, message: &str) -> Result<()> {
        if self.colored {
            eprintln!("{} {}", style("✗").red().bold(), message);
        } else {
            eprintln!("✗ {}", message);
        }
        Ok(())
    }

    pub fn print_info(&self, message: &str) -> Result<()> {
        if self.colored {
            println!("{} {}", style("ℹ").blue().bold(), message);
        } else {
            println!("ℹ {}", message);
        }
        Ok(())
    }

    pub fn create_progress_bar(&self, total: u64, message: &str) -> ProgressBar {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(message.to_string());
        pb
    }

    pub fn create_spinner(&self, message: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message(message.to_string());
        pb
    }

    fn print_health_table(&self, status: &HealthStatus) -> Result<()> {
        let status_color = match status.status.as_str() {
            "healthy" => Style::new().green(),
            "degraded" => Style::new().yellow(),
            "unhealthy" => Style::new().red(),
            _ => Style::new().white(),
        };

        if self.colored {
            println!("{}", style("Chronicle Health Status").bold().underlined());
            println!("{}: {}", style("Status").bold(), status_color.apply_to(&status.status));
            println!("{}: {}", style("Version").bold(), style(&status.version).cyan());
            println!("{}: {}", style("Uptime").bold(), format_duration(status.uptime));
            println!("{}: {}", style("Active Connections").bold(), status.active_connections);
            
            println!("\n{}", style("Storage Usage").bold().underlined());
            println!("  {}: {}", style("Total").bold(), format_bytes(status.storage_usage.total_bytes));
            println!("  {}: {}", style("Used").bold(), format_bytes(status.storage_usage.used_bytes));
            println!("  {}: {}", style("Available").bold(), format_bytes(status.storage_usage.available_bytes));
            println!("  {}: {:.1}%", style("Usage").bold(), status.storage_usage.usage_percent);
            
            println!("\n{}", style("Memory Usage").bold().underlined());
            println!("  {}: {}", style("Total").bold(), format_bytes(status.memory_usage.total_bytes));
            println!("  {}: {}", style("Used").bold(), format_bytes(status.memory_usage.used_bytes));
            println!("  {}: {}", style("Available").bold(), format_bytes(status.memory_usage.available_bytes));
            println!("  {}: {:.1}%", style("Usage").bold(), status.memory_usage.usage_percent);
            
            if let Some(last_event) = &status.last_event_time {
                println!("\n{}: {}", style("Last Event").bold(), last_event.format("%Y-%m-%d %H:%M:%S UTC"));
            }
        } else {
            println!("Chronicle Health Status");
            println!("Status: {}", status.status);
            println!("Version: {}", status.version);
            println!("Uptime: {}", format_duration(status.uptime));
            println!("Active Connections: {}", status.active_connections);
            
            println!("\nStorage Usage");
            println!("  Total: {}", format_bytes(status.storage_usage.total_bytes));
            println!("  Used: {}", format_bytes(status.storage_usage.used_bytes));
            println!("  Available: {}", format_bytes(status.storage_usage.available_bytes));
            println!("  Usage: {:.1}%", status.storage_usage.usage_percent);
            
            println!("\nMemory Usage");
            println!("  Total: {}", format_bytes(status.memory_usage.total_bytes));
            println!("  Used: {}", format_bytes(status.memory_usage.used_bytes));
            println!("  Available: {}", format_bytes(status.memory_usage.available_bytes));
            println!("  Usage: {:.1}%", status.memory_usage.usage_percent);
            
            if let Some(last_event) = &status.last_event_time {
                println!("\nLast Event: {}", last_event.format("%Y-%m-%d %H:%M:%S UTC"));
            }
        }
        Ok(())
    }

    fn print_health_csv(&self, status: &HealthStatus) -> Result<()> {
        println!("field,value");
        println!("status,{}", status.status);
        println!("version,{}", status.version);
        println!("uptime,{}", status.uptime);
        println!("active_connections,{}", status.active_connections);
        println!("storage_total,{}", status.storage_usage.total_bytes);
        println!("storage_used,{}", status.storage_usage.used_bytes);
        println!("storage_available,{}", status.storage_usage.available_bytes);
        println!("storage_usage_percent,{:.1}", status.storage_usage.usage_percent);
        println!("memory_total,{}", status.memory_usage.total_bytes);
        println!("memory_used,{}", status.memory_usage.used_bytes);
        println!("memory_available,{}", status.memory_usage.available_bytes);
        println!("memory_usage_percent,{:.1}", status.memory_usage.usage_percent);
        if let Some(last_event) = &status.last_event_time {
            println!("last_event,{}", last_event.to_rfc3339());
        }
        Ok(())
    }

    fn print_search_table(&self, results: &SearchResult) -> Result<()> {
        if results.events.is_empty() {
            if self.colored {
                println!("{}", style("No events found").yellow());
            } else {
                println!("No events found");
            }
            return Ok(());
        }

        if self.colored {
            println!("{}", style("Search Results").bold().underlined());
        } else {
            println!("Search Results");
        }

        // Print header
        println!("{:<20} {:<15} {:<36} {}", "Timestamp", "Type", "ID", "Data");
        println!("{:-<20} {:-<15} {:-<36} {:-<40}", "", "", "", "");

        for event in &results.events {
            let timestamp = event.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
            let data_preview = truncate_json(&event.data, 40);
            
            if self.colored {
                println!("{:<20} {:<15} {:<36} {}", 
                    style(&timestamp).dim(),
                    style(&event.event_type).cyan(),
                    style(&event.id).green(),
                    data_preview
                );
            } else {
                println!("{:<20} {:<15} {:<36} {}", 
                    timestamp,
                    event.event_type,
                    event.id,
                    data_preview
                );
            }
        }

        println!();
        if self.colored {
            println!("{}: {} events, {}: {}ms", 
                style("Total").bold(), results.total_count,
                style("Query time").bold(), results.query_time_ms
            );
        } else {
            println!("Total: {} events, Query time: {}ms", results.total_count, results.query_time_ms);
        }

        Ok(())
    }

    fn print_search_csv(&self, results: &SearchResult) -> Result<()> {
        println!("timestamp,type,id,data");
        for event in &results.events {
            let data_str = serde_json::to_string(&event.data)?;
            println!("{},{},{},\"{}\"", 
                event.timestamp.to_rfc3339(),
                event.event_type,
                event.id,
                data_str.replace("\"", "\"\"")
            );
        }
        Ok(())
    }

    fn print_config_table(&self, config: &HashMap<String, Value>) -> Result<()> {
        if self.colored {
            println!("{}", style("Configuration").bold().underlined());
        } else {
            println!("Configuration");
        }

        println!("{:<30} {}", "Key", "Value");
        println!("{:-<30} {:-<50}", "", "");

        for (key, value) in config {
            let value_str = match value {
                Value::String(s) => s.clone(),
                _ => serde_json::to_string(value)?,
            };
            
            if self.colored {
                println!("{:<30} {}", 
                    style(key).bold().blue(),
                    style(&value_str).green()
                );
            } else {
                println!("{:<30} {}", key, value_str);
            }
        }
        Ok(())
    }

    fn print_config_csv(&self, config: &HashMap<String, Value>) -> Result<()> {
        println!("key,value");
        for (key, value) in config {
            let value_str = match value {
                Value::String(s) => s.clone(),
                _ => serde_json::to_string(value)?,
            };
            println!("{},\"{}\"", key, value_str.replace("\"", "\"\""));
        }
        Ok(())
    }

    pub fn prompt_confirm(&self, message: &str) -> Result<bool> {
        print!("{} [y/N]: ", message);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        Ok(input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes")
    }

    pub fn prompt_input(&self, message: &str) -> Result<String> {
        print!("{}: ", message);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        Ok(input.trim().to_string())
    }

    pub fn prompt_password(&self, message: &str) -> Result<String> {
        print!("{}: ", message);
        io::stdout().flush()?;
        
        let password = rpassword::read_password()
            .map_err(|e| crate::error::ChronicleError::Io(e))?;
        Ok(password)
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    
    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, secs)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

fn truncate_json(value: &Value, max_len: usize) -> String {
    let json_str = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    if json_str.len() <= max_len {
        json_str
    } else {
        format!("{}...", &json_str[..max_len.saturating_sub(3)])
    }
}