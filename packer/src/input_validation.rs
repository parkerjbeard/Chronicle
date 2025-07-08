use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    str::FromStr,
};
use tracing::{warn, error};
use uuid::Uuid;

/// Comprehensive input validation for Chronicle API endpoints
/// This module provides security-focused validation for all user inputs

/// Configuration for input validation rules
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    pub max_string_length: usize,
    pub max_query_length: usize,
    pub max_path_length: usize,
    pub max_filename_length: usize,
    pub allowed_file_extensions: HashSet<String>,
    pub blocked_patterns: Vec<String>,
    pub enable_strict_mode: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_string_length: 1000,
            max_query_length: 5000,
            max_path_length: 4096,
            max_filename_length: 255,
            allowed_file_extensions: ["json", "csv", "parquet", "txt", "log"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            blocked_patterns: vec![
                // Script injection patterns
                "<script".to_string(),
                "javascript:".to_string(),
                "vbscript:".to_string(),
                "data:".to_string(),
                "file:".to_string(),
                
                // Path traversal patterns
                "../".to_string(),
                "..\\".to_string(),
                ".\\".to_string(),
                "./".to_string(),
                
                // Command injection patterns
                "$(".to_string(),
                "`".to_string(),
                "&&".to_string(),
                "||".to_string(),
                ";".to_string(),
                "|".to_string(),
                
                // SQL injection patterns
                "'".to_string(),
                "\"".to_string(),
                "--".to_string(),
                "/*".to_string(),
                "*/".to_string(),
                "xp_".to_string(),
                "sp_".to_string(),
                
                // NoSQL injection patterns
                "$where".to_string(),
                "$ne".to_string(),
                "$regex".to_string(),
                
                // Other dangerous patterns
                "eval(".to_string(),
                "exec(".to_string(),
                "system(".to_string(),
                "shell(".to_string(),
                "proc_open(".to_string(),
                "/dev/".to_string(),
                "/proc/".to_string(),
                "/sys/".to_string(),
                "\\x".to_string(), // Hex encoded characters
            ],
            enable_strict_mode: true,
        }
    }
}

/// Input validator with configurable rules
pub struct InputValidator {
    config: ValidationConfig,
    identifier_regex: Regex,
    filename_regex: Regex,
    path_regex: Regex,
    email_regex: Regex,
    uuid_regex: Regex,
}

impl InputValidator {
    pub fn new(config: ValidationConfig) -> Result<Self> {
        // Pre-compile regex patterns for performance
        let identifier_regex = Regex::new(r"^[a-zA-Z0-9_][a-zA-Z0-9_-]*$")?;
        let filename_regex = Regex::new(r"^[a-zA-Z0-9._-]+$")?;
        let path_regex = Regex::new(r"^[a-zA-Z0-9._/-]+$")?;
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")?;
        let uuid_regex = Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$")?;

        Ok(Self {
            config,
            identifier_regex,
            filename_regex,
            path_regex,
            email_regex,
            uuid_regex,
        })
    }

    /// Validate a general string input
    pub fn validate_string(&self, input: &str, field_name: &str) -> Result<String> {
        self.check_length(input, self.config.max_string_length, field_name)?;
        self.check_dangerous_patterns(input, field_name)?;
        self.check_encoding(input, field_name)?;
        Ok(input.trim().to_string())
    }

    /// Validate search query input with special handling for search operators
    pub fn validate_search_query(&self, query: &str) -> Result<String> {
        if query.is_empty() {
            return Err(anyhow!("Search query cannot be empty"));
        }

        self.check_length(query, self.config.max_query_length, "search_query")?;
        
        // For search queries, we allow some special characters but validate carefully
        self.check_search_injection(query)?;
        self.check_encoding(query, "search_query")?;
        
        // Validate search query structure
        self.validate_search_syntax(query)?;
        
        Ok(query.trim().to_string())
    }

    /// Validate identifier (alphanumeric + underscore + hyphen)
    pub fn validate_identifier(&self, input: &str, field_name: &str) -> Result<String> {
        if input.is_empty() {
            return Err(anyhow!("{} cannot be empty", field_name));
        }

        if !self.identifier_regex.is_match(input) {
            return Err(anyhow!("{} contains invalid characters", field_name));
        }

        self.check_length(input, 100, field_name)?; // Identifiers should be shorter
        Ok(input.to_string())
    }

    /// Validate filename with extension checking
    pub fn validate_filename(&self, filename: &str) -> Result<String> {
        if filename.is_empty() {
            return Err(anyhow!("Filename cannot be empty"));
        }

        self.check_length(filename, self.config.max_filename_length, "filename")?;
        
        if !self.filename_regex.is_match(filename) {
            return Err(anyhow!("Filename contains invalid characters"));
        }

        // Check for reserved names (Windows/Unix)
        let reserved_names = [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", 
            "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", 
            "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", ".", ".."
        ];
        
        let name_upper = filename.to_uppercase();
        for reserved in &reserved_names {
            if name_upper == *reserved || name_upper.starts_with(&format!("{}.", reserved)) {
                return Err(anyhow!("Filename uses reserved name: {}", reserved));
            }
        }

        // Validate file extension if provided
        if let Some(extension) = Path::new(filename).extension() {
            let ext_str = extension.to_string_lossy().to_lowercase();
            if !self.config.allowed_file_extensions.contains(&ext_str) {
                return Err(anyhow!("File extension '{}' is not allowed", ext_str));
            }
        }

        Ok(filename.to_string())
    }

    /// Validate file path with traversal protection
    pub fn validate_path(&self, path: &str) -> Result<PathBuf> {
        if path.is_empty() {
            return Err(anyhow!("Path cannot be empty"));
        }

        self.check_length(path, self.config.max_path_length, "path")?;
        
        // Check for dangerous path patterns
        if path.contains("..") {
            return Err(anyhow!("Path traversal detected"));
        }

        if path.contains('\0') {
            return Err(anyhow!("Null byte in path"));
        }

        // Convert to PathBuf and canonicalize
        let path_buf = PathBuf::from(path);
        
        // Check for absolute vs relative paths based on context
        if path_buf.is_absolute() {
            // For absolute paths, ensure they're within allowed directories
            let allowed_prefixes = ["/tmp", "/var/tmp", "/home", "/Users"];
            let path_str = path_buf.to_string_lossy();
            
            if !allowed_prefixes.iter().any(|prefix| path_str.starts_with(prefix)) {
                return Err(anyhow!("Absolute path not in allowed directory"));
            }
        }

        // Validate each component
        for component in path_buf.components() {
            let comp_str = component.as_os_str().to_string_lossy();
            if comp_str.starts_with('.') && comp_str != "." && comp_str != ".." {
                warn!("Hidden file/directory in path: {}", comp_str);
            }
        }

        Ok(path_buf)
    }

    /// Validate UUID format
    pub fn validate_uuid(&self, uuid_str: &str) -> Result<Uuid> {
        if !self.uuid_regex.is_match(uuid_str) {
            return Err(anyhow!("Invalid UUID format"));
        }

        Uuid::from_str(uuid_str).map_err(|e| anyhow!("UUID parsing error: {}", e))
    }

    /// Validate email address
    pub fn validate_email(&self, email: &str) -> Result<String> {
        if !self.email_regex.is_match(email) {
            return Err(anyhow!("Invalid email format"));
        }

        self.check_length(email, 254, "email")?; // RFC 5321 limit
        Ok(email.to_lowercase())
    }

    /// Validate JSON input
    pub fn validate_json(&self, json_str: &str) -> Result<serde_json::Value> {
        if json_str.len() > 1_000_000 { // 1MB limit
            return Err(anyhow!("JSON too large"));
        }

        let parsed: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| anyhow!("Invalid JSON: {}", e))?;

        // Check for potentially dangerous JSON content
        self.validate_json_content(&parsed)?;

        Ok(parsed)
    }

    /// Validate numeric inputs with range checking
    pub fn validate_number<T>(&self, value: T, min: T, max: T, field_name: &str) -> Result<T>
    where
        T: PartialOrd + Copy + std::fmt::Display,
    {
        if value < min || value > max {
            return Err(anyhow!(
                "{} must be between {} and {}, got {}",
                field_name, min, max, value
            ));
        }
        Ok(value)
    }

    /// Validate timestamp/date input
    pub fn validate_timestamp(&self, timestamp: &str) -> Result<chrono::DateTime<chrono::Utc>> {
        // Try multiple common timestamp formats
        let formats = [
            "%Y-%m-%dT%H:%M:%S%.3fZ",     // ISO 8601 with milliseconds
            "%Y-%m-%dT%H:%M:%SZ",         // ISO 8601 basic
            "%Y-%m-%d %H:%M:%S",          // SQL-like format
            "%Y-%m-%d",                   // Date only
        ];

        for format in &formats {
            if let Ok(dt) = chrono::DateTime::parse_from_str(timestamp, format) {
                return Ok(dt.with_timezone(&chrono::Utc));
            }
        }

        // Try parsing as Unix timestamp
        if let Ok(unix_ts) = timestamp.parse::<i64>() {
            if let Some(dt) = chrono::DateTime::from_timestamp(unix_ts, 0) {
                return Ok(dt);
            }
        }

        Err(anyhow!("Invalid timestamp format"))
    }

    // Private helper methods

    fn check_length(&self, input: &str, max_length: usize, field_name: &str) -> Result<()> {
        if input.len() > max_length {
            return Err(anyhow!(
                "{} exceeds maximum length of {} characters",
                field_name, max_length
            ));
        }
        Ok(())
    }

    fn check_dangerous_patterns(&self, input: &str, field_name: &str) -> Result<()> {
        let input_lower = input.to_lowercase();
        
        for pattern in &self.config.blocked_patterns {
            if input_lower.contains(pattern) {
                error!("Dangerous pattern detected in {}: {}", field_name, pattern);
                return Err(anyhow!("Input contains potentially dangerous content"));
            }
        }
        
        Ok(())
    }

    fn check_encoding(&self, input: &str, field_name: &str) -> Result<()> {
        // Check for non-printable characters
        for ch in input.chars() {
            if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
                return Err(anyhow!("{} contains control characters", field_name));
            }
        }

        // Check for unusual Unicode categories that might indicate encoding attacks
        let suspicious_categories = [
            unicode_general_category::GeneralCategory::Format,
            unicode_general_category::GeneralCategory::Surrogate,
            unicode_general_category::GeneralCategory::PrivateUse,
            unicode_general_category::GeneralCategory::Unassigned,
        ];

        for ch in input.chars() {
            let category = unicode_general_category::get_general_category(ch);
            if suspicious_categories.contains(&category) {
                warn!("Suspicious Unicode character in {}: U+{:04X}", field_name, ch as u32);
                if self.config.enable_strict_mode {
                    return Err(anyhow!("{} contains suspicious Unicode characters", field_name));
                }
            }
        }

        Ok(())
    }

    fn check_search_injection(&self, query: &str) -> Result<()> {
        // Allow basic search operators but block dangerous patterns
        let allowed_operators = ["AND", "OR", "NOT", "(", ")", "\"", "*", "?"];
        
        // Check for SQL injection in search
        let sql_patterns = ["UNION", "SELECT", "DROP", "DELETE", "INSERT", "UPDATE"];
        let query_upper = query.to_uppercase();
        
        for pattern in &sql_patterns {
            if query_upper.contains(pattern) {
                return Err(anyhow!("Search query contains SQL keywords"));
            }
        }

        // Check for NoSQL injection patterns
        let nosql_patterns = ["$where", "$ne", "$gt", "$lt", "$regex", "$exists"];
        for pattern in &nosql_patterns {
            if query.contains(pattern) {
                return Err(anyhow!("Search query contains NoSQL operators"));
            }
        }

        Ok(())
    }

    fn validate_search_syntax(&self, query: &str) -> Result<()> {
        // Basic syntax validation for search queries
        let mut paren_count = 0;
        let mut in_quotes = false;
        let mut escape_next = false;

        for ch in query.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' => escape_next = true,
                '"' => in_quotes = !in_quotes,
                '(' if !in_quotes => paren_count += 1,
                ')' if !in_quotes => paren_count -= 1,
                _ => {}
            }

            if paren_count < 0 {
                return Err(anyhow!("Unmatched closing parenthesis in search query"));
            }
        }

        if paren_count != 0 {
            return Err(anyhow!("Unmatched opening parenthesis in search query"));
        }

        if in_quotes {
            return Err(anyhow!("Unclosed quote in search query"));
        }

        Ok(())
    }

    fn validate_json_content(&self, value: &serde_json::Value) -> Result<()> {
        match value {
            serde_json::Value::String(s) => {
                self.check_dangerous_patterns(s, "json_string")?;
            }
            serde_json::Value::Array(arr) => {
                if arr.len() > 10000 {
                    return Err(anyhow!("JSON array too large"));
                }
                for item in arr {
                    self.validate_json_content(item)?;
                }
            }
            serde_json::Value::Object(obj) => {
                if obj.len() > 1000 {
                    return Err(anyhow!("JSON object has too many keys"));
                }
                for (key, val) in obj {
                    self.check_dangerous_patterns(key, "json_key")?;
                    self.validate_json_content(val)?;
                }
            }
            _ => {} // Numbers, booleans, null are safe
        }
        Ok(())
    }
}

/// Validation error types
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Input too long: {field} exceeds {max_length} characters")]
    TooLong { field: String, max_length: usize },
    
    #[error("Invalid format: {field} has invalid format")]
    InvalidFormat { field: String },
    
    #[error("Security violation: {field} contains dangerous content")]
    SecurityViolation { field: String },
    
    #[error("Out of range: {field} value {value} not in range {min}-{max}")]
    OutOfRange { field: String, value: String, min: String, max: String },
}

/// Convenience functions for common validations
pub fn validate_collector_id(id: &str) -> Result<String> {
    let validator = InputValidator::new(ValidationConfig::default())?;
    validator.validate_identifier(id, "collector_id")
}

pub fn validate_export_format(format: &str) -> Result<String> {
    let allowed_formats = ["json", "csv", "parquet", "sqlite"];
    if !allowed_formats.contains(&format.to_lowercase().as_str()) {
        return Err(anyhow!("Invalid export format: {}", format));
    }
    Ok(format.to_lowercase())
}

pub fn validate_time_range(time_str: &str) -> Result<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)> {
    let validator = InputValidator::new(ValidationConfig::default())?;
    
    // Handle relative time expressions
    if time_str == "today" {
        let now = chrono::Utc::now();
        let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        return Ok((start, now));
    }
    
    if time_str == "yesterday" {
        let yesterday = chrono::Utc::now() - chrono::Duration::days(1);
        let start = yesterday.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end = start + chrono::Duration::days(1);
        return Ok((start, end));
    }
    
    // Handle range format "start..end"
    if time_str.contains("..") {
        let parts: Vec<&str> = time_str.split("..").collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid time range format"));
        }
        
        let start = validator.validate_timestamp(parts[0])?;
        let end = validator.validate_timestamp(parts[1])?;
        
        if start >= end {
            return Err(anyhow!("Start time must be before end time"));
        }
        
        return Ok((start, end));
    }
    
    // Single timestamp
    let timestamp = validator.validate_timestamp(time_str)?;
    Ok((timestamp, timestamp + chrono::Duration::hours(1))) // 1 hour window
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_validator() -> InputValidator {
        InputValidator::new(ValidationConfig::default()).unwrap()
    }

    #[test]
    fn test_string_validation() {
        let validator = create_test_validator();
        
        // Valid string
        assert!(validator.validate_string("hello world", "test").is_ok());
        
        // String with dangerous pattern
        assert!(validator.validate_string("<script>alert('xss')</script>", "test").is_err());
        
        // Too long string
        let long_string = "a".repeat(2000);
        assert!(validator.validate_string(&long_string, "test").is_err());
    }

    #[test]
    fn test_search_query_validation() {
        let validator = create_test_validator();
        
        // Valid search queries
        assert!(validator.validate_search_query("hello world").is_ok());
        assert!(validator.validate_search_query("error AND warning").is_ok());
        assert!(validator.validate_search_query("\"exact phrase\"").is_ok());
        
        // Invalid search queries
        assert!(validator.validate_search_query("SELECT * FROM users").is_err());
        assert!(validator.validate_search_query("$where: function()").is_err());
        assert!(validator.validate_search_query("unclosed quote\"").is_err());
    }

    #[test]
    fn test_identifier_validation() {
        let validator = create_test_validator();
        
        // Valid identifiers
        assert!(validator.validate_identifier("valid_id", "test").is_ok());
        assert!(validator.validate_identifier("test-123", "test").is_ok());
        
        // Invalid identifiers
        assert!(validator.validate_identifier("", "test").is_err());
        assert!(validator.validate_identifier("invalid id", "test").is_err());
        assert!(validator.validate_identifier("test@example", "test").is_err());
    }

    #[test]
    fn test_filename_validation() {
        let validator = create_test_validator();
        
        // Valid filenames
        assert!(validator.validate_filename("test.json").is_ok());
        assert!(validator.validate_filename("data_file.csv").is_ok());
        
        // Invalid filenames
        assert!(validator.validate_filename("").is_err());
        assert!(validator.validate_filename("test.exe").is_err());
        assert!(validator.validate_filename("CON.txt").is_err());
        assert!(validator.validate_filename("file with spaces.txt").is_err());
    }

    #[test]
    fn test_path_validation() {
        let validator = create_test_validator();
        
        // Valid paths
        assert!(validator.validate_path("/tmp/test.txt").is_ok());
        assert!(validator.validate_path("relative/path.json").is_ok());
        
        // Invalid paths
        assert!(validator.validate_path("").is_err());
        assert!(validator.validate_path("../../../etc/passwd").is_err());
        assert!(validator.validate_path("/etc/shadow").is_err());
    }

    #[test]
    fn test_uuid_validation() {
        let validator = create_test_validator();
        
        // Valid UUID
        let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert!(validator.validate_uuid(valid_uuid).is_ok());
        
        // Invalid UUIDs
        assert!(validator.validate_uuid("invalid-uuid").is_err());
        assert!(validator.validate_uuid("").is_err());
    }

    #[test]
    fn test_json_validation() {
        let validator = create_test_validator();
        
        // Valid JSON
        assert!(validator.validate_json(r#"{"test": "value"}"#).is_ok());
        
        // Invalid JSON
        assert!(validator.validate_json("invalid json").is_err());
        
        // JSON with dangerous content
        assert!(validator.validate_json(r#"{"script": "<script>alert('xss')</script>"}"#).is_err());
    }

    #[test]
    fn test_number_validation() {
        let validator = create_test_validator();
        
        // Valid number
        assert!(validator.validate_number(50, 0, 100, "test").is_ok());
        
        // Out of range numbers
        assert!(validator.validate_number(-10, 0, 100, "test").is_err());
        assert!(validator.validate_number(150, 0, 100, "test").is_err());
    }

    #[test]
    fn test_timestamp_validation() {
        let validator = create_test_validator();
        
        // Valid timestamps
        assert!(validator.validate_timestamp("2024-01-01T12:00:00Z").is_ok());
        assert!(validator.validate_timestamp("2024-01-01").is_ok());
        assert!(validator.validate_timestamp("1672531200").is_ok()); // Unix timestamp
        
        // Invalid timestamps
        assert!(validator.validate_timestamp("invalid-date").is_err());
        assert!(validator.validate_timestamp("").is_err());
    }

    #[test]
    fn test_time_range_validation() {
        // Valid time ranges
        assert!(validate_time_range("today").is_ok());
        assert!(validate_time_range("2024-01-01..2024-01-02").is_ok());
        
        // Invalid time ranges
        assert!(validate_time_range("2024-01-02..2024-01-01").is_err()); // End before start
        assert!(validate_time_range("invalid..range").is_err());
    }
}