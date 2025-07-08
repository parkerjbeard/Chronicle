//
//  ChronicleCollectorsTests.swift
//  ChronicleCollectorsTests
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import XCTest
@testable import ChronicleCollectors

class ChronicleCollectorsTests: XCTestCase {
    
    var collectors: ChronicleCollectors!
    
    override func setUpWithError() throws {
        collectors = ChronicleCollectors.shared
    }
    
    override func tearDownWithError() throws {
        try? collectors.stopCollectors()
        collectors = nil
    }
    
    func testCollectorsInitialization() throws {
        XCTAssertNotNil(collectors)
        XCTAssertFalse(collectors.isRunning)
        XCTAssertGreaterThan(collectors.collectors.count, 0)
    }
    
    func testCollectorIdentifiers() throws {
        let expectedCollectors = [
            "key_tap",
            "screen_tap",
            "window_mon",
            "pointer_mon",
            "clip_mon",
            "fs_mon",
            "audio_mon",
            "net_mon"
        ]
        
        for expectedId in expectedCollectors {
            if expectedId == "screen_tap" && !ProcessInfo.processInfo.isOperatingSystemAtLeast(OperatingSystemVersion(majorVersion: 12, minorVersion: 3, patchVersion: 0)) {
                continue // Skip screen_tap on older macOS versions
            }
            XCTAssertNotNil(collectors.collectors[expectedId], "Collector \(expectedId) should be initialized")
        }
    }
    
    func testPermissionChecking() throws {
        let permissions = collectors.checkAllPermissions()
        XCTAssertGreaterThan(permissions.count, 0)
        
        // Check that we have permission entries for required types
        XCTAssertNotNil(permissions[.accessibility])
        XCTAssertNotNil(permissions[.inputMonitoring])
        XCTAssertNotNil(permissions[.screenRecording])
    }
    
    func testCollectorConfiguration() throws {
        let config = CollectorConfiguration(enabled: false, sampleRate: 0.5)
        
        try collectors.updateCollectorConfiguration("key_tap", config: config)
        
        let updatedConfig = ConfigManager.shared.getCollectorConfig("key_tap")
        XCTAssertEqual(updatedConfig.enabled, false)
        XCTAssertEqual(updatedConfig.sampleRate, 0.5)
    }
    
    func testRingBufferStatistics() throws {
        let stats = collectors.getRingBufferStatistics()
        XCTAssertGreaterThanOrEqual(stats.size, 0)
        XCTAssertGreaterThanOrEqual(stats.available, 0)
        XCTAssertGreaterThanOrEqual(stats.used, 0)
    }
    
    func testSystemHealth() throws {
        let health = collectors.getSystemHealth()
        XCTAssertGreaterThanOrEqual(health.totalCollectors, 0)
        XCTAssertGreaterThanOrEqual(health.runningCollectors, 0)
        XCTAssertLessThanOrEqual(health.runningCollectors, health.totalCollectors)
    }
    
    func testCollectorInfo() throws {
        guard let info = collectors.getCollectorInfo("key_tap") else {
            XCTFail("Key tap collector info should be available")
            return
        }
        
        XCTAssertEqual(info.identifier, "key_tap")
        XCTAssertEqual(info.displayName, "Keyboard Events")
        XCTAssertTrue(info.eventTypes.contains(.keyTap))
    }
    
    func testAvailableCollectorTypes() throws {
        let types = collectors.getAvailableCollectorTypes()
        XCTAssertGreaterThan(types.count, 0)
        XCTAssertNotNil(types["key_tap"])
        XCTAssertNotNil(types["window_mon"])
    }
    
    // MARK: - Performance Tests
    
    func testRingBufferPerformance() throws {
        let ringBuffer = PerformantRingBufferWriter()
        let event = ChronicleEvent(type: .keyTap, data: Data("test".utf8))
        
        measure {
            for _ in 0..<1000 {
                ringBuffer.writeAsync(event)
            }
        }
    }
    
    func testEventCreationPerformance() throws {
        let eventData = KeyTapEventData(
            keyCode: 65,
            modifierFlags: 0,
            isKeyDown: true,
            location: CGPoint(x: 100, y: 100)
        )
        
        measure {
            for _ in 0..<1000 {
                do {
                    let jsonData = try JSONEncoder().encode(eventData)
                    let _ = ChronicleEvent(type: .keyTap, data: jsonData)
                } catch {
                    XCTFail("Failed to create event: \(error)")
                }
            }
        }
    }
}

// MARK: - Mock Tests

class MockCollectorTests: XCTestCase {
    
    func testMockCollector() throws {
        let mockCollector = MockCollector()
        
        XCTAssertEqual(mockCollector.state, .stopped)
        XCTAssertFalse(mockCollector.isRunning)
        
        try mockCollector.start()
        XCTAssertEqual(mockCollector.state, .running)
        XCTAssertTrue(mockCollector.isRunning)
        
        try mockCollector.stop()
        XCTAssertEqual(mockCollector.state, .stopped)
        XCTAssertFalse(mockCollector.isRunning)
    }
}

// MARK: - Mock Collector

class MockCollector: CollectorBase {
    
    init() {
        let ringBuffer = PerformantRingBufferWriter()
        super.init(
            identifier: "mock",
            displayName: "Mock Collector",
            eventTypes: [.systemActivity],
            configuration: .default,
            ringBufferWriter: ringBuffer
        )
    }
    
    override func checkPermissions() -> Bool {
        return true
    }
    
    override func requestPermissions() async throws {
        // No permissions needed for mock
    }
    
    override func startCollector() throws {
        // Mock implementation
    }
    
    override func stopCollector() throws {
        // Mock implementation
    }
}

// MARK: - Event Type Tests

class EventTypeTests: XCTestCase {
    
    func testEventTypeSerialization() throws {
        let eventType = ChronicleEventType.keyTap
        let data = try JSONEncoder().encode(eventType)
        let decoded = try JSONDecoder().decode(ChronicleEventType.self, from: data)
        
        XCTAssertEqual(eventType, decoded)
    }
    
    func testKeyTapEventData() throws {
        let eventData = KeyTapEventData(
            keyCode: 65,
            modifierFlags: 256,
            isKeyDown: true,
            characters: "a",
            location: CGPoint(x: 100, y: 200)
        )
        
        let jsonData = try JSONEncoder().encode(eventData)
        let decoded = try JSONDecoder().decode(KeyTapEventData.self, from: jsonData)
        
        XCTAssertEqual(eventData.keyCode, decoded.keyCode)
        XCTAssertEqual(eventData.isKeyDown, decoded.isKeyDown)
        XCTAssertEqual(eventData.characters, decoded.characters)
    }
    
    func testScreenCaptureEventData() throws {
        let imageData = Data("fake_image_data".utf8)
        let eventData = ScreenCaptureEventData(
            imageData: imageData,
            format: "jpeg",
            width: 1920,
            height: 1080,
            scale: 2.0,
            display: "main",
            region: CGRect(x: 0, y: 0, width: 1920, height: 1080),
            compressionQuality: 0.8
        )
        
        let jsonData = try JSONEncoder().encode(eventData)
        let decoded = try JSONDecoder().decode(ScreenCaptureEventData.self, from: jsonData)
        
        XCTAssertEqual(eventData.imageData, decoded.imageData)
        XCTAssertEqual(eventData.format, decoded.format)
        XCTAssertEqual(eventData.width, decoded.width)
        XCTAssertEqual(eventData.height, decoded.height)
    }
    
    func testChronicleEvent() throws {
        let eventData = Data("test_data".utf8)
        let event = ChronicleEvent(
            type: .keyTap,
            data: eventData,
            metadata: ["test": "value"]
        )
        
        let jsonData = try JSONEncoder().encode(event)
        let decoded = try JSONDecoder().decode(ChronicleEvent.self, from: jsonData)
        
        XCTAssertEqual(event.type, decoded.type)
        XCTAssertEqual(event.data, decoded.data)
        XCTAssertEqual(event.metadata, decoded.metadata)
    }
}

// MARK: - Configuration Tests

class ConfigurationTests: XCTestCase {
    
    func testCollectorConfiguration() throws {
        let config = CollectorConfiguration(
            enabled: true,
            sampleRate: 0.8,
            adaptiveFrameRate: true,
            activeFrameRate: 10.0,
            idleFrameRate: 1.0
        )
        
        XCTAssertEqual(config.enabled, true)
        XCTAssertEqual(config.sampleRate, 0.8)
        XCTAssertEqual(config.adaptiveFrameRate, true)
        XCTAssertEqual(config.activeFrameRate, 10.0)
        XCTAssertEqual(config.idleFrameRate, 1.0)
    }
    
    func testChronicleConfig() throws {
        let config = ChronicleConfig.default
        
        XCTAssertGreaterThan(config.collectors.count, 0)
        XCTAssertNotNil(config.collectors["key_tap"])
        XCTAssertNotNil(config.collectors["window_mon"])
        
        try config.validate()
    }
    
    func testConfigValidation() throws {
        var config = ChronicleConfig.default
        
        // Test invalid configuration
        config.collectors["test"] = CollectorConfiguration(sampleRate: 2.0) // Invalid sample rate
        
        XCTAssertThrowsError(try config.validate()) { error in
            guard let collectorError = error as? ChronicleCollectorError else {
                XCTFail("Expected ChronicleCollectorError")
                return
            }
            
            if case .configurationError = collectorError {
                // Expected error type
            } else {
                XCTFail("Expected configuration error")
            }
        }
    }
}

// MARK: - Permission Tests

class PermissionTests: XCTestCase {
    
    func testPermissionManager() throws {
        let permissionManager = PermissionManager()
        
        // Test permission checking
        let accessibilityStatus = permissionManager.checkPermission(.accessibility)
        XCTAssertNotEqual(accessibilityStatus, .unknown)
        
        // Test required permissions
        let requiredPermissions = permissionManager.getRequiredPermissions(for: ["key_tap", "screen_tap"])
        XCTAssertTrue(requiredPermissions.contains(.accessibility))
        XCTAssertTrue(requiredPermissions.contains(.screenRecording))
    }
    
    func testPermissionTypeProperties() throws {
        let permission = PermissionType.accessibility
        
        XCTAssertEqual(permission.displayName, "Accessibility")
        XCTAssertFalse(permission.description.isEmpty)
    }
}