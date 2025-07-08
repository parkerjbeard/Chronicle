//! Chronicle Test Suite
//! 
//! Comprehensive testing framework for the Chronicle project, providing:
//! - Unit and integration tests
//! - Performance benchmarks
//! - Stress testing
//! - Mock components for isolated testing
//! - CI/CD integration
//! 
//! # Usage
//! 
//! Run all tests:
//! ```bash
//! ./run_tests.sh
//! ```
//! 
//! Run specific test categories:
//! ```bash
//! ./run_tests.sh --unit-only
//! ./run_tests.sh --performance-only
//! ./run_tests.sh --stress
//! ```
//! 
//! Run in CI mode:
//! ```bash
//! ./ci_tests.sh
//! ```

pub mod utils;
pub mod mocks;
pub mod integration;
pub mod performance;

// Re-export commonly used test utilities
pub use utils::{
    TestEvent, TestConfig, TestHarness, TestSuite, TestResult, TestStatus,
    generators, assertions,
};

pub use mocks::{
    MockCollector, MockRingBuffer, MockPacker,
};

// Test configuration constants
pub const DEFAULT_TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
pub const DEFAULT_STRESS_DURATION: std::time::Duration = std::time::Duration::from_secs(60);
pub const DEFAULT_PERFORMANCE_ITERATIONS: usize = 100;

// Test environment setup
use std::sync::Once;
static INIT: Once = Once::new();

/// Initialize the test environment
/// This should be called once before running any tests
pub fn init_test_environment() {
    INIT.call_once(|| {
        // Initialize logging for tests
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive("chronicle_tests=debug".parse().unwrap())
            )
            .with_test_writer()
            .init();
        
        // Set test-specific environment variables
        std::env::set_var("CHRONICLE_TEST_MODE", "1");
        std::env::set_var("RUST_BACKTRACE", "1");
        
        tracing::info!("Chronicle test environment initialized");
    });
}

/// Common test setup macro
#[macro_export]
macro_rules! test_setup {
    () => {
        $crate::init_test_environment();
        let _guard = tracing::info_span!("test").entered();
    };
}

/// Performance test macro with baseline checking
#[macro_export]
macro_rules! performance_test {
    ($name:expr, $baseline:expr, $test:expr) => {{
        $crate::init_test_environment();
        let start = std::time::Instant::now();
        let result = $test;
        let duration = start.elapsed();
        
        if duration > $baseline {
            panic!(
                "Performance test '{}' exceeded baseline: {:?} > {:?}",
                $name, duration, $baseline
            );
        }
        
        tracing::info!(
            "Performance test '{}' completed in {:?} (baseline: {:?})",
            $name, duration, $baseline
        );
        
        result
    }};
}

/// Stress test macro with resource monitoring
#[macro_export]
macro_rules! stress_test {
    ($name:expr, $duration:expr, $test:expr) => {{
        $crate::init_test_environment();
        
        let start = std::time::Instant::now();
        let mut iterations = 0;
        let mut errors = 0;
        
        tracing::info!("Starting stress test '{}' for {:?}", $name, $duration);
        
        while start.elapsed() < $duration {
            match $test {
                Ok(_) => iterations += 1,
                Err(e) => {
                    errors += 1;
                    tracing::warn!("Stress test iteration failed: {}", e);
                }
            }
        }
        
        let success_rate = iterations as f64 / (iterations + errors) as f64;
        
        tracing::info!(
            "Stress test '{}' completed: {} iterations, {} errors, {:.2}% success rate",
            $name, iterations, errors, success_rate * 100.0
        );
        
        if success_rate < 0.95 {
            panic!(
                "Stress test '{}' success rate too low: {:.2}%",
                $name, success_rate * 100.0
            );
        }
        
        (iterations, errors, success_rate)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_environment_initialization() {
        init_test_environment();
        
        // Verify environment variables are set
        assert_eq!(std::env::var("CHRONICLE_TEST_MODE").unwrap(), "1");
        assert_eq!(std::env::var("RUST_BACKTRACE").unwrap(), "1");
    }
    
    #[test]
    fn test_setup_macro() {
        test_setup!();
        // Test should run without panicking
    }
    
    #[test]
    fn test_performance_macro() {
        let result = performance_test!(
            "simple_operation",
            std::time::Duration::from_millis(100),
            {
                std::thread::sleep(std::time::Duration::from_millis(10));
                "completed"
            }
        );
        
        assert_eq!(result, "completed");
    }
    
    #[test]
    #[should_panic(expected = "Performance test")]
    fn test_performance_macro_failure() {
        performance_test!(
            "slow_operation",
            std::time::Duration::from_millis(10),
            {
                std::thread::sleep(std::time::Duration::from_millis(50));
                "completed"
            }
        );
    }
    
    #[test]
    fn test_stress_macro() {
        let (iterations, errors, success_rate) = stress_test!(
            "simple_stress",
            std::time::Duration::from_millis(100),
            {
                std::thread::sleep(std::time::Duration::from_millis(1));
                Ok::<(), &str>(())
            }
        );
        
        assert!(iterations > 0);
        assert_eq!(errors, 0);
        assert!(success_rate >= 0.95);
    }
}

/// Test utilities for common operations
pub mod test_utils {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;
    
    /// Create a temporary directory for testing
    pub fn create_temp_dir() -> anyhow::Result<TempDir> {
        Ok(TempDir::new()?)
    }
    
    /// Create a test event with default values
    pub fn create_test_event(id: u64) -> TestEvent {
        TestEvent::new(
            id,
            "test_event",
            serde_json::json!({
                "id": id,
                "test": true,
                "timestamp": chrono::Utc::now().timestamp()
            })
        )
    }
    
    /// Wait for a condition to be true with timeout
    pub async fn wait_for_condition<F>(
        mut condition: F,
        timeout: std::time::Duration,
        check_interval: std::time::Duration,
    ) -> anyhow::Result<()>
    where
        F: FnMut() -> bool,
    {
        let start = std::time::Instant::now();
        
        while start.elapsed() < timeout {
            if condition() {
                return Ok(());
            }
            tokio::time::sleep(check_interval).await;
        }
        
        Err(anyhow::anyhow!("Condition not met within timeout"))
    }
    
    /// Measure execution time of an async operation
    pub async fn measure_async<F, R>(operation: F) -> (R, std::time::Duration)
    where
        F: std::future::Future<Output = R>,
    {
        let start = std::time::Instant::now();
        let result = operation.await;
        let duration = start.elapsed();
        (result, duration)
    }
    
    /// Generate test data file
    pub fn generate_test_data_file(
        dir: &PathBuf,
        name: &str,
        size_kb: usize,
    ) -> anyhow::Result<PathBuf> {
        let file_path = dir.join(name);
        let data = "x".repeat(size_kb * 1024);
        std::fs::write(&file_path, data)?;
        Ok(file_path)
    }
}

/// Test result analysis utilities
pub mod analysis {
    use super::*;
    use std::collections::HashMap;
    
    /// Analyze test results and generate statistics
    pub fn analyze_test_results(results: &[TestResult]) -> TestAnalysis {
        let total = results.len();
        let passed = results.iter().filter(|r| matches!(r.status, TestStatus::Passed)).count();
        let failed = results.iter().filter(|r| matches!(r.status, TestStatus::Failed)).count();
        let skipped = results.iter().filter(|r| matches!(r.status, TestStatus::Skipped)).count();
        
        let total_duration: std::time::Duration = results.iter()
            .map(|r| r.duration)
            .sum();
        
        let average_duration = if total > 0 {
            total_duration / total as u32
        } else {
            std::time::Duration::new(0, 0)
        };
        
        TestAnalysis {
            total_tests: total,
            passed_tests: passed,
            failed_tests: failed,
            skipped_tests: skipped,
            success_rate: if total > 0 { passed as f64 / total as f64 } else { 0.0 },
            total_duration,
            average_duration,
            slowest_test: results.iter()
                .max_by_key(|r| r.duration)
                .map(|r| r.name.clone()),
        }
    }
    
    /// Compare performance metrics against baselines
    pub fn compare_performance(
        current: &HashMap<String, f64>,
        baseline: &HashMap<String, f64>,
        tolerance: f64,
    ) -> PerformanceComparison {
        let mut regressions = Vec::new();
        let mut improvements = Vec::new();
        let mut unchanged = Vec::new();
        
        for (metric, current_value) in current {
            if let Some(baseline_value) = baseline.get(metric) {
                let change_percent = (current_value - baseline_value) / baseline_value * 100.0;
                
                if change_percent.abs() <= tolerance {
                    unchanged.push(metric.clone());
                } else if change_percent > 0.0 {
                    regressions.push((metric.clone(), change_percent));
                } else {
                    improvements.push((metric.clone(), change_percent.abs()));
                }
            }
        }
        
        PerformanceComparison {
            regressions,
            improvements,
            unchanged,
        }
    }
    
    #[derive(Debug, Clone)]
    pub struct TestAnalysis {
        pub total_tests: usize,
        pub passed_tests: usize,
        pub failed_tests: usize,
        pub skipped_tests: usize,
        pub success_rate: f64,
        pub total_duration: std::time::Duration,
        pub average_duration: std::time::Duration,
        pub slowest_test: Option<String>,
    }
    
    #[derive(Debug, Clone)]
    pub struct PerformanceComparison {
        pub regressions: Vec<(String, f64)>,      // (metric, percent_increase)
        pub improvements: Vec<(String, f64)>,     // (metric, percent_decrease)
        pub unchanged: Vec<String>,
    }
}

// Export analysis utilities
pub use analysis::{TestAnalysis, PerformanceComparison};