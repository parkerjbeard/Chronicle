# Chronicle CLI (chronictl)

A powerful command-line interface for Chronicle data collection and analysis system.

## Overview

Chronicle CLI (`chronictl`) provides comprehensive command-line access to Chronicle's data collection, search, analysis, and management capabilities. It's designed for system administrators, security analysts, and developers who need to interact with Chronicle data programmatically or through terminal interfaces.

## Features

- **Status Monitoring**: Check service health, storage usage, and system metrics
- **Advanced Search**: Query events with regex, time ranges, and complex filters
- **Data Export**: Export data in multiple formats (JSON, CSV, Parquet, Arrow)
- **Event Replay**: Replay historical events with timing simulation
- **Backup Management**: Create, verify, and manage data backups
- **Secure Data Wipe**: Safely delete data with multiple confirmation levels
- **Configuration Management**: View, modify, and validate system configuration
- **Multiple Output Formats**: Table, JSON, CSV, YAML, and raw output
- **Interactive Mode**: Browse large datasets with pagination and filtering
- **Shell Completion**: Support for Bash, Zsh, Fish, and PowerShell

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/your-org/chronicle.git
cd chronicle/cli

# Build and install
cargo build --release
cargo install --path .
```

### Using Cargo

```bash
cargo install chronictl
```

## Quick Start

### 1. Check Service Status
```bash
# Basic health check
chronictl status

# Detailed system information
chronictl status --detailed

# Simple connectivity test
chronictl status --ping
```

### 2. Search Events
```bash
# Search for errors in the last day
chronictl search --query "error" --time "last-day"

# Search with regex pattern
chronictl search --query "/login.*fail/" --limit 50

# Search with filters
chronictl search --query "*" --filters "type=security,level=high"

# Search specific time range
chronictl search --query "authentication" --time "2024-01-01..2024-01-02"
```

### 3. Export Data
```bash
# Export to JSON
chronictl export --format json --time "today" --output events.json

# Export to CSV with compression
chronictl export --format csv --compression gzip --time "last-week"

# Export specific query results
chronictl export --query "error" --format parquet --time "last-month"
```

### 4. Configuration Management
```bash
# Show current configuration
chronictl config show

# Set a configuration value
chronictl config set service_url "http://localhost:8080"

# Get specific configuration value
chronictl config get timeout

# Export configuration to file
chronictl config export --file config.json --format json
```

## Command Reference

### Global Options

- `--url <URL>`: Override service URL
- `--format <FORMAT>`: Output format (table, json, csv, yaml, raw)
- `--no-color`: Disable colored output
- `--verbose`: Enable verbose logging
- `--debug`: Enable debug logging
- `--quiet`: Quiet mode (minimal output)
- `--config <FILE>`: Configuration file path

### Commands

#### `status` - Service Status
Check Chronicle service health and system information.

```bash
chronictl status [OPTIONS]

Options:
  -d, --detailed    Show detailed system information
      --ping        Check connectivity only
      --timeout     Timeout in seconds (default: 10)
      --raw         Show raw JSON output
```

#### `search` - Event Search
Search and filter events with advanced query capabilities.

```bash
chronictl search [OPTIONS] --query <QUERY>

Options:
  -q, --query <QUERY>           Search query (supports regex with /pattern/)
  -t, --time <TIME>             Time range (e.g., "last-week", "2024-01-01..2024-01-02")
  -l, --limit <LIMIT>           Maximum results (default: 100)
      --offset <OFFSET>         Skip results (for pagination)
  -f, --filters <FILTERS>       Additional filters (key=value,key2=value2)
      --sort <SORT>             Sort order (newest, oldest)
      --stats                   Show search statistics
  -i, --interactive             Interactive mode for large result sets
      --save <FILE>             Save results to file
      --timeout <SECONDS>       Search timeout (default: 30)
      --ids-only                Show only event IDs
  -c, --count                   Count only (don't return events)
```

#### `export` - Data Export
Export Chronicle data in various formats.

```bash
chronictl export [OPTIONS]

Options:
  -f, --format <FORMAT>         Export format (json, csv, parquet, arrow)
  -q, --query <QUERY>           Search query for data to export
  -t, --time <TIME>             Time range for export
  -l, --limit <LIMIT>           Maximum records to export
      --filters <FILTERS>       Additional filters
  -o, --output <FILE>           Output file path
  -c, --compression <TYPE>      Compression (gzip, bzip2, lz4)
      --overwrite               Overwrite existing files
      --stream                  Streaming mode for large datasets
      --progress                Show progress during export
      --validate                Validate exported data
      --timeout <SECONDS>       Export timeout (default: 300)
```

#### `replay` - Event Replay
Replay historical events with timing simulation.

```bash
chronictl replay [OPTIONS] --time <TIME>

Options:
  -t, --time <TIME>             Time range for replay (required)
  -q, --query <QUERY>           Filter events during replay
      --filters <FILTERS>       Additional filters
      --speed <MULTIPLIER>      Replay speed (1.0 = real-time, default: 1.0)
  -l, --limit <LIMIT>           Maximum events to replay (default: 1000)
  -o, --output <FILE>           Record replay session to file
      --follow                  Continue replaying new events
      --event-types <TYPES>     Show only specific event types
      --pause-at <IDS>          Pause at specific event IDs
  -i, --interactive             Interactive mode (pause/resume)
      --detailed                Show detailed event information
      --colorize                Colorize output by event type
      --timeout <SECONDS>       Timeout (default: 3600)
```

#### `backup` - Data Backup
Create and manage Chronicle data backups.

```bash
chronictl backup [OPTIONS] --destination <PATH>

Options:
  -d, --destination <PATH>      Backup destination path (required)
      --include-metadata        Include metadata in backup
  -c, --compression <TYPE>      Compression format (gzip, bzip2, lz4)
      --encryption <PASSWORD>   Encryption password
      --overwrite               Overwrite existing backup
      --verify                  Verify backup integrity
      --progress                Show progress during backup
      --time <TIME>             Backup specific time range
      --event-types <TYPES>     Backup only specific event types
      --dry-run                 Show what would be backed up
      --timeout <SECONDS>       Timeout (default: 3600)
```

#### `wipe` - Secure Data Deletion
Securely delete Chronicle data with multiple confirmations.

```bash
chronictl wipe [OPTIONS]

Options:
      --confirm-with-passphrase <PHRASE>  Confirmation passphrase
      --preserve-config                   Preserve configuration files
      --secure-delete                     Use secure deletion methods
      --force                             Skip additional confirmations
      --time <TIME>                       Wipe specific time range only
      --event-types <TYPES>               Wipe specific event types only
      --dry-run                           Show what would be wiped
      --timeout <SECONDS>                 Timeout (default: 600)
```

#### `config` - Configuration Management
Manage Chronicle configuration settings.

```bash
chronictl config <SUBCOMMAND>

Subcommands:
  show        Show current configuration
  set         Set configuration value
  get         Get configuration value
  delete      Delete configuration key
  validate    Validate configuration
  reset       Reset to default configuration
  import      Import configuration from file
  export      Export configuration to file
```

#### `completions` - Shell Completions
Generate shell completion scripts.

```bash
chronictl completions <SHELL>

Shells:
  bash        Bash completion
  zsh         Zsh completion
  fish        Fish completion
  powershell  PowerShell completion
```

## Configuration

Chronicle CLI can be configured through:

1. **Configuration file**: `~/.chronicle/config.toml` or `~/.chronicle/config.json`
2. **Environment variables**: `CHRONICLE_URL`, `CHRONICLE_CONFIG`, etc.
3. **Command-line options**: Override any setting

### Example Configuration File

```json
{
  "service_url": "http://localhost:8080",
  "timeout": 30,
  "max_results": 1000,
  "default_format": "table",
  "colored_output": true,
  "auto_paging": true
}
```

## Examples

### Basic Usage

```bash
# Check if Chronicle is running
chronictl status --ping

# Search for recent errors
chronictl search --query "error" --time "last-hour" --limit 20

# Export today's security events
chronictl export --query "security" --time "today" --format json
```

### Advanced Search

```bash
# Regex search for SQL injection attempts
chronictl search --query "/(union|select|insert|drop).*sql/i" --time "last-week"

# Search with multiple filters
chronictl search --query "*" --filters "type=web,status=failed,severity=high"

# Interactive search with large result sets
chronictl search --query "authentication" --interactive --limit 1000
```

### Data Management

```bash
# Create encrypted backup
chronictl backup --destination /backups/chronicle.tar.gz --compression gzip --encryption

# Replay events from a specific incident
chronictl replay --time "2024-01-15T14:00:00..2024-01-15T15:00:00" --speed 2.0

# Export high-priority events to CSV
chronictl export --filters "priority=high" --format csv --time "last-month"
```

### Configuration

```bash
# Show current configuration
chronictl config show

# Set service URL
chronictl config set service_url "https://chronicle.example.com"

# Export configuration for backup
chronictl config export --file chronicle-config.json
```

## Output Formats

Chronicle CLI supports multiple output formats:

- **table**: Human-readable table format (default)
- **json**: Structured JSON output
- **csv**: Comma-separated values
- **yaml**: YAML format
- **raw**: Raw data output

## Environment Variables

- `CHRONICLE_URL`: Service URL
- `CHRONICLE_CONFIG`: Configuration file path
- `CHRONICLE_CONFIG_DIR`: Configuration directory
- `CHRONICLE_DATA_DIR`: Data directory
- `CHRONICLE_CACHE_DIR`: Cache directory

## Security

Chronicle CLI includes several security features:

- **Encrypted backups**: Protect backup data with strong encryption
- **Secure deletion**: Multiple-pass data wiping with confirmation
- **Access validation**: Verify user permissions before destructive operations
- **Configuration validation**: Ensure configuration settings are secure

## Troubleshooting

### Common Issues

1. **Service unavailable**: Check if Chronicle service is running and accessible
2. **Permission denied**: Verify user permissions for data directories
3. **Configuration errors**: Validate configuration file syntax
4. **Network timeouts**: Increase timeout values for large operations

### Debug Mode

Enable debug logging for detailed troubleshooting:

```bash
chronictl --debug <command>
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Support

- Documentation: https://chronicle.example.com/docs
- Issues: https://github.com/your-org/chronicle/issues
- Community: https://community.chronicle.example.com