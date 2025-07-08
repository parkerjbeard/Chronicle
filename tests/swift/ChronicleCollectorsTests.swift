import XCTest
import Foundation
import ChronicleCollectors

/// Comprehensive test suite for Chronicle collectors
class ChronicleCollectorsTests: XCTestCase {
    
    // MARK: - Test Setup
    
    var ringBuffer: MockRingBuffer!
    var testConfig: TestConfiguration!
    var testHarness: TestHarness!
    
    override func setUp() {
        super.setUp()
        
        // Initialize test environment
        ringBuffer = MockRingBuffer(capacity: 1024 * 1024) // 1MB
        testConfig = TestConfiguration.default()
        testHarness = TestHarness()
        
        // Setup logging for tests
        TestLogger.shared.configure(level: .debug)
    }
    
    override func tearDown() {
        // Cleanup test environment
        ringBuffer = nil
        testConfig = nil
        testHarness?.cleanup()
        testHarness = nil
        
        super.tearDown()
    }
    
    // MARK: - Collector Initialization Tests
    
    func testKeyTapCollectorInitialization() {
        let collector = KeyTapCollector(ringBuffer: ringBuffer)
        
        XCTAssertNotNil(collector)
        XCTAssertFalse(collector.isCollecting)
        XCTAssertEqual(collector.collectorType, "KeyTapCollector")
        
        // Test configuration
        let config = CollectorConfiguration()
        config.enablePrivacyFiltering = true
        config.batchSize = 10
        
        collector.configure(with: config)
        XCTAssertEqual(collector.configuration.batchSize, 10)
        XCTAssertTrue(collector.configuration.enablePrivacyFiltering)
    }
    
    func testPointerMonCollectorInitialization() {
        let collector = PointerMonCollector(ringBuffer: ringBuffer)
        
        XCTAssertNotNil(collector)
        XCTAssertFalse(collector.isCollecting)
        XCTAssertEqual(collector.collectorType, "PointerMonCollector")
        
        // Test mouse tracking configuration
        let config = CollectorConfiguration()
        config.sampleRate = 60 // 60 Hz sampling
        config.enableFiltering = true
        
        collector.configure(with: config)
        XCTAssertEqual(collector.configuration.sampleRate, 60)
    }
    
    func testScreenTapCollectorInitialization() {
        let collector = ScreenTapCollector(ringBuffer: ringBuffer)
        
        XCTAssertNotNil(collector)
        XCTAssertFalse(collector.isCollecting)
        XCTAssertEqual(collector.collectorType, "ScreenTapCollector")
        
        // Test screenshot configuration
        let config = CollectorConfiguration()
        config.captureInterval = 5.0 // 5 seconds
        config.compressionQuality = 0.8
        
        collector.configure(with: config)
        XCTAssertEqual(collector.configuration.captureInterval, 5.0)
        XCTAssertEqual(collector.configuration.compressionQuality, 0.8)
    }
    
    func testWindowMonCollectorInitialization() {
        let collector = WindowMonCollector(ringBuffer: ringBuffer)
        
        XCTAssertNotNil(collector)
        XCTAssertFalse(collector.isCollecting)
        XCTAssertEqual(collector.collectorType, "WindowMonCollector")
        
        // Test window tracking configuration
        let config = CollectorConfiguration()
        config.trackWindowContent = false // Privacy mode
        config.enableFiltering = true
        
        collector.configure(with: config)
        XCTAssertFalse(collector.configuration.trackWindowContent)
    }
    
    func testClipMonCollectorInitialization() {
        let collector = ClipMonCollector(ringBuffer: ringBuffer)
        
        XCTAssertNotNil(collector)
        XCTAssertFalse(collector.isCollecting)
        XCTAssertEqual(collector.collectorType, "ClipMonCollector")
    }
    
    func testFSMonCollectorInitialization() {
        let collector = FSMonCollector(ringBuffer: ringBuffer)
        
        XCTAssertNotNil(collector)
        XCTAssertFalse(collector.isCollecting)
        XCTAssertEqual(collector.collectorType, "FSMonCollector")
        
        // Test filesystem monitoring configuration
        let config = CollectorConfiguration()
        config.watchPaths = ["/tmp/test", "/Users/test/Documents"]
        config.enableFiltering = true
        
        collector.configure(with: config)
        XCTAssertEqual(collector.configuration.watchPaths.count, 2)
    }
    
    func testNetMonCollectorInitialization() {
        let collector = NetMonCollector(ringBuffer: ringBuffer)
        
        XCTAssertNotNil(collector)
        XCTAssertFalse(collector.isCollecting)
        XCTAssertEqual(collector.collectorType, "NetMonCollector")
        
        // Test network monitoring configuration
        let config = CollectorConfiguration()
        config.monitorLocalTraffic = false
        config.enableFiltering = true
        
        collector.configure(with: config)
        XCTAssertFalse(collector.configuration.monitorLocalTraffic)
    }
    
    func testAudioMonCollectorInitialization() {
        let collector = AudioMonCollector(ringBuffer: ringBuffer)
        
        XCTAssertNotNil(collector)
        XCTAssertFalse(collector.isCollecting)
        XCTAssertEqual(collector.collectorType, "AudioMonCollector")
        
        // Test audio monitoring configuration
        let config = CollectorConfiguration()
        config.sampleRate = 44100
        config.enableCompression = true
        
        collector.configure(with: config)
        XCTAssertEqual(collector.configuration.sampleRate, 44100)
    }
    
    // MARK: - Permission Tests
    
    func testPermissionManager() {
        let permissionManager = PermissionManager.shared
        
        // Test permission checking
        XCTAssertNotNil(permissionManager)
        
        // Test individual permissions
        let accessibilityStatus = permissionManager.checkAccessibilityPermission()
        let screenRecordingStatus = permissionManager.checkScreenRecordingPermission()
        let inputMonitoringStatus = permissionManager.checkInputMonitoringPermission()
        
        XCTAssertTrue([
            PermissionStatus.granted,
            PermissionStatus.denied,
            PermissionStatus.notDetermined
        ].contains(accessibilityStatus))
        
        XCTAssertTrue([
            PermissionStatus.granted,
            PermissionStatus.denied,
            PermissionStatus.notDetermined
        ].contains(screenRecordingStatus))
        
        XCTAssertTrue([
            PermissionStatus.granted,
            PermissionStatus.denied,
            PermissionStatus.notDetermined
        ].contains(inputMonitoringStatus))
    }
    
    func testPermissionRequests() {
        let permissionManager = PermissionManager.shared
        let expectation = self.expectation(description: "Permission request")
        
        permissionManager.requestAllPermissions { permissions in
            XCTAssertNotNil(permissions)
            XCTAssertEqual(permissions.count, 3) // Accessibility, Screen Recording, Input Monitoring
            expectation.fulfill()
        }
        
        waitForExpectations(timeout: 5.0)
    }
    
    // MARK: - Collection Tests
    
    func testKeyTapCollection() {
        let collector = KeyTapCollector(ringBuffer: ringBuffer)
        let expectation = self.expectation(description: "Key tap collection")
        
        // Configure collector
        let config = CollectorConfiguration()
        config.testMode = true
        config.simulatedEventRate = 10 // 10 events per second
        collector.configure(with: config)
        
        // Start collection
        collector.startCollection()
        XCTAssertTrue(collector.isCollecting)
        
        // Wait for events to be collected
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.0) {
            collector.stopCollection()
            XCTAssertFalse(collector.isCollecting)
            
            // Verify events were collected
            let events = self.ringBuffer.getEvents()
            XCTAssertGreaterThan(events.count, 0)
            
            // Verify event structure
            for event in events {
                XCTAssertEqual(event.type, "keytap")
                XCTAssertNotNil(event.data["key"])
                XCTAssertNotNil(event.timestamp)
            }
            
            expectation.fulfill()
        }
        
        waitForExpectations(timeout: 5.0)
    }
    
    func testPointerMonCollection() {
        let collector = PointerMonCollector(ringBuffer: ringBuffer)
        let expectation = self.expectation(description: "Pointer collection")
        
        // Configure collector
        let config = CollectorConfiguration()
        config.testMode = true
        config.simulatedEventRate = 20 // 20 events per second
        collector.configure(with: config)
        
        // Start collection
        collector.startCollection()
        XCTAssertTrue(collector.isCollecting)
        
        // Wait for events to be collected
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) {
            collector.stopCollection()
            XCTAssertFalse(collector.isCollecting)
            
            // Verify events were collected
            let events = self.ringBuffer.getEvents()
            XCTAssertGreaterThan(events.count, 0)
            
            // Verify event structure
            for event in events {
                XCTAssertEqual(event.type, "pointer")
                XCTAssertNotNil(event.data["x"])
                XCTAssertNotNil(event.data["y"])
                XCTAssertNotNil(event.timestamp)
            }
            
            expectation.fulfill()
        }
        
        waitForExpectations(timeout: 3.0)
    }
    
    func testScreenTapCollection() {
        let collector = ScreenTapCollector(ringBuffer: ringBuffer)
        let expectation = self.expectation(description: "Screen tap collection")
        
        // Configure collector
        let config = CollectorConfiguration()
        config.testMode = true
        config.captureInterval = 1.0 // 1 second intervals
        collector.configure(with: config)
        
        // Start collection
        collector.startCollection()
        XCTAssertTrue(collector.isCollecting)
        
        // Wait for screenshots to be captured
        DispatchQueue.main.asyncAfter(deadline: .now() + 3.0) {
            collector.stopCollection()
            XCTAssertFalse(collector.isCollecting)
            
            // Verify events were collected
            let events = self.ringBuffer.getEvents()
            XCTAssertGreaterThan(events.count, 0)
            
            // Verify event structure
            for event in events {
                XCTAssertEqual(event.type, "screen")
                XCTAssertNotNil(event.data["screenshot_hash"])
                XCTAssertNotNil(event.data["screen_size"])
                XCTAssertNotNil(event.timestamp)
            }
            
            expectation.fulfill()
        }
        
        waitForExpectations(timeout: 5.0)
    }
    
    // MARK: - Performance Tests
    
    func testCollectorPerformance() {
        let collector = KeyTapCollector(ringBuffer: ringBuffer)
        
        // Configure for high-rate testing
        let config = CollectorConfiguration()
        config.testMode = true
        config.simulatedEventRate = 1000 // 1000 events per second
        collector.configure(with: config)
        
        // Measure performance
        let startTime = CFAbsoluteTimeGetCurrent()
        
        collector.startCollection()
        
        // Run for 5 seconds
        Thread.sleep(forTimeInterval: 5.0)
        
        collector.stopCollection()
        
        let endTime = CFAbsoluteTimeGetCurrent()
        let duration = endTime - startTime
        
        // Verify performance
        let events = ringBuffer.getEvents()
        let eventsPerSecond = Double(events.count) / duration
        
        XCTAssertGreaterThan(eventsPerSecond, 500) // Should handle at least 500 events/sec
        
        // Verify memory usage
        let memoryUsage = collector.getMemoryUsage()
        XCTAssertLessThan(memoryUsage, 100 * 1024 * 1024) // Should use less than 100MB
    }
    
    func testConcurrentCollectors() {
        let collectors = [
            KeyTapCollector(ringBuffer: ringBuffer),
            PointerMonCollector(ringBuffer: ringBuffer),
            WindowMonCollector(ringBuffer: ringBuffer)
        ]
        
        let expectation = self.expectation(description: "Concurrent collection")
        
        // Configure all collectors
        for collector in collectors {
            let config = CollectorConfiguration()
            config.testMode = true
            config.simulatedEventRate = 10
            collector.configure(with: config)
        }
        
        // Start all collectors
        for collector in collectors {
            collector.startCollection()
            XCTAssertTrue(collector.isCollecting)
        }
        
        // Wait for collection
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.0) {
            // Stop all collectors
            for collector in collectors {
                collector.stopCollection()
                XCTAssertFalse(collector.isCollecting)
            }
            
            // Verify events from all collectors
            let events = self.ringBuffer.getEvents()
            XCTAssertGreaterThan(events.count, 0)
            
            // Verify we have events from different collectors
            let eventTypes = Set(events.map { $0.type })
            XCTAssertGreaterThan(eventTypes.count, 1)
            
            expectation.fulfill()
        }
        
        waitForExpectations(timeout: 5.0)
    }
    
    // MARK: - Error Handling Tests
    
    func testCollectorErrorHandling() {
        let collector = KeyTapCollector(ringBuffer: ringBuffer)
        
        // Configure with invalid settings to trigger errors
        let config = CollectorConfiguration()
        config.testMode = true
        config.simulateErrors = true
        config.errorRate = 0.1 // 10% error rate
        collector.configure(with: config)
        
        let expectation = self.expectation(description: "Error handling")
        
        // Set error handler
        collector.setErrorHandler { error in
            XCTAssertNotNil(error)
            print("Collector error: \(error)")
        }
        
        // Start collection
        collector.startCollection()
        
        // Wait for errors to occur
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.0) {
            collector.stopCollection()
            
            // Verify collector handled errors gracefully
            XCTAssertFalse(collector.isCollecting)
            
            // Verify some events were still collected despite errors
            let events = self.ringBuffer.getEvents()
            XCTAssertGreaterThan(events.count, 0)
            
            expectation.fulfill()
        }
        
        waitForExpectations(timeout: 5.0)
    }
    
    // MARK: - Privacy Tests
    
    func testPrivacyFiltering() {
        let collector = KeyTapCollector(ringBuffer: ringBuffer)
        
        // Configure with privacy filtering
        let config = CollectorConfiguration()
        config.testMode = true
        config.enablePrivacyFiltering = true
        config.privacyLevel = .enhanced
        config.simulatedEventRate = 10
        collector.configure(with: config)
        
        let expectation = self.expectation(description: "Privacy filtering")
        
        // Start collection
        collector.startCollection()
        
        // Wait for collection
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.0) {
            collector.stopCollection()
            
            // Verify events were filtered for privacy
            let events = self.ringBuffer.getEvents()
            XCTAssertGreaterThan(events.count, 0)
            
            // Verify sensitive data was filtered
            for event in events {
                if let keyData = event.data["key"] as? String {
                    XCTAssertFalse(keyData.contains("password"))
                    XCTAssertFalse(keyData.contains("secret"))
                }
            }
            
            expectation.fulfill()
        }
        
        waitForExpectations(timeout: 5.0)
    }
    
    // MARK: - Ring Buffer Integration Tests
    
    func testRingBufferIntegration() {
        let collector = KeyTapCollector(ringBuffer: ringBuffer)
        
        // Test ring buffer overflow handling
        let config = CollectorConfiguration()
        config.testMode = true
        config.simulatedEventRate = 10000 // Very high rate to test overflow
        collector.configure(with: config)
        
        let expectation = self.expectation(description: "Ring buffer integration")
        
        // Start collection
        collector.startCollection()
        
        // Wait for buffer to fill
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) {
            collector.stopCollection()
            
            // Verify buffer handled overflow gracefully
            let events = self.ringBuffer.getEvents()
            XCTAssertLessThanOrEqual(events.count, self.ringBuffer.capacity)
            
            // Verify buffer stats
            let stats = self.ringBuffer.getStats()
            XCTAssertGreaterThan(stats.totalWrites, 0)
            XCTAssertGreaterThanOrEqual(stats.totalWrites, stats.totalReads)
            
            expectation.fulfill()
        }
        
        waitForExpectations(timeout: 3.0)
    }
    
    // MARK: - Configuration Tests
    
    func testConfigurationManagement() {
        let configManager = ConfigManager.shared
        
        // Test default configuration
        let defaultConfig = configManager.getDefaultConfiguration()
        XCTAssertNotNil(defaultConfig)
        XCTAssertEqual(defaultConfig.batchSize, 100)
        XCTAssertTrue(defaultConfig.enablePrivacyFiltering)
        
        // Test custom configuration
        var customConfig = CollectorConfiguration()
        customConfig.batchSize = 50
        customConfig.enablePrivacyFiltering = false
        customConfig.sampleRate = 30
        
        configManager.updateConfiguration(customConfig)
        
        let retrievedConfig = configManager.getCurrentConfiguration()
        XCTAssertEqual(retrievedConfig.batchSize, 50)
        XCTAssertFalse(retrievedConfig.enablePrivacyFiltering)
        XCTAssertEqual(retrievedConfig.sampleRate, 30)
        
        // Test configuration persistence
        configManager.saveConfiguration()
        
        let persistedConfig = configManager.loadConfiguration()
        XCTAssertEqual(persistedConfig.batchSize, 50)
        XCTAssertFalse(persistedConfig.enablePrivacyFiltering)
    }
    
    // MARK: - Stress Tests
    
    func testStressCollection() {
        let collector = KeyTapCollector(ringBuffer: ringBuffer)
        
        // Configure for stress testing
        let config = CollectorConfiguration()
        config.testMode = true
        config.simulatedEventRate = 5000 // 5000 events per second
        collector.configure(with: config)
        
        let expectation = self.expectation(description: "Stress test")
        
        // Monitor system resources
        let startMemory = testHarness.getCurrentMemoryUsage()
        let startCPU = testHarness.getCurrentCPUUsage()
        
        // Start collection
        collector.startCollection()
        
        // Run stress test for 10 seconds
        DispatchQueue.main.asyncAfter(deadline: .now() + 10.0) {
            collector.stopCollection()
            
            // Check system resources
            let endMemory = self.testHarness.getCurrentMemoryUsage()
            let endCPU = self.testHarness.getCurrentCPUUsage()
            
            // Verify system remained stable
            XCTAssertLessThan(endMemory - startMemory, 500 * 1024 * 1024) // Less than 500MB increase
            XCTAssertLessThan(endCPU - startCPU, 50) // Less than 50% CPU increase
            
            // Verify events were collected
            let events = self.ringBuffer.getEvents()
            XCTAssertGreaterThan(events.count, 1000) // Should have collected many events
            
            expectation.fulfill()
        }
        
        waitForExpectations(timeout: 15.0)
    }
}

// MARK: - Test Support Classes

class MockRingBuffer {
    private var events: [TestEvent] = []
    private let lock = NSLock()
    let capacity: Int
    
    init(capacity: Int) {
        self.capacity = capacity
    }
    
    func writeEvent(_ event: TestEvent) {
        lock.lock()
        defer { lock.unlock() }
        
        if events.count >= capacity {
            events.removeFirst()
        }
        events.append(event)
    }
    
    func getEvents() -> [TestEvent] {
        lock.lock()
        defer { lock.unlock() }
        return events
    }
    
    func getStats() -> RingBufferStats {
        lock.lock()
        defer { lock.unlock() }
        
        return RingBufferStats(
            totalWrites: events.count,
            totalReads: 0,
            capacity: capacity,
            utilization: Double(events.count) / Double(capacity)
        )
    }
}

struct TestEvent {
    let type: String
    let data: [String: Any]
    let timestamp: Date
}

struct RingBufferStats {
    let totalWrites: Int
    let totalReads: Int
    let capacity: Int
    let utilization: Double
}

class TestHarness {
    func getCurrentMemoryUsage() -> UInt64 {
        // Implementation would get actual memory usage
        return 100 * 1024 * 1024 // 100MB placeholder
    }
    
    func getCurrentCPUUsage() -> Double {
        // Implementation would get actual CPU usage
        return 25.0 // 25% placeholder
    }
    
    func cleanup() {
        // Cleanup test resources
    }
}

class TestLogger {
    static let shared = TestLogger()
    
    func configure(level: LogLevel) {
        // Configure logging for tests
    }
    
    enum LogLevel {
        case debug, info, warning, error
    }
}

// MARK: - Test Configuration

struct TestConfiguration {
    let timeout: TimeInterval
    let retryCount: Int
    let enablePerformanceTests: Bool
    let enableStressTests: Bool
    
    static func `default`() -> TestConfiguration {
        return TestConfiguration(
            timeout: 30.0,
            retryCount: 3,
            enablePerformanceTests: true,
            enableStressTests: false // Disable by default for CI
        )
    }
}