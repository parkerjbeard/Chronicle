# Chronicle Test Suite

A comprehensive testing framework for the Chronicle project, providing unit tests, integration tests, performance benchmarks, and stress tests across all components.

## Overview

The Chronicle test suite is designed to ensure the reliability, performance, and security of the Chronicle data collection and analysis system. It includes:

- **Unit Tests**: Component-level testing for individual modules
- **Integration Tests**: End-to-end pipeline testing 
- **Performance Tests**: Benchmarking and performance regression detection
- **Stress Tests**: System stability under extreme loads
- **Mock Components**: Isolated testing with realistic simulation
- **CI/CD Integration**: Automated testing in continuous integration

## Test Architecture

```
tests/
├── integration/          # Integration tests
│   ├── test_full_pipeline.rs
│   ├── test_ring_buffer_integration.rs
│   ├── test_packer_integration.rs
│   ├── test_cli_integration.rs
│   └── test_ui_integration.rs
├── performance/          # Performance benchmarks
│   ├── benchmark_ring_buffer.rs
│   ├── benchmark_collectors.rs
│   ├── benchmark_packer.rs
│   ├── benchmark_search.rs
│   └── stress_test.rs
├── mocks/               # Mock components
│   ├── mock_collectors.rs
│   ├── mock_ring_buffer.rs
│   ├── mock_packer.rs
│   └── test_data_generator.rs
├── utils/               # Test utilities
│   ├── test_harness.rs
│   ├── performance_utils.rs
│   ├── data_validation.rs
│   └── system_utils.rs
├── swift/               # Swift test integration
│   ├── ChronicleCollectorsTests.swift
│   ├── ChronicleUITests.swift
│   └── PerformanceTests.swift
├── run_tests.sh         # Main test runner
├── ci_tests.sh          # CI-optimized test runner
├── test_config.toml     # Test configuration
└── Cargo.toml           # Test dependencies
```

## Quick Start

### Prerequisites

- Rust 1.70+ with Cargo
- Xcode (for Swift tests on macOS)
- Git
- Basic system tools: `jq`, `bc` (optional)

### Running Tests

**Run all tests:**
```bash
./run_tests.sh
```

**Run specific test categories:**
```bash
./run_tests.sh --unit-only         # Unit tests only
./run_tests.sh --integration-only  # Integration tests only
./run_tests.sh --performance-only  # Performance tests only
./run_tests.sh --swift-only        # Swift tests only
./run_tests.sh --stress            # Include stress tests
```

**Run in CI mode:**
```bash
./ci_tests.sh                      # Standard CI run
./ci_tests.sh --quick              # Quick validation for PRs
./ci_tests.sh --enable-stress      # Include stress tests in CI
```

### Configuration

Test behavior can be configured via `test_config.toml`:

```toml
[general]
default_timeout_ms = 30000
log_level = "debug"

[performance]
warmup_iterations = 5
measurement_iterations = 10
confidence_level = 0.95

[stress]
duration_minutes = 5
max_memory_mb = 1000
max_cpu_percent = 80
```

## Test Categories

### Unit Tests

Component-level tests that verify individual module functionality:

- **Ring Buffer**: Memory management, concurrent access, overflow handling
- **Collectors**: Event generation, filtering, error handling
- **Packer**: Data compression, encryption, integrity
- **CLI**: Command parsing, output formatting
- **UI**: State management, user interactions

```bash
cargo test                          # Run all unit tests
cargo test ring_buffer             # Test specific component
cargo test --lib                   # Library tests only
```

### Integration Tests

End-to-end tests that verify component interactions:

- **Full Pipeline**: collectors → ring buffer → packer → storage
- **Concurrent Operations**: Multiple collectors and readers
- **Error Recovery**: Handling failures and restarts
- **Data Integrity**: Checksum validation throughout pipeline

```bash
cargo test --test integration_tests
cargo test test_full_pipeline
```

### Performance Tests

Benchmarking and performance regression detection:

- **Throughput**: Events processed per second
- **Latency**: Response time measurements
- **Memory Usage**: Peak and sustained memory consumption
- **Scalability**: Performance under increasing load

```bash
cargo bench                         # Run all benchmarks
cargo bench ring_buffer            # Specific component benchmarks
cargo bench -- --sample-size 100   # Custom sample size
```

**Performance Thresholds:**
- Ring buffer write: < 1μs per event
- Ring buffer read: < 500ns per event
- Packer processing: < 50μs per event
- Search queries: < 1ms per query

### Stress Tests

System stability testing under extreme conditions:

- **High Event Rates**: 10,000+ events per second
- **Memory Pressure**: Testing with limited memory
- **Long Duration**: Multi-hour stability runs
- **Resource Exhaustion**: Handling edge cases

```bash
./run_tests.sh --stress             # Enable stress tests
cargo bench stress_test             # Direct stress benchmark
```

### Swift Tests

Native macOS collector and UI testing:

- **Collector Tests**: Permission handling, event capture
- **UI Tests**: User interface interactions
- **Performance Tests**: Swift-specific performance metrics

```bash
./run_tests.sh --swift-only
xcodebuild test -project ChronicleCollectors.xcodeproj
```

## Mock Components

The test suite includes comprehensive mock implementations for isolated testing:

### MockRingBuffer

Simulates ring buffer behavior without actual shared memory:

```rust
let ring_buffer = MockRingBuffer::new(1024 * 1024)?;
ring_buffer.write_event(&event).await?;
let events = ring_buffer.read_events(100).await?;
```

Features:
- Configurable capacity and overflow strategies
- Persistence simulation
- Concurrent access testing
- Performance metrics collection

### MockCollector

Simulates various collector types with realistic event generation:

```rust
let collector = MockCollectorFactory::create_keytap_collector(ring_buffer)?;
collector.set_event_rate(100); // 100 events/sec
collector.start_collection().await?;
```

Features:
- Multiple collector types (keyboard, mouse, screen, etc.)
- Configurable event rates and error simulation
- Privacy filtering simulation
- Realistic data generation

### MockPacker

Simulates packer behavior for testing data processing:

```rust
let packer = MockPacker::new(ring_buffer, storage_path)?;
packer.enable_compression(true);
packer.start_packing().await?;
```

Features:
- Compression and encryption simulation
- Configurable batch sizes and processing rates
- Error injection for testing resilience
- Integrity checking

### TestDataGenerator

Generates realistic test data for comprehensive testing:

```rust
let mut generator = TestDataGenerator::new(config);
let events = generator.generate_events(1000)?;
let time_series = generator.generate_time_series_events(start, end, 100.0)?;
```

Features:
- Realistic event data for all collector types
- Deterministic generation for reproducible tests
- Pattern generation (bursts, periodic, random)
- Configurable data sizes and types

## Test Utilities

### TestHarness

Comprehensive test management and validation:

```rust
let harness = TestHarness::new().await?;
let validation = harness.validate_pipeline_output(&storage_path).await?;
let (result, duration) = harness.measure_performance("test", operation).await?;
```

Features:
- Temporary environment management
- Performance measurement utilities
- Data integrity validation
- Resource monitoring

### Assertions

Specialized assertions for Chronicle testing:

```rust
use chronicle_tests::assertions::*;

assert_performance_within_baseline(&metrics, &baseline)?;
assert_event_integrity(&event, &expected_checksum)?;
assert_events_ordered(&events)?;
assert_no_duplicate_events(&events)?;
```

### Test Macros

Convenient macros for common test patterns:

```rust
test_setup!(); // Initialize test environment

performance_test!("operation_name", baseline_duration, {
    // Test code
});

stress_test!("stress_name", duration, {
    // Stress test code
});
```

## CI/CD Integration

### GitHub Actions

```yaml
- name: Run Chronicle Tests
  run: |
    cd tests
    ./ci_tests.sh --coverage-threshold 80
    
- name: Upload Test Results
  uses: actions/upload-artifact@v3
  with:
    name: test-results
    path: tests/ci_results/
```

### GitLab CI

```yaml
test:
  script:
    - cd tests
    - ./ci_tests.sh --quick
  artifacts:
    reports:
      coverage: ci_results/cobertura.xml
    paths:
      - tests/ci_results/
```

### Jenkins

```groovy
stage('Test') {
    steps {
        sh 'cd tests && ./ci_tests.sh'
    }
    post {
        always {
            archiveArtifacts 'tests/ci_results/**/*'
            publishHTML([
                allowMissing: false,
                alwaysLinkToLastBuild: true,
                keepAll: true,
                reportDir: 'tests/ci_results',
                reportFiles: 'test_report.html',
                reportName: 'Chronicle Test Report'
            ])
        }
    }
}
```

## Configuration

### Environment Variables

- `RUST_LOG`: Set logging level (debug, info, warn, error)
- `CHRONICLE_TEST_MODE`: Enable test mode (set to "1")
- `CHRONICLE_TEST_DATA_DIR`: Custom test data directory
- `CI`: Detected automatically in CI environments
- `COVERAGE_THRESHOLD`: Minimum coverage percentage (default: 80)
- `CI_PARALLEL_JOBS`: Number of parallel test jobs

### Test Configuration File

The `test_config.toml` file controls test behavior:

```toml
[general]
default_timeout_ms = 30000
integration_timeout_ms = 60000
performance_timeout_ms = 120000
cleanup_after_tests = true

[ring_buffer]
default_size = 1048576
max_size = 10485760
test_entries = 1000

[collectors]
mock_events_per_second = 100
test_duration_seconds = 30
event_types = ["keypress", "mouse", "window", "clipboard"]

[performance]
warmup_iterations = 5
measurement_iterations = 10
sample_size = 100
confidence_level = 0.95

[ci]
parallel_jobs = 4
quick_tests_only = false
skip_stress_tests = true
coverage_threshold = 80
```

## Best Practices

### Writing Tests

1. **Use descriptive test names**: Clearly indicate what is being tested
2. **Test one thing at a time**: Keep tests focused and atomic
3. **Use appropriate timeouts**: Set realistic timeouts for async operations
4. **Clean up resources**: Ensure tests don't interfere with each other
5. **Mock external dependencies**: Use provided mock components

### Performance Testing

1. **Establish baselines**: Record performance metrics for regression detection
2. **Use consistent hardware**: Run performance tests on similar systems
3. **Warm up before measuring**: Allow JIT compilation and cache warming
4. **Measure multiple iterations**: Use statistical analysis for accuracy
5. **Monitor system resources**: Track memory, CPU, and I/O usage

### Debugging Tests

1. **Enable detailed logging**: Use `RUST_LOG=debug` for verbose output
2. **Preserve test data on failure**: Set `preserve_on_failure = true`
3. **Use test harness utilities**: Leverage validation and measurement tools
4. **Run tests in isolation**: Use `--test-threads=1` for sequential execution
5. **Check system resources**: Monitor memory and CPU usage during tests

## Troubleshooting

### Common Issues

**Tests timeout:**
- Increase timeout values in `test_config.toml`
- Check for deadlocks in concurrent tests
- Monitor system resource usage

**Performance regression:**
- Compare with baseline measurements
- Check for system load during testing
- Verify test environment consistency

**Memory leaks:**
- Use memory profiling tools
- Check mock component cleanup
- Monitor long-running tests

**Permission errors (macOS):**
- Grant accessibility permissions
- Enable screen recording permissions
- Check input monitoring permissions

### Getting Help

1. Check test logs in the results directory
2. Review the test configuration
3. Run tests with debug logging enabled
4. Consult the main Chronicle documentation
5. Open an issue with test output and system information

## Contributing

### Adding New Tests

1. Choose the appropriate test category (unit, integration, performance)
2. Use existing mock components or extend them as needed
3. Follow the established naming conventions
4. Include performance baselines for benchmarks
5. Update this README with new test descriptions

### Extending Mock Components

1. Implement new mock behaviors in the `mocks/` directory
2. Ensure thread safety for concurrent testing
3. Add configuration options for test scenarios
4. Include comprehensive unit tests for mocks
5. Document new mock features and usage

### Improving Test Infrastructure

1. Enhance test utilities and harnesses
2. Add new assertion functions for common patterns
3. Improve CI/CD integration and reporting
4. Optimize test performance and reliability
5. Extend platform support and compatibility

## License

This test suite is part of the Chronicle project and is licensed under the same terms as the main project.