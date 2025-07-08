use std::collections::{HashMap, HashSet};
use serde_json::Value;
use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::utils::TestEvent;

/// Data validation utilities for Chronicle tests
pub struct DataValidator {
    rules: Vec<ValidationRule>,
    stats: ValidationStats,
}

#[derive(Clone)]
pub enum ValidationRule {
    RequiredField(String),
    FieldType(String, FieldType),
    Range(String, f64, f64),
    Pattern(String, regex::Regex),
    Custom(String, Box<dyn Fn(&Value) -> bool + Send + Sync>),
}

#[derive(Clone, Debug)]
pub enum FieldType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Null,
}

#[derive(Default, Clone, Debug)]
pub struct ValidationStats {
    pub total_validated: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors_by_rule: HashMap<String, usize>,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub rule_name: String,
    pub field_path: String,
    pub message: String,
    pub severity: ErrorSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorSeverity {
    Error,
    Warning,
    Info,
}

impl DataValidator {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            stats: ValidationStats::default(),
        }
    }

    pub fn add_rule(&mut self, rule: ValidationRule) {
        self.rules.push(rule);
    }

    pub fn validate_event(&mut self, event: &TestEvent) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        self.stats.total_validated += 1;

        // Validate basic event structure
        if event.id == 0 {
            errors.push(ValidationError {
                rule_name: "event_id".to_string(),
                field_path: "id".to_string(),
                message: "Event ID cannot be zero".to_string(),
                severity: ErrorSeverity::Error,
            });
        }

        if event.event_type.is_empty() {
            errors.push(ValidationError {
                rule_name: "event_type".to_string(),
                field_path: "event_type".to_string(),
                message: "Event type cannot be empty".to_string(),
                severity: ErrorSeverity::Error,
            });
        }

        // Validate timestamp
        let now = Utc::now();
        let future_threshold = now + chrono::Duration::hours(1);
        let past_threshold = now - chrono::Duration::days(365);

        if event.timestamp > future_threshold {
            warnings.push("Event timestamp is in the future".to_string());
        }

        if event.timestamp < past_threshold {
            warnings.push("Event timestamp is more than a year old".to_string());
        }

        // Apply custom validation rules
        for rule in &self.rules {
            match self.apply_rule(rule, &event.data) {
                Ok(rule_errors) => errors.extend(rule_errors),
                Err(e) => {
                    errors.push(ValidationError {
                        rule_name: "rule_application".to_string(),
                        field_path: "data".to_string(),
                        message: format!("Failed to apply validation rule: {}", e),
                        severity: ErrorSeverity::Error,
                    });
                }
            }
        }

        // Update statistics
        if errors.is_empty() {
            self.stats.passed += 1;
        } else {
            self.stats.failed += 1;
            for error in &errors {
                *self.stats.errors_by_rule.entry(error.rule_name.clone()).or_insert(0) += 1;
            }
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    fn apply_rule(&self, rule: &ValidationRule, data: &Value) -> Result<Vec<ValidationError>> {
        let mut errors = Vec::new();

        match rule {
            ValidationRule::RequiredField(field) => {
                if !self.has_field(data, field) {
                    errors.push(ValidationError {
                        rule_name: format!("required_field_{}", field),
                        field_path: field.clone(),
                        message: format!("Required field '{}' is missing", field),
                        severity: ErrorSeverity::Error,
                    });
                }
            }

            ValidationRule::FieldType(field, expected_type) => {
                if let Some(value) = self.get_field_value(data, field) {
                    if !self.check_field_type(value, expected_type) {
                        errors.push(ValidationError {
                            rule_name: format!("field_type_{}", field),
                            field_path: field.clone(),
                            message: format!("Field '{}' has incorrect type", field),
                            severity: ErrorSeverity::Error,
                        });
                    }
                }
            }

            ValidationRule::Range(field, min, max) => {
                if let Some(value) = self.get_field_value(data, field) {
                    if let Some(num) = value.as_f64() {
                        if num < *min || num > *max {
                            errors.push(ValidationError {
                                rule_name: format!("range_{}", field),
                                field_path: field.clone(),
                                message: format!("Field '{}' value {} is outside range [{}, {}]", field, num, min, max),
                                severity: ErrorSeverity::Error,
                            });
                        }
                    }
                }
            }

            ValidationRule::Pattern(field, regex) => {
                if let Some(value) = self.get_field_value(data, field) {
                    if let Some(text) = value.as_str() {
                        if !regex.is_match(text) {
                            errors.push(ValidationError {
                                rule_name: format!("pattern_{}", field),
                                field_path: field.clone(),
                                message: format!("Field '{}' does not match required pattern", field),
                                severity: ErrorSeverity::Error,
                            });
                        }
                    }
                }
            }

            ValidationRule::Custom(name, validator) => {
                if !validator(data) {
                    errors.push(ValidationError {
                        rule_name: name.clone(),
                        field_path: "data".to_string(),
                        message: format!("Custom validation '{}' failed", name),
                        severity: ErrorSeverity::Error,
                    });
                }
            }
        }

        Ok(errors)
    }

    fn has_field(&self, data: &Value, field: &str) -> bool {
        self.get_field_value(data, field).is_some()
    }

    fn get_field_value(&self, data: &Value, field: &str) -> Option<&Value> {
        let parts: Vec<&str> = field.split('.').collect();
        let mut current = data;

        for part in parts {
            current = current.get(part)?;
        }

        Some(current)
    }

    fn check_field_type(&self, value: &Value, expected_type: &FieldType) -> bool {
        match expected_type {
            FieldType::String => value.is_string(),
            FieldType::Number => value.is_number(),
            FieldType::Boolean => value.is_boolean(),
            FieldType::Array => value.is_array(),
            FieldType::Object => value.is_object(),
            FieldType::Null => value.is_null(),
        }
    }

    pub fn validate_events(&mut self, events: &[TestEvent]) -> Vec<ValidationResult> {
        events.iter().map(|event| self.validate_event(event)).collect()
    }

    pub fn get_stats(&self) -> &ValidationStats {
        &self.stats
    }

    pub fn reset_stats(&mut self) {
        self.stats = ValidationStats::default();
    }
}

/// Event integrity checker
pub struct IntegrityChecker {
    expected_checksums: HashMap<u64, String>,
    sequence_tracker: SequenceTracker,
}

#[derive(Default)]
struct SequenceTracker {
    last_id: Option<u64>,
    missing_ids: HashSet<u64>,
    duplicate_ids: HashSet<u64>,
}

impl IntegrityChecker {
    pub fn new() -> Self {
        Self {
            expected_checksums: HashMap::new(),
            sequence_tracker: SequenceTracker::default(),
        }
    }

    pub fn add_expected_checksum(&mut self, event_id: u64, checksum: String) {
        self.expected_checksums.insert(event_id, checksum);
    }

    pub fn check_event_integrity(&mut self, event: &TestEvent) -> IntegrityResult {
        let mut issues = Vec::new();

        // Check checksum if available
        if let Some(expected) = self.expected_checksums.get(&event.id) {
            let actual = event.checksum();
            if &actual != expected {
                issues.push(IntegrityIssue {
                    issue_type: IntegrityIssueType::ChecksumMismatch,
                    event_id: event.id,
                    description: format!("Expected checksum {}, got {}", expected, actual),
                });
            }
        }

        // Check sequence
        if let Some(last_id) = self.sequence_tracker.last_id {
            let expected_next = last_id + 1;
            if event.id != expected_next {
                if event.id < expected_next {
                    // Duplicate or out-of-order
                    if self.sequence_tracker.duplicate_ids.contains(&event.id) {
                        issues.push(IntegrityIssue {
                            issue_type: IntegrityIssueType::DuplicateEvent,
                            event_id: event.id,
                            description: "Duplicate event ID detected".to_string(),
                        });
                    } else {
                        self.sequence_tracker.duplicate_ids.insert(event.id);
                        issues.push(IntegrityIssue {
                            issue_type: IntegrityIssueType::OutOfOrder,
                            event_id: event.id,
                            description: format!("Event {} received after {}", event.id, last_id),
                        });
                    }
                } else {
                    // Gap in sequence
                    for missing_id in (expected_next..event.id) {
                        self.sequence_tracker.missing_ids.insert(missing_id);
                        issues.push(IntegrityIssue {
                            issue_type: IntegrityIssueType::MissingEvent,
                            event_id: missing_id,
                            description: format!("Missing event ID {}", missing_id),
                        });
                    }
                }
            }
        }

        self.sequence_tracker.last_id = Some(event.id.max(self.sequence_tracker.last_id.unwrap_or(0)));

        IntegrityResult {
            event_id: event.id,
            is_valid: issues.is_empty(),
            issues,
        }
    }

    pub fn get_missing_events(&self) -> &HashSet<u64> {
        &self.sequence_tracker.missing_ids
    }

    pub fn get_duplicate_events(&self) -> &HashSet<u64> {
        &self.sequence_tracker.duplicate_ids
    }
}

#[derive(Debug, Clone)]
pub struct IntegrityResult {
    pub event_id: u64,
    pub is_valid: bool,
    pub issues: Vec<IntegrityIssue>,
}

#[derive(Debug, Clone)]
pub struct IntegrityIssue {
    pub issue_type: IntegrityIssueType,
    pub event_id: u64,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IntegrityIssueType {
    ChecksumMismatch,
    MissingEvent,
    DuplicateEvent,
    OutOfOrder,
    CorruptedData,
}

/// Predefined validation rule builders
pub struct ValidationRuleBuilder;

impl ValidationRuleBuilder {
    pub fn required_field(field: &str) -> ValidationRule {
        ValidationRule::RequiredField(field.to_string())
    }

    pub fn field_type(field: &str, field_type: FieldType) -> ValidationRule {
        ValidationRule::FieldType(field.to_string(), field_type)
    }

    pub fn number_range(field: &str, min: f64, max: f64) -> ValidationRule {
        ValidationRule::Range(field.to_string(), min, max)
    }

    pub fn string_pattern(field: &str, pattern: &str) -> Result<ValidationRule> {
        let regex = regex::Regex::new(pattern)?;
        Ok(ValidationRule::Pattern(field.to_string(), regex))
    }

    pub fn email_format(field: &str) -> Result<ValidationRule> {
        Self::string_pattern(field, r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
    }

    pub fn url_format(field: &str) -> Result<ValidationRule> {
        Self::string_pattern(field, r"^https?://[^\s/$.?#].[^\s]*$")
    }

    pub fn non_empty_string(field: &str) -> ValidationRule {
        ValidationRule::Custom(
            format!("non_empty_string_{}", field),
            Box::new({
                let field = field.to_string();
                move |data: &Value| {
                    if let Some(value) = data.get(&field) {
                        if let Some(s) = value.as_str() {
                            !s.trim().is_empty()
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
            })
        )
    }

    pub fn positive_number(field: &str) -> ValidationRule {
        ValidationRule::Custom(
            format!("positive_number_{}", field),
            Box::new({
                let field = field.to_string();
                move |data: &Value| {
                    if let Some(value) = data.get(&field) {
                        if let Some(num) = value.as_f64() {
                            num > 0.0
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
            })
        )
    }
}

impl Default for DataValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for IntegrityChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_data_validator() {
        let mut validator = DataValidator::new();
        
        // Add validation rules
        validator.add_rule(ValidationRuleBuilder::required_field("key"));
        validator.add_rule(ValidationRuleBuilder::field_type("key", FieldType::String));
        
        // Test valid event
        let valid_event = TestEvent::new(
            1,
            "test_event",
            json!({"key": "value"})
        );
        
        let result = validator.validate_event(&valid_event);
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
        
        // Test invalid event
        let invalid_event = TestEvent::new(
            2,
            "test_event",
            json!({"other": "value"})
        );
        
        let result = validator.validate_event(&invalid_event);
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_integrity_checker() {
        let mut checker = IntegrityChecker::new();
        
        // Add expected checksum
        let mut event = TestEvent::new(1, "test", json!({"data": "test"}));
        event.calculate_checksum();
        let checksum = event.checksum();
        checker.add_expected_checksum(1, checksum);
        
        // Check integrity
        let result = checker.check_event_integrity(&event);
        assert!(result.is_valid);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn test_validation_rule_builder() -> Result<()> {
        let email_rule = ValidationRuleBuilder::email_format("email")?;
        let url_rule = ValidationRuleBuilder::url_format("website")?;
        let positive_rule = ValidationRuleBuilder::positive_number("count");
        
        // These would be used in a validator
        let mut validator = DataValidator::new();
        validator.add_rule(email_rule);
        validator.add_rule(url_rule);
        validator.add_rule(positive_rule);
        
        Ok(())
    }
}