// Dynamic configuration system for future extensibility
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicConfig {
    pub version: String,
    pub last_updated: i64,
    pub sections: HashMap<String, ConfigSection>,
    pub feature_flags: HashMap<String, bool>,
    pub runtime_overrides: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSection {
    pub name: String,
    pub schema_version: String,
    pub values: HashMap<String, ConfigValue>,
    pub validation_rules: Vec<ValidationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<ConfigValue>),
    Object(HashMap<String, ConfigValue>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub field: String,
    pub rule_type: ValidationType,
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationType {
    Range { min: f64, max: f64 },
    Enum { values: Vec<String> },
    Regex { pattern: String },
    Custom { validator: String },
}

pub struct ConfigManager {
    config: RwLock<DynamicConfig>,
    watchers: RwLock<HashMap<String, Box<dyn ConfigWatcher>>>,
}

pub trait ConfigWatcher: Send + Sync {
    fn on_config_changed(&self, section: &str, old_value: &ConfigValue, new_value: &ConfigValue);
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(DynamicConfig::default()),
            watchers: RwLock::new(HashMap::new()),
        }
    }
    
    pub async fn get_value(&self, section: &str, key: &str) -> Option<ConfigValue> {
        let config = self.config.read().await;
        config.sections.get(section)?.values.get(key).cloned()
    }
    
    pub async fn set_value(&self, section: &str, key: &str, value: ConfigValue) -> Result<(), ConfigError> {
        let mut config = self.config.write().await;
        
        // Validate value
        if let Some(section_config) = config.sections.get(section) {
            self.validate_value(key, &value, &section_config.validation_rules)?;
        }
        
        // Get old value for watchers
        let old_value = config.sections
            .get(section)
            .and_then(|s| s.values.get(key).cloned());
        
        // Set new value
        config.sections
            .entry(section.to_string())
            .or_insert_with(|| ConfigSection::default())
            .values
            .insert(key.to_string(), value.clone());
        
        // Notify watchers
        if let Some(old_val) = old_value {
            self.notify_watchers(section, &old_val, &value).await;
        }
        
        Ok(())
    }
    
    pub async fn add_watcher(&self, name: String, watcher: Box<dyn ConfigWatcher>) {
        let mut watchers = self.watchers.write().await;
        watchers.insert(name, watcher);
    }
    
    async fn notify_watchers(&self, section: &str, old_value: &ConfigValue, new_value: &ConfigValue) {
        let watchers = self.watchers.read().await;
        for watcher in watchers.values() {
            watcher.on_config_changed(section, old_value, new_value);
        }
    }
    
    fn validate_value(&self, key: &str, value: &ConfigValue, rules: &[ValidationRule]) -> Result<(), ConfigError> {
        for rule in rules {
            if rule.field == key {
                match &rule.rule_type {
                    ValidationType::Range { min, max } => {
                        if let ConfigValue::Float(val) = value {
                            if val < min || val > max {
                                return Err(ConfigError::ValidationFailed(
                                    format!("Value {} out of range [{}, {}]", val, min, max)
                                ));
                            }
                        }
                    }
                    ValidationType::Enum { values } => {
                        if let ConfigValue::String(val) = value {
                            if !values.contains(val) {
                                return Err(ConfigError::ValidationFailed(
                                    format!("Value {} not in allowed values: {:?}", val, values)
                                ));
                            }
                        }
                    }
                    _ => {} // Other validations
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    #[error("Section not found: {0}")]
    SectionNotFound(String),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
}

impl Default for DynamicConfig {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            last_updated: chrono::Utc::now().timestamp(),
            sections: HashMap::new(),
            feature_flags: HashMap::new(),
            runtime_overrides: HashMap::new(),
        }
    }
}

impl Default for ConfigSection {
    fn default() -> Self {
        Self {
            name: String::new(),
            schema_version: "1.0.0".to_string(),
            values: HashMap::new(),
            validation_rules: Vec::new(),
        }
    }
}