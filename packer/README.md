# Chronicle Packer Service

The Chronicle packer service is responsible for draining the ring buffer nightly and converting Arrow data to Parquet files with HEIF frame organization. It provides secure, reliable, and performant data processing for the Chronicle lifelogging system.

## Features

- **Nightly Processing**: Automatically processes ring buffer data at 03:00 daily
- **Arrow to Parquet Conversion**: Efficient conversion of Arrow IPC messages to compressed Parquet files
- **HEIF Frame Processing**: Organizes and processes HEIF image frames with proper metadata
- **AES-256-GCM Encryption**: Comprehensive encryption for data at rest
- **Data Integrity Verification**: Blake3/SHA-256 checksums and schema validation
- **Performance Monitoring**: Prometheus metrics and comprehensive logging
- **Graceful Shutdown**: Proper signal handling and resource cleanup
- **Configurable Retention**: Automatic cleanup of old data based on policies
- **Key Rotation**: Automatic encryption key rotation for enhanced security

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Ring Buffer   │───▶│   Packer        │───▶│   Storage       │
│   (Arrow IPC)   │    │   Service       │    │   (Parquet)     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                               │
                               ▼
                      ┌─────────────────┐
                      │   Metrics &     │
                      │   Monitoring    │
                      └─────────────────┘
```

## Installation

### Prerequisites

- Rust 1.70+ with Cargo
- macOS 12+ (for keychain integration)
- 64-bit architecture

### Building from Source

```bash
# Clone the repository
git clone https://github.com/chronicle/chronicle.git
cd chronicle/packer

# Build the project
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Development Build

```bash
# Build with debug symbols
cargo build

# Run with logging
RUST_LOG=debug cargo run -- --help
```

## Configuration

The packer service uses TOML configuration files. The default location is `~/.config/chronicle/packer.toml`.

### Example Configuration

```toml
[storage]
base_path = "/ChronicleRaw"
retention_days = 60
compression_level = 6
max_file_size = 1073741824  # 1GB
directory_format = "%Y/%m/%d"

[storage.parquet]
row_group_size = 65536
page_size = 8192
compression = "SNAPPY"
dictionary_encoding = true
statistics = true

[storage.parquet.bloom_filter]
enabled = true
fpp = 0.1
columns = ["app_bundle_id", "window_title"]

[encryption]
enabled = true
algorithm = "AES-256-GCM"
kdf = "Argon2id"
kdf_iterations = 100000
key_rotation_days = 30
keychain_service = "com.chronicle.packer"

[scheduling]
daily_time = "03:00"
timezone = "UTC"
backup_threshold = 52428800  # 50MB
max_processing_time = 3600   # 1 hour

[ring_buffer]
path = "/tmp/chronicle_ring_buffer"
size = 67108864  # 64MB
backpressure_threshold = 0.8
read_timeout = 5000
write_timeout = 1000

[metrics]
enabled = true
bind_address = "127.0.0.1"
port = 9090
collection_interval = 60
export_format = "prometheus"

[logging]
level = "INFO"
format = "json"
file_path = "/var/log/chronicle/packer.log"
structured = true
console = true
```

## Usage

### Command Line Interface

```bash
# Start the service
chronicle-packer start

# Start with custom configuration
chronicle-packer --config /path/to/config.toml start

# Stop the service
chronicle-packer stop

# Check service status
chronicle-packer status

# Trigger manual processing
chronicle-packer process

# Process specific date
chronicle-packer process --date 2024-01-01

# Dry run (validation only)
chronicle-packer process --dry-run

# Export metrics
chronicle-packer metrics --format json --output metrics.json

# Health check
chronicle-packer health

# Configuration validation
chronicle-packer config --show
```

### Daemon Mode

```bash
# Run as daemon with PID file
chronicle-packer --daemon --pid-file /var/run/chronicle-packer.pid start

# JSON logging for structured output
chronicle-packer --json-logs start
```

### Service Management

```bash
# macOS LaunchAgent
sudo launchctl load /Library/LaunchAgents/com.chronicle.packer.plist

# Manual service management
systemctl start chronicle-packer    # Linux
systemctl enable chronicle-packer   # Auto-start
```

## Data Flow

1. **Ring Buffer Monitoring**: Service monitors ring buffer for data and size thresholds
2. **Scheduled Processing**: Daily processing at configured time (default 03:00 UTC)
3. **Data Extraction**: Drains Arrow IPC messages from shared memory ring buffer
4. **Event Processing**: Validates and processes Chronicle events by type
5. **Parquet Writing**: Converts events to optimized Parquet files with compression
6. **HEIF Processing**: Processes image frames with metadata and thumbnails
7. **Encryption**: Applies AES-256-GCM encryption to all files at rest
8. **Integrity Verification**: Calculates checksums and validates data consistency
9. **Cleanup**: Removes old files based on retention policies

## Data Formats

### Chronicle Event Schema

```json
{
  "timestamp_ns": 1640995200000000000,
  "event_type": "key",
  "app_bundle_id": "com.apple.Safari",
  "window_title": "Chronicle - Privacy-First Lifelogging",
  "data": "{\"key\": \"a\", \"modifiers\": [\"cmd\"]}",
  "session_id": "session_20240101_120000",
  "event_id": "event_00001234"
}
```

### Directory Structure

```
/ChronicleRaw/
├── parquet/
│   ├── 2024/
│   │   ├── 01/
│   │   │   ├── 01/
│   │   │   │   ├── events_20240101_030000.parquet
│   │   │   │   └── events_20240101_150000.parquet
│   │   │   └── 02/
│   │   └── 02/
│   └── 2023/
├── heif/
│   ├── 2024/
│   │   ├── 01/
│   │   │   ├── 01/
│   │   │   │   ├── frame_20240101_030000_000001.heif
│   │   │   │   └── frame_20240101_030000_000002.heif
├── metadata/
│   └── files.json
└── temp/
```

## Security

### Encryption

- **Algorithm**: AES-256-GCM with 256-bit keys
- **Key Derivation**: Argon2id with configurable iterations
- **Key Storage**: macOS Keychain with biometric protection
- **Key Rotation**: Automatic rotation based on age and usage
- **Nonce Generation**: Cryptographically secure random nonces

### Data Integrity

- **Checksums**: Blake3 (default) or SHA-256 for all files
- **Schema Validation**: Arrow schema verification for events
- **Temporal Consistency**: Timestamp ordering and duplicate detection
- **File Verification**: Size, format, and content validation

### Access Control

- **File Permissions**: Restrictive permissions on data directories
- **Keychain Access**: Biometric authentication for encryption keys
- **Process Isolation**: Service runs with minimal privileges
- **Audit Logging**: Comprehensive logging of all operations

## Performance

### Benchmarks

Typical performance on modern hardware:

- **Event Processing**: 50,000+ events/second
- **Parquet Writing**: 25 MB/second sustained
- **Encryption**: 100+ MB/second (AES-256-GCM)
- **Integrity Checks**: 200+ MB/second (Blake3)
- **Memory Usage**: <100MB for typical workloads

### Optimization

- **Batch Processing**: Events processed in optimized batches
- **Compression**: Configurable Parquet compression (Snappy, GZIP, LZ4, ZSTD)
- **Parallel Processing**: Multi-threaded I/O and computation
- **Memory Management**: Streaming processing for large datasets
- **Disk I/O**: Asynchronous operations with read-ahead

## Monitoring

### Metrics

The service exposes Prometheus metrics on port 9090 (configurable):

```
# Event processing
chronicle_events_processed_total
chronicle_events_failed_total
chronicle_processing_duration_seconds

# Storage
chronicle_files_created_total
chronicle_bytes_processed_total
chronicle_storage_duration_seconds

# System
chronicle_memory_usage_bytes
chronicle_cpu_usage_percent
chronicle_disk_usage_bytes

# Ring buffer
chronicle_ring_buffer_size_bytes
chronicle_ring_buffer_utilization_percent

# Errors
chronicle_ring_buffer_errors_total
chronicle_storage_errors_total
chronicle_encryption_errors_total
```

### Health Checks

```bash
# Service health
curl http://localhost:9090/health

# Detailed metrics
curl http://localhost:9090/metrics
```

### Logging

Structured JSON logging with configurable levels:

```json
{
  "timestamp": "2024-01-01T03:00:00.123Z",
  "level": "INFO",
  "message": "Daily processing completed",
  "events_processed": 12543,
  "files_created": 3,
  "bytes_processed": 2048576,
  "duration_ms": 1250
}
```

## Development

### Project Structure

```
packer/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library exports
│   ├── config.rs        # Configuration management
│   ├── error.rs         # Error types and handling
│   ├── packer.rs        # Core service logic
│   ├── storage.rs       # Parquet/HEIF storage
│   ├── encryption.rs    # AES-256-GCM encryption
│   ├── integrity.rs     # Data validation
│   └── metrics.rs       # Performance monitoring
├── tests/
│   └── integration_tests.rs
├── benches/
│   └── packer_benchmarks.rs
├── examples/
│   ├── basic_usage.rs
│   ├── storage_demo.rs
│   └── encryption_demo.rs
└── Cargo.toml
```

### Running Examples

```bash
# Basic usage demonstration
cargo run --example basic_usage

# Storage manager demo
cargo run --example storage_demo

# Encryption functionality demo
cargo run --example encryption_demo
```

### Testing

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration_tests

# Test with logging
RUST_LOG=debug cargo test

# Test specific module
cargo test storage

# Test with release optimizations
cargo test --release
```

### Benchmarking

```bash
# Run all benchmarks
cargo bench

# Specific benchmark
cargo bench event_processing

# Generate HTML reports
cargo bench -- --output-format html
```

## Troubleshooting

### Common Issues

1. **Permission Denied**
   ```bash
   # Check directory permissions
   ls -la /ChronicleRaw
   
   # Fix permissions
   sudo chown -R $(whoami) /ChronicleRaw
   chmod 755 /ChronicleRaw
   ```

2. **Keychain Access**
   ```bash
   # Check keychain status
   security list-keychains
   
   # Reset keychain (if needed)
   security delete-generic-password -s com.chronicle.packer
   ```

3. **Ring Buffer Connection**
   ```bash
   # Check ring buffer file
   ls -la /tmp/chronicle_ring_buffer
   
   # Check processes using ring buffer
   lsof /tmp/chronicle_ring_buffer
   ```

4. **Disk Space**
   ```bash
   # Check available space
   df -h /ChronicleRaw
   
   # Force cleanup
   chronicle-packer process --cleanup
   ```

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=chronicle_packer=debug cargo run

# Trace level (very verbose)
RUST_LOG=chronicle_packer=trace cargo run

# Log to file
RUST_LOG=info cargo run 2>&1 | tee packer.log
```

### Performance Issues

```bash
# Profile memory usage
cargo run --features="profiling" --bin chronicle-packer

# Check system resources
top -pid $(pgrep chronicle-packer)

# Monitor file I/O
sudo fs_usage -f filesys -pid $(pgrep chronicle-packer)
```

## Contributing

### Development Setup

1. Install Rust toolchain
2. Clone repository
3. Run tests: `cargo test`
4. Check formatting: `cargo fmt`
5. Run linter: `cargo clippy`
6. Run security audit: `cargo audit`

### Pull Request Process

1. Create feature branch
2. Write tests for new functionality
3. Update documentation
4. Ensure benchmarks pass
5. Submit pull request with description

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Use descriptive variable names
- Add comprehensive error handling
- Include unit tests for all functions
- Document public APIs with rustdoc

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Support

For support and questions:

- Documentation: https://chronicle.dev/docs
- Issues: https://github.com/chronicle/chronicle/issues
- Discussions: https://github.com/chronicle/chronicle/discussions
- Security: security@chronicle.dev