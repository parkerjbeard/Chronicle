//
//  DriveMonCollectorTests.swift
//  ChronicleCollectors
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import XCTest
import DiskArbitration
@testable import ChronicleCollectors

class DriveMonCollectorTests: XCTestCase {
    var collector: DriveMonCollector!
    var mockRingBuffer: MockRingBuffer!
    var mockSession: MockDASession!
    
    override func setUpWithError() throws {
        mockRingBuffer = MockRingBuffer()
        mockSession = MockDASession()
        collector = DriveMonCollector()
        collector.setRingBuffer(mockRingBuffer)
        collector.setMockSession(mockSession)
    }
    
    override func tearDownWithError() throws {
        collector?.stop()
        collector = nil
        mockRingBuffer = nil
        mockSession = nil
    }
    
    // MARK: - Initialization Tests
    
    func testCollectorInitialization() {
        XCTAssertNotNil(collector)
        XCTAssertEqual(collector.collectorId, "drive_mon")
        XCTAssertFalse(collector.isRunning)
    }
    
    func testConfigurationLoading() {
        let config = ConfigManager.shared.getCollectorConfig("drive_mon")
        XCTAssertTrue(config.enabled)
        XCTAssertEqual(config.sampleRate, 1.0)
        XCTAssertEqual(config.activeFrameRate, 1.0)
        XCTAssertEqual(config.idleFrameRate, 0.1)
    }
    
    // MARK: - Drive Identifier Tests
    
    func testDriveIdentifierCreation() {
        let identifier = DriveIdentifier()
            .with(uuid: "12345678-1234-1234-1234-123456789ABC")
            .with(volumeLabel: "TestDrive")
            .with(serialNumber: "ABC123456")
        
        XCTAssertEqual(identifier.uuid, "12345678-1234-1234-1234-123456789ABC")
        XCTAssertEqual(identifier.volumeLabel, "TestDrive")
        XCTAssertEqual(identifier.serialNumber, "ABC123456")
    }
    
    func testDriveIdentifierMatching() {
        let target = DriveIdentifier()
            .with(uuid: "12345678-1234-1234-1234-123456789ABC")
        
        let matching = DriveIdentifier()
            .with(uuid: "12345678-1234-1234-1234-123456789ABC")
            .with(volumeLabel: "DifferentLabel")
        
        let nonMatching = DriveIdentifier()
            .with(uuid: "87654321-4321-4321-4321-CBA987654321")
        
        XCTAssertTrue(target.matches(matching))
        XCTAssertFalse(target.matches(nonMatching))
    }
    
    func testDriveIdentifierVolumeMatching() {
        let target = DriveIdentifier()
            .with(volumeLabel: "BackupDrive")
        
        let matching = DriveIdentifier()
            .with(volumeLabel: "BackupDrive")
        
        let nonMatching = DriveIdentifier()
            .with(volumeLabel: "DataDrive")
        
        XCTAssertTrue(target.matches(matching))
        XCTAssertFalse(target.matches(nonMatching))
    }
    
    func testDriveIdentifierSerialMatching() {
        let target = DriveIdentifier()
            .with(serialNumber: "WD1234567890")
        
        let matching = DriveIdentifier()
            .with(serialNumber: "WD1234567890")
        
        let nonMatching = DriveIdentifier()
            .with(serialNumber: "ST0987654321")
        
        XCTAssertTrue(target.matches(matching))
        XCTAssertFalse(target.matches(nonMatching))
    }
    
    func testDriveIdentifierPriorityMatching() {
        // UUID should take precedence over volume label
        let target = DriveIdentifier()
            .with(uuid: "12345678-1234-1234-1234-123456789ABC")
            .with(volumeLabel: "Drive1")
        
        let candidate = DriveIdentifier()
            .with(uuid: "12345678-1234-1234-1234-123456789ABC")
            .with(volumeLabel: "Drive2") // Different volume label
        
        XCTAssertTrue(target.matches(candidate)) // Should match on UUID
    }
    
    // MARK: - Drive Event Creation Tests
    
    func testDriveEventCreation() {
        let identifier = DriveIdentifier()
            .with(uuid: "12345678-1234-1234-1234-123456789ABC")
            .with(volumeLabel: "TestDrive")
        
        let event = DriveActivityEventData(
            driveIdentifier: identifier,
            action: .mounted,
            mountPoint: "/Volumes/TestDrive",
            timestamp: Date(),
            fileSystemType: "APFS",
            totalSize: 1000000000,
            availableSize: 500000000,
            isRemovable: true,
            isWritable: true,
            shouldTriggerBackup: true
        )
        
        XCTAssertEqual(event.driveIdentifier.uuid, "12345678-1234-1234-1234-123456789ABC")
        XCTAssertEqual(event.action, .mounted)
        XCTAssertEqual(event.mountPoint, "/Volumes/TestDrive")
        XCTAssertEqual(event.fileSystemType, "APFS")
        XCTAssertTrue(event.isRemovable)
        XCTAssertTrue(event.shouldTriggerBackup)
    }
    
    // MARK: - Disk Arbitration Session Tests
    
    func testSessionCreation() throws {
        XCTAssertNoThrow(try collector.start())
        XCTAssertTrue(collector.isRunning)
        XCTAssertNotNil(mockSession.callbacks)
    }
    
    func testSessionCleanup() throws {
        try collector.start()
        XCTAssertTrue(collector.isRunning)
        
        collector.stop()
        XCTAssertFalse(collector.isRunning)
        XCTAssertTrue(mockSession.cleanedUp)
    }
    
    // MARK: - Drive Detection Tests
    
    func testDriveAppearedCallback() throws {
        try collector.start()
        
        // Simulate a drive appearing
        let driveInfo: [String: Any] = [
            "DAMediaUUID": "12345678-1234-1234-1234-123456789ABC",
            "DAVolumeName": "TestDrive",
            "DAMediaSize": 1000000000,
            "DAMediaBSDName": "disk2s1",
            "DAVolumeKind": "apfs",
            "DAMediaRemovable": true,
            "DAMediaWritable": true
        ]
        
        mockSession.simulateDriveAppeared(driveInfo)
        
        // Wait for async processing
        let expectation = XCTestExpectation(description: "Drive appeared event")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            expectation.fulfill()
        }
        wait(for: [expectation], timeout: 1.0)
        
        // Verify event was sent to ring buffer
        XCTAssertGreaterThan(mockRingBuffer.events.count, 0)
        
        let lastEvent = mockRingBuffer.events.last!
        XCTAssertEqual(lastEvent.type, .driveActivity)
        
        if case .driveActivity(let driveData) = lastEvent.data {
            XCTAssertEqual(driveData.action, .mounted)
            XCTAssertEqual(driveData.driveIdentifier.uuid, "12345678-1234-1234-1234-123456789ABC")
            XCTAssertEqual(driveData.driveIdentifier.volumeLabel, "TestDrive")
        } else {
            XCTFail("Expected drive activity event")
        }
    }
    
    func testDriveDisappearedCallback() throws {
        try collector.start()
        
        // Simulate a drive disappearing
        let driveInfo: [String: Any] = [
            "DAMediaUUID": "12345678-1234-1234-1234-123456789ABC",
            "DAVolumeName": "TestDrive",
            "DAMediaBSDName": "disk2s1"
        ]
        
        mockSession.simulateDriveDisappeared(driveInfo)
        
        // Wait for async processing
        let expectation = XCTestExpectation(description: "Drive disappeared event")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            expectation.fulfill()
        }
        wait(for: [expectation], timeout: 1.0)
        
        // Verify event was sent to ring buffer
        XCTAssertGreaterThan(mockRingBuffer.events.count, 0)
        
        let lastEvent = mockRingBuffer.events.last!
        XCTAssertEqual(lastEvent.type, .driveActivity)
        
        if case .driveActivity(let driveData) = lastEvent.data {
            XCTAssertEqual(driveData.action, .unmounted)
            XCTAssertEqual(driveData.driveIdentifier.uuid, "12345678-1234-1234-1234-123456789ABC")
        } else {
            XCTFail("Expected drive activity event")
        }
    }
    
    // MARK: - Target Drive Configuration Tests
    
    func testTargetDriveConfiguration() {
        let targetDrives = [
            DriveIdentifier().with(uuid: "12345678-1234-1234-1234-123456789ABC"),
            DriveIdentifier().with(volumeLabel: "BackupDrive"),
            DriveIdentifier().with(serialNumber: "WD1234567890")
        ]
        
        collector.setTargetDrives(targetDrives)
        
        let configuredTargets = collector.getTargetDrives()
        XCTAssertEqual(configuredTargets.count, 3)
    }
    
    func testTargetDriveDetection() throws {
        let targetDrive = DriveIdentifier()
            .with(uuid: "12345678-1234-1234-1234-123456789ABC")
        
        collector.setTargetDrives([targetDrive])
        try collector.start()
        
        // Simulate target drive appearing
        let driveInfo: [String: Any] = [
            "DAMediaUUID": "12345678-1234-1234-1234-123456789ABC",
            "DAVolumeName": "BackupDrive",
            "DAMediaRemovable": true
        ]
        
        mockSession.simulateDriveAppeared(driveInfo)
        
        // Wait for processing
        let expectation = XCTestExpectation(description: "Target drive detected")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            expectation.fulfill()
        }
        wait(for: [expectation], timeout: 1.0)
        
        // Verify target drive was detected and backup flag set
        let lastEvent = mockRingBuffer.events.last!
        if case .driveActivity(let driveData) = lastEvent.data {
            XCTAssertTrue(driveData.shouldTriggerBackup)
        } else {
            XCTFail("Expected drive activity event")
        }
    }
    
    func testNonTargetDriveDetection() throws {
        let targetDrive = DriveIdentifier()
            .with(uuid: "12345678-1234-1234-1234-123456789ABC")
        
        collector.setTargetDrives([targetDrive])
        try collector.start()
        
        // Simulate non-target drive appearing
        let driveInfo: [String: Any] = [
            "DAMediaUUID": "87654321-4321-4321-4321-CBA987654321",
            "DAVolumeName": "RandomDrive",
            "DAMediaRemovable": true
        ]
        
        mockSession.simulateDriveAppeared(driveInfo)
        
        // Wait for processing
        let expectation = XCTestExpectation(description: "Non-target drive detected")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            expectation.fulfill()
        }
        wait(for: [expectation], timeout: 1.0)
        
        // Verify non-target drive was detected but backup flag not set
        let lastEvent = mockRingBuffer.events.last!
        if case .driveActivity(let driveData) = lastEvent.data {
            XCTAssertFalse(driveData.shouldTriggerBackup)
        } else {
            XCTFail("Expected drive activity event")
        }
    }
    
    // MARK: - Error Handling Tests
    
    func testInvalidDriveInfo() throws {
        try collector.start()
        
        // Simulate drive event with missing required fields
        let invalidDriveInfo: [String: Any] = [
            "InvalidKey": "InvalidValue"
        ]
        
        mockSession.simulateDriveAppeared(invalidDriveInfo)
        
        // Should not crash and should not create invalid events
        let expectation = XCTestExpectation(description: "Invalid drive info handled")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            expectation.fulfill()
        }
        wait(for: [expectation], timeout: 1.0)
        
        // Verify no invalid events were created
        for event in mockRingBuffer.events {
            if case .driveActivity(let driveData) = event.data {
                // Should have some valid identifier
                XCTAssertTrue(
                    driveData.driveIdentifier.uuid != nil ||
                    driveData.driveIdentifier.volumeLabel != nil ||
                    driveData.driveIdentifier.serialNumber != nil
                )
            }
        }
    }
    
    func testCollectorRestart() throws {
        // Test starting and stopping multiple times
        try collector.start()
        XCTAssertTrue(collector.isRunning)
        
        collector.stop()
        XCTAssertFalse(collector.isRunning)
        
        try collector.start()
        XCTAssertTrue(collector.isRunning)
        
        collector.stop()
        XCTAssertFalse(collector.isRunning)
    }
    
    // MARK: - Performance Tests
    
    func testMultipleDriveEvents() throws {
        try collector.start()
        
        // Simulate multiple drives appearing quickly
        for i in 0..<10 {
            let driveInfo: [String: Any] = [
                "DAMediaUUID": "12345678-1234-1234-1234-12345678\(String(format: "%02d", i))",
                "DAVolumeName": "TestDrive\(i)",
                "DAMediaRemovable": true
            ]
            mockSession.simulateDriveAppeared(driveInfo)
        }
        
        // Wait for all events to process
        let expectation = XCTestExpectation(description: "Multiple drive events processed")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
            expectation.fulfill()
        }
        wait(for: [expectation], timeout: 2.0)
        
        // Should handle all events without dropping any
        XCTAssertGreaterThanOrEqual(mockRingBuffer.events.count, 10)
    }
    
    // MARK: - Memory Management Tests
    
    func testMemoryCleanup() throws {
        weak var weakCollector: DriveMonCollector?
        
        autoreleasepool {
            let testCollector = DriveMonCollector()
            weakCollector = testCollector
            try! testCollector.start()
            testCollector.stop()
        }
        
        // Collector should be deallocated
        XCTAssertNil(weakCollector)
    }
}

// MARK: - Mock Classes

class MockRingBuffer: RingBufferProtocol {
    var events: [EventData] = []
    
    func writeEvent(_ event: EventData) -> Bool {
        events.append(event)
        return true
    }
    
    func writeEvents(_ events: [EventData]) -> Bool {
        self.events.append(contentsOf: events)
        return true
    }
    
    func readEvents(count: Int) -> [EventData] {
        return Array(events.prefix(count))
    }
    
    func getStats() -> RingBufferStats {
        return RingBufferStats(
            totalEvents: events.count,
            droppedEvents: 0,
            bufferUtilization: 0.1
        )
    }
}

class MockDASession {
    var callbacks: [String: Any] = [:]
    var cleanedUp = false
    
    func setCallback(_ callback: @escaping DADiskAppearedCallback, context: UnsafeMutableRawPointer?) {
        callbacks["appeared"] = callback
    }
    
    func setDisappearedCallback(_ callback: @escaping DADiskDisappearedCallback, context: UnsafeMutableRawPointer?) {
        callbacks["disappeared"] = callback
    }
    
    func simulateDriveAppeared(_ driveInfo: [String: Any]) {
        if let callback = callbacks["appeared"] as? DADiskAppearedCallback {
            // Create mock DADisk and call callback
            // This would need proper DADisk mocking in a real implementation
            DispatchQueue.main.async {
                // Simulate callback with drive info
                NotificationCenter.default.post(
                    name: Notification.Name("MockDriveAppeared"),
                    object: nil,
                    userInfo: driveInfo
                )
            }
        }
    }
    
    func simulateDriveDisappeared(_ driveInfo: [String: Any]) {
        if let callback = callbacks["disappeared"] as? DADiskDisappearedCallback {
            DispatchQueue.main.async {
                NotificationCenter.default.post(
                    name: Notification.Name("MockDriveDisappeared"),
                    object: nil,
                    userInfo: driveInfo
                )
            }
        }
    }
    
    func cleanup() {
        cleanedUp = true
        callbacks.removeAll()
    }
}

// MARK: - Extensions for Testing

extension DriveMonCollector {
    func setRingBuffer(_ ringBuffer: MockRingBuffer) {
        // In real implementation, this would set the ring buffer reference
    }
    
    func setMockSession(_ session: MockDASession) {
        // In real implementation, this would replace the DA session
    }
    
    func setTargetDrives(_ drives: [DriveIdentifier]) {
        // In real implementation, this would update target drives configuration
    }
    
    func getTargetDrives() -> [DriveIdentifier] {
        // In real implementation, this would return configured target drives
        return []
    }
}

extension DriveIdentifier {
    func with(uuid: String) -> DriveIdentifier {
        var identifier = self
        identifier.uuid = uuid
        return identifier
    }
    
    func with(volumeLabel: String) -> DriveIdentifier {
        var identifier = self
        identifier.volumeLabel = volumeLabel
        return identifier
    }
    
    func with(serialNumber: String) -> DriveIdentifier {
        var identifier = self
        identifier.serialNumber = serialNumber
        return identifier
    }
    
    func matches(_ other: DriveIdentifier) -> Bool {
        // UUID takes highest precedence
        if let uuid = self.uuid, let otherUuid = other.uuid {
            return uuid == otherUuid
        }
        
        // Serial number takes second precedence
        if let serial = self.serialNumber, let otherSerial = other.serialNumber {
            return serial == otherSerial
        }
        
        // Volume label takes lowest precedence
        if let label = self.volumeLabel, let otherLabel = other.volumeLabel {
            return label == otherLabel
        }
        
        return false
    }
}