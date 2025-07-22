//
//  DriveMonCollector.swift
//  ChronicleCollectors
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import Foundation
import DiskArbitration
import os.log

/// Drive monitoring collector using DiskArbitration framework
public class DriveMonCollector: CollectorBase {
    private var session: DASession?
    private let permissionManager: PermissionManager
    private var isMonitoring = false
    private var targetDrives: [DriveIdentifier] = []
    
    public init(configuration: CollectorConfiguration = .default,
                ringBufferWriter: PerformantRingBufferWriter,
                permissionManager: PermissionManager = PermissionManager(),
                targetDrives: [DriveIdentifier] = []) {
        self.permissionManager = permissionManager
        self.targetDrives = targetDrives
        
        super.init(
            identifier: "drive_mon",
            displayName: "Drive Monitor",
            eventTypes: [.driveActivity],
            configuration: configuration,
            ringBufferWriter: ringBufferWriter
        )
    }
    
    // MARK: - CollectorProtocol Implementation
    
    public override func checkPermissions() -> Bool {
        // DiskArbitration doesn't require special permissions beyond file access
        return permissionManager.checkPermission(.fileAccess) == .granted
    }
    
    public override func requestPermissions() async throws {
        try await permissionManager.requestPermission(.fileAccess)
    }
    
    public override func startCollector() throws {
        guard checkPermissions() else {
            throw ChronicleCollectorError.permissionDenied("File access permission required for drive monitoring")
        }
        
        try startDiskArbitrationSession()
        isMonitoring = true
        
        logger.info("Drive monitor collector started successfully")
    }
    
    public override func stopCollector() throws {
        stopDiskArbitrationSession()
        isMonitoring = false
        
        logger.info("Drive monitor collector stopped")
    }
    
    // MARK: - DiskArbitration Implementation
    
    private func startDiskArbitrationSession() throws {
        session = DASessionCreate(kCFAllocatorDefault)
        
        guard let session = session else {
            throw ChronicleCollectorError.systemError("Failed to create DiskArbitration session")
        }
        
        // Set up disk appeared callback
        DARegisterDiskAppearedCallback(
            session,
            nil, // Match all disks
            { (disk, context) in
                guard let context = context else { return }
                let collector = Unmanaged<DriveMonCollector>.fromOpaque(context).takeUnretainedValue()
                collector.handleDiskAppeared(disk: disk)
            },
            Unmanaged.passUnretained(self).toOpaque()
        )
        
        // Set up disk disappeared callback
        DARegisterDiskDisappearedCallback(
            session,
            nil, // Match all disks
            { (disk, context) in
                guard let context = context else { return }
                let collector = Unmanaged<DriveMonCollector>.fromOpaque(context).takeUnretainedValue()
                collector.handleDiskDisappeared(disk: disk)
            },
            Unmanaged.passUnretained(self).toOpaque()
        )
        
        // Schedule session with run loop
        DASessionScheduleWithRunLoop(session, CFRunLoopGetCurrent(), CFRunLoopMode.defaultMode.rawValue)
        
        logger.info("DiskArbitration session started")
    }
    
    private func stopDiskArbitrationSession() {
        guard let session = session else { return }
        
        // Unregister callbacks
        DAUnregisterCallback(session, { (disk, context) in }, Unmanaged.passUnretained(self).toOpaque())
        
        // Unschedule from run loop
        DASessionUnscheduleFromRunLoop(session, CFRunLoopGetCurrent(), CFRunLoopMode.defaultMode.rawValue)
        
        self.session = nil
        
        logger.info("DiskArbitration session stopped")
    }
    
    private func handleDiskAppeared(disk: DADisk) {
        guard isRunning && isMonitoring else { return }
        
        // Skip if sampling rate doesn't match
        if configuration.sampleRate < 1.0 && Double.random(in: 0...1) > configuration.sampleRate {
            return
        }
        
        let driveInfo = extractDriveInfo(from: disk)
        let action: DriveAction = driveInfo.mountPoint != nil ? .mounted : .appeared
        
        // Check if this drive should trigger auto-backup
        let shouldTriggerBackup = isTargetDrive(driveInfo.driveIdentifier)
        
        let eventData = DriveActivityEventData(
            driveIdentifier: driveInfo.driveIdentifier,
            action: action,
            mountPoint: driveInfo.mountPoint,
            fileSystem: driveInfo.fileSystem,
            totalSize: driveInfo.totalSize,
            availableSize: driveInfo.availableSize,
            isRemovable: driveInfo.isRemovable,
            isInternal: driveInfo.isInternal,
            connectionType: driveInfo.connectionType,
            deviceName: driveInfo.deviceName,
            vendorName: driveInfo.vendorName,
            productName: driveInfo.productName,
            serialNumber: driveInfo.serialNumber
        )
        
        emitDriveEvent(eventData, shouldTriggerBackup: shouldTriggerBackup)
        updateActivity()
    }
    
    private func handleDiskDisappeared(disk: DADisk) {
        guard isRunning && isMonitoring else { return }
        
        // Skip if sampling rate doesn't match
        if configuration.sampleRate < 1.0 && Double.random(in: 0...1) > configuration.sampleRate {
            return
        }
        
        let driveInfo = extractDriveInfo(from: disk)
        
        let eventData = DriveActivityEventData(
            driveIdentifier: driveInfo.driveIdentifier,
            action: .disappeared,
            mountPoint: driveInfo.mountPoint,
            fileSystem: driveInfo.fileSystem,
            totalSize: driveInfo.totalSize,
            availableSize: driveInfo.availableSize,
            isRemovable: driveInfo.isRemovable,
            isInternal: driveInfo.isInternal,
            connectionType: driveInfo.connectionType,
            deviceName: driveInfo.deviceName,
            vendorName: driveInfo.vendorName,
            productName: driveInfo.productName,
            serialNumber: driveInfo.serialNumber
        )
        
        emitDriveEvent(eventData, shouldTriggerBackup: false)
        updateActivity()
    }
    
    private func emitDriveEvent(_ eventData: DriveActivityEventData, shouldTriggerBackup: Bool) {
        do {
            let jsonData = try JSONEncoder().encode(eventData)
            var metadata = [
                "action": eventData.action.rawValue,
                "is_removable": String(eventData.isRemovable),
                "is_internal": String(eventData.isInternal),
                "mount_point": eventData.mountPoint ?? "none"
            ]
            
            if shouldTriggerBackup {
                metadata["trigger_auto_backup"] = "true"
            }
            
            if let uuid = eventData.driveIdentifier.uuid {
                metadata["drive_uuid"] = uuid
            }
            
            if let bsdName = eventData.driveIdentifier.bsdName {
                metadata["bsd_name"] = bsdName
            }
            
            let chronicleEvent = createEvent(type: .driveActivity, data: jsonData, metadata: metadata)
            emitEvent(chronicleEvent)
            
        } catch {
            logger.error("Failed to encode drive activity event: \(error)")
        }
    }
    
    // MARK: - Utility Methods
    
    private func extractDriveInfo(from disk: DADisk) -> (
        driveIdentifier: DriveIdentifier,
        mountPoint: String?,
        fileSystem: String?,
        totalSize: UInt64?,
        availableSize: UInt64?,
        isRemovable: Bool,
        isInternal: Bool,
        connectionType: String?,
        deviceName: String?,
        vendorName: String?,
        productName: String?,
        serialNumber: String?
    ) {
        guard let description = DADiskCopyDescription(disk) as? [String: Any] else {
            return (
                driveIdentifier: DriveIdentifier(),
                mountPoint: nil,
                fileSystem: nil,
                totalSize: nil,
                availableSize: nil,
                isRemovable: false,
                isInternal: true,
                connectionType: nil,
                deviceName: nil,
                vendorName: nil,
                productName: nil,
                serialNumber: nil
            )
        }
        
        // Extract drive identifiers
        let uuid = description[kDADiskDescriptionVolumeUUIDKey as String] as? CFUUID
        let uuidString = uuid != nil ? CFUUIDCreateString(kCFAllocatorDefault, uuid!) as String : nil
        
        let bsdName = description[kDADiskDescriptionMediaBSDNameKey as String] as? String
        let volumeName = description[kDADiskDescriptionVolumeNameKey as String] as? String
        let mediaSerialNumber = description[kDADiskDescriptionMediaSerialKey as String] as? String
        
        let driveIdentifier = DriveIdentifier(
            uuid: uuidString,
            bsdName: bsdName,
            volumeLabel: volumeName,
            serialNumber: mediaSerialNumber
        )
        
        // Extract drive information
        let mountPoint = description[kDADiskDescriptionVolumeMountableKey as String] as? Bool == true ?
            (description[kDADiskDescriptionVolumePathKey as String] as? URL)?.path : nil
        
        let fileSystem = description[kDADiskDescriptionVolumeKindKey as String] as? String
        let totalSize = description[kDADiskDescriptionMediaSizeKey as String] as? UInt64
        let isRemovable = description[kDADiskDescriptionMediaRemovableKey as String] as? Bool ?? false
        let isInternal = !(description[kDADiskDescriptionDeviceInternalKey as String] as? Bool ?? true)
        
        // Connection and device info
        let connectionType = description[kDADiskDescriptionDeviceProtocolKey as String] as? String
        let deviceName = description[kDADiskDescriptionMediaNameKey as String] as? String
        let vendorName = description[kDADiskDescriptionDeviceVendorKey as String] as? String
        let productName = description[kDADiskDescriptionDeviceModelKey as String] as? String
        
        // Calculate available space if mounted
        var availableSize: UInt64? = nil
        if let mountPath = mountPoint {
            do {
                let url = URL(fileURLWithPath: mountPath)
                let values = try url.resourceValues(forKeys: [.volumeAvailableCapacityKey])
                availableSize = values.volumeAvailableCapacity.map { UInt64($0) }
            } catch {
                logger.debug("Could not get available space for \(mountPath): \(error)")
            }
        }
        
        return (
            driveIdentifier: driveIdentifier,
            mountPoint: mountPoint,
            fileSystem: fileSystem,
            totalSize: totalSize,
            availableSize: availableSize,
            isRemovable: isRemovable,
            isInternal: isInternal,
            connectionType: connectionType,
            deviceName: deviceName,
            vendorName: vendorName,
            productName: productName,
            serialNumber: mediaSerialNumber
        )
    }
    
    private func isTargetDrive(_ identifier: DriveIdentifier) -> Bool {
        for target in targetDrives {
            // Check UUID match
            if let targetUUID = target.uuid, let driveUUID = identifier.uuid {
                if targetUUID == driveUUID {
                    return true
                }
            }
            
            // Check BSD name match
            if let targetBSD = target.bsdName, let driveBSD = identifier.bsdName {
                if targetBSD == driveBSD {
                    return true
                }
            }
            
            // Check volume label match
            if let targetLabel = target.volumeLabel, let driveLabel = identifier.volumeLabel {
                if targetLabel == driveLabel {
                    return true
                }
            }
            
            // Check serial number match
            if let targetSerial = target.serialNumber, let driveSerial = identifier.serialNumber {
                if targetSerial == driveSerial {
                    return true
                }
            }
        }
        
        return false
    }
    
    /// Set target drives for auto-backup
    public func setTargetDrives(_ drives: [DriveIdentifier]) {
        targetDrives = drives
        logger.info("Updated target drives for auto-backup: \(drives.count) drives")
    }
    
    /// Get current target drives
    public func getTargetDrives() -> [DriveIdentifier] {
        return targetDrives
    }
    
    /// Get drive monitoring statistics
    public func getDriveStatistics() -> [String: Any] {
        return [
            "is_monitoring": isMonitoring,
            "session_active": session != nil,
            "target_drive_count": targetDrives.count
        ]
    }
    
    /// Get information about all currently connected drives
    public func getAllConnectedDrives() -> [DriveActivityEventData] {
        var drives: [DriveActivityEventData] = []
        
        guard let session = session else { return drives }
        
        // This is a simplified version - in a real implementation we'd enumerate all disks
        // For now, we'll return an empty array as this would require more complex DiskArbitration usage
        
        return drives
    }
}