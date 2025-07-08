# Chronicle Benchmarks

Comprehensive benchmarking and performance monitoring suite for Chronicle project.

## Overview

Chronicle Benchmarks provides a complete performance testing and monitoring framework with:

- **Core Component Benchmarks**: Ring buffer, collectors, packer, search, and storage performance tests
- **Real-time System Monitoring**: CPU, memory, disk I/O, and network monitoring
- **Performance Analysis**: Trend analysis, regression detection, and optimization recommendations
- **Web Dashboard**: Real-time performance visualization and alerting
- **Comprehensive Reporting**: Multiple output formats including JSON, CSV, HTML, and Prometheus

## Performance Targets

| Component | Target | Metric |
|-----------|---------|---------|
| Ring Buffer | >100,000 events/second | Throughput |
| Collectors | <3% CPU usage total | Resource Usage |
| Packer | Process 1GB/hour | Throughput |
| Search | <100ms for typical queries | Latency |
| Storage | <10MB/day overhead | Storage Efficiency |

## Quick Start

### Installation

```bash
cd /Users/parkerbeard/Chronicle/benchmarks
cargo build --release
```

### Running Benchmarks

```bash
# Run all benchmarks
./target/release/benchmark-runner all

# Run specific component benchmarks
./target/release/benchmark-runner component ring-buffer
./target/release/benchmark-runner component collectors
./target/release/benchmark-runner component packer

# Run specific test
./target/release/benchmark-runner component ring-buffer --test single_producer_single_consumer

# Custom configuration
./target/release/benchmark-runner all --duration 30 --iterations 1000 --concurrency 4
```

### Starting the Monitoring Dashboard

```bash
./target/release/dashboard --port 8080
```

Open http://localhost:8080 in your browser.

### Starting the Metrics Server

```bash
./target/release/monitor --port 9090
```

Prometheus metrics available at http://localhost:9090/metrics

## Benchmark Suites

### Ring Buffer Benchmarks

Tests the core ring buffer implementation for various producer/consumer patterns:

- `single_producer_single_consumer` - Basic single-threaded performance
- `single_producer_multi_consumer` - Fan-out pattern performance
- `multi_producer_single_consumer` - Fan-in pattern performance
- `multi_producer_multi_consumer` - Full concurrent access
- `burst_write_performance` - Peak write throughput
- `sustained_throughput` - Long-term sustained performance
- `memory_pressure` - Performance under memory constraints
- `lock_contention` - High contention scenarios

### Collectors Benchmarks

Tests individual and combined collector performance:

- `keyboard_collector_performance` - Keyboard event collection
- `mouse_collector_performance` - Mouse event collection
- `window_collector_performance` - Window event monitoring
- `filesystem_collector_performance` - File system monitoring
- `network_collector_performance` - Network activity monitoring
- `audio_collector_performance` - Audio capture monitoring
- `screen_collector_performance` - Screen capture performance
- `all_collectors_combined` - System-wide collection performance

### Packer Benchmarks

Tests data processing and storage pipeline:

- `data_processing_throughput` - Raw data processing speed
- `compression_performance` - Compression efficiency and speed
- `encryption_overhead` - Security processing impact
- `parquet_write_performance` - Structured data storage
- `heif_processing_performance` - Image data processing
- `batch_processing_efficiency` - Batch vs. streaming performance

### Search Benchmarks

Tests search functionality and query performance:

- `simple_text_search` - Basic text search performance
- `wildcard_search` - Pattern matching performance
- `regex_search` - Regular expression search
- `date_range_search` - Time-based queries
- `complex_query_search` - Multi-criteria searches
- `large_result_set_search` - Handling large result sets
- `concurrent_search` - Multi-user search performance

### Storage Benchmarks

Tests storage layer performance and efficiency:

- `file_write_performance` - File I/O write performance
- `file_read_performance` - File I/O read performance
- `database_insert_performance` - Database write operations
- `database_query_performance` - Database read operations
- `compression_efficiency` - Storage compression
- `storage_overhead` - Metadata and indexing costs

### System Monitoring Benchmarks

Tests monitoring system accuracy and overhead:

- `cpu_monitoring_accuracy` - CPU usage tracking
- `memory_monitoring_accuracy` - Memory usage tracking
- `disk_io_monitoring` - Disk I/O monitoring
- `network_io_monitoring` - Network I/O monitoring
- `process_monitoring` - Process tracking
- `monitoring_overhead` - Monitoring system impact

## Configuration

Generate a sample configuration file:

```bash
./target/release/benchmark-runner config
```

Example configuration (`benchmark_config.toml`):

```toml
[benchmarks]
enabled_suites = ["ring_buffer", "collectors", "packer", "search", "storage"]
default_duration_seconds = 10
default_iterations = 100
default_concurrency = 1
warmup_duration_seconds = 2
output_format = "json"
output_directory = "./benchmark_results"

[monitoring]
enabled = true
sample_interval_ms = 1000
retention_duration_hours = 24
metrics_export_enabled = true
metrics_export_port = 9090

[dashboard]
enabled = true
port = 8080
host = "127.0.0.1"
auto_refresh_seconds = 5
theme = "dark"

[storage]
database_path = "./benchmark_data.db"
backup_enabled = true
backup_interval_hours = 24
compression_enabled = true

[alerts]
enabled = true
email_notifications = false
webhook_url = ""

[alerts.thresholds]
cpu_usage_percent = 80.0
memory_usage_percent = 85.0
error_rate_percent = 5.0
response_time_ms = 1000.0
```

## Output Formats

### JSON Output

```bash
./target/release/benchmark-runner all --output json --file results.json
```

### CSV Output

```bash
./target/release/benchmark-runner all --output csv --file results.csv
```

### Prometheus Metrics

```bash
./target/release/benchmark-runner all --output prometheus --file metrics.txt
```

## Environment Variables

Configure benchmarks using environment variables:

```bash
export BENCHMARK_DURATION=30
export BENCHMARK_ITERATIONS=1000
export BENCHMARK_CONCURRENCY=4
export DASHBOARD_PORT=8080
export DASHBOARD_HOST=0.0.0.0
```

## Continuous Integration

Example GitHub Actions workflow:

```yaml
name: Performance Benchmarks

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  benchmarks:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    - name: Build benchmarks
      run: |
        cd benchmarks
        cargo build --release
    - name: Run benchmarks
      run: |
        cd benchmarks
        ./target/release/benchmark-runner all --output json --file benchmark_results.json
    - name: Upload results
      uses: actions/upload-artifact@v3
      with:
        name: benchmark-results
        path: benchmarks/benchmark_results.json
```

## Performance Analysis

The benchmark suite includes advanced analysis capabilities:

### Regression Detection

Automatically detects performance regressions by comparing results against historical baselines.

### Bottleneck Identification

Identifies system bottlenecks and provides optimization recommendations.

### Trend Analysis

Tracks performance trends over time and predicts future performance issues.

## API Integration

Chronicle Benchmarks can be integrated into other applications:

```rust
use chronicle_benchmarks::{
    BenchmarkConfig, BenchmarkComponent, PerformanceTargets,
    run_benchmark, run_all_benchmarks,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize benchmarking system
    chronicle_benchmarks::init()?;
    
    // Create benchmark configuration
    let config = BenchmarkConfig {
        duration: Duration::from_secs(10),
        warmup_duration: Duration::from_secs(2),
        iterations: 100,
        concurrency: 1,
        data_size: 1024 * 1024,
        targets: PerformanceTargets::default(),
    };
    
    // Run specific benchmark
    let result = run_benchmark(
        BenchmarkComponent::RingBuffer,
        "single_producer_single_consumer",
        &config
    ).await?;
    
    println!("Benchmark result: {}", serde_json::to_string_pretty(&result)?);
    
    // Run all benchmarks
    let results = run_all_benchmarks(&config).await?;
    println!("Completed {} benchmarks", results.len());
    
    Ok(())
}
```

## Troubleshooting

### Common Issues

1. **Permission denied errors**: Ensure proper permissions for system monitoring
2. **Port already in use**: Check that dashboard/metrics ports are available
3. **High CPU usage during benchmarks**: This is normal for performance testing
4. **Out of memory errors**: Reduce concurrency or iterations for resource-constrained systems

### Debug Mode

Run benchmarks with verbose logging:

```bash
./target/release/benchmark-runner all --verbose
```

### Performance Tuning

For optimal benchmark accuracy:

1. Run on dedicated hardware when possible
2. Disable unnecessary background processes
3. Use consistent system configuration across test runs
4. Allow sufficient warmup time for JIT compilation and caching

## Contributing

1. Add new benchmark tests to appropriate modules
2. Update benchmark test lists in binary and documentation
3. Ensure all tests follow the benchmark interface pattern
4. Include performance targets for new components
5. Add tests to the CI pipeline

## License

MIT License - see LICENSE file for details.