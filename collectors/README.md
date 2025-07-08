# Chronicle Collectors Framework

A comprehensive Swift framework for collecting user activity data on macOS, designed for productivity monitoring and analytics.

## Overview

Chronicle Collectors is a high-performance, privacy-focused data collection framework that captures various types of user activity:

- **Keyboard Events** - Key presses, modifier keys, and text input
- **Screen Capture** - Screenshots at adaptive frame rates using ScreenCaptureKit
- **Window Monitoring** - Window focus changes and application switching
- **Pointer Events** - Mouse movements, clicks, and gestures
- **Clipboard Activity** - Clipboard content changes (with privacy controls)
- **File System Events** - File creation, modification, and deletion
- **Audio Activity** - Audio input/output status and meeting detection
- **Network Activity** - Network usage and connection monitoring

## Architecture

### Core Components

1. **CollectorBase** - Abstract base class for all collectors
2. **EventTypes** - Comprehensive event data structures
3. **RingBufferWriter** - High-performance event storage using Arrow IPC format
4. **PermissionManager** - TCC permission handling for macOS
5. **ConfigManager** - Configuration management and persistence

### Collectors

- `KeyTapCollector` - CGEventTap-based keyboard monitoring
- `ScreenTapCollector` - ScreenCaptureKit-based screen capture
- `WindowMonCollector` - Accessibility API window tracking
- `PointerMonCollector` - Mouse event monitoring with gesture recognition
- `ClipMonCollector` - Pasteboard monitoring
- `FSMonCollector` - FSEvents-based file system monitoring
- `AudioMonCollector` - CoreAudio integration for audio activity
- `NetMonCollector` - Network activity tracking

## Features

### Performance Optimizations

- **Adaptive Frame Rates** - Automatically adjusts collection frequency based on activity
- **Sampling Control** - Configurable sampling rates to reduce overhead
- **Ring Buffer Storage** - Lock-free, high-performance event storage
- **Thread Safety** - All collectors are thread-safe with minimal locking
- **Memory Management** - Careful memory usage with configurable limits

### Privacy & Security

- **TCC Permission Handling** - Proper macOS permission requests
- **Data Filtering** - Configurable sensitive data filtering
- **Encryption Support** - Built-in encryption capabilities
- **Exclusion Lists** - Application and content-based exclusions
- **Data Retention** - Configurable retention policies

### Reliability

- **Error Handling** - Comprehensive error handling and recovery
- **State Management** - Robust collector state machine
- **Performance Monitoring** - Built-in performance metrics
- **Configuration Validation** - Runtime configuration validation

## Quick Start

### Basic Usage

```swift
import ChronicleCollectors

// Get the shared instance
let collectors = ChronicleCollectors.shared

// Check and request permissions
let permissions = collectors.checkAllPermissions()
if !collectors.hasAllRequiredPermissions() {
    try await collectors.requestAllPermissions()
}

// Start all enabled collectors
try await collectors.startCollectors()

// Monitor system health
let health = collectors.getSystemHealth()
print("Events collected: \(health.totalEventsCollected)")

// Stop collectors
try collectors.stopCollectors()
```

### Individual Collector Control

```swift
// Start specific collectors
try collectors.startCollector("key_tap")
try collectors.startCollector("window_mon")

// Get collector statistics
if let stats = collectors.getCollectorStatistics("key_tap") {
    print("Key events: \(stats.eventsCollected)")
    print("Frame rate: \(stats.currentFrameRate) fps")
}

// Stop specific collectors
try collectors.stopCollector("key_tap")
```

### Configuration Management

```swift
// Update collector configuration
let config = CollectorConfiguration(
    enabled: true,
    sampleRate: 0.5,        // 50% sampling
    activeFrameRate: 10.0,  // 10 FPS when active
    idleFrameRate: 1.0      // 1 FPS when idle
)

try collectors.updateCollectorConfiguration("key_tap", config: config)
```

## System Requirements

- **macOS 13.0+** (macOS 12.3+ for ScreenCaptureKit features)
- **Xcode 15.0+**
- **Swift 5.9+**

## Permissions Required

The framework requires various macOS permissions depending on which collectors are enabled:

| Collector | Permissions Required |
|-----------|---------------------|
| KeyTap | Accessibility, Input Monitoring |
| ScreenTap | Screen Recording |
| WindowMon | Accessibility |
| PointerMon | Accessibility, Input Monitoring |
| ClipMon | Accessibility |
| FSMon | Full Disk Access |
| AudioMon | Microphone |
| NetMon | System Policy Control |

## Configuration

### Default Configuration

```swift
let config = ChronicleConfig.default
```

### Custom Configuration

```toml
[general]
app_name = "Chronicle"
auto_start_collectors = true

[privacy]
enable_data_encryption = true
data_retention_days = 30
enable_sensitive_data_filtering = true

[performance]
max_cpu_usage = 10.0
max_memory_usage = 524288000  # 500MB
enable_throttling = true

[ring_buffer]
buffer_size = 104857600  # 100MB
compression_enabled = true
flush_interval = 5.0

[collectors.key_tap]
enabled = true
sample_rate = 1.0
active_frame_rate = 1.0
idle_frame_rate = 0.2

[collectors.screen_tap]
enabled = true
active_frame_rate = 1.0
idle_frame_rate = 0.2
```

## Performance Characteristics

### Typical Resource Usage

- **CPU Usage**: < 2% average, < 5% peak
- **Memory Usage**: 50-200MB depending on configuration
- **Disk I/O**: Minimal with ring buffer batching
- **Network**: None (all data stored locally)

### Adaptive Frame Rates

The framework automatically adjusts collection frequency:

- **Active State**: High frame rate for responsive monitoring
- **Idle State**: Reduced frame rate to conserve resources
- **Sample Rate**: Additional sampling control for further optimization

## Data Format

Events are stored in Arrow IPC format for efficient serialization:

```swift
struct ChronicleEvent {
    let id: UUID
    let type: ChronicleEventType
    let timestamp: TimeInterval
    let data: Data          // JSON-encoded event data
    let metadata: [String: String]
}
```

### Event Types

- `keyTap` - Keyboard events
- `screenCapture` - Screen captures
- `windowFocus` - Window focus changes
- `pointerMove` - Mouse movements
- `pointerClick` - Mouse clicks
- `clipboardChange` - Clipboard changes
- `fileSystemChange` - File system events
- `audioActivity` - Audio activity
- `networkActivity` - Network activity

## Testing

The framework includes comprehensive tests:

```bash
# Run unit tests
xcodebuild test -scheme ChronicleCollectors

# Run performance tests
xcodebuild test -scheme ChronicleCollectors -only-testing:PerformanceTests

# Run integration tests
xcodebuild test -scheme ChronicleCollectors -only-testing:IntegrationTests
```

## Building

### Prerequisites

1. Xcode 15.0 or later
2. macOS 13.0 or later
3. Ring buffer C library (included)

### Build Steps

```bash
# Clone the repository
git clone <repository-url>
cd Chronicle/collectors

# Build the framework
xcodebuild -scheme ChronicleCollectors -configuration Release

# Run tests
xcodebuild test -scheme ChronicleCollectors
```

### Integration

Add the framework to your Xcode project:

1. Drag `ChronicleCollectors.xcodeproj` into your project
2. Add `ChronicleCollectors.framework` to your target's dependencies
3. Import the framework: `import ChronicleCollectors`

## Privacy Considerations

The Chronicle Collectors framework is designed with privacy in mind:

- **Local Storage Only** - All data stays on the user's device
- **Configurable Filtering** - Sensitive content can be filtered
- **Permission Transparency** - Clear permission requests and explanations
- **Data Retention** - Automatic cleanup based on retention policies
- **Encryption** - Optional data encryption at rest

## Contributing

1. Follow Swift coding conventions
2. Add tests for new features
3. Update documentation
4. Ensure all collectors follow the `CollectorProtocol`

## License

Copyright © 2024 Chronicle. All rights reserved.

## Support

For questions and support, please refer to the documentation or submit an issue.