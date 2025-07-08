# Chronicle

**A privacy-first, local-only lifelogger for macOS designed for future AGI personalization**

Chronicle is a comprehensive system for capturing, storing, and analyzing your digital activity entirely on your local machine. Built to create a rich repository of personal computer use data, Chronicle prepares for the inevitable arrival of Artificial General Intelligence (AGI) by ensuring you have complete ownership and control over your digital behavioral patterns, preferences, and interactions.

With zero cloud dependencies and military-grade encryption, Chronicle gives you complete control over your personal data while building the foundation for truly personalized AI experiences when AGI becomes available.

[![Build Status](https://github.com/chronicle/chronicle/workflows/CI/badge.svg)](https://github.com/chronicle/chronicle/actions)
[![Coverage](https://codecov.io/gh/chronicle/chronicle/branch/main/graph/badge.svg)](https://codecov.io/gh/chronicle/chronicle)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![macOS](https://img.shields.io/badge/macOS-14.0%2B-blue)](https://www.apple.com/macos/)
[![Swift](https://img.shields.io/badge/Swift-5.9%2B-orange)](https://swift.org)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-red)](https://rustlang.org)

## 🚀 Quick Start

### Installation

Download the latest release from [Releases](https://github.com/chronicle/chronicle/releases) or install via Homebrew:

```bash
brew install chronicle-app/tap/chronicle
```

### First Launch

1. **Launch Chronicle** from Applications or via `chronictl start`
2. **Grant Permissions** - Chronicle will request necessary macOS permissions
3. **Configure Settings** - Customize capture preferences in the menu bar app
4. **Start Capturing** - Your digital activity is now being recorded locally

### Basic Usage

```bash
# Check system status
chronictl status

# Search your activity
chronictl search "important document" --last-week

# Export data for analysis
chronictl export --format csv --output ~/activity_data.csv

# Create encrypted backup
chronictl backup create ~/chronicle_backup.tar.gz
```

## ✨ Features

### 🤖 AGI Preparation
- **Personal Data Sovereignty** - Build your own comprehensive digital behavior dataset
- **Future-Proof Architecture** - Designed for seamless integration with future AGI systems
- **Rich Behavioral Patterns** - Capture work habits, preferences, and interaction styles
- **Complete Digital Context** - Document your unique way of using technology

### 🔒 Privacy First
- **100% Local** - All data stays on your device, never leaves your control
- **Zero Telemetry** - No analytics, tracking, or cloud connections
- **AES-256 Encryption** - Military-grade encryption for all stored data
- **Open Source** - Full transparency with complete source code access

### 📊 Comprehensive Capture
- **Screen Activity** - Adaptive screen capture (1fps active, 0.2fps idle)
- **Keyboard Events** - Keystroke patterns and application context
- **Mouse Tracking** - Movement patterns and interaction data
- **Window Management** - Application focus and window metadata
- **Clipboard History** - Secure clipboard change monitoring
- **File System** - Document access and file operation tracking
- **Network Activity** - Domain-level network request monitoring
- **Audio Events** - Meeting detection and audio playback metadata

### ⚡ High Performance
- **Lock-free Architecture** - >100,000 events/second throughput
- **Low Resource Usage** - <3% CPU under normal operation
- **Efficient Storage** - Parquet + HEIF compression (~1-5GB daily)
- **Fast Search** - <100ms query response time across all data

### 🛠 Developer Friendly
- **Multi-language** - Swift collectors, Rust services, C ring buffer
- **Modular Design** - Easy to extend and customize
- **Rich CLI** - Powerful command-line interface
- **Native Integration** - Full macOS system integration

## 🤖 The AGI Vision

Chronicle is built with a forward-looking vision: **when Artificial General Intelligence arrives, you should own and control the data that trains your personal AI.**

### Why Build a Personal Data Repository?

**AGI is Coming**: Whether in 2 years or 10, AGI will fundamentally change how we interact with computers. The question isn't *if*, but *when* - and *who controls the data* that makes AI truly personal.

**Current AI Limitations**: Today's AI systems are trained on generic internet data. They don't know *your* preferences, *your* work patterns, or *your* unique way of thinking and interacting with technology.

**The Personal Data Advantage**: Chronicle captures the rich context of how you actually use computers:
- **Work Patterns**: When you're most productive, what tools you prefer, how you solve problems
- **Behavioral Context**: Your unique keyboard/mouse patterns, application usage, and workflow preferences  
- **Decision Patterns**: What you click, what you ignore, how you navigate and make choices
- **Interaction Style**: Your communication patterns, writing style, and information consumption habits

### Privacy-First AGI Preparation

Unlike cloud-based solutions that harvest your data for corporate AI training, Chronicle ensures:

- **You Own Your Data**: Complete control over your digital behavior dataset
- **Privacy by Design**: Local-only processing means no one else ever sees your data
- **Future-Ready Format**: Structured data ready for integration with future AGI systems
- **Consent-Based Sharing**: You decide what data (if any) to share with AI systems

### The Chronicle Advantage

When AGI becomes available, Chronicle users will have:

1. **Rich Personal Context**: Years of detailed behavioral data for AI personalization
2. **Data Sovereignty**: Complete ownership and control over personal training data
3. **Privacy Protection**: No corporate intermediaries with access to your data
4. **Competitive Edge**: Truly personalized AI while others use generic systems

Chronicle isn't just a lifelogger - it's **preparation for the age of personal AGI**.

## 🏗 Architecture

Chronicle uses a sophisticated multi-process architecture designed for performance, reliability, and privacy:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Collectors    │    │   Ring Buffer   │    │   Packer        │
│   (Swift)       │───▶│   (C/mmap)      │───▶│   (Rust)        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │                       │                       ▼
         │                       │              ┌─────────────────┐
         │                       │              │   Storage       │
         │                       │              │ (Parquet/HEIF)  │
         │                       │              └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Menu Bar UI   │    │   CLI Tool      │    │   Backup        │
│   (SwiftUI)     │    │   (Rust)        │    │   (rsync)       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Core Components

- **Swift Collectors** - Native macOS event capture with proper permissions
- **C Ring Buffer** - High-performance, lock-free inter-process communication
- **Rust Packer** - Efficient data processing and storage service
- **CLI Tool** - Comprehensive command-line interface for all operations
- **SwiftUI App** - Native menu bar application for monitoring and control

## 📋 System Requirements

- **macOS 14.0+** (Sonoma or later)
- **Apple Silicon or Intel** processor
- **4GB RAM** minimum (8GB recommended)
- **10GB free disk space** (for 30 days of data)
- **Administrator privileges** (for initial setup only)

### Required Permissions

Chronicle requires the following macOS permissions:
- **Screen Recording** - For screen capture functionality
- **Input Monitoring** - For keyboard and mouse event capture
- **Accessibility** - For window metadata and application context

All permissions are requested on first launch with clear explanations.

## 🔧 Installation & Setup

### Option 1: Homebrew (Recommended)

```bash
# Add Chronicle tap
brew tap chronicle-app/tap

# Install Chronicle
brew install chronicle

# Start Chronicle service
brew services start chronicle
```

### Option 2: Direct Download

1. Download the latest `.dmg` from [Releases](https://github.com/chronicle/chronicle/releases)
2. Open the DMG and drag Chronicle to Applications
3. Launch Chronicle from Applications folder
4. Follow the setup wizard

### Option 3: Build from Source

```bash
# Clone repository
git clone https://github.com/chronicle/chronicle.git
cd chronicle

# Setup development environment
./scripts/chronicle.sh dev setup

# Build all components
./scripts/chronicle.sh build release

# Install locally
./scripts/chronicle.sh install
```

## 📖 Usage Guide

### Command Line Interface

Chronicle provides a powerful CLI tool called `chronictl`:

```bash
# System status and health
chronictl status
chronictl health

# Search your activity
chronictl search "project meeting" --last-month
chronictl search --app "Xcode" --since "2024-01-01"
chronictl search --type screenshot --before "2024-01-15"

# Export data
chronictl export --format json --output data.json
chronictl export --type events --csv --since "last week"
chronictl export --images --directory ~/screenshots

# Backup operations
chronictl backup create ~/backups/chronicle-$(date +%Y%m%d).tar.gz
chronictl backup restore ~/backups/chronicle-20240101.tar.gz
chronictl backup verify ~/backups/chronicle-20240101.tar.gz

# Configuration management
chronictl config list
chronictl config set capture.screen_fps_active 2.0
chronictl config get storage.retention_days

# Data management
chronictl wipe --before "2023-01-01" --confirm
chronictl analyze --summary --last-month
chronictl stats --detailed
```

### Menu Bar Application

The Chronicle menu bar app provides:

- **Real-time Status** - Live system health and capture statistics
- **Quick Controls** - Start/stop capture, create backups, access settings
- **Permission Management** - Check and repair system permissions
- **Search Interface** - Quick search through captured data
- **Settings Panel** - Configure capture preferences and storage options

### Configuration

Chronicle uses a hierarchical configuration system:

**Global Config** (`~/.config/chronicle/config.toml`):
```toml
[capture]
keyboard_enabled = true
mouse_enabled = true
screen_enabled = true
screen_fps_active = 1.0
screen_fps_idle = 0.2
idle_threshold_seconds = 30
exclude_apps = ["com.apple.keychainaccess"]

[storage]
base_path = "/ChronicleRaw"
retention_days = 60
compression_level = 6
encryption_enabled = true

[backup]
enabled = true
destination = "/Volumes/Backup/Chronicle"
schedule = "daily"
encryption_enabled = true

[security]
key_rotation_days = 30
audit_enabled = true
secret_detection = true

[privacy]
exclude_patterns = ["password", "secret", "token"]
blur_sensitive_areas = true
keyboard_filter_enabled = true
```

## 🔐 Security & Privacy

Chronicle is designed with privacy and security as fundamental principles:

### Privacy Protection

- **Local-Only Storage** - All data remains on your device
- **No Network Connections** - Zero telemetry or cloud synchronization
- **Configurable Capture** - Granular control over what gets recorded
- **Sensitive Data Detection** - Automatic filtering of passwords and secrets
- **Application Exclusions** - Exclude specific apps from monitoring

### Security Features

- **AES-256-GCM Encryption** - All stored data is encrypted at rest
- **Secure Key Management** - Keys stored in macOS Keychain
- **Integrity Verification** - Checksums protect against data corruption
- **Secure Deletion** - Multi-pass wiping for sensitive data removal
- **Code Signing** - All binaries are signed and notarized by Apple

### Audit and Compliance

- **Audit Logging** - Complete audit trail of all system operations
- **Data Minimization** - Only capture necessary data
- **Retention Policies** - Automatic data deletion based on age
- **Access Controls** - File permissions restrict access to authorized users

## 🧪 Development

### Prerequisites

- **Xcode 15.0+** (for Swift components)
- **Rust 1.70+** (for CLI and packer)
- **Clang/LLVM** (for C ring buffer)
- **Make** (for build orchestration)

### Development Setup

```bash
# Clone repository
git clone https://github.com/chronicle/chronicle.git
cd chronicle

# Setup development environment
./scripts/chronicle.sh dev setup

# Install development dependencies
./scripts/chronicle.sh dev deps

# Run development build
./scripts/chronicle.sh dev build

# Run comprehensive tests
./scripts/chronicle.sh test all

# Start development server
./scripts/chronicle.sh dev start
```

### Project Structure

```
Chronicle/
├── ring-buffer/          # C - Lock-free circular buffer
├── collectors/           # Swift - macOS event collectors
├── packer/              # Rust - Data processing service
├── cli/                 # Rust - Command-line interface
├── ui/                  # SwiftUI - Menu bar application
├── tests/               # Comprehensive test suite
├── benchmarks/          # Performance monitoring
├── scripts/             # Build and deployment automation
├── config/              # Configuration templates
└── docs/                # Documentation
```

### Testing

Chronicle includes comprehensive testing across all components:

```bash
# Run all tests
make test

# Run specific test suites
make test-swift      # Swift collector tests
make test-rust       # Rust service tests
make test-c          # C ring buffer tests
make test-integration # End-to-end integration tests

# Run with coverage
make test-coverage

# Run performance benchmarks
make benchmark

# Run stress tests
make stress-test
```

### Contributing

Contributions are welcome! If you'd like to contribute:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes with tests
4. Run the test suite (`make test`)
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

For major changes, please open an issue first or reach out on [X/Twitter @parkerjbeard](https://x.com/parkerjbeard) to discuss your ideas.

## 📊 Performance

Chronicle is optimized for minimal system impact:

| Metric | Target | Achieved |
|--------|---------|-----------|
| Ring Buffer Throughput | >10,000 events/sec | >100,000 events/sec |
| Total CPU Usage | <5% | <3% |
| Memory Usage | <500MB | <300MB |
| Storage Efficiency | 2-10GB/day | 1-5GB/day |
| Search Query Time | <500ms | <100ms |
| UI Responsiveness | <16ms | <16ms |

### Benchmarks

Run performance benchmarks to verify system performance:

```bash
# Full benchmark suite
make benchmark

# Specific component benchmarks
make benchmark-ring-buffer
make benchmark-collectors
make benchmark-packer
make benchmark-search

# Generate performance report
make benchmark-report
```

## 🗺 Roadmap

### Version 1.0 ✅ **Complete**
- Core data capture pipeline
- Local storage with encryption
- CLI interface
- Menu bar application
- Comprehensive testing

### Version 1.1 🚧 **In Progress**
- [ ] Enhanced search capabilities
- [ ] Advanced visualization tools
- [ ] Plugin architecture
- [ ] Cloud backup integration (opt-in)

### Version 1.2 📋 **Planned**
- [ ] AGI integration framework and APIs
- [ ] Behavioral pattern analysis and insights
- [ ] Advanced privacy controls for AI data sharing
- [ ] Cross-platform support (Linux, Windows)

See [ROADMAP.md](ROADMAP.md) for detailed planning.

## 🆘 Support

### Documentation
- [User Guide](docs/USER_GUIDE.md) - Comprehensive usage documentation
- [Developer Guide](docs/DEVELOPER_GUIDE.md) - Development and contribution guide
- [API Reference](docs/API_REFERENCE.md) - Complete API documentation
- [Troubleshooting](docs/TROUBLESHOOTING.md) - Common issues and solutions

### Support & Contact
- [GitHub Issues](https://github.com/chronicle/chronicle/issues) - Bug reports and feature requests
- [X/Twitter @parkerjbeard](https://x.com/parkerjbeard) - Creator and maintainer
- Email: Available via GitHub profile

For questions, feedback, or collaboration opportunities, feel free to reach out via X/Twitter or create a GitHub issue.

## 📄 License

Chronicle is released under the MIT License. See [LICENSE](LICENSE) for full terms.

```
MIT License

Copyright (c) 2024 Parker Beard

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## 🙏 Acknowledgments

Chronicle builds upon excellent open source technologies:

- **Apache Arrow** - Columnar data format
- **Rust** - Systems programming language
- **Swift** - macOS native development
- **Tokio** - Async runtime for Rust
- **Parquet** - Efficient data storage
- **ScreenCaptureKit** - macOS screen capture

Special thanks to the privacy and security communities for their guidance on best practices.

---

**Chronicle: Your digital life, your data, your control.**

Created by [Parker Beard](https://x.com/parkerjbeard) - Follow for updates and privacy-focused software development.