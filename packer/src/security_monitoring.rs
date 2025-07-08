use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};
use tokio::sync::mpsc;
use tracing::{error, info, warn, debug};

/// Security event types for monitoring and auditing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    // Authentication events
    AuthenticationAttempt,
    AuthenticationSuccess,
    AuthenticationFailure,
    AuthenticationLockout,
    
    // Authorization events
    AuthorizationGranted,
    AuthorizationDenied,
    PrivilegeEscalation,
    
    // Data access events
    DataAccess,
    SensitiveDataAccess,
    DataExport,
    DataWipe,
    DataBackup,
    
    // API security events
    SuspiciousRequest,
    RateLimitExceeded,
    InputValidationFailure,
    SqlInjectionAttempt,
    XssAttempt,
    
    // System security events
    UnauthorizedAccess,
    SecurityPolicyViolation,
    CryptographicFailure,
    IntegrityViolation,
    
    // Network security events
    SuspiciousNetworkActivity,
    UnauthorizedConnection,
    DataExfiltrationAttempt,
    
    // Administrative events
    ConfigurationChange,
    UserAdded,
    UserRemoved,
    PermissionChange,
}

/// Security event severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecuritySeverity {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/// Security event for audit logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub id: String,
    pub event_type: SecurityEventType,
    pub severity: SecuritySeverity,
    pub timestamp: DateTime<Utc>,
    pub source_ip: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub description: String,
    pub details: HashMap<String, String>,
    pub action_taken: Option<String>,
}

impl SecurityEvent {
    pub fn new(
        event_type: SecurityEventType,
        severity: SecuritySeverity,
        description: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            severity,
            timestamp: Utc::now(),
            source_ip: None,
            user_id: None,
            session_id: None,
            description,
            details: HashMap::new(),
            action_taken: None,
        }
    }

    pub fn with_source_ip(mut self, ip: String) -> Self {
        self.source_ip = Some(ip);
        self
    }

    pub fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_detail(mut self, key: String, value: String) -> Self {
        self.details.insert(key, value);
        self
    }

    pub fn with_action_taken(mut self, action: String) -> Self {
        self.action_taken = Some(action);
        self
    }
}

/// Security metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    pub failed_auth_attempts: u64,
    pub successful_auth_attempts: u64,
    pub rate_limit_violations: u64,
    pub input_validation_failures: u64,
    pub suspicious_requests: u64,
    pub data_access_events: u64,
    pub last_updated: DateTime<Utc>,
}

impl Default for SecurityMetrics {
    fn default() -> Self {
        Self {
            failed_auth_attempts: 0,
            successful_auth_attempts: 0,
            rate_limit_violations: 0,
            input_validation_failures: 0,
            suspicious_requests: 0,
            data_access_events: 0,
            last_updated: Utc::now(),
        }
    }
}

/// Alert configuration for security events
#[derive(Debug, Clone)]
pub struct AlertConfig {
    pub failed_auth_threshold: u32,
    pub failed_auth_window: Duration,
    pub rate_limit_threshold: u32,
    pub suspicious_request_threshold: u32,
    pub enable_email_alerts: bool,
    pub enable_webhook_alerts: bool,
    pub webhook_url: Option<String>,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            failed_auth_threshold: 5,
            failed_auth_window: Duration::from_secs(300), // 5 minutes
            rate_limit_threshold: 10,
            suspicious_request_threshold: 3,
            enable_email_alerts: false,
            enable_webhook_alerts: false,
            webhook_url: None,
        }
    }
}

/// Comprehensive security monitoring system
pub struct SecurityMonitor {
    events: Arc<RwLock<VecDeque<SecurityEvent>>>,
    metrics: Arc<RwLock<SecurityMetrics>>,
    alert_config: AlertConfig,
    event_sender: mpsc::UnboundedSender<SecurityEvent>,
    failure_tracking: Arc<RwLock<HashMap<String, VecDeque<DateTime<Utc>>>>>,
}

impl SecurityMonitor {
    pub fn new(alert_config: AlertConfig) -> (Self, mpsc::UnboundedReceiver<SecurityEvent>) {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        let monitor = Self {
            events: Arc::new(RwLock::new(VecDeque::new())),
            metrics: Arc::new(RwLock::new(SecurityMetrics::default())),
            alert_config,
            event_sender,
            failure_tracking: Arc::new(RwLock::new(HashMap::new())),
        };

        (monitor, event_receiver)
    }

    /// Log a security event
    pub fn log_event(&self, mut event: SecurityEvent) {
        // Update metrics based on event type
        self.update_metrics(&event);

        // Check for alert conditions
        if let Some(action) = self.check_alert_conditions(&event) {
            event.action_taken = Some(action);
        }

        // Log to structured logger
        match event.severity {
            SecuritySeverity::Critical => {
                error!(
                    event_id = %event.id,
                    event_type = ?event.event_type,
                    severity = ?event.severity,
                    source_ip = ?event.source_ip,
                    user_id = ?event.user_id,
                    description = %event.description,
                    "CRITICAL SECURITY EVENT"
                );
            }
            SecuritySeverity::High => {
                warn!(
                    event_id = %event.id,
                    event_type = ?event.event_type,
                    severity = ?event.severity,
                    source_ip = ?event.source_ip,
                    user_id = ?event.user_id,
                    description = %event.description,
                    "HIGH SECURITY EVENT"
                );
            }
            SecuritySeverity::Medium => {
                warn!(
                    event_id = %event.id,
                    event_type = ?event.event_type,
                    severity = ?event.severity,
                    description = %event.description,
                    "MEDIUM SECURITY EVENT"
                );
            }
            SecuritySeverity::Low => {
                info!(
                    event_id = %event.id,
                    event_type = ?event.event_type,
                    description = %event.description,
                    "Security event logged"
                );
            }
        }

        // Store event in memory (with size limit)
        {
            let mut events = self.events.write().unwrap();
            events.push_back(event.clone());
            
            // Keep only last 10,000 events in memory
            while events.len() > 10_000 {
                events.pop_front();
            }
        }

        // Send to event processor
        if let Err(e) = self.event_sender.send(event) {
            error!("Failed to send security event: {}", e);
        }
    }

    /// Update security metrics
    fn update_metrics(&self, event: &SecurityEvent) {
        let mut metrics = self.metrics.write().unwrap();
        
        match event.event_type {
            SecurityEventType::AuthenticationFailure => {
                metrics.failed_auth_attempts += 1;
            }
            SecurityEventType::AuthenticationSuccess => {
                metrics.successful_auth_attempts += 1;
            }
            SecurityEventType::RateLimitExceeded => {
                metrics.rate_limit_violations += 1;
            }
            SecurityEventType::InputValidationFailure => {
                metrics.input_validation_failures += 1;
            }
            SecurityEventType::SuspiciousRequest => {
                metrics.suspicious_requests += 1;
            }
            SecurityEventType::DataAccess |
            SecurityEventType::SensitiveDataAccess |
            SecurityEventType::DataExport => {
                metrics.data_access_events += 1;
            }
            _ => {}
        }
        
        metrics.last_updated = Utc::now();
    }

    /// Check if event triggers any alert conditions
    fn check_alert_conditions(&self, event: &SecurityEvent) -> Option<String> {
        match event.event_type {
            SecurityEventType::AuthenticationFailure => {
                if let Some(ip) = &event.source_ip {
                    self.track_auth_failure(ip, event.timestamp);
                    
                    let failure_count = self.get_failure_count(ip, self.alert_config.failed_auth_window);
                    if failure_count >= self.alert_config.failed_auth_threshold {
                        self.trigger_alert(format!(
                            "IP {} has {} failed authentication attempts in {} seconds",
                            ip,
                            failure_count,
                            self.alert_config.failed_auth_window.as_secs()
                        ));
                        return Some(format!("IP {} temporarily blocked", ip));
                    }
                }
            }
            SecurityEventType::RateLimitExceeded => {
                if let Some(ip) = &event.source_ip {
                    let recent_rate_limits = self.get_recent_rate_limit_count(ip);
                    if recent_rate_limits >= self.alert_config.rate_limit_threshold {
                        self.trigger_alert(format!(
                            "IP {} exceeded rate limit {} times recently",
                            ip, recent_rate_limits
                        ));
                        return Some(format!("IP {} rate limited", ip));
                    }
                }
            }
            SecurityEventType::SuspiciousRequest |
            SecurityEventType::SqlInjectionAttempt |
            SecurityEventType::XssAttempt => {
                if let Some(ip) = &event.source_ip {
                    let suspicious_count = self.get_suspicious_request_count(ip);
                    if suspicious_count >= self.alert_config.suspicious_request_threshold {
                        self.trigger_alert(format!(
                            "IP {} has {} suspicious requests",
                            ip, suspicious_count
                        ));
                        return Some(format!("IP {} flagged as suspicious", ip));
                    }
                }
            }
            SecurityEventType::DataWipe |
            SecurityEventType::PrivilegeEscalation |
            SecurityEventType::UnauthorizedAccess => {
                // Always alert on critical events
                self.trigger_alert(format!(
                    "CRITICAL: {} - {}",
                    format!("{:?}", event.event_type),
                    event.description
                ));
                return Some("Alert triggered".to_string());
            }
            _ => {}
        }
        
        None
    }

    /// Track authentication failures for rate limiting
    fn track_auth_failure(&self, ip: &str, timestamp: DateTime<Utc>) {
        let mut tracking = self.failure_tracking.write().unwrap();
        let failures = tracking.entry(ip.to_string()).or_insert_with(VecDeque::new);
        
        failures.push_back(timestamp);
        
        // Clean up old entries
        let cutoff = timestamp - chrono::Duration::from_std(self.alert_config.failed_auth_window).unwrap();
        while let Some(front) = failures.front() {
            if *front < cutoff {
                failures.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get failure count within time window
    fn get_failure_count(&self, ip: &str, window: Duration) -> u32 {
        let tracking = self.failure_tracking.read().unwrap();
        if let Some(failures) = tracking.get(ip) {
            let cutoff = Utc::now() - chrono::Duration::from_std(window).unwrap();
            failures.iter().filter(|&&ts| ts > cutoff).count() as u32
        } else {
            0
        }
    }

    /// Get recent rate limit violations for IP
    fn get_recent_rate_limit_count(&self, ip: &str) -> u32 {
        let events = self.events.read().unwrap();
        let cutoff = Utc::now() - chrono::Duration::minutes(15);
        
        events
            .iter()
            .filter(|event| {
                matches!(event.event_type, SecurityEventType::RateLimitExceeded) &&
                event.timestamp > cutoff &&
                event.source_ip.as_ref() == Some(ip)
            })
            .count() as u32
    }

    /// Get suspicious request count for IP
    fn get_suspicious_request_count(&self, ip: &str) -> u32 {
        let events = self.events.read().unwrap();
        let cutoff = Utc::now() - chrono::Duration::hours(1);
        
        events
            .iter()
            .filter(|event| {
                matches!(
                    event.event_type,
                    SecurityEventType::SuspiciousRequest |
                    SecurityEventType::SqlInjectionAttempt |
                    SecurityEventType::XssAttempt
                ) &&
                event.timestamp > cutoff &&
                event.source_ip.as_ref() == Some(ip)
            })
            .count() as u32
    }

    /// Trigger security alert
    fn trigger_alert(&self, message: String) {
        error!("🚨 SECURITY ALERT: {}", message);
        
        // In a real implementation, this would:
        // - Send email if configured
        // - Send webhook notification if configured
        // - Trigger SIEM integration
        // - Log to external security monitoring system
        
        if self.alert_config.enable_webhook_alerts {
            if let Some(webhook_url) = &self.alert_config.webhook_url {
                debug!("Would send webhook alert to: {}", webhook_url);
                // TODO: Implement webhook sending
            }
        }
    }

    /// Get current security metrics
    pub fn get_metrics(&self) -> SecurityMetrics {
        self.metrics.read().unwrap().clone()
    }

    /// Get recent security events
    pub fn get_recent_events(&self, limit: usize) -> Vec<SecurityEvent> {
        let events = self.events.read().unwrap();
        events.iter().rev().take(limit).cloned().collect()
    }

    /// Get events by severity
    pub fn get_events_by_severity(&self, severity: SecuritySeverity, limit: usize) -> Vec<SecurityEvent> {
        let events = self.events.read().unwrap();
        events
            .iter()
            .rev()
            .filter(|event| event.severity == severity)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Clean up old tracking data
    pub fn cleanup_old_data(&self) {
        let cutoff = Utc::now() - chrono::Duration::hours(24);
        
        // Clean up failure tracking
        {
            let mut tracking = self.failure_tracking.write().unwrap();
            for failures in tracking.values_mut() {
                while let Some(front) = failures.front() {
                    if *front < cutoff {
                        failures.pop_front();
                    } else {
                        break;
                    }
                }
            }
            tracking.retain(|_, failures| !failures.is_empty());
        }

        // Clean up old events (keep only last 24 hours)
        {
            let mut events = self.events.write().unwrap();
            while let Some(front) = events.front() {
                if front.timestamp < cutoff {
                    events.pop_front();
                } else {
                    break;
                }
            }
        }
    }
}

/// Convenience functions for logging common security events
impl SecurityMonitor {
    pub fn log_auth_attempt(&self, ip: &str, user_id: &str, success: bool) {
        let event = if success {
            SecurityEvent::new(
                SecurityEventType::AuthenticationSuccess,
                SecuritySeverity::Low,
                format!("User {} authenticated successfully", user_id),
            )
        } else {
            SecurityEvent::new(
                SecurityEventType::AuthenticationFailure,
                SecuritySeverity::Medium,
                format!("Authentication failed for user {}", user_id),
            )
        };

        self.log_event(
            event
                .with_source_ip(ip.to_string())
                .with_user(user_id.to_string())
        );
    }

    pub fn log_suspicious_request(&self, ip: &str, details: &str) {
        let event = SecurityEvent::new(
            SecurityEventType::SuspiciousRequest,
            SecuritySeverity::High,
            "Suspicious request detected".to_string(),
        )
        .with_source_ip(ip.to_string())
        .with_detail("details".to_string(), details.to_string());

        self.log_event(event);
    }

    pub fn log_data_access(&self, user_id: &str, resource: &str, sensitive: bool) {
        let event_type = if sensitive {
            SecurityEventType::SensitiveDataAccess
        } else {
            SecurityEventType::DataAccess
        };

        let severity = if sensitive {
            SecuritySeverity::Medium
        } else {
            SecuritySeverity::Low
        };

        let event = SecurityEvent::new(
            event_type,
            severity,
            format!("User {} accessed {}", user_id, resource),
        )
        .with_user(user_id.to_string())
        .with_detail("resource".to_string(), resource.to_string());

        self.log_event(event);
    }

    pub fn log_data_wipe(&self, user_id: &str, details: &str) {
        let event = SecurityEvent::new(
            SecurityEventType::DataWipe,
            SecuritySeverity::Critical,
            format!("Data wipe operation initiated by {}", user_id),
        )
        .with_user(user_id.to_string())
        .with_detail("operation_details".to_string(), details.to_string());

        self.log_event(event);
    }

    pub fn log_rate_limit_exceeded(&self, ip: &str, endpoint: &str) {
        let event = SecurityEvent::new(
            SecurityEventType::RateLimitExceeded,
            SecuritySeverity::Medium,
            format!("Rate limit exceeded for endpoint {}", endpoint),
        )
        .with_source_ip(ip.to_string())
        .with_detail("endpoint".to_string(), endpoint.to_string());

        self.log_event(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_monitor_creation() {
        let config = AlertConfig::default();
        let (monitor, _receiver) = SecurityMonitor::new(config);
        
        let metrics = monitor.get_metrics();
        assert_eq!(metrics.failed_auth_attempts, 0);
    }

    #[tokio::test]
    async fn test_event_logging() {
        let config = AlertConfig::default();
        let (monitor, mut receiver) = SecurityMonitor::new(config);
        
        let event = SecurityEvent::new(
            SecurityEventType::AuthenticationFailure,
            SecuritySeverity::Medium,
            "Test authentication failure".to_string(),
        );
        
        monitor.log_event(event);
        
        // Verify event was sent
        let received_event = receiver.recv().await.unwrap();
        assert_eq!(received_event.description, "Test authentication failure");
        
        // Verify metrics were updated
        let metrics = monitor.get_metrics();
        assert_eq!(metrics.failed_auth_attempts, 1);
    }

    #[tokio::test]
    async fn test_auth_failure_tracking() {
        let config = AlertConfig {
            failed_auth_threshold: 3,
            failed_auth_window: Duration::from_secs(60),
            ..Default::default()
        };
        let (monitor, _receiver) = SecurityMonitor::new(config);
        
        // Log multiple authentication failures
        for i in 0..5 {
            monitor.log_auth_attempt("192.168.1.100", &format!("user{}", i), false);
        }
        
        let failure_count = monitor.get_failure_count("192.168.1.100", Duration::from_secs(60));
        assert_eq!(failure_count, 5);
    }

    #[tokio::test]
    async fn test_metrics_accumulation() {
        let config = AlertConfig::default();
        let (monitor, _receiver) = SecurityMonitor::new(config);
        
        // Log various events
        monitor.log_auth_attempt("192.168.1.1", "user1", false);
        monitor.log_auth_attempt("192.168.1.2", "user2", true);
        monitor.log_suspicious_request("192.168.1.3", "SQL injection attempt");
        monitor.log_data_access("user1", "/sensitive/data", true);
        
        let metrics = monitor.get_metrics();
        assert_eq!(metrics.failed_auth_attempts, 1);
        assert_eq!(metrics.successful_auth_attempts, 1);
        assert_eq!(metrics.suspicious_requests, 1);
        assert_eq!(metrics.data_access_events, 1);
    }

    #[tokio::test]
    async fn test_event_filtering() {
        let config = AlertConfig::default();
        let (monitor, _receiver) = SecurityMonitor::new(config);
        
        // Log events of different severities
        let events = vec![
            SecurityEvent::new(SecurityEventType::AuthenticationSuccess, SecuritySeverity::Low, "Success".to_string()),
            SecurityEvent::new(SecurityEventType::AuthenticationFailure, SecuritySeverity::Medium, "Failure".to_string()),
            SecurityEvent::new(SecurityEventType::DataWipe, SecuritySeverity::Critical, "Wipe".to_string()),
        ];
        
        for event in events {
            monitor.log_event(event);
        }
        
        let critical_events = monitor.get_events_by_severity(SecuritySeverity::Critical, 10);
        assert_eq!(critical_events.len(), 1);
        assert_eq!(critical_events[0].description, "Wipe");
    }
}