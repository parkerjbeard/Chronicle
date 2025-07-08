use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use std::sync::{Arc, RwLock};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    pub version: String,
    pub last_updated: DateTime<Utc>,
    pub config_file: PathBuf,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<ConfigValue>),
    Object(HashMap<String, ConfigValue>),
}

impl ConfigValue {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            ConfigValue::String(s) => Some(s),
            _ => None,
        }
    }
    
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            ConfigValue::Integer(i) => Some(*i),
            _ => None,
        }
    }
    
    pub fn as_float(&self) -> Option<f64> {
        match self {
            ConfigValue::Float(f) => Some(*f),
            ConfigValue::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }
    
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            ConfigValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }
    
    pub fn as_array(&self) -> Option<&Vec<ConfigValue>> {
        match self {
            ConfigValue::Array(arr) => Some(arr),
            _ => None,
        }
    }
    
    pub fn as_object(&self) -> Option<&HashMap<String, ConfigValue>> {
        match self {
            ConfigValue::Object(obj) => Some(obj),
            _ => None,
        }
    }
    
    pub fn from_toml_value(value: toml::Value) -> Self {
        match value {
            toml::Value::String(s) => ConfigValue::String(s),
            toml::Value::Integer(i) => ConfigValue::Integer(i),
            toml::Value::Float(f) => ConfigValue::Float(f),
            toml::Value::Boolean(b) => ConfigValue::Boolean(b),
            toml::Value::Array(arr) => {
                ConfigValue::Array(arr.into_iter().map(Self::from_toml_value).collect())
            }
            toml::Value::Table(table) => {
                ConfigValue::Object(
                    table.into_iter()
                        .map(|(k, v)| (k, Self::from_toml_value(v)))
                        .collect()
                )
            }
            toml::Value::Datetime(dt) => ConfigValue::String(dt.to_string()),
        }
    }
    
    pub fn to_toml_value(&self) -> toml::Value {
        match self {
            ConfigValue::String(s) => toml::Value::String(s.clone()),
            ConfigValue::Integer(i) => toml::Value::Integer(*i),
            ConfigValue::Float(f) => toml::Value::Float(*f),
            ConfigValue::Boolean(b) => toml::Value::Boolean(*b),
            ConfigValue::Array(arr) => {
                toml::Value::Array(arr.iter().map(|v| v.to_toml_value()).collect())
            }
            ConfigValue::Object(obj) => {
                let mut table = toml::map::Map::new();
                for (k, v) in obj {
                    table.insert(k.clone(), v.to_toml_value());
                }
                toml::Value::Table(table)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub field_path: String,
    pub rule_type: ValidationType,
    pub error_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationType {
    Range { min: f64, max: f64 },
    MinLength { min: usize },
    MaxLength { max: usize },
    Pattern { regex: String },
    OneOf { values: Vec<String> },
    Required,
    Custom { validator_name: String },
}

impl ValidationRule {
    pub fn validate(&self, value: &ConfigValue) -> Result<()> {
        match &self.rule_type {
            ValidationType::Range { min, max } => {
                if let Some(num) = value.as_float() {
                    if num < *min || num > *max {
                        return Err(anyhow!("{}: Value {} is outside range [{}, {}]", 
                                         self.error_message, num, min, max));
                    }
                } else {
                    return Err(anyhow!("{}: Expected numeric value", self.error_message));
                }
            }
            ValidationType::MinLength { min } => {
                let len = match value {
                    ConfigValue::String(s) => s.len(),
                    ConfigValue::Array(a) => a.len(),
                    _ => return Err(anyhow!("{}: Expected string or array", self.error_message)),
                };
                if len < *min {
                    return Err(anyhow!("{}: Length {} is less than minimum {}", 
                                     self.error_message, len, min));
                }
            }
            ValidationType::MaxLength { max } => {
                let len = match value {
                    ConfigValue::String(s) => s.len(),
                    ConfigValue::Array(a) => a.len(),
                    _ => return Err(anyhow!("{}: Expected string or array", self.error_message)),
                };
                if len > *max {
                    return Err(anyhow!("{}: Length {} exceeds maximum {}", 
                                     self.error_message, len, max));
                }
            }
            ValidationType::Pattern { regex } => {
                if let Some(s) = value.as_string() {
                    let re = regex::Regex::new(regex)
                        .map_err(|e| anyhow!("Invalid regex pattern: {}", e))?;
                    if !re.is_match(s) {
                        return Err(anyhow!("{}: Value '{}' doesn't match pattern '{}'", 
                                         self.error_message, s, regex));
                    }
                } else {
                    return Err(anyhow!("{}: Expected string value", self.error_message));
                }
            }
            ValidationType::OneOf { values } => {
                if let Some(s) = value.as_string() {
                    if !values.contains(s) {
                        return Err(anyhow!("{}: Value '{}' is not one of {:?}", 
                                         self.error_message, s, values));
                    }
                } else {
                    return Err(anyhow!("{}: Expected string value", self.error_message));
                }
            }
            ValidationType::Required => {
                // This should be checked at a higher level when the field is missing
            }
            ValidationType::Custom { validator_name } => {
                // Custom validators would be registered separately
                return Err(anyhow!("Custom validator '{}' not implemented", validator_name));
            }
        }
        Ok(())
    }
}

pub trait ConfigWatcher: Send + Sync {
    fn on_config_changed(&self, path: &str, old_value: Option<&ConfigValue>, new_value: &ConfigValue);
    fn on_config_error(&self, error: &str);
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSection {
    pub name: String,
    pub description: String,
    pub values: HashMap<String, ConfigValue>,
    pub validation_rules: Vec<ValidationRule>,
    pub schema_version: String,
}

impl ConfigSection {
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            values: HashMap::new(),
            validation_rules: Vec::new(),
            schema_version: "1.0.0".to_string(),
        }
    }
    
    pub fn with_validation(mut self, rules: Vec<ValidationRule>) -> Self {
        self.validation_rules = rules;
        self
    }
    
    pub fn set_value(&mut self, key: String, value: ConfigValue) -> Result<()> {
        // Validate the value against rules
        for rule in &self.validation_rules {
            if rule.field_path == key {
                rule.validate(&value)?;
            }
        }
        
        self.values.insert(key, value);
        Ok(())
    }
    
    pub fn get_value(&self, key: &str) -> Option<&ConfigValue> {
        self.values.get(key)
    }
    
    pub fn validate_all(&self) -> Result<()> {
        for rule in &self.validation_rules {
            match rule.rule_type {
                ValidationType::Required => {
                    if !self.values.contains_key(&rule.field_path) {
                        return Err(anyhow!("{}: Required field '{}' is missing", 
                                         rule.error_message, rule.field_path));
                    }
                }
                _ => {
                    if let Some(value) = self.values.get(&rule.field_path) {
                        rule.validate(value)?;
                    }
                }
            }
        }
        Ok(())
    }
}

pub struct FlexibleConfig {
    sections: Arc<RwLock<HashMap<String, ConfigSection>>>,
    watchers: Arc<RwLock<HashMap<String, Box<dyn ConfigWatcher>>>>,
    metadata: Arc<RwLock<ConfigMetadata>>,
    config_paths: Vec<PathBuf>,
    auto_save: bool,
    auto_reload: bool,
}

impl FlexibleConfig {
    pub fn new() -> Self {
        let metadata = ConfigMetadata {
            version: "1.0.0".to_string(),
            last_updated: Utc::now(),
            config_file: PathBuf::new(),
            checksum: String::new(),
        };
        
        Self {
            sections: Arc::new(RwLock::new(HashMap::new())),
            watchers: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(metadata)),
            config_paths: Vec::new(),
            auto_save: true,
            auto_reload: false,
        }
    }
    
    pub fn with_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.config_paths = paths;
        self
    }
    
    pub fn with_auto_save(mut self, auto_save: bool) -> Self {
        self.auto_save = auto_save;
        self
    }
    
    pub fn with_auto_reload(mut self, auto_reload: bool) -> Self {
        self.auto_reload = auto_reload;
        self
    }
    
    pub fn load_from_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;
        let checksum = self.calculate_checksum(&content);
        
        // Parse TOML
        let toml_value: toml::Value = toml::from_str(&content)?;
        
        if let toml::Value::Table(table) = toml_value {
            let mut sections = self.sections.write().unwrap();
            
            for (section_name, section_value) in table {
                if let toml::Value::Table(section_table) = section_value {
                    let mut section = ConfigSection::new(
                        section_name.clone(),
                        format!("Loaded from {}", path.display())
                    );
                    
                    for (key, value) in section_table {
                        let config_value = ConfigValue::from_toml_value(value);
                        section.values.insert(key, config_value);
                    }
                    
                    sections.insert(section_name, section);
                }
            }
        }
        
        // Update metadata
        {
            let mut metadata = self.metadata.write().unwrap();
            metadata.config_file = path.to_path_buf();
            metadata.last_updated = Utc::now();
            metadata.checksum = checksum;
        }
        
        Ok(())
    }
    
    pub fn save_to_file<P: AsRef<Path>>(&self) -> Result<()> {
        let sections = self.sections.read().unwrap();
        let metadata = self.metadata.read().unwrap();
        
        let path = if metadata.config_file.as_os_str().is_empty() {
            return Err(anyhow!("No config file path set"));
        } else {
            &metadata.config_file
        };
        
        let mut toml_table = toml::map::Map::new();
        
        for (section_name, section) in sections.iter() {
            let mut section_table = toml::map::Map::new();
            
            for (key, value) in &section.values {
                section_table.insert(key.clone(), value.to_toml_value());
            }
            
            toml_table.insert(section_name.clone(), toml::Value::Table(section_table));
        }
        
        let toml_content = toml::to_string_pretty(&toml::Value::Table(toml_table))?;
        fs::write(path, &toml_content)?;
        
        Ok(())
    }
    
    pub fn register_section(&self, section: ConfigSection) -> Result<()> {
        // Validate section first
        section.validate_all()?;
        
        let mut sections = self.sections.write().unwrap();
        sections.insert(section.name.clone(), section);
        
        if self.auto_save {
            drop(sections); // Release lock before save
            self.save_to_file()?;
        }
        
        Ok(())
    }
    
    pub fn get_value(&self, section: &str, key: &str) -> Option<ConfigValue> {
        let sections = self.sections.read().unwrap();
        sections.get(section)?.get_value(key).cloned()
    }
    
    pub fn set_value(&self, section: &str, key: &str, value: ConfigValue) -> Result<()> {
        let old_value = self.get_value(section, key);
        
        {
            let mut sections = self.sections.write().unwrap();
            let section_obj = sections.get_mut(section)
                .ok_or_else(|| anyhow!("Section '{}' not found", section))?;
            
            section_obj.set_value(key.to_string(), value.clone())?;
        }
        
        // Notify watchers
        self.notify_watchers(&format!("{}.{}", section, key), old_value.as_ref(), &value);
        
        if self.auto_save {
            self.save_to_file()?;
        }
        
        Ok(())
    }
    
    pub fn register_watcher(&self, name: String, watcher: Box<dyn ConfigWatcher>) {
        let mut watchers = self.watchers.write().unwrap();
        watchers.insert(name, watcher);
    }
    
    pub fn remove_watcher(&self, name: &str) {
        let mut watchers = self.watchers.write().unwrap();
        watchers.remove(name);
    }
    
    pub fn get_typed_value<T: DeserializeOwned>(&self, section: &str, key: &str) -> Result<T> {
        let value = self.get_value(section, key)
            .ok_or_else(|| anyhow!("Key '{}.{}' not found", section, key))?;
        
        // Convert to JSON and then deserialize to the target type
        let json_value = serde_json::to_value(value)?;
        let typed_value: T = serde_json::from_value(json_value)?;
        
        Ok(typed_value)
    }
    
    pub fn set_typed_value<T: Serialize>(&self, section: &str, key: &str, value: T) -> Result<()> {
        let json_value = serde_json::to_value(value)?;
        let config_value: ConfigValue = serde_json::from_value(json_value)?;
        
        self.set_value(section, key, config_value)
    }
    
    pub fn reload(&self) -> Result<()> {
        let metadata = self.metadata.read().unwrap();
        let config_file = metadata.config_file.clone();
        drop(metadata);
        
        if !config_file.as_os_str().is_empty() {
            self.load_from_file(config_file)?;
        }
        
        Ok(())
    }
    
    pub fn validate_all(&self) -> Result<()> {
        let sections = self.sections.read().unwrap();
        
        for (section_name, section) in sections.iter() {
            section.validate_all()
                .map_err(|e| anyhow!("Validation failed for section '{}': {}", section_name, e))?;
        }
        
        Ok(())
    }
    
    pub fn get_section_names(&self) -> Vec<String> {
        let sections = self.sections.read().unwrap();
        sections.keys().cloned().collect()
    }
    
    pub fn get_section(&self, name: &str) -> Option<ConfigSection> {
        let sections = self.sections.read().unwrap();
        sections.get(name).cloned()
    }
    
    pub fn export_to_json(&self) -> Result<String> {
        let sections = self.sections.read().unwrap();
        let json = serde_json::to_string_pretty(&*sections)?;
        Ok(json)
    }
    
    pub fn import_from_json(&self, json: &str) -> Result<()> {
        let imported_sections: HashMap<String, ConfigSection> = serde_json::from_str(json)?;
        
        {
            let mut sections = self.sections.write().unwrap();
            sections.clear();
            sections.extend(imported_sections);
        }
        
        // Validate all imported sections
        self.validate_all()?;
        
        if self.auto_save {
            self.save_to_file()?;
        }
        
        Ok(())
    }
    
    fn notify_watchers(&self, path: &str, old_value: Option<&ConfigValue>, new_value: &ConfigValue) {
        let watchers = self.watchers.read().unwrap();
        
        for watcher in watchers.values() {
            watcher.on_config_changed(path, old_value, new_value);
        }
    }
    
    fn calculate_checksum(&self, content: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

// Helper trait for easy configuration access
pub trait Configurable {
    fn get_config_section() -> ConfigSection;
    fn from_config(config: &FlexibleConfig) -> Result<Self> where Self: Sized;
    fn to_config(&self, config: &FlexibleConfig) -> Result<()>;
}

// Example implementation for Chronicle config
#[derive(Debug, Serialize, Deserialize)]
pub struct ChronicleConfig {
    pub capture: CaptureConfig,
    pub storage: StorageConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureConfig {
    pub keyboard_enabled: bool,
    pub mouse_enabled: bool,
    pub screen_enabled: bool,
    pub screen_fps_active: f64,
    pub screen_fps_idle: f64,
    pub idle_threshold_seconds: u32,
    pub exclude_apps: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageConfig {
    pub base_path: String,
    pub retention_days: u32,
    pub compression_level: u32,
    pub max_file_size_mb: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub encryption_enabled: bool,
    pub key_rotation_days: u32,
    pub audit_enabled: bool,
    pub secret_detection: bool,
}

impl Configurable for ChronicleConfig {
    fn get_config_section() -> ConfigSection {
        let mut section = ConfigSection::new(
            "chronicle".to_string(),
            "Chronicle main configuration".to_string()
        );
        
        // Add validation rules
        section.validation_rules = vec![
            ValidationRule {
                field_path: "capture.screen_fps_active".to_string(),
                rule_type: ValidationType::Range { min: 0.1, max: 60.0 },
                error_message: "Screen FPS must be between 0.1 and 60".to_string(),
            },
            ValidationRule {
                field_path: "capture.screen_fps_idle".to_string(),
                rule_type: ValidationType::Range { min: 0.01, max: 10.0 },
                error_message: "Idle screen FPS must be between 0.01 and 10".to_string(),
            },
            ValidationRule {
                field_path: "storage.retention_days".to_string(),
                rule_type: ValidationType::Range { min: 1.0, max: 3650.0 },
                error_message: "Retention days must be between 1 and 3650 (10 years)".to_string(),
            },
            ValidationRule {
                field_path: "storage.base_path".to_string(),
                rule_type: ValidationType::Required,
                error_message: "Storage base path is required".to_string(),
            },
        ];
        
        section
    }
    
    fn from_config(config: &FlexibleConfig) -> Result<Self> {
        // Try to get from capture section first, then fall back to defaults
        let capture = CaptureConfig {
            keyboard_enabled: config.get_typed_value("capture", "keyboard_enabled").unwrap_or(true),
            mouse_enabled: config.get_typed_value("capture", "mouse_enabled").unwrap_or(true),
            screen_enabled: config.get_typed_value("capture", "screen_enabled").unwrap_or(true),
            screen_fps_active: config.get_typed_value("capture", "screen_fps_active").unwrap_or(1.0),
            screen_fps_idle: config.get_typed_value("capture", "screen_fps_idle").unwrap_or(0.2),
            idle_threshold_seconds: config.get_typed_value("capture", "idle_threshold_seconds").unwrap_or(30),
            exclude_apps: config.get_typed_value("capture", "exclude_apps").unwrap_or_default(),
        };
        
        let storage = StorageConfig {
            base_path: config.get_typed_value("storage", "base_path").unwrap_or_else(|_| "/ChronicleRaw".to_string()),
            retention_days: config.get_typed_value("storage", "retention_days").unwrap_or(60),
            compression_level: config.get_typed_value("storage", "compression_level").unwrap_or(6),
            max_file_size_mb: config.get_typed_value("storage", "max_file_size_mb").unwrap_or(100),
        };
        
        let security = SecurityConfig {
            encryption_enabled: config.get_typed_value("security", "encryption_enabled").unwrap_or(true),
            key_rotation_days: config.get_typed_value("security", "key_rotation_days").unwrap_or(30),
            audit_enabled: config.get_typed_value("security", "audit_enabled").unwrap_or(true),
            secret_detection: config.get_typed_value("security", "secret_detection").unwrap_or(true),
        };
        
        Ok(ChronicleConfig {
            capture,
            storage,
            security,
        })
    }
    
    fn to_config(&self, config: &FlexibleConfig) -> Result<()> {
        // Create sections if they don't exist
        let capture_section = ConfigSection::new("capture".to_string(), "Capture settings".to_string());
        let storage_section = ConfigSection::new("storage".to_string(), "Storage settings".to_string());
        let security_section = ConfigSection::new("security".to_string(), "Security settings".to_string());
        
        config.register_section(capture_section)?;
        config.register_section(storage_section)?;
        config.register_section(security_section)?;
        
        // Set capture values
        config.set_typed_value("capture", "keyboard_enabled", self.capture.keyboard_enabled)?;
        config.set_typed_value("capture", "mouse_enabled", self.capture.mouse_enabled)?;
        config.set_typed_value("capture", "screen_enabled", self.capture.screen_enabled)?;
        config.set_typed_value("capture", "screen_fps_active", self.capture.screen_fps_active)?;
        config.set_typed_value("capture", "screen_fps_idle", self.capture.screen_fps_idle)?;
        config.set_typed_value("capture", "idle_threshold_seconds", self.capture.idle_threshold_seconds)?;
        config.set_typed_value("capture", "exclude_apps", &self.capture.exclude_apps)?;
        
        // Set storage values
        config.set_typed_value("storage", "base_path", &self.storage.base_path)?;
        config.set_typed_value("storage", "retention_days", self.storage.retention_days)?;
        config.set_typed_value("storage", "compression_level", self.storage.compression_level)?;
        config.set_typed_value("storage", "max_file_size_mb", self.storage.max_file_size_mb)?;
        
        // Set security values
        config.set_typed_value("security", "encryption_enabled", self.security.encryption_enabled)?;
        config.set_typed_value("security", "key_rotation_days", self.security.key_rotation_days)?;
        config.set_typed_value("security", "audit_enabled", self.security.audit_enabled)?;
        config.set_typed_value("security", "secret_detection", self.security.secret_detection)?;
        
        Ok(())
    }
}

// Example watcher implementation
pub struct LoggingWatcher {
    name: String,
}

impl LoggingWatcher {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl ConfigWatcher for LoggingWatcher {
    fn on_config_changed(&self, path: &str, old_value: Option<&ConfigValue>, new_value: &ConfigValue) {
        if let Some(old) = old_value {
            println!("[{}] Config changed: {} = {:?} -> {:?}", self.name, path, old, new_value);
        } else {
            println!("[{}] Config set: {} = {:?}", self.name, path, new_value);
        }
    }
    
    fn on_config_error(&self, error: &str) {
        println!("[{}] Config error: {}", self.name, error);
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_config_value_conversion() {
        let string_val = ConfigValue::String("test".to_string());
        assert_eq!(string_val.as_string(), Some("test"));
        assert_eq!(string_val.as_integer(), None);
        
        let int_val = ConfigValue::Integer(42);
        assert_eq!(int_val.as_integer(), Some(42));
        assert_eq!(int_val.as_float(), Some(42.0));
        
        let bool_val = ConfigValue::Boolean(true);
        assert_eq!(bool_val.as_boolean(), Some(true));
    }
    
    #[test]
    fn test_validation_rules() {
        let rule = ValidationRule {
            field_path: "test_field".to_string(),
            rule_type: ValidationType::Range { min: 1.0, max: 10.0 },
            error_message: "Value out of range".to_string(),
        };
        
        assert!(rule.validate(&ConfigValue::Float(5.0)).is_ok());
        assert!(rule.validate(&ConfigValue::Float(0.5)).is_err());
        assert!(rule.validate(&ConfigValue::Float(15.0)).is_err());
    }
    
    #[test]
    fn test_config_section() {
        let mut section = ConfigSection::new("test".to_string(), "Test section".to_string());
        
        section.set_value("key1".to_string(), ConfigValue::String("value1".to_string())).unwrap();
        section.set_value("key2".to_string(), ConfigValue::Integer(42)).unwrap();
        
        assert_eq!(section.get_value("key1").unwrap().as_string(), Some("value1"));
        assert_eq!(section.get_value("key2").unwrap().as_integer(), Some(42));
    }
    
    #[test]
    fn test_flexible_config() {
        let config = FlexibleConfig::new();
        
        let mut section = ConfigSection::new("test".to_string(), "Test section".to_string());
        section.set_value("string_key".to_string(), ConfigValue::String("test_value".to_string())).unwrap();
        section.set_value("int_key".to_string(), ConfigValue::Integer(123)).unwrap();
        
        config.register_section(section).unwrap();
        
        assert_eq!(config.get_value("test", "string_key").unwrap().as_string(), Some("test_value"));
        assert_eq!(config.get_value("test", "int_key").unwrap().as_integer(), Some(123));
        
        // Test typed access
        let typed_string: String = config.get_typed_value("test", "string_key").unwrap();
        assert_eq!(typed_string, "test_value");
        
        let typed_int: i64 = config.get_typed_value("test", "int_key").unwrap();
        assert_eq!(typed_int, 123);
    }
    
    #[test]
    fn test_config_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let config = FlexibleConfig::new()
            .with_paths(vec![config_path.clone()])
            .with_auto_save(false);
        
        // Create test config
        let mut section = ConfigSection::new("app".to_string(), "Application settings".to_string());
        section.set_value("name".to_string(), ConfigValue::String("Chronicle".to_string())).unwrap();
        section.set_value("version".to_string(), ConfigValue::String("1.0.0".to_string())).unwrap();
        section.set_value("debug".to_string(), ConfigValue::Boolean(true)).unwrap();
        
        config.register_section(section).unwrap();
        
        // Set metadata path and save
        {
            let mut metadata = config.metadata.write().unwrap();
            metadata.config_file = config_path.clone();
        }
        config.save_to_file().unwrap();
        
        // Verify file was created
        assert!(config_path.exists());
        
        // Create new config and load
        let new_config = FlexibleConfig::new();
        new_config.load_from_file(&config_path).unwrap();
        
        assert_eq!(new_config.get_value("app", "name").unwrap().as_string(), Some("Chronicle"));
        assert_eq!(new_config.get_value("app", "debug").unwrap().as_boolean(), Some(true));
    }
    
    #[test]
    fn test_chronicle_config() {
        let config = FlexibleConfig::new().with_auto_save(false);
        
        let chronicle_config = ChronicleConfig {
            capture: CaptureConfig {
                keyboard_enabled: true,
                mouse_enabled: false,
                screen_enabled: true,
                screen_fps_active: 2.0,
                screen_fps_idle: 0.1,
                idle_threshold_seconds: 60,
                exclude_apps: vec!["com.apple.test".to_string()],
            },
            storage: StorageConfig {
                base_path: "/custom/path".to_string(),
                retention_days: 90,
                compression_level: 9,
                max_file_size_mb: 200,
            },
            security: SecurityConfig {
                encryption_enabled: true,
                key_rotation_days: 14,
                audit_enabled: false,
                secret_detection: true,
            },
        };
        
        // Save to config
        chronicle_config.to_config(&config).unwrap();
        
        // Load back
        let loaded_config = ChronicleConfig::from_config(&config).unwrap();
        
        assert_eq!(loaded_config.capture.screen_fps_active, 2.0);
        assert_eq!(loaded_config.storage.retention_days, 90);
        assert_eq!(loaded_config.security.key_rotation_days, 14);
        assert!(!loaded_config.capture.mouse_enabled);
    }
}