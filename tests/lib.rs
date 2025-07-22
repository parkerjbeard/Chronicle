//! Simplified Chronicle Test Suite
//! Only includes working compatibility tests

// Include our working test modules
pub mod simple_config_tests;
pub mod simple_performance_tests;
pub mod simple_security_tests;  
pub mod simple_integration_tests;
pub mod real_integration_tests;

// Test environment setup
use std::sync::Once;
static INIT: Once = Once::new();

/// Initialize the test environment  
pub fn init_test_environment() {
    INIT.call_once(|| {
        std::env::set_var("CHRONICLE_TEST_MODE", "1");
        std::env::set_var("RUST_BACKTRACE", "1");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test] 
    fn test_environment_initialization() {
        init_test_environment();
        assert_eq!(std::env::var("CHRONICLE_TEST_MODE").unwrap(), "1");
        assert_eq!(std::env::var("RUST_BACKTRACE").unwrap(), "1");
    }
}