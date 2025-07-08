//! Core benchmarking modules for Chronicle components

pub mod collectors_bench;
pub mod packer_bench;
pub mod ring_buffer_bench;
pub mod search_bench;
pub mod storage_bench;

use crate::{BenchmarkConfig, BenchmarkResult};
use anyhow::Result;

/// Trait for component benchmarks
pub trait ComponentBenchmark {
    /// Run a specific benchmark test
    async fn run_benchmark(test_name: &str, config: &BenchmarkConfig) -> Result<BenchmarkResult>;
    
    /// Run all benchmark tests for this component
    async fn run_all_benchmarks(config: &BenchmarkConfig) -> Result<Vec<BenchmarkResult>>;
    
    /// Get available benchmark tests
    fn get_benchmark_tests() -> Vec<&'static str>;
}