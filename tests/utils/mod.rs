use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Sha256, Digest};

pub mod test_harness;
pub mod performance_utils;
pub mod data_validation;
pub mod system_utils;

pub use test_harness::*;
pub use performance_utils::*;
pub use data_validation::*;
pub use system_utils::*;

/// Core test event structure used across all tests
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestEvent {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub data: Value,
    pub checksum: Option<String>,
}

impl TestEvent {
    pub fn new(id: u64, event_type: &str, data: Value) -> Self {
        Self {
            id,
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            data,
            checksum: None,
        }
    }

    pub fn with_timestamp(id: u64, event_type: &str, data: Value, timestamp: DateTime<Utc>) -> Self {
        Self {
            id,
            timestamp,
            event_type: event_type.to_string(),
            data,
            checksum: None,
        }
    }

    pub fn calculate_checksum(&mut self) {
        let mut hasher = Sha256::new();
        hasher.update(self.id.to_string().as_bytes());
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(self.event_type.as_bytes());
        hasher.update(self.data.to_string().as_bytes());
        
        let result = hasher.finalize();
        self.checksum = Some(format!("{:x}", result));
    }

    pub fn checksum(&self) -> String {
        self.checksum.clone().unwrap_or_else(|| {
            let mut hasher = Sha256::new();
            hasher.update(self.id.to_string().as_bytes());
            hasher.update(self.timestamp.to_rfc3339().as_bytes());
            hasher.update(self.event_type.as_bytes());
            hasher.update(self.data.to_string().as_bytes());
            
            let result = hasher.finalize();
            format!("{:x}", result)
        })
    }

    pub fn size_bytes(&self) -> usize {
        // Estimate size in bytes
        std::mem::size_of::<u64>() + // id
        std::mem::size_of::<DateTime<Utc>>() + // timestamp
        self.event_type.len() + // event_type
        self.data.to_string().len() + // data (serialized)
        self.checksum.as_ref().map_or(0, |c| c.len()) // checksum
    }
}

/// Common test configurations
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub timeout: std::time::Duration,
    pub retry_count: u32,
    pub parallel_tests: bool,
    pub stress_test_duration: std::time::Duration,
    pub performance_baseline: PerformanceBaseline,
}

#[derive(Debug, Clone)]
pub struct PerformanceBaseline {
    pub max_latency_ms: u64,
    pub min_throughput_ops_per_sec: f64,
    pub max_memory_mb: u64,
    pub max_cpu_percent: f64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            timeout: std::time::Duration::from_secs(30),
            retry_count: 3,
            parallel_tests: true,
            stress_test_duration: std::time::Duration::from_secs(60),
            performance_baseline: PerformanceBaseline {
                max_latency_ms: 100,
                min_throughput_ops_per_sec: 1000.0,
                max_memory_mb: 500,
                max_cpu_percent: 80.0,
            },
        }
    }
}

/// Test result aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuite {
    pub name: String,
    pub tests: Vec<TestResult>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub total_duration: std::time::Duration,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub status: TestStatus,
    pub duration: std::time::Duration,
    pub error: Option<String>,
    pub metrics: Option<TestMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMetrics {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub disk_io_mb: f64,
    pub network_io_mb: f64,
    pub latency_ms: f64,
    pub throughput_ops_per_sec: f64,
}

impl TestSuite {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tests: Vec::new(),
            start_time: Utc::now(),
            end_time: None,
            total_duration: std::time::Duration::new(0, 0),
            passed: 0,
            failed: 0,
            skipped: 0,
        }
    }

    pub fn add_test(&mut self, test: TestResult) {
        match test.status {
            TestStatus::Passed => self.passed += 1,
            TestStatus::Failed => self.failed += 1,
            TestStatus::Skipped => self.skipped += 1,
            TestStatus::Timeout => self.failed += 1,
        }
        self.tests.push(test);
    }

    pub fn finish(&mut self) {
        self.end_time = Some(Utc::now());
        self.total_duration = self.end_time.unwrap()
            .signed_duration_since(self.start_time)
            .to_std()
            .unwrap_or_default();
    }

    pub fn success_rate(&self) -> f64 {
        if self.tests.is_empty() {
            0.0
        } else {
            self.passed as f64 / self.tests.len() as f64
        }
    }
}

/// Common test assertions
pub mod assertions {
    use super::*;
    use anyhow::Result;

    pub fn assert_performance_within_baseline(
        metrics: &TestMetrics,
        baseline: &PerformanceBaseline,
    ) -> Result<()> {
        if metrics.latency_ms > baseline.max_latency_ms as f64 {
            return Err(anyhow::anyhow!(
                "Latency {} ms exceeds baseline {} ms",
                metrics.latency_ms,
                baseline.max_latency_ms
            ));
        }

        if metrics.throughput_ops_per_sec < baseline.min_throughput_ops_per_sec {
            return Err(anyhow::anyhow!(
                "Throughput {} ops/sec below baseline {} ops/sec",
                metrics.throughput_ops_per_sec,
                baseline.min_throughput_ops_per_sec
            ));
        }

        if metrics.memory_usage_mb > baseline.max_memory_mb as f64 {
            return Err(anyhow::anyhow!(
                "Memory usage {} MB exceeds baseline {} MB",
                metrics.memory_usage_mb,
                baseline.max_memory_mb
            ));
        }

        if metrics.cpu_usage_percent > baseline.max_cpu_percent {
            return Err(anyhow::anyhow!(
                "CPU usage {}% exceeds baseline {}%",
                metrics.cpu_usage_percent,
                baseline.max_cpu_percent
            ));
        }

        Ok(())
    }

    pub fn assert_event_integrity(event: &TestEvent, expected_checksum: &str) -> Result<()> {
        let actual_checksum = event.checksum();
        if actual_checksum != expected_checksum {
            return Err(anyhow::anyhow!(
                "Event checksum mismatch: expected {}, got {}",
                expected_checksum,
                actual_checksum
            ));
        }
        Ok(())
    }

    pub fn assert_events_ordered(events: &[TestEvent]) -> Result<()> {
        for i in 1..events.len() {
            if events[i].timestamp < events[i - 1].timestamp {
                return Err(anyhow::anyhow!(
                    "Events not in chronological order at index {} and {}",
                    i - 1,
                    i
                ));
            }
        }
        Ok(())
    }

    pub fn assert_no_duplicate_events(events: &[TestEvent]) -> Result<()> {
        let mut seen_ids = std::collections::HashSet::new();
        for event in events {
            if !seen_ids.insert(event.id) {
                return Err(anyhow::anyhow!(
                    "Duplicate event ID found: {}",
                    event.id
                ));
            }
        }
        Ok(())
    }
}

/// Test data generators
pub mod generators {
    use super::*;
    use rand::Rng;
    use serde_json::json;

    pub fn generate_test_events(count: usize) -> Vec<TestEvent> {
        (0..count)
            .map(|i| TestEvent::new(
                i as u64,
                "test_event",
                json!({
                    "sequence": i,
                    "timestamp": Utc::now().timestamp(),
                    "data": format!("test_data_{}", i)
                })
            ))
            .collect()
    }

    pub fn generate_keytap_events(count: usize) -> Vec<TestEvent> {
        let keys = ["a", "b", "c", "d", "e", "space", "enter", "backspace"];
        let modifiers = [
            vec![],
            vec!["shift"],
            vec!["cmd"],
            vec!["ctrl"],
            vec!["alt"],
            vec!["cmd", "shift"],
        ];

        (0..count)
            .map(|i| {
                let key = keys[i % keys.len()];
                let modifier = &modifiers[i % modifiers.len()];
                TestEvent::new(
                    i as u64,
                    "keytap",
                    json!({
                        "key": key,
                        "modifiers": modifier,
                        "application": "test_app",
                        "window_title": "Test Window"
                    })
                )
            })
            .collect()
    }

    pub fn generate_mouse_events(count: usize) -> Vec<TestEvent> {
        let mut rng = rand::thread_rng();
        let event_types = ["click", "move", "scroll", "drag"];
        let buttons = ["left", "right", "middle"];

        (0..count)
            .map(|i| {
                let event_type = event_types[i % event_types.len()];
                let button = buttons[i % buttons.len()];
                TestEvent::new(
                    i as u64,
                    "mouse",
                    json!({
                        "event_type": event_type,
                        "button": button,
                        "x": rng.gen_range(0..1920),
                        "y": rng.gen_range(0..1080),
                        "delta_x": rng.gen_range(-10..10),
                        "delta_y": rng.gen_range(-10..10)
                    })
                )
            })
            .collect()
    }

    pub fn generate_large_events(count: usize, size_kb: usize) -> Vec<TestEvent> {
        let large_data = "x".repeat(size_kb * 1024);
        
        (0..count)
            .map(|i| TestEvent::new(
                i as u64,
                "large_event",
                json!({
                    "sequence": i,
                    "large_data": large_data,
                    "metadata": {
                        "size_kb": size_kb,
                        "generated_at": Utc::now().timestamp()
                    }
                })
            ))
            .collect()
    }

    pub fn generate_random_events(count: usize) -> Vec<TestEvent> {
        let mut rng = rand::thread_rng();
        let event_types = ["keytap", "mouse", "window", "clipboard", "filesystem", "network"];
        
        (0..count)
            .map(|i| {
                let event_type = event_types[rng.gen_range(0..event_types.len())];
                let data = match event_type {
                    "keytap" => json!({
                        "key": format!("key_{}", rng.gen_range(0..26)),
                        "modifiers": if rng.gen_bool(0.3) { vec!["shift"] } else { vec![] }
                    }),
                    "mouse" => json!({
                        "x": rng.gen_range(0..1920),
                        "y": rng.gen_range(0..1080),
                        "button": if rng.gen_bool(0.8) { "left" } else { "right" }
                    }),
                    "window" => json!({
                        "title": format!("Window {}", rng.gen_range(1..100)),
                        "app": format!("App {}", rng.gen_range(1..20))
                    }),
                    "clipboard" => json!({
                        "content_type": "text",
                        "size": rng.gen_range(10..1000)
                    }),
                    "filesystem" => json!({
                        "path": format!("/tmp/file_{}.txt", rng.gen_range(1..1000)),
                        "operation": "write"
                    }),
                    "network" => json!({
                        "host": format!("192.168.1.{}", rng.gen_range(1..255)),
                        "port": rng.gen_range(1000..9999)
                    }),
                    _ => json!({})
                };
                
                TestEvent::new(i as u64, event_type, data)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_checksum() {
        let mut event = TestEvent::new(
            1,
            "test",
            serde_json::json!({"test": "data"})
        );
        
        event.calculate_checksum();
        assert!(event.checksum.is_some());
        
        let checksum1 = event.checksum();
        let checksum2 = event.checksum();
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_event_size() {
        let event = TestEvent::new(
            1,
            "test",
            serde_json::json!({"test": "data"})
        );
        
        assert!(event.size_bytes() > 0);
    }

    #[test]
    fn test_test_suite() {
        let mut suite = TestSuite::new("test_suite");
        
        suite.add_test(TestResult {
            name: "test1".to_string(),
            status: TestStatus::Passed,
            duration: std::time::Duration::from_millis(100),
            error: None,
            metrics: None,
        });
        
        suite.add_test(TestResult {
            name: "test2".to_string(),
            status: TestStatus::Failed,
            duration: std::time::Duration::from_millis(200),
            error: Some("Test failed".to_string()),
            metrics: None,
        });
        
        assert_eq!(suite.passed, 1);
        assert_eq!(suite.failed, 1);
        assert_eq!(suite.success_rate(), 0.5);
    }

    #[test]
    fn test_generators() {
        let events = generators::generate_test_events(10);
        assert_eq!(events.len(), 10);
        
        let keytap_events = generators::generate_keytap_events(5);
        assert_eq!(keytap_events.len(), 5);
        assert!(keytap_events.iter().all(|e| e.event_type == "keytap"));
        
        let mouse_events = generators::generate_mouse_events(3);
        assert_eq!(mouse_events.len(), 3);
        assert!(mouse_events.iter().all(|e| e.event_type == "mouse"));
    }
}