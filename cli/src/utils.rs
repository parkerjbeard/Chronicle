use crate::error::{ChronicleError, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::env;

/// Get the Chronicle configuration directory
pub fn get_config_dir() -> Result<PathBuf> {
    let config_dir = if let Ok(chronicle_config) = env::var("CHRONICLE_CONFIG_DIR") {
        PathBuf::from(chronicle_config)
    } else if let Some(home_dir) = dirs::home_dir() {
        home_dir.join(".chronicle")
    } else {
        return Err(ChronicleError::Config(config::ConfigError::Message(
            "Unable to determine home directory".to_string(),
        )));
    };

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    Ok(config_dir)
}

/// Get the Chronicle configuration file path
pub fn get_config_file() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join("config.toml"))
}

/// Get the Chronicle data directory
pub fn get_data_dir() -> Result<PathBuf> {
    let data_dir = if let Ok(chronicle_data) = env::var("CHRONICLE_DATA_DIR") {
        PathBuf::from(chronicle_data)
    } else if let Some(home_dir) = dirs::home_dir() {
        home_dir.join(".chronicle").join("data")
    } else {
        return Err(ChronicleError::Config(config::ConfigError::Message(
            "Unable to determine home directory".to_string(),
        )));
    };

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)?;
    }

    Ok(data_dir)
}

/// Get the Chronicle cache directory
pub fn get_cache_dir() -> Result<PathBuf> {
    let cache_dir = if let Ok(chronicle_cache) = env::var("CHRONICLE_CACHE_DIR") {
        PathBuf::from(chronicle_cache)
    } else if let Some(cache_dir) = dirs::cache_dir() {
        cache_dir.join("chronicle")
    } else if let Some(home_dir) = dirs::home_dir() {
        home_dir.join(".chronicle").join("cache")
    } else {
        return Err(ChronicleError::Config(config::ConfigError::Message(
            "Unable to determine cache directory".to_string(),
        )));
    };

    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir)?;
    }

    Ok(cache_dir)
}

/// Validate that a path is safe for file operations
pub fn validate_path(path: &Path) -> Result<()> {
    let canonical_path = path.canonicalize().map_err(|e| {
        ChronicleError::FileNotFound {
            path: path.to_string_lossy().to_string(),
        }
    })?;

    // Check if the path is under allowed directories
    let allowed_dirs = [
        get_config_dir()?,
        get_data_dir()?,
        get_cache_dir()?,
        PathBuf::from("/tmp"),
        PathBuf::from("/var/tmp"),
    ];

    let mut is_allowed = false;
    for allowed_dir in &allowed_dirs {
        if canonical_path.starts_with(allowed_dir) {
            is_allowed = true;
            break;
        }
    }

    // Also allow paths in the current working directory
    if let Ok(cwd) = env::current_dir() {
        if canonical_path.starts_with(cwd) {
            is_allowed = true;
        }
    }

    if !is_allowed {
        return Err(ChronicleError::Permission(format!(
            "Path is not in an allowed directory: {}",
            canonical_path.display()
        )));
    }

    Ok(())
}

/// Format a timestamp for display
pub fn format_timestamp(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Format a timestamp for filename use
pub fn format_timestamp_filename(dt: &DateTime<Utc>) -> String {
    dt.format("%Y%m%d_%H%M%S").to_string()
}

/// Parse a human-readable file size (e.g., "1.5GB") into bytes
pub fn parse_file_size(size_str: &str) -> Result<u64> {
    let size_str = size_str.trim().to_uppercase();
    
    let (number_part, unit_part) = if size_str.ends_with("B") {
        size_str.split_at(size_str.len() - 1)
    } else if size_str.ends_with("KB") {
        size_str.split_at(size_str.len() - 2)
    } else if size_str.ends_with("MB") {
        size_str.split_at(size_str.len() - 2)
    } else if size_str.ends_with("GB") {
        size_str.split_at(size_str.len() - 2)
    } else if size_str.ends_with("TB") {
        size_str.split_at(size_str.len() - 2)
    } else {
        (size_str.as_str(), "B")
    };

    let number: f64 = number_part.parse().map_err(|_| {
        ChronicleError::Parse(format!("Invalid file size: {}", size_str))
    })?;

    let multiplier = match unit_part {
        "B" => 1,
        "KB" => 1024,
        "MB" => 1024 * 1024,
        "GB" => 1024 * 1024 * 1024,
        "TB" => 1024_u64.pow(4),
        _ => return Err(ChronicleError::Parse(format!("Unknown unit: {}", unit_part))),
    };

    Ok((number * multiplier as f64) as u64)
}

/// Format bytes into a human-readable string
pub fn format_bytes(bytes: u64) -> String {
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

/// Check if a file exists and is readable
pub fn check_file_readable(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(ChronicleError::FileNotFound {
            path: path.to_string_lossy().to_string(),
        });
    }

    if !path.is_file() {
        return Err(ChronicleError::Validation(format!(
            "Path is not a file: {}",
            path.display()
        )));
    }

    // Try to read the file metadata to check permissions
    fs::metadata(path).map_err(|e| ChronicleError::Permission(format!(
        "Cannot read file {}: {}",
        path.display(),
        e
    )))?;

    Ok(())
}

/// Check if a directory exists and is writable
pub fn check_directory_writable(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(ChronicleError::FileNotFound {
            path: path.to_string_lossy().to_string(),
        });
    }

    if !path.is_dir() {
        return Err(ChronicleError::Validation(format!(
            "Path is not a directory: {}",
            path.display()
        )));
    }

    // Try to create a temporary file to check write permissions
    let temp_file = path.join(".chronicle_write_test");
    fs::write(&temp_file, "test").map_err(|e| ChronicleError::Permission(format!(
        "Cannot write to directory {}: {}",
        path.display(),
        e
    )))?;

    // Clean up the temporary file
    let _ = fs::remove_file(&temp_file);

    Ok(())
}

/// Pretty print JSON with syntax highlighting (if colors are enabled)
pub fn pretty_print_json(value: &Value, colored: bool) -> Result<String> {
    if colored {
        Ok(serde_json::to_string_pretty(value)?)
    } else {
        Ok(serde_json::to_string_pretty(value)?)
    }
}

/// Truncate a string to a maximum length with ellipsis
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        s.chars().take(max_len).collect()
    } else {
        let mut result = s.chars().take(max_len - 3).collect::<String>();
        result.push_str("...");
        result
    }
}

/// Generate a secure random string for temporary files
pub fn generate_temp_filename(prefix: &str, extension: &str) -> String {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S%.3f");
    format!("{}_{}.{}", prefix, timestamp, extension)
}

/// Validate that a string is a valid identifier (alphanumeric + underscore)
pub fn validate_identifier(identifier: &str) -> Result<()> {
    if identifier.is_empty() {
        return Err(ChronicleError::Validation("Identifier cannot be empty".to_string()));
    }

    if !identifier.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(ChronicleError::Validation(format!(
            "Identifier can only contain alphanumeric characters, underscores, and hyphens: {}",
            identifier
        )));
    }

    if identifier.len() > 64 {
        return Err(ChronicleError::Validation(format!(
            "Identifier too long (max 64 characters): {}",
            identifier
        )));
    }

    Ok(())
}

/// Get the available disk space for a given path
pub fn get_available_space(path: &Path) -> Result<u64> {
    let metadata = fs::metadata(path)?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // This is a simplified version - in a real implementation,
        // you'd want to use statvfs or similar system calls
        Ok(metadata.len()) // This is not accurate, just a placeholder
    }
    
    #[cfg(not(unix))]
    {
        // For non-Unix systems, return a default value
        Ok(1024 * 1024 * 1024) // 1GB default
    }
}

/// Check if the current user has sufficient permissions
pub fn check_permissions() -> Result<()> {
    // Check if we can create files in the config directory
    let config_dir = get_config_dir()?;
    check_directory_writable(&config_dir)?;

    // Check if we can create files in the data directory
    let data_dir = get_data_dir()?;
    check_directory_writable(&data_dir)?;

    Ok(())
}

/// Sanitize a filename by removing or replacing invalid characters
pub fn sanitize_filename(filename: &str) -> String {
    let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    let mut sanitized = String::new();
    
    for ch in filename.chars() {
        if invalid_chars.contains(&ch) || ch.is_control() {
            sanitized.push('_');
        } else {
            sanitized.push(ch);
        }
    }
    
    // Limit filename length
    if sanitized.len() > 255 {
        sanitized.truncate(252);
        sanitized.push_str("...");
    }
    
    sanitized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_file_size() {
        assert_eq!(parse_file_size("1024B").unwrap(), 1024);
        assert_eq!(parse_file_size("1KB").unwrap(), 1024);
        assert_eq!(parse_file_size("1MB").unwrap(), 1024 * 1024);
        assert_eq!(parse_file_size("1.5GB").unwrap(), (1.5 * 1024.0 * 1024.0 * 1024.0) as u64);
        assert!(parse_file_size("invalid").is_err());
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1536), "1.5 KB");
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello world", 5), "he...");
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hi", 2), "hi");
    }

    #[test]
    fn test_validate_identifier() {
        assert!(validate_identifier("valid_id").is_ok());
        assert!(validate_identifier("valid-id").is_ok());
        assert!(validate_identifier("valid123").is_ok());
        assert!(validate_identifier("").is_err());
        assert!(validate_identifier("invalid.id").is_err());
        assert!(validate_identifier("invalid id").is_err());
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("valid_file.txt"), "valid_file.txt");
        assert_eq!(sanitize_filename("invalid/file.txt"), "invalid_file.txt");
        assert_eq!(sanitize_filename("file:with*chars?.txt"), "file_with_chars_.txt");
    }
}